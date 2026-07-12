//! Central simulation state: entities, map, managers, DB handle.
//!
//! - `Game` / `Map` ownership — `game.cpp`.
//!   Tick: [`crate::game_world_tick`]. Lifecycle: [`crate::game_world_lifecycle`].
//!   Spectators: [`crate::game_world_spectators`]. Items: [`crate::game_world_item_cylinder`], [`crate::game_world_item_move`].
// C++ reference: `Game` / `Map` ownership in `game.cpp`.

pub use crate::game_world_spectators::{creature_can_see, protocol_can_see};

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use rand::rngs::StdRng;
use rand::SeedableRng;
use slotmap::SlotMap;
use tfs_rust_content::groups::GroupDatabase;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::monsters::MonsterDatabase;
use tfs_rust_content::vocations::VocationRegistry;

use tfs_rust_common::enums::Direction;
use tfs_rust_common::ConnId;
use tfs_rust_common::GamePacket;
use tfs_rust_common::Position;
use tfs_rust_db::DbPool;
use tfs_rust_net::Codec;

use crate::chat::ChatRegistry;
use crate::config::ConfigManager;
use crate::container::ContainerRegistry;
use crate::creature::CreatureKind;
use crate::decay::DecayManager;
use crate::event_dispatcher::EventDispatcher;
use crate::guild::GuildRegistry;
use crate::house::HouseManager;
use crate::ids::{CreatureId, ItemId};
use crate::item::Item;
use crate::map::Map;
use crate::party::{Party, PartyInviteState};
use crate::protocol_hooks::{NullProtocolHooks, SharedProtocolHooks};
use crate::spawn::SpawnManager;
use crate::stability::StabilityManager;
use crate::wildcard::WildcardTree;

/// Pending `0x6B` from `player_turn_request` — flushed or dropped after the next command is known
/// (`tasks/walk-smoothness-audit.md` Bug 7: coalesce Turn + Move / skip stale facing before walk).
#[derive(Clone)]
pub struct DeferredTurnBroadcast {
    pub guid: u32,
    pub pos: Position,
    pub stack_u8: u8,
    pub dir: Direction,
}

