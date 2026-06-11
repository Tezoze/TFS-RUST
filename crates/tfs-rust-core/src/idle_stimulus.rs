//! 772 drain-triggered idle AI — `IdleStimulus` on ToDo queue drain.
//!
//! - `TCreature::IdleStimulus` — virtual dispatch after `Execute` drains the action list.
//! - `TMonster::IdleStimulus` — `crnonpl.cc:2386`.
//!
//! Profile-gated via `GameWorld::beat_driven_loop` (same flag as P2 ToDo walk).

use std::time::Instant;

use tfs_rust_common::Position;

use crate::chase_debug;
use crate::creature::CreatureKind;
use crate::creature_think::EVENT_CREATURE_THINK_INTERVAL_MS;
use crate::creature_todo::{
    trace_creature_todo, CreatureAction, MONSTER_IDLE_WAIT_MS,
};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::monster_ai::{
    chebyshev, manhattan, monster_idle_chase_step_budget, monster_master_follow_in_wait_band,
    MonsterIdleChaseRepathOutcome,
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

        self.monster_on_think_target(cid, EVENT_CREATURE_THINK_INTERVAL_MS);
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

        if self.creature_todo_queue_empty(cid) {
            self.monster_idle_maybe_enqueue_attack(cid);
        }
        if self.creature_todo_queue_empty(cid) {
            self.monster_idle_maybe_enqueue_at_goal_wait(cid);
        }
    }

    /// 772 melee tail uses cheb band; TFS `canUseAttack` is spell-range only (`monster.cpp` ~876).
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
        if target_distance <= 1 {
            return dist <= 1;
        }
        self.monster_can_use_attack(cid, pos, attack_id)
    }

    /// C++ idle combat tail — `Rotate` + `ToDoAttack` (`crnonpl.cc:2795`); stub until E2/E5.
    fn monster_idle_maybe_enqueue_attack(&mut self, cid: CreatureId) {
        let (attack_id, pos) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => {
                if m.is_fleeing() {
                    return;
                }
                let Some(attack_id) = m.base.attack_target else {
                    return;
                };
                (attack_id, m.base.position)
            }
            _ => return,
        };
        if !self.creatures.contains_key(attack_id) {
            return;
        }
        let target_pos = match self.creatures.get(attack_id) {
            Some(k) => k.position(),
            None => return,
        };
        if !self.map.is_sight_clear(pos, target_pos) {
            return;
        }
        if !self.monster_idle_can_enqueue_attack(cid, pos, attack_id, target_pos) {
            return;
        }
        if self.enqueue_creature_attack(cid) {
            trace_creature_todo(self, cid, "idle_enqueue_attack");
            self.schedule_immediate_todo_wakeup(cid);
        }
    }

    /// `ToDoWait(1000)` when at-goal dance/attack could not arm (`crnonpl.cc:2791`).
    fn monster_idle_maybe_enqueue_at_goal_wait(&mut self, cid: CreatureId) {
        if !self.beat_driven_loop {
            return;
        }
        let branch = self.monster_idle_classify_walk_branch(cid);
        if !matches!(
            branch,
            MonsterIdleWalkBranch::MeleeDance | MonsterIdleWalkBranch::DistDance
        ) {
            return;
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
            MonsterIdleWalkBranch::MeleeChase
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
                trace_creature_todo(self, cid, "execute_attack");
                self.monster_do_attacking(cid, EVENT_CREATURE_THINK_INTERVAL_MS);
                trace_creature_todo(self, cid, "execute_attack_done");
                TodoExecuteKind::Attack
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
            // Re-arm `Go` — popped on execute; reference drains one step per `Execute` call.
            let _ = self.enqueue_creature_go(cid);
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
            Some(TodoExecuteKind::Wait) => {}
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use tfs_rust_common::enums::Direction;
    use tfs_rust_common::Position;

    use crate::creature::CreatureKind;
    use crate::creature::MonsterAiConfig;
    use crate::creature_think::EVENT_CREATURE_THINK_INTERVAL_MS;
    use crate::creature_todo::{CreatureAction, MONSTER_IDLE_WAIT_MS};
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

        let melee_monster = insert_monster(&mut world, "Rat", mpos, 200);
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
            MonsterIdleWalkBranch::MeleeDance
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

    /// A2 — melee chase at cheb==2 queues one `must:1` shortway step, not a 3-hop batch.
    #[test]
    fn test_772_melee_chase_cheb2_one_step() {
        use crate::monster_ai::{monster_idle_chase_step_budget, MonsterIdleChaseRepathOutcome};

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
        assert_eq!((max_steps, must_reach), (1, true));

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
            "melee chase at cheb==2 must queue exactly one step"
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
            MonsterIdleWalkBranch::MeleeDance
        );

        world.monster_idle_stimulus(monster);

        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go()),
            "772 idle Hold after blocked dance must not poll via monster_should_keep_dance_walk_alive"
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

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
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

        world.monster_idle_stimulus(monster);

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(todo.has_go(), "dist_dance must enqueue Go");
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
}
