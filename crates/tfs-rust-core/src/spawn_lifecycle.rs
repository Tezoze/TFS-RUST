//! Spawn placement, respawn consumption, and creature appear/disappear broadcasts.
// C++ reference: `game.cpp` `internalPlaceCreature` / `placeCreature` / `removeCreature`,
// `spawn.cpp` `Spawn::spawnMonster`, `protocolgame.cpp` `sendAddCreature`.
// 772 placement: `spawn_placement.rs` (`info.cc` `SearchSpawnField`, `crnonpl.cc` `LoadMonsterhomes`).

use rand::seq::SliceRandom;
use slotmap::Key;
use std::sync::Arc;
use tfs_rust_common::ConnId;
use tfs_rust_common::Position;
use tfs_rust_common::enums::{Direction, SkullType, ZoneType};
use tfs_rust_content::monsters::MonsterOutfit;
use tfs_rust_content::npcs::{DialoguePolicy, NpcAppearance};
use tfs_rust_net::creature_known::check_creature_known;
use tracing::{debug, info, warn};

use crate::creature::CreatureBase;
use crate::creature::CreatureKind;
use crate::creature::{Monster, MonsterAiConfig, Npc, NpcRuntimeState, Outfit};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::login_out::{build_add_creature_wire, creature_wire_id};
use crate::player_flags::{PLAYER_FLAG_IGNORED_BY_MONSTERS, flags_for_group, has_player_flag};
use crate::return_value::ReturnValue;
use crate::spawn::{SpawnEntryKind, SpawnRequest};
use crate::tile::client_creature_stack_pos;
use crate::walk::{FLAG_IGNOREBLOCKITEM, tile_query_add_creature};

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

fn npc_appearance_to_outfit(a: &NpcAppearance) -> Outfit {
    Outfit {
        look_type: i32::from(a.look_type),
        look_head: i32::from(a.look_head),
        look_body: i32::from(a.look_body),
        look_legs: i32::from(a.look_legs),
        look_feet: i32::from(a.look_feet),
        look_addons: i32::from(a.look_addons),
    }
}