pub struct GameWorld {
    pub creatures: SlotMap<CreatureId, CreatureKind>,
    pub items: SlotMap<ItemId, Item>,
    pub map: Map,
    pub events: Box<dyn EventDispatcher>,
    /// Game-thread-only: holds an `mlua::Lua` (`!Send`), so `Rc` not `Arc`.
    pub config: Rc<ConfigManager>,
    pub db: DbPool,
    /// GAME THREAD ONLY — insert/remove from IO threads must not be added without review.
    pub player_by_name: HashMap<String, CreatureId>,
    /// GAME THREAD ONLY — paired with `player_by_name`.
    pub player_by_guid: HashMap<u32, CreatureId>,
    pub guilds: GuildRegistry,
    pub parties: HashMap<u32, Party>,
    pub party_invites: PartyInviteState,
    pub next_party_id: u32,
    /// TFS `Chat` (`chat.h:105`) — static + private channel registry. Game-thread only.
    /// CH-1: skeleton only (SAY does not touch channels); CH-4 seeds static channels
    /// from `data/scripts/chatchannels/*.lua` and adds membership/lookup methods.
    pub chat: ChatRegistry,
    /// C++ `Player::muteCountMap` — flood protection escalation (player guid → mute count).
    /// Game-thread only; static in C++ (`player.cpp:112`), per-world here for test isolation.
    pub mute_count_map: HashMap<u32, u32>,
    pub decay: DecayManager,
    pub spawns: SpawnManager,
    pub houses: HouseManager,
    pub wildcards: WildcardTree,
    pub stability: StabilityManager,
    pub tick_counter: u64,
    /// Per-connection outgoing payloads queued on the game thread; drained each tick (`flush_output_buffers`).
    pub pending_outgoing: HashMap<ConnId, Vec<Vec<u8>>>,
    /// Extended opcode + async Lua result hooks (Phase 8: Lua `PacketHandler`).
    pub protocol_hooks: SharedProtocolHooks,
    /// Wire encoder for `clientVersion` (GAME THREAD ONLY — Phase A1 codec seam).
    pub codec: Codec,
    /// Era-tuned mechanics knobs + Tier-2 Lua formula hooks (GAME THREAD ONLY — Track B §12.11/§12.13).
    pub mechanics: crate::formulas::Mechanics,
    /// TCP connection → logged-in player (`conn_id` from `tfs-rust-net`).
    pub conn_to_creature: HashMap<ConnId, CreatureId>,
    /// Reverse index of [`Self::conn_to_creature`] — `CreatureId → ConnId`.
    /// Maintained by [`Self::register_conn_mapping`] / [`Self::unregister_conn_mapping`]
    /// so spatial fan-out (`spectator_conns_via_grid`, audit #4) can resolve a creature's
    /// connection in O(1) instead of scanning all online players.
    pub creature_to_conn: HashMap<CreatureId, ConnId>,
    /// Game-thread only — see [`DeferredTurnBroadcast`].
    pub deferred_turn_broadcast: HashMap<CreatureId, DeferredTurnBroadcast>,
    /// `ProtocolGame::knownCreatureSet` — must persist across `0x64` / move strips (`src/protocolgame.cpp`).
    pub known_creatures_by_conn: HashMap<ConnId, HashSet<u32>>,
    /// Wire ids this conn received with a full `AddCreature` block (map `known=false` or `0x6A`).
    /// Prevents `known=true` short encoding before the client has outfit/name data.
    pub creature_fully_sent_by_conn: HashMap<ConnId, HashSet<u32>>,
    /// OTB + `items.xml` — server item id → client id for map / `addItem` (`src/items.cpp`).
    pub items_db: Arc<ItemDatabase>,
    /// `data/monster/` — spawn instantiation (`monsters.cpp`).
    pub monsters_db: Arc<MonsterDatabase>,
    /// `data/XML/groups.xml` — player GM flags (`src/groups.cpp`).
    pub groups: Arc<GroupDatabase>,
    pub vocations: Arc<VocationRegistry>,
    /// PC-2b: spell definitions loaded from `data/scripts/spells/**/*.lua` via the
    /// TFS Lua `Spell(SPELL_INSTANT|SPELL_RUNE)` API. Used by `player_say_spell`
    /// for spellword dispatch (`Say` → `onCastSpell`).
    pub spells: Arc<tfs_rust_content::spells::SpellRegistry>,
    /// PC-2b: weapon definitions loaded from `data/scripts/weapons/*.lua` via the
    /// TFS Lua `Weapon(WEAPON_*)` API. Used by PC-3 (wand/distance strikes).
    pub weapons: Arc<tfs_rust_content::weapons::WeaponRegistry>,
    /// C++ `ProtocolGame::sendCreatureSay` static `statementId` (`src/protocolgame.cpp` ~2432).
    pub next_statement_id: u32,
    /// 772 global action scheduler (`crmain.cc` `MoveCreatures`).
    pub(crate) todo_queue: crate::todo_queue::ToDoQueue,
    /// Logical game clock — advanced in `beat_ms` steps on the beat loop (`crmain.cc` `ServerMilliseconds`).
    pub(crate) server_ms: u64,
    /// TFS `Game::ReleaseCreature` → `ToReleaseCreatures` (`src/game.cpp` ~4766–4768), drained in [`Self::cleanup`].
    pub(crate) creatures_pending_release: Vec<CreatureId>,
    /// TFS `Game::ReleaseItem` → `ToReleaseItems` (`src/game.cpp` ~4771–4773).
    pub(crate) items_pending_release: Vec<ItemId>,
    /// Open bags / loaded `player_items` containers — `container.h` / `player.cpp`.
    pub container_registry: ContainerRegistry,
    /// Reverse link spawn slot ↔ creature for respawn scheduling.
    pub(crate) spawn_slot_by_creature: HashMap<CreatureId, usize>,
    /// 772 `AdvanceGame` staggered ~1000 ms subsystem counters (772 loop only).
    pub(crate) subsystem_counters: crate::subsystem_counters::SubsystemCounters,
    /// Monster despawn / walk-back radii from `config.lua` (`configmanager.cpp`).
    pub monster_world_config: crate::config::MonsterWorldConfig,
    /// Connection idle/timeout settings from `config.lua` (`kickIdlePlayerAfterMinutes`).
    pub connection_config: crate::config::ConnectionConfig,
    /// Chat / yell settings from `config.lua` (`yellMinimumLevel`, `yellAlwaysAllowPremium`).
    pub chat_config: crate::config::ChatConfig,
    /// PvP world settings from `config.lua` (`worldType`, `protectionLevel`) — PC-4.
    pub pvp_config: crate::config::PvpConfig,
    /// Nesting depth for [`crate::monster_events::GameWorld::monster_notify_creature_enter_viewport`]
    /// (login fan-out). Suppresses synchronous chase acquire on idle-wake while > 0.
    pub(crate) monster_viewport_notify_depth: u32,
    /// AI/combat RNG — re-seeded from `TFS_SIM_SEED` in harness runs (`crnonpl.cc` dance/attack rolls).
    pub(crate) ai_rng: StdRng,
    /// Per-world glibc parity stream for 772 — avoids process-global `libc::srand` (Finding 8/15).
    pub(crate) parity_rng: crate::sim_glibc_rand::GlibcRngState,
    /// 772 `RoundNr` — incremented each `Other` subsystem tick (`main.cc:350`).
    pub(crate) round_nr: u32,
    /// Last broadcast ambiente brightness — `AdvanceGame` `OldAmbiente` (`main.cc:323`).
    pub(crate) last_ambiente_brightness: i8,
    /// True when last beat advance skipped `MoveCreatures` due to lag (`main.cc:449`).
    pub(crate) lag: bool,
    /// Idle-kick disconnects queued from `process_connections` — drained by the 772 game loop.
    pub(crate) pending_idle_kick: Vec<ConnId>,
    /// `addEvent` / `stopEvent` scheduler — `None` in tests / when Lua is unavailable.
    /// Game-thread only (`Rc` → `!Send`); used by the game loop to `forget` fired timers.
    pub(crate) scheduler: Option<std::rc::Rc<crate::scheduler::Scheduler>>,
}

