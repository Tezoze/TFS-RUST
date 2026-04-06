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
use crate::creature::CreatureKind;
use crate::cylinder::{Cylinder, CylinderFlags};
use crate::decay::DecayManager;
use crate::event_dispatcher::EventDispatcher;
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

            // Scripting event - onLogout (Phase 8)
            // C++: if (!g_creatureEvents->playerLogout(player)) return;
            // For now, we assume the event allows logout (no Lua scripting yet)
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
    pub fn on_tick(&mut self, _now: std::time::Instant) {
        if self.walk_wake_tx.is_none() {
            self.process_walk_deadlines();
        }

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
        let cap = p.capacity.max(0) as u32;
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
            free_capacity: cap,
            total_capacity: cap,
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
        // Container (y & 0x40)
        if pos.y & 0x40 != 0 {
            let _from_cid = pos.y & 0x0F;
            // TODO(Phase B.7): resolve player container by client cid
            tracing::debug!("Container cylinder lookup cid={} not yet wired", _from_cid);
            return None;
        }
        // Inventory slot
        Some(Cylinder::Inventory {
            player_id: cid,
            slot: pos.y as u8,
        })
    }

    /// Resolve a client-encoded position to a `Thing` (item or creature on a tile).
    // C++ ref: src/game.cpp:213 Game::internalGetThing (STACKPOS_MOVE path)
    pub fn internal_get_thing_move(&self, _cid: CreatureId, pos: Position, _stack_pos: u8) -> Option<Thing> {
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
        // Container slot
        if pos.y & 0x40 != 0 {
            let _from_cid = pos.y & 0x0F;
            let _slot = pos.z;
            // TODO(Phase B.7): resolve item in container by slot
            return None;
        }
        // Inventory
        // TODO(Phase C): resolve inventory item
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
            Cylinder::Container { .. } => {
                // TODO(Phase B.7): check container.contains(item_id)
                Ok(())
            }
            Cylinder::Inventory { .. } => {
                // TODO(Phase C): check inventory slot
                Ok(())
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
        from_cylinder: Cylinder,
        to_cylinder: Cylinder,
        item_id: ItemId,
        count: u16,
        flags: CylinderFlags,
    ) -> Result<ItemId, ReturnValue> {
        // Validate source has the item
        self.validate_item_in_cylinder(&from_cylinder, item_id)?;

        // For tile destinations, check queryAdd
        if let Cylinder::Tile { pos } = to_cylinder {
            let rv = self.query_add_item_to_tile(pos, item_id, flags);
            if rv.is_error() {
                return Err(rv);
            }
        }

        let item = self.items.get(item_id).ok_or(ReturnValue::NotPossible)?;
        let is_stackable = self.items_db.items.get(&item.item_type).map(|t| t.stackable()).unwrap_or(false);
        let item_count = item.count;
        let item_type = item.item_type;

        let m = if is_stackable { count.min(item_count) } else { item_count };

        match (&from_cylinder, &to_cylinder) {
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
                let from_pos = *from_pos;
                let stack_pos = self.map.get_tile(from_pos)
                    .and_then(|t| t.get_item_stack_pos(item_id))
                    .unwrap_or(0);
                let tile = self.map.get_tile_mut(from_pos).ok_or(ReturnValue::NotPossible)?;
                if tile.remove_item_by_id(item_id).is_none() {
                    return Err(ReturnValue::NotPossible);
                }
                self.broadcast_tile_item_remove(from_pos, stack_pos);
                // TODO(Phase B.7): add to container
                tracing::debug!("item {:?} moved tile→container (container add pending)", item_id);
                Ok(item_id)
            }
            (Cylinder::Container { .. }, Cylinder::Tile { pos: to_pos }) => {
                let to_pos = *to_pos;
                // TODO(Phase B.7): remove from container
                tracing::debug!("item {:?} moved container→tile (container remove pending)", item_id);
                self.internal_add_item_to_tile(to_pos, item_id, flags)
            }
            (Cylinder::Container { .. }, Cylinder::Container { .. }) => {
                // TODO(Phase B.7): container→container
                tracing::debug!("item {:?} moved container→container (pending)", item_id);
                Ok(item_id)
            }
            _ => {
                // Inventory moves — Phase C
                tracing::warn!("Inventory cylinder moves not yet implemented");
                Err(ReturnValue::NotPossible)
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
                self.player_move_item(conn_id, cid, from_pos, sprite_id, from_stack_pos, to_pos, count, item_id);
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
        _from_stack_pos: u8,
        to_pos: Position,
        count: u8,
        item_id: ItemId,
    ) {
        // Verify client sprite ID matches
        if let Some(item) = self.items.get(item_id) {
            let client_id = self.items_db.items.get(&item.item_type).map(|t| t.client_id).unwrap_or(0);
            if client_id != sprite_id {
                self.send_cancel_message(conn_id, ReturnValue::NotPossible);
                return;
            }
            // Check moveable
            let is_moveable = self.items_db.items.get(&item.item_type).map(|t| t.moveable()).unwrap_or(false);
            if !is_moveable {
                self.send_cancel_message(conn_id, ReturnValue::NotMoveable);
                return;
            }
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

        // Range check — player must be able to see source
        if let Some(player) = self.creatures.get(cid) {
            let player_pos = player.position();
            if from_pos.x != 0xFFFF {
                // Z-level check
                if player_pos.z != from_pos.z {
                    let rv = if player_pos.z > from_pos.z {
                        ReturnValue::FirstGoUpStairs
                    } else {
                        ReturnValue::FirstGoDownStairs
                    };
                    self.send_cancel_message(conn_id, rv);
                    return;
                }
                // Distance check
                let dx = (player_pos.x as i32 - from_pos.x as i32).unsigned_abs();
                let dy = (player_pos.y as i32 - from_pos.y as i32).unsigned_abs();
                if dx > 1 || dy > 1 {
                    self.send_cancel_message(conn_id, ReturnValue::TooFarAway);
                    return;
                }
            }
        }

        let result = self.internal_move_item(from_cylinder, to_cylinder, item_id, count as u16, CylinderFlags::NONE);
        if let Err(rv) = result {
            self.send_cancel_message(conn_id, rv);
        }
    }

    /// Send a cancel message to a player.
    fn send_cancel_message(&mut self, conn_id: ConnId, rv: ReturnValue) {
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

    // === B.7: Container UI packet handlers ===
    // C++ ref: src/game.cpp Game::playerCloseContainer, playerBrowseField, etc.

    /// Handle CloseContainer packet.
    pub fn player_close_container(&mut self, _conn_id: ConnId, _cid: CreatureId, _client_cid: u8) {
        // TODO: look up container by client_cid, remove viewer, send close packet
        tracing::debug!("player_close_container: stub");
    }

    /// Handle UpArrowContainer packet (go to parent container).
    pub fn player_up_container(&mut self, _conn_id: ConnId, _cid: CreatureId, _client_cid: u8) {
        // TODO: look up parent container, send new container open
        tracing::debug!("player_up_container: stub");
    }

    /// Handle UpdateContainer packet (refresh container view).
    pub fn player_update_container(&mut self, _conn_id: ConnId, _cid: CreatureId, _client_cid: u8) {
        // TODO: re-send container content
        tracing::debug!("player_update_container: stub");
    }

    /// Handle SeekInContainer packet (pagination scroll).
    pub fn player_seek_in_container(&mut self, _conn_id: ConnId, _cid: CreatureId, _client_cid: u8, _first_index: u16) {
        // TODO: send container with new first_index
        tracing::debug!("player_seek_in_container: stub");
    }
}
