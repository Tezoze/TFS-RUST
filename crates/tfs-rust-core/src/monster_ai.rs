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

use tfs_rust_common::enums::Direction;
use tfs_rust_common::Position;
use rand::Rng;

use crate::chase_debug;
use crate::creature::{CreatureKind, MonsterAiPhase};
use crate::game_world::{creature_can_see, GameWorld};
use crate::ids::CreatureId;
use crate::monster_distance_step::{
    distance_x, distance_y, get_dance_step, get_distance_step, get_random_step, offset_x, offset_y,
    DistanceStepOutcome, search_flight_field,
};
use crate::pathfinding::{
    scan_min_terrain_waypoints, FindPathParams, CHASE_PATH_MAX_STEPS,
    REVERSE_PATH_VIEW_RADIUS, uses_reverse_terrain_path,
};
use crate::walk::{creature_turn_with_broadcast, PATHFIND_WALK_FLAGS, tile_query_add_creature};

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

pub(crate) fn manhattan(a: Position, b: Position) -> i32 {
    distance_x(a, b) + distance_y(a, b)
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
pub fn compute_look_toward_target(from: Position, target: Position, current: Direction) -> Direction {
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
    } else if ox < 0 && oy < 0 {
        match current {
            Direction::South | Direction::North => Direction::West,
            Direction::East => Direction::North,
            other => other,
        }
    } else if ox < 0 && oy > 0 {
        match current {
            Direction::North | Direction::South => Direction::West,
            Direction::East => Direction::South,
            other => other,
        }
    } else if ox > 0 && oy < 0 {
        match current {
            Direction::South | Direction::North => Direction::East,
            Direction::West => Direction::North,
            other => other,
        }
    } else if ox > 0 && oy > 0 {
        match current {
            Direction::North | Direction::South => Direction::East,
            Direction::West => Direction::South,
            other => other,
        }
    } else {
        current
    }
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
        wrong_distance || !self.map.is_sight_clear(expected_pos, target_pos)
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
        let at_goal = self.monster_at_follow_goal(
            cid,
            follow_id,
            pos,
            target_pos,
            fleeing,
            target_distance,
        );
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
        if self.creatures.get(cid).is_some_and(|k| matches!(k, CreatureKind::Monster(_))) {
            self.monster_on_follow_creature_complete(cid, follow_id);
        }
    }

    /// Used by [`crate::creature_think::GameWorld::creature_on_think`] to skip redundant repaths.
    pub(crate) fn monster_should_skip_follow_repath(&self, cid: CreatureId, follow_id: CreatureId) -> bool {
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

    /// C++ `Monster::onCreatureAppear` self branch — `monster.cpp` ~159–166.

    /// TFS `Monster::doAttacking` — `monster.cpp` ~806.
    pub fn monster_do_attacking(&mut self, cid: CreatureId, _interval_ms: u32) {
        self.monster_update_look_direction(cid);
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
            if !self.beat_driven_loop {
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
                        let attack = self
                            .creatures
                            .get(cid)
                            .and_then(|k| k.base().attack_target);
                        if let Some(target_id) = attack {
                            if !self.monster_can_use_attack(cid, pos, target_id) {
                                let _ = self.monster_search_target(cid, TargetSearchType::AttackRange);
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
    /// Called from [`crate::idle_stimulus::GameWorld::monster_idle_prepare_and_enqueue_go`], not from
    /// think-time repath. On path failure (non-flee) returns [`MonsterIdleChaseRepathOutcome::Noway`].
    pub(crate) fn monster_idle_chase_repath(
        &mut self,
        cid: CreatureId,
        repath_reason: Option<&str>,
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

        if chase_debug::chase_path_debug_enabled() {
            if let Some(k) = self.creatures.get(cid) {
                let branch = if fleeing {
                    "flee"
                } else if target_distance > 1 {
                    "dist_chase"
                } else {
                    "melee_chase"
                };
                chase_debug::log_branch(
                    self.tick_counter,
                    cid,
                    k.base().name.as_str(),
                    branch,
                    pos,
                    target_pos,
                    false,
                    CHASE_PATH_MAX_STEPS as i32,
                    repath_reason,
                );
            }
        }

        if self.monster_try_apply_chase_path(cid, target_pos, fleeing, target_distance, &fpp) {
            if self.creatures.get(cid).is_some_and(|k| matches!(k, CreatureKind::Monster(_))) {
                self.monster_on_follow_creature_complete(cid, follow_id);
            }
            return MonsterIdleChaseRepathOutcome::PathQueued;
        }

        if fleeing {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().has_follow_path = false;
            }
            if self.creatures.get(cid).is_some_and(|k| matches!(k, CreatureKind::Monster(_))) {
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

    /// TFS `Creature::goToFollowCreature` — `creature.cpp` ~1011 (1098 only).
    pub fn go_to_follow_creature(&mut self, cid: CreatureId, repath_reason: Option<&str>) {
        if self.beat_driven_loop {
            let _ = self.monster_idle_chase_repath(cid, repath_reason);
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
        let use_distance_step = !self.beat_driven_loop && !is_summon && (fleeing || target_distance > 1);

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
                    if self.creatures.get(cid).is_some_and(|k| matches!(k, CreatureKind::Monster(_))) {
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
                        if self.creatures.get(cid).is_some_and(|k| matches!(k, CreatureKind::Monster(_))) {
                            self.monster_on_follow_creature_complete(cid, follow_id);
                        }
                        return;
                    }
                    // keep-distance: fall through to A* when getDistanceStep fails.
                }
            }
        }

        if self.monster_try_apply_chase_path(cid, target_pos, fleeing, target_distance, &fpp) {
            if self.creatures.get(cid).is_some_and(|k| matches!(k, CreatureKind::Monster(_))) {
                self.monster_on_follow_creature_complete(cid, follow_id);
            }
            return;
        }
        if fleeing {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().has_follow_path = false;
            }
            if self.creatures.get(cid).is_some_and(|k| matches!(k, CreatureKind::Monster(_))) {
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

        if self.creatures.get(cid).is_some_and(|k| matches!(k, CreatureKind::Monster(_))) {
            self.monster_on_follow_creature_complete(cid, follow_id);
        }
    }

    /// Apply A* (primary + relaxed) or return false so caller can try one-tile fallbacks.
    fn monster_try_apply_chase_path(
        &mut self,
        cid: CreatureId,
        target_pos: Position,
        fleeing: bool,
        target_distance: i32,
        fpp: &FindPathParams,
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
            let Some(mut steps) = self.get_creature_path_to_with_fpp(cid, target_pos, try_fpp) else {
                continue;
            };
            if steps.is_empty() {
                let dist = chebyshev(pos, target_pos);
                if dist > 1.max(target_distance) {
                    continue;
                }
            } else if self.beat_driven_loop {
                // 772 `ToDoGo(..., false, 3)` — only the next few hops are queued per repath,
                // stopping early if distance to target becomes <= 1.
                steps.reverse();
                steps = crate::pathfinding::truncate_cipsoft_chase_queue(
                    pos,
                    target_pos,
                    steps,
                    CHASE_PATH_MAX_STEPS,
                    false,
                );
                steps.reverse();
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
                    let min_wp = scan_min_terrain_waypoints(
                        &self.map,
                        pos,
                        REVERSE_PATH_VIEW_RADIUS,
                        |p| {
                            self.map
                                .get_tile(p)
                                .filter(|_| self.map.is_walkable(p))
                                .map(|t| self.tile_ground_speed(t.body()))
                                .unwrap_or(0)
                        },
                    );
                    chase_debug::log_shortway(
                        self.tick_counter,
                        cid,
                        name.as_str(),
                        pos,
                        target_pos,
                        10,
                        min_wp,
                        false,
                        CHASE_PATH_MAX_STEPS as i32,
                        true,
                        &path_positions,
                    );
                }
            }
            if let Some(k) = self.creatures.get_mut(cid) {
                let base = k.base_mut();
                base.walk_queue.clear();
                for d in steps {
                    base.walk_queue.push_back(d);
                }
                base.has_follow_path = true;
            }
            // Let the active walk timer continue naturally rather than cancelling/restarting
            self.monster_start_chase_walk(cid, true);
            return true;
        }
        if self.beat_driven_loop && chase_debug::chase_path_debug_enabled() {
            if let Some(k) = self.creatures.get(cid) {
                let name = k.base().name.clone();
                chase_debug::log_shortway(
                    self.tick_counter,
                    cid,
                    name.as_str(),
                    pos,
                    target_pos,
                    10,
                    scan_min_terrain_waypoints(
                        &self.map,
                        pos,
                        REVERSE_PATH_VIEW_RADIUS,
                        |p| {
                            self.map
                                .get_tile(p)
                                .filter(|_| self.map.is_walkable(p))
                                .map(|t| self.tile_ground_speed(t.body()))
                                .unwrap_or(0)
                        },
                    ),
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
            &[Direction::North, Direction::East, Direction::South, Direction::West][..]
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
        if self.creatures.get(cid).is_some_and(|k| matches!(k, CreatureKind::Monster(_))) {
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
        let mut rng = rand::thread_rng();
        let can_walk = |dir: Direction| self.monster_can_walk_to(cid, pos, dir);
        match get_distance_step(pos, target_pos, 1, fleeing, sight, can_walk, &mut rng) {
            DistanceStepOutcome::Step(dir) => {
                self.monster_start_follow_step(cid, dir);
                if self.creatures.get(cid).is_some_and(|k| matches!(k, CreatureKind::Monster(_))) {
                    self.monster_on_follow_creature_complete(cid, follow_id);
                }
                true
            }
            _ => false,
        }
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
                fpp.full_path_search = target_pos
                    .is_some_and(|tp| chebyshev(pos, tp) != target_distance);
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
            fpp.full_path_search = target_pos
                .is_some_and(|tp| chebyshev(pos, tp) != target_distance);
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
        use crate::pathfinding::{get_path_matching, CREATURE_ON_TILE_PATH_COST};

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
        get_path_matching(
            &self.map,
            start,
            target,
            fpp,
            self.mechanics.profile.path_cost,
            self.mechanics.profile.path_search,
            self.mechanics.profile.path_forward_fallback,
            |pos| ctx.world.monster_can_occupy_chase_tile(ctx.cid, pos),
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
            // 772 `WAYPOINTS` — OTB `ITEM_ATTR_SPEED` on the tile ground item (`TShortway::FillMap`).
            |pos| {
                let Some(tile) = ctx.world.map.get_tile(pos) else {
                    return 0;
                };
                if !ctx.world.map.is_walkable(pos) {
                    return 0;
                }
                ctx.world.tile_ground_speed(tile.body())
            },
        )
    }


    /// Recompute chase path immediately — C++ `Creature::onCreatureMove` instant repath
    /// (`creature.cpp` ~619–637) and avoids waiting for `onThink` (1 s bucket).
    /// Walk execution stays in `creature_start_chase_auto_walk` / scheduler — do not call
    /// `check_creature_walk` here (would deepen the `onWalk` stack and risk recursion on blocked tiles).
    pub(crate) fn monster_follow_repath_now(&mut self, cid: CreatureId, repath_reason: Option<&str>) {
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
                if let Some(master_attack) = self.creatures.get(master_id).and_then(|k| k.base().attack_target) {
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

    /// TFS `Monster::onThinkTarget` — `monster.cpp` ~923.
    pub(crate) fn monster_on_think_target(&mut self, cid: CreatureId, interval_ms: u32) {
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
                )
            })
        });

        if !walking_to_spawn
            && follow.is_none()
            && (!is_summon || !master_in_range)
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
                        let can_walk = |dir: Direction| self.monster_can_walk_to(cid, pos, dir);
                        let mut rng = rand::thread_rng();
                        if fleeing {
                            return search_flight_field(pos, target_pos, can_walk, &mut rng);
                        }
                        if dist < target_distance {
                            return search_flight_field(pos, target_pos, can_walk, &mut rng);
                        } else if dist == target_distance {
                            let choice = rng.gen_range(0..5);
                            let dirs = [
                                Some(Direction::West),
                                Some(Direction::East),
                                Some(Direction::North),
                                Some(Direction::South),
                                None,
                            ];
                            if let Some(dir) = dirs[choice] {
                                let dest = pos.offset(dir);
                                if chebyshev(dest, target_pos) == target_distance && can_walk(dir) {
                                    if chase_debug::chase_path_debug_enabled() {
                                        if let Some(k) = self.creatures.get(cid) {
                                            let branch = if target_distance > 1 {
                                                "dist_dance"
                                            } else {
                                                "melee_dance"
                                            };
                                            chase_debug::log_branch(
                                                self.tick_counter,
                                                cid,
                                                k.base().name.as_str(),
                                                branch,
                                                pos,
                                                dest,
                                                true,
                                                i32::MAX,
                                                None,
                                            );
                                        }
                                    }
                                    return Some(dir);
                                }
                            }
                            return None; // stand still
                        }
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
                    let can_use_from = |from: Position| {
                        self.monster_can_use_attack(cid, from, target_id)
                    };
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

    /// Spawn leash + pathfinding tile check — shared by A* and step selection (`monster.cpp` `canWalkTo`).
    fn monster_can_occupy_chase_tile(&self, cid: CreatureId, pos: Position) -> bool {
        let (spawn, cfg) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (m.spawn_position, self.monster_world_config),
            _ => return false,
        };
        if !is_in_spawn_range(
            pos,
            spawn,
            cfg.despawn_radius,
            cfg.despawn_z_range,
        ) {
            return false;
        }
        let Some(tile) = self.map.get_tile(pos) else {
            return false;
        };
        tile_query_add_creature(self, tile, cid, PATHFIND_WALK_FLAGS)
            == crate::return_value::ReturnValue::NoError
    }

    fn monster_can_walk_to(&self, cid: CreatureId, from: Position, dir: Direction) -> bool {
        self.monster_can_occupy_chase_tile(cid, from.offset(dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_fleeing_gate() {
        assert!(!is_fleeing(10, 5, false));
        assert!(is_fleeing(5, 5, false));
        assert!(!is_fleeing(5, 5, true));
    }

    #[test]
    fn is_in_spawn_range_chebyshev_and_z() {
        let spawn = Position::new(100, 100, 7);
        assert!(is_in_spawn_range(
            Position::new(110, 110, 7),
            spawn,
            50,
            2
        ));
        assert!(!is_in_spawn_range(
            Position::new(200, 100, 7),
            spawn,
            50,
            2
        ));
        assert!(!is_in_spawn_range(
            Position::new(100, 100, 10),
            spawn,
            50,
            2
        ));
    }

    #[test]
    fn is_within_walk_to_spawn_range_axis_box() {
        let spawn = Position::new(100, 100, 7);
        assert!(is_within_walk_to_spawn_range(
            Position::new(110, 110, 7),
            spawn,
            15
        ));
        assert!(!is_within_walk_to_spawn_range(
            Position::new(120, 100, 7),
            spawn,
            15
        ));
        assert!(is_within_walk_to_spawn_range(
            Position::new(100, 100, 7),
            spawn,
            15
        ));
    }

    #[test]
    fn compute_look_faces_target() {
        let from = Position::new(10, 10, 7);
        assert_eq!(
            compute_look_toward_target(from, Position::new(12, 10, 7), Direction::North),
            Direction::East
        );
        assert_eq!(
            compute_look_toward_target(from, Position::new(10, 8, 7), Direction::East),
            Direction::North
        );
    }
}

#[cfg(test)]
mod world_tests {
    use std::time::{Duration, Instant};

    use tfs_rust_common::ConnId;
    use std::collections::VecDeque;

    use tfs_rust_common::enums::Direction;
    use tfs_rust_common::Position;

    use crate::creature::{CreatureKind, MonsterAiConfig};
    use crate::monster_ai::MonsterIdleChaseRepathOutcome;
    use crate::login_out::creature_wire_id;
    use crate::formulas::MechanicsProfile;
    use crate::pathfinding::uses_reverse_terrain_path;
    use crate::test_world::support::{
        beat_driven_world, ensure_walkable_tile, insert_monster_with_config, insert_player,
        insert_spectator_player, minimal_world, test_player,
    };

    #[test]
    fn monster_acquires_target_and_steps_toward_player() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        for x in 100..=106 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 100);
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        world.monster_on_creature_appear_self(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if !m.opponent_ids.is_empty())),
            "player should be registered as opponent"
        );
        assert!(
            world.creatures.get(monster).unwrap().base().follow_target.is_some(),
            "appear-self should select target without waiting for onThink"
        );

        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().next_walk_check = Some(Instant::now());
        }
        world.process_walk_deadlines();

        let new_pos = world.creatures.get(monster).unwrap().position();
        assert!(
            new_pos.x > mpos.x,
            "monster should step toward player (was {:?}, now {:?})",
            mpos,
            new_pos
        );
        assert!(
            world.creatures.get(monster).unwrap().base().follow_target.is_some(),
            "monster should acquire follow target"
        );
    }

    #[test]
    fn monster_repaths_when_follow_target_moves() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        let ppos_moved = Position::new(104, 100, 7);
        for x in 100..=106 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 100);
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        world.monster_on_creature_appear_self(monster);
        assert_eq!(
            world.creatures.get(monster).unwrap().base().follow_target,
            Some(player),
            "monster should be chasing player before target moves"
        );
        let queue_before = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .clone();
        assert!(
            !queue_before.is_empty(),
            "chasing monster should have a follow path queued"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .has_follow_path,
            "chasing monster should have has_follow_path set"
        );

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_moved;
        }
        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_moved, player);
        world.monster_dispatch_creature_move(player, ppos, ppos_moved);

        assert_eq!(
            world.creatures.get(monster).unwrap().base().follow_target,
            Some(player),
            "follow target should remain after target moves one tile"
        );
        let queue_after = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .clone();
        assert!(
            !queue_after.is_empty(),
            "monster should repath when follow target moves"
        );
        assert_ne!(
            queue_before, queue_after,
            "walk queue should be recomputed after target move"
        );
        assert!(
            queue_after.iter().all(|&d| d == Direction::East),
            "repath should still step east toward player at {:?}, got {:?}",
            ppos_moved,
            queue_after
        );
    }

    #[test]
    fn monster_follow_repath_without_path_is_profile_gated() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        let ppos_moved = Position::new(104, 100, 7);
        let ppos_moved_again = Position::new(103, 100, 7);
        for x in 100..=106 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 100);
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        // Chase state without has_follow_path — keep-distance band / idle before pathfinding.
        if let Some(k) = world.creatures.get_mut(monster) {
            let base = k.base_mut();
            base.follow_target = Some(player);
            base.has_follow_path = false;
            base.walk_queue = VecDeque::from([Direction::North]);
        }
        assert!(!world.mechanics.profile.follow_repath_without_path);

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_moved;
        }
        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_moved, player);
        world.monster_dispatch_creature_move(player, ppos, ppos_moved);

        assert_eq!(
            world.creatures.get(monster).unwrap().base().walk_queue,
            VecDeque::from([Direction::North]),
            "1098 profile should not repath when has_follow_path is false"
        );

        // 772 / 772 profile: repath even without has_follow_path.
        world.mechanics.profile.follow_repath_without_path = true;
        if let Some(k) = world.creatures.get_mut(monster) {
            let base = k.base_mut();
            base.walk_queue = VecDeque::from([Direction::North]);
        }
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_moved_again;
        }
        world.map.unregister_creature_at(ppos_moved, player);
        world.map.register_creature_at(ppos_moved_again, player);
        world.monster_dispatch_creature_move(player, ppos_moved, ppos_moved_again);

        let queue_beat_driven = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .clone();
        assert_ne!(
            queue_beat_driven,
            VecDeque::from([Direction::North]),
            "772 profile should repath when follow target moves without has_follow_path"
        );
        assert!(
            queue_beat_driven.iter().all(|&d| d == Direction::East),
            "repath should step east toward player at {:?}, got {:?}",
            ppos_moved_again,
            queue_beat_driven
        );
    }

    #[test]
    fn monster_acquires_target_when_player_walks_into_viewport() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let far = Position::new(112, 100, 7);
        let near = Position::new(111, 100, 7);
        for x in 100..=112 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 100);
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Wolf",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", far));
        world.map.register_creature_at(far, player);

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = near;
        }
        world.map.unregister_creature_at(far, player);
        world.map.register_creature_at(near, player);
        world.monster_dispatch_creature_move(player, far, near);

        assert!(
            world.creatures.get(monster).unwrap().base().follow_target == Some(player),
            "monster should target player as soon as they enter viewport"
        );
    }

    #[test]
    fn fleeing_monster_steps_away_from_player() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        for x in 99..=102 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 100);
        }

        let config = MonsterAiConfig {
            run_away_health: 50,
            ..Default::default()
        };
        let monster = insert_monster_with_config(&mut world, "Rat", mpos, 200, config);
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.health = 30;
            m.opponent_ids.push(player);
            m.is_idle = false;
        }
        let _ = world.monster_search_target(monster, super::TargetSearchType::Default);
        world.go_to_follow_creature(monster, None);
        world.process_walk_deadlines();

        let new_pos = world.creatures.get(monster).unwrap().position();
        assert!(
            new_pos.x < mpos.x,
            "fleeing monster should step away from player on the east"
        );
    }

    #[test]
    fn update_look_direction_broadcasts_turn() {
        let mut world = minimal_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 100);
        ensure_walkable_tile(&mut world.map, ppos, 100);

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let conn = ConnId(7);
        let player = insert_spectator_player(&mut world, conn, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.attack_target = Some(player);
            m.base.direction = Direction::North;
        }
        let wire_id = creature_wire_id(monster, world.creatures.get(monster).unwrap());
        world
            .creature_fully_sent_by_conn
            .entry(conn)
            .or_default()
            .insert(wire_id);

        world.monster_update_look_direction(monster);

        let pending = world.pending_outgoing.get(&conn).cloned().unwrap_or_default();
        assert!(
            pending.iter().any(|p| p.first() == Some(&0x6B)),
            "look-at-target should emit 0x6B turn packet"
        );
        assert_eq!(
            world.creatures.get(monster).unwrap().base().direction,
            Direction::East
        );
     }

    #[test]
    fn update_look_direction_ignored_when_walking() {
        let mut world = minimal_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 100);
        ensure_walkable_tile(&mut world.map, ppos, 100);

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );

        let conn = ConnId(7);
        let player = insert_spectator_player(&mut world, conn, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.attack_target = Some(player);
            m.base.direction = Direction::North;
            m.base.next_walk_check = Some(Instant::now() + std::time::Duration::from_millis(500));
        }

        world.monster_update_look_direction(monster);

        // Direction should remain North because walk is active (not idle)
        assert_eq!(
            world.creatures.get(monster).unwrap().base().direction,
            Direction::North
        );
    }

    #[test]
    fn select_target_automatically_updates_look_direction() {
        let mut world = minimal_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 100);
        ensure_walkable_tile(&mut world.map, ppos, 100);

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let conn = ConnId(7);
        let player = insert_spectator_player(&mut world, conn, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.direction = Direction::North;
            m.opponent_ids.push(player);
        }

        // monster_select_target should automatically set follow/attack target and update look direction to East (towards player).
        let selected = world.monster_select_target(monster, player);
        assert!(selected);

        let dir = world.creatures.get(monster).unwrap().base().direction;
        assert_eq!(dir, Direction::East);
    }

    #[test]
    fn monster_does_not_acquire_distant_player() {
        let mut world = minimal_world();
        let mpos = Position::new(100, 100, 8);
        let ppos = Position::new(130, 100, 8);
        ensure_walkable_tile(&mut world.map, mpos, 100);
        ensure_walkable_tile(&mut world.map, ppos, 100);

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        world.monster_update_target_list(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if m.opponent_ids.is_empty())),
            "player 30 tiles away must not enter opponent list"
        );
    }

    #[test]
    fn monster_prunes_opponent_when_player_leaves_can_see_range() {
        let mut world = minimal_world();
        let mpos = Position::new(100, 100, 8);
        let near = Position::new(105, 100, 8);
        let far = Position::new(130, 100, 8);
        for p in [mpos, near, far] {
            ensure_walkable_tile(&mut world.map, p, 100);
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", near));
        world.map.register_creature_at(near, player);
        world.monster_on_creature_appear_self(monster);
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if !m.opponent_ids.is_empty()))
        );

        // Player teleports out of monster viewport — C++ updateTargetList prunes via canSee.
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = far;
        }
        world.map.unregister_creature_at(near, player);
        world.map.register_creature_at(far, player);

        world.monster_update_target_list(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if m.opponent_ids.is_empty())),
            "opponent must be pruned when outside Creature::canSee range"
        );
    }

    #[test]
    fn monster_walks_back_when_target_lost() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let spawn = Position::new(100, 100, 7);
        let far = Position::new(120, 100, 7);
        for x in 100..=120 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 100);
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            far,
            200,
            MonsterAiConfig::default(),
        );
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.spawn_position = spawn;
            m.is_idle = false;
        }
        let player = insert_player(&mut world, test_player("Hero", Position::new(121, 100, 7)));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.opponent_ids.push(player);
        }

        world.monster_remove_creature_from_lists(monster, player);

        assert!(
            world.creatures.get(monster).is_some_and(|k| {
                matches!(k, CreatureKind::Monster(m) if m.walking_to_spawn && !m.base.walk_queue.is_empty())
            }),
            "monster outside walkToSpawnRadius should path toward spawn when last opponent leaves"
        );
    }

    #[test]
    fn idle_monster_does_not_random_walk() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            pos,
            200,
            MonsterAiConfig::default(),
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if m.is_idle))
        );

        let now = Instant::now();
        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().last_step = Some(now - Duration::from_secs(2));
            k.base_mut().next_walk_check = Some(now);
        }
        world.process_walk_deadlines();

        assert_eq!(world.creatures.get(monster).unwrap().position(), pos);
    }

    #[test]
    fn active_monster_random_roams_after_one_second() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new((100 + dx) as u16, (100 + dy) as u16, 7),
                    100,
                );
            }
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            pos,
            200,
            MonsterAiConfig::default(),
        );
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }

        let now = Instant::now();
        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().last_step = Some(now - Duration::from_secs(2));
        }
        let step = world.monster_next_walk_step(monster, now);
        assert!(
            step.is_some(),
            "active monster should pick a random roam direction after 1 s idle on tile"
        );
    }

    #[test]
    fn ranged_monster_steps_away_when_adjacent() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        for x in 99..=102 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 100);
        }

        let config = MonsterAiConfig {
            target_distance: 4,
            ..Default::default()
        };
        let monster = insert_monster_with_config(&mut world, "Rat", mpos, 200, config);
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = true;
        }

        world.go_to_follow_creature(monster, None);

        let stepped_away = world.creatures.get(monster).is_some_and(|k| {
            k.base().walk_queue.iter().any(|&d| d == Direction::West)
        });
        if stepped_away {
            return;
        }

        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().next_walk_check = Some(Instant::now());
        }
        world.process_walk_deadlines();

        let final_pos = world.creatures.get(monster).unwrap().position();
        assert!(
            final_pos.x < mpos.x,
            "ranged monster should step west away from adjacent player (was {:?}, now {:?})",
            mpos,
            final_pos
        );
    }

    #[test]
    fn ranged_keep_distance_clamps_to_melee_when_attack_unusable() {
        let mut world = minimal_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        for x in 100..=104 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 100);
        }

        // Rat has melee-only attack data in content, so at distance 4 it cannot use attack.
        let config = MonsterAiConfig {
            target_distance: 4,
            ..Default::default()
        };
        let monster = insert_monster_with_config(&mut world, "Rat", mpos, 200, config);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
        }

        let fpp = world.monster_path_search_params(monster, player, false, 4, false, false);
        assert_eq!(
            fpp.max_target_dist, 4,
            "TFS keeps maxTargetDist at XML targetDistance"
        );
        assert!(
            fpp.full_path_search,
            "TFS sets fullPathSearch when attack is unusable at range"
        );
    }

    // ---- B3 mechanics-profile knobs ----

    /// B3.2 — `DistanceKeep::PerType` keeps the monster's XML `targetDistance`; `Fixed(n)` overrides.
    #[test]
    fn effective_target_distance_follows_profile() {
        use tfs_rust_common::ProtocolVersion;
        let mut world = minimal_world();

        // 1098 default: per-type passes through unchanged.
        assert_eq!(world.monster_effective_target_distance(1), 1);
        assert_eq!(world.monster_effective_target_distance(7), 7);

        // 772 default: per-type from monster file (no era-wide override).
        world.mechanics = crate::formulas::Mechanics::for_version(ProtocolVersion::V772);
        assert_eq!(world.monster_effective_target_distance(1), 1);
        assert_eq!(world.monster_effective_target_distance(7), 7);
    }

    /// B3.1 — weakest-target metric: 772 compares current HP, 1098 compares max HP. Construct two
    /// players where the lowest-current and lowest-max are different creatures.
    #[test]
    fn weakest_opponent_metric_follows_profile() {
        use tfs_rust_common::ProtocolVersion;
        let mut world = minimal_world();

        // Player A: big max pool, badly wounded (low current).
        let a = insert_player(&mut world, test_player("Tank", Position::new(100, 100, 7)));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.base.max_health = 1000;
            p.base.health = 20;
        }
        // Player B: small max pool, full health (low max, higher current than A).
        let b = insert_player(&mut world, test_player("Squire", Position::new(101, 100, 7)));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(b) {
            p.base.max_health = 100;
            p.base.health = 100;
        }
        let candidates = [a, b];

        // 1098 (max HP): B is weakest (max 100 < 1000).
        assert_eq!(world.monster_weakest_opponent(&candidates), Some(b));

        // 772 (current HP): A is weakest (current 20 < 100).
        world.mechanics = crate::formulas::Mechanics::for_version(ProtocolVersion::V772);
        assert_eq!(world.monster_weakest_opponent(&candidates), Some(a));
    }

    /// P7 — 772 beat loop: `monster_arm_event_walk` must not re-arm when `next_wakeup` is set.
    #[test]
    fn beat_driven_arm_event_walk_skips_when_wakeup_set() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        world.server_ms = 0;

        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 2148);
        let cid = insert_monster_with_config(
            &mut world,
            "Rat",
            pos,
            200,
            MonsterAiConfig::default(),
        );

        world.monster_arm_event_walk(cid);
        let wakeup = world
            .creatures
            .get(cid)
            .unwrap()
            .base()
            .next_wakeup
            .expect("first arm should schedule ToDoQueue wakeup");
        assert_eq!(world.todo_queue.len(), 1);

        world.monster_arm_event_walk(cid);

        assert_eq!(
            world.creatures.get(cid).unwrap().base().next_wakeup,
            Some(wakeup),
            "second arm must not reschedule walk"
        );
        assert_eq!(world.todo_queue.len(), 1, "no duplicate heap entry");
    }

    #[test]
    fn test_772_melee_dance_only_cardinal() {
        let mut world = beat_driven_world();
        
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        
        // Make surrounding tiles walkable
        for dx in -1i32..=1i32 {
            for dy in -1i32..=1i32 {
                ensure_walkable_tile(&mut world.map, Position::new((100 + dx) as u16, (100 + dy) as u16, 7), 2148);
            }
        }
        
        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }
        
        // Sample several times to verify all chosen step directions are cardinal (or None)
        let now = std::time::Instant::now();
        for _ in 0..100 {
            if let Some(dir) = world.monster_next_walk_step(monster, now) {
                assert!(
                    matches!(
                        dir,
                        Direction::North | Direction::East | Direction::South | Direction::West
                    ),
                    "772 melee dance step must be cardinal, got {:?}",
                    dir
                );
            }
        }
    }

    #[test]
    fn test_772_walk_queue_hysteresis() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        let ppos_moved = Position::new(104, 100, 7);
        for x in 100..=106 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 100);
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        world.monster_on_creature_appear_self(monster);
        let queue_before = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .clone();
        assert!(!queue_before.is_empty());

        // Target moves 1 tile closer (105 -> 104)
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_moved;
        }
        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_moved, player);
        world.monster_dispatch_creature_move(player, ppos, ppos_moved);

        let queue_after = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .clone();
        
        // Hysteresis: Walk queue should NOT clear/recompute because target is still within goal range
        assert_eq!(
            queue_before, queue_after,
            "772 walk queue should be retained due to hysteresis when target moves slightly"
        );
    }

    #[test]
    fn test_772_ensure_follow_band_keeps_queue_when_not_stale() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        let ppos_moved = Position::new(104, 100, 7);
        for x in 100..=106 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 100);
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        world.monster_on_creature_appear_self(monster);
        let queue_before = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .clone();
        assert!(!queue_before.is_empty());

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_moved;
        }
        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_moved, player);

        let repathed = world.monster_ensure_follow_band(monster, "idle");
        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            !repathed,
            "772 ensure_follow_band must not schedule repath when queue still valid"
        );
        assert_eq!(base.walk_queue, queue_before);
        assert!(!base.force_update_follow_path);
    }

    #[test]
    fn test_772_target_move_empty_queue_defers_to_idle() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        let ppos_moved = Position::new(104, 100, 7);
        for x in 100..=106 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 100);
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.follow_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_moved;
        }
        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_moved, player);
        world.monster_dispatch_creature_move(player, ppos, ppos_moved);

        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            !base.force_update_follow_path,
            "772 empty queue must not force_update on every target tile"
        );
        assert!(base.walk_queue.is_empty());

        let (needs, reason) = world.monster_idle_chase_needs_repath(monster);
        assert!(needs, "idle should still repath via idle_drain or off_band");
        assert!(matches!(reason, Some("idle_drain") | Some("off_band")));
    }

    #[test]
    fn test_772_ensure_follow_band_defers_repath_via_force_update() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(108, 100, 7);
        for x in 100..=109 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 100);
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = true;
            m.base.walk_queue = VecDeque::from([Direction::North]);
        }

        let scheduled = world.monster_ensure_follow_band(monster, "idle");
        assert!(scheduled, "stale queue must schedule repath on 772");
        let base = world.creatures.get(monster).unwrap().base();
        assert!(base.force_update_follow_path);
        assert!(base.walk_queue.is_empty());
        assert!(!base.has_follow_path);

        world.monster_idle_stimulus(monster);
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| !k.base().walk_queue.is_empty() || k.base().todo.has_go()),
            "idle must repath after ensure_follow_band deferred force_update"
        );
    }

    #[test]
    fn test_772_path_prefers_cardinal_on_open_terrain() {
        let mut world = beat_driven_world();

        let mpos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        for x in 90..=110 {
            for y in 90..=110 {
                ensure_walkable_tile(&mut world.map, Position::new(x, y, 7), 150);
            }
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", mpos));
        world.map.register_creature_at(mpos, player);

        for dx in -5..=5 {
            for dy in -5..=5 {
                let target_pos = Position::new((100 + dx) as u16, (100 + dy) as u16, 7);
                if target_pos == mpos {
                    continue;
                }

                if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
                    p.base.position = target_pos;
                }

                let fpp = world.monster_path_search_params(
                    monster,
                    player,
                    false,
                    1,
                    false,
                    false,
                );

                if let Some(steps) = world.get_creature_path_to_with_fpp(monster, target_pos, &fpp) {
                    for step in steps {
                        assert!(
                            matches!(
                                step,
                                Direction::North | Direction::East | Direction::South | Direction::West
                            ),
                            "3× diagonal cost should prefer cardinals on open uniform terrain; \
                             path to ({}, {}) used {:?}",
                            100 + dx,
                            100 + dy,
                            step,
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_772_allow_diagonal_true_stays_reverse_path_stack() {
        use tfs_rust_common::ProtocolVersion;

        let profile = MechanicsProfile::for_version(ProtocolVersion::V772);
        assert!(uses_reverse_terrain_path(profile.path_cost, profile.path_search));
        assert!(!profile.path_forward_fallback);

        let mut world = beat_driven_world();
        assert_eq!(world.mechanics.profile.path_forward_fallback, profile.path_forward_fallback);

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 105, 7);
        for x in 95..=110u16 {
            for y in 95..=110u16 {
                ensure_walkable_tile(&mut world.map, Position::new(x, y, 7), 102);
            }
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        let fpp = world.monster_path_search_params(monster, player, false, 1, false, false);
        assert!(
            fpp.allow_diagonal,
            "772 allows diagonal neighbors in 772 expansion"
        );

        let path = world
            .get_creature_path_to_with_fpp(monster, ppos, &fpp)
            .expect("reverse TShortway path");
        assert!(!path.is_empty());
        for step in &path {
            assert!(
                matches!(
                    step,
                    Direction::North | Direction::East | Direction::South | Direction::West
                ),
                "allow_diagonal=true on 772 must still use terrain×3 (cardinals on open grass), not TFS 10/25 bias: {step:?} in {path:?}"
            );
        }
    }

    #[test]
    fn test_772_diagonal_detour_when_cardinals_blocked() {
        use crate::tile::{flags as tilestate, Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;

        let mut world = beat_driven_world();

        let mpos = Position::new(10, 10, 7);
        let ppos = Position::new(12, 12, 7);
        for x in 8..=14u16 {
            for y in 8..=14u16 {
                ensure_walkable_tile(&mut world.map, Position::new(x, y, 7), 150);
            }
        }
        // Block all four cardinals around the monster — only diagonal exits remain.
        for (x, y) in [(10, 9), (10, 11), (9, 10), (11, 10)] {
            world.map.insert_tile(
                Position::new(x, y, 7),
                Tile::Normal(TileBody {
                    ground: Some(150),
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        let fpp = world.monster_path_search_params(monster, player, false, 1, false, false);
        let path = world
            .get_creature_path_to_with_fpp(monster, ppos, &fpp)
            .expect("reverse TShortway must diagonal out when cardinals are blocked");
        assert!(
            path.iter().any(|d| {
                matches!(
                    d,
                    Direction::NorthEast
                        | Direction::NorthWest
                        | Direction::SouthEast
                        | Direction::SouthWest
                )
            }),
            "path must use a diagonal to leave the cardinal trap: {path:?}"
        );
    }

    #[test]
    fn test_772_dance_retry_cadence() {
        let mut world = beat_driven_world();
        
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);
        
        // Do NOT make surrounding tiles walkable to guarantee blocked movement
        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            // Stand still recently so walk_delay is 0
            m.base.last_step_server_ms = None;
        }
        
        // Run walk. Since all directions are blocked, it must stand still / getNextStep false.
        let now = std::time::Instant::now();
        world.on_walk(monster, false, now, None);
        
        let wakeup = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .next_wakeup;
            
        // The wakeup should be scheduled at server_ms + step_duration (approx 350 ms for speed 200, ground 150)
        // rather than immediate retry (+1 ms).
        assert!(wakeup.is_some());
        let delay = wakeup.unwrap() - world.server_ms;
        assert!(delay >= 300, "expected dance retry delay to be at least step duration, got {}", delay);
    }

    #[test]
    fn test_772_repath_first_step_delay() {
        let mut world = beat_driven_world();
        
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);
        for x in 100..=105 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }
        
        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            
            // Set last step to now so there is a positive walk delay
            m.base.last_step_server_ms = Some(world.server_ms);
            m.base.last_step_ground_speed = 150;
            m.base.last_step_cost = 1;
        }
        
        // Trigger repath
        world.monster_follow_repath_now(monster, None);
        
        let wakeup = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .next_wakeup;
            
        // Wakeup should be scheduled for server_ms + walk_delay (approx 350 ms)
        assert!(wakeup.is_some());
        let delay = wakeup.unwrap() - world.server_ms;
        assert!(delay >= 300, "expected repath delay to respect walk delay, got {}", delay);
    }

    #[test]
    fn test_772_flee_steps_away() {
        let mut world = beat_driven_world();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7); // player to the east
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);
        // Make the west tile walkable so monster can flee west
        let west_pos = Position::new(99, 100, 7);
        ensure_walkable_tile(&mut world.map, west_pos, 150);

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            // force fleeing state by setting high run_away_health or modifying HP
            m.base.health = 10;
            m.run_away_health = 20; // health <= run_away_health -> fleeing
        }

        let now = std::time::Instant::now();
        let step = world.monster_next_walk_step(monster, now);
        // Flee step must be West (away from player who is East)
        assert_eq!(step, Some(Direction::West));
    }

    #[test]
    fn test_772_hunter_band_dance_cardinal() {
        let mut world = beat_driven_world();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7); // player is 4 tiles East
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);

        // Make all cardinal directions from 100,100 walkable
        ensure_walkable_tile(&mut world.map, Position::new(99, 100, 7), 150); // West (dist becomes 5)
        ensure_walkable_tile(&mut world.map, Position::new(101, 100, 7), 150); // East (dist becomes 3)
        ensure_walkable_tile(&mut world.map, Position::new(100, 99, 7), 150); // North (dist becomes 4)
        ensure_walkable_tile(&mut world.map, Position::new(100, 101, 7), 150); // South (dist becomes 4)

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            // Set target distance to 4 (keep distance)
            m.target_distance = 4;
        }

        // Run dance check multiple times. Since we are at distance 4, the only allowed step directions
        // that maintain distance 4 are North, South, or None. West (dist=5) and East (dist=3) must not be chosen.
        let now = std::time::Instant::now();
        for _ in 0..50 {
            if let Some(dir) = world.monster_next_walk_step(monster, now) {
                assert!(
                    matches!(dir, Direction::North | Direction::South),
                    "only North or South maintain target distance 4 from East-aligned target, got {:?}",
                    dir
                );
            }
        }
    }

    #[test]
    fn test_772_blocked_flee_stops() {
        let mut world = beat_driven_world();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7); // player to the east
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);

        // All neighbor tiles are blocked (non-walkable)
        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.health = 10;
            m.run_away_health = 20; // fleeing
        }

        // Run go_to_follow_creature. Since fleeing is true and pathing fails, it should clear follow path and stop,
        // without attempting any closer steps.
        world.go_to_follow_creature(monster, None);

        let walk_queue_empty = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .is_empty();
        assert!(walk_queue_empty, "blocked flee must not populate walk queue");
    }

    #[test]
    fn test_772_at_follow_goal_keep_distance_without_spell_range() {
        let mut world = beat_driven_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);

        let config = MonsterAiConfig {
            target_distance: 4,
            is_hostile: true,
            ..Default::default()
        };
        let monster = insert_monster_with_config(&mut world, "Rat", mpos, 200, config);
        let player = insert_player(&mut world, test_player("Hero", ppos));

        assert!(
            !world.monster_can_use_attack(monster, mpos, player),
            "test assumes no in-range attack spells for Rat at dist 4"
        );
        assert!(
            world.monster_at_follow_goal(monster, player, mpos, ppos, false, 4),
            "772 keep-distance at cheb==target_distance is at goal without canUseAttack"
        );
    }

    #[test]
    fn test_772_full_path_search_off_band() {
        let mut world = beat_driven_world();
        let mpos = Position::new(100, 100, 7);
        let at_band = Position::new(104, 100, 7);
        let off_band = Position::new(106, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, at_band, 150);
        ensure_walkable_tile(&mut world.map, off_band, 150);

        let config = MonsterAiConfig {
            target_distance: 4,
            ..Default::default()
        };
        let monster = insert_monster_with_config(&mut world, "Rat", mpos, 200, config);
        let player_at = insert_player(&mut world, test_player("HeroAt", at_band));
        let player_off = insert_player(&mut world, test_player("HeroOff", off_band));

        let fpp_at = world.monster_path_search_params(monster, player_at, false, 4, false, false);
        assert!(
            !fpp_at.full_path_search,
            "at keep band cheb==4 must use directional search box"
        );

        let fpp_off = world.monster_path_search_params(monster, player_off, false, 4, false, false);
        assert!(
            fpp_off.full_path_search,
            "off keep band cheb>4 must use full search box"
        );
    }

    #[test]
    fn test_772_idle_no_greedy_step_on_path_fail() {
        let mut world = beat_driven_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }

        let outcome = world.monster_idle_chase_repath(monster, Some("idle_drain"));
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::Noway);
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .is_empty(),
            "772 path fail must not fall back to greedy closer step"
        );
    }
}
