//! 772 drain-triggered idle AI — `IdleStimulus` on ToDo queue drain.
//!
//! - `TCreature::IdleStimulus` — virtual dispatch after `Execute` drains the action list.
//! - `TMonster::IdleStimulus` — `crnonpl.cc:2386`.
//!
//! Profile-gated via `GameWorld::beat_driven_loop` (same flag as P2 ToDo walk).

use std::time::Instant;

use tfs_rust_common::Position;

use crate::chase_debug;
use crate::creature::{CreatureKind, MonsterChaseMode, MonsterState, monster_weapon_attack_distance};
use crate::creature_think::EVENT_CREATURE_THINK_INTERVAL_MS;
use crate::creature_todo::{
    trace_creature_todo, CreatureAction, MONSTER_IDLE_WAIT_MS,
};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::monster_ai::{
    chebyshev, manhattan, monster_idle_chase_step_budget, monster_master_follow_in_wait_band,
    MonsterCombatCloseChaseEnqueue, MonsterEnqueueAttackResult, MonsterIdleChaseRepathOutcome,
};
use crate::monster_targets::TargetSearchType;

/// C++ `TMonster::IdleStimulus` walking arms — `crnonpl.cc:2676`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MonsterIdleWalkBranch {
    /// `crnonpl.cc:2678` — `IsFleeing` + `SearchFlightField`.
    Flee,
    /// `crnonpl.cc:2686` — summon following master.
    MasterFollow,
    /// `crnonpl.cc:2732` — melee `ToDoGo` toward target.
    MeleeChase,
    /// `crnonpl.cc:2751` — adjacent cardinal sidestep.
    MeleeDance,
    /// `crnonpl.cc:2762` — too close for keep-distance band.
    DistFlee,
    /// `crnonpl.cc:2769` — approach keep-distance band.
    DistChase,
    /// `crnonpl.cc:2787` — lateral at keep-distance.
    DistDance,
    /// `crnonpl.cc:2850` — random roam when no target.
    Roam,
    /// At band / no movement this idle tick.
    Hold,
}

/// Result of executing one idle walk arm.
enum MonsterIdleWalkOutcome {
    QueuedGo {
        via: &'static str,
        wait_after: bool,
    },
    QueuedWait,
    Noway,
    Hold,
}

/// Which todo action ran — drives post-execute chaining.
pub(crate) enum TodoExecuteKind {
    Go,
    Wait,
    Attack,
    /// Attack cadence not ready — re-queued and wakeup scheduled (`cract.cc:909`).
    AttackDeferred,
}

