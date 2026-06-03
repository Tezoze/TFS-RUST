//! Monster native AI — TFS `Monster::onThink`, target list, chase/flee/return.
//!
//! - `Monster::onThink` — `monster.cpp` (~732).
//! - `Monster::searchTarget` — `monster.cpp` (~517).
//! - `Creature::goToFollowCreature` — `creature.cpp` (~1011).
//! - `Monster::walkToSpawn` — `monster.cpp` (~1087).
//! - `Monster::updateLookDirection` — `monster.cpp` (~1967).
//! - `Monster::doAttacking` — `monster.cpp` (~806).

use tfs_rust_common::enums::{Direction, ZoneType};
use tfs_rust_common::Position;
use tfs_rust_content::monsters::MonsterSpellNode;
use rand::Rng;
use slotmap::Key;

use crate::creature::{CreatureKind, MonsterAiPhase};
use crate::game_world::{creature_can_see, GameWorld};
use crate::ids::CreatureId;
use crate::monster_distance_step::{get_dance_step, get_distance_step, get_random_step, DistanceStepOutcome};
use crate::pathfinding::FindPathParams;
use crate::player_flags::{flags_for_group, has_player_flag, PLAYER_FLAG_IGNORED_BY_MONSTERS};
use crate::walk::{creature_turn_with_broadcast, PATHFIND_WALK_FLAGS, tile_query_add_creature};

/// C++ `Map::maxViewportX` (`map.h`).
const MAP_MAX_VIEWPORT: u16 = 11;

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

/// TFS `TargetSearchType_t` (`monster.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetSearchType {
    Default,
    Nearest,
    AttackRange,
    Random,
    /// Lowest-health opponent (CipSoft `Strategy` weakest bucket / TFS `<targetstrategy>` health).
    /// The HP metric (current vs max) is profile-driven (B3.1, `WeakestTargetMetric`).
    HealthLow,
}

/// TFS `Position::getDistanceX/Y` — absolute axis delta.
fn distance_x(a: Position, b: Position) -> i32 {
    (a.x as i32 - b.x as i32).unsigned_abs() as i32
}

fn distance_y(a: Position, b: Position) -> i32 {
    (a.y as i32 - b.y as i32).unsigned_abs() as i32
}

fn chebyshev(a: Position, b: Position) -> i32 {
    distance_x(a, b).max(distance_y(a, b))
}

fn manhattan(a: Position, b: Position) -> i32 {
    distance_x(a, b) + distance_y(a, b)
}

fn offset_x(from: Position, to: Position) -> i32 {
    to.x as i32 - from.x as i32
}

