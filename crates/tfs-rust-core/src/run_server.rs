//! Integrated TCP login + game + `GameWorld` + `run_game_loop` (OTBM + DB + outbound flush).
// C++ reference: `src/otserv.cpp` startup — load map, start game thread, accept connections.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::LocalSet;
use tracing::info;

use tfs_rust_common::GameCommand;
use tfs_rust_net::{GameWireConfig, LoginWireConfig, OutRegistry, Server};

use crate::config::ConfigManager;
use crate::event_dispatcher::NullEventDispatcher;
use crate::game_loop::run_game_loop;
use crate::game_world::GameWorld;
use crate::map::Map;
use crate::spawn::SpawnManager;

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
/// Env: `DATABASE_URL` (required), `TFS_DATA_DIR` (default `data` — use repo `data/` from project root), `TFS_MAP_OTBM` (default `world/forgotten.otbm`),
/// `TFS_CONFIG` (default `config.lua`),
/// `TFS_LOGIN_ADDR` / `TFS_GAME_ADDR` / `TFS_PUBLIC_IP` / `TFS_GAME_PORT` / `TFS_SERVER_NAME` / `TFS_MOTD` / `TFS_MOTD_NUM`.
pub async fn run() -> anyhow::Result<()> {
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

    let database_url = std::env::var("DATABASE_URL")
        .context("set DATABASE_URL (MariaDB) for authentication")?;
    let db = tfs_rust_db::DbPool::connect(&database_url, 1, 5)
        .await
        .map_err(|e| anyhow::anyhow!("database connect: {e}"))?;

    let data_dir = std::env::var("TFS_DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let data_path = PathBuf::from(&data_dir);
    let map_rel = std::env::var("TFS_MAP_OTBM").unwrap_or_else(|_| "world/forgotten.otbm".to_string());
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
    let map = Map::from_map_data(content.map, items_db.as_ref());
    let spawns = SpawnManager::from_zones(spawn_zones);
    let vocations = std::sync::Arc::new(content.vocations);

    let config_path = std::env::var("TFS_CONFIG").unwrap_or_else(|_| "config.lua".to_string());
    let config = Arc::new(
        ConfigManager::load(Path::new(&config_path)).map_err(|e| anyhow::anyhow!("config: {e}"))?,
    );

    let events: Box<dyn crate::event_dispatcher::EventDispatcher> = Box::new(NullEventDispatcher);
    let world = GameWorld::new(map, events, config, db.clone(), spawns, items_db, vocations);
    info!("GameWorld ready (map + spawns)");

    let out_registry: OutRegistry = Arc::new(Mutex::new(HashMap::new()));
    let (cmd_tx, cmd_rx) = mpsc::channel::<GameCommand>(256);
    let out_for_loop = out_registry.clone();

    let login_addr =
        std::env::var("TFS_LOGIN_ADDR").unwrap_or_else(|_| "127.0.0.1:7171".to_string());
    let game_addr = std::env::var("TFS_GAME_ADDR").unwrap_or_else(|_| "127.0.0.1:7172".to_string());
    let game_port: u16 = std::env::var("TFS_GAME_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7172);

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
    let adv_ip = std::env::var("TFS_PUBLIC_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
    info!(advertise = %format!("{adv_ip}:{game_port}"), "character list game address");

    let server_name =
        std::env::var("TFS_SERVER_NAME").unwrap_or_else(|_| "Australis".to_string());
    let public_ip = std::env::var("TFS_PUBLIC_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
    let motd = std::env::var("TFS_MOTD").unwrap_or_default();
    let motd_num: u32 = std::env::var("TFS_MOTD_NUM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let login_cfg = LoginWireConfig {
        rsa_private_key: rsa_private_key.clone(),
        db: db.clone(),
        motd_num,
        motd: motd.clone(),
        server_name: server_name.clone(),
        public_ip: public_ip.clone(),
        game_port,
        free_premium: true,
    };

    let game_cfg = GameWireConfig {
        cmd_tx,
        rsa_private_key,
        db,
        out_registry,
        motd_num,
        motd,
        server_name,
        public_ip,
        game_port,
        free_premium: true,
    };

    // `GameWorld` holds `ConfigManager` → mlua `Lua` (not `Send`); drive the simulation on a `LocalSet`.
    let local = LocalSet::new();
    local
        .run_until(async move {
            tokio::task::spawn_local(async move {
                if let Err(e) = run_game_loop(world, cmd_rx, Some(out_for_loop)).await {
                    tracing::error!(?e, "game loop exited");
                }
            });
            tokio::spawn(async move {
                let mut login_server = Server::from_listener(login_listener);
                login_server.accept_loop_with_login(login_cfg).await;
            });
            let mut game_server = Server::from_listener(game_listener);
            game_server.accept_loop_with_game(game_cfg).await;
        })
        .await;

    Ok(())
}
