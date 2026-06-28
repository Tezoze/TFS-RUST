//! Spawn placement, respawn consumption, and creature appear/disappear broadcasts.
// C++ reference: `game.cpp` `internalPlaceCreature` / `placeCreature` / `removeCreature`,
// `spawn.cpp` `Spawn::spawnMonster`, `protocolgame.cpp` `sendAddCreature`.
// 772 placement: `spawn_placement.rs` (`info.cc` `SearchSpawnField`, `crnonpl.cc` `LoadMonsterhomes`).

use rand::seq::SliceRandom;
use tfs_rust_common::enums::{Direction, SkullType, ZoneType};
use tfs_rust_common::ConnId;
use tfs_rust_common::Position;
use tfs_rust_content::monsters::MonsterOutfit;
use tfs_rust_net::creature_known::check_creature_known;
use tracing::{info, warn};

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

fn offset_position(center: Position, dx: i32, dy: i32) -> Position {
    Position::new(
        (center.x as i32 + dx).max(0) as u16,
        (center.y as i32 + dy).max(0) as u16,
        center.z,
    )
}

/// Chebyshev ring offsets inside `radius`, excluding (0, 0) (handled separately).
fn spawn_radius_offsets(radius: i32) -> Vec<(i32, i32)> {
    let r = radius.clamp(1, 30);
    let mut out = Vec::new();
    for dx in -r..=r {
        for dy in -r..=r {
            if dx == 0 && dy == 0 {
                continue;
            }
            if dx.abs().max(dy.abs()) <= r {
                out.push((dx, dy));
            }
        }
    }
    out
}

impl GameWorld {
    /// C++ `Spawns::startup` — force-spawn all slots once after map load (`spawn.cpp` ~197).
    pub fn startup_spawns(&mut self) {
        if self.spawns.started {
            return;
        }
        let requests = self.spawns.startup_requests();
        let slot_count = requests.len();
        let creatures_before = self.creatures.len();
        for req in requests {
            self.process_spawn_request(req);
        }
        let placed = self.creatures.len().saturating_sub(creatures_before);
        info!(
            spawn_slots = slot_count,
            creatures_placed = placed,
            "startup spawns finished"
        );
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
        let radius = slot.radius;
        match &slot.entry {
            SpawnEntryKind::Npc { name } => {
                let _ = self.spawn_npc(
                    name,
                    slot.position,
                    direction_from_spawn(slot.direction),
                    slot.position,
                    req.slot_index,
                    radius,
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
                    radius,
                    req.startup,
                    req.startup,
                );
            }
        }
    }

