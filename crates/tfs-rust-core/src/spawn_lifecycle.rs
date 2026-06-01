//! Spawn placement, respawn consumption, and creature appear/disappear broadcasts.
// C++ reference: `game.cpp` `internalPlaceCreature` / `placeCreature` / `removeCreature`,
// `spawn.cpp` `Spawn::spawnMonster`, `protocolgame.cpp` `sendAddCreature`.

use std::time::Instant;

use rand::seq::SliceRandom;
use tracing::warn;
use tfs_rust_common::enums::{Direction, SkullType, ZoneType};
use tfs_rust_common::ConnId;
use tfs_rust_common::Position;
use tfs_rust_content::monsters::MonsterOutfit;
use tfs_rust_net::creature_known::check_creature_known;

use crate::creature::CreatureBase;
use crate::creature::CreatureKind;
use crate::creature::{Monster, MonsterAiConfig, Npc, Outfit};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::login_out::{build_add_creature_wire, creature_wire_id};
use crate::player_flags::{flags_for_group, has_player_flag, PLAYER_FLAG_IGNORED_BY_MONSTERS};
use crate::return_value::ReturnValue;
use crate::spawn::{SpawnEntryKind, SpawnRequest};
use crate::tile::client_creature_stack_pos;
use crate::walk::{tile_query_add_creature, FLAG_IGNOREBLOCKITEM};

fn direction_from_spawn(dir: Option<u16>) -> Direction {
    match dir.unwrap_or(2) {
        0 => Direction::North,
        1 => Direction::East,
        2 => Direction::South,
        3 => Direction::West,
        _ => Direction::South,
    }
}

fn monster_outfit_to_base(o: &MonsterOutfit) -> Outfit {
    Outfit {
        look_type: o.look_type,
        look_head: o.look_head,
        look_body: o.look_body,
        look_legs: o.look_legs,
        look_feet: o.look_feet,
        look_addons: o.look_addons,
    }
}

