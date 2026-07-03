//! Monster native AI — TFS `Monster::onThink`, target list, chase/flee/return.
//!
//! - `Monster::onThink` — `monster.cpp` (~732).
//! - `Monster::searchTarget` — `monster.cpp` (~517).
//! - `Creature::goToFollowCreature` — `creature.cpp` (~1011).
//! - `Monster::walkToSpawn` — `monster.cpp` (~1087).
//! - `Monster::updateLookDirection` — `monster.cpp` (~1967).
//! - `Monster::doAttacking` — `monster.cpp` (~806).
//!
//! Target list / search: [`crate::monster_targets`]. Move/appear fan-out: [`crate::monster_events`].

pub use crate::monster_targets::TargetSearchType;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use slotmap::Key;
use tfs_rust_common::enums::{CombatType, Direction, ZoneType};
use tfs_rust_common::Position;

use crate::chase_debug;
use crate::combat::{
    armor_reduction, melee_damage_after_defense_and_armor, weapon_damage, CombatDamage,
    CombatParams, FightMode,
};
use crate::creature::{
    creature_immune_poison, melee_defense_snapshot, melee_poison_on_hit,
    monster_weapon_attack_distance, roll_target_defense,
};
use crate::creature::{CreatureKind, MonsterAiPhase, ChaseMode, MonsterState};
use crate::game_world::{creature_can_see, GameWorld};
use crate::ids::CreatureId;
use crate::monster_distance_step::{
    distance_x, distance_y, get_dance_step, get_distance_step, get_random_step, offset_x, offset_y,
    search_flight_field, DistanceStepOutcome,
};
use crate::pathfinding::{
    scan_min_terrain_waypoints, uses_reverse_terrain_path, FindPathParams, CHASE_PATH_MAX_STEPS,
    REVERSE_PATH_VIEW_RADIUS,
};
use crate::player_flags::{flags_for_group, has_player_flag, PLAYER_FLAG_IGNORED_BY_MONSTERS};
use crate::tile::{flags as tilestate, MapStackEntry};
use crate::walk::{creature_turn_with_broadcast, tile_query_add_creature, PATHFIND_WALK_FLAGS};

/// C++ `Map::maxViewportX` (`map.h`).
pub(crate) const MAP_MAX_VIEWPORT: u16 = 11;

/// All map directions for brute-force chase steps when A* / `getDistanceStep` fail.
const CHASE_STEP_DIRECTIONS: [Direction; 8] = [
    Direction::North,
    Direction::East,
    Direction::South,
    Direction::West,
    Direction::NorthEast,
    Direction::SouthEast,
    Direction::SouthWest,
    Direction::NorthWest,
];

pub(crate) fn chebyshev(a: Position, b: Position) -> i32 {
    distance_x(a, b).max(distance_y(a, b))
}

/// 772 idle `ToDoGo` batch size — `crnonpl.cc:2732–2733` (melee `must:false, max:3`),
/// `crnonpl.cc:2769` (dist chase), `cract.cc:260–261` (trim stops at cheb≤1).
///
/// Melee chase uses `max:3, must:false`; dist chase uses `cheb - target_distance`
/// (per-type band from `monsters.xml`, not a hardcoded keep distance).
pub(crate) fn monster_idle_chase_step_budget(
    _is_melee_chase: bool,
    is_dist_chase: bool,
    cheb_to_target: i32,
    target_distance: i32,
) -> (usize, bool) {
    if is_dist_chase {
        let steps = (cheb_to_target - target_distance).max(1);
        (steps as usize, false)
    } else {
        (CHASE_PATH_MAX_STEPS, false)
    }
}

/// Result of 772 idle chase repath (`TShortway` via `monster_idle_chase_repath`).
///
/// C++ reference: `ToDoGo` / `TShortway::Calculate` — `cract.cc:1067`; NOWAY → roam — `crnonpl.cc:2813`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MonsterIdleChaseRepathOutcome {
    /// Already in melee / keep-distance band — no walk queue needed.
    AtGoal,
    /// `TShortway` succeeded — `walk_queue` populated.
    PathQueued,
    /// `TShortway` failed on non-flee chase — caller clears target and roams.
    Noway,
}

/// Result of `CanToDoAttack` close-chase enqueue — `crcombat.cc:496-498`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MonsterCombatCloseChaseEnqueue {
    Queued,
    Skipped,
    /// Transient path failure — yield and retry next beat; target retained (`cract.cc:845-852`).
    Retry,
    /// Idle walk-branch path failure — target cleared (`crnonpl.cc:2813`).
    Noway,
}

/// Result of `ToDoAttack` action list enqueue — `cract.cc:1325-1334`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MonsterEnqueueAttackResult {
    Enqueued,
    Failed,
    /// Close-chase path blocked — yield and retry; target retained.
    Retry,
    /// Idle walk-branch NOWAY — target cleared, roam fallback.
    Noway,
}

pub(crate) fn manhattan(a: Position, b: Position) -> i32 {
    distance_x(a, b) + distance_y(a, b)
}

/// 772 master follow wait band — `crnonpl.cc:2691` (Manhattan 2–3 → `ToDoWait` only).
pub(crate) fn monster_master_follow_in_wait_band(manhattan_dist: i32) -> bool {
    (2..=3).contains(&manhattan_dist)
}

/// TFS `Monster::isFleeing` gate — `monster.h` ~154.
// Parity helper; wired into monster AI flee logic (see todo.md). Retained ahead of caller.
#[allow(dead_code)]
pub fn is_fleeing(health: i32, run_away_health: i32, is_summon: bool) -> bool {
    !is_summon && run_away_health > 0 && health <= run_away_health
}

/// TFS `Monster::isInSpawnRange` — `monster.cpp` ~1931.
pub fn is_in_spawn_range(
    pos: Position,
    master_pos: Position,
    despawn_radius: i32,
    despawn_z_range: i32,
) -> bool {
    if despawn_radius == 0 {
        return true;
    }
    if chebyshev(pos, master_pos) > despawn_radius {
        return false;
    }
    if despawn_z_range == 0 {
        return true;
    }
    let z_dist = (pos.z as i32 - master_pos.z as i32).unsigned_abs() as i32;
    z_dist <= despawn_z_range
}

/// TFS `Position::areInRange` for walk-back — `position.h` ~38, `monster.cpp` ~510.
pub fn is_within_walk_to_spawn_range(pos: Position, spawn: Position, radius: i32) -> bool {
    if radius <= 0 {
        return true;
    }
    distance_x(pos, spawn) <= radius && distance_y(pos, spawn) <= radius
}

/// TFS `Monster::updateLookDirection` — `monster.cpp` ~1967.
/// C++ `getOffsetX(attackedCreaturePos, pos)` = target.x − monster.x → `offset_x(target, from)`.
pub fn compute_look_toward_target(
    from: Position,
    target: Position,
    _current: Direction,
) -> Direction {
    let ox = offset_x(target, from);
    let oy = offset_y(target, from);
    let dx = ox.unsigned_abs() as i32;
    let dy = oy.unsigned_abs() as i32;

    if dx > dy {
        if ox < 0 {
            Direction::West
        } else {
            Direction::East
        }
    } else if dx < dy {
        if oy < 0 {
            Direction::North
        } else {
            Direction::South
        }
    } else {
        // C++ `TCreature::Rotate(TCreature*)` — `cract.cc:463-466` (`DistanceY > DistanceX` else horizontal).
        if ox < 0 {
            Direction::West
        } else {
            Direction::East
        }
    }
}

/// One tile from [`GameWorld::dump_tshortway_fill_walkable_viewport`] — P2.5 FillMap parity probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TShortwayFillTile {
    pub pos: Position,
    /// `true` when `MovePossible(Execute=false)` would allow planning through the tile.
    pub walkable: bool,
    /// Ground `WAYPOINTS` when walkable terrain exists; `-1` when blocked or no bank.
    pub wp: i32,
}

impl GameWorld {
    /// Monster keep-distance from the type's XML `targetDistance` (`monsters.xml`).
    ///
    /// Optional shard override: `distanceKeep = N` in `data/formulas/*.lua` forces a fixed band for
    /// all types; default is [`DistanceKeep::PerType`] for both 772 and 1098.
    #[inline]
    pub(crate) fn monster_effective_target_distance(&self, per_type: i32) -> i32 {
        match self.mechanics.profile.distance_keep {
            crate::formulas::DistanceKeep::PerType => per_type,
            crate::formulas::DistanceKeep::Fixed(n) => n,
        }
    }

    /// 772 idle distance-fighting branch — `DistanceFighting && ThrowPossible` (`crnonpl.cc:2795-2834`).
    pub(crate) fn monster_idle_uses_dist_branch(
        &self,
        cid: CreatureId,
        pos: Position,
        follow_id: CreatureId,
        target_distance: i32,
    ) -> bool {
        target_distance > 1 && self.monster_throw_possible(cid, pos, follow_id)
    }

    /// After walk steps finish, re-check keep-distance / melee band (772 rush-then-kite fix).
    fn monster_reconcile_follow_position(&mut self, cid: CreatureId, follow_id: CreatureId) {
        let _ = follow_id;
        self.monster_ensure_follow_band(cid, "walk_complete");
    }

