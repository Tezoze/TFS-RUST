//! Central simulation state: entities, map, managers, DB handle.
// C++ reference: `Game` / `Map` ownership in `game.cpp`.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc::UnboundedSender;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::vocations::VocationDatabase;
use slotmap::{Key, SlotMap};

use tfs_rust_common::ConnId;
use tfs_rust_common::enums::{ConditionType, Direction};
use tfs_rust_common::protocol_constants::{MAX_CLIENT_VIEWPORT_X, MAX_CLIENT_VIEWPORT_Y};
use tfs_rust_common::Position;
use tfs_rust_db::DbPool;
use tfs_rust_net::outgoing_extra::{send_creature_say, send_player_stats_1098, PlayerStats1098};

use crate::config::ConfigManager;
use crate::condition::ActiveCondition;
use crate::container_ui::ContainerContentChange;
use crate::creature::PlayerWalkAction;
use crate::creature::CreatureKind;
use crate::cylinder::{Cylinder, CylinderFlags};
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
use crate::return_value::ReturnValue;
use crate::spawn::SpawnManager;
use crate::stability::StabilityManager;
use crate::thing::Thing;
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
    pub config: Arc<ConfigManager>,
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
    /// TCP connection → logged-in player (`conn_id` from `tfs-rust-net`).
    pub conn_to_creature: HashMap<ConnId, CreatureId>,
    /// Game-thread only — see [`DeferredTurnBroadcast`].
    pub deferred_turn_broadcast: HashMap<CreatureId, DeferredTurnBroadcast>,
    /// `ProtocolGame::knownCreatureSet` — must persist across `0x64` / move strips (`src/protocolgame.cpp`).
    pub known_creatures_by_conn: HashMap<ConnId, HashSet<u32>>,
    /// OTB + `items.xml` — server item id → client id for map / `addItem` (`src/items.cpp`).
    pub items_db: Arc<ItemDatabase>,
    pub vocations: Arc<VocationDatabase>,
    /// C++ `ProtocolGame::sendCreatureSay` static `statementId` (`src/protocolgame.cpp` ~2432).
    pub next_statement_id: u32,
    /// When set, walk wake uses Tokio one-shot timers (`src/scheduler.cpp`); `None` falls back to polling in `process_walk_deadlines`.
    pub(crate) walk_wake_tx: Option<UnboundedSender<CreatureId>>,
    /// TFS `Game::ReleaseCreature` → `ToReleaseCreatures` (`src/game.cpp` ~4766–4768), drained in [`Self::cleanup`].
    creatures_pending_release: Vec<CreatureId>,
    /// TFS `Game::ReleaseItem` → `ToReleaseItems` (`src/game.cpp` ~4771–4773).
    items_pending_release: Vec<ItemId>,
    /// Open bags / loaded `player_items` containers — `container.h` / `player.cpp`.
    pub container_registry: ContainerRegistry,
}

