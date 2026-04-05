//! Central simulation state: entities, map, managers, DB handle.
// C++ reference: `Game` / `Map` ownership in `game.cpp`.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
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
    pub config: Arc<ConfigManager>,
    pub db: DbPool,
    /// GAME THREAD ONLY — insert/remove from IO threads must not be added without review.
    pub player_by_name: DashMap<String, CreatureId>,
    /// GAME THREAD ONLY — paired with `player_by_name`.
    pub player_by_guid: DashMap<u32, CreatureId>,
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

    pub fn new(
        map: Map,
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
            items: SlotMap::with_key(),
            map,
            events,
            config,
            db,
            player_by_name: DashMap::new(),
            player_by_guid: DashMap::new(),
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
            let m = match k {
                CreatureKind::Player(p) => p.base.master,
                CreatureKind::Monster(mo) => mo.base.master,
                CreatureKind::Npc(n) => n.base.master,
            };
            if m == Some(id) {
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
            let invisible = match k {
                CreatureKind::Player(p) => Self::has_invisible(&p.base.active_conditions),
                CreatureKind::Monster(m) => Self::has_invisible(&m.base.active_conditions),
                CreatureKind::Npc(n) => Self::has_invisible(&n.base.active_conditions),
            };
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
}
