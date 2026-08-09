//! Central simulation state: entities, map, managers, DB handle.
//!
//! - `Game` / `Map` ownership — `game.cpp`.
//!   Tick: [`crate::game_world_tick`]. Lifecycle: [`crate::game_world_lifecycle`].
//!   Spectators: [`crate::game_world_spectators`]. Items: [`crate::game_world_item_cylinder`], [`crate::game_world_item_move`].
// C++ reference: `Game` / `Map` ownership in `game.cpp`.

pub use crate::game_world_spectators::{creature_can_see, protocol_can_see};

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use slotmap::SlotMap;
use tfs_rust_content::groups::GroupDatabase;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::monsters::MonsterDatabase;
use tfs_rust_content::npcs::NpcDatabase;
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

/// Queued `MoveEvents` StepOut/StepIn until after move packets.
///
/// C++ `Map::moveCreature` sends `sendCreatureMove` **then** `postRemoveNotification` /
/// StepOut (`map.cpp` ~309–327). Firing StepOut earlier lets `closing_doors.lua`
/// `item:transform` emit `0x6B` while the client still has the creature on that tile
/// — stock 772 asserts / debugs.
#[derive(Clone)]
pub(crate) struct PendingCreatureStepEvent {
    pub cid: CreatureId,
    pub from: Position,
    pub to: Position,
    pub step_out_items: Vec<(ItemId, u16)>,
    pub step_in_items: Vec<(ItemId, u16)>,
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
    /// 772 `CONNECTION_DEAD` — set on player death (`connections.cc` `TConnection::Die`).
    /// Survives `remove_creature` so OK→`CL_CMD_LOGOUT` still finalizes the TCP session
    /// after the body is gone. Cleared on disconnect.
    pub dead_connections: HashSet<ConnId>,
    /// Game-thread only — see [`DeferredTurnBroadcast`].
    pub deferred_turn_broadcast: HashMap<CreatureId, DeferredTurnBroadcast>,
    /// StepOut/StepIn deferred until after move packets — see [`PendingCreatureStepEvent`].
    pub(crate) pending_creature_step_events: Vec<PendingCreatureStepEvent>,
    /// `ProtocolGame::knownCreatureSet` — must persist across `0x64` / move strips (`src/protocolgame.cpp`).
    pub known_creatures_by_conn: HashMap<ConnId, HashSet<u32>>,
    /// Wire ids this conn received with a full `AddCreature` block (map `known=false` or `0x6A`).
    /// Prevents `known=true` short encoding before the client has outfit/name data.
    pub creature_fully_sent_by_conn: HashMap<ConnId, HashSet<u32>>,
    /// OTB + `items.xml` — server item id → client id for map / `addItem` (`src/items.cpp`).
    pub items_db: Arc<ItemDatabase>,
    /// `data/monster/` — spawn instantiation (`monsters.cpp`).
    pub monsters_db: Arc<MonsterDatabase>,
    /// `data/npc/scripts/` — NPC type definitions (`NpcType` Lua / NPC-1).
    pub npcs_db: Arc<NpcDatabase>,
    /// `data/XML/groups.xml` — player GM flags (`src/groups.cpp`).
    pub groups: Arc<GroupDatabase>,
    /// `data/XML/outfits.xml` — change-outfit window / canWear (`src/outfit.cpp`).
    pub outfits_db: Arc<tfs_rust_content::outfits::OutfitDatabase>,
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
    /// C++ `Monster::monsterAutoID` — auto-incrementing wire id for monsters/npcs
    /// (`monster.h:43-46`, `monster.cpp:18`). Starts at `0x40000000`, never reused.
    /// Prevents wire-id collisions when SlotMap slots are recycled.
    pub next_monster_wire_id: u32,
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
    /// Per-world glibc `rand()` stream — sole production RNG for combat / AI / spawn.
    pub(crate) parity_rng: crate::sim_glibc_rand::GlibcRngState,
    /// 772 `RoundNr` — incremented each `Other` subsystem tick (`main.cc:350`).
    pub(crate) round_nr: u32,
    /// Last broadcast ambiente brightness — `AdvanceGame` `OldAmbiente` (`main.cc:323`).
    /// Uses `i16` so `0xFF` (255) does not collide with the `-1` sentinel.
    pub(crate) last_ambiente_brightness: i16,
    /// Manual `setWorldLight(level, color)` override when `defaultWorldLight` is false.
    /// TFS `Game::setWorldLightInfo` (`gameserver/src/luascript.cpp:3132-3145`).
    pub(crate) world_light_override: Option<(u8, u8)>,
    /// True when last beat advance skipped `MoveCreatures` due to lag (`main.cc:449`).
    pub(crate) lag: bool,
    /// Idle-kick / dead-connection disconnects queued from `process_connections`.
    /// `(ConnId, stop_fight)` — idle kick uses `stop_fight=true`, command-timeout uses `false`
    /// (`connections.cc:35-38`).
    pub(crate) pending_idle_kick: Vec<(ConnId, bool)>,
    /// `addEvent` / `stopEvent` scheduler — `None` in tests / when Lua is unavailable.
    /// Game-thread only (`Rc` → `!Send`); used by the game loop to `forget` fired timers.
    pub(crate) scheduler: Option<std::rc::Rc<crate::scheduler::Scheduler>>,
    /// Reusable 772 `TShortway` search buffer — game thread only (`pathfinding.rs`).
    pub(crate) tshortway_scratch: RefCell<crate::pathfinding::TShortwayScratch>,
    /// Cached `config.lua` `itemsDecayInsideDepots` — DEC-4.
    pub(crate) items_decay_inside_depots: bool,
    /// Game-thread scratch `Vec`s — GL-4 / IDLE-3 (reuse capacity across periodic passes).
    pub(crate) scratch_creature_ids: Vec<CreatureId>,
    pub(crate) scratch_stats_dirty: Vec<CreatureId>,
    pub(crate) scratch_pk_marks: Vec<CreatureId>,
    pub(crate) scratch_dead: Vec<CreatureId>,
    pub(crate) scratch_spectators: Vec<CreatureId>,
    /// IDLE-3: temporary buffer for one-floor sector collect before gen-dedup into `scratch_spectators`.
    pub(crate) scratch_sector_buf: Vec<CreatureId>,
    /// IDLE-3: generation-stamped spectator dedup (avoids sort+dedup across Z / old+new).
    pub(crate) scratch_spectator_seen: rustc_hash::FxHashMap<CreatureId, u32>,
    pub(crate) scratch_spectator_gen: u32,
    /// OBS-1: aggregated window histograms / counters (Phase 0).
    pub(crate) obs: crate::obs::GameObs,
    /// TFS `ScriptEnvironment::localMap` — per-script-execution UID → ItemId mapping
    /// for items without `ATTR_UNIQUE_ID` (`luascript.cpp:110-134`). Generated UIDs
    /// start at 65536 (`> u16::MAX`) to distinguish from `ATTR_UNIQUE_ID` lookups.
    /// Not cleared between executions (UIDs are unique; depot items are static map items).
    /// Interior-mutable because `ScriptContext` trait methods receive `&self`.
    pub(crate) script_env_local_map: RefCell<HashMap<u32, ItemId>>,
    /// Reverse index for `script_env_local_map` — O(1) "is this item already registered?".
    pub(crate) script_env_item_to_uid: RefCell<HashMap<ItemId, u32>>,
    pub(crate) script_env_last_uid: Cell<u32>,
}