impl GameWorld {
    /// TFS `Player::canDoAction` — false while `nextAction` is in the future (`player.cpp`).
    /// Non-players are treated as ready (no `next_action_until`).
    pub fn player_timed_action_ready(&self, cid: CreatureId, now: Instant) -> bool {
        match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.timed_action_ready(now),
            _ => true,
        }
    }

    /// TFS `ProtocolGame::canSee(Position)` — viewport around viewer (`protocolgame.cpp`).
    pub fn can_see_position(&self, viewer: CreatureId, pos: Position) -> bool {
        let Some(vp) = self.creatures.get(viewer).map(|k| k.position()) else {
            return false;
        };
        if vp.z != pos.z {
            return false;
        }
        let dx = (vp.x as i32 - pos.x as i32).unsigned_abs();
        let dy = (vp.y as i32 - pos.y as i32).unsigned_abs();
        dx <= MAX_CLIENT_VIEWPORT_X as u32 && dy <= MAX_CLIENT_VIEWPORT_Y as u32
    }

    /// Collect all `ConnId`s whose creature can see `pos`. Used by every broadcast.
    fn spectator_conns(&self, pos: Position) -> Vec<ConnId> {
        self.conn_to_creature.iter()
            .filter(|(_, &vid)| self.can_see_position(vid, pos))
            .map(|(&c, _)| c)
            .collect()
    }

    /// Enqueue the same packet bytes for every connection that can see `pos` (clone per viewer).
    // C++ ref: repeated `ProtocolGame` fan-out in `game.cpp` / `protocolgame.cpp`.
    fn broadcast_to_spectators(&mut self, pos: Position, packet: Vec<u8>) {
        let conns = self.spectator_conns(pos);
        for conn in conns {
            self.enqueue_outgoing(conn, packet.clone());
        }
    }

    pub fn new(
        map: Map,
        items: SlotMap<ItemId, Item>,
        events: Box<dyn EventDispatcher>,
        config: Arc<ConfigManager>,
        db: DbPool,
        spawns: SpawnManager,
        items_db: Arc<ItemDatabase>,
        vocations: Arc<VocationDatabase>,
        walk_wake_tx: Option<UnboundedSender<CreatureId>>,
    ) -> Self {
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
            conn_to_creature: HashMap::new(),
            deferred_turn_broadcast: HashMap::new(),
            known_creatures_by_conn: HashMap::new(),
            items_db,
            vocations,
            next_statement_id: 0,
            walk_wake_tx,
            creatures_pending_release: Vec::new(),
            items_pending_release: Vec::new(),
            container_registry: ContainerRegistry::new(),
        }
    }

    /// TFS `Game::ReleaseCreature` — deferred until [`Self::cleanup`] (`src/game.cpp` ~4766).
    pub fn release_creature(&mut self, id: CreatureId) {
        self.creatures_pending_release.push(id);
    }

    /// TFS `Game::ReleaseItem` — deferred until [`Self::cleanup`] (`src/game.cpp` ~4771).
    pub fn release_item(&mut self, id: ItemId) {
        self.items_pending_release.push(id);
    }

    /// TFS `Game::cleanup` (`src/game.cpp` ~4752) — after `Creature::onWalk` (`src/game.cpp` ~3778).
    pub fn cleanup(&mut self) {
        let creatures = std::mem::take(&mut self.creatures_pending_release);
        for id in creatures {
            if self.creatures.contains_key(id) {
                self.remove_creature(id);
            }
        }
        let items = std::mem::take(&mut self.items_pending_release);
        for id in items {
            self.decay.cancel(id);
            let _ = self.items.remove(id);
        }
    }

    /// C++ `++statementId` before each `sendCreatureSay` / related speech packet.
    pub fn alloc_statement_id(&mut self) -> u32 {
        self.next_statement_id = self.next_statement_id.wrapping_add(1);
        self.next_statement_id
    }

    /// TFS `Game::internalCreatureSay` — one `ProtocolGame::sendCreatureSay` per viewer in range (`game.cpp` ~3723–3758).
    pub fn broadcast_creature_say_viewport(
        &mut self,
        speaker: CreatureId,
        speak_type: u8,
        text: &str,
    ) {
        let (pos, name, level) = match self.creatures.get(speaker) {
            Some(CreatureKind::Player(p)) => (p.base.position, p.base.name.clone(), p.level as u16),
            _ => return,
        };
        let viewers: Vec<(ConnId, CreatureId)> = self
            .conn_to_creature
            .iter()
            .map(|(&conn, &viewer)| (conn, viewer))
            .collect();
        for (conn, viewer) in viewers {
            if self.can_see_position(viewer, pos) {
                let sid = self.alloc_statement_id();
                let packet = send_creature_say(sid, &name, level, speak_type, pos, text).into_bytes();
                self.enqueue_outgoing(conn, packet);
            }
        }
    }

    /// Test / custom hook injection (same thread as game loop).
    pub fn set_protocol_hooks(&mut self, hooks: SharedProtocolHooks) {
        self.protocol_hooks = hooks;
    }

    /// Queue raw packet bytes for a connection (built by `tfs-rust-net` outgoing helpers).
    pub fn enqueue_outgoing(&mut self, conn: ConnId, packet: Vec<u8>) {
        self.pending_outgoing.entry(conn).or_default().push(packet);
    }

    /// Drain all queued outgoing packets at end of tick; IO layer sends each blob in order per connection.
    pub fn flush_output_buffers(&mut self) -> HashMap<ConnId, Vec<Vec<u8>>> {
        std::mem::take(&mut self.pending_outgoing)
    }

    /// Remove creature from map index, player lookups, guild online; remove summons if master dies.
    // C++ reference: `Game::removeCreature` — summon chain.
    pub fn remove_creature(&mut self, id: CreatureId) {
        let mut summons: Vec<CreatureId> = Vec::new();
        for (cid, k) in self.creatures.iter() {
            if k.base().master == Some(id) {
                summons.push(cid);
            }
        }
        for s in summons {
            self.remove_creature(s);
        }

        let pos = self.creatures.get(id).map(|k| k.position());
        let player_cleanup = self.creatures.get(id).and_then(|k| {
            if let CreatureKind::Player(pl) = k {
                Some((pl.base.name.clone(), pl.guid, pl.social.guild_id.is_some()))
            } else {
                None
            }
        });

        if let Some(p) = pos {
            self.map.unregister_creature_index(p, id);
            if let Some(t) = self.map.get_tile_mut(p) {
                t.remove_creature(id);
            }
        }

        if let Some((name, guid, in_guild)) = player_cleanup {
            self.player_by_name.remove(&name);
            self.player_by_guid.remove(&guid);
            if in_guild {
                self.guilds.unregister_online(id);
            }
        }

        let _ = self.container_registry.close_all_for_player(id);

        self.deferred_turn_broadcast.remove(&id);
        self.stop_event_walk(id);
        self.creatures.remove(id);
    }

    /// TFS `ProtocolGame::logout` (`protocolgame.cpp:336-372`).
    /// Handles player logout with validation, effects, and cleanup.
    // C++ ref: src/protocolgame.cpp:336-372
    pub fn player_logout(&mut self, conn_id: ConnId, cid: CreatureId, display_effect: bool, forced: bool) {
        use tfs_rust_common::enums::ZoneType;

        // Verify player exists
        let Some(CreatureKind::Player(player)) = self.creatures.get(cid) else {
            return;
        };

        // Check logout conditions if not forced
        if !forced {
            // Check if player has access (gamemaster/canAlwaysLogin flag equivalent)
            // C++: player->isAccessPlayer() checks group access
            // Using ghost_mode as proxy for GM access until proper groups are implemented
            let has_access = player.ghost_mode;

            if !has_access {
                // Check no-logout zone (TILESTATE_NOLOGOUT)
                let pos = player.base.position;
                if let Some(tile) = self.map.get_tile(pos) {
                    if tile.body().zone == ZoneType::NoLogout {
                        self.send_cancel_message(conn_id, ReturnValue::YouCannotLogoutHere);
                        return;
                    }

                    // Check infight condition outside protection zone
                    let in_protection_zone = tile.body().zone == ZoneType::Protection;
                    let has_infight = player.base.active_conditions.iter()
                        .any(|c| c.ctype == ConditionType::Infight);
                    if !in_protection_zone && has_infight {
                        self.send_cancel_message(conn_id, ReturnValue::YouMayNotLogoutDuringAFight);
                        return;
                    }
                }
            }

            // Scripting event - onLogout
            // C++ ref: src/protocolgame.cpp:357 (`g_creatureEvents->playerLogout(player)`).
            self.events.on_logout(cid, self);
        }

        // Get player data for effect
        let health = player.base.health;
        let ghost_mode = player.ghost_mode;
        let pos = player.base.position;

        // Show logout effect if requested and player is alive and not in ghost mode
        // C++: if (displayEffect && player->getHealth() > 0 && !player->isInGhostMode())
        if display_effect && health > 0 && !ghost_mode {
            // Magic effect CONST_ME_POFF (value 4)
            self.broadcast_magic_effect(pos, 4);
        }

        // Remove connection mapping
        self.conn_to_creature.remove(&conn_id);
        self.known_creatures_by_conn.remove(&conn_id);

        // Remove creature from world (C++: g_game.removeCreature(player))
        self.remove_creature(cid);

        tracing::info!(guid = self.player_guid(cid).unwrap_or(0), "player logged out");
    }

    /// Broadcast a magic effect to all spectators at a position.
    // C++ ref: src/game.cpp:4816 Game::addMagicEffect
    pub fn broadcast_magic_effect(&mut self, pos: Position, effect_id: u8) {
        use tfs_rust_net::outgoing::send_magic_effect;
        let pkt = send_magic_effect(pos, effect_id).into_bytes();
        self.broadcast_to_spectators(pos, pkt);
    }

    /// Run death XP / events / corpse scheduling, then remove the creature (and summons).
    pub fn apply_creature_death(&mut self, victim: CreatureId) {
        crate::death::handle_creature_death(
            &mut self.creatures,
            &mut self.items,
            &mut self.decay,
            self.events.as_ref(),
            victim,
            self.tick_counter,
            None,
        );
        self.remove_creature(victim);
    }

    /// One simulation tick (~50 ms target).
    pub fn on_tick(&mut self, now: std::time::Instant) {
        if self.walk_wake_tx.is_none() {
            self.process_walk_deadlines();
        }
        self.process_walk_action_tasks(now);

        self.tick_counter = self.tick_counter.wrapping_add(1);

        let tick = self.tick_counter;
        for (cid, k) in self.creatures.iter_mut() {
            if let CreatureKind::Monster(m) = k {
                m.think_tick(tick, cid);
            }
        }

        let _ = self.decay.tick(self.tick_counter);
        self.spawns.tick(std::time::Instant::now());
        if self.tick_counter.is_multiple_of(5) {
            self.events.lua_gc_step();
        }
        self.tick_player_pings(now);
    }

    /// Whether `viewer` may treat `target_protocol_id` as “seen” for `knownCreatureSet` eviction.
    /// C++: `ProtocolGame::canSee` / `Player::canSeeCreature` (`protocolgame.cpp` ~778+).
    pub fn can_see_creature_for_known_set(&self, viewer: CreatureId, target_protocol_id: u32) -> bool {
        if self.player_guid(viewer) == Some(target_protocol_id) {
            return true;
        }
        for (cid, k) in self.creatures.iter() {
            let wire_id = match k {
                CreatureKind::Player(p) => p.guid,
                CreatureKind::Monster(_) | CreatureKind::Npc(_) => {
                    (cid.data().as_ffi() & 0xFFFF_FFFF) as u32
                }
            };
            if wire_id != target_protocol_id {
                continue;
            }
            let invisible = Self::has_invisible(&k.base().active_conditions);
            if invisible && viewer != cid {
                return false;
            }
            if let CreatureKind::Player(p) = k {
                if p.ghost_mode {
                    return false;
                }
            }
            return true;
        }
        true
    }

    fn has_invisible(conditions: &[ActiveCondition]) -> bool {
        conditions
            .iter()
            .any(|c| c.ctype == ConditionType::Invisible)
    }

    fn player_guid(&self, cid: CreatureId) -> Option<u32> {
        self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Player(p) => Some(p.guid),
            _ => None,
        })
    }

    /// C++ `Player::sendStats` (`player.cpp` ~882) — builds a full `0xA0` stats packet and enqueues
    /// it for the player's connection. Must be called after any health/mana/soul/experience/capacity
    /// change (mirrors every `sendStats()` call site in TFS C++).
    pub fn send_player_stats(&mut self, cid: CreatureId) {
        let Some(conn_id) = self.conn_for_creature(cid) else {
            return;
        };
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };

        let hl = p.base.health.max(0).min(u16::MAX as i32) as u16;
        let max_h = p.base.max_health.max(0).min(u16::MAX as i32) as u16;
        let total_cap = p.get_capacity_u32();
        let free_cap = p.get_free_capacity_u32();
        let level = p.level.max(0).min(u16::MAX as i32) as u16;

        // C++ `Player::getPercentLevel` (`player.cpp` ~1914).
        let level_percent = {
            let curr = crate::creature::vocation::total_experience_for_level(level as u32);
            let next = crate::creature::vocation::total_experience_for_level(level as u32 + 1);
            if next > curr && p.experience >= curr {
                (((p.experience - curr) * 100) / (next - curr)).min(100) as u8
            } else {
                0u8
            }
        };

        let stats = PlayerStats1098 {
            health: hl,
            max_health: max_h,
            free_capacity: free_cap,
            total_capacity: total_cap,
            experience: p.experience,
            level,
            level_percent,
            mana: p.mana.max(0).min(u16::MAX as i32) as u16,
            max_mana: p.max_mana.max(0).min(u16::MAX as i32) as u16,
            magic_level: p.skills.maglevel.clamp(0, 255) as u8,
            base_magic_level: p.skills.maglevel.clamp(0, 255) as u8,
            magic_level_percent: 0,
            soul: p.economy.soul.clamp(0, 255) as u8,
            stamina_minutes: p.stamina_minutes,
            base_speed_half: (p.base.base_speed.max(0) as u32 / 2).min(0xFFFF) as u16,
            regeneration_ticks_sec: 0,
            offline_training_time: (p.offline_training_ms / 60 / 1000).min(65535) as u16,
        };

        self.enqueue_outgoing(conn_id, send_player_stats_1098(&stats).into_bytes());
    }

    // === Item Movement (B.4) ===
    // C++ reference: `src/game.cpp` Game::internalMoveItem (~1078), internalAddItem (~1287),
    //                internalRemoveItem (~1376), internalGetCylinder (~197), internalGetThing (~213).

    /// Resolve a client-encoded position to a `Cylinder`.
    // C++ ref: src/game.cpp:197 Game::internalGetCylinder
    pub fn internal_get_cylinder(&self, cid: CreatureId, pos: Position) -> Option<Cylinder> {
        if pos.x != 0xFFFF {
            // Map tile
            if self.map.get_tile(pos).is_some() {
                return Some(Cylinder::Tile { pos });
            }
            return None;
        }
        // Container (y & 0x40) — `game.cpp` `internalGetCylinder` container branch.
        if pos.y & 0x40 != 0 {
            let client_cid = (pos.y & 0x0F) as u8;
            let slot_index = pos.z as i32;
            let container_id = self
                .container_registry
                .get_container_by_cid(cid, client_cid)?;
            return Some(Cylinder::Container {
                item_id: container_id,
                index: slot_index,
            });
        }
        // Inventory slot
        Some(Cylinder::Inventory {
            player_id: cid,
            slot: pos.y as u8,
        })
    }

    /// Resolve a client-encoded position to a `Thing` (item or creature on a tile).
    // C++ ref: src/game.cpp:213 Game::internalGetThing (STACKPOS_MOVE path)
    pub fn internal_get_thing_move(&self, cid: CreatureId, pos: Position, _stack_pos: u8) -> Option<Thing> {
        if pos.x != 0xFFFF {
            let tile = self.map.get_tile(pos)?;
            // STACKPOS_MOVE: prefer top moveable down item, else top visible creature
            if let Some(top_item_id) = tile.get_top_down_item() {
                if let Some(item) = self.items.get(top_item_id) {
                    let it = self.items_db.items.get(&item.item_type);
                    if it.map(|t| t.moveable()).unwrap_or(false) {
                        return Some(Thing::Item(top_item_id));
                    }
                }
            }
            // Fall through to creature
            let body = tile.body();
            if let Some(&creature_id) = body.creatures.last() {
                return Some(Thing::Creature(creature_id));
            }
            return None;
        }
        // Container slot — `internalGetThing` container UI position.
        if pos.y & 0x40 != 0 {
            let client_cid = (pos.y & 0x0F) as u8;
            let slot = pos.z as usize;
            let container_id = self.container_registry.get_container_by_cid(cid, client_cid)?;
            let c = self.container_registry.get(container_id)?;
            let iid = c.get_item(slot)?;
            return Some(Thing::Item(iid));
        }
        // Inventory — `pos.y` is `slots_t` (`game.cpp` ~320–326).
        let slot = pos.y as u8;
        if let Some(iid) = self.get_player_inventory_item(cid, slot) {
            return Some(Thing::Item(iid));
        }
        None
    }

    /// Query if a tile can accept an item.
    // C++ ref: src/tile.cpp:629-702 Tile::queryAdd for items
    fn query_add_item_to_tile(&self, pos: Position, item_id: ItemId, flags: CylinderFlags) -> ReturnValue {
        let Some(tile) = self.map.get_tile(pos) else {
            return ReturnValue::NotPossible;
        };
        // Max items check
        if tile.total_item_count() >= 0xFFFF {
            return ReturnValue::NotPossible;
        }
        if flags.contains(CylinderFlags::NO_LIMIT) {
            return ReturnValue::NoError;
        }
        let Some(item) = self.items.get(item_id) else {
            return ReturnValue::NotPossible;
        };
        let it = self.items_db.items.get(&item.item_type);
        let is_hangable = it.map(|t| t.is_hangable()).unwrap_or(false);
        // Non-hangable items need ground
        if tile.body().ground.is_none() && !is_hangable {
            return ReturnValue::NotPossible;
        }
        // Blocking item can't be placed where non-ghost creatures are
        let is_blocking = it.map(|t| t.block_solid()).unwrap_or(false);
        if is_blocking && !flags.contains(CylinderFlags::IGNORE_BLOCK_CREATURE) {
            let body = tile.body();
            if !body.creatures.is_empty() {
                return ReturnValue::NotEnoughRoom;
            }
        }
        ReturnValue::NoError
    }

    /// Validate that an item exists in the specified cylinder.
    fn validate_item_in_cylinder(&self, cylinder: &Cylinder, item_id: ItemId) -> Result<(), ReturnValue> {
        match cylinder {
            Cylinder::Tile { pos } => {
                let tile = self.map.get_tile(*pos).ok_or(ReturnValue::NotPossible)?;
                if !tile.has_item(item_id) {
                    return Err(ReturnValue::NotPossible);
                }
                Ok(())
            }
            Cylinder::Container {
                item_id: container_id,
                ..
            } => {
                let c = self
                    .container_registry
                    .get(*container_id)
                    .ok_or(ReturnValue::NotPossible)?;
                if !c.contains(item_id) {
                    return Err(ReturnValue::NotPossible);
                }
                Ok(())
            }
            Cylinder::Inventory { player_id, slot } => {
                self.validate_inventory_item(*player_id, *slot, item_id)
            }
        }
    }

    /// Add an item to a tile, handling stackable merge.
    /// Returns the ItemId that ended up on the tile (may differ if merged into existing stack).
    // C++ ref: src/game.cpp:1287 Game::internalAddItem (tile path)
    pub fn internal_add_item_to_tile(
        &mut self,
        pos: Position,
        item_id: ItemId,
        _flags: CylinderFlags,
    ) -> Result<ItemId, ReturnValue> {
        let is_stackable;
        let item_type;
        let item_count;
        {
            let item = self.items.get(item_id).ok_or(ReturnValue::NotPossible)?;
            item_type = item.item_type;
            item_count = item.count;
            is_stackable = self.items_db.items.get(&item.item_type).map(|t| t.stackable()).unwrap_or(false);
        }

        // Try stackable merge
        if is_stackable {
            let tile = self.map.get_tile(pos).ok_or(ReturnValue::NotPossible)?;
            // Look for an existing stack of the same type
            let mut merge_target: Option<ItemId> = None;
            for &did in &tile.body().down_items {
                if let Some(existing) = self.items.get(did) {
                    if existing.item_type == item_type && existing.count < 100 {
                        merge_target = Some(did);
                        break;
                    }
                }
            }
            if let Some(target_id) = merge_target {
                let target_count = self.items.get(target_id).map(|i| i.count).unwrap_or(0);
                let can_add = (100u16).saturating_sub(target_count).min(item_count);
                if can_add > 0 {
                    if let Some(target) = self.items.get_mut(target_id) {
                        target.count += can_add;
                    }
                    // Get stack pos for update packet
                    let stack_pos = self.map.get_tile(pos)
                        .and_then(|t| t.get_item_stack_pos(target_id))
                        .unwrap_or(0);
                    self.broadcast_tile_item_update(pos, target_id, stack_pos);

                    let remainder = item_count.saturating_sub(can_add);
                    if remainder == 0 {
                        // Fully merged — remove the source item from SlotMap
                        self.items.remove(item_id);
                        return Ok(target_id);
                    }
                    // Partial merge — update source item count and add remainder to tile
                    if let Some(item) = self.items.get_mut(item_id) {
                        item.count = remainder;
                    }
                }
            }
        }

        // Add item to tile
        let tile = self.map.get_tile_mut(pos).ok_or(ReturnValue::NotPossible)?;
        let stack_pos = tile.down_item_start_stack_pos();
        tile.add_item(item_id);

        // Broadcast add
        self.broadcast_tile_item_add(pos, item_id, stack_pos);

        Ok(item_id)
    }

    /// Remove an item (or count of a stackable) from a tile.
    // C++ ref: src/game.cpp:1376 Game::internalRemoveItem
    pub fn internal_remove_item_from_tile(
        &mut self,
        pos: Position,
        item_id: ItemId,
        count: u16,
    ) -> Result<(), ReturnValue> {
        let item = self.items.get(item_id).ok_or(ReturnValue::NotPossible)?;
        let is_stackable = self.items_db.items.get(&item.item_type).map(|t| t.stackable()).unwrap_or(false);
        let item_count = item.count;

        if is_stackable && count < item_count {
            // Partial removal — just reduce count and send update
            if let Some(item) = self.items.get_mut(item_id) {
                item.count -= count;
            }
            let stack_pos = self.map.get_tile(pos)
                .and_then(|t| t.get_item_stack_pos(item_id))
                .unwrap_or(0);
            self.broadcast_tile_item_update(pos, item_id, stack_pos);
        } else {
            // Full removal
            let stack_pos = self.map.get_tile(pos)
                .and_then(|t| t.get_item_stack_pos(item_id))
                .unwrap_or(0);
            let tile = self.map.get_tile_mut(pos).ok_or(ReturnValue::NotPossible)?;
            if tile.remove_item_by_id(item_id).is_none() {
                return Err(ReturnValue::NotPossible);
            }
            self.broadcast_tile_item_remove(pos, stack_pos);
            // Remove from SlotMap
            self.items.remove(item_id);
        }
        Ok(())
    }

    /// Move an item between cylinders. Handles tile↔tile with stackable merge/split.
    /// Returns the ItemId that ended up at the destination.
    // C++ ref: src/game.cpp:1078 Game::internalMoveItem
    pub fn internal_move_item(
        &mut self,
        acting_player: Option<CreatureId>,
        from_cylinder: Cylinder,
        to_cylinder: Cylinder,
        item_id: ItemId,
        count: u16,
        flags: CylinderFlags,
    ) -> Result<ItemId, ReturnValue> {
        // Validate source has the item
        self.validate_item_in_cylinder(&from_cylinder, item_id)?;
        let source_parent = from_cylinder.as_container();

        let (to_work, to_merge_item) =
            self.resolve_move_destination(to_cylinder, item_id, source_parent, flags)?;

        // For tile destinations, check queryAdd
        if let Cylinder::Tile { pos } = to_work {
            let rv = self.query_add_item_to_tile(pos, item_id, flags);
            if rv.is_error() {
                return Err(rv);
            }
        }
        if let Cylinder::Inventory {
            player_id: to_pid,
            slot: to_slot,
        } = to_work
        {
            let move_count = self
                .items
                .get(item_id)
                .map(|i| (i.count as u32).min(u32::from(count)))
                .unwrap_or(1);
            let rv = self.player_query_add(to_pid, to_slot, item_id, move_count, flags);
            match rv {
                ReturnValue::NoError | ReturnValue::NeedExchange => {}
                _ => return Err(rv),
            }
        }
        if let Cylinder::Container { item_id: cid, index } = to_work {
            let m_pre = self.items.get(item_id).map(|i| i.count).unwrap_or(1);
            let m_pre = m_pre.min(count);
            let ret = self.container_query_add(
                cid,
                index,
                item_id,
                u32::from(m_pre),
                flags,
                acting_player,
            );
            if ret.is_error() {
                return Err(ret);
            }
        }

        let item = self.items.get(item_id).ok_or(ReturnValue::NotPossible)?;
        let is_stackable = self.items_db.items.get(&item.item_type).map(|t| t.stackable()).unwrap_or(false);
        let item_count = item.count;
        let item_type = item.item_type;

        let m = if is_stackable { count.min(item_count) } else { item_count };

        if to_merge_item == Some(item_id) {
            return Ok(item_id);
        }

        let max_query_count: u32 = match to_work {
            Cylinder::Container { item_id: cid, index } => {
                self.container_query_max_count(cid, index, item_id, u32::from(m), flags)?
            }
            _ => u32::from(m),
        };
        let mut m_move = if is_stackable {
            m.min(max_query_count as u16)
        } else {
            m.min(max_query_count as u16)
        };

        if let Some(merge_id) = to_merge_item {
            let merge_room = 100u16.saturating_sub(self.items.get(merge_id).map(|i| i.count).unwrap_or(0));
            m_move = m_move.min(merge_room);
        }

        match (&from_cylinder, &to_work) {
            (Cylinder::Tile { pos: from_pos }, Cylinder::Tile { pos: to_pos }) => {
                let from_pos = *from_pos;
                let to_pos = *to_pos;

                if is_stackable && m < item_count {
                    // Partial stack move — reduce source count, create new item for destination
                    if let Some(src) = self.items.get_mut(item_id) {
                        src.count -= m;
                    }
                    let src_stack_pos = self.map.get_tile(from_pos)
                        .and_then(|t| t.get_item_stack_pos(item_id))
                        .unwrap_or(0);
                    self.broadcast_tile_item_update(from_pos, item_id, src_stack_pos);

                    // Create new item for the moved portion
                    let new_item = Item::new(ItemId::default(), item_type, m);
                    let new_id = self.items.insert(new_item);
                    self.internal_add_item_to_tile(to_pos, new_id, flags)?;
                    Ok(new_id)
                } else {
                    // Full move
                    let stack_pos = self.map.get_tile(from_pos)
                        .and_then(|t| t.get_item_stack_pos(item_id))
                        .unwrap_or(0);
                    let tile = self.map.get_tile_mut(from_pos).ok_or(ReturnValue::NotPossible)?;
                    if tile.remove_item_by_id(item_id).is_none() {
                        return Err(ReturnValue::NotPossible);
                    }
                    self.broadcast_tile_item_remove(from_pos, stack_pos);
                    self.internal_add_item_to_tile(to_pos, item_id, flags)
                }
            }
            (Cylinder::Tile { pos: from_pos }, Cylinder::Container { .. }) => {
                let Cylinder::Container {
                    item_id: dest_cid,
                    index: dest_idx,
                } = to_work
                else {
                    return Err(ReturnValue::NotPossible);
                };
                let from_pos = *from_pos;
                if is_stackable && m_move < item_count {
                    if let Some(merge_id) = to_merge_item {
                        if merge_id == item_id {
                            return Ok(item_id);
                        }
                        let room = 100u16.saturating_sub(
                            self.items.get(merge_id).map(|i| i.count).unwrap_or(0),
                        );
                        if room < m_move {
                            return Err(ReturnValue::ContainerNotEnoughRoom);
                        }
                        if let Some(src) = self.items.get_mut(item_id) {
                            src.count = src.count.saturating_sub(m_move);
                        }
                        let src_stack_pos = self
                            .map
                            .get_tile(from_pos)
                            .and_then(|t| t.get_item_stack_pos(item_id))
                            .unwrap_or(0);
                        self.broadcast_tile_item_update(from_pos, item_id, src_stack_pos);
                        if let Some(t) = self.items.get_mut(merge_id) {
                            t.count = t.count.saturating_add(m_move);
                        }
                        self.refresh_container_chain(dest_cid);
                        let merge_slot = self
                            .get_thing_index_in_container(dest_cid, merge_id)
                            .map(|i| i as u16)
                            .unwrap_or(0);
                        self.notify_container_content_changed(
                            dest_cid,
                            ContainerContentChange::Update { slot: merge_slot },
                        );
                        return Ok(merge_id);
                    }
                    return Err(ReturnValue::NotPossible);
                }
                let dest_has_room = self
                    .container_registry
                    .get(dest_cid)
                    .map(|c| !c.is_full())
                    .ok_or(ReturnValue::NotPossible)?;
                if !dest_has_room {
                    return Err(ReturnValue::ContainerNotEnoughRoom);
                }
                let stack_pos = self
                    .map
                    .get_tile(from_pos)
                    .and_then(|t| t.get_item_stack_pos(item_id))
                    .unwrap_or(0);
                let tile = self.map.get_tile_mut(from_pos).ok_or(ReturnValue::NotPossible)?;
                if tile.remove_item_by_id(item_id).is_none() {
                    return Err(ReturnValue::NotPossible);
                }
                self.broadcast_tile_item_remove(from_pos, stack_pos);
                if let Some(merge_id) = to_merge_item {
                    let room = 100u16.saturating_sub(
                        self.items.get(merge_id).map(|i| i.count).unwrap_or(0),
                    );
                    if room < m_move {
                        return Err(ReturnValue::ContainerNotEnoughRoom);
                    }
                    if let Some(t) = self.items.get_mut(merge_id) {
                        t.count = t.count.saturating_add(m_move);
                    }
                    self.items.remove(item_id);
                    self.refresh_container_chain(dest_cid);
                    let merge_slot = self
                        .get_thing_index_in_container(dest_cid, merge_id)
                        .map(|i| i as u16)
                        .unwrap_or(0);
                    self.notify_container_content_changed(
                        dest_cid,
                        ContainerContentChange::Update {
                            slot: merge_slot,
                        },
                    );
                    return Ok(merge_id);
                }
                self.container_add_thing(dest_cid, dest_idx, item_id)?;
                Ok(item_id)
            }
            (Cylinder::Container { .. }, Cylinder::Tile { pos: to_pos }) => {
                let Cylinder::Container {
                    item_id: from_cid,
                    ..
                } = from_cylinder
                else {
                    return Err(ReturnValue::NotPossible);
                };
                let to_pos = *to_pos;
                if is_stackable && m_move < item_count {
                    let rv = self.container_query_remove(
                        from_cid,
                        item_id,
                        u32::from(m_move),
                        flags,
                        acting_player,
                    );
                    if rv.is_error() {
                        return Err(rv);
                    }
                    self.container_remove_thing(from_cid, item_id, u32::from(m_move))?;
                    let new_item = Item::new(ItemId::default(), item_type, m_move);
                    let new_id = self.items.insert(new_item);
                    self.internal_add_item_to_tile(to_pos, new_id, flags)?;
                    return Ok(new_id);
                }
                let rv = self.container_query_remove(
                    from_cid,
                    item_id,
                    u32::from(m_move),
                    flags,
                    acting_player,
                );
                if rv.is_error() {
                    return Err(rv);
                }
                self.container_detach_item(from_cid, item_id)?;
                self.internal_add_item_to_tile(to_pos, item_id, flags)
            }
            (Cylinder::Container { .. }, Cylinder::Container { .. }) => {
                let Cylinder::Container {
                    item_id: from_cid,
                    ..
                } = from_cylinder
                else {
                    return Err(ReturnValue::NotPossible);
                };
                let Cylinder::Container {
                    item_id: dest_cid,
                    index: dest_idx,
                } = to_work
                else {
                    return Err(ReturnValue::NotPossible);
                };
                let dest_has_room = self
                    .container_registry
                    .get(dest_cid)
                    .map(|c| !c.is_full())
                    .ok_or(ReturnValue::NotPossible)?;
                if !dest_has_room {
                    return Err(ReturnValue::ContainerNotEnoughRoom);
                }
                if is_stackable && m_move < item_count {
                    let rv = self.container_query_remove(
                        from_cid,
                        item_id,
                        u32::from(m_move),
                        flags,
                        acting_player,
                    );
                    if rv.is_error() {
                        return Err(rv);
                    }
                    self.container_remove_thing(from_cid, item_id, u32::from(m_move))?;
                    if let Some(merge_id) = to_merge_item {
                        if merge_id == item_id {
                            return Ok(item_id);
                        }
                        let room = 100u16.saturating_sub(
                            self.items.get(merge_id).map(|i| i.count).unwrap_or(0),
                        );
                        if room < m_move {
                            return Err(ReturnValue::ContainerNotEnoughRoom);
                        }
                        if let Some(t) = self.items.get_mut(merge_id) {
                            t.count = t.count.saturating_add(m_move);
                        }
                        self.refresh_container_chain(dest_cid);
                        let merge_slot = self
                            .get_thing_index_in_container(dest_cid, merge_id)
                            .map(|i| i as u16)
                            .unwrap_or(0);
                        self.notify_container_content_changed(
                            dest_cid,
                            ContainerContentChange::Update {
                                slot: merge_slot,
                            },
                        );
                        return Ok(merge_id);
                    }
                    let new_item = Item::new(ItemId::default(), item_type, m_move);
                    let new_id = self.items.insert(new_item);
                    self.container_add_thing(dest_cid, dest_idx, new_id)?;
                    return Ok(new_id);
                }
                let rv = self.container_query_remove(
                    from_cid,
                    item_id,
                    u32::from(m_move),
                    flags,
                    acting_player,
                );
                if rv.is_error() {
                    return Err(rv);
                }
                if let Some(merge_id) = to_merge_item {
                    let room = 100u16.saturating_sub(
                        self.items.get(merge_id).map(|i| i.count).unwrap_or(0),
                    );
                    if room < m_move {
                        return Err(ReturnValue::ContainerNotEnoughRoom);
                    }
                    self.container_remove_thing(from_cid, item_id, u32::from(m_move))?;
                    if let Some(t) = self.items.get_mut(merge_id) {
                        t.count = t.count.saturating_add(m_move);
                    }
                    self.refresh_container_chain(dest_cid);
                    let merge_slot = self
                        .get_thing_index_in_container(dest_cid, merge_id)
                        .map(|i| i as u16)
                        .unwrap_or(0);
                    self.notify_container_content_changed(
                        dest_cid,
                        ContainerContentChange::Update {
                            slot: merge_slot,
                        },
                    );
                    return Ok(merge_id);
                }
                let dest_has_room = self
                    .container_registry
                    .get(dest_cid)
                    .map(|c| !c.is_full())
                    .ok_or(ReturnValue::NotPossible)?;
                if !dest_has_room {
                    return Err(ReturnValue::ContainerNotEnoughRoom);
                }
                self.container_detach_item(from_cid, item_id)?;
                self.container_add_thing(dest_cid, dest_idx, item_id)?;
                Ok(item_id)
            }
            (
                Cylinder::Container {
                    item_id: from_container,
                    ..
                },
                Cylinder::Inventory {
                    player_id,
                    slot,
                },
            ) => {
                let cid = *player_id;
                let slot = *slot;
                let from_container = *from_container;
                if is_stackable && m_move < item_count {
                    let rv = self.container_query_remove(
                        from_container,
                        item_id,
                        u32::from(m_move),
                        flags,
                        acting_player,
                    );
                    if rv.is_error() {
                        return Err(rv);
                    }
                    if self.get_player_inventory_item(cid, slot).is_some() {
                        return Err(ReturnValue::NeedExchange);
                    }
                    self.container_remove_thing(from_container, item_id, u32::from(m_move))?;
                    let new_item = Item::new(ItemId::default(), item_type, m_move);
                    let new_id = self.items.insert(new_item);
                    self.internal_add_item_to_inventory_slot(cid, slot, new_id)?;
                    self.recompute_player_inventory_weight(cid);
                    self.broadcast_player_inventory_slot(cid, slot, Some(new_id));
                    self.send_player_stats(cid);
                    return Ok(new_id);
                }
                let rv = self.container_query_remove(
                    from_container,
                    item_id,
                    u32::from(m_move),
                    flags,
                    acting_player,
                );
                if rv.is_error() {
                    return Err(rv);
                }
                if let Some(dest_id) = self.get_player_inventory_item(cid, slot) {
                    if dest_id == item_id {
                        return Ok(item_id);
                    }
                    let idx = self
                        .get_thing_index_in_container(from_container, item_id)
                        .ok_or(ReturnValue::NotPossible)? as usize;
                    self.container_detach_item(from_container, item_id)?;
                    self.internal_remove_item_from_inventory_slot(cid, slot, dest_id)?;
                    self.broadcast_player_inventory_slot(cid, slot, None);
                    self.container_insert_item_at(from_container, idx, dest_id)?;
                    self.internal_add_item_to_inventory_slot(cid, slot, item_id)?;
                    self.broadcast_player_inventory_slot(cid, slot, Some(item_id));
                    self.recompute_player_inventory_weight(cid);
                    self.send_player_stats(cid);
                    return Ok(item_id);
                }
                self.container_detach_item(from_container, item_id)?;
                self.internal_add_item_to_inventory_slot(cid, slot, item_id)?;
                self.recompute_player_inventory_weight(cid);
                self.broadcast_player_inventory_slot(cid, slot, Some(item_id));
                self.send_player_stats(cid);
                Ok(item_id)
            }
            (
                Cylinder::Inventory {
                    player_id,
                    slot,
                },
                Cylinder::Container {
                    item_id: to_container,
                    index: to_idx,
                },
            ) => {
                let cid = *player_id;
                let slot = *slot;
                let to_container = *to_container;
                let to_idx = *to_idx;

                if let Some(merge_id) = to_merge_item {
                    if merge_id == item_id {
                        return Ok(item_id);
                    }
                    let room = 100u16.saturating_sub(
                        self.items.get(merge_id).map(|i| i.count).unwrap_or(0),
                    );
                    if room < m_move {
                        return Err(ReturnValue::ContainerNotEnoughRoom);
                    }
                    if is_stackable && m_move < item_count {
                        if let Some(src) = self.items.get_mut(item_id) {
                            src.count = src.count.saturating_sub(m_move);
                        }
                        if let Some(t) = self.items.get_mut(merge_id) {
                            t.count = t.count.saturating_add(m_move);
                        }
                        self.recompute_player_inventory_weight(cid);
                        self.send_player_stats(cid);
                        let mut registry = std::mem::take(&mut self.container_registry);
                        self.ensure_container_registered(&mut registry, to_container);
                        self.container_registry = registry;
                        self.refresh_container_chain(to_container);
                        let merge_slot = self
                            .get_thing_index_in_container(to_container, merge_id)
                            .map(|i| i as u16)
                            .unwrap_or(0);
                        self.notify_container_content_changed(
                            to_container,
                            ContainerContentChange::Update { slot: merge_slot },
                        );
                        self.broadcast_player_inventory_slot(cid, slot, Some(item_id));
                        return Ok(merge_id);
                    }
                    self.internal_remove_item_from_inventory_slot(cid, slot, item_id)?;
                    self.broadcast_player_inventory_slot(cid, slot, None);
                    if let Some(t) = self.items.get_mut(merge_id) {
                        t.count = t.count.saturating_add(m_move);
                    }
                    self.items.remove(item_id);
                    let mut registry = std::mem::take(&mut self.container_registry);
                    self.ensure_container_registered(&mut registry, to_container);
                    self.container_registry = registry;
                    self.refresh_container_chain(to_container);
                    let merge_slot = self
                        .get_thing_index_in_container(to_container, merge_id)
                        .map(|i| i as u16)
                        .unwrap_or(0);
                    self.notify_container_content_changed(
                        to_container,
                        ContainerContentChange::Update { slot: merge_slot },
                    );
                    self.recompute_player_inventory_weight(cid);
                    self.send_player_stats(cid);
                    return Ok(merge_id);
                }

                if is_stackable && m_move < item_count {
                    return Err(ReturnValue::NotPossible);
                }
                let dest_has_room = self
                    .container_registry
                    .get(to_container)
                    .map(|c| !c.is_full())
                    .ok_or(ReturnValue::NotPossible)?;
                if !dest_has_room {
                    return Err(ReturnValue::ContainerNotEnoughRoom);
                }
                self.internal_remove_item_from_inventory_slot(cid, slot, item_id)?;
                self.broadcast_player_inventory_slot(cid, slot, None);
                let mut registry = std::mem::take(&mut self.container_registry);
                self.ensure_container_registered(&mut registry, to_container);
                self.container_registry = registry;
                self.container_add_thing(to_container, to_idx, item_id)?;
                self.recompute_player_inventory_weight(cid);
                self.send_player_stats(cid);
                Ok(item_id)
            }
            (Cylinder::Tile { pos: from_pos }, Cylinder::Inventory { player_id, slot }) => {
                let from_pos = *from_pos;
                let cid = *player_id;
                let slot = *slot;
                if is_stackable && m_move < item_count {
                    if self.get_player_inventory_item(cid, slot).is_some() {
                        return Err(ReturnValue::NeedExchange);
                    }
                    if let Some(src) = self.items.get_mut(item_id) {
                        src.count -= m_move;
                    }
                    let src_stack_pos = self
                        .map
                        .get_tile(from_pos)
                        .and_then(|t| t.get_item_stack_pos(item_id))
                        .unwrap_or(0);
                    self.broadcast_tile_item_update(from_pos, item_id, src_stack_pos);
                    let new_item = Item::new(ItemId::default(), item_type, m_move);
                    let new_id = self.items.insert(new_item);
                    self.internal_add_item_to_inventory_slot(cid, slot, new_id)?;
                    self.recompute_player_inventory_weight(cid);
                    self.broadcast_player_inventory_slot(cid, slot, Some(new_id));
                    self.send_player_stats(cid);
                    return Ok(new_id);
                }
                if let Some(dest_id) = self.get_player_inventory_item(cid, slot) {
                    if dest_id == item_id {
                        return Ok(item_id);
                    }
                    let stack_pos = self.map.get_tile(from_pos)
                        .and_then(|t| t.get_item_stack_pos(item_id))
                        .unwrap_or(0);
                    let tile = self.map.get_tile_mut(from_pos).ok_or(ReturnValue::NotPossible)?;
                    if tile.remove_item_by_id(item_id).is_none() {
                        return Err(ReturnValue::NotPossible);
                    }
                    self.broadcast_tile_item_remove(from_pos, stack_pos);
                    self.internal_remove_item_from_inventory_slot(cid, slot, dest_id)?;
                    self.broadcast_player_inventory_slot(cid, slot, None);
                    self.internal_add_item_to_inventory_slot(cid, slot, item_id)?;
                    self.broadcast_player_inventory_slot(cid, slot, Some(item_id));
                    self.internal_add_item_to_tile(from_pos, dest_id, flags)?;
                    self.recompute_player_inventory_weight(cid);
                    self.send_player_stats(cid);
                    return Ok(item_id);
                }
                let stack_pos = self.map.get_tile(from_pos)
                    .and_then(|t| t.get_item_stack_pos(item_id))
                    .unwrap_or(0);
                let tile = self.map.get_tile_mut(from_pos).ok_or(ReturnValue::NotPossible)?;
                if tile.remove_item_by_id(item_id).is_none() {
                    return Err(ReturnValue::NotPossible);
                }
                self.broadcast_tile_item_remove(from_pos, stack_pos);
                self.internal_add_item_to_inventory_slot(cid, slot, item_id)?;
                self.recompute_player_inventory_weight(cid);
                self.broadcast_player_inventory_slot(cid, slot, Some(item_id));
                self.send_player_stats(cid);
                Ok(item_id)
            }
            (Cylinder::Inventory { player_id, slot }, Cylinder::Tile { pos: to_pos }) => {
                let cid = *player_id;
                let slot = *slot;
                let to_pos = *to_pos;
                self.internal_remove_item_from_inventory_slot(cid, slot, item_id)?;
                self.recompute_player_inventory_weight(cid);
                self.broadcast_player_inventory_slot(cid, slot, None);
                self.internal_add_item_to_tile(to_pos, item_id, flags)?;
                Ok(item_id)
            }
            (
                Cylinder::Inventory {
                    player_id: from_pid,
                    slot: from_slot,
                },
                Cylinder::Inventory {
                    player_id: to_pid,
                    slot: to_slot,
                },
            ) => {
                // `Game::internalMoveItem` inventory↔inventory — `game.cpp` ~1078 (Player cylinders).
                if *from_pid != *to_pid {
                    return Err(ReturnValue::NotPossible);
                }
                let cid = *from_pid;
                if *from_slot == *to_slot {
                    return Ok(item_id);
                }
                if is_stackable && m < item_count {
                    return Err(ReturnValue::NotPossible);
                }
                let dest_id = self.get_player_inventory_item(cid, *to_slot);
                if let Some(did) = dest_id {
                    if did == item_id {
                        return Ok(item_id);
                    }
                    let dest_count = self.items.get(did).map(|i| i.count as u32).unwrap_or(1);
                    let rv = self.player_query_add(cid, *from_slot, did, dest_count, flags);
                    if rv != ReturnValue::NoError {
                        return Err(rv);
                    }
                    let idx_f = crate::inventory::slot_to_array_index(*from_slot)
                        .ok_or(ReturnValue::NotPossible)?;
                    let idx_t = crate::inventory::slot_to_array_index(*to_slot)
                        .ok_or(ReturnValue::NotPossible)?;
                    if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                        p.equipment_slots[idx_f] = Some(did);
                        p.equipment_slots[idx_t] = Some(item_id);
                    }
                    self.events.on_player_deequip(cid, item_id, *from_slot);
                    self.events.on_player_equip(cid, item_id, *to_slot);
                    self.events.on_player_deequip(cid, did, *to_slot);
                    self.events.on_player_equip(cid, did, *from_slot);
                    self.broadcast_player_inventory_slot(cid, *from_slot, Some(did));
                    self.broadcast_player_inventory_slot(cid, *to_slot, Some(item_id));
                    return Ok(item_id);
                }
                self.internal_remove_item_from_inventory_slot(cid, *from_slot, item_id)?;
                self.internal_add_item_to_inventory_slot(cid, *to_slot, item_id)?;
                self.broadcast_player_inventory_slot(cid, *from_slot, None);
                self.broadcast_player_inventory_slot(cid, *to_slot, Some(item_id));
                Ok(item_id)
            }
        }
    }

    // === B.5: Player Throw (item move from client) ===
    // C++ ref: src/game.cpp:644 Game::playerMoveThing, :905 Game::playerMoveItem

    /// Handle `parseThrow` — player moves a thing from one position to another.
    pub fn player_move_thing(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        from_pos: Position,
        sprite_id: u16,
        from_stack_pos: u8,
        to_pos: Position,
        count: u8,
        now: Instant,
    ) {
        if from_pos == to_pos {
            return;
        }
        // Resolve source thing
        let Some(thing) = self.internal_get_thing_move(cid, from_pos, from_stack_pos) else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };

        match thing {
            Thing::Creature(_moving_creature) => {
                // Creature move — already handled by walk system for players;
                // NPC/monster push is Phase 9+.
                tracing::debug!("player_move_thing: creature move not yet wired");
            }
            Thing::Item(item_id) => {
                self.player_move_item(
                    conn_id,
                    cid,
                    from_pos,
                    sprite_id,
                    from_stack_pos,
                    to_pos,
                    count,
                    item_id,
                    now,
                );
            }
        }
    }

    /// Handle the item branch of playerMoveThing.
    // C++ ref: src/game.cpp:905 Game::playerMoveItem
    fn player_move_item(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        from_pos: Position,
        sprite_id: u16,
        from_stack_pos: u8,
        to_pos: Position,
        count: u8,
        item_id: ItemId,
        now: Instant,
    ) {
        let item_is_pickupable;
        let item_throw_range;
        // Verify client sprite ID matches
        if let Some(item) = self.items.get(item_id) {
            let it = self.items_db.items.get(&item.item_type);
            let client_id = it.map(|t| t.client_id).unwrap_or(0);
            if client_id != sprite_id {
                self.send_cancel_message(conn_id, ReturnValue::NotPossible);
                return;
            }
            // Check moveable
            let is_moveable = it.map(|t| t.moveable()).unwrap_or(false);
            if !is_moveable {
                self.send_cancel_message(conn_id, ReturnValue::NotMoveable);
                return;
            }
            item_is_pickupable = it.map(|t| t.pickupable()).unwrap_or(false);
            // C++ ref: src/item.h:828-829 Item::getThrowRange (pickupable ? 15 : 2)
            item_throw_range = if item_is_pickupable { 15u32 } else { 2u32 };
        } else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        }

        // Resolve cylinders
        let Some(from_cylinder) = self.internal_get_cylinder(cid, from_pos) else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };
        let Some(to_cylinder) = self.internal_get_cylinder(cid, to_pos) else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };

        let Some(player_pos) = self.creatures.get(cid).map(|p| p.position()) else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };

        let map_from_pos = match from_cylinder {
            Cylinder::Tile { pos } => pos,
            Cylinder::Container { .. } | Cylinder::Inventory { .. } => player_pos,
        };
        let map_to_pos = match to_cylinder {
            Cylinder::Tile { pos } => pos,
            Cylinder::Container { .. } | Cylinder::Inventory { .. } => player_pos,
        };

        // Range check — player must be able to see source
        if from_pos.x != 0xFFFF {
            // Z-level check — TFS uses `mapFromPos` (`game.cpp` ~965).
            if player_pos.z != map_from_pos.z {
                let rv = if player_pos.z > map_from_pos.z {
                    ReturnValue::FirstGoUpStairs
                } else {
                    ReturnValue::FirstGoDownStairs
                };
                self.send_cancel_message(conn_id, rv);
                return;
            }
            // Distance check — walk to item first if out of range (`game.cpp` ~970–983).
            let dx = (player_pos.x as i32 - map_from_pos.x as i32).unsigned_abs();
            let dy = (player_pos.y as i32 - map_from_pos.y as i32).unsigned_abs();
            if dx > 1 || dy > 1 {
                if to_pos.x != 0xFFFF
                    && !self.throw_dest_reachable_after_walk_to_item(
                        cid,
                        map_from_pos,
                        map_to_pos,
                        item_throw_range,
                    )
                {
                    self.send_cancel_message(conn_id, ReturnValue::DestinationOutOfReach);
                    return;
                }
                let action = PlayerWalkAction::MoveItem {
                    from_pos,
                    sprite_id,
                    from_stack_pos,
                    to_pos,
                    count,
                };
                if !self.try_walk_to_and_action(conn_id, cid, map_from_pos, action, now) {
                    self.send_cancel_message(conn_id, ReturnValue::ThereIsNoWay);
                }
                return;
            }
        }

        // C++ ref: src/game.cpp:1046-1060 Game::playerMoveItem
        if !item_is_pickupable && player_pos.z != map_to_pos.z {
            self.send_cancel_message(conn_id, ReturnValue::DestinationOutOfReach);
            return;
        }

        let to_dx = (player_pos.x as i32 - map_to_pos.x as i32).unsigned_abs();
        let to_dy = (player_pos.y as i32 - map_to_pos.y as i32).unsigned_abs();
        if to_dx > item_throw_range || to_dy > item_throw_range {
            self.send_cancel_message(conn_id, ReturnValue::DestinationOutOfReach);
            return;
        }

        // C++ ref: src/game.cpp:1058 `canThrowObjectTo(mapFromPos, mapToPos, true, false, throwRange, throwRange)`
        if !self.can_throw_object_to(map_from_pos, map_to_pos, item_throw_range) {
            self.send_cancel_message(conn_id, ReturnValue::CannotThrow);
            return;
        }

        // Check if destination tile can accept the thrown item
        if to_pos.x != 0xFFFF && !self.can_throw_to_tile(map_to_pos, item_id) {
            self.send_cancel_message(conn_id, ReturnValue::NotEnoughRoom);
            return;
        }

        let result = self.internal_move_item(
            Some(cid),
            from_cylinder,
            to_cylinder,
            item_id,
            count as u16,
            CylinderFlags::NONE,
        );
        if let Err(rv) = result {
            self.send_cancel_message(conn_id, rv);
        }
    }

    // C++ ref: src/map.cpp:486-494 `Map::canThrowObjectTo` + `isSightClear` / `isTileClear`
    fn can_throw_object_to(&self, from: Position, to: Position, throw_range: u32) -> bool {
        if from.z != to.z {
            return false;
        }
        let dx = (from.x as i32 - to.x as i32).unsigned_abs();
        let dy = (from.y as i32 - to.y as i32).unsigned_abs();
        if dx > throw_range || dy > throw_range {
            return false;
        }
        // C++ `isSightClear` — adjacent tiles skip line checks (`map.cpp` ~573).
        if dx < 2 && dy < 2 {
            return true;
        }
        for p in crate::map::walk_grid_line(from, to) {
            if p == from || p == to {
                continue;
            }
            if !self.is_tile_clear_for_throw(p, false) {
                return false;
            }
        }
        true
    }

    /// Before walk-to-item: reject if no stand tile adjacent to the source can reach the destination.
    /// Matches post-walk `playerPos`→`mapToPos` + `canThrowObjectTo` checks in `game.cpp` (~1051–1060).
    fn throw_dest_reachable_after_walk_to_item(
        &self,
        cid: CreatureId,
        map_from: Position,
        map_to: Position,
        throw_range: u32,
    ) -> bool {
        if !self.can_throw_object_to(map_from, map_to, throw_range) {
            return false;
        }

        const ADJACENT: [(i32, i32); 8] = [
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ];
        for (ox, oy) in ADJACENT {
            let nx = map_from.x as i32 + ox;
            let ny = map_from.y as i32 + oy;
            if nx < 0 || ny < 0 {
                continue;
            }
            let stand = Position {
                x: nx as u16,
                y: ny as u16,
                z: map_from.z,
            };
            let dx = (stand.x as i32 - map_to.x as i32).unsigned_abs();
            let dy = (stand.y as i32 - map_to.y as i32).unsigned_abs();
            if dx > throw_range || dy > throw_range {
                continue;
            }
            if !crate::walk::player_can_stand_for_pathfind(self, cid, stand) {
                continue;
            }
            return true;
        }
        false
    }

    // Check if the destination tile can accept thrown items
    // C++ ref: Part of Tile::queryAdd logic for thrown items
    fn can_throw_to_tile(&self, pos: Position, item_id: ItemId) -> bool {
        let Some(tile) = self.map.get_tile(pos) else {
            // No tile means you can't throw there
            return false;
        };

        // Check tile flags first
        let body = tile.body();
        if body.flags & (crate::tile::flags::BLOCK_PROJECTILE | crate::tile::flags::BLOCK_SOLID) != 0 {
            return false;
        }

        // Check ground item
        if let Some(ground_id) = body.ground {
            let ground_item = self.items_db.items.get(&ground_id);
            if let Some(ground_type) = ground_item {
                if ground_type.block_projectile() || ground_type.block_solid() {
                    return false;
                }
            }
        }

        // Check all items on the tile
        for &iid in body.top_items.iter().chain(body.down_items.iter()) {
            let Some(item) = self.items.get(iid) else {
                continue;
            };
            if let Some(item_type) = self.items_db.items.get(&item.item_type) {
                if item_type.block_projectile() || item_type.block_solid() {
                    return false;
                }
            }
        }

        true
    }

    // C++ ref: src/map.cpp:496-508 Map::isTileClear, src/tile.cpp:27-40 Tile::hasProperty
    fn is_tile_clear_for_throw(&self, pos: Position, block_floor: bool) -> bool {
        let Some(tile) = self.map.get_tile(pos) else {
            return true;
        };

        let body = tile.body();

        if block_floor && body.ground.is_some() {
            return false;
        }

        if body.flags & crate::tile::flags::BLOCK_PROJECTILE != 0 {
            return false;
        }

        if let Some(ground_id) = body.ground {
            let ground_blocks = self
                .items_db
                .items
                .get(&ground_id)
                .map(|it| it.block_projectile())
                .unwrap_or(false);
            if ground_blocks {
                return false;
            }
        }

        for &iid in body.top_items.iter().chain(body.down_items.iter()) {
            let Some(item) = self.items.get(iid) else {
                continue;
            };
            let blocks = self
                .items_db
                .items
                .get(&item.item_type)
                .map(|it| it.block_projectile())
                .unwrap_or(false);
            if blocks {
                return false;
            }
        }

        true
    }

    /// Send a cancel message to a player.
    pub(crate) fn send_cancel_message(&mut self, conn_id: ConnId, rv: ReturnValue) {
        use tfs_rust_net::outgoing_extra::send_text_message_simple;
        const MESSAGE_STATUS_SMALL: u8 = 0x15;
        let msg = rv.description();
        self.enqueue_outgoing(conn_id, send_text_message_simple(MESSAGE_STATUS_SMALL, msg).into_bytes());
    }

    // === B.6: Tile item change broadcasts ===
    // C++ ref: src/protocolgame.cpp sendAddTileItem (~2605), sendUpdateTileItem (~2619),
    //          sendRemoveTileThing (~2633)

    /// Broadcast `sendAddTileItem` (0x6A) to all spectators.
    fn broadcast_tile_item_add(&mut self, pos: Position, item_id: ItemId, stack_pos: u8) {
        let (client_id, count, stackable, is_splash_or_fluid, is_animation) = match self.items.get(item_id) {
            Some(item) => {
                let it = self.items_db.items.get(&item.item_type);
                (
                    it.map(|t| t.client_id).unwrap_or(0),
                    item.client_count(),
                    it.map(|t| t.stackable()).unwrap_or(false),
                    it.map(|t| t.is_splash() || t.is_fluid_container()).unwrap_or(false),
                    it.map(|t| t.is_animation()).unwrap_or(false),
                )
            }
            None => return,
        };
        let pkt = tfs_rust_net::outgoing_extra::send_add_tile_item_template(
            pos, stack_pos, client_id, count, stackable, is_splash_or_fluid, is_animation, false,
        )
        .into_bytes();
        self.broadcast_to_spectators(pos, pkt);
    }

    /// Broadcast `sendUpdateTileItem` (0x6B) to all spectators.
    fn broadcast_tile_item_update(&mut self, pos: Position, item_id: ItemId, stack_pos: u8) {
        let (client_id, count, stackable, is_splash_or_fluid, is_animation) = match self.items.get(item_id) {
            Some(item) => {
                let it = self.items_db.items.get(&item.item_type);
                (
                    it.map(|t| t.client_id).unwrap_or(0),
                    item.client_count(),
                    it.map(|t| t.stackable()).unwrap_or(false),
                    it.map(|t| t.is_splash() || t.is_fluid_container()).unwrap_or(false),
                    it.map(|t| t.is_animation()).unwrap_or(false),
                )
            }
            None => return,
        };
        let pkt = tfs_rust_net::outgoing_extra::send_update_tile_item_template(
            pos, stack_pos, client_id, count, stackable, is_splash_or_fluid, is_animation, false,
        )
        .into_bytes();
        self.broadcast_to_spectators(pos, pkt);
    }

    /// Broadcast `sendRemoveTileThing` (0x6C) to all spectators.
    fn broadcast_tile_item_remove(&mut self, pos: Position, stack_pos: u8) {
        let pkt = tfs_rust_net::outgoing_extra::send_remove_tile_thing(pos, stack_pos).into_bytes();
        self.broadcast_to_spectators(pos, pkt);
    }

}