/// Resolve spawn XML name → definition (case-insensitive).
///
/// TVP-era spawn files sometimes append `" npc"` (`cobra npc`); strip that once
/// if the primary lookup misses.
fn resolve_npc_definition(
    db: &tfs_rust_content::npcs::NpcDatabase,
    name: &str,
) -> Option<std::sync::Arc<tfs_rust_content::npcs::NpcDefinition>> {
    if let Some(d) = db.get_by_name(name) {
        return Some(Arc::clone(d));
    }
    let lower = name.to_ascii_lowercase();
    if let Some(stem) = lower.strip_suffix(" npc")
        && !stem.is_empty()
    {
        return db.get_by_name(stem).map(Arc::clone);
    }
    None
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
    /// `server_ms` for both eras (Phase 6 collapse — audit Finding 13).
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
            var_speed: 0,
            skull: SkullType::None,
            drunkenness: 0,
            active_conditions: Vec::new(),
            walk_queue: Default::default(),
            walk_destinations: Default::default(),
            last_step: None,
            last_step_cost: 1,
            last_step_ground_speed: 150,
            next_wakeup: None,
            last_step_server_ms: None,
            earliest_walk_server_ms: 0,
            earliest_spell_server_ms: 0,
            earliest_multiuse_server_ms: 0,
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
            last_hit_by: None,
            poison_damage_origin: None,
            fire_damage_origin: None,
            energy_damage_origin: None,
            earliest_attack_ms: 0,
            latest_attack_round: 0,
            earliest_defend_ms: 0,
            last_defend_ms: 0,
            learning_points: 0,
            todo: Default::default(),
            chase_mode: Default::default(),
            last_auto_walk_armed_ms: u64::MAX,
        };

        let ai_config = MonsterAiConfig::from_monster_type(&mtype);
        let cid = self
            .creatures
            .insert(CreatureKind::Monster(Monster::with_config(
                base, spawn_pos, ai_config,
            )));
        crate::login_out::assign_creature_wire_id(self, cid);
        // CipSoft `TMonsterhome::Radius` — per-home roam leash (`crnonpl.cc:2157`). Carried from the
        // spawn zone radius; ≤0 (TVP `-1` / no radius) falls back to the global despawn radius.
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.home_radius = spawn_radius;
        }

        // TFS `placeCreature(..., force)` is true on startup; Classic772Bfs ignores `forced`
        // (`SearchSpawnField` has no force path). Never invert — `!startup` made respawns
        // treat every ground tile as placeable (walls).
        let placed = self.place_spawn_creature(
            cid,
            slot_index,
            center,
            spawn_radius,
            startup,
            startup,
            extended_pos,
        );
        if !placed {
            // C++ `ProcessMonsterhomes` silently skips when `MaxRadius < 0` (player nearby) or
            // `SearchSpawnField` finds no valid tile (`crnonpl.cc:1457-1483`). No warning is
            // emitted — the timer is simply reset via `StartMonsterhomeTimer`. Use `debug` to
            // avoid log spam when a player stands near a spawn point after killing the monster.
            debug!(
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

        // Phase 3: both eras run the 772 spawn loot / combat recompute path.
        {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.experience = mtype.experience;
                m.corpse_id = mtype.outfit.corpse_id;
                m.blood = mtype.blood_type();
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
    /// Resolves `name` case-insensitively against [`GameWorld::npcs_db`] (NPC-3).
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
        let def = match resolve_npc_definition(&self.npcs_db, name) {
            Some(d) => d,
            None => {
                warn!(
                    npc = %name,
                    ?center,
                    "spawn: unknown NPC type"
                );
                return None;
            }
        };

        let max_hp = def.health_max.max(1) as i32;
        let speed = i32::from(def.movement.speed.max(1));
        let policy = def
            .dialogue
            .as_ref()
            .map(|d| d.policy)
            .unwrap_or(DialoguePolicy::QueuedSingleFocus);

        let base = CreatureBase {
            name: def.name.clone(),
            position: center,
            direction: dir,
            health: max_hp,
            max_health: max_hp,
            outfit: npc_appearance_to_outfit(&def.appearance),
            speed,
            base_speed: speed,
            var_speed: 0,
            skull: SkullType::None,
            drunkenness: 0,
            active_conditions: Vec::new(),
            walk_queue: Default::default(),
            walk_destinations: Default::default(),
            last_step: None,
            last_step_cost: 1,
            last_step_ground_speed: 150,
            next_wakeup: None,
            last_step_server_ms: None,
            earliest_walk_server_ms: 0,
            earliest_spell_server_ms: 0,
            earliest_multiuse_server_ms: 0,
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
            last_hit_by: None,
            poison_damage_origin: None,
            fire_damage_origin: None,
            energy_damage_origin: None,
            earliest_attack_ms: 0,
            latest_attack_round: 0,
            earliest_defend_ms: 0,
            last_defend_ms: 0,
            learning_points: 0,
            todo: Default::default(),
            chase_mode: Default::default(),
            last_auto_walk_armed_ms: u64::MAX,
        };

        let cid = self.creatures.insert(CreatureKind::Npc(Npc {
            base,
            definition: def.id,
            speech_bubble: def.speech_bubble,
            wire_id: 0,
            runtime: NpcRuntimeState::at_home(center, def.movement.radius, policy),
        }));
        crate::login_out::assign_creature_wire_id(self, cid);

        let placed = self.place_spawn_creature(
            cid,
            slot_index,
            center,
            spawn_radius,
            startup,
            startup,
            extended_pos,
        );
        if !placed {
            warn!(
                npc = %def.name,
                ?center,
                spawn_radius,
                "could not place spawned NPC on map"
            );
            self.creatures.remove(cid);
            return None;
        }

        // Placement may offset from center — pin home to the final tile.
        if let Some(CreatureKind::Npc(n)) = self.creatures.get_mut(cid) {
            n.runtime.home_position = n.base.position;
        }

        self.spawns.on_creature_spawned(slot_index, cid);
        self.spawn_slot_by_creature.insert(cid, slot_index);

        // C++ `TNPC` ctor ends with `ToDoYield` so IdleStimulus (roam/sleep) can run
        // (`crnonpl.cc:1665`).
        self.creature_todo_yield(cid);

        if !startup {
            let pos = self
                .creatures
                .get(cid)
                .map(|k| k.position())
                .unwrap_or(center);
            self.broadcast_creature_appear(cid, pos);
            self.broadcast_magic_effect(pos, 4);
        }

        if let Some(cb) = def.on_appear {
            crate::lua_scope::fire_npc_appear(self, cid, cb);
        }

        Some(cid)
    }

    /// `Game.createMonster` — PC-3a Gap 5. Like [`Self::spawn_monster`] without a spawn slot.
    /// C++ `luascript.cpp` `luaGameCreateMonster` → `Monster::createMonster` + `placeCreature`.
    pub fn lua_script_create_monster(
        &mut self,
        name: &str,
        x: u16,
        y: u16,
        z: u8,
        extended: bool,
        force: bool,
    ) -> Result<Option<u64>, String> {
        let center = Position { x, y, z };
        let mtype = match self.monsters_db.get_by_name(name) {
            Some(t) => t.clone(),
            None => return Ok(None),
        };
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
            direction: Direction::South,
            health: now_hp,
            max_health: max_hp,
            outfit: monster_outfit_to_base(&mtype.outfit),
            speed,
            base_speed: speed,
            var_speed: 0,
            skull: SkullType::None,
            drunkenness: 0,
            active_conditions: Vec::new(),
            walk_queue: Default::default(),
            walk_destinations: Default::default(),
            last_step: None,
            last_step_cost: 1,
            last_step_ground_speed: 150,
            next_wakeup: None,
            last_step_server_ms: None,
            earliest_walk_server_ms: 0,
            earliest_spell_server_ms: 0,
            earliest_multiuse_server_ms: 0,
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
            last_hit_by: None,
            poison_damage_origin: None,
            fire_damage_origin: None,
            energy_damage_origin: None,
            earliest_attack_ms: 0,
            latest_attack_round: 0,
            earliest_defend_ms: 0,
            last_defend_ms: 0,
            learning_points: 0,
            todo: Default::default(),
            chase_mode: Default::default(),
            last_auto_walk_armed_ms: u64::MAX,
        };
        let ai_config = MonsterAiConfig::from_monster_type(&mtype);
        let cid = self
            .creatures
            .insert(CreatureKind::Monster(Monster::with_config(
                base, center, ai_config,
            )));
        crate::login_out::assign_creature_wire_id(self, cid);
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.experience = mtype.experience;
            m.corpse_id = mtype.outfit.corpse_id;
            m.blood = mtype.blood_type();
        }
        if !self.find_and_place_creature_tfs(cid, center, extended, force, 0) {
            self.creatures.remove(cid);
            return Ok(None);
        }
        let placed = self
            .creatures
            .get(cid)
            .map(|k| k.position())
            .unwrap_or(center);
        self.monster_on_creature_appear_self(cid);
        self.broadcast_creature_appear(cid, placed);
        Ok(Some(cid.data().as_ffi()))
    }

    /// `creature:addSummon(monster)` — PC-3a Gap 5. Sets master; clears target/follow.
    pub fn lua_script_add_summon(
        &mut self,
        master_u64: u64,
        summon_u64: u64,
    ) -> Result<bool, String> {
        let master = self
            .resolve_creature_u64(master_u64)
            .ok_or_else(|| "addSummon: master not found".to_string())?;
        let summon = self
            .resolve_creature_u64(summon_u64)
            .ok_or_else(|| "addSummon: summon not found".to_string())?;
        let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(summon) else {
            return Ok(false);
        };
        m.base.attack_target = None;
        m.base.follow_target = None;
        m.base.master = Some(master);
        Ok(true)
    }

    /// 772 `SearchSummonField` — `info.cc:1118`. Picks a nearby free tile for `TSummonImpact`.
    ///
    /// Gates: BANK, !UNPASS, !AVOID, !PZ, !House, ThrowPossible from origin. Tie-break =
    /// `random(0, 99)` keeping the highest roll (`info.cc:1124–1140`).
    pub(crate) fn search_summon_field(&self, origin: Position, distance: i32) -> Option<Position> {
        let mut best: Option<(Position, i32)> = None;
        for dy in -distance..=distance {
            for dx in -distance..=distance {
                let tie = self.parity_random(0, 99);
                if best.is_some_and(|(_, b)| tie <= b) {
                    continue;
                }
                let x = origin.x as i32 + dx;
                let y = origin.y as i32 + dy;
                if x < 0 || y < 0 || x > i32::from(u16::MAX) || y > i32::from(u16::MAX) {
                    continue;
                }
                let pos = Position::new(x as u16, y as u16, origin.z);
                if !self.summon_field_tile_ok(origin, pos) {
                    continue;
                }
                best = Some((pos, tie));
            }
        }
        best.map(|(p, _)| p)
    }

    fn summon_field_tile_ok(&self, origin: Position, pos: Position) -> bool {
        let Some(tile) = self.map.get_tile(pos) else {
            return false;
        };
        if matches!(tile, crate::tile::Tile::House(_)) {
            return false;
        }
        if tile.body().zone == ZoneType::Protection {
            return false;
        }
        let chain = tile.body().map_object_chain();
        let Some(crate::tile::MapStackEntry::Ground(server_id)) = chain.first() else {
            return false;
        };
        if !self.items_db.is_terrain_bank(*server_id)
            || self.items_db.is_unpassable_for_field(*server_id)
        {
            return false;
        }
        // Any stack object with AVOID / UNPASS blocks (decompile `!CoordinateFlag(…, AVOID)`).
        for entry in &chain {
            match entry {
                crate::tile::MapStackEntry::Ground(sid) => {
                    if self.items_db.is_avoid_hazard(*sid) {
                        return false;
                    }
                }
                crate::tile::MapStackEntry::Item(item_id) => {
                    let Some(item) = self.items.get(*item_id) else {
                        return false;
                    };
                    let sid = item.item_type;
                    if self.items_db.is_avoid_hazard(sid)
                        || self.items_db.is_unpassable_for_field(sid)
                    {
                        return false;
                    }
                }
                crate::tile::MapStackEntry::Creature(_) => {}
            }
        }
        if !tile.body().creatures.is_empty() {
            return false;
        }
        self.monster_sight_clear(origin, pos)
    }

    /// 772 `TSummonImpact::handleField` → `CreateMonster(…, MasterID, ShowEffect)` —
    /// `magic.cc:385–395`, `crnonpl.cc:3158`.
    ///
    /// `search_origin` is the field tile from `ExecuteCircleSpell` (Origin r=0 = actor).
    /// `SearchSummonField` → `SearchFreeField` nudge → place; wire `EFFECT_ENERGY` (11).
    pub(crate) fn monster_create_summon(
        &mut self,
        master_id: CreatureId,
        race_name: &str,
        force: bool,
        search_origin: Position,
    ) -> Option<CreatureId> {
        if !self.creatures.contains_key(master_id) {
            return None;
        }
        // `TMonster` ctor reparents summon-of-summon up to the wild/player ancestor
        // (`crnonpl.cc:2012–2028`). CASTING still only *builds* IMPACT_SUMMON when Master==0.
        let effective_master = self.effective_summon_master(master_id)?;
        let summon_field = self.search_summon_field(search_origin, 2)?;
        // `CreateMonster` ignores `SearchFreeField` failure — keep SearchSummonField coords
        // (`crnonpl.cc:3169`).
        let place_at = self
            .search_free_field(summon_field, 2)
            .unwrap_or(summon_field);
        let mtype = self.monsters_db.get_by_name(race_name)?.clone();
        let max_hp = mtype.health_max.max(1) as i32;
        let now_hp = if mtype.health_now > 0 {
            mtype.health_now as i32
        } else {
            max_hp
        };
        let speed = mtype.speed as i32;
        let base = CreatureBase {
            name: mtype.name.clone(),
            position: place_at,
            direction: Direction::South,
            health: now_hp,
            max_health: max_hp,
            outfit: monster_outfit_to_base(&mtype.outfit),
            speed,
            base_speed: speed,
            var_speed: 0,
            skull: SkullType::None,
            drunkenness: 0,
            active_conditions: Vec::new(),
            walk_queue: Default::default(),
            walk_destinations: Default::default(),
            last_step: None,
            last_step_cost: 1,
            last_step_ground_speed: 150,
            next_wakeup: None,
            last_step_server_ms: None,
            earliest_walk_server_ms: 0,
            earliest_spell_server_ms: 0,
            earliest_multiuse_server_ms: 0,
            cancel_next_walk: false,
            force_update_follow_path: false,
            walk_update_ticks: 0,
            is_updating_path: false,
            has_follow_path: false,
            movement_blocked: false,
            stairhop_blocked_until: None,
            follow_target: None,
            attack_target: None,
            master: Some(effective_master),
            damage_map: Default::default(),
            last_hit_by: None,
            poison_damage_origin: None,
            fire_damage_origin: None,
            energy_damage_origin: None,
            earliest_attack_ms: 0,
            latest_attack_round: 0,
            earliest_defend_ms: 0,
            last_defend_ms: 0,
            learning_points: 0,
            todo: Default::default(),
            chase_mode: Default::default(),
            last_auto_walk_armed_ms: u64::MAX,
        };
        let ai_config = MonsterAiConfig::from_monster_type(&mtype);
        let cid = self
            .creatures
            .insert(CreatureKind::Monster(Monster::with_config(
                base, place_at, ai_config,
            )));
        crate::login_out::assign_creature_wire_id(self, cid);
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            // Summons do not grant XP / drop loot (`setDropLoot(false)` / master gate).
            m.experience = 0;
            m.corpse_id = mtype.outfit.corpse_id;
            m.blood = mtype.blood_type();
        }
        if !self.find_and_place_creature_tfs(cid, place_at, false, force, 0) {
            self.creatures.remove(cid);
            return None;
        }
        // `CreateMonster` ShowEffect → `EFFECT_ENERGY` (`enums.hh` = 11).
        let placed = self
            .creatures
            .get(cid)
            .map(|k| k.position())
            .unwrap_or(place_at);
        // Same order as `spawn_monster` (non-startup): AI bookkeeping → spectator AddCreature → effect.
        // Skipping `broadcast_creature_appear` left clients with Move/effect for an unknown id → crash.
        self.monster_on_creature_appear_self(cid);
        self.broadcast_creature_appear(cid, placed);
        self.broadcast_magic_effect(placed, 11);
        Some(cid)
    }

    /// 772 `TMonster` master-chain walk (`crnonpl.cc:2012–2028`).
    ///
    /// Rebases summon-of-summon up to the first non-monster-summon ancestor (wild monster or
    /// player). Returns `None` if the chain is broken (missing creature).
    fn effective_summon_master(&self, master_id: CreatureId) -> Option<CreatureId> {
        let mut current = master_id;
        for _ in 0..8 {
            match self.creatures.get(current) {
                Some(CreatureKind::Monster(m)) if m.base.master.is_some() => {
                    tracing::debug!(
                        ?current,
                        parent = ?m.base.master,
                        "CreateMonster: reparent summon-of-summon to grandparent"
                    );
                    current = m.base.master?;
                }
                Some(_) => return Some(current),
                None => return None,
            }
        }
        Some(current)
    }

    /// `creature:move(tile, flags)` — returns true on `RETURNVALUE_NOERROR`.
    pub fn lua_script_creature_move_to_tile(
        &mut self,
        creature_u64: u64,
        x: u16,
        y: u16,
        z: u8,
        flags: u32,
    ) -> Result<bool, String> {
        use crate::return_value::ReturnValue;
        use crate::walk::{internal_teleport_player, tile_query_add_creature};
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "move: creature not found".to_string())?;
        let dest = Position { x, y, z };
        let Some(tile) = self.map.get_tile(dest) else {
            return Ok(false);
        };
        if tile_query_add_creature(self, tile, cid, flags) != ReturnValue::NoError {
            return Ok(false);
        }
        // Prefer teleport semantics for floor change / levitate (ignore walk path).
        if let Some(conn) = self.conn_for_creature(cid) {
            let ret = internal_teleport_player(self, conn, cid, dest, false);
            return Ok(ret == ReturnValue::NoError);
        }
        // Non-player: move on map directly.
        let old = self
            .creatures
            .get(cid)
            .map(|k| k.position())
            .ok_or_else(|| "move: creature missing".to_string())?;
        self.move_creature_on_map(cid, old, dest);
        self.flush_pending_creature_step_events();
        Ok(true)
    }

    /// `creature:teleportTo(pos[, pushMovement])`.
    ///
    /// C++ reference: `luascript.cpp` `luaCreatureTeleportTo` → `Game::internalTeleport`
    /// (`game.cpp` ~1784) with `Map::moveCreature(..., !pushMovement)`. When
    /// `push_movement` and destination is adjacent, clients get a walk animation
    /// (doors.lua quest/level doors); otherwise the teleport blink path.
    pub fn lua_script_creature_teleport(
        &mut self,
        creature_u64: u64,
        x: u16,
        y: u16,
        z: u8,
        push_movement: bool,
    ) -> Result<bool, String> {
        use crate::return_value::ReturnValue;
        use crate::walk::{
            are_in_range_1_1_0, creature_turn_with_broadcast, internal_teleport_player,
            set_direction_from_step_for_kick,
        };
        use tfs_rust_common::enums::Direction;
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "teleportTo: creature not found".to_string())?;
        let dest = Position { x, y, z };
        let old = self
            .creatures
            .get(cid)
            .map(|k| k.position())
            .ok_or_else(|| "teleportTo: creature missing".to_string())?;
        if old == dest {
            return Ok(true);
        }

        if let Some(conn) = self.conn_for_creature(cid) {
            let ret = internal_teleport_player(self, conn, cid, dest, push_movement);
            if ret != ReturnValue::NoError {
                return Ok(false);
            }
        } else {
            // Non-player: same forceTeleport = !pushMove rule as Map::moveCreature.
            let has_ground = self
                .map
                .get_tile(dest)
                .map(|t| t.body().ground.is_some())
                .unwrap_or(false);
            let teleport = !push_movement || !has_ground || !are_in_range_1_1_0(old, dest);
            let old_creatures = self
                .map
                .get_tile(old)
                .map(|t| t.body().creatures.clone())
                .unwrap_or_default();
            if !teleport && let Some(k) = self.creatures.get_mut(cid) {
                set_direction_from_step_for_kick(old, dest, k);
            }
            self.move_creature_on_map(cid, old, dest);
            if !teleport {
                self.broadcast_spectator_move(cid, old, dest, &old_creatures);
            }
            self.flush_pending_creature_step_events();
        }

        // C++ `luaCreatureTeleportTo` post-move facing when pushMovement (`luascript.cpp` ~8220–8231).
        if push_movement {
            let dir = if old.x == dest.x {
                if old.y < dest.y {
                    Direction::South
                } else {
                    Direction::North
                }
            } else if old.x > dest.x {
                Direction::West
            } else {
                Direction::East
            };
            creature_turn_with_broadcast(self, cid, dir);
        }
        Ok(true)
    }

    /// `creature:sendTextMessage(type, text)`.
    pub fn lua_script_player_send_text_message(
        &mut self,
        creature_u64: u64,
        msg_class: u8,
        text: String,
    ) -> Result<(), String> {
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "sendTextMessage: creature not found".to_string())?;
        if let Some(conn) = self.conn_for_creature(cid) {
            let msg = tfs_rust_net::outgoing_extra::send_text_message_simple(msg_class, &text);
            self.enqueue_outgoing(conn, msg.into_bytes());
        }
        Ok(())
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
            let mut rng = rand::rng();
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
            let mut rng = rand::rng();
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
        tracing::info!(
            ?cid,
            placed_at = ?pos,
            "LOGIN: creature registered on map at login position"
        );
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
        // `forced` only sets `FLAG_IGNOREBLOCKITEM` (TFS startup / temple fallback). Never
        // treat a failed `queryAdd` as success — that placed creatures on UNPASS walls.
        let flags = if forced { FLAG_IGNOREBLOCKITEM } else { 0 };
        let ret = tile_query_add_creature(self, tile, cid, flags);
        if ret == ReturnValue::NoError || ret == ReturnValue::PlayerIsNotInvited {
            Some(pos)
        } else {
            None
        }
    }

    /// Place a player on login — C++ `Game::placeCreature` login flow.
    ///
    /// TFS 1.4.2 (`src/protocolgame.cpp:258-263`): try `placeCreature(loginPos)` → if fail,
    /// `placeCreature(templePos, force=true)` → if fail, disconnect.
    /// 772 decompile (`cract.cc:314-332` `TCreature::SetOnMap`): `SearchLoginField` at saved
    /// pos → fall back to `startx/starty/startz` (hometown temple).
    ///
    /// Returns the actual placement position, or `None` if both login and temple positions
    /// are unplaceable (caller should disconnect the client, not crash).
    pub(crate) fn place_player_on_login(
        &mut self,
        cid: CreatureId,
        login_pos: Position,
        town_id: i32,
    ) -> Option<Position> {
        // 1. Try the saved login position (with neighbor search like `Map::placeCreature`).
        if self.find_and_place_creature_tfs(cid, login_pos, false, false, 0) {
            return self.creatures.get(cid).map(|k| k.position());
        }

        // 2. Fall back to the town temple position (forced — `FLAG_IGNOREBLOCKITEM`).
        let temple_pos = self
            .map
            .towns
            .get(&(town_id as u32))
            .map(|t| t.temple_position);

        if let Some(temple) = temple_pos {
            tracing::warn!(
                ?login_pos,
                temple = ?temple,
                town_id,
                "login position unplaceable — falling back to town temple"
            );
            if self.find_and_place_creature_tfs(cid, temple, false, true, 0) {
                return self.creatures.get(cid).map(|k| k.position());
            }
            tracing::error!(
                ?temple,
                town_id,
                "town temple position also unplaceable — player will be disconnected"
            );
        } else {
            tracing::error!(
                town_id,
                ?login_pos,
                "login position unplaceable and town_id has no temple — player will be disconnected"
            );
        }

        None
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
        let limit = self.codec.caps().known_creature_limit as usize;
        let (known_flag, remove_known) =
            check_creature_known(wire_id, &mut known, &mut can_see, limit);
        let mut wire = build_add_creature_wire(self, cid, viewer);
        wire.known = known_flag;
        wire.remove_known = remove_known;
        wire.id = wire_id;
        let packet = self
            .codec
            .encode_add_tile_creature(pos, stack_pos, &wire, false)
            .into_bytes();
        self.known_creatures_by_conn.insert(conn, known);
        // Always mark fully-sent: `known=true` means the client already has name/HP from an
        // earlier full AddCreature; `known=false` just sent them. Either way, subsequent
        // spectator moves must use 0x6D (not a second appear) or bars flicker.
        self.mark_creature_fully_sent(conn, wire_id);
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
        // Grid-based fan-out (audit #4) — `spectator_conns_via_grid` already applies
        // `can_see_position(viewer, pos)`, so every conn here can see the target tile.
        // C++ reference: `Game::addCreature` spectator fan-out via `Map::getSpectators`
        // (`game.cpp` ~463, `map.cpp` ~386–474).
        let spectators: Vec<(ConnId, CreatureId)> = self
            .spectator_conns_via_grid(pos)
            .into_iter()
            .filter_map(|conn| {
                let viewer = *self.conn_to_creature.get(&conn)?;
                if viewer == cid {
                    return None;
                }
                Some((conn, viewer))
            })
            .collect();

        for (conn, viewer) in spectators {
            self.send_creature_appear_to_conn(conn, viewer, cid, pos);
        }
    }

    /// C++ `Game::removeCreature` spectator strip (`game.cpp` ~545–578).
    ///
    /// Grid-based fan-out (audit #4): spatial collection via `spectator_conns_via_grid`
    /// (which applies `can_see_position`), then the per-creature `canSeeCreature` check
    /// for ghost/invisibility filtering — matching C++ `getSpectators` + `canSeeCreature`.
    ///
    /// **Includes the removed creature's own connection** when they are a player spectator
    /// on that tile (TVP sends `sendRemoveTileCreature` to every spectator player, including
    /// the dying/logging-out body). OTClient keeps the local player as a tile creature and
    /// needs this `0x6C`/`remove-by-id` or the model stays after death.
    pub(crate) fn broadcast_creature_disappear(
        &mut self,
        cid: CreatureId,
        pos: Position,
        stack_raw: i32,
    ) {
        let spectators: Vec<(ConnId, CreatureId)> = self
            .spectator_conns_via_grid(pos)
            .into_iter()
            .filter_map(|conn| {
                let viewer = *self.conn_to_creature.get(&conn)?;
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
            // Players included — skipping left ghosts on death/logout (spectators + OTClient self).
            self.broadcast_creature_disappear(cid, pos, stack_raw);
        }
        if let Some(slot_index) = self.spawn_slot_by_creature.remove(&cid) {
            let regen_ms = self
                .spawns
                .slot(slot_index)
                .filter(|s| s.respawns)
                .map(|s| s.spawntime_ms)
                .unwrap_or(0);
            let delay_ms = self.compute_respawn_delay_ms(regen_ms);
            self.spawns
                .on_creature_removed(slot_index, now_ms, delay_ms);
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
        TEST_SYNTHETIC_GROUND_WP, beat_driven_test_world, ensure_walkable_tile, insert_monster,
        insert_player, insert_spectator_player, minimal_world, test_player,
    };
    use std::collections::HashMap;
    use std::collections::HashSet;
    use std::sync::Arc;
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
            mana_cost: 0,
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
                immunity_life_drain: false,
                see_invisible: false,
                immunity_physical: false,
            },
            max_summons: 0,
            summons: Vec::new(),
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
        let conn = ConnId(1);
        let viewer = insert_spectator_player(
            &mut world,
            conn,
            test_player("Spec", Position::new(101, 100, 7)),
        );
        world.known_creatures_by_conn.insert(conn, HashSet::new());

        world.remove_creature(monster_cid);
        world.pending_outgoing.clear();

        // Respawn runs on `server_ms` (Phase 6: both eras use the unified beat clock).
        // Advance `server_ms` past the slot's respawn deadline + check interval.
        world.server_ms = 50_000_000;
        // `advance_beat` drains `poll_spawn_respawns` via `run_other_subsystems`.
        // 5 beats × 200 ms = 1000 ms → `other` subsystem fires once.
        for _ in 0..5 {
            world.advance_beat(200);
        }

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
        world.mechanics.profile.spawn_near_player = crate::formulas::SpawnNearPlayer::Block;
        world.startup_spawns();
        let (monster_cid, _) = world.creatures.iter().next().unwrap();
        let conn = ConnId(3);
        let viewer = insert_spectator_player(
            &mut world,
            conn,
            test_player("Spec", Position::new(101, 100, 7)),
        );
        world.known_creatures_by_conn.insert(conn, HashSet::new());

        world.remove_creature(monster_cid);
        world.pending_outgoing.clear();

        // Respawn on `server_ms` (Phase 6: unified beat clock) — advance past the deadline.
        world.server_ms = 50_000_000;
        // `advance_beat` drains `poll_spawn_respawns` via `run_other_subsystems`.
        // 5 beats × 200 ms = 1000 ms → `other` subsystem fires once.
        for _ in 0..5 {
            world.advance_beat(200);
        }
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
        let conn = ConnId(2);
        let viewer = insert_spectator_player(
            &mut world,
            conn,
            test_player("Spec", Position::new(101, 100, 7)),
        );
        world.known_creatures_by_conn.insert(conn, HashSet::new());

        world.remove_creature(monster_cid);

        let packets = world.pending_outgoing.get(&conn);
        assert!(packets.is_some_and(|p| p.iter().any(|b| !b.is_empty() && b[0] == 0x6C)));
        let _ = viewer;
    }

    /// Player remove must emit `0x6C` to spectators **and** the dying player's own conn
    /// (TVP `Game::removeCreature`; OTClient keeps local player as a tile creature).
    #[test]
    fn player_death_broadcasts_remove_including_self() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let conn = ConnId(1);
        let victim = insert_spectator_player(&mut world, conn, test_player("Victim", pos));
        world.known_creatures_by_conn.insert(conn, HashSet::new());

        // Spec watches the death.
        let spec_conn = ConnId(2);
        let _spec = insert_spectator_player(
            &mut world,
            spec_conn,
            test_player("Spec", Position::new(101, 100, 7)),
        );
        world
            .known_creatures_by_conn
            .insert(spec_conn, HashSet::new());

        // Lethal HP → full death path (message + CONNECTION_DEAD + remove).
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(victim) {
            p.base.health = 0;
        }
        world.apply_creature_death(victim);

        assert!(
            world.dead_connections.contains(&conn),
            "772 Connection::Die must mark the session dead"
        );
        assert!(
            world.conn_to_creature.get(&conn).is_none(),
            "dead session must drop ConnId↔CreatureId mapping"
        );
        assert!(world.creatures.get(victim).is_none());

        let self_pkts = world
            .pending_outgoing
            .get(&conn)
            .cloned()
            .unwrap_or_default();
        assert!(
            self_pkts.iter().any(|b| !b.is_empty() && b[0] == 0x6C),
            "dying player must receive self remove (OTClient tile creature)"
        );
        // MESSAGE_EVENT_ADVANCE / TALK_EVENT_MESSAGE = 0xB4 then type 0x13
        assert!(
            self_pkts.iter().any(|b| {
                b.len() >= 3 && b[0] == 0xB4 && b[1] == 0x13 && {
                    let text = String::from_utf8_lossy(&b[4..]);
                    text.contains("You are dead.")
                }
            }),
            "must send 'You are dead.\\n' event message, got {self_pkts:?}"
        );

        let spec_pkts = world
            .pending_outgoing
            .get(&spec_conn)
            .cloned()
            .unwrap_or_default();
        assert!(
            spec_pkts.iter().any(|b| !b.is_empty() && b[0] == 0x6C),
            "spectators must see the player model removed"
        );
    }

    /// Death save must write temple + full vitals, not the death tile / HP=1.
    /// TFS `Player::death` sets `loginPosition = temple` and `health = healthMax` before save.
    #[test]
    fn death_save_uses_temple_and_full_vitals() {
        use crate::condition::{ActiveCondition, ConditionData};
        use tfs_rust_common::enums::ConditionType;

        let mut world = beat_driven_test_world();
        let death_pos = Position::new(200, 200, 7);
        ensure_walkable_tile(&mut world.map, death_pos, TEST_SYNTHETIC_GROUND_WP);
        let mut player = test_player("DeadHero", death_pos);
        player.town_id = 1;
        player.base.health = 0;
        player.base.max_health = 185;
        player.mana = 0;
        player.max_mana = 35;
        player.base.active_conditions.push(ActiveCondition::new(
            1,
            0,
            ConditionType::Fire,
            ConditionData::Damage {
                total_rank: 10,
                factor_percent: 0,
            },
            Some(3),
        ));
        let victim = insert_spectator_player(&mut world, ConnId(9), player);

        world.prepare_player_death_save(victim);
        let mut data = world.build_player_save_data(victim).expect("save data");
        let temple = world.player_temple_position(victim).expect("town temple");
        data.player.posx = i32::from(temple.x);
        data.player.posy = i32::from(temple.y);
        data.player.posz = i32::from(temple.z);

        // Live body must stay on the death tile until remove (client remove packet).
        let CreatureKind::Player(p) = world.creatures.get(victim).unwrap() else {
            panic!("player");
        };
        assert_eq!(p.base.position, death_pos);

        assert_eq!(data.player.posx, i32::from(temple.x));
        assert_eq!(data.player.posy, i32::from(temple.y));
        assert_eq!(data.player.posz, i32::from(temple.z));
        assert_eq!(data.player.health, 185);
        assert_eq!(data.player.mana, 35);
        assert!(
            data.player.conditions.as_ref().is_none_or(|b| b.is_empty()),
            "death must not persist DoT conditions into the next login"
        );
    }

    #[test]
    fn dead_connection_allows_logout() {
        let mut world = beat_driven_test_world();
        let conn = ConnId(7);
        world.dead_connections.insert(conn);
        assert!(world.player_logout_allowed(conn, CreatureId::default(), false));
    }

    // --- Phase 6: 772 respawn timing (Finding 18, `crnonpl.cc:1296` StartMonsterhomeTimer) ---

    /// 772 respawn delay falls in `[regen/2, regen]` with no players online.
    #[test]
    fn respawn_772_randomized_in_regen_band() {
        let mut world = beat_driven_test_world();
        world.mechanics.profile.respawn_model = crate::formulas::RespawnModel::Monsterhome772;
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
        world.mechanics.profile.respawn_model = crate::formulas::RespawnModel::Monsterhome772;
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
        world.mechanics.profile.respawn_model = crate::formulas::RespawnModel::Monsterhome772;
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

    fn poison_spider_type() -> MonsterType {
        MonsterType {
            name: "Poison Spider".into(),
            filename: "poison spider.xml".into(),
            name_description: "a poison spider".into(),
            race: "venom".into(),
            experience: 0,
            speed: 40,
            health_now: 26,
            health_max: 26,
            outfit: MonsterOutfit::default(),
            flags: MonsterTypeFlags::default(),
            mana_cost: 0,
            loot: Vec::new(),
            attack_spells: Vec::new(),
            defenses: MonsterDefenses {
                armor: None,
                defense: None,
                spells: Vec::new(),
                immunity_poison: false,
                immunity_fire: false,
                immunity_energy: false,
                immunity_life_drain: false,
                see_invisible: false,
                immunity_physical: false,
            },
            max_summons: 0,
            summons: Vec::new(),
            talk_texts: Vec::new(),
        }
    }

    /// 772 `TSummonImpact` / `CreateMonster` — master link + no XP.
    #[test]
    fn monster_create_summon_links_master() {
        use crate::creature::CreatureKind;
        use crate::test_world::support::{TEST_SYNTHETIC_GROUND_WP, insert_monster};

        let mut world = beat_driven_test_world();
        let mut monsters = HashMap::new();
        monsters.insert("poison spider".into(), poison_spider_type());
        world.monsters_db = Arc::new(MonsterDatabase { monsters });

        let mpos = Position::new(100, 100, 7);
        for dx in -2i32..=2 {
            for dy in -2i32..=2 {
                let p = Position::new((100 + dx) as u16, (100 + dy) as u16, 7);
                ensure_walkable_tile(&mut world.map, p, TEST_SYNTHETIC_GROUND_WP);
            }
        }
        let master = insert_monster(&mut world, "Giant Spider", mpos, 80);
        world.map.register_creature_at(mpos, master);

        let first = world
            .monster_create_summon(master, "Poison Spider", false, mpos)
            .expect("first summon");
        assert_eq!(
            world.creatures.get(first).and_then(|k| k.base().master),
            Some(master)
        );
        assert_eq!(
            world.creatures.get(first).and_then(|k| match k {
                CreatureKind::Monster(m) => Some(m.experience),
                _ => None,
            }),
            Some(0),
            "summons grant no XP"
        );

        let second = world
            .monster_create_summon(master, "Poison Spider", false, mpos)
            .expect("second summon");
        assert_ne!(first, second);
        let summoned = world
            .creatures
            .iter()
            .filter(|(_, k)| k.base().master == Some(master))
            .count();
        assert_eq!(summoned, 2);
    }

    /// 772 `TMonster` ctor reparents summon-of-summon (`crnonpl.cc:2012–2028`).
    #[test]
    fn monster_create_summon_reparents_summon_of_summon() {
        use crate::creature::CreatureKind;
        use crate::test_world::support::{TEST_SYNTHETIC_GROUND_WP, insert_monster};

        let mut world = beat_driven_test_world();
        let mut monsters = HashMap::new();
        monsters.insert("poison spider".into(), poison_spider_type());
        world.monsters_db = Arc::new(MonsterDatabase { monsters });

        let mpos = Position::new(100, 100, 7);
        for dx in -2i32..=2 {
            for dy in -2i32..=2 {
                let p = Position::new((100 + dx) as u16, (100 + dy) as u16, 7);
                ensure_walkable_tile(&mut world.map, p, TEST_SYNTHETIC_GROUND_WP);
            }
        }
        let wild = insert_monster(&mut world, "Giant Spider", mpos, 80);
        world.map.register_creature_at(mpos, wild);

        let mid = world
            .monster_create_summon(wild, "Poison Spider", false, mpos)
            .expect("mid summon");
        let child = world
            .monster_create_summon(mid, "Poison Spider", false, mpos)
            .expect("child of summon");
        assert_eq!(
            world.creatures.get(child).and_then(|k| k.base().master),
            Some(wild),
            "summon-of-summon must reparent to wild ancestor"
        );
        let under_wild = world
            .creatures
            .iter()
            .filter(|(_, k)| k.base().master == Some(wild))
            .count();
        assert_eq!(
            under_wild, 2,
            "wild master's summon count includes reparented child"
        );
        assert!(matches!(
            world.creatures.get(mid),
            Some(CreatureKind::Monster(_))
        ));
    }

    /// `CreateMonster` `SearchFreeField` after `SearchSummonField` (`crnonpl.cc:3169`).
    #[test]
    fn search_free_field_nudges_off_occupied_center() {
        use crate::test_world::support::{TEST_SYNTHETIC_GROUND_WP, insert_monster};

        let mut world = beat_driven_test_world();
        let center = Position::new(100, 100, 7);
        let east = Position::new(101, 100, 7);
        for dx in -2i32..=2 {
            for dy in -2i32..=2 {
                let p = Position::new((100 + dx) as u16, (100 + dy) as u16, 7);
                ensure_walkable_tile(&mut world.map, p, TEST_SYNTHETIC_GROUND_WP);
            }
        }
        assert_eq!(
            world.search_free_field(center, 2),
            Some(center),
            "clear center stays put"
        );
        let blocker = insert_monster(&mut world, "Blocker", center, 80);
        world.map.register_creature_at(center, blocker);
        assert_eq!(
            world.search_free_field(center, 2),
            Some(east),
            "occupied center nudges east-first spiral"
        );
    }

    // --- NPC-3: spawn from NpcDatabase ---

    fn quentin_pending() -> tfs_rust_content::npcs::PendingNpcDefinition {
        use tfs_rust_content::npcs::{
            DialogueAction, DialoguePolicy, DialoguePredicate, DialogueProgram, DialogueRule,
            DialogueSituation, NpcAppearance, NpcMovement, PendingNpcDefinition, SourceSpan,
        };
        let span = SourceSpan::lua("quentin.lua", 1);
        PendingNpcDefinition {
            name: "Quentin".into(),
            source_file: "quentin.lua".into(),
            appearance: NpcAppearance {
                look_type: 57,
                look_head: 0,
                look_body: 0,
                look_legs: 0,
                look_feet: 0,
                look_addons: 0,
                look_type_ex: 0,
                look_mount: 0,
            },
            health_max: 100,
            movement: NpcMovement {
                radius: 4,
                speed: 10,
            },
            speech_bubble: 1,
            sex: 1,
            race: 1,
            dialogue: Some(DialogueProgram {
                policy: DialoguePolicy::QueuedSingleFocus,
                rules: vec![DialogueRule {
                    predicates: vec![DialoguePredicate::Situation {
                        kind: DialogueSituation::Address,
                        span: span.clone(),
                    }],
                    actions: vec![DialogueAction::Say {
                        text: "Welcome".into(),
                        span: span.clone(),
                    }],
                    span,
                }],
            }),
            ..Default::default()
        }
    }

    fn world_with_npc_db(pending: Vec<tfs_rust_content::npcs::PendingNpcDefinition>) -> GameWorld {
        use tfs_rust_content::npcs::validate_pending_definitions;
        use tfs_rust_content::spawns::{SpawnEntry, SpawnZone};

        let mut world = minimal_world();
        world.npcs_db =
            Arc::new(validate_pending_definitions(pending, None).expect("npc db validate"));

        let home = Position::new(100, 100, 7);
        let zone = SpawnZone {
            center: home,
            radius: 3,
            entries: vec![SpawnEntry::Npc {
                name: "Quentin".into(),
                position: home,
                spawntime_ms: 60_000,
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
    fn spawn_npc_applies_definition_appearance() {
        use crate::creature::CreatureKind;
        use tfs_rust_content::npcs::NpcTypeId;

        let mut world = world_with_npc_db(vec![quentin_pending()]);
        let home = Position::new(100, 100, 7);
        let cid = world
            .spawn_npc("Quentin", home, Direction::South, home, 0, 3, true, true)
            .expect("spawn Quentin");

        let CreatureKind::Npc(n) = world.creatures.get(cid).expect("npc") else {
            panic!("expected Npc");
        };
        assert_eq!(n.base.name, "Quentin");
        assert_eq!(n.base.outfit.look_type, 57);
        assert_eq!(n.base.health, 100);
        assert_eq!(n.base.max_health, 100);
        assert_eq!(n.base.speed, 10);
        assert_eq!(n.base.base_speed, 10);
        assert_eq!(n.definition, NpcTypeId(0));
        assert_ne!(
            n.definition,
            NpcTypeId(u32::MAX),
            "must not be a sentinel placeholder"
        );
        assert_eq!(n.speech_bubble, 1);
        assert_eq!(n.runtime.radius, 4);
        assert_eq!(n.runtime.home_position, n.base.position);
        assert_eq!(
            n.runtime.policy,
            tfs_rust_content::npcs::DialoguePolicy::QueuedSingleFocus
        );
    }

    #[test]
    fn spawn_npc_lookup_case_insensitive() {
        use crate::creature::CreatureKind;

        let mut world = world_with_npc_db(vec![quentin_pending()]);
        let home = Position::new(100, 100, 7);
        // Clear first spawn slot claim by spawning via lowercase then removing isn't needed —
        // call spawn with alternate casing on a fresh world for each.
        let cid = world
            .spawn_npc("quentin", home, Direction::South, home, 0, 3, true, true)
            .expect("lowercase quentin");
        let CreatureKind::Npc(n) = world.creatures.get(cid).expect("npc") else {
            panic!("expected Npc");
        };
        assert_eq!(n.base.name, "Quentin");
        assert_eq!(n.base.outfit.look_type, 57);

        // Second world: Title Case already covered by applies_definition; try mixed.
        let mut world2 = world_with_npc_db(vec![quentin_pending()]);
        let cid2 = world2
            .spawn_npc("QUENTIN", home, Direction::South, home, 0, 3, true, true)
            .expect("uppercase QUENTIN");
        assert!(matches!(
            world2.creatures.get(cid2),
            Some(CreatureKind::Npc(_))
        ));
    }

    #[test]
    fn spawn_npc_unknown_name_returns_none() {
        let mut world = world_with_npc_db(vec![quentin_pending()]);
        let home = Position::new(100, 100, 7);
        let before = world.creatures.len();
        assert!(
            world
                .spawn_npc("Nobody", home, Direction::South, home, 0, 3, true, true)
                .is_none()
        );
        assert_eq!(world.creatures.len(), before);
        assert!(world.spawns.slot(0).unwrap().current.is_none());
    }

    #[test]
    fn spawn_npc_default_movement_when_minimal_def() {
        use crate::creature::CreatureKind;
        use tfs_rust_content::npcs::{
            DialogueAction, DialoguePolicy, DialoguePredicate, DialogueProgram, DialogueRule,
            DialogueSituation, NpcAppearance, NpcMovement, PendingNpcDefinition, SourceSpan,
        };

        let span = SourceSpan::lua("min.lua", 1);
        let pending = PendingNpcDefinition {
            name: "Minimal".into(),
            source_file: "min.lua".into(),
            appearance: NpcAppearance::default(),
            health_max: 100,
            movement: NpcMovement::default(),
            dialogue: Some(DialogueProgram {
                policy: DialoguePolicy::QueuedSingleFocus,
                rules: vec![DialogueRule {
                    predicates: vec![DialoguePredicate::Situation {
                        kind: DialogueSituation::Address,
                        span: span.clone(),
                    }],
                    actions: vec![DialogueAction::Say {
                        text: "hi".into(),
                        span: span.clone(),
                    }],
                    span,
                }],
            }),
            ..Default::default()
        };

        let mut world = world_with_npc_db(vec![pending]);
        // Replace spawn slot name to Minimal.
        let home = Position::new(100, 100, 7);
        world.spawns = SpawnManager::from_zones(vec![tfs_rust_content::spawns::SpawnZone {
            center: home,
            radius: 3,
            entries: vec![tfs_rust_content::spawns::SpawnEntry::Npc {
                name: "Minimal".into(),
                position: home,
                spawntime_ms: 60_000,
                direction: Some(2),
            }],
        }]);
        ensure_walkable_tile(&mut world.map, home, 100);

        let cid = world
            .spawn_npc("Minimal", home, Direction::South, home, 0, 3, true, true)
            .expect("spawn Minimal");
        let CreatureKind::Npc(n) = world.creatures.get(cid).expect("npc") else {
            panic!("expected Npc");
        };
        assert_eq!(n.base.outfit.look_type, 136);
        assert_eq!(n.base.speed, 100);
        assert_eq!(n.runtime.radius, 0);
        assert_eq!(n.speech_bubble, 0);
    }

    #[test]
    fn spawn_npc_places_on_protection_zone() {
        use crate::creature::CreatureKind;
        use crate::tile::{Tile, TileBody, flags as tilestate};
        use tfs_rust_common::enums::ZoneType;

        let mut world = world_with_npc_db(vec![quentin_pending()]);
        let home = Position::new(100, 100, 7);
        // Temple-style PZ tile (Classic772 place_in_pz requires zone match).
        world.map.insert_tile(
            home,
            Tile::Normal(TileBody {
                ground: Some(100),

                ground_item: None,
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: tilestate::PROTECTIONZONE,
                zone: ZoneType::Protection,
            }),
        );
        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let p = Position::new(
                    (home.x as i32 + dx) as u16,
                    (home.y as i32 + dy) as u16,
                    home.z,
                );
                world.map.insert_tile(
                    p,
                    Tile::Normal(TileBody {
                        ground: Some(100),

                        ground_item: None,
                        down_items: Vec::new(),
                        top_items: Vec::new(),
                        creatures: Vec::new(),
                        flags: tilestate::PROTECTIONZONE,
                        zone: ZoneType::Protection,
                    }),
                );
            }
        }

        let cid = world
            .spawn_npc("Quentin", home, Direction::South, home, 0, 3, true, true)
            .expect("Quentin must place on PZ");
        assert!(matches!(
            world.creatures.get(cid),
            Some(CreatureKind::Npc(_))
        ));
    }

    #[test]
    fn spawn_npc_strips_tvp_npc_suffix() {
        use crate::creature::CreatureKind;
        use tfs_rust_content::npcs::{
            DialogueAction, DialoguePolicy, DialoguePredicate, DialogueProgram, DialogueRule,
            DialogueSituation, NpcAppearance, NpcMovement, PendingNpcDefinition, SourceSpan,
        };

        let span = SourceSpan::lua("cobra.lua", 1);
        let pending = PendingNpcDefinition {
            name: "Cobra".into(),
            source_file: "cobra.lua".into(),
            appearance: NpcAppearance {
                look_type: 0,
                ..NpcAppearance::default()
            },
            health_max: 100,
            movement: NpcMovement {
                radius: 0,
                speed: 1,
            },
            dialogue: Some(DialogueProgram {
                policy: DialoguePolicy::QueuedSingleFocus,
                rules: vec![DialogueRule {
                    predicates: vec![DialoguePredicate::Situation {
                        kind: DialogueSituation::Address,
                        span: span.clone(),
                    }],
                    actions: vec![DialogueAction::Idle { span: span.clone() }],
                    span,
                }],
            }),
            ..Default::default()
        };

        let mut world = world_with_npc_db(vec![pending]);
        let home = Position::new(100, 100, 7);
        world.spawns = SpawnManager::from_zones(vec![SpawnZone {
            center: home,
            radius: 2,
            entries: vec![SpawnEntry::Npc {
                name: "cobra npc".into(),
                position: home,
                spawntime_ms: 60_000,
                direction: Some(2),
            }],
        }]);
        ensure_walkable_tile(&mut world.map, home, 100);

        let cid = world
            .spawn_npc("cobra npc", home, Direction::South, home, 0, 2, true, true)
            .expect("cobra npc → Cobra");
        let CreatureKind::Npc(n) = world.creatures.get(cid).expect("npc") else {
            panic!("expected Npc");
        };
        assert_eq!(n.base.name, "Cobra");
    }
}