impl GameWorld {
    /// Logical millisecond clock for subsystem scheduling — always `server_ms` (the beat clock).
    /// Phase 6: the `beat_driven_loop` fork is collapsed; both eras run on the unified beat engine.
    pub(crate) fn now_ms(&self) -> u64 {
        self.server_ms
    }

    /// Active decay/cron clock — `MechanicsProfile::decay_clock` (DEC-3).
    ///
    /// 772: `RoundNr` (`map.cc` `CronCheck`); 1098: movement `server_ms`.
    pub(crate) fn decay_clock_now(&self) -> u64 {
        match self.mechanics.profile.decay_clock {
            crate::formulas::DecayClockModel::ServerMilliseconds => self.server_ms,
            crate::formulas::DecayClockModel::RoundNumber => u64::from(self.round_nr),
        }
    }

    /// Schedule deadline in the active decay clock's units.
    pub(crate) fn decay_schedule_deadline(&self, duration_ms: i32) -> u64 {
        match self.mechanics.profile.decay_clock {
            crate::formulas::DecayClockModel::ServerMilliseconds => self
                .server_ms
                .saturating_add(duration_ms.max(0) as u64),
            crate::formulas::DecayClockModel::RoundNumber => {
                let sec = duration_ms.max(0) as u64 / 1000;
                let sec = sec.max(1);
                u64::from(self.round_nr).saturating_add(sec)
            }
        }
    }