    /// Poll respawn timers — C++ `Spawn::checkSpawn`. Driven on the **logical** clock (`now_ms`):
    /// `server_ms` on 772, `tick_counter*50` on 1098 (audit Finding 13).
    pub fn poll_spawn_respawns(&mut self, now_ms: u64) {
        if !self.spawns.should_run_check(now_ms) {
            return;
        }
        let indices = self.spawns.due_slot_indices(now_ms);
        self.spawns.mark_checked(now_ms);
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
            // stalls the respawn (`spawn.cpp` `findPlayer`). 772 (`RadiusShrink`,
            // `crnonpl.cc:1414`): never stall — still spawn, just further out; the placement search
            // (`find_spawn_position`) already avoids occupied tiles, so a player only pushes the
            // monster outward instead of suppressing the spawn.
            let stall_on_player =
                self.mechanics.profile.spawn_near_player == crate::formulas::SpawnNearPlayer::Block;
            if blocked && stall_on_player {
                self.spawns.stall_respawn(slot_index, now_ms);
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
        spawn_radius: i32,
        startup: bool,
        extended_pos: bool,
    ) -> Option<CreatureId> {
        let mtype = match self.monsters_db.monsters.get(&name.to_lowercase()) {
            Some(t) => t.clone(),
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
            next_wakeup: None,
            last_step_server_ms: None,
            earliest_walk_server_ms: 0,
            earliest_spell_server_ms: 0,
            earliest_multiuse_server_ms: 0,
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
            earliest_attack_ms: 0,
            earliest_defend_ms: 0,
            last_defend_ms: 0,
            todo: Default::default(),
        };

        let ai_config = MonsterAiConfig::from_monster_type(&mtype);
        let cid = self
            .creatures
            .insert(CreatureKind::Monster(Monster::with_config(
                base, spawn_pos, ai_config,
            )));
        // CipSoft `TMonsterhome::Radius` — per-home roam leash (`crnonpl.cc:2157`). Carried from the
        // spawn zone radius; ≤0 (TVP `-1` / no radius) falls back to the global despawn radius.
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.home_radius = spawn_radius;
        }

        let placed = self.place_spawn_creature(
            cid,
            slot_index,
            center,
            spawn_radius,
            startup,
            !startup,
            extended_pos,
        );
        if !placed {
            warn!(
                monster = %name,
                ?center,
                spawn_radius,
                "could not place spawned monster on map"
            );
            self.creatures.remove(cid);
            // Avoid tight respawn loops on blocked tiles — C++ `checkSpawn` only advances timer on success.
            self.spawns.stall_respawn(slot_index, self.now_ms());
            return None;
        }

        self.spawns.on_creature_spawned(slot_index, cid);
        self.spawn_slot_by_creature.insert(cid, slot_index);
        self.monster_on_creature_appear_self(cid);

        if self.beat_driven_loop {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.experience = mtype.experience;
                m.corpse_id = mtype.outfit.corpse_id;
            }
            let is_summon = self
                .creatures
                .get(cid)
                .is_some_and(|k| k.base().master.is_some());
            if !is_summon {
                self.roll_monster_spawn_loot(cid, &mtype);
                self.recompute_monster_combat_from_equipment(cid);
            }
        }

        if !startup {
            let pos = self
                .creatures
                .get(cid)
                .map(|k| k.position())
                .unwrap_or(center);
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
        spawn_radius: i32,
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
            next_wakeup: None,
            last_step_server_ms: None,
            earliest_walk_server_ms: 0,
            earliest_spell_server_ms: 0,
            earliest_multiuse_server_ms: 0,
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
            earliest_attack_ms: 0,
            earliest_defend_ms: 0,
            last_defend_ms: 0,
            todo: Default::default(),
        };

        let cid = self.creatures.insert(CreatureKind::Npc(Npc {
            base,
            npc_type_id: 0,
        }));

        let placed = self.place_spawn_creature(
            cid,
            slot_index,
            center,
            spawn_radius,
            startup,
            !startup,
            extended_pos,
        );
        if !placed {
            warn!(
                npc = %name,
                ?center,
                spawn_radius,
                "could not place spawned NPC on map"
            );
            self.creatures.remove(cid);
            return None;
        }

        self.spawns.on_creature_spawned(slot_index, cid);
        self.spawn_slot_by_creature.insert(cid, slot_index);
        self.add_creature_think_check(cid);

        if !startup {
            let pos = self
                .creatures
                .get(cid)
                .map(|k| k.position())
                .unwrap_or(center);
            self.broadcast_creature_appear(cid, pos);
            self.broadcast_magic_effect(pos, 4);
        }