impl tfs_rust_common::ScriptContext for GameWorld {
    fn get_creature(&self, id: tfs_rust_common::ScriptCreatureId) -> Option<tfs_rust_common::ScriptCreatureData> {
        self.creatures.iter().find_map(|(cid, k)| {
            if cid.data().as_ffi() != id {
                return None;
            }
            Some(match k {
            CreatureKind::Player(p) => Some(tfs_rust_common::ScriptCreatureData {
                name: p.base.name.clone(),
                guid: p.guid,
            }),
            CreatureKind::Monster(m) => Some(tfs_rust_common::ScriptCreatureData {
                name: m.base.name.clone(),
                guid: 0, // Monsters don't have GUIDs
            }),
            CreatureKind::Npc(n) => Some(tfs_rust_common::ScriptCreatureData {
                name: n.base.name.clone(),
                guid: 0, // NPCs don't have GUIDs
            }),
            })
        }).flatten()
    }

    fn get_item(&self, id: tfs_rust_common::ScriptItemId) -> Option<tfs_rust_common::ScriptItemRef> {
        self.items
            .iter()
            .find(|(item_id, _)| item_id.data().as_ffi() == id)
            .map(|_| tfs_rust_common::ScriptItemRef(id))
    }

    fn get_config_string(&self, key: &str) -> Option<String> {
        self.config.get_string(key).ok()
    }

