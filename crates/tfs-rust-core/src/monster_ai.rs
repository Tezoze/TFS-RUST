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

#[allow(unused_imports)]
pub use crate::monster_targets::TargetSearchType;

use slotmap::Key;
use tfs_rust_common::enums::{CombatType, Direction, ZoneType};
use tfs_rust_common::Position;

use crate::chase_debug;
use crate::combat::{
    armor_reduction, melee_damage_after_defense_and_armor, weapon_damage, CombatDamage,
    CombatParams, FightMode,
};
use crate::creature::{
    creature_immune_poison, melee_poison_on_hit, roll_target_defense,
};
use crate::creature::{ChaseMode, CreatureKind, MonsterState};
use crate::game_world::{creature_can_see, GameWorld};
use crate::ids::CreatureId;
use crate::monster_distance_step::{
    distance_x, distance_y, offset_x, offset_y, search_flight_field,
};
use crate::pathfinding::{
    scan_min_terrain_waypoints, uses_reverse_terrain_path, FindPathParams, CHASE_PATH_MAX_STEPS,
    REVERSE_PATH_VIEW_RADIUS,
};
use crate::player_flags::{flags_for_group, has_player_flag, PLAYER_FLAG_IGNORED_BY_MONSTERS};
use crate::tile::{flags as tilestate, MapStackEntry};
use crate::walk::creature_turn_with_broadcast;

/// C++ `Map::maxViewportX` (`map.h`).
pub(crate) const MAP_MAX_VIEWPORT: u16 = 11;