impl GameWorld {
    /// Logical millisecond clock for subsystem scheduling — always `server_ms` (the beat clock).
    /// Phase 6: the `beat_driven_loop` fork is collapsed; both eras run on the unified beat engine.
    pub(crate) fn now_ms(&self) -> u64 {
        self.server_ms
    }

    pub fn player_timed_action_ready(&self, cid: CreatureId) -> bool {
        let now_ms = self.now_ms();
        match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.timed_action_ready(now_ms),
            _ => true,
        }
    }

    /// Per-packet action gate — 772 `Earliest*Time` (Phase 4: both eras).
    /// C++ ref: `cract.cc:906–940` `CalculateDelay`; `crmain.cc:924` combat gates.
    pub fn player_packet_action_ready(&self, cid: CreatureId, packet: &GamePacket) -> bool {
        let now_ms = self.now_ms();
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return true;
        };
        // Phase 4: 1098 `nextAction` gate deleted — both eras use `Earliest*Time`.
        let base = &p.base;
        match packet {
            GamePacket::Attack { .. } => {
                base.attack_ready_at(now_ms, base.earliest_spell_server_ms)
            }
            // F8 S6 — `Throw`/`UseItem`/`UseItemEx`/`RotateItem` no longer reach this gate
            // (excluded from `game_packet_requires_timed_action`); their timing is owned by
            // the ToDo engine (`Wait{100}` + `CalculateDelay`). The `_` arm covers any
            // remaining gated opcode (e.g. future combat-adjacent packets).
            _ => base.attack_ready_at(now_ms, base.earliest_spell_server_ms),
        }
    }

    /// C++ `Use` two-object exhaustion — `cract.cc:765`.
    pub(crate) fn player_apply_multiuse_exhaust(&mut self, cid: CreatureId) {
        // Phase 4: 1098 defer deleted — both eras apply multiuse exhaust.
        let now_ms = self.now_ms();
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.base
                .delay_multiuse_ms(now_ms, crate::walk_action::MULTIUSE_EXHAUST_MS);
        }
    }

    /// C++ `CheckMana` spell exhaustion — `magic.cc:770–772` (2000 ms default world).
    #[allow(dead_code)]
    pub(crate) fn player_apply_spell_exhaust(&mut self, cid: CreatureId, delay_ms: u64) {
        // Phase 4: 1098 defer deleted — both eras apply spell exhaust.
        let now_ms = self.now_ms();
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.base.delay_spell_ms(now_ms, delay_ms);
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        map: Map,
        items: SlotMap<ItemId, Item>,
        events: Box<dyn EventDispatcher>,
        config: Rc<ConfigManager>,
        db: DbPool,
        spawns: SpawnManager,
        items_db: Arc<ItemDatabase>,
        monsters_db: Arc<MonsterDatabase>,
        groups: Arc<GroupDatabase>,
        vocations: Arc<VocationRegistry>,
        codec: Codec,
        mechanics: crate::formulas::Mechanics,
    ) -> Self {
        let monster_world_config = crate::config::MonsterWorldConfig::from_config(config.as_ref())
            .unwrap_or_else(|_| crate::config::MonsterWorldConfig::defaults());
        let connection_config = crate::config::ConnectionConfig::from_config(config.as_ref())
            .unwrap_or_else(|_| crate::config::ConnectionConfig::defaults());
        let chat_config = crate::config::ChatConfig::from_config(config.as_ref())
            .unwrap_or_else(|_| crate::config::ChatConfig::defaults());
        let pvp_config = crate::config::PvpConfig::from_config(config.as_ref())
            .unwrap_or_else(|_| crate::config::PvpConfig::defaults());
        Self {
            creatures: SlotMap::with_key(),
            items,
            map,
            events,
            config,
            db,
            player_by_name: HashMap::new(),
            player_by_guid: HashMap::new(),
            guilds: GuildRegistry::default(),
            parties: HashMap::new(),
            party_invites: PartyInviteState::default(),
            next_party_id: 1,
            decay: DecayManager::default(),
            chat: ChatRegistry::new(),
            mute_count_map: HashMap::new(),
            spawns,
            houses: HouseManager::default(),
            wildcards: WildcardTree::default(),
            stability: StabilityManager::default(),
            tick_counter: 0,
            pending_outgoing: HashMap::new(),
            protocol_hooks: Arc::new(NullProtocolHooks),
            codec,
            mechanics,
            conn_to_creature: HashMap::new(),
            creature_to_conn: HashMap::new(),
            deferred_turn_broadcast: HashMap::new(),
            known_creatures_by_conn: HashMap::new(),
            creature_fully_sent_by_conn: HashMap::new(),
            items_db,
            monsters_db,
            groups,
            vocations,
            spells: Arc::new(tfs_rust_content::spells::SpellRegistry::default()),
            weapons: Arc::new(tfs_rust_content::weapons::WeaponRegistry::default()),
            next_statement_id: 0,
            todo_queue: crate::todo_queue::ToDoQueue::default(),
            server_ms: 0,
            creatures_pending_release: Vec::new(),
            items_pending_release: Vec::new(),
            container_registry: ContainerRegistry::new(),
            spawn_slot_by_creature: HashMap::new(),
            subsystem_counters: crate::subsystem_counters::SubsystemCounters::default(),
            monster_world_config,
            connection_config,
            chat_config,
            pvp_config,
            monster_viewport_notify_depth: 0,
            ai_rng: StdRng::from_entropy(),
            parity_rng: crate::sim_glibc_rand::GlibcRngState::default(),
            round_nr: 0,
            last_ambiente_brightness: -1,
            lag: false,
            pending_idle_kick: Vec::new(),
            scheduler: None,
        }
    }

    /// Millisecond clock for chase JSONL — matches C++ `ServerMilliseconds` in `chase_path_debug.cc`.
    #[inline]
    pub fn chase_trace_tick(&self) -> u64 {
        self.server_ms
    }

    /// Re-seed glibc `rand()` after spawn loot — chase harness idle/combat parity.
    pub fn resync_sim_glibc_rng(&mut self) {
        #[cfg(any(test, feature = "sim"))]
        crate::sim_glibc_rand::resync_harness_glibc_rng_from_env();
        #[cfg(not(any(test, feature = "sim")))]
        {
            // No-op in production builds — sim harness not compiled.
        }
    }

    /// Re-seed [`Self::ai_rng`] when `TFS_SIM_SEED` is set (headless parity harness).
    pub fn init_sim_rng_from_env(&mut self) {
        if let Ok(seed_str) = std::env::var("TFS_SIM_SEED") {
            if let Ok(seed) = seed_str.parse::<u64>() {
                self.ai_rng = StdRng::seed_from_u64(seed);
                self.parity_rng = crate::sim_glibc_rand::GlibcRngState::seed(seed as u32);
                // C++ `srand(TFS_SIM_SEED)` — legacy harness global stream (sim only).
                #[cfg(any(test, feature = "sim"))]
                {
                    unsafe { libc::srand(seed as u32) };
                    crate::sim_glibc_rand::enable_sim_glibc_rng();
                }
            }
        }
    }

    /// Deterministic parity stream for unit tests and live 772 (`Finding 8/15`).
    pub fn seed_parity_rng(&mut self, seed: u32) {
        self.parity_rng = crate::sim_glibc_rand::GlibcRngState::seed(seed);
    }

    /// Inclusive random on the era-appropriate stream — K1 profile knob.
    /// 772 (`PerWorldGlibc`) uses per-world glibc state; 1098 (`EnvGlobal`) uses env/global.
    pub(crate) fn parity_random(&self, min: i32, max: i32) -> i32 {
        if self.mechanics.profile.parity_rng_source
            == crate::formulas::ParityRngSource::PerWorldGlibc
        {
            self.parity_rng.random(min, max)
        } else {
            crate::sim_glibc_rand::parity_random(min, max)
        }
    }

    /// Modulo roll on the era-appropriate stream — K1 profile knob.
    pub(crate) fn parity_rand_mod(&self, modulus: u32) -> u32 {
        if self.mechanics.profile.parity_rng_source
            == crate::formulas::ParityRngSource::PerWorldGlibc
        {
            self.parity_rng.rand_mod(modulus)
        } else {
            crate::sim_glibc_rand::parity_rand_mod(modulus)
        }
    }

    /// Forward Fisher-Yates shuffle on the era-appropriate parity stream — K1 profile knob.
    #[allow(dead_code)]
    pub(crate) fn parity_random_shuffle<T>(&self, buf: &mut [T]) {
        if self.mechanics.profile.parity_rng_source
            == crate::formulas::ParityRngSource::PerWorldGlibc
        {
            self.parity_rng.random_shuffle(buf);
        } else {
            crate::sim_glibc_rand::parity_random_shuffle(buf);
        }
    }

    /// Dance / harness rolls — K1: per-world glibc on 772; env/global or [`Self::ai_rng`] on 1098.
    pub(crate) fn sim_dance_choice(&mut self) -> u32 {
        if self.mechanics.profile.parity_rng_source
            == crate::formulas::ParityRngSource::PerWorldGlibc
        {
            self.parity_rand_mod(5)
        } else {
            #[cfg(any(test, feature = "sim"))]
            if crate::sim_glibc_rand::sim_glibc_rng_enabled() {
                return crate::sim_glibc_rand::sim_rand_mod(5);
            }
            use rand::Rng;
            self.ai_rng.gen_range(0..5)
        }
    }

    pub(crate) fn tile_ground_speed(&self, body: &crate::tile::TileBody) -> u32 {
        match body.ground {
            Some(gid) => self.items_db.ground_speed_for_item(gid),
            None => 150,
        }
    }
    pub fn set_protocol_hooks(&mut self, hooks: SharedProtocolHooks) {
        self.protocol_hooks = hooks;
    }
}