    fn get_player_slot_item_id(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
        slot: u8,
    ) -> Option<tfs_rust_common::ScriptItemId> {
        let cid = self
            .creatures
            .iter()
            .find(|(k, _)| k.data().as_ffi() == creature_id)
            .map(|(k, _)| k)?;
        self.get_player_inventory_item(cid, slot)
            .map(|i| i.data().as_ffi())
    }

    fn get_player_capacity(&self, creature_id: tfs_rust_common::ScriptCreatureId) -> Option<u32> {
        let _cid = self
            .creatures
            .iter()
            .find(|(k, _)| k.data().as_ffi() == creature_id)
            .map(|(k, _)| k)?;
        match self.creatures.get(_cid)? {
            CreatureKind::Player(p) => Some(p.get_capacity_u32()),
            _ => None,
        }
    }

    fn get_player_free_capacity(&self, creature_id: tfs_rust_common::ScriptCreatureId) -> Option<u32> {
        let _cid = self
            .creatures
            .iter()
            .find(|(k, _)| k.data().as_ffi() == creature_id)
            .map(|(k, _)| k)?;
        match self.creatures.get(_cid)? {
            CreatureKind::Player(p) => Some(p.get_free_capacity_u32()),
            _ => None,
        }
    }

    fn get_item_data(&self, id: tfs_rust_common::ScriptItemId) -> Option<tfs_rust_common::ScriptItemData> {
        let iid = self
            .items
            .iter()
            .find(|(item_id, _)| item_id.data().as_ffi() == id)
            .map(|(k, _)| k)?;
        let item = self.items.get(iid)?;
        let it = self.items_db.items.get(&item.item_type);
        let tw = it.map(|t| t.weight).unwrap_or(0);
        let stack = it.map(|t| t.stackable()).unwrap_or(false);
        let w = item.total_weight_oz(tw, stack);
        Some(tfs_rust_common::ScriptItemData {
            item_type: item.item_type,
            count: item.count,
            weight: w,
            name: it.map(|t| t.name.clone()).unwrap_or_default(),
        })
    }
}
