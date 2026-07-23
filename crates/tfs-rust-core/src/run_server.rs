//! Integrated TCP login + game + `GameWorld` + `run_game_loop` (OTBM + DB + outbound flush).
// C++ reference: `src/otserv.cpp` startup — load map, start game thread, accept connections.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Context;
use tokio::net::TcpListener;
use tokio::task::LocalSet;
use tracing::info;
use tfs_rust_common::GameCommand;

use tfs_rust_net::{
    open_game_command_channels, Codec, GameWireConfig, LoginWireConfig, OutRegistry, Server,
};

use crate::config::{
    password_hash_config_from, resolve_protocol_version, ConfigManager, DbConfig, MysqlPoolConfig,
    NetConfig,
};
use crate::event_dispatcher::NullEventDispatcher;
use crate::game_loop::{run_game_loop, wait_for_shutdown_signal};
use crate::game_world::GameWorld;
use crate::lua_event_dispatcher::LuaEventDispatcher;
use crate::lua_scope::register_lua_mutation_hooks;
use crate::map::Map;
use crate::spawn::SpawnManager;
use tfs_rust_lua::{load_chat_channel_scripts, load_talkaction_scripts, LuaRuntime, ScriptLoader};

/// Resolve PEM: `TFS_RSA_PEM` if set, else workspace-root `key.pem`, else `./key.pem`.
fn resolve_pem_path() -> anyhow::Result<PathBuf> {
    if let Ok(p) = std::env::var("TFS_RSA_PEM") {
        return Ok(PathBuf::from(p));
    }
    let workspace_root_key = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("key.pem");
    if workspace_root_key.is_file() {
        return Ok(workspace_root_key);
    }
    let cwd_key = PathBuf::from("key.pem");
    if cwd_key.is_file() {
        return Ok(cwd_key);
    }
    anyhow::bail!(
        "no RSA PEM found. Add key.pem at the workspace root, run from a directory containing key.pem, \
         or set TFS_RSA_PEM to a PKCS#1 private key path"
    )
}