const EXTENDED_REL: [(i32, i32); 13] = [
    (0, -2),
    (-1, -1),
    (0, -1),
    (1, -1),
    (-2, 0),
    (-1, 0),
    (1, 0),
    (2, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
    (0, 2),
    (0, 0),
];

const NORMAL_REL: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

impl GameWorld {
    /// C++ `Spawns::startup` — force-spawn all slots once after map load (`spawn.cpp` ~197).
    pub fn startup_spawns(&mut self) {
        if self.spawns.started {
            return;
        }
        let requests = self.spawns.startup_requests();
        for req in requests {
            self.process_spawn_request(req);
        }
        self.spawns.started = true;
    }

    /// Execute one spawn plan entry from [`crate::spawn::SpawnManager`].
    pub fn process_spawn_request(&mut self, req: SpawnRequest) {
        if self
            .spawns
            .slot(req.slot_index)
            .and_then(|s| s.current)
            .is_some()
        {
            return;
        }

        let Some(slot) = self.spawns.slot(req.slot_index).cloned() else {
            return;
        };
        match &slot.entry {
            SpawnEntryKind::Npc { name } => {
                let _ = self.spawn_npc(
                    name,
                    slot.position,
                    direction_from_spawn(slot.direction),
                    slot.position,
                    req.slot_index,
                    req.startup,
                    req.startup,
                );
            }
            SpawnEntryKind::Monster { .. } | SpawnEntryKind::Monsters { .. } => {
                let Some(name) = req.monster_name else {
                    return;
                };
                let _ = self.spawn_monster(
                    &name,
                    slot.position,
                    direction_from_spawn(slot.direction),
                    slot.position,
                    req.slot_index,
                    req.startup,
                    req.startup,
                );
            }
        }
    }

    /// Poll respawn timers — C++ `Spawn::checkSpawn` driven from `GameWorld::on_tick`.
    pub fn poll_spawn_respawns(&mut self, now: Instant) {
        if !self.spawns.should_run_check(now) {
            return;
        }
        let indices = self.spawns.due_slot_indices(now);
        self.spawns.mark_checked(now);
        for slot_index in indices {
            if self
                .spawns
                .slot(slot_index)
                .and_then(|s| s.current)
                .is_some()
            {
                continue;
            }
            let blocked = self
                .spawns
                .slot(slot_index)
                .map(|s| self.spawn_find_player(s.position))
                .unwrap_or(false);
            // B3.4 — spawn-near-player policy. TFS 1.4.2 (`Block`): a player on the spawn block tile
            // stalls the respawn (`spawn.cpp` `findPlayer`). CipSoft 7.72 (`RadiusShrink`,
            // `crnonpl.cc:1414`): never stall — still spawn, just further out; the placement search
            // (`find_spawn_position`) already avoids occupied tiles, so a player only pushes the
            // monster outward instead of suppressing the spawn.
            let stall_on_player =
                self.mechanics.profile.spawn_near_player == crate::formulas::SpawnNearPlayer::Block;
            if blocked && stall_on_player {
                self.spawns.stall_respawn(slot_index, now);
                continue;
            }
            let Some(slot) = self.spawns.slot(slot_index).cloned() else {
                continue;
            };
            if let Some(req) = crate::spawn::build_spawn_request(slot_index, &slot, false) {
                self.process_spawn_request(req);
            }
        }
    }

    /// C++ `Spawn::findPlayer` — player on spawn tile blocks respawn (`spawn.cpp` ~256).
    pub fn spawn_find_player(&self, pos: Position) -> bool {
        let Some(tile) = self.map.get_tile(pos) else {
            return false;
        };
        for &cid in &tile.body().creatures {
            let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
                continue;
            };
            if p.ghost_mode {
                continue;
            }
            let flags = flags_for_group(&self.groups, p.group_id);
            if has_player_flag(flags, PLAYER_FLAG_IGNORED_BY_MONSTERS) {
                continue;
            }
            return true;
        }
        false
    }

    /// C++ `Spawn::spawnMonster` + `Game::internalPlaceCreature` / `placeCreature`.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_monster(
        &mut self,
        name: &str,
        center: Position,
        dir: Direction,
        spawn_pos: Position,
        slot_index: usize,
        startup: bool,
        extended_pos: bool,
    ) -> Option<CreatureId> {
        let mtype = match self.monsters_db.monsters.get(&name.to_lowercase()) {
            Some(t) => t,
            None => {
                warn!(monster = %name, "spawn: unknown monster type");
                return None;
            }
        };
        if !self.events.on_monster_spawn(name, center, startup) {
            return None;
        }

        let max_hp = mtype.health_max.max(1) as i32;
        let now_hp = if mtype.health_now > 0 {
            mtype.health_now as i32
        } else {
            max_hp
        };
        let speed = mtype.speed as i32;

        let base = CreatureBase {
            name: mtype.name.clone(),
            position: center,
            direction: dir,
            health: now_hp,
            max_health: max_hp,
            outfit: monster_outfit_to_base(&mtype.outfit),
            speed,
            base_speed: speed,
            skull: SkullType::None,
            drunkenness: 0,
            active_conditions: Vec::new(),
            walk_queue: Default::default(),
            last_step: None,
            last_step_cost: 1,
            last_step_ground_speed: 150,
            next_walk_check: None,
            walk_timer: Default::default(),
            cancel_next_walk: false,
            force_update_follow_path: false,
            walk_update_ticks: 0,
            is_updating_path: false,
            has_follow_path: false,
            movement_blocked: false,
            stairhop_blocked_until: None,
            follow_target: None,
            attack_target: None,
            master: None,
            damage_map: Default::default(),
            think_check_bucket: None,
        };

        let ai_config = MonsterAiConfig::from(mtype.flags);
        let cid = self.creatures.insert(CreatureKind::Monster(Monster::with_config(
            base,
            spawn_pos,
            ai_config,
        )));

        let placed = self.find_and_place_creature(cid, center, extended_pos, !startup);
        if !placed {
            warn!(
                monster = %name,
                ?center,
                "could not place spawned monster on map"
            );
            self.creatures.remove(cid);
            // Avoid tight respawn loops on blocked tiles — C++ `checkSpawn` only advances timer on success.
            self.spawns.stall_respawn(slot_index, std::time::Instant::now());
            return None;
        }

        self.spawns.on_creature_spawned(slot_index, cid);
        self.spawn_slot_by_creature.insert(cid, slot_index);
        self.monster_on_creature_appear_self(cid);

        if !startup {
            let pos = self.creatures.get(cid).map(|k| k.position()).unwrap_or(center);
            self.broadcast_creature_appear(cid, pos);
            self.broadcast_magic_effect(pos, 4);
        }

        Some(cid)
    }

    /// NPC spawn from spawn XML — no respawn timer (C++ `Spawns::startup` NPC path).
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_npc(
        &mut self,
        name: &str,
        center: Position,
        dir: Direction,
        _spawn_pos: Position,
        slot_index: usize,
        startup: bool,
        extended_pos: bool,
    ) -> Option<CreatureId> {
        let base = CreatureBase {
            name: name.to_string(),
            position: center,
            direction: dir,
            health: 100,
            max_health: 100,
            outfit: Outfit::default(),
            speed: 100,
            base_speed: 100,
            skull: SkullType::None,
            drunkenness: 0,
            active_conditions: Vec::new(),
            walk_queue: Default::default(),
            last_step: None,
            last_step_cost: 1,
            last_step_ground_speed: 150,
            next_walk_check: None,
            walk_timer: Default::default(),
            cancel_next_walk: false,
            force_update_follow_path: false,
            walk_update_ticks: 0,
            is_updating_path: false,
            has_follow_path: false,
            movement_blocked: false,
            stairhop_blocked_until: None,
            follow_target: None,
            attack_target: None,
            master: None,
            damage_map: Default::default(),
            think_check_bucket: None,
        };

        let cid = self
            .creatures
            .insert(CreatureKind::Npc(Npc {
                base,
                npc_type_id: 0,
            }));

        let placed = self.find_and_place_creature(cid, center, extended_pos, !startup);
        if !placed {
            warn!(npc = %name, ?center, "could not place spawned NPC on map");
            self.creatures.remove(cid);
            return None;
        }

        self.spawns.on_creature_spawned(slot_index, cid);
        self.spawn_slot_by_creature.insert(cid, slot_index);
        self.add_creature_think_check(cid);

        if !startup {
            let pos = self.creatures.get(cid).map(|k| k.position()).unwrap_or(center);
            self.broadcast_creature_appear(cid, pos);
            self.broadcast_magic_effect(pos, 4);
        }

        Some(cid)
    }

    /// C++ `Map::placeCreature` tile search (`map.cpp` ~183).
    fn find_and_place_creature(
        &mut self,
        cid: CreatureId,
        center: Position,
        extended_pos: bool,
        forced: bool,
    ) -> bool {
        let place_in_pz = self
            .map
            .get_tile(center)
            .map(|t| t.body().zone == ZoneType::Protection)
            .unwrap_or(false);

        let mut found_pos = None;

        if let Some(tile) = self.map.get_tile(center) {
            let ret = tile_query_add_creature(self, tile, cid, FLAG_IGNOREBLOCKITEM);
            if forced
                || ret == ReturnValue::NoError
                || ret == ReturnValue::PlayerIsNotInvited
            {
                found_pos = Some(center);
            }
        }

        if found_pos.is_none() {
            let mut rel: Vec<(i32, i32)> = if extended_pos {
                EXTENDED_REL[..12].to_vec()
            } else {
                NORMAL_REL.to_vec()
            };
            let mut rng = rand::thread_rng();
            if extended_pos {
                rel[..4].shuffle(&mut rng);
                rel[4..].shuffle(&mut rng);
            } else {
                rel.shuffle(&mut rng);
            }

            for (dx, dy) in rel {
                let try_pos = Position::new(
                    (center.x as i32 + dx).max(0) as u16,
                    (center.y as i32 + dy).max(0) as u16,
                    center.z,
                );
                let Some(tile) = self.map.get_tile(try_pos) else {
                    continue;
                };
                if place_in_pz && tile.body().zone != ZoneType::Protection {
                    continue;
                }
                if tile_query_add_creature(self, tile, cid, 0) != ReturnValue::NoError {
                    continue;
                }
                found_pos = Some(try_pos);
                break;
            }
        }

        let Some(pos) = found_pos else {
            return false;
        };

        if let Some(k) = self.creatures.get_mut(cid) {
            k.set_position(pos);
        }
        self.map.register_creature_index(pos, cid);
        if let Some(t) = self.map.get_tile_mut(pos) {
            t.add_creature(cid);
        }
        true
    }

    /// Whether `conn` received a full `AddCreature` block for `wire_id`.
    pub(crate) fn is_creature_fully_sent_to_conn(&self, conn: ConnId, wire_id: u32) -> bool {
        self.creature_fully_sent_by_conn
            .get(&conn)
            .is_some_and(|s| s.contains(&wire_id))
    }

    /// C++ `ProtocolGame::sendAddCreature` for one viewer (`protocolgame.cpp` ~2730).
    pub(crate) fn send_creature_appear_to_conn(
        &mut self,
        conn: ConnId,
        viewer: CreatureId,
        cid: CreatureId,
        pos: Position,
    ) -> bool {
        let wire_id = match self.creatures.get(cid) {
            Some(k) => creature_wire_id(cid, k),
            None => return false,
        };
        let stack_raw = self
            .map
            .get_tile(pos)
            .map(|t| client_creature_stack_pos(t.body(), cid))
            .unwrap_or(-1);
        if !(0..10).contains(&stack_raw) {
            tracing::warn!(?cid, stack_raw, "creature appear stackpos out of range; skipping 0x6A");
            return false;
        }
        let stack_pos = stack_raw as u8;
        let mut known = self
            .known_creatures_by_conn
            .remove(&conn)
            .unwrap_or_default();
        let mut can_see = |id: u32| self.can_see_creature_for_known_set(viewer, id);
        let (known_flag, remove_known) =
            check_creature_known(wire_id, &mut known, &mut can_see);
        let mut wire = build_add_creature_wire(self, cid, viewer);
        wire.known = known_flag;
        wire.remove_known = remove_known;
        wire.id = wire_id;
        let packet = self.codec.encode_add_tile_creature(pos, stack_pos, &wire).into_bytes();
        self.known_creatures_by_conn.insert(conn, known);
        if !known_flag {
            self.mark_creature_fully_sent(conn, wire_id);
        }
        self.enqueue_outgoing(conn, packet);
        true
    }

    /// C++ `ProtocolGame::sendRemoveTileCreature` for one viewer.
    pub(crate) fn send_creature_remove_to_conn(
        &mut self,
        conn: ConnId,
        cid: CreatureId,
        pos: Position,
        stack_raw: i32,
    ) {
        let wire_id = match self.creatures.get(cid) {
            Some(k) => creature_wire_id(cid, k),
            None => return,
        };
        let packet = if (0..10).contains(&stack_raw) {
            self.codec
                .encode_remove_tile_thing(pos, stack_raw as u8)
                .into_bytes()
        } else {
            self.codec
                .encode_remove_tile_creature_by_id(wire_id)
                .into_bytes()
        };
        self.enqueue_outgoing(conn, packet);
        if let Some(known) = self.known_creatures_by_conn.get_mut(&conn) {
            known.remove(&wire_id);
        }
        if let Some(sent) = self.creature_fully_sent_by_conn.get_mut(&conn) {
            sent.remove(&wire_id);
        }
    }

    /// C++ `Game::placeCreature` → `sendAddCreature` for spectators (`game.cpp` ~552).
    pub fn broadcast_creature_appear(&mut self, cid: CreatureId, pos: Position) {
        let spectators: Vec<(ConnId, CreatureId)> = self
            .conn_to_creature
            .iter()
            .filter_map(|(&conn, &viewer)| {
                if viewer == cid {
                    return None;
                }
                if self.can_see_position(viewer, pos) {
                    Some((conn, viewer))
                } else {
                    None
                }
            })
            .collect();

        for (conn, viewer) in spectators {
            self.send_creature_appear_to_conn(conn, viewer, cid, pos);
        }
    }

    /// C++ `Game::removeCreature` spectator strip (`game.cpp` ~577).
    pub(crate) fn broadcast_creature_disappear(
        &mut self,
        cid: CreatureId,
        pos: Position,
        stack_raw: i32,
    ) {
        let spectators: Vec<(ConnId, CreatureId)> = self
            .conn_to_creature
            .iter()
            .filter_map(|(&conn, &viewer)| {
                if viewer == cid {
                    return None;
                }
                if self.can_see_creature(viewer, cid) {
                    Some((conn, viewer))
                } else {
                    None
                }
            })
            .collect();

        for (conn, _viewer) in spectators {
            self.send_creature_remove_to_conn(conn, cid, pos, stack_raw);
        }
    }

    /// Spawn-slot cleanup + disappear broadcast hook for [`GameWorld::remove_creature`].
    pub(crate) fn on_creature_removed_for_spawn(&mut self, cid: CreatureId, now: std::time::Instant) {
        if let Some(pos) = self.creatures.get(cid).map(|k| k.position()) {
            let stack_raw = self
                .map
                .get_tile(pos)
                .map(|t| client_creature_stack_pos(t.body(), cid))
                .unwrap_or(-1);
            if !matches!(self.creatures.get(cid), Some(CreatureKind::Player(_))) {
                self.broadcast_creature_disappear(cid, pos, stack_raw);
            }
        }
        if let Some(slot_index) = self.spawn_slot_by_creature.remove(&cid) {
            self.spawns.on_creature_removed(slot_index, now);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use super::*;
    use crate::spawn::SpawnManager;
    use crate::test_world::support::{
        ensure_walkable_tile, insert_player, minimal_world, test_player,
    };
    use tfs_rust_common::ConnId;
    use tfs_rust_content::monsters::{MonsterDatabase, MonsterDefenses, MonsterOutfit, MonsterType, MonsterTypeFlags};
    use tfs_rust_content::spawns::{SpawnEntry, SpawnZone};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;

    fn rat_type() -> MonsterType {
        MonsterType {
            name: "Rat".into(),
            filename: "rat.xml".into(),
            name_description: "a rat".into(),
            race: "blood".into(),
            experience: 5,
            speed: 200,
            health_now: 20,
            health_max: 20,
            outfit: MonsterOutfit::default(),
            flags: MonsterTypeFlags::default(),
            loot: Vec::new(),
            attack_spells: Vec::new(),
            defenses: MonsterDefenses {
                armor: None,
                defense: None,
                spells: Vec::new(),
            },
        }
    }

    fn world_with_spawn() -> GameWorld {
        let mut world = minimal_world();
        let mut monsters = HashMap::new();
        monsters.insert("rat".into(), rat_type());
        world.monsters_db = Arc::new(MonsterDatabase { monsters });

        let zone = SpawnZone {
            center: Position::new(100, 100, 7),
            radius: 3,
            entries: vec![SpawnEntry::Monster {
                name: "Rat".into(),
                position: Position::new(101, 101, 7),
                spawntime_ms: 5_000,
                direction: Some(2),
            }],
        };
        world.spawns = SpawnManager::from_zones(vec![zone]);
        ensure_walkable_tile(&mut world.map, Position::new(101, 101, 7), 100);
        world
    }

    #[test]
    fn startup_spawns_places_monster_without_appear_packet() {
        let mut world = world_with_spawn();
        world.startup_spawns();
        assert_eq!(world.creatures.len(), 1);
        assert!(world.pending_outgoing.is_empty());
    }

    #[test]
    fn respawn_skips_when_slot_still_occupied() {
        let mut world = world_with_spawn();
        world.startup_spawns();
        assert_eq!(world.creatures.len(), 1);

        let req = world.spawns.startup_requests();
        assert!(req.is_empty(), "slot should be occupied after startup");

        // Force a respawn request while the live monster still holds the slot.
        let forced = crate::spawn::SpawnRequest {
            slot_index: 0,
            monster_name: Some("Rat".into()),
            startup: false,
        };
        world.process_spawn_request(forced);
        assert_eq!(world.creatures.len(), 1, "must not spawn duplicate while slot.current is set");
    }

    #[test]
    fn respawn_queues_appear_packet() {
        let mut world = world_with_spawn();
        world.startup_spawns();
        let (monster_cid, _) = world.creatures.iter().next().unwrap();
        let viewer = insert_player(
            &mut world,
            test_player("Spec", Position::new(100, 100, 7)),
        );
        let conn = ConnId(1);
        world.conn_to_creature.insert(conn, viewer);
        world.known_creatures_by_conn.insert(conn, HashSet::new());

        world.remove_creature(monster_cid);
        world.pending_outgoing.clear();

        let later = Instant::now() + std::time::Duration::from_secs(6);
        world.on_tick(later);

        let packets = world.pending_outgoing.get(&conn);
        assert!(packets.is_some_and(|p| p.iter().any(|b| !b.is_empty() && b[0] == 0x6A)));
    }

    #[test]
    fn disappear_on_death_broadcasts_remove() {
        let mut world = world_with_spawn();
        world.startup_spawns();
        let (monster_cid, _) = world.creatures.iter().next().unwrap();
        let viewer = insert_player(
            &mut world,
            test_player("Spec", Position::new(100, 100, 7)),
        );
        let conn = ConnId(2);
        world.conn_to_creature.insert(conn, viewer);
        world.known_creatures_by_conn.insert(conn, HashSet::new());

        world.remove_creature(monster_cid);

        let packets = world.pending_outgoing.get(&conn);
        assert!(packets.is_some_and(|p| p.iter().any(|b| !b.is_empty() && b[0] == 0x6C)));
    }
}