    /// True when a **non-empty** `walk_queue` no longer reaches the follow band or sight is blocked.
    ///
    /// Empty queue is not stale here — 772 batch replan runs from idle segment drain / `off_band`,
    /// not from every target tile (`crnonpl.cc` `ToDoGo` after drain).
    pub(crate) fn monster_chase_queue_stale(
        &self,
        monster_id: CreatureId,
        target_pos: Position,
    ) -> bool {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) else {
            return true;
        };
        if m.base.walk_queue.is_empty() {
            return false;
        }
        let mut expected_pos = m.base.position;
        for &dir in m.base.walk_queue.iter().rev() {
            expected_pos = expected_pos.offset(dir);
        }
        let target_distance = self.monster_effective_target_distance(m.target_distance);
        let expected_dist = chebyshev(expected_pos, target_pos);
        let wrong_distance = if target_distance <= 1 {
            expected_dist > 1
        } else {
            expected_dist != target_distance
        };
        wrong_distance || !self.monster_sight_clear(expected_pos, target_pos)
    }

    /// Central guard: `has_follow_path` vs actual follow band.
    ///
    /// 772: defers repath via `force_update_follow_path` (same idle tick); keeps in-flight batches
    /// when [`Self::monster_chase_queue_stale`] is false. 1098: sync [`Self::monster_follow_repath_now`].
    ///
    /// Returns true when a repath was scheduled or invoked.
    pub(crate) fn monster_ensure_follow_band(&mut self, cid: CreatureId, _reason: &str) -> bool {
        let follow_id = match self.creatures.get(cid).and_then(|k| k.base().follow_target) {
            Some(id) => id,
            None => return false,
        };
        let (walking_to_spawn, fleeing, pos, target_distance, has_path, queue_empty) =
            match self.creatures.get(cid) {
                Some(CreatureKind::Monster(m)) => (
                    m.walking_to_spawn,
                    m.is_fleeing(),
                    m.base.position,
                    self.monster_effective_target_distance(m.target_distance),
                    m.base.has_follow_path,
                    m.base.walk_queue.is_empty(),
                ),
                _ => return false,
            };
        if walking_to_spawn || fleeing {
            return false;
        }
        let Some(target_pos) = self.creatures.get(follow_id).map(|k| k.position()) else {
            return false;
        };
        let at_goal =
            self.monster_at_follow_goal(cid, follow_id, pos, target_pos, fleeing, target_distance);
        if at_goal {
            if !has_path {
                self.monster_mark_at_follow_goal(cid, follow_id);
            }
            return false;
        }

        if self.beat_driven_loop {
            let stale = self.monster_chase_queue_stale(cid, target_pos);
            if !queue_empty && !stale {
                return false;
            }
            if let Some(k) = self.creatures.get_mut(cid) {
                let base = k.base_mut();
                if stale && !queue_empty {
                    base.walk_queue.clear();
                }
                base.has_follow_path = false;
                base.force_update_follow_path = true;
            }
            return true;
        }

        if !queue_empty {
            // C++ `onCreatureMove` clears `listWalkDir` when the target leaves the band — abort a
            // stale in-flight A* queue that no longer matches the keep-distance goal.
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().walk_queue.clear();
                k.base_mut().has_follow_path = false;
            }
            self.stop_event_walk(cid);
        }
        if has_path {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().has_follow_path = false;
            }
        }
        self.monster_follow_repath_now(cid, Some("ensure_band"));
        true
    }

    /// True when the monster is already in the desired follow/attack band (C++ empty `listWalkDir` at goal).
    pub(crate) fn monster_at_follow_goal(
        &self,
        cid: CreatureId,
        follow_id: CreatureId,
        pos: Position,
        target_pos: Position,
        fleeing: bool,
        target_distance: i32,
    ) -> bool {
        if fleeing {
            return false;
        }
        let dist = chebyshev(pos, target_pos);
        if target_distance <= 1 {
            return dist <= 1;
        }
        if self.beat_driven_loop {
            // 772 keep-distance — per-type band from monsters.xml (`crnonpl.cc` dist branches).
            return dist == target_distance;
        }
        if self.monster_can_use_attack(cid, pos, follow_id) {
            // TFS `getDistanceStep` — `AtTargetDistance` when `distance == targetDistance`.
            return dist == target_distance;
        }
        // Keep-distance types off-band without a usable attack are not "at goal" — movement continues.
        false
    }

    /// Mark chase path satisfied at the current follow goal — no walk queue needed.
    fn monster_mark_at_follow_goal(&mut self, cid: CreatureId, follow_id: CreatureId) {
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().has_follow_path = true;
        }
        if self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
        {
            self.monster_on_follow_creature_complete(cid, follow_id);
        }
    }

    /// Used by [`crate::creature_think::GameWorld::creature_on_think`] to skip redundant repaths.
    pub(crate) fn monster_should_skip_follow_repath(
        &self,
        cid: CreatureId,
        follow_id: CreatureId,
    ) -> bool {
        let (pos, fleeing, target_distance) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                m.base.position,
                m.is_fleeing(),
                self.monster_effective_target_distance(m.target_distance),
            ),
            _ => return false,
        };
        let target_pos = match self.creatures.get(follow_id) {
            Some(k) => k.position(),
            None => return false,
        };
        self.monster_at_follow_goal(cid, follow_id, pos, target_pos, fleeing, target_distance)
    }

    /// B3.1 — lowest-health opponent from `candidates`, using the profile's [`WeakestTargetMetric`]
    /// (current HP for 772, max HP for TFS). Ties keep the first candidate.
    ///
    /// C++ `Monster::onCreatureAppear` self branch — `monster.cpp` ~159–166.
    ///
    /// C++ `TCombat::Attack` / `CloseAttack` / `DistanceAttack` — `crcombat.cc:530`, `:609`, `:647`.
    pub fn monster_do_attacking(&mut self, cid: CreatureId, _interval_ms: u32) {
        self.monster_update_look_direction(cid);

        if !self.beat_driven_loop {
            return;
        }

        let server_ms = self.server_ms;
        let profile = self.mechanics.profile;
        let hooks = &self.mechanics.hooks;

        let (
            target_id,
            monster_pos,
            melee_skill,
            melee_attack,
            poison_cycles,
            has_ranged_spell,
            shoot_effect,
        ) = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
                return;
            };
            let Some(target_id) = m.base.attack_target else {
                return;
            };
            let has_ranged_spell = m.spells.iter().any(|s| s.range > 1);
            let weapon_dist = monster_weapon_attack_distance(m.melee_skill, has_ranged_spell);
            if m.melee_skill <= 0 && weapon_dist <= 1 {
                return;
            }
            let shoot = m
                .spells
                .iter()
                .find_map(|s| s.shoot_effect)
                .or(if weapon_dist >= 3 {
                    Some(tfs_rust_common::enums::ShootEffect::Arrow as u8)
                } else if weapon_dist >= 2 {
                    Some(tfs_rust_common::enums::ShootEffect::Spear as u8)
                } else {
                    None
                });
            (
                target_id,
                m.base.position,
                m.melee_skill,
                m.melee_attack,
                m.poison_cycles,
                has_ranged_spell,
                shoot,
            )
        };

        let target_alive = self
            .creatures
            .get(target_id)
            .is_some_and(|k| k.base().health > 0);
        if !target_alive {
            return;
        }

        let target_pos = self.creatures.get(target_id).unwrap().position();
        // C++ `ObjectDistance` returns `INT_MAX` when Z-levels differ (`info.cc:313`),
        // so `TCombat::Attack` gets `Distance > 8` → `StopAttack` + `TARGETLOST`
        // (`crcombat.cc:574-578`). Block the attack when on a different floor —
        // `chebyshev` only uses x/y and would otherwise allow cross-floor melee.
        if monster_pos.z != target_pos.z {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().delay_attack_ms(server_ms, 200);
            }
            return;
        }
        let cheb = chebyshev(monster_pos, target_pos);
        let in_pz = self.monster_tile_in_protection_zone(monster_pos)
            || self.monster_tile_in_protection_zone(target_pos);
        let weapon_dist = monster_weapon_attack_distance(melee_skill, has_ranged_spell) as u32;

        // C++ `DistanceAttack` / `WandAttack` — `crcombat.cc:609-637`.
        if weapon_dist >= 2 && cheb >= 2 && cheb <= weapon_dist as i32 && !in_pz {
            let dx = (target_pos.x as i32 - monster_pos.x as i32).unsigned_abs();
            let dy = (target_pos.y as i32 - monster_pos.y as i32).unsigned_abs();
            if dx > 7 || dy > 5 {
                if let Some(k) = self.creatures.get_mut(cid) {
                    k.base_mut().delay_attack_ms(server_ms, 200);
                }
                return;
            }
            if !self.monster_sight_clear(monster_pos, target_pos) {
                if let Some(k) = self.creatures.get_mut(cid) {
                    k.base_mut().delay_attack_ms(server_ms, 200);
                }
                return;
            }

            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().delay_attack_ms(server_ms, 200);
            }

            let defense_snap = melee_defense_snapshot(self.creatures.get(target_id).unwrap());
            let mut rng = std::mem::replace(&mut self.ai_rng, StdRng::from_entropy());
            let attack_roll = weapon_damage(
                &profile,
                hooks,
                &mut rng,
                melee_skill,
                melee_attack,
                FightMode::Balanced,
                0,
            );
            let defense_roll = {
                let Some(kind) = self.creatures.get_mut(target_id) else {
                    self.ai_rng = rng;
                    return;
                };
                roll_target_defense(
                    kind.base_mut(),
                    server_ms,
                    &profile,
                    hooks,
                    &mut rng,
                    defense_snap,
                )
            };
            let armor_roll = armor_reduction(&profile, hooks, &mut rng, defense_snap.armor);
            let dmg = melee_damage_after_defense_and_armor(attack_roll, defense_roll, armor_roll);

            if let Some(shoot) = shoot_effect {
                self.broadcast_distance_shoot(monster_pos, target_pos, shoot);
            }

            let hp_before = self.creatures.get(target_id).unwrap().base().health;
            let _ = self.combat_execute_with_stimulus(
                Some(cid),
                target_id,
                &CombatDamage {
                    primary: (CombatType::Physical, -dmg),
                    secondary: (CombatType::Physical, 0),
                },
                &CombatParams::default(),
            );
            let hp_after = self
                .creatures
                .get(target_id)
                .map(|k| k.base().health)
                .unwrap_or(hp_before);
            if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                crate::chase_debug::log_ranged_hit(
                    self.chase_trace_tick(),
                    cid,
                    &m.base.name,
                    target_id.data().as_ffi(),
                    attack_roll,
                    defense_roll,
                    armor_roll,
                    dmg,
                    hp_before,
                    hp_after,
                    self.creatures
                        .get(cid)
                        .map(|k| k.base().earliest_attack_ms)
                        .unwrap_or(0),
                );
            }
            self.notify_player_combat_damage(Some(cid), target_id, (hp_before - hp_after).max(0));
            self.ai_rng = rng;

            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().delay_attack_ms(server_ms, 2000);
            }
            return;
        }

        if cheb > 1 || in_pz || melee_skill <= 0 {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().delay_attack_ms(server_ms, 200);
            }
            return;
        }

        // C++ `ResyncHarnessRng` at appear + one lose/talk prelude per idle (`crnonpl.cc:2429`, `:2440`).
        // Rust harness drains can run extra idle preambles before the first strike — realign probes.
        // Dual-monster real-map bowl: C++ draw order differs from one_real; skip global realign (T5).
        #[cfg(any(test, feature = "sim"))]
        {
            let melee_realign = std::env::var("TFS_SIM_MELEE_REALIGN")
                .map(|v| !v.is_empty() && v != "0")
                .unwrap_or(true);
            if melee_realign
                && !self.beat_driven_loop
                && crate::sim_glibc_rand::sim_glibc_rng_enabled()
                && crate::sim_glibc_rand::sim_rng_call_count() > 2
                && !crate::sim_glibc_rand::harness_melee_realign_done()
            {
                crate::sim_glibc_rand::resync_harness_glibc_rng_from_env();
                let _trace = crate::sim_glibc_rand::sim_rng_trace_site("melee_realign_lose");
                let _ = crate::sim_glibc_rand::parity_random(0, 99);
                let _trace = crate::sim_glibc_rand::sim_rng_trace_site("melee_realign_talk");
                let _ = crate::sim_glibc_rand::parity_rand_mod(50);
                crate::sim_glibc_rand::mark_harness_melee_realign_done();
            }
        }

        let _trace_atk = crate::sim_glibc_rand::sim_rng_trace_site("melee_attack_probe");

        let defense_snap = melee_defense_snapshot(self.creatures.get(target_id).unwrap());
        let target_immune_poison = creature_immune_poison(self.creatures.get(target_id).unwrap());

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().delay_attack_ms(server_ms, 200);
        }

        let mut rng = std::mem::replace(&mut self.ai_rng, StdRng::from_entropy());
        let attack_roll = weapon_damage(
            &profile,
            hooks,
            &mut rng,
            melee_skill,
            melee_attack,
            FightMode::Balanced,
            0,
        );

        let defense_roll = {
            let Some(kind) = self.creatures.get_mut(target_id) else {
                self.ai_rng = rng;
                return;
            };
            let _trace = crate::sim_glibc_rand::sim_rng_trace_site("melee_defense_probe");
            roll_target_defense(
                kind.base_mut(),
                server_ms,
                &profile,
                hooks,
                &mut rng,
                defense_snap,
            )
        };

        let _trace_armor = crate::sim_glibc_rand::sim_rng_trace_site("melee_armor_probe");
        let armor_roll = armor_reduction(&profile, hooks, &mut rng, defense_snap.armor);
        let dmg = melee_damage_after_defense_and_armor(attack_roll, defense_roll, armor_roll);

        let hp_before = self.creatures.get(target_id).unwrap().base().health;
        let damage = CombatDamage {
            primary: (CombatType::Physical, -dmg),
            secondary: (CombatType::Physical, 0),
        };
        let _ = self.combat_execute_with_stimulus(
            Some(cid),
            target_id,
            &damage,
            &CombatParams::default(),
        );
        let hp_after = self
            .creatures
            .get(target_id)
            .map(|k| k.base().health)
            .unwrap_or(hp_before);
        let damage_done = (hp_before - hp_after).max(0);
        self.notify_player_combat_damage(Some(cid), target_id, damage_done);

        if !target_immune_poison {
            if let Some(cond) = melee_poison_on_hit(
                &mut rng,
                poison_cycles,
                attack_roll,
                defense_roll,
                damage_done,
            ) {
                let params = CombatParams {
                    primary_type: CombatType::Physical,
                    dispel: None,
                    apply_condition: Some(cond),
                };
                let _ = self.combat_execute_with_stimulus(
                    Some(cid),
                    target_id,
                    &CombatDamage {
                        primary: (CombatType::Physical, 0),
                        secondary: (CombatType::Physical, 0),
                    },
                    &params,
                );
            }
        }
        self.ai_rng = rng;

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().delay_attack_ms(server_ms, 2000);
        }

        // C++ panic melee at band 1 — observable `combat_state` logs `attacking` after first hit.
        if cheb == 1 {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                if m.state == MonsterState::Panic && m.melee_skill > 0 {
                    m.state = MonsterState::Attacking;
                }
            }
        }

        if chase_debug::chase_path_debug_enabled() {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                let earliest = m.base.earliest_attack_ms;
                chase_debug::log_melee_hit(
                    self.chase_trace_tick(),
                    cid,
                    m.base.name.as_str(),
                    target_id.data().as_ffi(),
                    attack_roll,
                    defense_roll,
                    armor_roll,
                    dmg,
                    hp_before,
                    hp_after,
                    earliest,
                );
            }
        }
    }

    fn monster_tile_in_protection_zone(&self, pos: Position) -> bool {
        self.map
            .get_tile(pos)
            .is_some_and(|t| t.body().zone == ZoneType::Protection)
    }

    /// TFS `Monster::onThink` native body — `monster.cpp` ~732.
    pub fn monster_native_on_think(&mut self, cid: CreatureId, interval_ms: u32) {
        if !self.creatures.contains_key(cid) {
            return;
        }
        if self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if m.wants_lua_think()))
        {
            return;
        }

        self.monster_update_target_list(cid);

        let (pos, in_range) = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
                return;
            };
            let pos = m.base.position;
            let cfg = self.monster_world_config;
            (
                pos,
                is_in_spawn_range(
                    pos,
                    m.spawn_position,
                    cfg.despawn_radius,
                    cfg.despawn_z_range,
                ),
            )
        };

        if !in_range {
            self.monster_handle_out_of_spawn_range(cid);
            return;
        }

        self.monster_update_idle_status(cid);

        let (is_idle, is_summon, has_opponents, follow, _has_path, fleeing) = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
                return;
            };
            (
                m.is_idle,
                m.base.is_summon(),
                !m.opponent_ids.is_empty(),
                m.base.follow_target,
                m.base.has_follow_path,
                m.is_fleeing(),
            )
        };

        if !is_idle {
            if self.beat_driven_loop {
                // 772 `ProcessCreatures` ~1 Hz — stall rescue only; no TFS `onThinkTarget`
                // (`changeTargetSpeed` / `changeTargetChance` — `monster.cpp` ~923, absent in `crnonpl.cc`).
                if self.monster_combat_scheduler_needs_refresh(cid) {
                    self.monster_combat_reschedule_if_stalled(cid);
                }
            } else {
                self.monster_arm_event_walk(cid);

                if is_summon {
                    self.monster_think_summon_stub(cid);
                } else if has_opponents {
                    if follow.is_none() {
                        let _ = self.monster_search_target(cid, TargetSearchType::Default);
                    } else {
                        self.monster_ensure_follow_band(cid, "think");
                    }
                    if fleeing {
                        let attack = self.creatures.get(cid).and_then(|k| k.base().attack_target);
                        if let Some(target_id) = attack {
                            if !self.monster_can_use_attack(cid, pos, target_id) {
                                let _ =
                                    self.monster_search_target(cid, TargetSearchType::AttackRange);
                            }
                        }
                    }
                }

                self.monster_on_think_target(cid, interval_ms);
                self.monster_update_look_direction(cid);
            }
        }

        let phase = if fleeing {
            MonsterAiPhase::Flee
        } else if follow.is_some() {
            MonsterAiPhase::Chase
        } else if self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if m.walking_to_spawn))
        {
            MonsterAiPhase::ReturnToSpawn
        } else if is_idle {
            MonsterAiPhase::Idle
        } else {
            MonsterAiPhase::Chase
        };
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.ai_phase = phase;
        }
    }

    /// 772 idle chase repath — `TShortway` only, no TFS fallbacks (`cract.cc:1067`, `crnonpl.cc:2676`).
    ///
    /// Called from idle `MeleeChase` / `DistChase` / `MasterFollow` arms only — not from flee or roam.
    /// On path failure (non-flee) returns [`MonsterIdleChaseRepathOutcome::Noway`].
    pub(crate) fn monster_idle_chase_repath(
        &mut self,
        cid: CreatureId,
        _repath_reason: Option<&str>,
        max_steps: usize,
        must_reach: bool,
    ) -> MonsterIdleChaseRepathOutcome {
        debug_assert!(self.beat_driven_loop, "772-only entry point");
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().force_update_follow_path = false;
        }
        let follow_id = match self.creatures.get(cid).and_then(|k| k.base().follow_target) {
            Some(id) => id,
            None => return MonsterIdleChaseRepathOutcome::AtGoal,
        };
        let target_pos = match self.creatures.get(follow_id) {
            Some(k) => k.position(),
            None => return MonsterIdleChaseRepathOutcome::AtGoal,
        };
        let (target_distance, fleeing, is_summon, has_follow_path) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                self.monster_effective_target_distance(m.target_distance),
                m.is_fleeing(),
                m.base.is_summon(),
                m.base.has_follow_path,
            ),
            _ => return MonsterIdleChaseRepathOutcome::AtGoal,
        };

        let fpp = self.monster_path_search_params(
            cid,
            follow_id,
            fleeing,
            target_distance,
            is_summon,
            has_follow_path,
        );

        let pos = match self.creatures.get(cid) {
            Some(k) => k.position(),
            None => return MonsterIdleChaseRepathOutcome::AtGoal,
        };
        if self.monster_at_follow_goal(cid, follow_id, pos, target_pos, fleeing, target_distance) {
            self.monster_mark_at_follow_goal(cid, follow_id);
            return MonsterIdleChaseRepathOutcome::AtGoal;
        }

        if self.monster_try_apply_chase_path(
            cid,
            target_pos,
            fleeing,
            target_distance,
            &fpp,
            max_steps,
            must_reach,
        ) {
            if self
                .creatures
                .get(cid)
                .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
            {
                self.monster_on_follow_creature_complete(cid, follow_id);
            }
            return MonsterIdleChaseRepathOutcome::PathQueued;
        }

        if fleeing {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().has_follow_path = false;
            }
            if self
                .creatures
                .get(cid)
                .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
            {
                self.monster_on_follow_creature_complete(cid, follow_id);
            }
            return MonsterIdleChaseRepathOutcome::AtGoal;
        }

        MonsterIdleChaseRepathOutcome::Noway
    }

    /// 772 NOWAY handler — clear chase target and fall through to roam (`crnonpl.cc:2813`).
    pub(crate) fn monster_on_chase_noway_772(&mut self, cid: CreatureId) {
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            base.clear_targets();
            base.has_follow_path = false;
            base.walk_queue.clear();
            base.force_update_follow_path = false;
        }
    }

    /// 772 idle roam — random cardinal step (`crnonpl.cc:2827–2850`).
    ///
    /// Returns true when a single step was queued into `walk_queue`.
    pub(crate) fn monster_idle_roam_step(&mut self, cid: CreatureId) -> bool {
        let pos = match self.creatures.get(cid) {
            Some(k) => k.position(),
            None => return false,
        };
        const ROAM_DIRS: [Direction; 4] = [
            Direction::West,
            Direction::East,
            Direction::North,
            Direction::South,
        ];
        for _ in 0..10 {
            // C++ `switch(rand()%4)` (`crnonpl.cc:2833`) — glibc parity stream, not `ai_rng` (Finding 10).
            let dir = ROAM_DIRS[self.parity_rand_mod(4) as usize];
            if !self.monster_can_walk_to(cid, pos, dir) {
                continue;
            }
            let dest = pos.offset(dir);
            if let Some(k) = self.creatures.get_mut(cid) {
                let base = k.base_mut();
                base.walk_queue.clear();
                base.walk_queue.push_back(dir);
                base.has_follow_path = false;
                base.force_update_follow_path = false;
            }
            if chase_debug::chase_path_debug_enabled() {
                if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                    chase_debug::log_branch(
                        self.chase_trace_tick(),
                        cid,
                        m.base.name.as_str(),
                        "roam",
                        pos,
                        dest,
                        true,
                        i32::MAX,
                        None,
                    );
                }
            }
            return true;
        }
        false
    }

    /// 772 idle flee / dist-flee — `SearchFlightField` (`crnonpl.cc:2680`, `2762`).
    ///
    /// `SearchFlightField` returns one adjacent tile (`info.cc:1030`); decompile follows with
    /// `ToDoGo(must:true, INT_MAX)` which is a single `TDGo` for that distance.
    /// Returns true when a single step was queued (no `TShortway`).
    pub(crate) fn monster_idle_flee_step(&mut self, cid: CreatureId) -> bool {
        let (pos, follow_id) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => {
                let Some(follow_id) = m.base.follow_target else {
                    return false;
                };
                (m.base.position, follow_id)
            }
            _ => return false,
        };
        let target_pos = match self.creatures.get(follow_id) {
            Some(k) => k.position(),
            None => return false,
        };
        // `SearchFlightField` shuffles on the glibc parity stream internally (Finding 9) — no `ai_rng`.
        let Some(dir) = search_flight_field(pos, target_pos, |dir| {
            self.monster_can_walk_to(cid, pos, dir)
        }, |buf| self.parity_rng.random_shuffle(buf)) else {
            return false;
        };
        let dest = pos.offset(dir);
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            base.walk_queue.clear();
            base.walk_queue.push_back(dir);
            base.has_follow_path = true;
            base.force_update_follow_path = false;
        }
        if chase_debug::chase_path_debug_enabled() {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                let branch = if m.is_fleeing() { "flee" } else { "dist_flee" };
                chase_debug::log_branch(
                    self.chase_trace_tick(),
                    cid,
                    m.base.name.as_str(),
                    branch,
                    pos,
                    dest,
                    true,
                    i32::MAX,
                    None,
                );
            }
        }
        true
    }

    /// Whether idle `MeleeChase` at cheb>1 must not run — `crnonpl.cc:2731` (`ATTACKING`/`PANIC`).
    ///
    /// Close walk at distance comes from `ToDoAttack` → `CanToDoAttack` (`crcombat.cc:496`), not idle chase.
    pub(crate) fn monster_idle_skip_idle_melee_chase(&self, cid: CreatureId) -> bool {
        if !self.beat_driven_loop {
            return false;
        }
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return false;
        };
        if m.is_fleeing() {
            return false;
        }
        matches!(m.state, MonsterState::Attacking | MonsterState::Panic)
    }

    /// 772 `ATTACKING`/`PANIC` at melee band (cheb==1) — stick-fight wait gate only.
    ///
    /// Dist>1 chase skip uses [`Self::monster_idle_skip_idle_melee_chase`]; `melee_dance` still runs at cheb==1.
    pub(crate) fn monster_idle_is_attacking_posture(
        &self,
        cid: CreatureId,
        target_distance: i32,
    ) -> bool {
        if !self.beat_driven_loop || target_distance > 1 {
            return false;
        }
        self.monster_idle_skip_idle_melee_chase(cid)
    }

    /// Empty `walk_queue` while ATTACKING close-chase must repath on target move (`crmain.cc:888`).
    pub(crate) fn monster_chase_needs_attacking_close_repath(
        &self,
        monster_id: CreatureId,
        target_pos: Position,
    ) -> bool {
        if !self.beat_driven_loop {
            return false;
        }
        let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) else {
            return false;
        };
        if !matches!(m.state, MonsterState::Attacking | MonsterState::Panic) {
            return false;
        }
        if m.is_fleeing() || !m.base.walk_queue.is_empty() {
            return false;
        }
        let target_distance = self.monster_effective_target_distance(m.target_distance);
        let uses_dist_branch = m.base.follow_target.is_some_and(|follow_id| {
            self.monster_idle_uses_dist_branch(
                monster_id,
                m.base.position,
                follow_id,
                target_distance,
            )
        });
        if uses_dist_branch || m.melee_skill <= 0 {
            return false;
        }
        chebyshev(m.base.position, target_pos) > 1
    }

    /// C++ `TCombat::CanToDoAttack` close branch — `crcombat.cc:441`, `:496-498`.
    ///
    /// When `CHASE_MODE_CLOSE` and cheb>1, queues `ToDoGo(false, 3)` ahead of `TDAttack`.
    pub(crate) fn monster_combat_enqueue_close_chase_go(
        &mut self,
        cid: CreatureId,
    ) -> MonsterCombatCloseChaseEnqueue {
        if !self.beat_driven_loop {
            return MonsterCombatCloseChaseEnqueue::Skipped;
        }
        let (chase_mode, attack_id, pos, fleeing) = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
                return MonsterCombatCloseChaseEnqueue::Skipped;
            };
            (
                m.base.chase_mode,
                m.base.attack_target,
                m.base.position,
                m.is_fleeing(),
            )
        };
        if chase_mode != ChaseMode::Close || fleeing {
            return MonsterCombatCloseChaseEnqueue::Skipped;
        }
        let Some(attack_id) = attack_id else {
            return MonsterCombatCloseChaseEnqueue::Skipped;
        };
        debug_assert!(
            self.creatures
                .get(cid)
                .is_none_or(|k| { k.base().follow_target == Some(attack_id) }),
            "close-chase repath requires follow_target == attack_target"
        );
        if self
            .creatures
            .get(attack_id)
            .is_none_or(|k| k.base().health <= 0)
        {
            return MonsterCombatCloseChaseEnqueue::Skipped;
        }
        let target_pos = self.creatures.get(attack_id).unwrap().position();
        let cheb = chebyshev(pos, target_pos);
        // C++ `CanToDoAttack` close `ToDoGo` at cheb>1 — LOS gates strike only (`crcombat.cc:496`).
        if cheb <= 1 {
            return MonsterCombatCloseChaseEnqueue::Skipped;
        }
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().has_follow_path = false;
        }
        let (max_steps, must_reach) = monster_idle_chase_step_budget(true, false, cheb, 1);
        let outcome =
            self.monster_idle_chase_repath(cid, Some("attack_close_chase"), max_steps, must_reach);
        if outcome == MonsterIdleChaseRepathOutcome::Noway {
            // C++ `CanToDoAttack` close-chase `ToDoGo` throws NOWAY when `TShortway::Calculate`
            // finds no path (`cract.cc:1104`). This propagates up to the `IdleStimulus`
            // `catch(RESULT r)` block (`crnonpl.cc:2890-2898`) which clears `Target` and, for
            // NOWAY (non-EXHAUSTED), falls through to the idle-wandering roam tail. The caller
            // (`monster_idle_maybe_enqueue_attack` / execute-Attack path) performs the
            // clear-target + roam fall-through — mirroring C++. Was: `Retry` (keep target +
            // wait), which left the monster parked indefinitely re-failing the same pathfind.
            return MonsterCombatCloseChaseEnqueue::Noway;
        }
        if outcome != MonsterIdleChaseRepathOutcome::PathQueued {
            return MonsterCombatCloseChaseEnqueue::Skipped;
        }
        if !self.enqueue_creature_go_at(cid, true) {
            if self.monster_close_chase_go_already_armed(cid) {
                return MonsterCombatCloseChaseEnqueue::Queued;
            }
            return MonsterCombatCloseChaseEnqueue::Skipped;
        }
        if chase_debug::chase_path_debug_enabled() {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                chase_debug::log_todo_go_aligned(
                    self.chase_trace_tick(),
                    cid,
                    m.base.name.as_str(),
                    pos,
                    target_pos,
                    false,
                    CHASE_PATH_MAX_STEPS as i32,
                    Some("attack_close_chase"),
                );
            }
        }
        MonsterCombatCloseChaseEnqueue::Queued
    }

    /// Close-chase repath populated `walk_queue` and `ToDoGo` is already queued (mid-batch re-arm).
    pub(crate) fn monster_close_chase_go_already_armed(&self, cid: CreatureId) -> bool {
        self.creatures.get(cid).is_some_and(|k| {
            let base = k.base();
            base.todo.has_go() && !base.walk_queue.is_empty()
        })
    }

    /// Close-chase `ToDoGo` batch mid-drain — do not idle-repath on target kite steps.
    ///
    /// C++ executes the initial chase path without per-tile `IdleStimulus` while segment
    /// pacing holds the next step (`crmain.cc:920-966`, `crnonpl.cc:2959`).
    pub(crate) fn monster_close_chase_batch_in_flight(&self, cid: CreatureId) -> bool {
        self.creatures.get(cid).is_some_and(|k| {
            let base = k.base();
            base.todo.locked
                || base.next_wakeup.is_some()
                || !base.walk_queue.is_empty()
                || self.monster_close_chase_go_already_armed(cid)
                || (base.todo.has_attack() && !base.todo.has_go())
        })
    }

    /// Follower stalled with no todo/walk/wakeup while still off the follow band.
    pub(crate) fn monster_chase_stalled_without_wakeup(&self, cid: CreatureId) -> bool {
        if !self.beat_driven_loop {
            return false;
        }
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return false;
        };
        if m.is_idle || m.is_fleeing() {
            return false;
        }
        let base = &m.base;
        let chase_target = base.follow_target.or(base.attack_target);
        if chase_target.is_none() {
            return false;
        }
        if !base.todo.is_empty() || base.next_wakeup.is_some() {
            return false;
        }
        let Some(follow_id) = chase_target else {
            return false;
        };
        let Some(target_pos) = self.creatures.get(follow_id).map(|k| k.position()) else {
            return false;
        };
        let target_distance = self.monster_effective_target_distance(m.target_distance);
        if self.monster_at_follow_goal(
            cid,
            follow_id,
            base.position,
            target_pos,
            false,
            target_distance,
        ) {
            return false;
        }
        chebyshev(base.position, target_pos) > 1
    }

    /// Combat monster lost its self-refresh cadence — dead todo queue or parked on target.
    pub(crate) fn monster_combat_scheduler_needs_refresh(&self, cid: CreatureId) -> bool {
        if !self.beat_driven_loop {
            return false;
        }
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return false;
        };
        if m.is_idle || m.is_fleeing() {
            return false;
        }
        let base = &m.base;
        if base.next_wakeup.is_some() {
            return false;
        }
        if !base.todo.is_empty() {
            return true;
        }
        let has_combat_target = base.follow_target.is_some() || base.attack_target.is_some();
        if !has_combat_target {
            return false;
        }
        let Some(target_id) = base.follow_target.or(base.attack_target) else {
            return false;
        };
        let Some(target_pos) = self.creatures.get(target_id).map(|k| k.position()) else {
            return false;
        };
        let target_distance = self.monster_effective_target_distance(m.target_distance);
        if !self.monster_at_follow_goal(
            cid,
            target_id,
            base.position,
            target_pos,
            false,
            target_distance,
        ) {
            return true;
        }
        // At melee band with live target — reference idle tail keeps re-arming attack/dance.
        target_distance <= 1 && m.melee_skill > 0 && has_combat_target
    }

    /// Re-arm todo drain or idle when [`Self::monster_combat_scheduler_needs_refresh`] is set.
    pub(crate) fn monster_combat_reschedule_if_stalled(&mut self, cid: CreatureId) {
        if !self.monster_combat_scheduler_needs_refresh(cid) {
            return;
        }
        if !self.creature_todo_queue_empty(cid) {
            self.schedule_immediate_todo_wakeup(cid);
            return;
        }
        // Stalled mid-batch: walk_queue has steps but no armed Go (`cract.cc:728`).
        if self.creatures.get(cid).is_some_and(|k| {
            let base = k.base();
            !base.walk_queue.is_empty() && !base.todo.has_go()
        }) {
            let _ = self.enqueue_creature_go_at(cid, true);
            if self.todo_start_go_delay(cid, false) {
                self.schedule_immediate_todo_wakeup(cid);
            }
            return;
        }
        self.request_idle_stimulus(cid);
    }

    /// 772 idle melee/dist dance — `crnonpl.cc:2736`, `2772` (rand(0,4) cardinal sidestep).
    ///
    /// 1098 `getDanceStep` / `staticAttackChance` must not be used on this path — see
    /// `monster_next_walk_step` 772 early return (X4).
    ///
    /// Returns true when a single lateral step was queued.
    pub(crate) fn monster_idle_dance_step(&mut self, cid: CreatureId) -> bool {
        let (pos, follow_id, target_distance) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => {
                let Some(follow_id) = m.base.follow_target else {
                    return false;
                };
                if m.base.attack_target != Some(follow_id) {
                    return false;
                }
                (
                    m.base.position,
                    follow_id,
                    self.monster_effective_target_distance(m.target_distance),
                )
            }
            _ => return false,
        };
        let target_pos = match self.creatures.get(follow_id) {
            Some(k) => k.position(),
            None => return false,
        };
        let band = if target_distance > 1 {
            target_distance
        } else {
            1
        };
        if chebyshev(pos, target_pos) != band {
            return false;
        }
        let choice = self.sim_dance_choice();
        let dir = crate::sim_glibc_rand::DANCE_DIR_ORDER[choice as usize];
        let dest = match dir {
            Some(step) => {
                let dest = pos.offset(step);
                if chebyshev(dest, target_pos) != band || !self.monster_can_walk_to(cid, pos, step) {
                    return false;
                }
                dest
            }
            None => {
                // C++ `rand()%5` case 4 — hold at band; still logs when DestDistance==1 (`crnonpl.cc:2814-2827`).
                if chebyshev(pos, target_pos) != band {
                    return false;
                }
                pos
            }
        };
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            base.walk_queue.clear();
            if let Some(step) = dir {
                base.walk_queue.push_back(step);
            }
            base.has_follow_path = true;
            base.force_update_follow_path = false;
        }
        // C++ `crnonpl.cc:2830` — successful melee dance promotes PANIC → ATTACKING.
        if band == 1 {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                if m.state == MonsterState::Panic {
                    m.state = MonsterState::Attacking;
                }
            }
        }
        if chase_debug::chase_path_debug_enabled() {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                let branch = if target_distance > 1 {
                    "dist_dance"
                } else {
                    "melee_dance"
                };
                chase_debug::log_branch(
                    self.chase_trace_tick(),
                    cid,
                    m.base.name.as_str(),
                    branch,
                    pos,
                    dest,
                    true,
                    i32::MAX,
                    None,
                );
            }
        }
        true
    }

    /// 772 idle master follow — `crnonpl.cc:2686` (`ToDoGo` max 3; Manhattan 2–3 hold).
    pub(crate) fn monster_idle_master_follow(
        &mut self,
        cid: CreatureId,
        repath_reason: Option<&str>,
    ) -> MonsterIdleChaseRepathOutcome {
        let (pos, follow_id) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => {
                let Some(follow_id) = m.base.follow_target else {
                    return MonsterIdleChaseRepathOutcome::AtGoal;
                };
                (m.base.position, follow_id)
            }
            _ => return MonsterIdleChaseRepathOutcome::AtGoal,
        };
        let target_pos = match self.creatures.get(follow_id) {
            Some(k) => k.position(),
            None => return MonsterIdleChaseRepathOutcome::AtGoal,
        };
        let dist = manhattan(pos, target_pos);
        if dist <= 1 {
            return MonsterIdleChaseRepathOutcome::AtGoal;
        }
        if monster_master_follow_in_wait_band(dist) {
            if chase_debug::chase_path_debug_enabled() {
                if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                    chase_debug::log_branch(
                        self.chase_trace_tick(),
                        cid,
                        m.base.name.as_str(),
                        "master_follow_wait",
                        pos,
                        target_pos,
                        false,
                        0,
                        repath_reason,
                    );
                }
            }
            return MonsterIdleChaseRepathOutcome::AtGoal;
        }
        if chase_debug::chase_path_debug_enabled() {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                chase_debug::log_branch(
                    self.chase_trace_tick(),
                    cid,
                    m.base.name.as_str(),
                    "master_follow",
                    pos,
                    target_pos,
                    false,
                    CHASE_PATH_MAX_STEPS as i32,
                    repath_reason,
                );
            }
        }
        self.monster_idle_chase_repath(cid, repath_reason, CHASE_PATH_MAX_STEPS, false)
    }

    /// TFS `Creature::goToFollowCreature` — `creature.cpp` ~1011 (1098 only).
    pub fn go_to_follow_creature(&mut self, cid: CreatureId, repath_reason: Option<&str>) {
        if self.beat_driven_loop {
            let branch = self.monster_idle_classify_walk_branch(cid);
            let cheb = self
                .creatures
                .get(cid)
                .and_then(|k| {
                    let follow_id = k.base().follow_target?;
                    let target_pos = self.creatures.get(follow_id)?.position();
                    Some(chebyshev(k.position(), target_pos))
                })
                .unwrap_or(0);
            let is_melee_chase = branch == crate::idle_stimulus::MonsterIdleWalkBranch::MeleeChase;
            let is_dist_chase = branch == crate::idle_stimulus::MonsterIdleWalkBranch::DistChase;
            let target_distance = self
                .creatures
                .get(cid)
                .map(|k| match k {
                    CreatureKind::Monster(m) => {
                        self.monster_effective_target_distance(m.target_distance)
                    }
                    _ => 1,
                })
                .unwrap_or(1);
            let (max_steps, must_reach) = monster_idle_chase_step_budget(
                is_melee_chase,
                is_dist_chase,
                cheb,
                target_distance,
            );
            if self.monster_idle_chase_repath(cid, repath_reason, max_steps, must_reach)
                == MonsterIdleChaseRepathOutcome::PathQueued
            {
                self.idle_enqueue_go_and_start(cid, true, repath_reason);
            }
            return;
        }
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().force_update_follow_path = false;
        }
        let follow_id = match self.creatures.get(cid).and_then(|k| k.base().follow_target) {
            Some(id) => id,
            None => return,
        };
        let target_pos = match self.creatures.get(follow_id) {
            Some(k) => k.position(),
            None => return,
        };
        let (target_distance, fleeing, is_summon, has_follow_path) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                self.monster_effective_target_distance(m.target_distance),
                m.is_fleeing(),
                m.base.is_summon(),
                m.base.has_follow_path,
            ),
            _ => return,
        };

        let fpp = self.monster_path_search_params(
            cid,
            follow_id,
            fleeing,
            target_distance,
            is_summon,
            has_follow_path,
        );

        let pos = match self.creatures.get(cid) {
            Some(k) => k.position(),
            None => return,
        };
        if self.monster_at_follow_goal(cid, follow_id, pos, target_pos, fleeing, target_distance) {
            self.monster_mark_at_follow_goal(cid, follow_id);
            return;
        }

        // TFS `Creature::goToFollowCreature` — getDistanceStep when fleeing or maxTargetDist > 1
        // (`creature.cpp` ~1018–1034); not gated on `canUseAttack`.
        // Gated to 1098 only (!self.beat_driven_loop).
        let use_distance_step =
            !self.beat_driven_loop && !is_summon && (fleeing || target_distance > 1);

        if use_distance_step {
            let sight = self.map.is_sight_clear(pos, target_pos);
            let mut rng = rand::thread_rng();
            let can_walk = |dir: Direction| self.monster_can_walk_to(cid, pos, dir);
            match get_distance_step(
                pos,
                target_pos,
                target_distance,
                fleeing,
                sight,
                can_walk,
                &mut rng,
            ) {
                DistanceStepOutcome::Step(dir) => {
                    self.monster_start_follow_step(cid, dir);
                    if self
                        .creatures
                        .get(cid)
                        .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
                    {
                        self.monster_on_follow_creature_complete(cid, follow_id);
                    }
                    return;
                }
                DistanceStepOutcome::AtTargetDistance => {
                    // C++ `hasFollowPath` stays true at keep-distance so `onCreatureMove` repaths
                    // when the target leaves the band (`creature.cpp` ~619–637).
                    self.monster_mark_at_follow_goal(cid, follow_id);
                    return;
                }
                DistanceStepOutcome::NeedPathfinding => {
                    if fleeing {
                        if let Some(k) = self.creatures.get_mut(cid) {
                            k.base_mut().has_follow_path = false;
                        }
                        if self
                            .creatures
                            .get(cid)
                            .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
                        {
                            self.monster_on_follow_creature_complete(cid, follow_id);
                        }
                        return;
                    }
                    // keep-distance: fall through to A* when getDistanceStep fails.
                }
            }
        }

        if self.monster_try_apply_chase_path(
            cid,
            target_pos,
            fleeing,
            target_distance,
            &fpp,
            CHASE_PATH_MAX_STEPS,
            false,
        ) {
            if self
                .creatures
                .get(cid)
                .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
            {
                self.monster_on_follow_creature_complete(cid, follow_id);
            }
            return;
        }
        if fleeing {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().has_follow_path = false;
            }
            if self
                .creatures
                .get(cid)
                .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
            {
                self.monster_on_follow_creature_complete(cid, follow_id);
            }
            return;
        }

        let pos = match self.creatures.get(cid) {
            Some(k) => k.position(),
            None => return,
        };
        if self.monster_at_follow_goal(cid, follow_id, pos, target_pos, fleeing, target_distance) {
            self.monster_mark_at_follow_goal(cid, follow_id);
            return;
        }
        if self.monster_try_any_closer_step(cid, pos, target_pos, follow_id)
            || self.monster_try_greedy_chase_step(cid, pos, target_pos, follow_id, fleeing)
        {
            return;
        }

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().has_follow_path = false;
        }

        if self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
        {
            self.monster_on_follow_creature_complete(cid, follow_id);
        }
    }

    /// Apply A* (primary + relaxed) or return false so caller can try one-tile fallbacks.
    #[allow(clippy::too_many_arguments)]
    fn monster_try_apply_chase_path(
        &mut self,
        cid: CreatureId,
        target_pos: Position,
        fleeing: bool,
        target_distance: i32,
        fpp: &FindPathParams,
        max_steps: usize,
        must_reach: bool,
    ) -> bool {
        let pos = match self.creatures.get(cid) {
            Some(k) => k.position(),
            None => return false,
        };
        let relaxed = FindPathParams {
            min_target_dist: 1,
            max_target_dist: if fleeing {
                i32::from(MAP_MAX_VIEWPORT)
            } else {
                1
            },
            clear_sight: false,
            allow_diagonal: true,
            full_path_search: true,
            max_search_dist: 0,
        };
        let tries: &[&FindPathParams] = if self.beat_driven_loop {
            &[fpp]
        } else {
            &[fpp, &relaxed]
        };
        for &try_fpp in tries {
            let Some(mut steps) = self.get_creature_path_to_with_fpp(cid, target_pos, try_fpp)
            else {
                continue;
            };
            if steps.is_empty() {
                let dist = chebyshev(pos, target_pos);
                if dist > 1.max(target_distance) {
                    continue;
                }
            } else if self.beat_driven_loop {
                // 772 `ToDoGo` trim — `cract.cc:241-258`; melee adjacent uses `must:1` max 1.
                // `get_creature_path_to_with_fpp` returns C++ predecessor-chain order (first hop first).
                let stop_at_cheb = if fleeing || target_distance <= 1 {
                    1
                } else {
                    target_distance
                };
                steps = crate::pathfinding::truncate_cipsoft_chase_queue(
                    pos,
                    target_pos,
                    steps,
                    max_steps,
                    must_reach,
                    stop_at_cheb,
                );
                // `truncate_cipsoft_chase_queue` returns execution order (first step first).
            }
            if self.beat_driven_loop && chase_debug::chase_path_debug_enabled() {
                if let Some(k) = self.creatures.get(cid) {
                    let name = k.base().name.clone();
                    let mut path_positions = Vec::with_capacity(steps.len());
                    let mut cursor = pos;
                    for &dir in &steps {
                        cursor = cursor.offset(dir);
                        path_positions.push(cursor);
                    }
                    let min_wp =
                        scan_min_terrain_waypoints(&self.map, pos, REVERSE_PATH_VIEW_RADIUS, |p| {
                            self.map
                                .get_tile(p)
                                .filter(|_| self.map.is_walkable(p))
                                .map(|t| self.tile_ground_speed(t.body()))
                                .unwrap_or(0)
                        });
                    chase_debug::log_shortway(
                        self.chase_trace_tick(),
                        cid,
                        name.as_str(),
                        pos,
                        target_pos,
                        10,
                        min_wp,
                        must_reach,
                        max_steps as i32,
                        true,
                        &path_positions,
                    );
                }
            }
            if self.beat_driven_loop && steps.is_empty() {
                continue;
            }
            if let Some(k) = self.creatures.get_mut(cid) {
                let base = k.base_mut();
                base.walk_queue.clear();
                // `listWalkDir` — `getNextStep` pops from the back (`creature.cpp`).
                for d in steps.iter().rev() {
                    base.walk_queue.push_back(*d);
                }
                base.has_follow_path = true;
            }
            // 772 idle executor owns `Go` enqueue via `monster_idle_prepare_and_enqueue_go`.
            if !self.beat_driven_loop {
                self.monster_start_chase_walk(cid, true);
            }
            return true;
        }
        if self.beat_driven_loop && chase_debug::chase_path_debug_enabled() {
            if let Some(k) = self.creatures.get(cid) {
                let name = k.base().name.clone();
                chase_debug::log_shortway(
                    self.chase_trace_tick(),
                    cid,
                    name.as_str(),
                    pos,
                    target_pos,
                    10,
                    scan_min_terrain_waypoints(&self.map, pos, REVERSE_PATH_VIEW_RADIUS, |p| {
                        self.map
                            .get_tile(p)
                            .filter(|_| self.map.is_walkable(p))
                            .map(|t| self.tile_ground_speed(t.body()))
                            .unwrap_or(0)
                    }),
                    false,
                    CHASE_PATH_MAX_STEPS as i32,
                    false,
                    &[],
                );
            }
        }
        false
    }

    /// Pick any legal step that reduces Chebyshev distance (obstacle / corridor sidestep).
    fn monster_try_any_closer_step(
        &mut self,
        cid: CreatureId,
        pos: Position,
        target_pos: Position,
        follow_id: CreatureId,
    ) -> bool {
        let current = chebyshev(pos, target_pos);
        let mut best: Option<(Direction, i32)> = None;
        let dirs = if self.beat_driven_loop {
            &[
                Direction::North,
                Direction::East,
                Direction::South,
                Direction::West,
            ][..]
        } else {
            &CHASE_STEP_DIRECTIONS[..]
        };
        for &dir in dirs {
            if !self.monster_can_walk_to(cid, pos, dir) {
                continue;
            }
            let to = pos.offset(dir);
            let d = chebyshev(to, target_pos);
            if d < current && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                best = Some((dir, d));
            }
        }
        let Some((dir, _)) = best else {
            return false;
        };
        self.monster_start_follow_step(cid, dir);
        if self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
        {
            self.monster_on_follow_creature_complete(cid, follow_id);
        }
        true
    }

    /// One-step chase when A* fails or returns empty while still out of melee reach — TFS
    /// `getDistanceStep` before `getPathTo` (`creature.cpp` ~1011–1046).
    fn monster_try_greedy_chase_step(
        &mut self,
        cid: CreatureId,
        pos: Position,
        target_pos: Position,
        follow_id: CreatureId,
        fleeing: bool,
    ) -> bool {
        let sight = self.map.is_sight_clear(pos, target_pos);
        let mut rng = std::mem::replace(&mut self.ai_rng, StdRng::from_entropy());
        let can_walk = |dir: Direction| self.monster_can_walk_to(cid, pos, dir);
        let stepped =
            match get_distance_step(pos, target_pos, 1, fleeing, sight, can_walk, &mut rng) {
                DistanceStepOutcome::Step(dir) => {
                    self.monster_start_follow_step(cid, dir);
                    if self
                        .creatures
                        .get(cid)
                        .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
                    {
                        self.monster_on_follow_creature_complete(cid, follow_id);
                    }
                    true
                }
                _ => false,
            };
        self.ai_rng = rng;
        stepped
    }

    fn monster_start_chase_walk(&mut self, cid: CreatureId, first_step: bool) {
        if self.beat_driven_loop {
            self.idle_enqueue_go_and_start(cid, first_step, None);
        } else {
            self.creature_start_chase_auto_walk(cid);
        }
    }

    fn monster_start_follow_step(&mut self, cid: CreatureId, dir: Direction) {
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            base.walk_queue.clear();
            base.walk_queue.push_back(dir);
            base.has_follow_path = true;
        }
        // Let the active walk timer continue naturally rather than cancelling/restarting
        self.monster_start_chase_walk(cid, true);
    }

    fn monster_path_search_params(
        &self,
        cid: CreatureId,
        follow_id: CreatureId,
        fleeing: bool,
        target_distance: i32,
        is_summon: bool,
        has_follow_path: bool,
    ) -> FindPathParams {
        let pos = self
            .creatures
            .get(cid)
            .map(|k| k.position())
            .unwrap_or(Position::new(0, 0, 7));
        let target_pos = self.creatures.get(follow_id).map(|k| k.position());
        let mut fpp = FindPathParams {
            min_target_dist: 1,
            max_target_dist: target_distance,
            clear_sight: true,
            // 772 `TShortway::Expand` always considers all 8 neighbors; diagonals are
            // discouraged by 3× waypoint cost, not by removing them from the search graph.
            allow_diagonal: true,
            full_path_search: !has_follow_path,
            // 772: `TShortway` uses VisibleX/Y=10 internally — not TFS `maxSearchDist=12` (`creature.cpp`).
            max_search_dist: if self.beat_driven_loop { 0 } else { 12 },
        };

        if is_summon {
            let master = self.creatures.get(cid).and_then(|k| k.base().master);
            if master == Some(follow_id) {
                fpp.max_target_dist = 2;
                fpp.full_path_search = true;
            } else if target_distance <= 1 {
                fpp.full_path_search = true;
            } else if self.beat_driven_loop {
                fpp.full_path_search =
                    target_pos.is_some_and(|tp| chebyshev(pos, tp) != target_distance);
            } else {
                fpp.full_path_search = !self.monster_can_use_attack(cid, pos, follow_id);
            }
        } else if fleeing {
            fpp.max_target_dist = i32::from(MAP_MAX_VIEWPORT);
            fpp.clear_sight = false;
            fpp.full_path_search = false;
        } else if target_distance <= 1 {
            fpp.full_path_search = true;
        } else if self.beat_driven_loop {
            // 772 `DistanceFighting` — cheb band, not TFS `canUseAttack` (`crnonpl.cc:2723`).
            fpp.full_path_search =
                target_pos.is_some_and(|tp| chebyshev(pos, tp) != target_distance);
        } else {
            // TFS `Monster::getPathSearchParams` — `maxTargetDist` stays at targetDistance;
            // only `fullPathSearch` toggles on `canUseAttack` (`monster.cpp` ~2111–2115).
            fpp.full_path_search = !self.monster_can_use_attack(cid, pos, follow_id);
        }

        fpp
    }

    fn get_creature_path_to_with_fpp(
        &self,
        cid: CreatureId,
        target: Position,
        fpp: &FindPathParams,
    ) -> Option<Vec<Direction>> {
        use crate::pathfinding::{get_path_matching_with_fill, CREATURE_ON_TILE_PATH_COST};

        let start = self.creatures.get(cid)?.position();
        struct PathCtx<'a> {
            world: &'a GameWorld,
            cid: CreatureId,
        }
        let ctx = PathCtx { world: self, cid };
        let uses_reverse_terrain = uses_reverse_terrain_path(
            self.mechanics.profile.path_cost,
            self.mechanics.profile.path_search,
        );
        debug_assert!(
            !self.beat_driven_loop || uses_reverse_terrain,
            "772 monster chase requires reverse TShortway + terrain costs (check MechanicsProfile / formulas lua)"
        );
        let fill_walkable = |pos: Position| {
            if uses_reverse_terrain && self.beat_driven_loop {
                ctx.world
                    .monster_tshortway_fill_walkable(ctx.cid, pos, target)
            } else {
                ctx.world.monster_can_occupy_chase_tile(ctx.cid, pos)
            }
        };
        get_path_matching_with_fill(
            &self.map,
            start,
            target,
            fpp,
            self.mechanics.profile.path_cost,
            self.mechanics.profile.path_search,
            self.mechanics.profile.path_forward_fallback,
            fill_walkable,
            |pos| {
                if uses_reverse_terrain {
                    return 0;
                }
                let Some(tile) = ctx.world.map.get_tile(pos) else {
                    return 0;
                };
                let mut cost = 0u32;
                for &c in tile.body().creatures.iter() {
                    if c != ctx.cid {
                        cost += CREATURE_ON_TILE_PATH_COST;
                    }
                }
                cost
            },
            // Heuristic / non-tshortway reverse paths still use ground speed.
            |pos| {
                let Some(tile) = ctx.world.map.get_tile(pos) else {
                    return 0;
                };
                ctx.world.tile_ground_speed(tile.body())
            },
            |pos| ctx.world.fillmap_terrain_waypoints_at(pos),
        )
    }

    /// Recompute chase path immediately — C++ `Creature::onCreatureMove` instant repath
    /// (`creature.cpp` ~619–637) and avoids waiting for `onThink` (1 s bucket).
    /// Walk execution stays in `creature_start_chase_auto_walk` / scheduler — do not call
    /// `check_creature_walk` here (would deepen the `onWalk` stack and risk recursion on blocked tiles).
    pub(crate) fn monster_follow_repath_now(
        &mut self,
        cid: CreatureId,
        repath_reason: Option<&str>,
    ) {
        if !self.creatures.get(cid).is_some_and(|k| {
            matches!(k, CreatureKind::Monster(_)) && k.base().follow_target.is_some()
        }) {
            return;
        }
        self.go_to_follow_creature(cid, repath_reason);
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            base.force_update_follow_path = false;
            base.is_updating_path = false;
        }
    }

    pub(crate) fn monster_think_summon_stub(&mut self, cid: CreatureId) {
        let (master, attack) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (m.base.master, m.base.attack_target),
            _ => return,
        };
        if attack.is_none() {
            if let Some(master_id) = master {
                if let Some(master_attack) = self
                    .creatures
                    .get(master_id)
                    .and_then(|k| k.base().attack_target)
                {
                    let _ = self.monster_select_target(cid, master_attack);
                } else if self.creatures.get(cid).map(|k| k.base().follow_target) != Some(master) {
                    let _ = self.monster_set_follow_creature(cid, master);
                }
            }
        } else if attack == Some(cid) {
            let _ = self.monster_set_follow_creature(cid, None);
        } else if let Some(attack_id) = attack {
            if self.creatures.get(cid).map(|k| k.base().follow_target) != Some(Some(attack_id)) {
                let _ = self.monster_set_follow_creature(cid, Some(attack_id));
            }
        }
    }

    /// TFS `Monster::onThinkTarget` — `monster.cpp` ~923 (1098 only).
    ///
    /// 772 target pick / retarget runs from [`crate::idle_stimulus`] `Strategy[]`
    /// (`crnonpl.cc:2468`), not `changeTargetSpeed` rolls.
    pub(crate) fn monster_on_think_target(&mut self, cid: CreatureId, interval_ms: u32) {
        if self.beat_driven_loop {
            return;
        }
        let (
            change_speed,
            change_chance,
            target_distance,
            is_summon,
            mut target_change_ticks,
            mut target_change_cooldown,
            mut challenge_focus_duration,
        ) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                m.change_target_speed,
                m.change_target_chance,
                self.monster_effective_target_distance(m.target_distance),
                m.base.is_summon(),
                m.target_change_ticks,
                m.target_change_cooldown,
                m.challenge_focus_duration,
            ),
            _ => return,
        };

        if is_summon || change_speed == 0 {
            return;
        }

        let mut can_change_target = true;

        if challenge_focus_duration > 0 {
            challenge_focus_duration = challenge_focus_duration.saturating_sub(interval_ms);
        }

        if target_change_cooldown > 0 {
            target_change_cooldown = target_change_cooldown.saturating_sub(interval_ms);
            if target_change_cooldown == 0 {
                target_change_ticks = change_speed;
            } else {
                can_change_target = false;
            }
        }

        if can_change_target {
            target_change_ticks = target_change_ticks.saturating_add(interval_ms);
            if target_change_ticks >= change_speed {
                target_change_ticks = 0;
                target_change_cooldown = change_speed;
                if challenge_focus_duration > 0 {
                    challenge_focus_duration = 0;
                }
                let roll = i32::try_from(rand::random::<u32>() % 100 + 1).unwrap_or(100);
                if change_chance >= roll {
                    if target_distance <= 1 {
                        let _ = self.monster_search_target(cid, TargetSearchType::Random);
                    } else {
                        let _ = self.monster_search_target(cid, TargetSearchType::Nearest);
                    }
                }
            }
        }

        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.target_change_ticks = target_change_ticks;
            m.target_change_cooldown = target_change_cooldown;
            m.challenge_focus_duration = challenge_focus_duration;
        }
    }

    /// Re-arm walk timer while actively chasing with an empty queue so `getNextStep` can repath.
    pub(crate) fn monster_should_keep_chase_walk_alive(&self, cid: CreatureId) -> bool {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return false;
        };
        !m.is_idle
            && m.base.health > 0
            && m.base.walk_queue.is_empty()
            && m.base.follow_target.is_some()
            && !m.walking_to_spawn
    }

    /// True when an active melee chase should keep polling `getDanceStep` with an armed walk timer.
    pub(crate) fn monster_should_keep_dance_walk_alive(&self, cid: CreatureId) -> bool {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return false;
        };
        !m.is_idle
            && m.base.health > 0
            && m.base.walk_queue.is_empty()
            && m.base.follow_target.is_some()
            && m.base.follow_target == m.base.attack_target
    }

    /// C++ `Monster::onThink` `addEventWalk()` — `monster.cpp` ~772.
    /// Unlike players, monsters arm walk while active even with an empty queue so
    /// `Monster::getNextStep` can random-roam or wait for the next flee/chase step.
    fn monster_arm_event_walk(&mut self, cid: CreatureId) {
        let (should_arm, chasing) = self
            .creatures
            .get(cid)
            .map(|k| {
                (
                    k.base().health > 0 && k.base().walk_timer_idle(self.beat_driven_loop),
                    k.base().follow_target.is_some(),
                )
            })
            .unwrap_or((false, false));
        if should_arm {
            if chasing {
                self.monster_start_chase_walk(cid, true);
            } else {
                self.creature_start_auto_walk(cid);
            }
        }
    }

    /// TFS `Monster::updateLookDirection` + `0x6B` broadcast.
    ///
    /// NOTE: the ATTACKING/PANIC rotate-toward-attack-target path no longer calls this
    /// function — it calls [`GameWorld::monster_execute_rotate_toward`] directly via
    /// [`GameWorld::monster_idle_rotate_toward_attack_target`], which has NO
    /// `walk_timer_idle` gate (matching C++'s unconditional `Rotate(Target)` at
    /// `crnonpl.cc:2872-2873`). This function is still used by the casting turn
    /// (`monster_idle_try_casting`) and the 1098 `onThink` path, where the
    /// `walk_timer_idle` gate remains correct.
    pub fn monster_update_look_direction(&mut self, cid: CreatureId) {
        let (pos, target_id, current, is_idle) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                m.base.position,
                m.base.attack_target,
                m.base.direction,
                m.base.walk_timer_idle(self.beat_driven_loop),
            ),
            _ => return,
        };
        if !is_idle {
            return;
        }
        let Some(target_id) = target_id else {
            return;
        };
        let target_pos = match self.creatures.get(target_id) {
            Some(k) => k.position(),
            None => return,
        };
        let new_dir = compute_look_toward_target(pos, target_pos, current);
        if new_dir != current {
            creature_turn_with_broadcast(self, cid, new_dir);
            if chase_debug::chase_path_debug_enabled() {
                if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                    chase_debug::log_rotate(
                        self.chase_trace_tick(),
                        cid,
                        m.base.name.as_str(),
                        new_dir as u8,
                        Some(target_id.data().as_ffi()),
                    );
                }
            }
        }
    }

    /// TFS `Monster::walkToSpawn` — `monster.cpp` ~1087.
    pub fn monster_walk_to_spawn(&mut self, cid: CreatureId) {
        let (pos, spawn, walking, has_opponents) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                m.base.position,
                m.spawn_position,
                m.walking_to_spawn,
                !m.opponent_ids.is_empty(),
            ),
            _ => return,
        };
        if walking || has_opponents {
            return;
        }
        let dist = chebyshev(pos, spawn);
        if dist == 0 {
            return;
        }
        let max_dist = 0_i32.max(dist - 5);
        let path = self.get_creature_path_to(cid, spawn, 0, max_dist);
        if path.is_none() {
            return;
        }
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.walking_to_spawn = true;
            m.base.walk_queue.clear();
            if let Some(path) = path {
                for d in path {
                    m.base.walk_queue.push_back(d);
                }
            }
            m.base.has_follow_path = true;
        }
        self.creature_start_auto_walk(cid);
    }

    /// TFS `Monster::onCreatureLeave` walk-back trigger — `monster.cpp` ~508–512.
    pub fn monster_maybe_walk_to_spawn(&mut self, cid: CreatureId) {
        if self.beat_driven_loop {
            return;
        }
        let (walking, is_summon, opponents_empty, pos, spawn) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                m.walking_to_spawn,
                m.base.is_summon(),
                m.opponent_ids.is_empty(),
                m.base.position,
                m.spawn_position,
            ),
            _ => return,
        };
        if walking || is_summon || !opponents_empty {
            return;
        }
        let radius = self.monster_world_config.walk_to_spawn_radius;
        if radius <= 0 || is_within_walk_to_spawn_range(pos, spawn, radius) {
            return;
        }
        self.monster_walk_to_spawn(cid);
    }

    /// Out-of-despawn-range handling — `monster.cpp` ~760–767.
    fn monster_handle_out_of_spawn_range(&mut self, cid: CreatureId) {
        let pos = match self.creatures.get(cid) {
            Some(k) => k.position(),
            None => return,
        };
        self.broadcast_magic_effect(pos, 4); // CONST_ME_POFF
        if self.monster_world_config.remove_on_despawn {
            self.remove_creature(cid);
        } else {
            self.monster_teleport_to_spawn(cid);
        }
    }

    /// TFS `Monster::onWalkComplete` spawn continuation — `monster.cpp` ~1113.
    pub fn monster_on_walk_complete(&mut self, cid: CreatureId) {
        let (walking_to_spawn, follow, queue_empty) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                m.walking_to_spawn,
                m.base.follow_target,
                m.base.walk_queue.is_empty(),
            ),
            _ => return,
        };

        if walking_to_spawn {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.walking_to_spawn = false;
            }
            self.monster_walk_to_spawn(cid);
            return;
        }

        if queue_empty {
            if let Some(target_id) = follow {
                let had_follow_path = self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| k.base().has_follow_path);
                self.monster_on_follow_creature_complete(cid, target_id);
                // 772: band reconcile + look runs from idle; 1098: reconcile after follow walk.
                if had_follow_path && !self.beat_driven_loop {
                    self.monster_reconcile_follow_position(cid, target_id);
                    self.monster_update_look_direction(cid);
                }
            }
        }
    }

    /// TFS `Monster::getNextStep` — `monster.cpp` ~1224.
    pub(crate) fn monster_next_walk_step(
        &mut self,
        cid: CreatureId,
        now: std::time::Instant,
    ) -> Option<Direction> {
        let (
            walking_to_spawn,
            is_idle,
            health,
            follow,
            attack,
            _has_path,
            is_summon,
            master,
            pos,
            fleeing,
            static_attack_chance,
            target_distance,
        ) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                m.walking_to_spawn,
                m.is_idle,
                m.base.health,
                m.base.follow_target,
                m.base.attack_target,
                m.base.has_follow_path,
                m.base.is_summon(),
                m.base.master,
                m.base.position,
                m.is_fleeing(),
                m.static_attack_chance,
                self.monster_effective_target_distance(m.target_distance),
            ),
            _ => return None,
        };

        if !walking_to_spawn && (is_idle || health <= 0) {
            return None;
        }

        let master_in_range = master.is_some_and(|mid| {
            self.creatures.get(mid).is_some_and(|master_kind| {
                creature_can_see(
                    pos,
                    master_kind.position(),
                    i32::from(MAP_MAX_VIEWPORT),
                    i32::from(MAP_MAX_VIEWPORT),
                    self.beat_driven_loop,
                )
            })
        });

        // 772: roam step picked in idle + `ToDoWait` pacing only (X6).
        if !walking_to_spawn
            && follow.is_none()
            && (!is_summon || !master_in_range)
            && !self.beat_driven_loop
        {
            let elapsed_ms = self
                .creatures
                .get(cid)
                .and_then(|k| k.base().last_step)
                .map(|last| now.duration_since(last).as_millis())
                .unwrap_or(u128::MAX);
            if elapsed_ms < 1000 {
                return None;
            }
            let can_walk = |dir: Direction| self.monster_can_walk_to(cid, pos, dir);
            let mut rng = rand::thread_rng();
            return get_random_step(can_walk, &mut rng);
        }

        if (is_summon && master_in_range) || follow.is_some() || walking_to_spawn {
            if let Some(k) = self.creatures.get_mut(cid) {
                if let Some(dir) = k.base_mut().walk_queue.pop_back() {
                    return Some(dir);
                }
            }

            // C++ `Creature::getNextStep` returns false when the queue is empty (`creature.cpp` ~251–260);
            // repath runs from `onThink` / target-move only, not synchronously from `getNextStep`.

            // C++ target dancing when follow queue empty — `monster.cpp` ~1244–1256.
            if follow == attack {
                if let Some(target_id) = follow {
                    let target_pos = self.creatures.get(target_id).map(|k| k.position())?;
                    let dist = chebyshev(pos, target_pos);
                    if self.beat_driven_loop {
                        // 772 idle drain owns flee/dance/chase — no TFS getNextStep poll (X4).
                        return None;
                    }

                    // C++ dance at attack distance (`monster.cpp` ~1249); melee uses 1 tile, not keep-distance 4.
                    let dance_range = if target_distance > 1
                        && self.monster_can_use_attack(cid, pos, target_id)
                    {
                        target_distance
                    } else {
                        1
                    };
                    if dist > dance_range {
                        return None;
                    }
                    let can_walk = |dir: Direction| self.monster_can_walk_to(cid, pos, dir);
                    let can_use_now = self.monster_can_use_attack(cid, pos, target_id);
                    let can_use_from =
                        |from: Position| self.monster_can_use_attack(cid, from, target_id);
                    let mut rng = rand::thread_rng();
                    if fleeing {
                        let step = get_dance_step(
                            pos,
                            target_pos,
                            false,
                            false,
                            can_walk,
                            can_use_from,
                            can_use_now,
                            &mut rng,
                        );
                        return step;
                    }
                    if static_attack_chance < rng.gen_range(1..=100) {
                        return get_dance_step(
                            pos,
                            target_pos,
                            true,
                            true,
                            can_walk,
                            can_use_from,
                            can_use_now,
                            &mut rng,
                        );
                    }
                }
            }
        }

        None
    }

    /// Out-of-range despawn: teleport to `spawn_position` (C++ `internalTeleport` branch).
    fn monster_teleport_to_spawn(&mut self, cid: CreatureId) {
        let (old_pos, spawn) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (m.base.position, m.spawn_position),
            _ => return,
        };
        if old_pos == spawn {
            return;
        }
        self.map.unregister_creature_at(old_pos, cid);
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.base.position = spawn;
            m.base.walk_queue.clear();
            m.base.has_follow_path = false;
            m.base.clear_targets();
            m.is_idle = true;
            m.walking_to_spawn = false;
        }
        self.map.register_creature_at(spawn, cid);
    }

    /// 772 `TShortway::FillMap` stack-head BANK `WAYPOINTS` (`cract.cc:89-99`) — OTB only, no `MovePossible`.
    pub(crate) fn fillmap_terrain_waypoints_at(&self, pos: Position) -> i32 {
        let Some(tile) = self.map.get_tile(pos) else {
            return -1;
        };
        let chain = tile.body().map_object_chain();
        let Some(MapStackEntry::Ground(server_id)) = chain.first() else {
            return -1;
        };
        if !self.items_db.is_terrain_bank_772(*server_id) || self.items_db.is_unpass_772(*server_id)
        {
            return -1;
        }
        let wp = self
            .items_db
            .waypoints_raw_for_item(*server_id)
            .unwrap_or(0);
        if wp == 0 {
            return -1;
        }
        wp as i32
    }

    /// 772 `TShortway::FillMap` per-tile weight after `MovePossible(Execute=false)` (`cract.cc:89-103`).
    pub(crate) fn fillmap_waypoints_at(
        &self,
        cid: CreatureId,
        pos: Position,
        target: Position,
    ) -> i32 {
        let wp = self.fillmap_terrain_waypoints_at(pos);
        if wp <= 0 || !self.monster_tshortway_fill_walkable(cid, pos, target) {
            return -1;
        }
        wp
    }

    /// Dump `TShortway::FillMap` viewport walkability for parity diff vs C++ (`cract.cc:80-114`).
    ///
    /// `radius` is typically [`REVERSE_PATH_VIEW_RADIUS`] (10). `state` is the monster posture
    /// at dump time (same for all tiles).
    pub fn dump_tshortway_fill_walkable_viewport(
        &self,
        cid: CreatureId,
        target: Position,
        radius: i32,
    ) -> (MonsterState, Vec<TShortwayFillTile>) {
        let state = self
            .creatures
            .get(cid)
            .and_then(|k| match k {
                CreatureKind::Monster(m) => Some(m.state),
                _ => None,
            })
            .unwrap_or(MonsterState::Idle);
        let origin = self
            .creatures
            .get(cid)
            .map(|k| k.position())
            .unwrap_or(target);
        let mut tiles = Vec::new();
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let x = origin.x as i32 + dx;
                let y = origin.y as i32 + dy;
                if x < 0 || y < 0 || x > i32::from(u16::MAX) || y > i32::from(u16::MAX) {
                    continue;
                }
                let pos = Position::new(x as u16, y as u16, origin.z);
                let wp = self.fillmap_waypoints_at(cid, pos, target);
                let walkable = wp > 0;
                tiles.push(TShortwayFillTile { pos, walkable, wp });
            }
        }
        (state, tiles)
    }

    /// 772 `TShortway::FillMap` walkability — `TMonster::MovePossible(Execute=false)` (`crnonpl.cc:2141`).
    ///
    /// Chase fill may plan through pushable creatures when `can_push_creatures`; unpushable
    /// monsters always block. Follow/attack target tile is **not** walkable (`Target` match).
    /// 772 `TMonster::MovePossible(Execute=false)` creature/item gate — `crnonpl.cc:2141–2293`.
    ///
    /// This is the planning-phase `MovePossible` check (no side effects). It validates:
    /// - Home-radius leash (non-ATTACKING/PANIC only)
    /// - PZ / house / floorchange / teleport tile blocks
    /// - Creature-block gate (772 model: no `!is_summon`, player plannable-through, invisibility,
    ///   IGNORED_BY_MONSTERS)
    /// - Item-block gate (UNPASS/AVOID with per-damage-type hazard immunity)
    ///
    /// It does **not** include the `TShortway::FillMap` terrain checks (`BANK`, waypoint chain) —
    /// those are [`Self::monster_tshortway_fill_walkable`]'s responsibility. Used by
    /// [`Self::monster_can_occupy_chase_tile`] for single-step gates (dance/roam/flee/chase).
    pub(crate) fn monster_move_possible_planning_772(&self, cid: CreatureId, pos: Position) -> bool {
        let (
            spawn,
            cfg,
            home_radius,
            can_push_creatures,
            can_push_items,
            immunity_poison,
            immunity_fire,
            immunity_energy,
            see_invisible,
            state,
            chase_target,
            master,
        ) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                m.spawn_position,
                self.monster_world_config,
                m.home_radius,
                m.can_push_creatures,
                m.can_push_items,
                m.immunity_poison,
                m.immunity_fire,
                m.immunity_energy,
                m.see_invisible,
                m.state,
                m.base.attack_target.or(m.base.follow_target),
                m.base.master,
            ),
            _ => return false,
        };
        // C++ skips home/radius when `ATTACKING|PANIC` (`crnonpl.cc:2148-2159`); the roam bound uses
        // the per-home radius (Finding 17/17b).
        if state != MonsterState::Attacking && state != MonsterState::Panic {
            let radius = self.monster_roam_leash_radius(home_radius);
            if !is_in_spawn_range(pos, spawn, radius, cfg.despawn_z_range) {
                return false;
            }
        }
        let Some(tile) = self.map.get_tile(pos) else {
            return false;
        };
        // C++ `MovePossible` blocks house tiles (`crnonpl.cc:2168` `IsHouse(x,y,z)`).
        if matches!(tile, crate::tile::Tile::House(_)) {
            return false;
        }
        let body = tile.body();
        if (body.flags
            & (tilestate::PROTECTIONZONE
                | tilestate::FLOORCHANGE
                | tilestate::TELEPORT
                | tilestate::BLOCKSOLID))
            != 0
        {
            return false;
        }

        let chain = body.map_object_chain();
        for entry in &chain {
            match entry {
                MapStackEntry::Ground(_) => {}
                MapStackEntry::Creature(tile_c) => {
                    if *tile_c == cid {
                        // C++ `MovePossible(Execute=false)` — own tile keeps terrain wp (`crnonpl.cc:2191-2287`).
                        continue;
                    }
                    if state != MonsterState::Attacking && state != MonsterState::Panic {
                        return false;
                    }
                    if chase_target.is_none() {
                        return false;
                    }
                    if !can_push_creatures {
                        return false;
                    }
                    if Some(*tile_c) == chase_target {
                        return false;
                    }
                    if master == Some(*tile_c) {
                        return false;
                    }
                    let Some(other) = self.creatures.get(*tile_c) else {
                        return false;
                    };
                    // C++ `MovePossible` invisibility gate (`crnonpl.cc:2221-2223`): a blocker
                    // the mover can't see (no SeeInvisible + blocker invisible) is a hard block.
                    if !see_invisible && other.base().is_invisible() {
                        return false;
                    }
                    match other {
                        CreatureKind::Monster(m) => {
                            if !m.is_pushable() {
                                return false;
                            }
                            // C++ `MovePossible` has no summon gate — a summon with KickCreatures
                            // plans through pushable monsters like any other kicker (`crnonpl.cc:2202`).
                            // P1-A1: the old `!is_summon` gate is dropped.
                            continue;
                        }
                        CreatureKind::Player(p) if p.ghost_mode => continue,
                        // C++ `crnonpl.cc:2230`: a summon (Master != 0) treats a player tile as a
                        // hard block. `IGNORED_BY_MONSTERS` players are also hard blocks.
                        CreatureKind::Player(p) if master.is_some() => return false,
                        CreatureKind::Player(p)
                            if has_player_flag(
                                flags_for_group(&self.groups, p.group_id),
                                PLAYER_FLAG_IGNORED_BY_MONSTERS,
                            ) =>
                        {
                            return false
                        }
                        // C++ `crnonpl.cc:2229-2233`: a non-summon kicker facing a normal player
                        // falls past the creature (plannable-through); EXHAUSTED fires at Execute.
                        CreatureKind::Player(_) => continue,
                        CreatureKind::Npc(_) => return false,
                    }
                }
                MapStackEntry::Item(item_id) => {
                    let Some(item) = self.items.get(*item_id) else {
                        return false;
                    };
                    let server_id = item.item_type;
                    if self.items_db.is_unpass_772(server_id) {
                        if self.items_db.is_unmove_772(server_id) || !can_push_items {
                            return false;
                        }
                        continue;
                    }
                    if self.items_db.is_avoid_hazard_772(server_id) {
                        // C++ `MovePossible` AVOID branch (`crnonpl.cc:2264-2267`): per-damage-type
                        // immunity — PANIC ignores all hazards; NoPoison/NoBurning/NoEnergy ignore
                        // matching fields only. P1-B1: was poison-only, now per-type.
                        let ignore_hazard = state == MonsterState::Panic
                            || match self.items_db.avoid_damage_type_772(server_id) {
                                Some(tfs_rust_content::items::FieldDamageType::Poison) => {
                                    immunity_poison
                                }
                                Some(tfs_rust_content::items::FieldDamageType::Fire) => {
                                    immunity_fire
                                }
                                Some(tfs_rust_content::items::FieldDamageType::Energy) => {
                                    immunity_energy
                                }
                                None => false,
                            };
                        if !ignore_hazard
                            && (self.items_db.is_unmove_772(server_id) || !can_push_items)
                        {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    /// 772 `TShortway::FillMap` walkable gate — `cract.cc` `FillMap` + `MovePossible(Execute=false)`.
    ///
    /// Adds the TShortway-specific terrain checks (`BANK` ground, waypoint chain) on top of
    /// [`Self::monster_move_possible_planning_772`]. Used by the A*/TShortway pathfinder only.
    pub(crate) fn monster_tshortway_fill_walkable(
        &self,
        cid: CreatureId,
        pos: Position,
        _target: Position,
    ) -> bool {
        // `MovePossible` creature/item gate (PZ, house, leash, creatures, items).
        if !self.monster_move_possible_planning_772(cid, pos) {
            return false;
        }
        // TShortway `FillMap` terrain gate — `BANK` ground required for waypoint overlay.
        let Some(tile) = self.map.get_tile(pos) else {
            return false;
        };
        let body = tile.body();
        let chain = body.map_object_chain();
        let Some(MapStackEntry::Ground(head_id)) = chain.first() else {
            return false;
        };
        if !self.items_db.is_terrain_bank_772(*head_id) {
            return false;
        }
        true
    }

    /// Spawn leash + pathfinding tile check — shared by A* and step selection (`monster.cpp` `canWalkTo`).
    ///
    /// On 772 (`beat_driven_loop`), delegates to [`Self::monster_tshortway_fill_walkable`] which
    /// mirrors `MovePossible(Execute=false)` (`crnonpl.cc:2141–2293`): the 772 creature/item gate
    /// with per-damage-type hazard immunity, invisibility, house, and player-plannable-through
    /// semantics. On 1098, uses the TFS `Tile::queryAdd` model (`tile_query_add_monster`).
    fn monster_can_occupy_chase_tile(&self, cid: CreatureId, pos: Position) -> bool {
        if self.beat_driven_loop {
            // P1-A3: route single-step gates (dance/roam/flee/chase) through the 772 `MovePossible`
            // planning model, not the 1098 `tile_query_add_monster`. Uses
            // `monster_move_possible_planning_772` (no TShortway terrain checks).
            return self.monster_move_possible_planning_772(cid, pos);
        }
        let (spawn, home_radius, state) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (m.spawn_position, m.home_radius, m.state),
            _ => return false,
        };
        // C++ `MovePossible` applies the home-radius leash **only when not** ATTACKING/PANIC
        // (`crnonpl.cc:2148-2157`): a chasing monster follows its target out of range and despawns
        // later via the out-of-range check, rather than pinning at the radius edge (audit Finding 17).
        if state != MonsterState::Attacking && state != MonsterState::Panic {
            let cfg = self.monster_world_config;
            let radius = self.monster_roam_leash_radius(home_radius);
            if !is_in_spawn_range(pos, spawn, radius, cfg.despawn_z_range) {
                return false;
            }
        }
        let Some(tile) = self.map.get_tile(pos) else {
            return false;
        };
        tile_query_add_creature(self, tile, cid, PATHFIND_WALK_FLAGS)
            == crate::return_value::ReturnValue::NoError
    }

    /// Effective per-home roam leash radius (axis-box, non-attacking). 772 uses the monster's
    /// `home_radius` (CipSoft `MonsterhomeInRange`, `crnonpl.cc:2157`); 1098 or an unset home
    /// (`home_radius <= 0`) falls back to the global despawn radius (audit Finding 17b).
    fn monster_roam_leash_radius(&self, home_radius: i32) -> i32 {
        if self.beat_driven_loop && home_radius > 0 {
            home_radius
        } else {
            self.monster_world_config.despawn_radius
        }
    }

    fn monster_can_walk_to(&self, cid: CreatureId, from: Position, dir: Direction) -> bool {
        self.monster_can_occupy_chase_tile(cid, from.offset(dir))
    }
}


#[cfg(test)]
#[path = "monster_ai_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "monster_ai_world_tests.rs"]
mod world_tests;