/// Full server: load `config.lua`, data dir (OTBM + items XML, …), MariaDB, RSA, then
/// **login** (`TFS_LOGIN_ADDR`) + **game** (`TFS_GAME_ADDR`) with `run_game_loop` + `out_registry`.
///
/// Env: `DATABASE_URL` (if set, wins over `config.lua` MySQL keys), else URL is built from `config.lua` via `DbConfig`.
/// `TFS_DATA_DIR` (default `data` — use repo `data/` from project root), `TFS_MAP_OTBM` (default `world/forgotten.otbm`),
/// `TFS_CONFIG` (default `config.lua`),
/// `TFS_LOGIN_ADDR` / `TFS_GAME_ADDR` / `TFS_PUBLIC_IP` / `TFS_GAME_PORT` / `TFS_SERVER_NAME` / `TFS_MOTD` / `TFS_MOTD_NUM`.
pub async fn run() -> anyhow::Result<()> {
    register_lua_mutation_hooks();
    let pem_path = resolve_pem_path()?;
    info!(path = %pem_path.display(), "RSA PEM");

    let pem = std::fs::read_to_string(&pem_path).with_context(|| {
        format!(
            "cannot read RSA PEM at {:?}\n\
             Set TFS_RSA_PEM, or place key.pem at the workspace root.",
            pem_path
        )
    })?;
    let rsa_private_key = tfs_rust_net::rsa::private_key_from_pkcs1_pem(&pem)
        .context("load PKCS#1 RSA private key (BEGIN RSA PRIVATE KEY)")?;

    let config_path = std::env::var("TFS_CONFIG").unwrap_or_else(|_| "config.lua".to_string());
    let config = Rc::new(
        ConfigManager::load(Path::new(&config_path)).map_err(|e| anyhow::anyhow!("config: {e}"))?,
    );
    let net_cfg = NetConfig::from_config(config.as_ref())
        .map_err(|e| anyhow::anyhow!("config network settings: {e}"))?;
    let password_hash = password_hash_config_from(config.as_ref())
        .map_err(|e| anyhow::anyhow!("config password hash settings: {e}"))?;
    let protocol_version = resolve_protocol_version(config.as_ref())
        .map_err(|e| anyhow::anyhow!("config protocol version: {e}"))?;
    let codec =
        Codec::from_version(protocol_version).map_err(|e| anyhow::anyhow!("wire codec: {e}"))?;
    let protocol_caps = protocol_version.caps();
    info!(
        version = %protocol_version,
        ?protocol_caps,
        "protocol version"
    );

    // C++ ref: src/configmanager.cpp:178-184 (MySQL defaults and keys)
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => {
            info!(
                "database: using DATABASE_URL from environment (overrides config.lua mysql* keys)"
            );
            url
        }
        Err(_) => {
            let db_cfg = DbConfig::from_config(config.as_ref())
                .map_err(|e| anyhow::anyhow!("config database settings: {e}"))?;
            info!(
                host = %db_cfg.host,
                port = db_cfg.port,
                user = %db_cfg.user,
                database = %db_cfg.database,
                "database: using mysql* keys from config.lua (set DATABASE_URL to override)"
            );
            format!(
                "mysql://{}:{}@{}:{}/{}",
                db_cfg.user, db_cfg.pass, db_cfg.host, db_cfg.port, db_cfg.database
            )
        }
    };
    let pool_cfg = MysqlPoolConfig::from_config(config.as_ref())
        .map_err(|e| anyhow::anyhow!("config MySQL pool settings: {e}"))?;
    info!(
        min = pool_cfg.min_connections,
        max = pool_cfg.max_connections,
        idle_secs = pool_cfg.idle_timeout_secs,
        acquire_secs = pool_cfg.acquire_timeout_secs,
        "database pool: mysqlConnection* from config.lua"
    );
    let db = tfs_rust_db::DbPool::connect(&database_url, &pool_cfg.to_db_pool_options())
        .await
        .map_err(|e| anyhow::anyhow!("database connect: {e}"))?;
    let migrations_dir =
        tfs_rust_db::resolve_migrations_dir().map_err(|e| anyhow::anyhow!("{e}"))?;
    tfs_rust_db::run_migrations(&db, &migrations_dir)
        .await
        .map_err(|e| anyhow::anyhow!("database migrations: {e}"))?;

    let data_dir = std::env::var("TFS_DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let data_path = PathBuf::from(&data_dir);
    let map_rel =
        std::env::var("TFS_MAP_OTBM").unwrap_or_else(|_| "world/forgotten.otbm".to_string());
    let map_file = data_path.join(&map_rel);
    if !map_file.is_file() {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        anyhow::bail!(
            "map not found: {}\n\
             Set TFS_MAP_OTBM to the OTBM path under TFS_DATA_DIR (e.g. world/forgotten.otbm). Current TFS_DATA_DIR={:?}, cwd {}.\n\
             Copy your C++/Forgotten Server `data` folder here or: export TFS_DATA_DIR=/path/to/your-server/data",
            map_file.display(),
            data_dir,
            cwd.display()
        );
    }
    info!(dir = %data_path.display(), otbm = %map_rel, "loading content (OTBM, items, …)");
    let mut content = tfs_rust_content::pipeline::load_all(&data_path, Some(map_rel.as_str()))
        .await
        .map_err(|e| anyhow::anyhow!("content load: {e}"))?;

    let spawn_zones = std::mem::take(&mut content.map.spawn_zones);
    let items_db = std::sync::Arc::new(content.items);
    let monsters_db = std::sync::Arc::new(content.monsters);
    let groups = std::sync::Arc::new(content.groups);

    // Create items SlotMap first - needed for map loading to create Item instances
    let mut items = slotmap::SlotMap::with_key();
    let map = Map::from_map_data(content.map, items_db.as_ref(), &mut items);
    let spawns = SpawnManager::from_zones(spawn_zones);
    let vocations = std::sync::Arc::new(content.vocations);

    // Dual-lane game command bus (GL-2): bounded game packets + control lane.
    let (cmd_tx, game_rx, ctrl_rx) = open_game_command_channels();
    // `addEvent` / `stopEvent` scheduler — game-thread only (`Rc` → `!Send`).
    // C++ reference: `g_scheduler` (`scheduler.cpp`).
    let scheduler = std::rc::Rc::new(crate::scheduler::Scheduler::new(
        cmd_tx.clone(),
        tokio::runtime::Handle::current(),
    ));
    // Bind the scheduler to the thread-local so `addEvent`/`stopEvent` Lua closures
    // can reach it. Set once for the game thread's lifetime.
    tfs_rust_lua::set_timer_scheduler(scheduler.clone());

    let (events, chat_channels, weapon_registry, spell_registry, npc_database): (
        Box<dyn crate::event_dispatcher::EventDispatcher>,
        Vec<tfs_rust_lua::ChatChannelDef>,
        tfs_rust_content::weapons::WeaponRegistry,
        tfs_rust_content::spells::SpellRegistry,
        tfs_rust_content::npcs::NpcDatabase,
    ) = match LuaRuntime::new() {
        Ok(mut lua_runtime) => {
            // Load chat channels from Lua scripts (CH-4)
            let chat_channels = match load_chat_channel_scripts(&mut lua_runtime, &data_path) {
                Ok(channels) => {
                    tracing::info!("Loaded {} chat channels from Lua scripts", channels.len());
                    channels
                }
                Err(e) => {
                    tracing::warn!("Chat channel loading failed: {}", e);
                    Vec::new()
                }
            };

            let mut move_events = tfs_rust_lua::MoveEventsRegistry::default();
            if let Err(e) = move_events.load_from_xml(&mut lua_runtime, &data_path) {
                tracing::warn!("Lua movements loading failed: {}", e);
            }

            // CH-6: Load talkaction scripts from `data/scripts/talkactions/god/*.lua`.
            // Self-registering via `TalkAction(words):register()`.
            let talkaction_defs = match load_talkaction_scripts(&mut lua_runtime, &data_path, "god")
            {
                Ok(defs) => {
                    tracing::info!("Loaded {} talkaction definitions from god/", defs.len());
                    defs
                }
                Err(e) => {
                    tracing::warn!("Talkaction loading failed: {}", e);
                    Vec::new()
                }
            };
            let talkactions = crate::talkactions::TalkActionRegistry::from_defs(talkaction_defs);

            // PC-2b/PC-3: Load weapon + spell scripts from `data/scripts/weapons/*.lua`
            // and `data/scripts/spells/**/*.lua`. These drain `_pending_weapons` /
            // `_pending_spells` into `WeaponRegistry` / `SpellRegistry`. Without this,
            // `world.weapons` stays empty and `player_wand_attack` silently no-ops on
            // every wand (the `get_wand()` lookup returns `None` → re-arm without
            // striking). Must run before the runtime is moved into `LuaEventDispatcher`.
            let weapon_registry = match lua_runtime.load_weapon_scripts(&data_path) {
                Ok(reg) => {
                    tracing::info!(
                        wands = reg.wands.len(),
                        distance = reg.distance.len(),
                        melee = reg.melee.len(),
                        "Loaded weapon scripts"
                    );
                    reg
                }
                Err(e) => {
                    tracing::warn!("Weapon script loading failed: {}", e);
                    tfs_rust_content::weapons::WeaponRegistry::default()
                }
            };
            let spell_registry = match lua_runtime.load_spell_scripts(&data_path) {
                Ok(reg) => {
                    tracing::info!(
                        instant = reg.instant_by_words.len(),
                        runes = reg.runes_by_id.len(),
                        "Loaded spell scripts"
                    );
                    reg
                }
                Err(e) => {
                    tracing::warn!("Spell script loading failed: {}", e);
                    tfs_rust_content::spells::SpellRegistry::default()
                }
            };

            // NPC-3: Load declarative NpcType definitions before dispatcher wrap.
            // Hard-fail: unknown spawn names would otherwise place broken NPCs.
            let npc_database =
                match lua_runtime.load_npc_definitions(&data_path, items_db.as_ref()) {
                    Ok(db) => {
                        tracing::info!(types = db.len(), "Loaded NPC definitions");
                        db
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("NPC definition loading failed: {e}"));
                    }
                };

            let mut loader = ScriptLoader::new(&mut lua_runtime);
            let creature_events = match loader.load_creaturescripts(&data_path) {
                Ok(events) => events,
                Err(e) => {
                    tracing::warn!(
                        "Lua creaturescript loading failed (continuing with empty events): {}",
                        e
                    );
                    HashMap::new()
                }
            };
            let player_events = loader.load_player_events(&data_path).unwrap_or_else(|e| {
                tracing::warn!("Lua player events loading failed: {}", e);
                HashMap::new()
            });
            tracing::info!(
                "Lua creaturescripts loaded: login={} logout={} inventory_update={} move_events={}",
                creature_events
                    .get(&tfs_rust_lua::CreatureEventType::Login)
                    .map_or(0, |v| v.len()),
                creature_events
                    .get(&tfs_rust_lua::CreatureEventType::Logout)
                    .map_or(0, |v| v.len()),
                player_events
                    .get(&tfs_rust_lua::PlayerEventType::InventoryUpdate)
                    .map_or(0, |v| v.len()),
                move_events.len(),
            );
            let mut dispatcher = Box::new(LuaEventDispatcher::new(
                lua_runtime,
                creature_events,
                player_events,
                move_events,
            ));
            // CH-6: Inject talkaction registry into the dispatcher.
            // Must be done after `LuaEventDispatcher::new` since the
            // registry holds `mlua::RegistryKey`s tied to the runtime.
            dispatcher.set_talkactions(talkactions);
            (
                dispatcher,
                chat_channels,
                weapon_registry,
                spell_registry,
                npc_database,
            )
        }
        Err(e) => {
            tracing::warn!("Lua runtime init failed, using NullEventDispatcher: {}", e);
            (
                Box::new(NullEventDispatcher),
                Vec::new(),
                tfs_rust_content::weapons::WeaponRegistry::default(),
                tfs_rust_content::spells::SpellRegistry::default(),
                tfs_rust_content::npcs::NpcDatabase::new(),
            )
        }
    };
    let mechanics = crate::formulas::load_mechanics(&data_path, protocol_version);
    info!(profile = ?mechanics.profile, hooks = ?mechanics.hooks, "mechanics profile");
    // Phase 7: both eras run on the single unified beat loop (`run_game_loop`).
    // The 1098 reactive loop (`run_game_loop_1098`) + `walk_wake_tx`/`sleep_until` walk
    // scheduling are deleted. The `beat_driven_loop` flag is collapsed — era differences
    // live in `MechanicsProfile` / `ProtocolCodec` only.
    let mut world = GameWorld::new(
        map,
        items,
        events,
        config,
        db.clone(),
        spawns,
        items_db,
        monsters_db,
        std::sync::Arc::new(npc_database),
        groups,
        vocations,
        codec,
        mechanics,
    );
    world.scheduler = Some(scheduler.clone());

    // PC-2b/PC-3: Inject the weapon + spell registries drained from Lua scripts.
    // `GameWorld::new` initializes these as empty `Arc<WeaponRegistry::default()>` /
    // `Arc<SpellRegistry::default()>`; replace them with the populated registries so
    // `player_wand_attack` (`get_wand`) and spell dispatch (`instant_by_words`) resolve.
    world.weapons = Arc::new(weapon_registry);
    world.spells = Arc::new(spell_registry);

    // Seed chat channels from Lua scripts (CH-4)
    for channel_def in chat_channels {
        use crate::chat::ChatChannel;
        let channel = ChatChannel {
            id: channel_def.id,
            name: channel_def.name,
            public_channel: channel_def.public,
            users: std::collections::HashSet::new(),
        };
        world.chat.add_normal_channel(channel);
    }

    world.startup_spawns();
    info!(
        map_chunks = world.map.grid.chunk_count(),
        map_tiles = world.map.grid.populated_tile_count(),
        tile_stack_item_refs = world.map.grid.tile_stack_item_refs(),
        map_chunk_slots_per_chunk = crate::map::CHUNK_AREA,
        items_slotmap = world.items.len(),
        creatures_slotmap = world.creatures.len(),
        spawn_slots = world.spawns.slots.len(),
        "GameWorld ready — steady-state entity counts (RSS diagnostic; compare to `ps` after load)"
    );

    let out_registry: OutRegistry = Arc::new(Mutex::new(HashMap::new()));
    let shutdown_cmd_tx = cmd_tx.clone();
    let out_for_loop = out_registry.clone();

    // C++ ref: src/otserv.cpp:298-305 (`bindOnlyGlobalAddress` + ip/ports binding behavior)
    let bind_host = if net_cfg.bind_only_global_address {
        net_cfg.ip.as_str()
    } else {
        "0.0.0.0"
    };
    let login_addr = std::env::var("TFS_LOGIN_ADDR")
        .unwrap_or_else(|_| format!("{bind_host}:{}", net_cfg.login_port));
    let game_addr = std::env::var("TFS_GAME_ADDR")
        .unwrap_or_else(|_| format!("{bind_host}:{}", net_cfg.game_port));
    let game_port: u16 = std::env::var("TFS_GAME_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(net_cfg.game_port);

    if net_cfg.status_port != net_cfg.login_port {
        tracing::warn!(
            login_port = net_cfg.login_port,
            status_port = net_cfg.status_port,
            "statusProtocolPort differs from loginProtocolPort; separate status listener is not yet wired"
        );
    }

    let login_listener = TcpListener::bind(&login_addr)
        .await
        .with_context(|| format!("bind login server {login_addr}"))?;
    let game_listener = TcpListener::bind(&game_addr)
        .await
        .with_context(|| format!("bind game server {game_addr}"))?;

    info!(
        login = %login_listener.local_addr()?,
        game = %game_listener.local_addr()?,
        "listening (OTClient: login → login port, game → game port)"
    );
    let adv_ip = std::env::var("TFS_PUBLIC_IP").unwrap_or_else(|_| net_cfg.ip.clone());
    info!(advertise = %format!("{adv_ip}:{game_port}"), "character list game address");

    let server_name = std::env::var("TFS_SERVER_NAME").unwrap_or_else(|_| "Australis".to_string());
    let public_ip = std::env::var("TFS_PUBLIC_IP").unwrap_or_else(|_| net_cfg.ip.clone());
    let motd = std::env::var("TFS_MOTD").unwrap_or_default();
    let motd_num: u32 = std::env::var("TFS_MOTD_NUM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let login_cfg = LoginWireConfig {
        rsa_private_key: rsa_private_key.clone(),
        db: db.clone(),
        password_hash,
        motd_num,
        motd: motd.clone(),
        server_name: server_name.clone(),
        public_ip: public_ip.clone(),
        game_port,
        free_premium: true,
        protocol_version,
        protocol_caps,
    };

    let game_cfg = GameWireConfig {
        cmd_tx: cmd_tx.clone(),
        rsa_private_key,
        db,
        password_hash,
        out_registry,
        motd_num,
        motd,
        server_name,
        public_ip,
        game_port,
        free_premium: true,
        protocol_version,
        protocol_caps,
    };

    // `GameWorld` holds `ConfigManager` → mlua `Lua` (not `Send`); drive the simulation on a `LocalSet`.
    // SIGINT → `GameCommand::Shutdown` → `run_game_loop` flushes online players, then we stop TCP
    // acceptors (C++: `Game::saveGameState` before exit). If the game thread is wedged in a long
    // sync beat (pathfinding storm), Shutdown never drains — force-exit after timeout / 2nd Ctrl+C.
    const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
    let local = LocalSet::new();
    local
        .run_until(async move {
            let force_exit = tokio::spawn(async move {
                match wait_for_shutdown_signal().await {
                    Ok(()) => {
                        if shutdown_cmd_tx.send(GameCommand::Shutdown).is_err() {
                            tracing::warn!("could not send Shutdown — game command channel closed");
                        } else {
                            tracing::info!(
                                "shutdown signal: requested graceful exit (flush runs on game thread)"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(?e, "wait_for_shutdown_signal");
                        return;
                    }
                }
                tokio::select! {
                    second = wait_for_shutdown_signal() => {
                        match second {
                            Ok(()) => tracing::error!(
                                "second Ctrl+C — forcing process exit (game thread may be wedged)"
                            ),
                            Err(e) => tracing::error!(?e, "second wait_for_shutdown_signal"),
                        }
                        std::process::exit(1);
                    }
                    _ = tokio::time::sleep(GRACEFUL_SHUTDOWN_TIMEOUT) => {
                        tracing::error!(
                            timeout_secs = GRACEFUL_SHUTDOWN_TIMEOUT.as_secs(),
                            "graceful shutdown timed out — forcing process exit"
                        );
                        std::process::exit(1);
                    }
                }
            });
            let game_jh = tokio::task::spawn_local(async move {
                // Phase 7: both eras run on the single unified beat loop.
                let loop_result =
                    run_game_loop(world, game_rx, ctrl_rx, cmd_tx, Some(out_for_loop)).await;
                if let Err(e) = loop_result {
                    tracing::error!(?e, "game loop exited with error");
                } else {
                    tracing::info!("game loop finished");
                }
            });
            let mut login_server = Server::from_listener(login_listener);
            let login_jh = tokio::spawn(async move {
                login_server.accept_loop_with_login(login_cfg).await;
            });
            let mut game_server = Server::from_listener(game_listener);
            let game_accept_jh = tokio::spawn(async move {
                game_server.accept_loop_with_game(game_cfg).await;
            });
            if let Err(e) = game_jh.await {
                tracing::error!(?e, "game loop task join error");
            }
            force_exit.abort();
            login_jh.abort();
            game_accept_jh.abort();
        })
        .await;

    Ok(())
}