/// All map directions for brute-force chase steps when A* / `getDistanceStep` fail.
#[allow(dead_code)]
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
/// `crnonpl.cc` dist chase `ToDoGo(..., MaxSteps = Distance − keep)`; trim is `cheb≤1`
/// only (`cract.cc:282-301`). Melee chase uses `max:3, must:false`.
pub(crate) fn monster_idle_chase_step_budget(
    _is_melee_chase: bool,
    is_dist_chase: bool,
    cheb_to_target: i32,
    target_distance: i32,
) -> (usize, bool) {
    if is_dist_chase {
        // C++ `Distance - 4` can be 0 at exact band (`crnonpl.cc`); do not `.max(1)`.
        let steps = (cheb_to_target - target_distance).max(0) as usize;
        (steps, false)
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

/// 772 master follow wait-only band — `crnonpl.cc:2766` (Manhattan 2 → `ToDoWait` only).
pub(crate) fn monster_master_follow_wait_only_band(manhattan_dist: i32) -> bool {
    manhattan_dist == 2
}

/// 772 master follow includes `ToDoWait` before `ToDoGo` — `crnonpl.cc:2769`.
pub(crate) fn monster_master_follow_wait_before_go(manhattan_dist: i32) -> bool {
    manhattan_dist == 3
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
#[allow(dead_code)]
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

    /// True when a **non-empty** `walk_queue` no longer reaches the follow band or sight is blocked.
    ///
    /// Empty queue is not stale here — 772 batch replan runs from idle segment drain / `off_band`,
    /// not from every target tile (`crnonpl.cc` `ToDoGo` after drain).
    #[allow(dead_code)]
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

    /// True when the monster is already in the desired follow/attack band (C++ empty `listWalkDir` at goal).
    pub(crate) fn monster_at_follow_goal(
        &self,
        _cid: CreatureId,
        _follow_id: CreatureId,
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
        // 772 keep-distance — per-type band from monsters.xml (`crnonpl.cc` dist branches).
        dist == target_distance
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

    /// B3.1 — lowest-health opponent from `candidates`, using the profile's [`WeakestTargetMetric`]
    /// (current HP for 772, max HP for TFS). Ties keep the first candidate.
    ///
    /// C++ `Monster::onCreatureAppear` self branch — `monster.cpp` ~159–166.
    ///
    /// C++ `TCombat::Attack` / `CloseAttack` / `DistanceAttack` — `crcombat.cc:530`, `:609`, `:647`.
    pub fn monster_do_attacking(&mut self, cid: CreatureId, _interval_ms: u32) {
        // Shared `TCombat::Attack` gate — delayed StopAttack expire (`crcombat.cc:551-553`).
        if self.combat_expire_delayed_stop_attack(cid) {
            return;
        }

        self.monster_update_look_direction(cid);

        let server_ms = self.server_ms;
        let profile = self.mechanics.profile;

        let (
            target_id,
            monster_pos,
            melee_skill,
            melee_attack,
            poison_cycles,
        ) = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
                return;
            };
            let Some(target_id) = m.base.attack_target else {
                return;
            };
            // Fist-only Attack — `GetDistance=1` (`crcombat.cc:309-318`). Spellcasters
            // with melee_skill=0 never DistanceAttack; ranged damage is CASTING only.
            if m.melee_skill <= 0 {
                return;
            }
            (
                target_id,
                m.base.position,
                m.melee_skill,
                m.melee_attack,
                m.poison_cycles,
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

        if cheb > 1 || in_pz || melee_skill <= 0 {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().delay_attack_ms(server_ms, 200);
            }
            return;
        }

        // C++ `ResyncHarnessRng` at appear + one lose/talk prelude per idle (`crnonpl.cc:2429`, `:2440`).
        // Rust harness drains can run extra idle preambles before the first strike — realign probes.
        // Dual-monster real-map bowl: C++ draw order differs from one_real; skip global realign (T5).
        // 1098-only realign removed (era gating eliminated in Phase 5).

        let _trace_atk = crate::sim_glibc_rand::sim_rng_trace_site("melee_attack_probe");

        let defense_snap = self.melee_defense_snapshot_for(target_id);
        let target_immune_poison = creature_immune_poison(self.creatures.get(target_id).unwrap());

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().delay_attack_ms(server_ms, 200);
        }

        let attack_roll = {
            let hooks = &self.mechanics.hooks;
            weapon_damage(
                &profile,
                hooks,
                melee_skill,
                melee_attack,
                FightMode::Balanced,
                0,
                &self.parity_rng,
            )
        };

        // M11 — shield wearout gate check: capture whether the defense gate will pass before
        // `roll_target_defense` updates the timestamps. Shield wearout happens only when the
        // gate passes (`crcombat.cc:265-281`). Player targets only (monsters have no shields).
        let defense_gate_passed = self
            .creatures
            .get(target_id)
            .is_some_and(|k| server_ms >= k.base().earliest_defend_ms);
        let mut defense_snap = defense_snap;
        // ProbeValue Increase before Get when the defend gate will fire (`crcombat.cc:259-263`).
        if defense_gate_passed
            && defense_snap.has_shield
            && matches!(self.creatures.get(target_id), Some(CreatureKind::Player(_)))
        {
            self.player_shield_skill_learning(target_id, true);
            defense_snap = self.melee_defense_snapshot_for(target_id);
        }
        let hooks = &self.mechanics.hooks;
        let defense_roll = {
            let Some(kind) = self.creatures.get_mut(target_id) else {
                return;
            };
            let _trace = crate::sim_glibc_rand::sim_rng_trace_site("melee_defense_probe");
            roll_target_defense(
                kind.base_mut(),
                server_ms,
                &profile,
                hooks,
                defense_snap,
                &self.parity_rng,
            )
        };

        let _trace_armor = crate::sim_glibc_rand::sim_rng_trace_site("melee_armor_probe");
        let armor_roll =
            armor_reduction(&profile, hooks, defense_snap.armor, &self.parity_rng);
        // M11 — Shield wearout: decrement the player defender's shield `REMAININGUSES` when the
        // defense gate passed (`crcombat.cc:265-281`). Player-only. Called after `hooks` is last
        // used to avoid borrow conflict with `&mut self`.
        if defense_gate_passed
            && matches!(self.creatures.get(target_id), Some(CreatureKind::Player(_)))
        {
            self.player_shield_wearout(target_id);
        }
        let dmg = melee_damage_after_defense_and_armor(attack_roll, defense_roll, armor_roll);

        // Poff / spark — C++ `TCreature::Damage` (`crmain.cc:577-579, 624-628`).
        if dmg <= 0 {
            if let Some(pos) = self.creatures.get(target_id).map(|k| k.base().position) {
                let effect = if attack_roll <= defense_roll {
                    3u8
                } else {
                    4u8
                };
                self.broadcast_magic_effect(pos, effect);
            }
        }

        let notify_snap = self.combat_notify_snapshot(target_id);
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
        if let Some(snap) = notify_snap {
            self.notify_player_combat_damage(Some(cid), target_id, damage_done, CombatType::Physical, snap);
        }

        // A2 — `if (DamageDone > 0) ActivateLearning()` (`crcombat.cc:664-666`). Mirrors the
        // player strike path; monsters rarely live long enough to level, but the C++ `CloseAttack`
        // fires `ActivateLearning` for all attacker types. The `LearningPoints = 30` window gates
        // the (PC-5) per-skill `Increase(1)` accumulation in `ProbeValue`.
        if damage_done > 0 {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().activate_learning();
            }
        }

        if !target_immune_poison {
            if let Some(cond) = melee_poison_on_hit(
                poison_cycles,
                attack_roll,
                defense_roll,
                damage_done,
                &self.parity_rng,
            ) {
                // 772 CloseAttack poison → `Damage(…, DAMAGE_POISON_PERIODIC)` (`crcombat.cc:660`).
                let strength = match cond.data {
                    crate::condition::ConditionData::Damage { total_rank } => total_rank,
                    _ => 0,
                };
                if strength > 0 {
                    let _ = self.combat_execute_with_stimulus(
                        Some(cid),
                        target_id,
                        &crate::combat::CombatDamage {
                            primary: (
                                tfs_rust_common::enums::CombatType::PoisonPeriodic,
                                -strength,
                            ),
                            secondary: (CombatType::Undefined, 0),
                        },
                        &CombatParams::default(),
                    );
                }
                // M10 — `SendMessage(Target->Connection, TALK_STATUS_MESSAGE, "You are poisoned.")`
                // (`crcombat.cc:674-676`). Sent to a player target after the poison condition lands.
                if let Some(CreatureKind::Player(_)) = self.creatures.get(target_id) {
                    self.send_player_status_message(target_id, "You are poisoned.");
                }
            }
        }

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().delay_attack_ms(server_ms, 2000);
        }

        // A1 — `if (Target->IsDead) this->StopAttack(0)` (`crcombat.cc:643-645`). C++ `CloseAttack`
        // clears the attacker's combat targets after the strike when the victim died. Our melee
        // arm early-returns when `target_alive` is false at entry, but the strike itself can kill
        // the target (HP ≤ 0 → `apply_creature_death` removes it from `world.creatures`). Without
        // this, a monster keeps swinging at a removed target id until the next `target_alive` gate.
        let target_dead = !self.creatures.contains_key(target_id);
        if target_dead {
            if let Some(k) = self.creatures.get_mut(cid) {
                let base = k.base_mut();
                base.attack_target = None;
                base.follow_target = None;
            }
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
    pub(crate) fn monster_on_chase_noway(&mut self, cid: CreatureId) {
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            base.clear_targets();
            base.has_follow_path = false;
            base.walk_queue.clear();
            base.walk_destinations.clear();
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
                base.walk_destinations.clear();
                base.walk_queue.push_back(dir);
                base.walk_destinations.push_back(dest);
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
        let Some(dir) = search_flight_field(
            pos,
            target_pos,
            |dir| self.monster_can_walk_to(cid, pos, dir),
            |buf| self.parity_rng.random_shuffle(buf),
        ) else {
            return false;
        };
        let dest = pos.offset(dir);
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            base.walk_queue.clear();
            base.walk_destinations.clear();
            base.walk_queue.push_back(dir);
            base.walk_destinations.push_back(dest);
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
        if target_distance > 1 {
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
                if chebyshev(dest, target_pos) != band || !self.monster_can_walk_to(cid, pos, step)
                {
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
            base.walk_destinations.clear();
            if let Some(step) = dir {
                base.walk_queue.push_back(step);
                base.walk_destinations.push_back(dest);
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

    /// 772 idle master follow — `crnonpl.cc:2760-2773` (`ToDoGo` max 3 when Manhattan ≥ 3).
    ///
    /// Caller must gate Manhattan ≤ 2 / empty `walk_queue` before invoking.
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
        debug_assert!(
            dist >= 3,
            "monster_idle_master_follow is for Manhattan ≥ 3 only (got {dist})"
        );
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

    /// Apply A* path or return false so caller can try one-tile fallbacks.
    #[allow(clippy::too_many_arguments)]
    fn monster_try_apply_chase_path(
        &mut self,
        cid: CreatureId,
        target_pos: Position,
        _fleeing: bool,
        target_distance: i32,
        fpp: &FindPathParams,
        max_steps: usize,
        must_reach: bool,
    ) -> bool {
        let pos = match self.creatures.get(cid) {
            Some(k) => k.position(),
            None => return false,
        };
        let tries: &[&FindPathParams] = &[fpp];
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
                // Already in goal band — C++ `ToDoGo` early-return / Calculate with 0 Gos.
                return true;
            }
            // 772 `TShortway::Calculate` trim — `cract.cc:282-301` (`CurDistance > 1` + MaxSteps).
            // Dist keep-band is MaxSteps only (`cheb − target_distance`), not a trim stop.
            // Predecessor-chain order (first hop first).
            steps = crate::pathfinding::truncate_tshortway_go_queue(
                pos,
                target_pos,
                steps,
                max_steps,
                must_reach,
            );
            if chase_debug::chase_path_debug_enabled() {
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
            // Path reachable but MaxSteps/adjacent trim yielded no Go — C++ still returns true.
            if steps.is_empty() {
                return true;
            }
            if let Some(k) = self.creatures.get_mut(cid) {
                let base = k.base_mut();
                base.walk_queue.clear();
                base.walk_destinations.clear();
                // `listWalkDir` — `getNextStep` pops from the back (`creature.cpp`).
                // Accumulate absolute destinations in execution order (matching C++ TDGo
                // absolute coords — `cract.cc:286-288`), then push in reverse so `pop_back`
                // on both queues stays in sync.
                let mut acc = pos;
                let dests: Vec<Position> = steps
                    .iter()
                    .map(|&d| {
                        acc = acc.offset(d);
                        acc
                    })
                    .collect();
                for d in steps.iter().rev() {
                    base.walk_queue.push_back(*d);
                }
                for dest in dests.iter().rev() {
                    base.walk_destinations.push_back(*dest);
                }
                base.has_follow_path = true;
            }
            // 772 idle executor owns `Go` enqueue via `monster_idle_prepare_and_enqueue_go`.
            return true;
        }
        if chase_debug::chase_path_debug_enabled() {
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
            max_search_dist: 0,
        };

        if is_summon {
            let master = self.creatures.get(cid).and_then(|k| k.base().master);
            if master == Some(follow_id) {
                fpp.max_target_dist = 2;
                fpp.full_path_search = true;
            } else if target_distance <= 1 {
                fpp.full_path_search = true;
            } else {
                fpp.full_path_search =
                    target_pos.is_some_and(|tp| chebyshev(pos, tp) != target_distance);
            }
        } else if fleeing {
            fpp.max_target_dist = i32::from(MAP_MAX_VIEWPORT);
            fpp.clear_sight = false;
            fpp.full_path_search = false;
        } else if target_distance <= 1 {
            fpp.full_path_search = true;
        } else {
            // 772 `DistanceFighting` — cheb band, not TFS `canUseAttack` (`crnonpl.cc:2723`).
            fpp.full_path_search =
                target_pos.is_some_and(|tp| chebyshev(pos, tp) != target_distance);
        }

        fpp
    }

    fn get_creature_path_to_with_fpp(
        &mut self,
        cid: CreatureId,
        target: Position,
        fpp: &FindPathParams,
    ) -> Option<Vec<Direction>> {
        use crate::pathfinding::{
            get_path_matching_with_fill, CREATURE_ON_TILE_PATH_COST, REVERSE_PATH_VIEW_RADIUS,
        };

        let start = self.creatures.get(cid)?.position();
        let uses_reverse_terrain = uses_reverse_terrain_path(
            self.mechanics.profile.path_cost,
            self.mechanics.profile.path_search,
        );
        debug_assert!(
            uses_reverse_terrain,
            "772 monster chase requires reverse TShortway + terrain costs (check MechanicsProfile / formulas lua)"
        );
        let path_cost = self.mechanics.profile.path_cost;
        let path_search = self.mechanics.profile.path_search;
        let path_forward_fallback = self.mechanics.profile.path_forward_fallback;
        let path_t0 = std::time::Instant::now();
        let path = {
            let world = &*self;
            struct PathCtx<'a> {
                world: &'a GameWorld,
                cid: CreatureId,
            }
            let ctx = PathCtx { world, cid };
            let fill_walkable = |pos: Position| {
                if uses_reverse_terrain {
                    ctx.world
                        .monster_tshortway_fill_walkable(ctx.cid, pos, target)
                } else {
                    ctx.world.monster_can_occupy_chase_tile(ctx.cid, pos)
                }
            };
            let mut scratch = world.tshortway_scratch.borrow_mut();
            get_path_matching_with_fill(
                &world.map,
                start,
                target,
                fpp,
                path_cost,
                path_search,
                path_forward_fallback,
                REVERSE_PATH_VIEW_RADIUS,
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
                |pos| {
                    let Some(tile) = ctx.world.map.get_tile(pos) else {
                        return 0;
                    };
                    ctx.world.tile_ground_speed(tile.body())
                },
                |pos| ctx.world.fillmap_terrain_waypoints_at(pos),
                Some(&mut *scratch),
            )
        };
        let expanded = self.tshortway_scratch.borrow().last_expanded;
        let path_us = path_t0.elapsed().as_micros() as u64;
        self.obs
            .record_path_search(path_us, expanded, path.is_some());
        path
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
                m.base.walk_timer_idle(),
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
        let path = self.get_creature_path_to(cid, spawn, 0, max_dist, usize::MAX);
        if path.is_none() {
            return;
        }
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.walking_to_spawn = true;
            m.base.walk_queue.clear();
            m.base.walk_destinations.clear();
            if let Some(path) = path {
                // `get_creature_path_to` returns forward execution order (first step first).
                // `walk_queue` uses `push_back` + `pop_back` (LIFO), so push in reverse to
                // make `pop_back` yield the first step. Accumulate absolute destinations
                // in execution order (matching C++ TDGo — `cract.cc:286-288`), then push
                // both in reverse so `pop_back` on each queue stays in sync.
                let mut acc = pos;
                let dests: Vec<Position> = path
                    .iter()
                    .map(|&d| {
                        acc = acc.offset(d);
                        acc
                    })
                    .collect();
                for d in path.iter().rev() {
                    m.base.walk_queue.push_back(*d);
                }
                for dest in dests.iter().rev() {
                    m.base.walk_destinations.push_back(*dest);
                }
            }
            m.base.has_follow_path = true;
        }
        self.creature_start_auto_walk(cid);
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
                self.monster_on_follow_creature_complete(cid, target_id);
                // 772: band reconcile + look runs from idle.
            }
        }
    }

    /// TFS `Monster::getNextStep` — `monster.cpp` ~1224.
    pub(crate) fn monster_next_walk_step(
        &mut self,
        cid: CreatureId,
        _now: std::time::Instant,
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
            _fleeing,
            _static_attack_chance,
            _target_distance,
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
                    self.mechanics.profile.underground_sees_surface,
                )
            })
        });

        if (is_summon && master_in_range) || follow.is_some() || walking_to_spawn {
            if let Some(k) = self.creatures.get_mut(cid) {
                if let Some(dir) = k.base_mut().walk_queue.pop_back() {
                    return Some(dir);
                }
            }

            // C++ `Creature::getNextStep` returns false when the queue is empty (`creature.cpp` ~251–260);
            // repath runs from `onThink` / target-move only, not synchronously from `getNextStep`.

            // C++ target dancing when follow queue empty — `monster.cpp` ~1244–1256.
            if follow == attack && follow.is_some() {
                // 772 idle drain owns flee/dance/chase — no TFS getNextStep poll (X4).
                return None;
            }
        }

        None
    }

    /// 772 `TShortway::FillMap` stack-head BANK `WAYPOINTS` (`cract.cc:89-99`) — OTB only, no `MovePossible`.
    ///
    /// Raw OTB speed `0` → `-1` (C++ invalid Waypoints / mountain Bank Unpass). Passable Clip borders
    /// used as sole OTBM ground should be patched to 150 offline (`build_passable_zero_speed_defaults`).
    pub(crate) fn fillmap_terrain_waypoints_at(&self, pos: Position) -> i32 {
        let Some(tile) = self.map.get_tile(pos) else {
            return -1;
        };
        let chain = tile.body().map_object_chain();
        let Some(MapStackEntry::Ground(server_id)) = chain.first() else {
            return -1;
        };
        if !self.items_db.is_terrain_bank(*server_id)
            || self.items_db.is_unpassable_for_field(*server_id)
        {
            return -1;
        }
        let wp = self
            .items_db
            .waypoints_raw_for_item(*server_id)
            .unwrap_or(0);
        // Non-Bank Unpass already rejected above; remaining wp0 is invalid terrain.
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
    pub(crate) fn monster_move_possible_planning(&self, cid: CreatureId, pos: Position) -> bool {
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
                // Ground BANK with objects.srv Unpass (incl. Bank+wp0 after cliff clear-solid).
                MapStackEntry::Ground(server_id) => {
                    if self.items_db.is_unpassable_for_field(*server_id) {
                        return false;
                    }
                }
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
                    // Field Unpass includes Bank+wp0 cliffs (lesson 171) — players walk them via
                    // cleared blockSolid; monsters still see objects.srv Unpass (`crnonpl.cc:2249`).
                    if self.items_db.is_unpassable_for_field(server_id) {
                        if self.items_db.is_immovable(server_id) || !can_push_items {
                            return false;
                        }
                        continue;
                    }
                    if self.items_db.is_avoid_hazard(server_id) {
                        // C++ `MovePossible` AVOID branch (`crnonpl.cc:2264-2267`): per-damage-type
                        // immunity — PANIC ignores all hazards; NoPoison/NoBurning/NoEnergy ignore
                        // matching fields only. P1-B1: was poison-only, now per-type.
                        let ignore_hazard = state == MonsterState::Panic
                            || match self.items_db.avoid_damage_type(server_id) {
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
                            && (self.items_db.is_immovable(server_id) || !can_push_items)
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
    /// [`Self::monster_move_possible_planning`]. Used by the A*/TShortway pathfinder only.
    pub(crate) fn monster_tshortway_fill_walkable(
        &self,
        cid: CreatureId,
        pos: Position,
        _target: Position,
    ) -> bool {
        // `MovePossible` creature/item gate (PZ, house, leash, creatures, items).
        if !self.monster_move_possible_planning(cid, pos) {
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
        if !self.items_db.is_terrain_bank(*head_id) {
            return false;
        }
        true
    }

    /// Spawn leash + pathfinding tile check — shared by A* and step selection (`monster.cpp` `canWalkTo`).
    ///
    /// Delegates to [`Self::monster_tshortway_fill_walkable`] which mirrors `MovePossible(Execute=false)`
    /// (`crnonpl.cc:2141–2293`): the 772 creature/item gate with per-damage-type hazard immunity,
    /// invisibility, house, and player-plannable-through semantics.
    fn monster_can_occupy_chase_tile(&self, cid: CreatureId, pos: Position) -> bool {
        // P1-A3: route single-step gates (dance/roam/flee/chase) through the 772 `MovePossible`
        // planning model. Uses `monster_move_possible_planning` (no TShortway terrain checks).
        self.monster_move_possible_planning(cid, pos)
    }

    /// Effective per-home roam leash radius (axis-box, non-attacking). Uses the monster's
    /// `home_radius` (CipSoft `MonsterhomeInRange`, `crnonpl.cc:2157`); an unset home
    /// (`home_radius <= 0`) falls back to the global despawn radius (audit Finding 17b).
    fn monster_roam_leash_radius(&self, home_radius: i32) -> i32 {
        if home_radius > 0 {
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