    /// Convert remaining decay clock units to item `duration` milliseconds.
    pub(crate) fn decay_clock_remaining_to_item_ms(&self, remaining: u64) -> u64 {
        match self.mechanics.profile.decay_clock {
            crate::formulas::DecayClockModel::ServerMilliseconds => remaining,
            crate::formulas::DecayClockModel::RoundNumber => remaining.saturating_mul(1000),
        }
    }

    /// Live remaining decay duration in **item milliseconds** (look / save / UI).
    ///
    /// Uses [`Self::decay_clock_now`] — never pass `server_ms` into the heap on 772
    /// (`RoundNumber` deadlines are rounds, not wall-clock ms).
    pub(crate) fn item_decay_remaining_ms(&self, item_id: ItemId) -> Option<u64> {
        let rem = self.decay.remaining_ms(item_id, self.decay_clock_now())?;
        Some(self.decay_clock_remaining_to_item_ms(rem))
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
    /// Not gated by `HasNoExhaustion` (772 Use path has no that right).
    pub(crate) fn player_apply_multiuse_exhaust(&mut self, cid: CreatureId) {
        // Phase 4: 1098 defer deleted — both eras apply multiuse exhaust.
        let now_ms = self.now_ms();
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.base
                .delay_multiuse_ms(now_ms, crate::walk_action::MULTIUSE_EXHAUST_MS);
        }
    }

