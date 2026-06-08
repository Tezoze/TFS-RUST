//! Central simulation state: entities, map, managers, DB handle.
//!
//! - `Game` / `Map` ownership — `game.cpp`.
//! Tick: [`crate::game_world_tick`]. Lifecycle: [`crate::game_world_lifecycle`].
//! Spectators: [`crate::game_world_spectators`]. Items: [`crate::game_world_item_cylinder`], [`crate::game_world_item_move`].
// C++ reference: `Game` / `Map` ownership in `game.cpp`.

pub use crate::game_world_spectators::{creature_can_see, protocol_can_see};

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc::UnboundedSender;
use tfs_rust_content::groups::GroupDatabase;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::monsters::MonsterDatabase;
use tfs_rust_content::vocations::VocationDatabase;
use slotmap::SlotMap;

use tfs_rust_common::ConnId;
use tfs_rust_common::enums::Direction;
use tfs_rust_common::Position;
use tfs_rust_db::DbPool;
use tfs_rust_net::Codec;

use crate::config::ConfigManager;
use crate::creature::CreatureKind;
use crate::decay::DecayManager;
use crate::event_dispatcher::EventDispatcher;
use crate::container::ContainerRegistry;
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
    pub vocations: Arc<VocationDatabase>,
    /// C++ `ProtocolGame::sendCreatureSay` static `statementId` (`src/protocolgame.cpp` ~2432).
    pub next_statement_id: u32,
    /// When set, walk wake uses Tokio one-shot timers (`src/scheduler.cpp`); `None` falls back to polling in `process_walk_deadlines`.
    pub(crate) walk_wake_tx: Option<UnboundedSender<CreatureId>>,
    /// 772 global action scheduler (`crmain.cc` `MoveCreatures`).
    pub(crate) todo_queue: crate::todo_queue::ToDoQueue,
    /// Logical game clock — advanced in `beat_ms` steps on the 772 loop (`crmain.cc` `ServerMilliseconds`).
    pub(crate) server_ms: u64,
    /// True when `StepSpeedModel::LinearGo` — beat-driven loop + ToDoQueue walk scheduling.
    pub(crate) beat_driven_loop: bool,
    /// TFS `Game::ReleaseCreature` → `ToReleaseCreatures` (`src/game.cpp` ~4766–4768), drained in [`Self::cleanup`].
    pub(crate) creatures_pending_release: Vec<CreatureId>,
    /// TFS `Game::ReleaseItem` → `ToReleaseItems` (`src/game.cpp` ~4771–4773).
    pub(crate) items_pending_release: Vec<ItemId>,
    /// Open bags / loaded `player_items` containers — `container.h` / `player.cpp`.
    pub container_registry: ContainerRegistry,
    /// Next bucket index for TFS staggered `Game::checkCreatures` (`game.cpp` ~3819).
    pub(crate) check_creature_bucket_index: u32,
    /// Wall-clock of last per-bucket think tick (`EVENT_CHECK_CREATURE_INTERVAL`).
    pub(crate) last_creature_bucket_tick: Option<Instant>,
    /// Reverse link spawn slot ↔ creature for respawn scheduling.
    pub(crate) spawn_slot_by_creature: HashMap<CreatureId, usize>,
    /// 772 `AdvanceGame` staggered ~1000 ms subsystem counters (772 loop only).
    pub(crate) subsystem_counters_772: crate::subsystem_counters_772::SubsystemCounters772,
    /// Monster despawn / walk-back radii from `config.lua` (`configmanager.cpp`).
    pub monster_world_config: crate::config::MonsterWorldConfig,
    /// Nesting depth for [`crate::monster_events::GameWorld::monster_notify_creature_enter_viewport`]
    /// (login fan-out). Suppresses synchronous chase acquire on idle-wake while > 0.
    pub(crate) monster_viewport_notify_depth: u32,
}

impl GameWorld {
    pub fn player_timed_action_ready(&self, cid: CreatureId, now: Instant) -> bool {
        match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.timed_action_ready(now),
            _ => true,
        }
    }
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
        vocations: Arc<VocationDatabase>,
        walk_wake_tx: Option<UnboundedSender<CreatureId>>,
        codec: Codec,
        mechanics: crate::formulas::Mechanics,
    ) -> Self {
        let monster_world_config = crate::config::MonsterWorldConfig::from_config(config.as_ref())
            .unwrap_or_else(|_| crate::config::MonsterWorldConfig::defaults());
        let beat_driven_loop =
            mechanics.profile.step_speed == crate::formulas::StepSpeedModel::LinearGo;
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
            deferred_turn_broadcast: HashMap::new(),
            known_creatures_by_conn: HashMap::new(),
            creature_fully_sent_by_conn: HashMap::new(),
            items_db,
            monsters_db,
            groups,
            vocations,
            next_statement_id: 0,
            walk_wake_tx,
            todo_queue: crate::todo_queue::ToDoQueue::default(),
            server_ms: 0,
            beat_driven_loop,
            creatures_pending_release: Vec::new(),
            items_pending_release: Vec::new(),
            container_registry: ContainerRegistry::new(),
            check_creature_bucket_index: 0,
            last_creature_bucket_tick: None,
            spawn_slot_by_creature: HashMap::new(),
            subsystem_counters_772: crate::subsystem_counters_772::SubsystemCounters772::default(),
            monster_world_config,
            monster_viewport_notify_depth: 0,
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