        Some(cid)
    }

    /// C++ `Map::placeCreature` tile search (`map.cpp` ~183); TVP uses `searchSpawnField` /
    /// `searchFreeField` within spawn radius (`gameserver/src/game.cpp`, `spawn.cpp`).
    pub(crate) fn find_and_place_creature_tfs(
        &mut self,
        cid: CreatureId,
        center: Position,
        extended_pos: bool,
        forced: bool,
        spawn_radius: i32,
    ) -> bool {
        let place_in_pz = self
            .map
            .get_tile(center)
            .map(|t| t.body().zone == ZoneType::Protection)
            .unwrap_or(false);

        let search_radius = spawn_radius.clamp(-1, 30);
        let search_radius = if search_radius < 0 { 1 } else { search_radius };

        let mut found_pos = self.try_creature_tile(cid, center, place_in_pz, forced);

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
                let try_pos = offset_position(center, dx, dy);
                if self
                    .try_creature_tile(cid, try_pos, place_in_pz, false)
                    .is_some()
                {
                    found_pos = Some(try_pos);
                    break;
                }
            }
        }

        if found_pos.is_none() && search_radius >= 2 {
            let mut offsets = spawn_radius_offsets(search_radius);
            let mut rng = rand::thread_rng();
            offsets.shuffle(&mut rng);
            for (dx, dy) in offsets {
                let try_pos = offset_position(center, dx, dy);
                if self
                    .try_creature_tile(cid, try_pos, place_in_pz, false)
                    .is_some()
                {
                    found_pos = Some(try_pos);
                    break;
                }
            }
        }

        let Some(pos) = found_pos else {
            return false;
        };

        if let Some(k) = self.creatures.get_mut(cid) {
            k.set_position(pos);
        }
        self.map.register_creature_at(pos, cid);
        true
    }

    fn try_creature_tile(
        &self,
        cid: CreatureId,
        pos: Position,
        place_in_pz: bool,
        forced: bool,
    ) -> Option<Position> {
        let tile = self.map.get_tile(pos)?;
        if place_in_pz && tile.body().zone != ZoneType::Protection {
            return None;
        }
        let flags = if forced { FLAG_IGNOREBLOCKITEM } else { 0 };
        let ret = tile_query_add_creature(self, tile, cid, flags);
        if forced || ret == ReturnValue::NoError || ret == ReturnValue::PlayerIsNotInvited {
            Some(pos)
        } else {
            None
        }
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
            tracing::warn!(
                ?cid,
                stack_raw,
                "creature appear stackpos out of range; skipping 0x6A"
            );
            return false;
        }
        let stack_pos = stack_raw as u8;
        let mut known = self
            .known_creatures_by_conn
            .remove(&conn)
            .unwrap_or_default();
        let mut can_see = |id: u32| self.can_see_creature_for_known_set(viewer, id);
        let (known_flag, remove_known) = check_creature_known(wire_id, &mut known, &mut can_see);
        let mut wire = build_add_creature_wire(self, cid, viewer);
        wire.known = known_flag;
        wire.remove_known = remove_known;
        wire.id = wire_id;
        let packet = self
            .codec
            .encode_add_tile_creature(pos, stack_pos, &wire, false)
            .into_bytes();
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
    /// `now_ms` is the logical clock (audit Finding 13).
    pub(crate) fn on_creature_removed_for_spawn(&mut self, cid: CreatureId, now_ms: u64) {
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
            let regen_ms = self
                .spawns
                .slot(slot_index)
                .filter(|s| s.respawns)
                .map(|s| s.spawntime_ms)
                .unwrap_or(0);
            let delay_ms = self.compute_respawn_delay_ms(regen_ms);
            self.spawns.on_creature_removed(slot_index, now_ms, delay_ms);
        }
    }

    /// 772 `StartMonsterhomeTimer` respawn delay — `crnonpl.cc:1296`:
    /// ```text
    /// MaxTimer = RegenerationTime;
    /// if (NumPlayers > 800)      MaxTimer = MaxTimer * 2 / 5;
    /// else if (NumPlayers > 200) MaxTimer = MaxTimer * 200 / (NumPlayers/2 + 100);
    /// Timer = random(MaxTimer/2, MaxTimer);   // glibc parity stream
    /// ```
    /// 1098 (`RespawnModel::Fixed`) returns `regen_ms` unchanged. The parity draw runs on the
    /// game thread (this is called from `remove_creature` → `on_creature_removed_for_spawn`).
    pub fn compute_respawn_delay_ms(&self, regen_ms: u64) -> u64 {
        if self.mechanics.profile.respawn_model != crate::formulas::RespawnModel::Monsterhome772 {
            return regen_ms;
        }
        if regen_ms == 0 {
            return 0;
        }
        let num_players = self.player_by_name.len() as u64;
        // C++ integer math — `MaxTimer * 2 / 5` and `MaxTimer * 200 / (NumPlayers/2 + 100)`.
        let max_timer = if num_players > 800 {
            regen_ms.saturating_mul(2) / 5
        } else if num_players > 200 {
            let denom = (num_players / 2).saturating_add(100).max(1);
            regen_ms.saturating_mul(200) / denom
        } else {
            regen_ms
        };
        let max_timer = max_timer.max(1);
        let half = max_timer / 2;
        // `parity_random(min, max)` returns `[min, max]` inclusive on glibc `random()`.
        let draw = self.parity_random(half as i32, max_timer as i32);
        draw.max(0) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spawn::SpawnManager;
    use crate::test_world::support::{
        beat_driven_test_world, ensure_walkable_tile, insert_player, minimal_world, test_player,
    };
    use std::collections::HashMap;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::time::Instant;
    use tfs_rust_common::ConnId;
    use tfs_rust_common::ProtocolVersion;
    use tfs_rust_content::monsters::{
        MonsterDatabase, MonsterDefenses, MonsterOutfit, MonsterSpellNode, MonsterType,
        MonsterTypeFlags,
    };
    use tfs_rust_content::spawns::{SpawnEntry, SpawnZone};
    use tfs_rust_net::Codec;

    fn rat_type() -> MonsterType {
        let mut melee_attrs = HashMap::new();
        melee_attrs.insert("name".into(), "melee".into());
        melee_attrs.insert("skill".into(), "15".into());
        melee_attrs.insert("attack".into(), "7".into());
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
            attack_spells: vec![MonsterSpellNode {
                element: "attack".into(),
                attributes: melee_attrs,
                attribute_children: Vec::new(),
            }],
            defenses: MonsterDefenses {
                armor: Some(1),
                defense: Some(3),
                spells: Vec::new(),
                immunity_poison: false,
                immunity_fire: false,
                immunity_energy: false,
                see_invisible: false,
            },
            talk_texts: Vec::new(),
        }
    }

    fn world_with_spawn() -> GameWorld {
        let mut world = minimal_world();
        let mut monsters = HashMap::new();
        monsters.insert("rat".into(), rat_type());
        world.monsters_db = Arc::new(MonsterDatabase { monsters });

        let home = Position::new(100, 100, 7);
        let zone = SpawnZone {
            center: home,
            radius: 3,
            entries: vec![SpawnEntry::Monster {
                name: "Rat".into(),
                position: home,
                spawntime_ms: 5_000,
                direction: Some(2),
            }],
        };
        world.spawns = SpawnManager::from_zones(vec![zone]);
        ensure_walkable_tile(&mut world.map, home, 100);
        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new(
                        (home.x as i32 + dx).max(0) as u16,
                        (home.y as i32 + dy).max(0) as u16,
                        home.z,
                    ),
                    100,
                );
            }
        }
        world
    }

    #[test]
    fn test_e0_spawn_monster_carries_combat() {
        let mut world = world_with_spawn();
        world.startup_spawns();
        assert_eq!(world.creatures.len(), 1);
        let (_, kind) = world.creatures.iter().next().unwrap();
        let crate::creature::CreatureKind::Monster(m) = kind else {
            panic!("expected monster");
        };
        assert_eq!(m.melee_skill, 15);
        assert_eq!(m.melee_attack, 7);
        assert_eq!(m.defense, 3);
        assert_eq!(m.armor, 1);
        assert_eq!(m.poison_cycles, 0);
        assert!(m.spells.is_empty());
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
        assert_eq!(
            world.creatures.len(),
            1,
            "must not spawn duplicate while slot.current is set"
        );
    }

    #[test]
    fn respawn_queues_appear_packet() {
        let mut world = world_with_spawn();
        world.startup_spawns();
        let (monster_cid, _) = world.creatures.iter().next().unwrap();
        let viewer = insert_player(&mut world, test_player("Spec", Position::new(101, 100, 7)));
        let conn = ConnId(1);
        world.conn_to_creature.insert(conn, viewer);
        world.known_creatures_by_conn.insert(conn, HashSet::new());

        world.remove_creature(monster_cid);
        world.pending_outgoing.clear();

        // Respawn now runs on the logical clock (audit Finding 13): advance `now_ms` (= tick_counter*50
        // on the 1098 on_tick path) past the slot's respawn deadline + check interval.
        world.tick_counter = 1_000_000;
        let later = Instant::now() + std::time::Duration::from_secs(6);
        world.on_tick(later);

        let packets = world.pending_outgoing.get(&conn);
        assert!(packets.is_some_and(|p| p.iter().any(|b| !b.is_empty() && b[0] == 0x6A)));
    }

    /// OTCv8 772 `parseTileAddThing`: position then `getThing()` — no stackpos byte (`GameTileAddThingWithStackpos` is 841+).
    #[test]
    fn respawn_appear_772_creature_marker_follows_position() {
        let mut world = world_with_spawn();
        world.codec = Codec::from_version(ProtocolVersion::V772).expect("772 codec");
        world.mechanics = crate::formulas::Mechanics::for_version(ProtocolVersion::V772);
        // Spectator must see the spawn tile but stay off the home coord; radius-shrink
        // suppresses respawns when any player is within the C++ search window.
        world.mechanics.profile.spawn_near_player =
            crate::formulas::SpawnNearPlayer::Block;
        world.startup_spawns();
        let (monster_cid, _) = world.creatures.iter().next().unwrap();
        let viewer = insert_player(&mut world, test_player("Spec", Position::new(101, 100, 7)));
        let conn = ConnId(3);
        world.conn_to_creature.insert(conn, viewer);
        world.known_creatures_by_conn.insert(conn, HashSet::new());

        world.remove_creature(monster_cid);
        world.pending_outgoing.clear();

        // Respawn on the logical clock (audit Finding 13) — advance `now_ms` past the deadline.
        world.tick_counter = 1_000_000;
        let later = Instant::now() + std::time::Duration::from_secs(6);
        world.on_tick(later);
        let monsters = world
            .creatures
            .iter()
            .filter(|(_, k)| matches!(k, CreatureKind::Monster(_)))
            .count();
        assert_eq!(monsters, 1, "772 classic respawn should place one monster");

        let packets = world.pending_outgoing.get(&conn);
        let appear = packets
            .and_then(|packets| packets.iter().find(|b| !b.is_empty() && b[0] == 0x6A))
            .expect("0x6A appear packet");
        assert_eq!(
            appear[6], 0x61,
            "creature marker low byte must follow position"
        );
        assert_eq!(appear[7], 0x00, "unknown creature marker is 0x0061");
    }

    #[test]
    fn classic772_first_slot_spawns_within_one_tile_of_home() {
        let mut world = minimal_world();
        world.mechanics = crate::formulas::Mechanics::for_version(ProtocolVersion::V772);
        let mut monsters = HashMap::new();
        monsters.insert("rat".into(), rat_type());
        world.monsters_db = Arc::new(MonsterDatabase { monsters });

        let home = Position::new(100, 100, 7);
        for dx in -2..=2 {
            for dy in -2..=2 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new(
                        (home.x as i32 + dx).max(0) as u16,
                        (home.y as i32 + dy).max(0) as u16,
                        home.z,
                    ),
                    100,
                );
            }
        }

        let zone = SpawnZone {
            center: home,
            radius: 10,
            entries: vec![
                SpawnEntry::Monster {
                    name: "Rat".into(),
                    position: home,
                    spawntime_ms: 5_000,
                    direction: Some(2),
                },
                SpawnEntry::Monster {
                    name: "Rat".into(),
                    position: home,
                    spawntime_ms: 5_000,
                    direction: Some(2),
                },
            ],
        };
        world.spawns = SpawnManager::from_zones(vec![zone]);
        world.startup_spawns();
        assert_eq!(world.creatures.len(), 2);

        let mut positions = Vec::new();
        for (cid, kind) in world.creatures.iter() {
            let CreatureKind::Monster(_) = kind else {
                continue;
            };
            positions.push(kind.position());
            let _ = cid;
        }
        let cheb = |p: Position| {
            (p.x as i32 - home.x as i32)
                .abs()
                .max((p.y as i32 - home.y as i32).abs())
        };
        assert!(
            positions.iter().any(|p| cheb(*p) <= 1),
            "first slot should land within radius-1 search of home, got {positions:?}"
        );
    }

    #[test]
    fn disappear_on_death_broadcasts_remove() {
        let mut world = world_with_spawn();
        world.startup_spawns();
        let (monster_cid, _) = world.creatures.iter().next().unwrap();
        let viewer = insert_player(&mut world, test_player("Spec", Position::new(101, 100, 7)));
        let conn = ConnId(2);
        world.conn_to_creature.insert(conn, viewer);
        world.known_creatures_by_conn.insert(conn, HashSet::new());

        world.remove_creature(monster_cid);

        let packets = world.pending_outgoing.get(&conn);
        assert!(packets.is_some_and(|p| p.iter().any(|b| !b.is_empty() && b[0] == 0x6C)));
    }

    // --- Phase 6: 772 respawn timing (Finding 18, `crnonpl.cc:1296` StartMonsterhomeTimer) ---

    /// 772 respawn delay falls in `[regen/2, regen]` with no players online.
    #[test]
    fn respawn_772_randomized_in_regen_band() {
        let mut world = beat_driven_test_world();
        world.mechanics.profile.respawn_model =
            crate::formulas::RespawnModel::Monsterhome772;
        // regen = 60s; no players → max_timer = 60s; draw ∈ [30s, 60s].
        let regen = 60_000u64;
        for _ in 0..50 {
            let delay = world.compute_respawn_delay_ms(regen);
            assert!(
                delay >= regen / 2 && delay <= regen,
                "delay {delay} outside [{}, {}]",
                regen / 2,
                regen
            );
        }
    }

    /// 772 respawn scales down above 200 players: `max_timer = regen * 200 / (n/2 + 100)`.
    #[test]
    fn respawn_772_scales_down_above_200_players() {
        let mut world = beat_driven_test_world();
        world.mechanics.profile.respawn_model =
            crate::formulas::RespawnModel::Monsterhome772;
        // Insert 300 fake players by name to drive `player_by_name.len()`.
        for i in 0..300 {
            world
                .player_by_name
                .insert(format!("P{i}"), CreatureId::default());
        }
        let regen = 60_000u64;
        // n=300 → denom = 150+100 = 250 → max_timer = 60000*200/250 = 48000.
        // draw ∈ [24000, 48000] — strictly below the no-load [30000, 60000] band.
        for _ in 0..50 {
            let delay = world.compute_respawn_delay_ms(regen);
            assert!(
                delay >= 24_000 && delay <= 48_000,
                "delay {delay} outside [24000, 48000] for 300 players"
            );
        }
    }

    /// 772 respawn halves above 800 players: `max_timer = regen * 2 / 5`.
    #[test]
    fn respawn_772_halves_above_800_players() {
        let mut world = beat_driven_test_world();
        world.mechanics.profile.respawn_model =
            crate::formulas::RespawnModel::Monsterhome772;
        for i in 0..900 {
            world
                .player_by_name
                .insert(format!("P{i}"), CreatureId::default());
        }
        let regen = 60_000u64;
        // n=900 → max_timer = 60000*2/5 = 24000. draw ∈ [12000, 24000].
        for _ in 0..50 {
            let delay = world.compute_respawn_delay_ms(regen);
            assert!(
                delay >= 12_000 && delay <= 24_000,
                "delay {delay} outside [12000, 24000] for 900 players"
            );
        }
    }

    /// 1098 respawn stays fixed at `spawntime_ms` regardless of player count.
    #[test]
    fn respawn_1098_stays_fixed() {
        let mut world = beat_driven_test_world();
        world.mechanics.profile.respawn_model = crate::formulas::RespawnModel::Fixed;
        for i in 0..500 {
            world
                .player_by_name
                .insert(format!("P{i}"), CreatureId::default());
        }
        let regen = 60_000u64;
        for _ in 0..10 {
            assert_eq!(world.compute_respawn_delay_ms(regen), regen);
        }
    }
}