    /// Apply `EarliestSpellTime` delay (`magic.cc:770–772` `CheckMana`).
    /// Skipped when the player has `HasNoExhaustion` / `NO_EXHAUSTION`.
    pub(crate) fn player_apply_spell_exhaust_ms(&mut self, cid: CreatureId, delay_ms: u64) {
        if delay_ms == 0 {
            return;
        }
        if self.player_has_flag(cid, crate::player_flags::PLAYER_FLAG_HAS_NO_EXHAUSTION) {
            return;
        }
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
        npcs_db: Arc<NpcDatabase>,
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
        let items_decay_inside_depots = crate::config::get_bool_or(
            config.as_ref(),
            "itemsDecayInsideDepots",
            false,
        )
        .unwrap_or(false);
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
            dead_connections: HashSet::new(),
            deferred_turn_broadcast: HashMap::new(),
            pending_creature_step_events: Vec::new(),
            known_creatures_by_conn: HashMap::new(),
            creature_fully_sent_by_conn: HashMap::new(),
            items_db,
            monsters_db,
            npcs_db,
            groups,
            outfits_db: Arc::new(tfs_rust_content::outfits::OutfitDatabase::default()),
            vocations,
            spells: Arc::new(tfs_rust_content::spells::SpellRegistry::default()),
            weapons: Arc::new(tfs_rust_content::weapons::WeaponRegistry::default()),
            next_statement_id: 0,
            next_monster_wire_id: 0x4000_0000,
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
            parity_rng: crate::sim_glibc_rand::GlibcRngState::default(),
            round_nr: 0,
            last_ambiente_brightness: -1,
            world_light_override: None,
            lag: false,
            pending_idle_kick: Vec::new(),
            scheduler: None,
            tshortway_scratch: RefCell::new(crate::pathfinding::TShortwayScratch::new()),
            items_decay_inside_depots,
            scratch_creature_ids: Vec::new(),
            scratch_stats_dirty: Vec::new(),
            scratch_pk_marks: Vec::new(),
            scratch_dead: Vec::new(),
            scratch_spectators: Vec::new(),
            scratch_sector_buf: Vec::new(),
            scratch_spectator_seen: rustc_hash::FxHashMap::default(),
            scratch_spectator_gen: 0,
            obs: crate::obs::GameObs::new(),
            script_env_local_map: RefCell::new(HashMap::new()),
            script_env_item_to_uid: RefCell::new(HashMap::new()),
            script_env_last_uid: Cell::new(65536),
        }
    }

    /// IDLE-3: bump spectator-seen generation; clear map on wrap to avoid stale hits.
    pub(crate) fn bump_spectator_gen(&mut self) -> u32 {
        self.scratch_spectator_gen = self.scratch_spectator_gen.wrapping_add(1);
        if self.scratch_spectator_gen == 0 {
            self.scratch_spectator_seen.clear();
            self.scratch_spectator_gen = 1;
        }
        self.scratch_spectator_gen
    }

    /// IDLE-3: mark `id` seen for `spectator_gen`; returns `true` on first sight this generation.
    pub(crate) fn spectator_mark_new(&mut self, id: CreatureId, spectator_gen: u32) -> bool {
        let entry = self.scratch_spectator_seen.entry(id).or_insert(0);
        if *entry == spectator_gen {
            false
        } else {
            *entry = spectator_gen;
            true
        }
    }

    /// OBS-1: record game-lane commands processed in one loop turn.
    pub(crate) fn obs_record_commands(&mut self, count: usize) {
        self.obs.record_commands_processed(count);
    }

    /// OBS-1: emit summary if the aggregation window elapsed.
    pub(crate) fn obs_maybe_emit(&mut self) {
        self.obs.maybe_emit(std::time::Instant::now());
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

    /// Re-seed [`Self::parity_rng`] when `TFS_SIM_SEED` is set (headless parity harness).
    pub fn init_sim_rng_from_env(&mut self) {
        if let Ok(seed_str) = std::env::var("TFS_SIM_SEED") {
            if let Ok(seed) = seed_str.parse::<u64>() {
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

    /// Deterministic parity stream for unit tests and live production.
    pub fn seed_parity_rng(&mut self, seed: u32) {
        self.parity_rng = crate::sim_glibc_rand::GlibcRngState::seed(seed);
    }

    /// Inclusive random on the per-world glibc stream (sim harness overrides when enabled).
    pub(crate) fn parity_random(&self, min: i32, max: i32) -> i32 {
        #[cfg(any(test, feature = "sim"))]
        if crate::sim_glibc_rand::sim_glibc_rng_enabled() {
            return crate::sim_glibc_rand::sim_random(min, max);
        }
        self.parity_rng.random(min, max)
    }

    /// Modulo roll on the per-world glibc stream (sim harness overrides when enabled).
    pub(crate) fn parity_rand_mod(&self, modulus: u32) -> u32 {
        #[cfg(any(test, feature = "sim"))]
        if crate::sim_glibc_rand::sim_glibc_rng_enabled() {
            return crate::sim_glibc_rand::sim_rand_mod(modulus);
        }
        self.parity_rng.rand_mod(modulus)
    }

    /// Forward Fisher-Yates shuffle on the per-world glibc stream.
    #[allow(dead_code)]
    pub(crate) fn parity_random_shuffle<T>(&self, buf: &mut [T]) {
        #[cfg(any(test, feature = "sim"))]
        if crate::sim_glibc_rand::sim_glibc_rng_enabled() {
            crate::sim_glibc_rand::parity_random_shuffle(buf);
            return;
        }
        self.parity_rng.random_shuffle(buf);
    }

    /// Dance sidestep roll — `%5` on the unified glibc stream.
    pub(crate) fn sim_dance_choice(&mut self) -> u32 {
        self.parity_rand_mod(5)
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