impl GameWorld {
    /// 772 `TCreature::IdleStimulus` — dispatch on creature kind.
    pub(crate) fn idle_stimulus(&mut self, cid: CreatureId) {
        if !self.beat_driven_loop {
            return;
        }
        if !self.creatures.contains_key(cid) {
            return;
        }
        if self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().todo.locked)
        {
            return;
        }
        match self.creatures.get(cid) {
            Some(CreatureKind::Monster(_)) => {
                trace_creature_todo(self, cid, "idle_stimulus_enter");
                self.monster_idle_stimulus(cid);
                trace_creature_todo(self, cid, "idle_stimulus_exit");
            }
            _ => {}
        }
    }

    /// Request idle when the action queue is drained — sync or deferred to next wakeup.
    pub(crate) fn request_idle_stimulus(&mut self, cid: CreatureId) {
        if !self.beat_driven_loop {
            return;
        }
        if !self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
        {
            return;
        }
        if !self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().walk_timer_idle(self.beat_driven_loop))
        {
            return;
        }
        if !self.creature_todo_queue_empty(cid) {
            return;
        }
        trace_creature_todo(self, cid, "request_idle_stimulus");
        self.idle_stimulus(cid);
    }

    /// 772 `TMonster::IdleStimulus` — chase/repath/roam decisions (772 only).
    pub(crate) fn monster_idle_stimulus(&mut self, cid: CreatureId) {
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

        let (is_idle, is_summon, has_opponents, follow, fleeing, pos) = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
                return;
            };
            (
                m.is_idle,
                m.base.is_summon(),
                !m.opponent_ids.is_empty(),
                m.base.follow_target,
                m.is_fleeing(),
                m.base.position,
            )
        };

        if is_idle {
            return;
        }

        if self.beat_driven_loop {
            self.monster_idle_reset_combat_state(cid);
        }

        if is_summon {
            self.monster_think_summon_stub(cid);
        } else if has_opponents {
            if follow.is_none() {
                let _ = self.monster_search_target(cid, TargetSearchType::Default);
            }
            // 772 chase repath: segment drain + target-move queue hysteresis only — not TFS
            // `monster_ensure_follow_band` (1098 think / walk-complete guard).
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

        if !self.beat_driven_loop {
            self.monster_on_think_target(cid, EVENT_CREATURE_THINK_INTERVAL_MS);
        }
        // 1098: `onThink` drives `updateLookDirection` once per tick.
        // 772: avoid force-facing while an active chase batch is running; let walk direction
        // carry facing, and only snap toward target when not chasing.
        if !self.beat_driven_loop {
            self.monster_update_look_direction(cid);
        } else if self
            .creatures
            .get(cid)
            .is_some_and(|k| {
                let base = k.base();
                base.attack_target.is_some()
                    && base.walk_queue.is_empty()
                    && base.todo.is_empty()
                    && base.follow_target.is_none()
            })
        {
            self.monster_update_look_direction(cid);
        }

        if !self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().health > 0 && k.base().walk_timer_idle(self.beat_driven_loop))
        {
            return;
        }

        self.monster_idle_prepare_and_enqueue_go(cid);

        // C++ idle tail appends `ToDoAttack` even when walk already queued `ToDoGo` (`crnonpl.cc:2795`).
        let attack_enqueued = self.monster_idle_maybe_enqueue_attack(cid);
        if self.creature_todo_queue_empty(cid) {
            self.monster_idle_maybe_enqueue_at_goal_wait(cid, attack_enqueued);
        }
    }

    /// C++ `TMonster::IdleStimulus` — `crnonpl.cc:2387` (reset unless PANIC/UNDERATTACK).
    fn monster_idle_reset_combat_state(&mut self, cid: CreatureId) {
        let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) else {
            return;
        };
        if !matches!(m.state, MonsterState::Panic | MonsterState::UnderAttack) {
            m.state = MonsterState::Idle;
        }
    }

    /// C++ walking prelude — `crnonpl.cc:2705` (`SKILL_FIST > 0 && State != PANIC`).
    fn monster_idle_maybe_enter_attacking(&mut self, cid: CreatureId) {
        let should_attack = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
                return;
            };
            if m.is_fleeing() {
                return;
            }
            let Some(follow_id) = m.base.follow_target else {
                return;
            };
            if m.base.master == Some(follow_id) {
                return;
            }
            if m.state == MonsterState::Panic {
                return;
            }
            m.melee_skill > 0
        };
        if should_attack {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.state = MonsterState::Attacking;
            }
        }
    }

    /// C++ ATTACKING walk prelude — `crnonpl.cc:2709-2726` (`SetChaseMode` reset then CLOSE for melee).
    pub(crate) fn monster_idle_prepare_combat_chase(&mut self, cid: CreatureId) {
        if !self.beat_driven_loop {
            return;
        }
        let snapshot = self.creatures.get(cid).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            Some((
                m.state,
                m.base.follow_target,
                m.base.position,
                m.target_distance,
            ))
        });
        let Some((state, follow_id, pos, raw_target_distance)) = snapshot else {
            return;
        };
        let new_mode = if !matches!(state, MonsterState::Attacking | MonsterState::Panic) {
            MonsterChaseMode::None
        } else if let Some(follow_id) = follow_id {
            let target_distance = self.monster_effective_target_distance(raw_target_distance);
            let uses_dist_branch =
                target_distance > 1 && self.monster_can_use_attack(cid, pos, follow_id);
            if uses_dist_branch {
                MonsterChaseMode::None
            } else {
                MonsterChaseMode::Close
            }
        } else {
            MonsterChaseMode::None
        };
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.chase_mode = new_mode;
        }
    }

    /// C++ `TCreature::ToDoAttack` action list — `cract.cc:1325-1334`.
    pub(crate) fn monster_enqueue_todo_attack_actions(
        &mut self,
        cid: CreatureId,
    ) -> MonsterEnqueueAttackResult {
        let (weapon_distance, needs_close_step) = self
            .creatures
            .get(cid)
            .map(|k| match k {
                CreatureKind::Monster(m) => {
                    let weapon_distance = monster_weapon_attack_distance(
                        m.melee_skill,
                        m.spells.iter().any(|s| s.range > 1),
                    );
                    let needs_close_step = m.base.attack_target.is_some_and(|aid| {
                        self.creatures.get(aid).is_some_and(|t| {
                            weapon_distance == 1 && chebyshev(m.base.position, t.position()) > 1
                        })
                    });
                    (weapon_distance, needs_close_step)
                }
                _ => (1, false),
            })
            .unwrap_or((1, false));
        let close_chase = self.monster_combat_enqueue_close_chase_go(cid);
        if close_chase == MonsterCombatCloseChaseEnqueue::Noway {
            return MonsterEnqueueAttackResult::Noway;
        }
        if needs_close_step
            && close_chase != MonsterCombatCloseChaseEnqueue::Queued
            && !self.monster_close_chase_go_already_armed(cid)
        {
            return MonsterEnqueueAttackResult::Failed;
        }
        if weapon_distance != 1 {
            self.enqueue_creature_wait(cid, 100);
        }
        if self.enqueue_creature_attack(cid) {
            self.schedule_immediate_todo_wakeup(cid);
            MonsterEnqueueAttackResult::Enqueued
        } else {
            MonsterEnqueueAttackResult::Failed
        }
    }

    /// 772 melee tail uses cheb band for strike; enqueue uses `ToDoAttack` walk path (`cract.cc:1325`).
    fn monster_idle_can_enqueue_attack(
        &self,
        cid: CreatureId,
        pos: Position,
        attack_id: CreatureId,
        target_pos: Position,
    ) -> bool {
        if !self.beat_driven_loop {
            return self.monster_can_use_attack(cid, pos, attack_id);
        }
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return false;
        };
        let target_distance = self.monster_effective_target_distance(m.target_distance);
        let dist = chebyshev(pos, target_pos);
        // `CanToDoAttack` close walk while approaching — not gated on strike range (`crcombat.cc:496`).
        if m.melee_skill > 0 && m.chase_mode == MonsterChaseMode::Close && dist <= 8 {
            return true;
        }
        if target_distance <= 1 {
            // Melee `ToDoAttack` closes via `CanToDoAttack` — not limited to cheb==1 at enqueue.
            return dist <= 8;
        }
        self.monster_can_use_attack(cid, pos, attack_id)
    }

    /// C++ idle combat tail — `Rotate` + `ToDoAttack` (`crnonpl.cc:2795`).
    fn monster_idle_maybe_enqueue_attack(&mut self, cid: CreatureId) -> bool {
        let (attack_id, pos) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => {
                if m.is_fleeing() {
                    return false;
                }
                let Some(attack_id) = m.base.attack_target else {
                    return false;
                };
                (attack_id, m.base.position)
            }
            _ => return false,
        };
        if !self.creatures.contains_key(attack_id) {
            return false;
        }
        let target_pos = match self.creatures.get(attack_id) {
            Some(k) => k.position(),
            None => return false,
        };
        if !self.map.is_sight_clear(pos, target_pos) {
            return false;
        }
        if !self.monster_idle_can_enqueue_attack(cid, pos, attack_id, target_pos) {
            return false;
        }
        // C++ always appends `ToDoAttack` at the idle tail (`crnonpl.cc:2795`); cadence is enforced
        // by `TDAttack` on execute (`cract.cc:909`), not by skipping enqueue here.
        match self.monster_enqueue_todo_attack_actions(cid) {
            MonsterEnqueueAttackResult::Enqueued => {
                trace_creature_todo(self, cid, "idle_enqueue_attack");
                true
            }
            MonsterEnqueueAttackResult::Noway => {
                self.monster_idle_prepare_and_enqueue_go(cid);
                false
            }
            MonsterEnqueueAttackResult::Failed => {
                // Blocked close-chase: retry on short cadence instead of dead queue until
                // target/blocker moves (`crmain.cc:919` `ToDoWait(200)`).
                self.idle_enqueue_wait_and_start(cid, 200);
                false
            }
        }
    }

    /// `ToDoWait(1000)` when at-goal dance could not arm (`crnonpl.cc:2791` dist band).
    /// Melee `ATTACKING` tail gets `ToDoAttack` only — no trailing wait (`crnonpl.cc:2795–2807`).
    fn monster_idle_maybe_enqueue_at_goal_wait(
        &mut self,
        cid: CreatureId,
        attack_enqueued: bool,
    ) {
        if !self.beat_driven_loop {
            return;
        }
        let branch = self.monster_idle_classify_walk_branch(cid);
        match branch {
            MonsterIdleWalkBranch::DistDance => {}
            MonsterIdleWalkBranch::MeleeDance => {
                if attack_enqueued {
                    return;
                }
                if self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| k.base().todo.has_attack())
                {
                    return;
                }
                let hostile_melee_at_band = match self.creatures.get(cid) {
                    Some(CreatureKind::Monster(m)) => match m.base.follow_target {
                        None => false,
                        Some(follow_id) => {
                            let target_distance =
                                self.monster_effective_target_distance(m.target_distance);
                            if target_distance > 1 {
                                false
                            } else if let Some(t) = self.creatures.get(follow_id) {
                                chebyshev(m.base.position, t.position()) == 1
                                    && self.monster_idle_is_attacking_posture(cid, target_distance)
                            } else {
                                false
                            }
                        }
                    },
                    _ => false,
                };
                if hostile_melee_at_band {
                    return;
                }
            }
            _ => return,
        }
        self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
    }

    /// When idle should run [`GameWorld::monster_idle_chase_repath`] for an active chase (772 only).
    pub(crate) fn monster_idle_chase_needs_repath(
        &mut self,
        cid: CreatureId,
    ) -> (bool, Option<&'static str>) {
        let Some(k) = self.creatures.get(cid) else {
            return (false, None);
        };
        let base = k.base();
        if base.force_update_follow_path {
            if let Some(follow_id) = base.follow_target {
                let pos = k.position();
                if let Some(target_pos) = self.creatures.get(follow_id).map(|t| t.position()) {
                    let (fleeing, target_distance) = match self.creatures.get(cid) {
                        Some(CreatureKind::Monster(m)) => (
                            m.is_fleeing(),
                            self.monster_effective_target_distance(m.target_distance),
                        ),
                        _ => return (true, Some("force_update")),
                    };
                    if self.monster_at_follow_goal(
                        cid,
                        follow_id,
                        pos,
                        target_pos,
                        fleeing,
                        target_distance,
                    ) {
                        if let Some(k) = self.creatures.get_mut(cid) {
                            k.base_mut().force_update_follow_path = false;
                        }
                        return (false, None);
                    }
                }
            }
            return (true, Some("force_update"));
        }
        if !base.walk_queue.is_empty() {
            return (false, None);
        }
        if !base.has_follow_path {
            return (true, Some("idle_drain"));
        }
        let Some(follow_id) = base.follow_target else {
            return (false, None);
        };
        let pos = k.position();
        let Some(target_pos) = self.creatures.get(follow_id).map(|t| t.position()) else {
            return (false, None);
        };
        let (fleeing, target_distance) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                m.is_fleeing(),
                self.monster_effective_target_distance(m.target_distance),
            ),
            _ => return (false, None),
        };
        if self.monster_at_follow_goal(cid, follow_id, pos, target_pos, fleeing, target_distance) {
            return (false, None);
        }
        (true, Some("off_band"))
    }

    /// Classify the idle walk arm — `crnonpl.cc:2676` priority order.
    ///
    /// Melee vs ranged split proxies `!DistanceFighting || !ThrowPossible` via
    /// `target_distance <= 1 || !monster_can_use_attack`.
    pub(crate) fn monster_idle_classify_walk_branch(
        &self,
        cid: CreatureId,
    ) -> MonsterIdleWalkBranch {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return MonsterIdleWalkBranch::Hold;
        };

        let follow_id = match m.base.follow_target {
            Some(id) => id,
            None => return MonsterIdleWalkBranch::Roam,
        };

        if m.is_fleeing() {
            return MonsterIdleWalkBranch::Flee;
        }

        if m.base.master == Some(follow_id) {
            return MonsterIdleWalkBranch::MasterFollow;
        }

        let pos = m.base.position;
        let target_pos = match self.creatures.get(follow_id) {
            Some(k) => k.position(),
            None => return MonsterIdleWalkBranch::Roam,
        };
        let target_distance = self.monster_effective_target_distance(m.target_distance);
        let dist = chebyshev(pos, target_pos);

        let uses_dist_branch =
            target_distance > 1 && self.monster_can_use_attack(cid, pos, follow_id);

        if uses_dist_branch {
            if dist < target_distance {
                MonsterIdleWalkBranch::DistFlee
            } else if dist > target_distance {
                MonsterIdleWalkBranch::DistChase
            } else {
                MonsterIdleWalkBranch::DistDance
            }
        } else if dist > 1 {
            if self.monster_idle_skip_idle_melee_chase(cid) {
                MonsterIdleWalkBranch::Hold
            } else {
                MonsterIdleWalkBranch::MeleeChase
            }
        } else if dist == 1 {
            MonsterIdleWalkBranch::MeleeDance
        } else {
            MonsterIdleWalkBranch::Hold
        }
    }

    fn monster_idle_log_walk_branch(
        &self,
        cid: CreatureId,
        branch: &str,
        dest: Position,
        must: bool,
        max_steps: i32,
        reason: Option<&str>,
    ) {
        if !chase_debug::chase_path_debug_enabled() {
            return;
        }
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return;
        };
        chase_debug::log_branch(
            self.tick_counter,
            cid,
            m.base.name.as_str(),
            branch,
            m.base.position,
            dest,
            must,
            max_steps,
            reason,
        );
    }

    /// Execute one classified walk arm — returns outcome without enqueuing `Go`.
    fn monster_idle_execute_walk_branch(
        &mut self,
        cid: CreatureId,
        branch: MonsterIdleWalkBranch,
    ) -> MonsterIdleWalkOutcome {
        match branch {
            MonsterIdleWalkBranch::Flee => {
                if self.monster_idle_flee_step(cid) {
                    MonsterIdleWalkOutcome::QueuedGo {
                        via: "idle_flee",
                        wait_after: false,
                    }
                } else {
                    MonsterIdleWalkOutcome::Hold
                }
            }
            MonsterIdleWalkBranch::DistFlee => {
                if self.monster_idle_flee_step(cid) {
                    MonsterIdleWalkOutcome::QueuedGo {
                        via: "idle_flee",
                        wait_after: false,
                    }
                } else {
                    MonsterIdleWalkOutcome::QueuedWait
                }
            }
            MonsterIdleWalkBranch::MasterFollow => {
                let (needs_repath, repath_reason) = self.monster_idle_chase_needs_repath(cid);
                if !needs_repath {
                    return self.monster_idle_master_follow_hold_or_wait(cid);
                }
                match self.monster_idle_master_follow(cid, repath_reason) {
                    MonsterIdleChaseRepathOutcome::PathQueued => MonsterIdleWalkOutcome::QueuedGo {
                        via: repath_reason.unwrap_or("idle_drain"),
                        wait_after: false,
                    },
                    MonsterIdleChaseRepathOutcome::AtGoal => {
                        self.monster_idle_master_follow_hold_or_wait(cid)
                    }
                    MonsterIdleChaseRepathOutcome::Noway => MonsterIdleWalkOutcome::Noway,
                }
            }
            MonsterIdleWalkBranch::MeleeChase | MonsterIdleWalkBranch::DistChase => {
                let (needs_repath, repath_reason) = self.monster_idle_chase_needs_repath(cid);
                if !needs_repath {
                    return MonsterIdleWalkOutcome::Hold;
                }
                let branch_name = if branch == MonsterIdleWalkBranch::MeleeChase {
                    "melee_chase"
                } else {
                    "dist_chase"
                };
                let cheb = self
                    .creatures
                    .get(cid)
                    .and_then(|k| {
                        let follow_id = k.base().follow_target?;
                        let target_pos = self.creatures.get(follow_id)?.position();
                        Some(chebyshev(k.position(), target_pos))
                    })
                    .unwrap_or(0);
                let is_melee_chase = branch == MonsterIdleWalkBranch::MeleeChase;
                let is_dist_chase = branch == MonsterIdleWalkBranch::DistChase;
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
                if let Some(target_pos) = self
                    .creatures
                    .get(cid)
                    .and_then(|k| k.base().follow_target)
                    .and_then(|tid| self.creatures.get(tid).map(|t| t.position()))
                {
                    self.monster_idle_log_walk_branch(
                        cid,
                        branch_name,
                        target_pos,
                        must_reach,
                        max_steps as i32,
                        repath_reason,
                    );
                }
                match self.monster_idle_chase_repath(cid, repath_reason, max_steps, must_reach) {
                    MonsterIdleChaseRepathOutcome::PathQueued => MonsterIdleWalkOutcome::QueuedGo {
                        via: repath_reason.unwrap_or("idle_drain"),
                        wait_after: false,
                    },
                    MonsterIdleChaseRepathOutcome::AtGoal => MonsterIdleWalkOutcome::Hold,
                    MonsterIdleChaseRepathOutcome::Noway => MonsterIdleWalkOutcome::Noway,
                }
            }
            MonsterIdleWalkBranch::MeleeDance => {
                if self.monster_idle_dance_step(cid) {
                    MonsterIdleWalkOutcome::QueuedGo {
                        via: "idle_dance",
                        wait_after: false,
                    }
                } else {
                    MonsterIdleWalkOutcome::Hold
                }
            }
            MonsterIdleWalkBranch::DistDance => {
                if self.monster_idle_dance_step(cid) {
                    MonsterIdleWalkOutcome::QueuedGo {
                        via: "idle_dance",
                        wait_after: true,
                    }
                } else {
                    MonsterIdleWalkOutcome::Hold
                }
            }
            MonsterIdleWalkBranch::Roam => {
                let pos = self
                    .creatures
                    .get(cid)
                    .map(|k| k.position())
                    .unwrap_or(Position::new(0, 0, 7));
                self.monster_idle_log_walk_branch(cid, "roam", pos, false, 1, None);
                if self.monster_idle_roam_step(cid) {
                    MonsterIdleWalkOutcome::QueuedGo {
                        via: "roam",
                        wait_after: true,
                    }
                } else {
                    MonsterIdleWalkOutcome::Hold
                }
            }
            MonsterIdleWalkBranch::Hold => MonsterIdleWalkOutcome::Hold,
        }
    }

    /// Master follow Manhattan 2–3 → `ToDoWait` only (`crnonpl.cc:2691`).
    fn monster_idle_master_follow_hold_or_wait(&self, cid: CreatureId) -> MonsterIdleWalkOutcome {
        let (pos, follow_id) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => {
                let Some(follow_id) = m.base.follow_target else {
                    return MonsterIdleWalkOutcome::Hold;
                };
                (m.base.position, follow_id)
            }
            _ => return MonsterIdleWalkOutcome::Hold,
        };
        let target_pos = match self.creatures.get(follow_id) {
            Some(k) => k.position(),
            None => return MonsterIdleWalkOutcome::Hold,
        };
        if monster_master_follow_in_wait_band(manhattan(pos, target_pos)) {
            MonsterIdleWalkOutcome::QueuedWait
        } else {
            MonsterIdleWalkOutcome::Hold
        }
    }

    /// Fill walk queue from reference-ordered idle arms, then enqueue `Go` + heap arm.
    ///
    /// C++ walking section — `crnonpl.cc:2676`.
    fn monster_idle_prepare_and_enqueue_go(&mut self, cid: CreatureId) {
        if self.beat_driven_loop {
            self.monster_idle_maybe_enter_attacking(cid);
            self.monster_idle_prepare_combat_chase(cid);
        }
        let branch = self.monster_idle_classify_walk_branch(cid);
        let mut outcome = self.monster_idle_execute_walk_branch(cid, branch);

        if matches!(outcome, MonsterIdleWalkOutcome::Noway) {
            self.monster_on_chase_noway_772(cid);
            outcome = self.monster_idle_execute_walk_branch(cid, MonsterIdleWalkBranch::Roam);
        }

        match outcome {
            MonsterIdleWalkOutcome::QueuedGo { via, wait_after } => {
                self.idle_enqueue_paced_go(
                    cid,
                    true,
                    Some(via),
                    wait_after.then_some(MONSTER_IDLE_WAIT_MS),
                );
            }
            MonsterIdleWalkOutcome::QueuedWait => {
                self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
            }
            MonsterIdleWalkOutcome::Hold => {
                // 772 idle drain owns dance pacing — no TFS `getNextStep` poll (X5).
                if !self.beat_driven_loop && self.monster_should_keep_dance_walk_alive(cid) {
                    self.idle_enqueue_go_and_start(cid, true, None);
                }
            }
            MonsterIdleWalkOutcome::Noway => {}
        }
    }

    /// Execute the front todo action for 772 monsters.
    pub(crate) fn execute_creature_todo_action(&mut self, cid: CreatureId) -> Option<TodoExecuteKind> {
        let action = {
            let Some(k) = self.creatures.get_mut(cid) else {
                return None;
            };
            if k.base().todo.locked {
                return None;
            }
            k.base_mut().todo.queue.pop_front()
        };
        let action = action?;

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().todo.locked = true;
        }

        let kind = match action {
            CreatureAction::Go => {
                trace_creature_todo(self, cid, "execute_go");
                let now = Instant::now();
                self.on_walk(cid, false, now, None);
                trace_creature_todo(self, cid, "execute_go_done");
                TodoExecuteKind::Go
            }
            CreatureAction::Wait { delay_ms } => {
                trace_creature_todo(self, cid, "execute_wait");
                self.todo_start_from_action(cid, delay_ms);
                trace_creature_todo(self, cid, "execute_wait_done");
                TodoExecuteKind::Wait
            }
            CreatureAction::Attack => {
                let delay = self.todo_attack_delay_ms(cid);
                if delay > 0 {
                    if let Some(k) = self.creatures.get_mut(cid) {
                        k.base_mut().todo.queue.push_front(CreatureAction::Attack);
                    }
                    trace_creature_todo(self, cid, "execute_attack_deferred");
                    self.todo_start_from_action(cid, delay);
                    trace_creature_todo(self, cid, "execute_attack_deferred_done");
                    TodoExecuteKind::AttackDeferred
                } else {
                    let needs_close_step = self
                        .creatures
                        .get(cid)
                        .and_then(|k| {
                            let CreatureKind::Monster(m) = k else {
                                return None;
                            };
                            let aid = m.base.attack_target?;
                            let cheb = chebyshev(
                                m.base.position,
                                self.creatures.get(aid)?.position(),
                            );
                            let weapon_dist = monster_weapon_attack_distance(
                                m.melee_skill,
                                m.spells.iter().any(|s| s.range > 1),
                            );
                            Some(weapon_dist == 1 && cheb > 1)
                        })
                        .unwrap_or(false);
                    if needs_close_step {
                        if let Some(k) = self.creatures.get_mut(cid) {
                            k.base_mut().todo.queue.push_front(CreatureAction::Attack);
                        }
                        match self.monster_combat_enqueue_close_chase_go(cid) {
                            MonsterCombatCloseChaseEnqueue::Queued => {
                                if self
                                    .creatures
                                    .get(cid)
                                    .is_some_and(|k| k.base().todo.has_go())
                                {
                                    if self.todo_start_go_delay(cid, false) {
                                        self.schedule_immediate_todo_wakeup(cid);
                                    }
                                }
                            }
                            MonsterCombatCloseChaseEnqueue::Noway => {
                                if let Some(k) = self.creatures.get_mut(cid) {
                                    k.base_mut().todo.queue.pop_front();
                                }
                                self.idle_stimulus(cid);
                            }
                            MonsterCombatCloseChaseEnqueue::Skipped => {
                                self.request_idle_stimulus(cid);
                            }
                        }
                        trace_creature_todo(self, cid, "execute_attack_out_of_range");
                        TodoExecuteKind::AttackDeferred
                    } else {
                        trace_creature_todo(self, cid, "execute_attack");
                        self.monster_do_attacking(cid, EVENT_CREATURE_THINK_INTERVAL_MS);
                        trace_creature_todo(self, cid, "execute_attack_done");
                        TodoExecuteKind::Attack
                    }
                }
            }
        };

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().todo.locked = false;
        }

        Some(kind)
    }

    /// Execute one `CreatureAction::Go` for 772 monsters — returns true if an action ran.
    pub(crate) fn execute_creature_todo_go(&mut self, cid: CreatureId) -> bool {
        matches!(
            self.execute_creature_todo_action(cid),
            Some(TodoExecuteKind::Go)
        )
    }

    /// After Go/Attack execute: schedule next step or chain queued actions.
    pub(crate) fn finish_creature_todo_execute(&mut self, cid: CreatureId) {
        if !self.creature_uses_todo_execute(cid) {
            return;
        }

        let walk_queue_has_more = self
            .creatures
            .get(cid)
            .is_some_and(|k| !k.base().walk_queue.is_empty());

        if walk_queue_has_more {
            let force_repath = self
                .creatures
                .get(cid)
                .is_some_and(|k| k.base().force_update_follow_path);
            if force_repath {
                if let Some(k) = self.creatures.get_mut(cid) {
                    let base = k.base_mut();
                    base.walk_queue.clear();
                    base.has_follow_path = false;
                }
                self.request_idle_stimulus(cid);
                return;
            }
            // Re-arm `Go` before pending `Attack` — one step per execute (`cract.cc:728`).
            let _ = self.enqueue_creature_go_at(cid, true);
            if self.todo_start_go_delay(cid, false) {
                self.schedule_immediate_todo_wakeup(cid);
            }
            return;
        }

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().has_follow_path = false;
        }

        if !self.creature_todo_queue_empty(cid) {
            self.run_monster_todo_execute(cid);
            return;
        }

        self.idle_stimulus(cid);
    }

    /// Run one queued action (772 monsters).
    pub(crate) fn run_monster_todo_execute(&mut self, cid: CreatureId) {
        match self.execute_creature_todo_action(cid) {
            Some(TodoExecuteKind::Go) | Some(TodoExecuteKind::Attack) => {
                self.finish_creature_todo_execute(cid);
            }
            Some(TodoExecuteKind::Wait) | Some(TodoExecuteKind::AttackDeferred) => {}
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use tfs_rust_common::enums::{ConditionType, Direction};
    use tfs_rust_common::Position;

    use crate::monster_ai::{MonsterCombatCloseChaseEnqueue, MonsterEnqueueAttackResult};
    use crate::creature::{CreatureKind, MonsterAiConfig, MonsterChaseMode, MonsterState, MonsterSpell, SpellImpact, SpellShape};
    use crate::creature_think::EVENT_CREATURE_THINK_INTERVAL_MS;
    use crate::creature_todo::{CreatureAction, MONSTER_IDLE_WAIT_MS};
    use crate::game_world::GameWorld;
    use crate::ids::CreatureId;
    use crate::idle_stimulus::MonsterIdleWalkBranch;
    use crate::test_world::support::{
        ensure_walkable_tile, insert_monster, insert_monster_with_config, insert_player,
        minimal_world, test_player,
    };

    fn beat_driven_test_world() -> crate::game_world::GameWorld {
        let mut world = minimal_world();
        world.mechanics =
            crate::formulas::Mechanics::for_version(tfs_rust_common::ProtocolVersion::V772);
        world.beat_driven_loop = true;
        world.walk_wake_tx = None;
        world.server_ms = 0;
        world
    }

    /// Phase A — idle enqueues Go on drain; think no longer arms walk on 772.
    #[test]
    fn idle_stimulus_enqueues_go_for_active_monster() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        for x in 101..=104 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 2148);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.attack_target = Some(player);
        }
        world.add_creature_think_check(monster);
        assert!(
            world.monster_set_follow_creature(monster, Some(player)),
            "set_follow must succeed in view"
        );

        let has_go = world
            .creatures
            .get(monster)
            .is_some_and(|k| k.base().todo.has_go());
        let armed = world
            .creatures
            .get(monster)
            .and_then(|k| k.base().next_wakeup)
            .is_some();
        assert!(
            has_go || armed,
            "772 set_follow must enqueue Go or schedule wakeup via idle"
        );

        if world
            .creatures
            .get(monster)
            .is_some_and(|k| k.base().todo.has_go())
        {
            world.execute_creature_todo_go(monster);
        }

        world.monster_native_on_think(monster, EVENT_CREATURE_THINK_INTERVAL_MS);
        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go()),
            "772 think must not enqueue Go actions"
        );
    }

    /// Phase A — duplicate Go / heap entries suppressed when wakeup already armed.
    #[test]
    fn idle_go_enqueue_respects_wakeup_gate() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 2148);
        let monster = insert_monster(&mut world, "Rat", pos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }

        assert!(world.enqueue_creature_go(monster));
        world.todo_start_from_action(monster, 500);
        let wakeup = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .next_wakeup
            .expect("wakeup armed");
        let heap_len = world.todo_queue.len();

        assert!(!world.enqueue_creature_go(monster), "duplicate Go rejected");
        world.request_idle_stimulus(monster);

        assert_eq!(
            world.creatures.get(monster).unwrap().base().next_wakeup,
            Some(wakeup)
        );
        assert_eq!(world.todo_queue.len(), heap_len);
    }

    /// Phase A — process_creature_todo runs idle when action queue empty on wakeup.
    #[test]
    fn process_creature_todo_runs_idle_on_empty_queue() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(108, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        for x in 101..=108 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 2148);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 220);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }
        world.add_creature_think_check(monster);

        world.schedule_creature_wakeup(monster, 0);
        world.process_creature_todo(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go() || k.base().next_wakeup.is_some()),
            "drain with empty queue must idle-enqueue chase Go"
        );
    }

    /// Phase A — segment drain clears `has_follow_path` so idle repaths on next wakeup.
    #[test]
    fn idle_repaths_after_segment_drain_clears_follow_path() {
        let mut world = beat_driven_test_world();
        world.mechanics.profile.follow_repath_without_path = true;

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(108, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        for x in 101..=108 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 2148);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 220);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = true;
            m.base.walk_queue.clear();
        }

        world.finish_creature_todo_execute(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| !k.base().walk_queue.is_empty() || k.base().todo.has_go()),
            "772 finish must idle-repath after segment drain (has_follow_path cleared)"
        );
    }

    /// 772 active monster without follow enqueues roam Go from idle (TFS `getRandomStep` arm).
    #[test]
    fn idle_stimulus_enqueues_roam_for_active_monster_without_follow() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new((100 + dx) as u16, (100 + dy) as u16, 7),
                    2148,
                );
            }
        }

        let monster = insert_monster(&mut world, "Wolf", pos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }

        world.monster_idle_stimulus(monster);

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(todo.has_go(), "772 idle must enqueue roam Go");
        assert!(todo.has_wait(), "772 roam must enqueue Wait(1000) after Go");
    }

    /// Blocked dance / stand-still at melee goal must not force a chase repath on next idle.
    #[test]
    fn force_update_at_follow_goal_skips_idle_repath() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = true;
            m.base.force_update_follow_path = true;
            m.base.walk_queue.clear();
        }

        let (needs, reason) = world.monster_idle_chase_needs_repath(monster);
        assert!(!needs, "at-goal force_update must not schedule repath");
        assert!(reason.is_none());
        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().force_update_follow_path),
            "stale force_update must be cleared at follow goal"
        );
    }

    /// 1098 regression — think still arms walk when not beat-driven.
    #[test]
    fn think_arm_still_runs_on_1098() {
        let mut world = minimal_world();
        assert!(!world.beat_driven_loop);

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        for x in 101..=104 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 2148);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.attack_target = Some(player);
        }
        world.add_creature_think_check(monster);
        assert!(world.monster_set_follow_creature(monster, Some(player)));

        world.monster_native_on_think(monster, EVENT_CREATURE_THINK_INTERVAL_MS);

        let armed = world.creatures.get(monster).is_some_and(|k| {
            k.base().next_walk_check.is_some() || !k.base().walk_queue.is_empty()
        });
        assert!(armed, "1098 think must still arm monster walk");
    }

    #[test]
    fn test_772_classify_roam_without_follow() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 2148);
        let monster = insert_monster(&mut world, "Wolf", pos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::Roam
        );
    }

    #[test]
    fn test_772_classify_flee_before_melee() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.health = 10;
            m.run_away_health = 20;
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::Flee
        );
    }

    #[test]
    fn test_772_classify_master_follow() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);

        let master = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.master = Some(master);
            m.base.follow_target = Some(master);
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MasterFollow
        );
    }

    #[test]
    fn test_772_classify_melee_vs_dist() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos_melee = Position::new(103, 100, 7);
        let ppos_dist = Position::new(106, 100, 7);
        for x in 99..=106 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 2148);
        }

        let melee_monster = insert_monster_with_config(
            &mut world,
            "FixtureIdleChase772",
            mpos,
            200,
            MonsterAiConfig {
                is_hostile: false,
                ..MonsterAiConfig::default()
            },
        );
        let melee_player = insert_player(&mut world, test_player("Hero1", ppos_melee));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(melee_monster) {
            m.is_idle = false;
            m.base.follow_target = Some(melee_player);
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(melee_monster),
            MonsterIdleWalkBranch::MeleeChase
        );

        let dist_monster = insert_monster(&mut world, "Rat", mpos, 200);
        let dist_player = insert_player(&mut world, test_player("Hero2", ppos_dist));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(dist_monster) {
            m.is_idle = false;
            m.target_distance = 4;
            m.is_hostile = false;
            m.base.follow_target = Some(dist_player);
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(dist_monster),
            MonsterIdleWalkBranch::DistChase
        );
    }

    #[test]
    fn test_772_classify_dist_dance_at_band() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.target_distance = 4;
            m.is_hostile = false;
            m.base.follow_target = Some(player);
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::DistDance
        );
    }

    #[test]
    fn test_772_classify_melee_dance_adjacent() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeDance,
            "follow without attack_target may still rand(0,4) dance"
        );
    }

    #[test]
    fn test_772_attacking_posture_keeps_melee_dance_at_adjacent() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeDance,
            "ATTACKING melee still rand(0,4) dances at cheb==1"
        );
    }

    /// Flee arm uses `SearchFlightField` (single step), not a multi-step `TShortway` batch.
    #[test]
    fn test_772_flee_uses_flight_field_not_shortway() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        ensure_walkable_tile(&mut world.map, Position::new(99, 100, 7), 2148);

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
            m.run_away_health = 20;
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        world.monster_idle_stimulus(monster);

        let queue_len = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .len();
        assert!(
            queue_len <= 1,
            "flee idle must queue at most one flight-field step, got {queue_len}"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go() || k.base().next_wakeup.is_some()),
            "flee idle must enqueue Go"
        );
    }

    /// P0-4 — melee chase at cheb==2 uses reference `must:false, max:3`; trim stops at cheb≤1.
    ///
    /// Uses default spawn (`melee_skill==0`, state not `Attacking`) so classify stays `MeleeChase`;
    /// fist monsters in `Attacking` skip idle chase — see `test_e3_attacking_skips_idle_melee_chase`.
    #[test]
    fn test_772_melee_chase_cheb2_must_false_max_three() {
        use crate::monster_ai::{monster_idle_chase_step_budget, MonsterIdleChaseRepathOutcome};
        use crate::pathfinding::CHASE_PATH_MAX_STEPS;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeChase
        );
        let (max_steps, must_reach) = monster_idle_chase_step_budget(true, false, 2, 1);
        assert_eq!((max_steps, must_reach), (CHASE_PATH_MAX_STEPS, false));

        let outcome = world.monster_idle_chase_repath(
            monster,
            Some("idle_drain"),
            max_steps,
            must_reach,
        );
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert_eq!(
            world.creatures.get(monster).unwrap().base().walk_queue.len(),
            1,
            "melee chase at cheb==2 must queue one step (trim at cheb≤1), not must:true NOWAY"
        );
    }

    /// A2 regression — farther melee chase still allows up to 3 steps.
    #[test]
    fn test_772_melee_chase_cheb4_three_steps() {
        use crate::monster_ai::{monster_idle_chase_step_budget, MonsterIdleChaseRepathOutcome};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        for x in 100..=104u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        let (max_steps, must_reach) = monster_idle_chase_step_budget(true, false, 4, 1);
        assert_eq!((max_steps, must_reach), (3, false));

        let outcome = world.monster_idle_chase_repath(
            monster,
            Some("idle_drain"),
            max_steps,
            must_reach,
        );
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert_eq!(
            world.creatures.get(monster).unwrap().base().walk_queue.len(),
            3,
            "open-line melee chase at cheb==4 should queue three steps"
        );
    }

    /// A3 — dist chase step budget is `cheb - target_distance`, not global `CHASE_PATH_MAX_STEPS`.
    #[test]
    fn test_772_dist_chase_step_budget_from_target_distance() {
        use crate::monster_ai::{monster_idle_chase_step_budget, MonsterIdleChaseRepathOutcome};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos_band4 = Position::new(106, 100, 7);
        let ppos_band3 = Position::new(106, 110, 7);
        for x in 100..=106u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let dist_monster = insert_monster(&mut world, "Rat", mpos, 200);
        let dist_player = insert_player(&mut world, test_player("Hero4", ppos_band4));
        world.map.register_creature_at(ppos_band4, dist_player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(dist_monster) {
            m.is_idle = false;
            m.target_distance = 4;
            m.is_hostile = false;
            m.base.follow_target = Some(dist_player);
            m.base.attack_target = Some(dist_player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(dist_monster),
            MonsterIdleWalkBranch::DistChase
        );
        let (max_steps, must_reach) = monster_idle_chase_step_budget(false, true, 6, 4);
        assert_eq!((max_steps, must_reach), (2, false));

        let outcome = world.monster_idle_chase_repath(
            dist_monster,
            Some("idle_drain"),
            max_steps,
            must_reach,
        );
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert_eq!(
            world
                .creatures
                .get(dist_monster)
                .unwrap()
                .base()
                .walk_queue
                .len(),
            2,
            "dist chase at cheb==6 with band 4 should queue two steps"
        );

        let mpos_band3 = Position::new(100, 110, 7);
        for x in 100..=106u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 110, 7), 150);
        }
        let band3_monster = insert_monster(&mut world, "Rat", mpos_band3, 200);
        let band3_player = insert_player(&mut world, test_player("Hero3", ppos_band3));
        world.map.register_creature_at(ppos_band3, band3_player);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(band3_monster) {
            m.is_idle = false;
            m.target_distance = 3;
            m.is_hostile = false;
            m.base.follow_target = Some(band3_player);
            m.base.attack_target = Some(band3_player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        let (max_steps, must_reach) = monster_idle_chase_step_budget(false, true, 6, 3);
        assert_eq!((max_steps, must_reach), (3, false));
        let outcome = world.monster_idle_chase_repath(
            band3_monster,
            Some("idle_drain"),
            max_steps,
            must_reach,
        );
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert_eq!(
            world
                .creatures
                .get(band3_monster)
                .unwrap()
                .base()
                .walk_queue
                .len(),
            3,
            "dist chase at cheb==6 with band 3 should queue three steps"
        );
    }

    /// A2 / X5 — failed melee dance at band must not re-enqueue Go on 772 idle Hold.
    #[test]
    fn test_772_idle_hold_no_dance_poll() {
        use crate::tile::{flags as tilestate, Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);
        for (x, y) in [(99, 100), (101, 100), (100, 99), (100, 101)] {
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

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeDance,
            "ATTACKING melee still attempts rand(0,4) dance at cheb==1"
        );

        world.monster_idle_stimulus(monster);

        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go()),
            "blocked dance tiles must not enqueue spurious Go"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_attack()),
            "stick-fight must enqueue Attack when dance cannot move"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .is_empty()
        );
    }

    /// A0 — TShortway NOWAY clears chase target and enqueues roam Go same idle tick.
    #[test]
    fn test_772_chase_noway_clears_target_and_roams() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new((100 + dx) as u16, (100 + dy) as u16, 7),
                    2148,
                );
            }
        }
        ensure_walkable_tile(&mut world.map, ppos, 150);

        let monster = insert_monster_with_config(
            &mut world,
            "FixtureIdleChase772",
            mpos,
            200,
            MonsterAiConfig {
                is_hostile: false,
                ..MonsterAiConfig::default()
            },
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeChase,
            "non-fist fixture must use idle melee chase"
        );

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().follow_target.is_none()),
            "NOWAY must clear follow target"
        );
        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(todo.has_go(), "NOWAY must enqueue roam Go on same idle tick");
        assert!(
            todo.has_wait(),
            "NOWAY roam must enqueue trailing Wait(1000)"
        );
    }

    /// A4 / X4 — 772 `getNextStep` must not inline flee when queue is empty.
    #[test]
    fn test_772_get_next_step_no_inline_flee_on_772() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);

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
            m.run_away_health = 20;
            m.base.has_follow_path = true;
            m.base.walk_queue.clear();
        }

        let now = std::time::Instant::now();
        assert_eq!(
            world.monster_next_walk_step(monster, now),
            None,
            "772 getNextStep must defer flee to idle drain"
        );
    }

    /// A4 — dist_dance at keep band via idle only, not `getNextStep`.
    #[test]
    fn test_772_dist_dance_via_idle() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        ensure_walkable_tile(&mut world.map, Position::new(99, 100, 7), 2148);
        ensure_walkable_tile(&mut world.map, Position::new(101, 100, 7), 2148);
        ensure_walkable_tile(&mut world.map, Position::new(100, 99, 7), 2148);
        ensure_walkable_tile(&mut world.map, Position::new(100, 101, 7), 2148);

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
            m.target_distance = 4;
            m.is_hostile = false;
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        for _ in 0..50 {
            if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
                m.base.walk_queue.clear();
                m.base.has_follow_path = false;
            }
            world.monster_idle_stimulus(monster);
            if let Some(dir) = world
                .creatures
                .get(monster)
                .and_then(|k| k.base().walk_queue.back().copied())
            {
                assert!(
                    matches!(dir, Direction::North | Direction::South),
                    "only North or South maintain target distance 4 from East-aligned target, got {:?}",
                    dir
                );
            }
        }
    }

    /// A5 / B2 — master follow Manhattan 2 enqueues Wait only (no Go).
    #[test]
    fn test_772_master_follow_manhattan_2_hold() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);

        let master = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.master = Some(master);
            m.base.follow_target = Some(master);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .is_empty(),
            "Manhattan 2 must hold without chase path"
        );
        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go()),
            "Manhattan 2 must not enqueue Go"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_wait()),
            "Manhattan 2 must enqueue Wait(1000)"
        );
    }

    /// A5 / B2 — master follow Manhattan 3 enqueues Wait only.
    #[test]
    fn test_772_master_follow_manhattan_3_hold() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(103, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);

        let master = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.master = Some(master);
            m.base.follow_target = Some(master);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .is_empty(),
            "Manhattan 3 must hold without chase path"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_wait()),
            "Manhattan 3 must enqueue Wait(1000)"
        );
    }

    /// A5 — master follow beyond wait band queues up to 3 steps.
    #[test]
    fn test_772_master_follow_manhattan_5_chases() {
        use crate::monster_ai::MonsterIdleChaseRepathOutcome;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        for x in 100..=105u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 2148);
        }

        let master = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        world.map.register_creature_at(ppos, master);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.master = Some(master);
            m.base.follow_target = Some(master);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        let outcome = world.monster_idle_master_follow(monster, Some("idle_drain"));
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .len()
                <= 3,
            "master follow must cap at 3 steps"
        );
    }

    #[test]
    fn test_772_wait_schedules_1000ms_wakeup() {
        let mut world = beat_driven_test_world();
        world.server_ms = 200;
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 2148);
        let monster = insert_monster(&mut world, "Rat", pos, 200);

        world.idle_enqueue_wait_and_start(monster, MONSTER_IDLE_WAIT_MS);
        world.run_monster_todo_execute(monster);

        assert!(
            world.creatures.get(monster).unwrap().base().todo.is_empty()
        );
        assert_eq!(
            world.creatures.get(monster).unwrap().base().next_wakeup,
            Some(200 + MONSTER_IDLE_WAIT_MS)
        );
    }

    /// Regression: multi-step chase must drain the full `walk_queue`, not freeze after one Go.
    #[test]
    fn test_772_multi_step_chase_continues_after_first_go() {
        use crate::monster_ai::{monster_idle_chase_step_budget, MonsterIdleChaseRepathOutcome};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        for x in 100..=104u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        let (max_steps, must_reach) = monster_idle_chase_step_budget(true, false, 4, 1);
        assert_eq!((max_steps, must_reach), (3, false));
        let outcome = world.monster_idle_chase_repath(
            monster,
            Some("idle_drain"),
            max_steps,
            must_reach,
        );
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert_eq!(
            world.creatures.get(monster).unwrap().base().walk_queue.len(),
            3
        );

        world.enqueue_creature_go(monster);
        world.schedule_immediate_todo_wakeup(monster);
        world.process_creature_todo(monster);

        let pos_after_one = world.creatures.get(monster).unwrap().position();
        assert!(
            pos_after_one.x > mpos.x,
            "first Go must move monster east from {:?}, got {:?}",
            mpos,
            pos_after_one
        );

        let wq_after_one = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .len();
        assert!(
            wq_after_one >= 1,
            "after first step walk_queue should still have pending steps, got {wq_after_one}"
        );

        // Drain all scheduled wakeups until monster reaches player column or stalls.
        for _ in 0..20 {
            let wakeup = world
                .creatures
                .get(monster)
                .and_then(|k| k.base().next_wakeup);
            let Some(wu) = wakeup else {
                break;
            };
            world.server_ms = wu;
            while world
                .todo_queue
                .peek()
                .is_some_and(|e| e.execution_time <= world.server_ms)
            {
                world.drain_todo_queue();
            }
        }

        let final_pos = world.creatures.get(monster).unwrap().position();
        assert!(
            final_pos.x > pos_after_one.x,
            "multi-step chase must continue past first tile (after one={:?}, final={:?}, wq={})",
            pos_after_one,
            final_pos,
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .len()
        );
    }

    #[test]
    fn test_772_roam_pacing_via_wait_not_last_step() {
        let mut world = beat_driven_test_world();
        world.server_ms = 0;
        let pos = Position::new(100, 100, 7);
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new((100 + dx) as u16, (100 + dy) as u16, 7),
                    2148,
                );
            }
        }
        let monster = insert_monster(&mut world, "Wolf", pos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }

        world.monster_idle_stimulus(monster);
        assert!(world.creatures.get(monster).unwrap().base().todo.has_go());

        world.run_monster_todo_execute(monster);
        assert!(
            world.creatures.get(monster).unwrap().base().todo.is_empty(),
            "Go then Wait chain must drain Go and schedule Wait"
        );
        assert!(
            world.creatures.get(monster).unwrap().base().next_wakeup.unwrap() >= MONSTER_IDLE_WAIT_MS
        );

        world.monster_idle_stimulus(monster);
        assert!(
            !world.creatures.get(monster).unwrap().base().todo.has_go(),
            "Wait in flight must block immediate re-roam"
        );
    }

    #[test]
    fn test_772_dist_flee_fail_enqueues_wait() {
        use tfs_rust_common::enums::ZoneType;

        use crate::tile::{flags as tilestate, Tile, TileBody};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        for (x, y) in [(99, 100), (100, 99), (100, 101)] {
            world.map.insert_tile(
                Position::new(x, y, 7),
                Tile::Normal(TileBody {
                    ground: Some(2148),
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.target_distance = 4;
            m.is_hostile = false;
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::DistFlee
        );

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_wait() && !k.base().todo.has_go()),
            "dist_flee fail must enqueue Wait only"
        );
    }

    #[test]
    fn test_772_dist_dance_enqueues_go_and_wait() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        ensure_walkable_tile(&mut world.map, Position::new(99, 100, 7), 2148);
        ensure_walkable_tile(&mut world.map, Position::new(101, 100, 7), 2148);
        ensure_walkable_tile(&mut world.map, Position::new(100, 99, 7), 2148);
        ensure_walkable_tile(&mut world.map, Position::new(100, 101, 7), 2148);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.target_distance = 4;
            m.is_hostile = false;
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::DistDance
        );

        let mut got_go = false;
        for _ in 0..50 {
            if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
                m.base.walk_queue.clear();
                m.base.todo.queue.clear();
                m.base.has_follow_path = false;
                m.base.next_wakeup = None;
            }
            world.monster_idle_stimulus(monster);
            if world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go())
            {
                got_go = true;
                break;
            }
        }

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(got_go, "dist_dance must enqueue Go");
        assert!(todo.has_wait(), "dist_dance must enqueue Wait after Go");
    }

    #[test]
    fn test_772_get_next_step_no_roam_on_beat_loop() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new((100 + dx) as u16, (100 + dy) as u16, 7),
                    2148,
                );
            }
        }
        let monster = insert_monster(&mut world, "Wolf", pos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }

        let now = std::time::Instant::now();
        assert_eq!(
            world.monster_next_walk_step(monster, now),
            None,
            "772 getNextStep must not pick roam step inline"
        );
    }

    #[test]
    fn test_772_attack_from_idle_queue() {
        use tfs_rust_common::enums::ZoneType;

        use crate::tile::{flags as tilestate, Tile, TileBody};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        for (x, y) in [(99, 100), (100, 99), (100, 101)] {
            world.map.insert_tile(
                Position::new(x, y, 7),
                Tile::Normal(TileBody {
                    ground: Some(2148),
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_attack()),
            "hostile melee at cheb==1 must enqueue Attack without spell-range canUseAttack"
        );
    }

    /// P0-2 — change-target ticks advance on `ProcessCreatures` only, not each idle drain.
    #[test]
    fn test_772_change_target_only_on_process_creatures() {
        use crate::creature_think::EVENT_CREATURE_THINK_INTERVAL_MS;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        for x in 100..=105u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let config = MonsterAiConfig {
            change_target_speed: 4_000,
            change_target_chance: 100,
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
            m.target_change_ticks = 0;
            m.target_change_cooldown = 0;
        }

        for _ in 0..5 {
            world.monster_idle_stimulus(monster);
        }
        let ticks_after_idle = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m.target_change_ticks,
            _ => 0,
        };
        assert_eq!(
            ticks_after_idle, 0,
            "idle drain must not advance change-target ticks on 772"
        );

        world.add_creature_think_check(monster);
        world.process_creatures_772();
        let ticks_after_think = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m.target_change_ticks,
            _ => 0,
        };
        assert_eq!(
            ticks_after_think, EVENT_CREATURE_THINK_INTERVAL_MS,
            "ProcessCreatures must advance change-target once per ~1 Hz think"
        );
    }

    /// P0-3 — melee stick-fight enqueues Attack without trailing 1 s Wait.
    #[test]
    fn test_772_melee_stick_fight_no_wait_after_attack() {
        use tfs_rust_common::enums::ZoneType;

        use crate::creature_todo::{CreatureAction, MONSTER_IDLE_WAIT_MS};
        use crate::tile::{flags as tilestate, Tile, TileBody};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        for (x, y) in [(99, 100), (100, 99), (100, 101)] {
            world.map.insert_tile(
                Position::new(x, y, 7),
                Tile::Normal(TileBody {
                    ground: Some(2148),
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        world.monster_idle_stimulus(monster);

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(todo.has_attack(), "melee stick-fight must enqueue Attack");
        assert!(
            !todo.queue.iter().any(|a| {
                matches!(a, CreatureAction::Wait { delay_ms } if *delay_ms == MONSTER_IDLE_WAIT_MS)
            }),
            "melee stick-fight must not enqueue trailing 1 s Wait after Attack"
        );
    }

    #[test]
    fn test_772_think_skips_creature_on_attacking() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.attack_target = Some(player);
            m.base.follow_target = Some(player);
        }
        world.add_creature_think_check(monster);

        world.process_creatures_772();

        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_attack()),
            "772 ~1 Hz think must not enqueue Attack — idle todo path owns combat tail"
        );
    }

    fn e1_melee_target_setup(
        world: &mut GameWorld,
        melee_skill: i32,
    ) -> (CreatureId, CreatureId) {
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        let player = insert_player(world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = melee_skill;
            m.is_hostile = true;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }
        (monster, player)
    }

    #[test]
    fn test_e1_melee_monster_enters_attacking_on_idle() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e1_melee_target_setup(&mut world, 15);

        world.monster_idle_stimulus(monster);

        assert_eq!(
            world
                .creatures
                .get(monster)
                .and_then(|k| match k {
                    CreatureKind::Monster(m) => Some(m.state),
                    _ => None,
                }),
            Some(MonsterState::Attacking),
            "hostile melee with target must enter Attacking on idle drain"
        );
    }

    #[test]
    fn test_e1_idle_reset_reasserts_attacking_each_tick() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e1_melee_target_setup(&mut world, 15);

        for tick in 0..2 {
            if tick > 0 {
                if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
                    m.base.todo.queue.clear();
                    m.base.walk_queue.clear();
                    m.base.next_wakeup = None;
                }
            }
            world.monster_idle_stimulus(monster);
            assert_eq!(
                world
                    .creatures
                    .get(monster)
                    .and_then(|k| match k {
                        CreatureKind::Monster(m) => Some(m.state),
                        _ => None,
                    }),
                Some(MonsterState::Attacking),
                "reset→Idle then walk must re-set Attacking when walk section runs"
            );
        }
    }

    #[test]
    fn test_e1_under_attack_promoted_to_attacking_in_walk_section() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e1_melee_target_setup(&mut world, 15);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::UnderAttack;
        }

        world.monster_idle_stimulus(monster);

        assert_eq!(
            world
                .creatures
                .get(monster)
                .and_then(|k| match k {
                    CreatureKind::Monster(m) => Some(m.state),
                    _ => None,
                }),
            Some(MonsterState::Attacking),
            "top reset preserves UnderAttack; walk prelude promotes to Attacking — crnonpl.cc:2705"
        );
    }

    #[test]
    fn test_e1_no_attacking_without_melee_skill() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e1_melee_target_setup(&mut world, 0);

        world.monster_idle_stimulus(monster);

        assert_eq!(
            world
                .creatures
                .get(monster)
                .and_then(|k| match k {
                    CreatureKind::Monster(m) => Some(m.state),
                    _ => None,
                }),
            Some(MonsterState::Idle),
            "melee_skill==0 must not enter Attacking"
        );
    }

    #[test]
    fn test_e1_panic_blocks_attacking_set() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e1_melee_target_setup(&mut world, 15);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::Panic;
        }

        world.monster_idle_stimulus(monster);

        assert_eq!(
            world
                .creatures
                .get(monster)
                .and_then(|k| match k {
                    CreatureKind::Monster(m) => Some(m.state),
                    _ => None,
                }),
            Some(MonsterState::Panic),
            "PANIC must block Attacking transition"
        );
    }

    fn e3_melee_target_at_cheb2(world: &mut GameWorld, melee_skill: i32) -> (CreatureId, CreatureId) {
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }
        let player = insert_player(world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = melee_skill;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
            m.base.todo.queue.clear();
        }
        (monster, player)
    }

    #[test]
    fn test_e3_attacking_skips_idle_melee_chase() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e3_melee_target_at_cheb2(&mut world, 15);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::Hold,
            "ATTACKING at cheb==2 must not use idle MeleeChase"
        );
    }

    #[test]
    fn test_e3_attack_path_enqueues_close_chase_at_cheb2() {
        let mut world = beat_driven_test_world();
        world.server_ms = 1000;
        let (monster, _player) = e3_melee_target_at_cheb2(&mut world, 15);

        world.monster_idle_stimulus(monster);

        let m = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m,
            _ => panic!("expected monster"),
        };
        assert_eq!(
            m.chase_mode,
            MonsterChaseMode::Close,
            "melee ATTACKING must set CHASE_MODE_CLOSE"
        );
        assert!(
            !m.base.walk_queue.is_empty(),
            "attack-path CanToDoAttack must populate walk_queue at cheb==2"
        );
        let todo = &m.base.todo;
        assert!(todo.has_go(), "attack tail must enqueue Go before Attack");
        assert!(todo.has_attack(), "attack tail must enqueue Attack");
        assert!(
            !todo.queue.iter().any(|a| matches!(a, CreatureAction::Wait { delay_ms: 100 })),
            "fist ToDoAttack skips Wait(100) when GetDistance()==1 (cract.cc:1327)"
        );
        let go_idx = todo
            .queue
            .iter()
            .position(|a| matches!(a, CreatureAction::Go))
            .expect("Go in queue");
        let attack_idx = todo
            .queue
            .iter()
            .position(|a| matches!(a, CreatureAction::Attack))
            .expect("Attack in queue");
        assert!(
            go_idx < attack_idx,
            "ToDoAttack order: Go before Attack (cract.cc:1325-1334)"
        );
    }

    fn e2_adjacent_combat_setup(
        world: &mut GameWorld,
        melee_skill: i32,
        melee_attack: i32,
    ) -> (CreatureId, CreatureId) {
        let (monster, player) = e1_melee_target_setup(world, melee_skill);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.melee_attack = melee_attack;
        }
        (monster, player)
    }

    fn e2_run_attack_todo(world: &mut GameWorld, monster: CreatureId) {
        world.enqueue_creature_attack(monster);
        world.schedule_immediate_todo_wakeup(monster);
        world.run_monster_todo_execute(monster);
    }

    fn e2_drain_until_idle(world: &mut GameWorld, monster: CreatureId) {
        for _ in 0..30 {
            let wakeup = world
                .creatures
                .get(monster)
                .and_then(|k| k.base().next_wakeup);
            let Some(wu) = wakeup else {
                break;
            };
            world.server_ms = wu;
            while world
                .todo_queue
                .peek()
                .is_some_and(|e| e.execution_time <= world.server_ms)
            {
                world.drain_todo_queue();
            }
        }
    }

    #[test]
    fn test_e2_melee_damage_and_damage_map() {
        use crate::max_melee_damage_monster;

        let mut world = beat_driven_test_world();
        world.server_ms = 1000;
        let (monster, player) = e2_adjacent_combat_setup(&mut world, 15, 7);
        let hp_before = world.creatures.get(player).unwrap().base().health;

        e2_run_attack_todo(&mut world, monster);

        let hp_after = world.creatures.get(player).unwrap().base().health;
        assert!(hp_after < hp_before, "adjacent melee must reduce target HP");
        let dealt = (hp_before - hp_after) as u64;
        assert!(
            dealt <= max_melee_damage_monster(15, 7) as u64,
            "damage must not exceed max roll"
        );
        assert_eq!(
            world
                .creatures
                .get(player)
                .unwrap()
                .base()
                .damage_map
                .get(&monster)
                .copied(),
            Some(dealt),
            "damage_map must attribute dealt HP to attacker"
        );
    }

    #[test]
    fn test_e2_attack_cadence_2000ms() {
        let mut world = beat_driven_test_world();
        world.server_ms = 5000;
        let (monster, player) = e2_adjacent_combat_setup(&mut world, 15, 7);

        e2_run_attack_todo(&mut world, monster);
        let hp_after_first = world.creatures.get(player).unwrap().base().health;
        let earliest = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .earliest_attack_ms;
        assert_eq!(earliest, 5000 + 2000, "CloseAttack must DelayAttack(2000)");

        world.server_ms = earliest - 1;
        e2_run_attack_todo(&mut world, monster);
        assert_eq!(
            world.creatures.get(player).unwrap().base().health,
            hp_after_first,
            "attack must not land before cadence elapses"
        );

        world.server_ms = earliest;
        e2_drain_until_idle(&mut world, monster);
        assert!(
            world.creatures.get(player).unwrap().base().health < hp_after_first,
            "second hit must land after 2000 ms cadence"
        );
    }

    #[test]
    fn test_e2_melee_adjacent_enqueues_attack_without_wait() {
        use crate::creature::monster_weapon_attack_distance;

        let mut world = beat_driven_test_world();
        let (monster, _player) = e2_adjacent_combat_setup(&mut world, 15, 7);
        let (melee_skill, has_ranged) = world
            .creatures
            .get(monster)
            .map(|k| match k {
                CreatureKind::Monster(m) => (m.melee_skill, m.spells.iter().any(|s| s.range > 1)),
                _ => (0, false),
            })
            .unwrap();
        assert_eq!(monster_weapon_attack_distance(melee_skill, has_ranged), 1);

        assert!(world.enqueue_creature_attack(monster));

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert_eq!(todo.queue.len(), 1);
        assert!(matches!(todo.queue[0], CreatureAction::Attack));
    }

    #[test]
    fn test_e2_wait_100_before_attack_when_weapon_range_not_close() {
        use crate::creature::monster_weapon_attack_distance;

        assert_eq!(monster_weapon_attack_distance(0, true), 3);
        assert_eq!(monster_weapon_attack_distance(15, true), 1);

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        let spell = MonsterSpell {
            delay: 4,
            range: 5,
            min_cycle: 6,
            shape: SpellShape::Victim,
            impact: SpellImpact::Condition {
                condition: ConditionType::Poison,
                cycle: 20,
                min_cycle: 6,
            },
            shoot_effect: None,
            area_effect: None,
        };
        let mut cfg = MonsterAiConfig::default();
        cfg.melee_skill = 0;
        cfg.spells = vec![spell];
        let monster = insert_monster_with_config(&mut world, "Cobra", mpos, 200, cfg);

        if monster_weapon_attack_distance(0, true) != 1 {
            world.enqueue_creature_wait(monster, 100);
        }
        world.enqueue_creature_attack(monster);

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert_eq!(todo.queue.len(), 2);
        assert!(matches!(todo.queue[0], CreatureAction::Wait { delay_ms: 100 }));
        assert!(matches!(todo.queue[1], CreatureAction::Attack));
    }

    #[test]
    fn test_e2_attack_deferred_until_cadence() {
        let mut world = beat_driven_test_world();
        world.server_ms = 2000;
        let (monster, player) = e2_adjacent_combat_setup(&mut world, 15, 7);

        e2_run_attack_todo(&mut world, monster);
        let hp_after_first = world.creatures.get(player).unwrap().base().health;
        assert!(
            hp_after_first < 100,
            "first attack must deal damage"
        );

        world.enqueue_creature_attack(monster);
        world.schedule_immediate_todo_wakeup(monster);
        world.run_monster_todo_execute(monster);
        assert_eq!(
            world.creatures.get(player).unwrap().base().health,
            hp_after_first,
            "immediate re-attack must defer without damage"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .next_wakeup
                .is_some(),
            "deferred attack must schedule a wakeup"
        );
    }

    /// Regression: adjacent melee must not freeze after first hit while target stands still.
    ///
    /// C++ always enqueues `ToDoAttack` at the idle tail; `TDAttack` arms the cadence wakeup.
    #[test]
    fn test_e2_melee_adjacent_does_not_freeze_after_first_strike() {
        let mut world = beat_driven_test_world();
        world.server_ms = 5000;
        let (monster, player) = e2_adjacent_combat_setup(&mut world, 15, 7);

        e2_run_attack_todo(&mut world, monster);
        let hp_after_first = world.creatures.get(player).unwrap().base().health;
        assert!(
            hp_after_first < 100,
            "first attack must deal damage"
        );

        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            base.todo.has_attack() || base.next_wakeup.is_some(),
            "adjacent melee on cooldown must keep Attack or cadence wakeup armed (not freeze)"
        );

        let earliest = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .earliest_attack_ms;
        e2_drain_until_idle(&mut world, monster);
        assert!(
            world.creatures.get(player).unwrap().base().health < hp_after_first,
            "second hit must land after cadence without target moving"
        );
        assert_eq!(
            earliest,
            5000 + 2000,
            "cadence must remain DelayAttack(2000) after idle re-enqueue"
        );
    }

    /// ATTACKING + empty queue: target kiting one tile away must re-arm close chase immediately.
    #[test]
    fn test_chase_freeze_attacking_repaths_on_target_kite() {
        let mut world = beat_driven_test_world();
        world.server_ms = 5000;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let ppos_kited = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 2148);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
            m.base.walk_queue.clear();
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
            m.base.has_follow_path = false;
            m.base.earliest_attack_ms = world.server_ms + 2000;
        }

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_kited;
        }
        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_kited, player);
        world.monster_dispatch_creature_move(player, ppos, ppos_kited);

        assert_eq!(
            world.creatures.get(monster).unwrap().base().follow_target,
            Some(player),
            "kite must not drop follow during close chase"
        );
        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            base.todo.has_go() || !base.walk_queue.is_empty(),
            "empty-queue ATTACKING must re-arm close chase when target kites (not wait 2s cadence)"
        );
    }

    /// Attack-path `TShortway` fail must NOWAY-clear target and not enqueue undeliverable Attack.
    #[test]
    fn test_chase_freeze_attack_path_noway_clears_target() {
        use crate::map::Map;
        use crate::test_world::support::{beat_driven_world, insert_monster_with_config};
        use crate::tile::{Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;

        fn sight_open_unwalkable(map: &mut Map, pos: Position) {
            map.insert_tile(
                pos,
                Tile::Normal(TileBody {
                    ground: None,
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: 0,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let mut world = beat_driven_world();
        let mpos = Position::new(100, 100, 7);
        let mid = Position::new(101, 100, 7);
        let ppos = Position::new(102, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        sight_open_unwalkable(&mut world.map, mid);
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
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
        }

        assert_eq!(
            world.monster_combat_enqueue_close_chase_go(monster),
            MonsterCombatCloseChaseEnqueue::Noway,
            "attack-path close chase must NOWAY when TShortway fails"
        );
        let base = world.creatures.get(monster).unwrap().base();
        assert_eq!(base.follow_target, None, "NOWAY must clear chase target");
        assert_eq!(base.attack_target, None);
        assert!(
            !base.todo.has_attack(),
            "NOWAY must not leave undeliverable Attack"
        );

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.chase_mode = MonsterChaseMode::Close;
            m.base.todo.queue.clear();
        }
        assert!(
            matches!(
                world.monster_enqueue_todo_attack_actions(monster),
                MonsterEnqueueAttackResult::Noway | MonsterEnqueueAttackResult::Failed,
            ),
            "blocked chase must not enqueue Attack"
        );
        assert!(
            !world.creatures.get(monster).unwrap().base().todo.has_attack(),
            "blocked chase must not leave Attack on the todo queue"
        );
    }

    /// Blocked mid-batch step must idle-repath instead of re-arming stale walk_queue dirs.
    #[test]
    fn test_chase_freeze_force_update_clears_stale_walk_batch() {
        use std::collections::VecDeque;
        use tfs_rust_common::enums::Direction;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 2148);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = 15;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.walk_queue = VecDeque::from([Direction::East, Direction::East]);
            m.base.force_update_follow_path = true;
            m.base.todo.queue.clear();
        }

        world.finish_creature_todo_execute(monster);

        assert!(
            world.creatures.get(monster).is_some_and(|k| {
                let base = k.base();
                base.walk_queue.is_empty() || base.todo.has_go()
            }),
            "force_update after blocked step must clear stale batch or idle-repath"
        );
    }

    #[test]
    fn test_e3_attack_enqueue_succeeds_when_close_go_already_queued() {
        use std::collections::VecDeque;
        use tfs_rust_common::enums::Direction;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 2148);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = 15;
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.walk_queue = VecDeque::from([Direction::East]);
            m.base.todo.queue.push_back(CreatureAction::Go);
        }

        assert_eq!(
            world.monster_enqueue_todo_attack_actions(monster),
            MonsterEnqueueAttackResult::Enqueued,
            "mid-batch close Go must not fail attack enqueue"
        );
        assert!(
            world.creatures.get(monster).unwrap().base().todo.has_attack(),
            "Attack must append when close Go already queued"
        );
    }

    #[test]
    fn test_chase_blocked_follower_rewakes_when_blocker_moves() {
        let mut world = beat_driven_test_world();
        let bpos = Position::new(100, 100, 7);
        let apos = Position::new(101, 100, 7);
        let ppos = Position::new(103, 100, 7);
        let apos_moved = Position::new(101, 101, 7);
        for pos in [bpos, apos, apos_moved, ppos] {
            ensure_walkable_tile(&mut world.map, pos, 2148);
        }
        ensure_walkable_tile(&mut world.map, Position::new(100, 101, 7), 2148);
        ensure_walkable_tile(&mut world.map, Position::new(102, 100, 7), 2148);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let blocker = insert_monster(&mut world, "Rat", apos, 200);
        let follower = insert_monster(&mut world, "Rat", bpos, 200);
        for id in [blocker, follower] {
            if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(id) {
                m.is_idle = false;
                m.is_hostile = true;
                m.melee_skill = 15;
                m.opponent_ids.push(player);
                m.base.follow_target = Some(player);
                m.base.attack_target = Some(player);
                m.state = MonsterState::Attacking;
                m.chase_mode = MonsterChaseMode::Close;
            }
        }
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(follower) {
            m.base.todo.queue.clear();
            m.base.walk_queue.clear();
            m.base.next_wakeup = None;
            m.base.has_follow_path = false;
        }

        world.map.register_creature_at(apos, blocker);
        world.map.unregister_creature_at(apos, blocker);
        world.map.register_creature_at(apos_moved, blocker);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(blocker) {
            m.base.position = apos_moved;
        }
        world.monster_dispatch_creature_move(blocker, apos, apos_moved);

        let base = world.creatures.get(follower).unwrap().base();
        assert!(
            base.todo.has_go()
                || base.next_wakeup.is_some()
                || !base.walk_queue.is_empty(),
            "stalled follower must re-arm chase when a blocking monster moves"
        );
    }
}