fn offset_y(from: Position, to: Position) -> i32 {
    to.y as i32 - from.y as i32
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
/// C++ `getOffsetX(attackedCreaturePos, pos)` = target.x − monster.x.
pub fn compute_look_toward_target(from: Position, target: Position, current: Direction) -> Direction {
    let ox = offset_x(from, target);
    let oy = offset_y(from, target);
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
            Direction::South => Direction::West,
            Direction::East => Direction::North,
            other => other,
        }
    } else if ox < 0 && oy > 0 {
        match current {
            Direction::North => Direction::West,
            Direction::East => Direction::South,
            other => other,
        }
    } else if ox > 0 && oy < 0 {
        match current {
            Direction::South => Direction::East,
            Direction::West => Direction::North,
            other => other,
        }
    } else if ox > 0 && oy > 0 {
        match current {
            Direction::North => Direction::East,
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

    /// Central guard: `has_follow_path` vs actual follow band. Repaths immediately when off-band
    /// and the walk queue is idle (no deferred `force_update_follow_path` flag).
    ///
    /// Returns true when [`Self::monster_follow_repath_now`] was invoked.
    fn monster_ensure_follow_band(&mut self, cid: CreatureId, reason: &str) -> bool {
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
        self.monster_follow_repath_now(cid);
        true
    }

    /// True when the monster is already in the desired follow/attack band (C++ empty `listWalkDir` at goal).
    fn monster_at_follow_goal(
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
    /// (current HP for CipSoft 7.72, max HP for TFS). Ties keep the first candidate.
    pub(crate) fn monster_weakest_opponent(&self, candidates: &[CreatureId]) -> Option<CreatureId> {
        let metric = self.mechanics.profile.weakest_target_metric;
        let mut best: Option<(CreatureId, i32)> = None;
        for &oid in candidates {
            let Some(k) = self.creatures.get(oid) else {
                continue;
            };
            let base = k.base();
            let hp = match metric {
                crate::formulas::WeakestTargetMetric::CurrentHp => base.health,
                crate::formulas::WeakestTargetMetric::MaxHp => base.max_health,
            };
            if best.map(|(_, b)| hp < b).unwrap_or(true) {
                best = Some((oid, hp));
            }
        }
        best.map(|(id, _)| id)
    }

    /// C++ `Monster::onCreatureAppear` self branch — `monster.cpp` ~159–166.
    pub fn monster_on_creature_appear_self(&mut self, cid: CreatureId) {
        self.monster_update_target_list(cid);
        self.monster_update_idle_status(cid);
        self.monster_try_acquire_chase_target(cid, None);
    }

    /// TFS `Monster::doAttacking` — `monster.cpp` ~806.
    ///
    /// Stub: wired from [`GameWorld::creature_on_attacking`] each think tick; spell cast +
    /// melee cadence (`attackSpells`, `canUseSpell`, `castSpell`) to be implemented.
    pub fn monster_do_attacking(&mut self, _cid: CreatureId, _interval_ms: u32) {}

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

        let (is_idle, is_summon, has_opponents, follow, has_path, fleeing) = {
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

    /// TFS `Creature::goToFollowCreature` — `creature.cpp` ~1011.
    pub fn go_to_follow_creature(&mut self, cid: CreatureId) {
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
        let use_distance_step = !is_summon && (fleeing || target_distance > 1);

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
        for (path_kind, try_fpp) in [("primary", fpp), ("relaxed", &relaxed)] {
            let Some(steps) = self.get_creature_path_to_with_fpp(cid, target_pos, try_fpp) else {
                continue;
            };
            if steps.is_empty() {
                let dist = chebyshev(pos, target_pos);
                if dist > 1.max(target_distance) {
                    continue;
                }
            } else {
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
            self.creature_start_chase_auto_walk(cid);
            return true;
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
        for dir in CHASE_STEP_DIRECTIONS {
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

    fn monster_start_follow_step(&mut self, cid: CreatureId, dir: Direction) {
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            base.walk_queue.clear();
            base.walk_queue.push_back(dir);
            base.has_follow_path = true;
        }
        // Let the active walk timer continue naturally rather than cancelling/restarting
        self.creature_start_chase_auto_walk(cid);
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
        let mut fpp = FindPathParams {
            min_target_dist: 1,
            max_target_dist: target_distance,
            clear_sight: true,
            allow_diagonal: true,
            full_path_search: !has_follow_path,
            max_search_dist: 12,
        };

        if is_summon {
            let master = self.creatures.get(cid).and_then(|k| k.base().master);
            if master == Some(follow_id) {
                fpp.max_target_dist = 2;
                fpp.full_path_search = true;
            } else if target_distance <= 1 {
                fpp.full_path_search = true;
            } else {
                fpp.full_path_search = !self.monster_can_use_attack(cid, pos, follow_id);
            }
        } else if fleeing {
            fpp.max_target_dist = i32::from(MAP_MAX_VIEWPORT);
            fpp.clear_sight = false;
            fpp.full_path_search = false;
        } else if target_distance <= 1 {
            fpp.full_path_search = true;
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
        get_path_matching(
            &self.map,
            start,
            target,
            fpp,
            self.mechanics.profile.path_cost,
            |pos| ctx.world.monster_can_occupy_chase_tile(ctx.cid, pos),
            |pos| {
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
            // CipSoft terrain weight: ground "waypoints" (= TFS ground speed; higher = slower tile).
            |pos| {
                ctx.world
                    .map
                    .get_tile(pos)
                    .map(|t| ctx.world.tile_ground_speed(t.body()))
                    .unwrap_or(150)
            },
        )
    }

    /// TFS `Map::getSpectators` multifloor Z span — `map.cpp` ~444–462.
    fn spectator_z_range(center_z: u8, multifloor: bool) -> std::ops::RangeInclusive<u8> {
        if !multifloor {
            return center_z..=center_z;
        }
        if center_z > 7 {
            let min_z = center_z.saturating_sub(2);
            let max_z = (center_z + 2).min(15);
            return min_z..=max_z;
        }
        if center_z == 6 {
            return 0..=8;
        }
        if center_z == 7 {
            return 0..=9;
        }
        0..=7
    }

    /// C++ `Map::getSpectators` — spatial viewport box only (`map.cpp` ~386–474).
    /// Used for move/appear fan-out; per-creature `canSee` is checked in `Monster::onCreatureMove`.
    fn collect_spatial_spectators(&self, center: Position, multifloor: bool) -> Vec<CreatureId> {
        let mut out = Vec::new();
        for z in Self::spectator_z_range(center.z, multifloor) {
            self.map.grid.collect_spectators(
                center.x,
                center.y,
                z,
                MAP_MAX_VIEWPORT,
                MAP_MAX_VIEWPORT,
                &mut out,
            );
        }
        out.sort_by_key(|id| id.data().as_ffi());
        out.dedup();
        out
    }

    /// Creatures within `Creature::canSee` of `center` (monster `updateTargetList` / spawn scan).
    fn collect_creature_spectators(&mut self, center: Position, multifloor: bool) -> Vec<CreatureId> {
        let range = i32::from(MAP_MAX_VIEWPORT);
        self.collect_spatial_spectators(center, multifloor)
            .into_iter()
            .filter(|&other| {
                let Some(other_pos) = self.creatures.get(other).map(|k| k.position()) else {
                    return false;
                };
                creature_can_see(center, other_pos, range, range)
            })
            .collect()
    }

    /// TFS `Monster::updateTargetList` — `monster.cpp` ~366.
    pub fn monster_update_target_list(&mut self, cid: CreatureId) {
        let pos = match self.creatures.get(cid) {
            Some(k) => k.position(),
            None => return,
        };

        self.monster_prune_creature_lists(cid);

        let spectators = self.collect_creature_spectators(pos, true);
        for other in spectators {
            if other == cid {
                continue;
            }
            self.monster_on_creature_found(cid, other, false);
        }
    }

    /// Monsters that should receive `Monster::onCreatureMove` for a move (`map.cpp` ~264–323).
    fn monsters_witnessing_move(&mut self, old_pos: Position, new_pos: Position) -> Vec<CreatureId> {
        let mut ids: Vec<CreatureId> = self
            .collect_spatial_spectators(old_pos, true)
            .into_iter()
            .chain(self.collect_spatial_spectators(new_pos, true))
            .filter(|&id| {
                self.creatures
                    .get(id)
                    .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
            })
            .collect();
        ids.sort_by_key(|id| id.data().as_ffi());
        ids.dedup();
        ids
    }

    /// TFS `Monster::onCreatureMove` — `monster.cpp` ~212.
    pub fn monster_on_creature_move(
        &mut self,
        monster_id: CreatureId,
        creature_id: CreatureId,
        old_pos: Position,
        new_pos: Position,
    ) {
        if !self.creatures.contains_key(monster_id) {
            return;
        }

        if creature_id == monster_id {
            self.monster_update_target_list(monster_id);
            self.monster_update_idle_status(monster_id);
            return;
        }

        let monster_pos = match self.creatures.get(monster_id) {
            Some(k) => k.position(),
            None => return,
        };
        let range = i32::from(MAP_MAX_VIEWPORT);
        let can_see_new = creature_can_see(monster_pos, new_pos, range, range);
        let can_see_old = creature_can_see(monster_pos, old_pos, range, range);

        if can_see_new && !can_see_old {
            self.monster_on_creature_found(monster_id, creature_id, true);
        } else if !can_see_new && can_see_old {
            self.monster_remove_creature_from_lists(monster_id, creature_id);
        }

        self.monster_update_idle_status(monster_id);

        let (is_summon, follow, has_path) = match self.creatures.get(monster_id) {
            Some(CreatureKind::Monster(m)) => (
                m.base.is_summon(),
                m.base.follow_target,
                m.base.has_follow_path,
            ),
            _ => return,
        };

        if follow == Some(creature_id) {
            self.monster_on_follow_creature_moved(monster_id, has_path);
            let target_visible = self
                .creatures
                .get(creature_id)
                .map(|k| creature_can_see(monster_pos, k.position(), range, range))
                .unwrap_or(false);
            if new_pos.z != old_pos.z || !target_visible {
                if let Some(k) = self.creatures.get_mut(monster_id) {
                    if k.base().follow_target == Some(creature_id) {
                        k.base_mut().clear_follow_for_target(creature_id);
                    }
                    if k.base().attack_target == Some(creature_id) {
                        k.base_mut().clear_attack_for_target(creature_id);
                    }
                }
            }
            return;
        }

        // TFS `Monster::onCreatureMove` — `monster.cpp` ~287–289: `selectTarget(creature)` only
        // when we have no follow; no `searchTarget` on every move. Requires line-of-sight to the
        // mover's new tile (spatial spectators alone can include off-screen monsters).
        if !is_summon
            && can_see_new
            && self.monster_is_opponent(monster_id, creature_id)
            && follow.is_none()
        {
            self.monster_ensure_opponent_listed(monster_id, creature_id);
            let selected = self.monster_select_target(monster_id, creature_id);
        }
    }

    /// TFS `Creature::onCreatureMove` follow-target branch — `creature.cpp` ~619–637.
    ///
    /// CipSoft does not gate this on a path flag: `IdleStimulus` enqueues fresh `ToDoGo` when the
    /// target moves (`crnonpl.cc` via `SearchFlightField`). TFS uses `hasFollowPath`; repath when
    /// still chasing (see `PROTOCOL_VERSIONING.md` §12.1).
    fn monster_on_follow_creature_moved(&mut self, monster_id: CreatureId, has_path: bool) {
        if !self.creatures.get(monster_id).is_some_and(|k| k.base().follow_target.is_some()) {
            return;
        }
        let is_772 = matches!(self.codec, tfs_rust_net::codec::Codec::V772(_));
        if !has_path {
            if !is_772 {
                return;
            }
        }
        if self
            .creatures
            .get(monster_id)
            .is_some_and(|k| k.base().is_updating_path)
        {
            return;
        }
        // Do not skip repath when the follow *target moved*: `getPathTo` can return an empty
        // list while still inside the monster's keep-distance band, which left monsters frozen
        // after the player kited away from melee.
        if let Some(k) = self.creatures.get_mut(monster_id) {
            let base = k.base_mut();
            base.walk_queue.clear();
            base.walk_update_ticks = 0;
            base.is_updating_path = false;
            base.force_update_follow_path = false;
        }
        self.monster_follow_repath_now(monster_id);
    }

    fn monster_remove_creature_from_lists(&mut self, monster_id: CreatureId, creature_id: CreatureId) {
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) {
            m.opponent_ids.retain(|&id| id != creature_id);
            m.friend_ids.retain(|&id| id != creature_id);
        }
        self.monster_update_idle_status(monster_id);
        self.monster_maybe_walk_to_spawn(monster_id);
    }

    fn monster_prune_creature_lists(&mut self, cid: CreatureId) {
        let pos = match self.creatures.get(cid) {
            Some(k) => k.position(),
            None => return,
        };
        let (mut opponents, mut friends) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (m.opponent_ids.clone(), m.friend_ids.clone()),
            _ => return,
        };

        opponents.retain(|&oid| self.monster_creature_visible_to(cid, pos, oid));
        friends.retain(|&fid| self.monster_creature_visible_to(cid, pos, fid));

        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.opponent_ids = opponents;
            m.friend_ids = friends;
        }
    }

    fn monster_creature_visible_to(&self, viewer: CreatureId, viewer_pos: Position, other: CreatureId) -> bool {
        let Some(other_kind) = self.creatures.get(other) else {
            return false;
        };
        if other_kind.base().health <= 0 {
            return false;
        }
        if !self.can_see_creature(viewer, other) {
            return false;
        }
        let op = other_kind.position();
        creature_can_see(
            viewer_pos,
            op,
            i32::from(MAP_MAX_VIEWPORT),
            i32::from(MAP_MAX_VIEWPORT),
        )
    }

    /// TFS `Monster::onCreatureFound` — `monster.cpp` ~414.
    fn monster_on_creature_found(&mut self, monster_id: CreatureId, creature_id: CreatureId, push_front: bool) {
        if creature_id == monster_id {
            return;
        }
        let pos = match self.creatures.get(monster_id) {
            Some(k) => k.position(),
            None => return,
        };
        let creature_pos = match self.creatures.get(creature_id) {
            Some(k) => k.position(),
            None => return,
        };
        if !self.can_see_creature(monster_id, creature_id) {
            return;
        }
        if !creature_can_see(
            pos,
            creature_pos,
            i32::from(MAP_MAX_VIEWPORT),
            i32::from(MAP_MAX_VIEWPORT),
        ) {
            return;
        }

        if self.monster_is_friend(monster_id, creature_id) {
            self.monster_add_friend(monster_id, creature_id);
        }
        if self.monster_is_opponent(monster_id, creature_id) {
            let had_follow = self
                .creatures
                .get(monster_id)
                .and_then(|k| k.base().follow_target)
                .is_some();
            self.monster_add_opponent(monster_id, creature_id, push_front);
        }
        self.monster_update_idle_status(monster_id);
        // Already-active monsters (not via `set_idle` wake) still need chase scheduling.
        if push_front {
            self.monster_schedule_chase_after_opponent_add(monster_id, Some(creature_id));
        }
        // C++ `Monster::onCreatureFound` stops here (`monster.cpp` ~414) — no `searchTarget` /
        // `setFollowCreature` on enter. Chase is acquired from `onThink` / move handlers only;
        // synchronous acquire on login fan-out ran A* for every viewport monster (~4s Forgotten).
    }

    /// Chase scheduling after a new opponent enters the list (viewport / move-enter).
    fn monster_schedule_chase_after_opponent_add(
        &mut self,
        monster_id: CreatureId,
        preferred: Option<CreatureId>,
    ) {
        let should = self.creatures.get(monster_id).is_some_and(|k| {
            matches!(k, CreatureKind::Monster(m) if {
                !m.base.is_summon()
                    && m.base.follow_target.is_none()
                    && !m.opponent_ids.is_empty()
            })
        });
        if !should {
            return;
        }
        if self.monster_viewport_notify_depth > 0 {
            let bucket = self.check_creature_bucket_index as u8;
            if self
                .creatures
                .get(monster_id)
                .is_some_and(|k| k.base().think_check_bucket.is_none())
            {
                self.add_creature_think_check(monster_id);
            }
            if let Some(k) = self.creatures.get_mut(monster_id) {
                k.base_mut().think_check_bucket = Some(bucket);
            }
        } else {
            self.monster_try_acquire_chase_target(monster_id, preferred);
        }
    }

    /// Acquire `follow_target` immediately when a player/opponent enters range — do not wait for `onThink`.
    fn monster_try_acquire_chase_target(
        &mut self,
        monster_id: CreatureId,
        preferred: Option<CreatureId>,
    ) {
        if self.creatures.get(monster_id).is_some_and(|k| {
            matches!(k, CreatureKind::Monster(m) if m.base.is_summon()) || k.base().follow_target.is_some()
        }) {
            return;
        }
        if let Some(pid) = preferred {
            if self.monster_is_opponent(monster_id, pid) {
                self.monster_ensure_opponent_listed(monster_id, pid);
            }
            if self.monster_select_target(monster_id, pid) {
                return;
            }
        }
        let ok = self.monster_search_target(monster_id, TargetSearchType::Default);
    }

    fn monster_is_friend(&self, monster_id: CreatureId, creature_id: CreatureId) -> bool {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) else {
            return false;
        };
        if m.base.is_summon() {
            return false;
        }
        matches!(self.creatures.get(creature_id), Some(CreatureKind::Monster(other)) if !other.base.is_summon())
    }

    fn monster_is_opponent(&self, monster_id: CreatureId, creature_id: CreatureId) -> bool {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) else {
            return false;
        };
        if m.base.is_summon() {
            let master = m.base.master;
            return master != Some(creature_id);
        }
        match self.creatures.get(creature_id) {
            Some(CreatureKind::Player(p)) => {
                if p.ghost_mode {
                    return false;
                }
                let flags = flags_for_group(&self.groups, p.group_id);
                !has_player_flag(flags, PLAYER_FLAG_IGNORED_BY_MONSTERS)
            }
            Some(other) if other.base().is_summon() => other
                .base()
                .master
                .and_then(|mid| self.creatures.get(mid))
                .is_some_and(|master| matches!(master, CreatureKind::Player(_))),
            _ => false,
        }
    }

    fn monster_add_friend(&mut self, monster_id: CreatureId, friend_id: CreatureId) {
        let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) else {
            return;
        };
        if !m.friend_ids.contains(&friend_id) {
            m.friend_ids.push(friend_id);
        }
    }

    /// Ensure `opponent_id` is in the monster target list before `selectTarget` / move-acquire paths.
    fn monster_ensure_opponent_listed(&mut self, monster_id: CreatureId, opponent_id: CreatureId) {
        let already = self.creatures.get(monster_id).is_some_and(|k| {
            matches!(k, CreatureKind::Monster(m) if m.opponent_ids.contains(&opponent_id))
        });
        if !already {
            self.monster_add_opponent(monster_id, opponent_id, true);
            self.monster_update_idle_status(monster_id);
        }
    }

    fn monster_add_opponent(&mut self, monster_id: CreatureId, opponent_id: CreatureId, push_front: bool) {
        let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) else {
            return;
        };
        if m.opponent_ids.contains(&opponent_id) {
            return;
        }
        if push_front {
            m.opponent_ids.insert(0, opponent_id);
        } else {
            m.opponent_ids.push(opponent_id);
        }
    }

    /// TFS `Monster::updateIdleStatus` / `setIdle` — `monster.cpp` ~700–711.
    pub fn monster_update_idle_status(&mut self, cid: CreatureId) {
        let idle = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => {
                !m.base.is_summon() && m.opponent_ids.is_empty()
            }
            _ => return,
        };
        self.monster_set_idle(cid, idle);
    }

    fn monster_set_idle(&mut self, cid: CreatureId, idle: bool) {
        let became_idle = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) else {
                return;
            };
            if m.base.health <= 0 {
                return;
            }
            if m.is_idle == idle {
                return;
            }
            let was_idle = m.is_idle;
            m.is_idle = idle;
            if !idle && was_idle {
            }
            if idle {
                m.base.damage_map.clear();
                m.opponent_ids.clear();
                m.friend_ids.clear();
                m.base.clear_targets();
                m.base.has_follow_path = false;
                m.base.walk_queue.clear();
            }
            idle
        };
        if became_idle {
            self.remove_creature_think_check(cid);
        } else {
            self.add_creature_think_check(cid);
        }
    }

    /// TFS `Monster::isTarget` — `monster.cpp` ~649.
    fn monster_is_target(&self, monster_id: CreatureId, target_id: CreatureId) -> bool {
        let Some(monster_pos) = self.creatures.get(monster_id).map(|k| k.position()) else {
            return false;
        };
        let Some(target) = self.creatures.get(target_id) else {
            return false;
        };
        if target.base().health <= 0 {
            return false;
        }
        if !self.can_see_creature(monster_id, target_id) {
            return false;
        }
        let tp = target.position();
        if tp.z != monster_pos.z {
            return false;
        }
        if let Some(tile) = self.map.get_tile(tp) {
            if tile.body().zone == ZoneType::Protection {
                return false;
            }
        }
        true
    }

    /// TFS `Monster::canUseAttack` — `monster.cpp` ~876.
    pub fn monster_can_use_attack(&self, monster_id: CreatureId, pos: Position, target_id: CreatureId) -> bool {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) else {
            return false;
        };
        if !m.is_hostile {
            return true;
        }
        let target_pos = match self.creatures.get(target_id) {
            Some(k) => k.position(),
            None => return false,
        };
        let dist = chebyshev(pos, target_pos) as u32;
        let db_name = m.base.name.to_lowercase();
        let spells = self
            .monsters_db
            .monsters
            .get(&db_name)
            .map(|t| t.attack_spells.as_slice())
            .unwrap_or(&[]);
        for spell in spells {
            if spell_in_attack_range(spell, dist) && self.map.is_sight_clear(pos, target_pos) {
                return true;
            }
        }
        false
    }

    /// TFS `Monster::searchTarget` — `monster.cpp` ~517.
    pub fn monster_search_target(&mut self, monster_id: CreatureId, search_type: TargetSearchType) -> bool {
        let (pos, opponents, follow) = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) else {
                return false;
            };
            (m.base.position, m.opponent_ids.clone(), m.base.follow_target)
        };

        let mut result_list: Vec<CreatureId> = Vec::new();
        for &oid in &opponents {
            if follow == Some(oid) {
                continue;
            }
            if !self.monster_is_target(monster_id, oid) {
                continue;
            }
            if search_type == TargetSearchType::Random
                || self.monster_can_use_attack(monster_id, pos, oid)
            {
                result_list.push(oid);
            }
        }

        match search_type {
            TargetSearchType::HealthLow => {
                // B3.1 — pick the weakest reachable opponent. Metric (current vs max HP) is
                // profile-driven: CipSoft 7.72 compares **current** HP (`crnonpl.cc` Strategy),
                // TFS compares **max** HP (`monsters.cpp` `<targetstrategy>`).
                if let Some(best) = self.monster_weakest_opponent(&result_list) {
                    return self.monster_select_target(monster_id, best);
                }
            }
            TargetSearchType::Nearest => {
                if !result_list.is_empty() {
                    let mut best = result_list[0];
                    let mut min_range = self
                        .creatures
                        .get(best)
                        .map(|k| manhattan(pos, k.position()))
                        .unwrap_or(i32::MAX);
                    for &oid in result_list.iter().skip(1) {
                        let Some(d) = self
                            .creatures
                            .get(oid)
                            .map(|k| manhattan(pos, k.position()))
                        else {
                            continue;
                        };
                        if d < min_range {
                            best = oid;
                            min_range = d;
                        }
                    }
                    return self.monster_select_target(monster_id, best);
                }
                let mut best: Option<(CreatureId, i32)> = None;
                for &oid in &opponents {
                    if !self.monster_is_target(monster_id, oid) {
                        continue;
                    }
                    let Some(d) = self
                        .creatures
                        .get(oid)
                        .map(|k| manhattan(pos, k.position()))
                    else {
                        continue;
                    };
                    if best.map(|(_, m)| d < m).unwrap_or(true) {
                        best = Some((oid, d));
                    }
                }
                if let Some((oid, _)) = best {
                    return self.monster_select_target(monster_id, oid);
                }
            }
            TargetSearchType::Default | TargetSearchType::Random | TargetSearchType::AttackRange => {
                if !result_list.is_empty() {
                    let idx = if result_list.len() == 1 {
                        0
                    } else {
                        rand::random::<usize>() % result_list.len()
                    };
                    return self.monster_select_target(monster_id, result_list[idx]);
                }
                if search_type == TargetSearchType::AttackRange {
                    return false;
                }
            }
        }

        for &oid in &opponents {
            if follow != Some(oid) && self.monster_select_target(monster_id, oid) {
                return true;
            }
        }
        false
    }

    /// TFS `Monster::selectTarget` — `monster.cpp` ~662.
    fn monster_select_target(&mut self, monster_id: CreatureId, target_id: CreatureId) -> bool {
        if !self.monster_is_target(monster_id, target_id) {
            return false;
        }
        let in_list = self
            .creatures
            .get(monster_id)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if m.opponent_ids.contains(&target_id)));
        if !in_list {
            return false;
        }

        if let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) {
            if m.is_hostile || m.base.is_summon() {
                if let Some(k) = self.creatures.get_mut(monster_id) {
                    k.base_mut().attack_target = Some(target_id);
                }
            }
        }

        let ret = self.monster_set_follow_creature(monster_id, Some(target_id));
        if ret {
            self.monster_update_look_direction(monster_id);
        }
        ret
    }

    /// Recompute chase path immediately — C++ `Creature::onCreatureMove` instant repath
    /// (`creature.cpp` ~619–637) and avoids waiting for `onThink` (1 s bucket).
    /// Walk execution stays in `creature_start_chase_auto_walk` / scheduler — do not call
    /// `check_creature_walk` here (would deepen the `onWalk` stack and risk recursion on blocked tiles).
    pub(crate) fn monster_follow_repath_now(&mut self, cid: CreatureId) {
        if !self.creatures.get(cid).is_some_and(|k| {
            matches!(k, CreatureKind::Monster(_)) && k.base().follow_target.is_some()
        }) {
            return;
        }
        self.go_to_follow_creature(cid);
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            base.force_update_follow_path = false;
            base.is_updating_path = false;
        }
    }

    /// TFS `Creature::setFollowCreature` — `creature.cpp` ~1058.
    fn monster_set_follow_creature(&mut self, monster_id: CreatureId, target: Option<CreatureId>) -> bool {
        let Some(target_id) = target else {
            if let Some(k) = self.creatures.get_mut(monster_id) {
                let base = k.base_mut();
                base.is_updating_path = false;
                base.follow_target = None;
                base.has_follow_path = false;
            }
            return true;
        };

        if self
            .creatures
            .get(monster_id)
            .and_then(|k| k.base().follow_target)
            == Some(target_id)
        {
            return true;
        }

        let (monster_pos, target_pos) = {
            let Some(mp) = self.creatures.get(monster_id).map(|k| k.position()) else {
                return false;
            };
            let Some(tp) = self.creatures.get(target_id).map(|k| k.position()) else {
                return false;
            };
            (mp, tp)
        };
        if !self.can_see_creature(monster_id, target_id)
            || !creature_can_see(
                monster_pos,
                target_pos,
                i32::from(MAP_MAX_VIEWPORT),
                i32::from(MAP_MAX_VIEWPORT),
            )
        {
            if let Some(k) = self.creatures.get_mut(monster_id) {
                k.base_mut().follow_target = None;
            }
            return false;
        }

        if let Some(k) = self.creatures.get_mut(monster_id) {
            let base = k.base_mut();
            if !base.walk_queue.is_empty() {
                base.walk_queue.clear();
            }
            base.has_follow_path = false;
            base.force_update_follow_path = false;
            base.follow_target = Some(target_id);
            base.is_updating_path = true;
        }
        self.monster_follow_repath_now(monster_id);
        true
    }

    fn monster_think_summon_stub(&mut self, cid: CreatureId) {
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
    fn monster_on_think_target(&mut self, cid: CreatureId, interval_ms: u32) {
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
                    k.base().health > 0 && k.base().next_walk_check.is_none(),
                    k.base().follow_target.is_some(),
                )
            })
            .unwrap_or((false, false));
        if should_arm {
            if chasing {
                self.creature_start_chase_auto_walk(cid);
            } else {
                self.creature_start_auto_walk(cid);
            }
        }
    }

    /// TFS `Monster::updateLookDirection` + `0x6B` broadcast.
    pub fn monster_update_look_direction(&mut self, cid: CreatureId) {
        let (pos, attack, current) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => {
                (m.base.position, m.base.attack_target, m.base.direction)
            }
            _ => return,
        };
        let Some(target_id) = attack else {
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

    /// TFS `Monster::onFollowCreatureComplete` — `monster.cpp` ~599.
    fn monster_on_follow_creature_complete(&mut self, cid: CreatureId, target_id: CreatureId) {
        let (has_path, is_summon) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (m.base.has_follow_path, m.base.is_summon()),
            _ => return,
        };
        let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) else {
            return;
        };
        let idx = m.opponent_ids.iter().position(|&id| id == target_id);
        let Some(idx) = idx else {
            return;
        };
        m.opponent_ids.remove(idx);
        if has_path {
            m.opponent_ids.insert(0, target_id);
        } else if !is_summon {
            m.opponent_ids.push(target_id);
        }
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
                // Only reconcile after a real follow walk finished — not when the queue was already
                // empty from `chase_fully_blocked` (avoids repath storm every walk tick).
                if had_follow_path {
                    self.monster_reconcile_follow_position(cid, target_id);
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

    /// TFS `Monster::onCreatureEnter` via `onCreatureAppear` spectator fan-out — `monster.cpp` ~435.
    pub fn monster_notify_creature_enter_viewport(&mut self, creature_id: CreatureId, pos: Position) {
        let monsters: Vec<CreatureId> = self
            .collect_spatial_spectators(pos, true)
            .into_iter()
            .filter(|&id| {
                id != creature_id
                    && self
                        .creatures
                        .get(id)
                        .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
            })
            .collect();
        self.monster_viewport_notify_depth += 1;
        for monster_id in monsters {
            // C++ `Monster::onCreatureAppear` → `onCreatureEnter` for each spatial spectator (`monster.cpp` ~167).
            self.monster_on_creature_found(monster_id, creature_id, true);
        }
        self.monster_viewport_notify_depth =
            self.monster_viewport_notify_depth.saturating_sub(1);
    }

    /// Notify monsters near a creature move (`Map::moveCreature` spectator fan-out).
    pub fn monster_dispatch_creature_move(
        &mut self,
        moved: CreatureId,
        old_pos: Position,
        new_pos: Position,
    ) {
        let monsters = self.monsters_witnessing_move(old_pos, new_pos);
        let witness_count = monsters.len();
        let moved_is_player = self
            .creatures
            .get(moved)
            .is_some_and(|k| matches!(k, CreatureKind::Player(_)));
        if moved_is_player {
        }
        for monster_id in monsters {
            self.monster_on_creature_move(monster_id, moved, old_pos, new_pos);
        }
    }
}

fn spell_in_attack_range(spell: &MonsterSpellNode, distance: u32) -> bool {
    let range = spell
        .attributes
        .get("range")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    if range == 0 {
        spell.element.eq_ignore_ascii_case("melee") && distance <= 1
    } else {
        distance <= range
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
    use tfs_rust_common::enums::Direction;
    use tfs_rust_common::Position;

    use crate::creature::{CreatureKind, MonsterAiConfig};
    use crate::login_out::creature_wire_id;
    use crate::test_world::support::{
        ensure_walkable_tile, insert_monster_with_config, insert_player, insert_spectator_player,
        minimal_world, test_player,
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
        world.go_to_follow_creature(monster);
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

        world.go_to_follow_creature(monster);

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
}
