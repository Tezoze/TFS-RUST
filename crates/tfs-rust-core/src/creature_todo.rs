//! Per-creature ToDo action queue for 772 idle-driven AI.
//!
//! - 772 `TCreature::Execute` / `ToDoList` — `cract.cc:728`.
//! - `ToDoWait` — `cract.cc:1008`; idle pacing — `crnonpl.cc:2693–2807,2852`.
//! - Global wakeup heap: [`ToDoQueue`](crate::todo_queue::ToDoQueue) + `next_wakeup`.

use std::collections::VecDeque;

use crate::chase_debug;
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::monster_ai::{chebyshev, monster_idle_chase_step_budget};
use crate::pathfinding::CHASE_PATH_MAX_STEPS;
use tfs_rust_common::Position;

/// C++ `ToDoWait(1000)` after roam, dist_dance, dist_flee fail, master dist 2–3 (`crnonpl.cc`).
pub const MONSTER_IDLE_WAIT_MS: u64 = 1000;

/// Snapshot per-creature + global ToDo state — enable with
/// `RUST_LOG=tfs_rust_core::creature_todo=debug,tfs_rust_core::idle_stimulus=debug`.
pub(crate) fn trace_creature_todo(world: &GameWorld, cid: CreatureId, event: &str) {
    let Some(k) = world.creatures.get(cid) else {
        tracing::debug!(event, ?cid, "idle_todo: creature gone");
        return;
    };
    let base = k.base();
    let name = base.name.as_str();
    let action_queue_len = base.todo.queue.len();
    let action_locked = base.todo.locked;
    let walk_queue_len = base.walk_queue.len();
    let follow = base
        .follow_target
        .map(|id| format!("{id:?}"))
        .unwrap_or_else(|| "-".into());
    tracing::debug!(
        event,
        creature = name,
        ?cid,
        server_ms = world.server_ms,
        action_queue_len,
        action_locked,
        walk_queue_len,
        next_wakeup = ?base.next_wakeup,
        heap_len = world.todo_queue.len(),
        follow,
        beat_driven = world.beat_driven_loop,
        "idle_todo"
    );
    if chase_debug::chase_path_debug_enabled() {
        chase_debug::log_todo_label(
            world.chase_trace_tick(),
            cid,
            name,
            event,
            action_queue_len,
            action_locked,
            walk_queue_len,
        );
    }
}

/// 772 ToDo action kinds — Rust enum instead of C++ `void*` task list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureAction {
    /// `TDGo` — execute one walk step from `listWalkDir`.
    Go,
    /// `TDWait` — logical delay before the next action (`cract.cc:1008`).
    Wait { delay_ms: u64 },
    /// `TDAttack` — melee/ranged strike (`cract.cc:1325`); execute stub until Phase E2.
    Attack,
}

/// Per-creature action queue paired with the global wakeup heap.
#[derive(Debug, Clone, Default)]
pub struct CreatureTodo {
    pub queue: VecDeque<CreatureAction>,
    /// C++ `LockToDo` while an action is executing.
    pub locked: bool,
}

impl CreatureTodo {
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn has_go(&self) -> bool {
        self.queue.iter().any(|a| matches!(a, CreatureAction::Go))
    }

    pub fn has_wait(&self) -> bool {
        self.queue
            .iter()
            .any(|a| matches!(a, CreatureAction::Wait { .. }))
    }

    pub fn has_attack(&self) -> bool {
        self.queue
            .iter()
            .any(|a| matches!(a, CreatureAction::Attack))
    }
}

impl GameWorld {
    pub(crate) fn creature_todo_queue_empty(&self, cid: CreatureId) -> bool {
        self.creatures
            .get(cid)
            .is_some_and(|k| k.base().todo.is_empty())
    }

    /// Push `Go` if not already queued — avoids duplicate action storms.
    pub(crate) fn enqueue_creature_go(&mut self, cid: CreatureId) -> bool {
        self.enqueue_creature_go_at(cid, false)
    }

    /// Push `Go` at queue front or back when not already queued.
    ///
    /// Mid-batch chase re-arms must use `front=true` so steps drain before `Attack`
    /// (`ToDoGo` then `TDAttack` — `cract.cc:1325`).
    pub(crate) fn enqueue_creature_go_at(&mut self, cid: CreatureId, front: bool) -> bool {
        let Some(k) = self.creatures.get_mut(cid) else {
            return false;
        };
        if k.base().todo.has_go() {
            return false;
        }
        if front {
            k.base_mut().todo.queue.push_front(CreatureAction::Go);
        } else {
            k.base_mut().todo.queue.push_back(CreatureAction::Go);
        }
        tracing::debug!(
            creature = k.base().name.as_str(),
            ?cid,
            front,
            action_queue_len = k.base().todo.queue.len(),
            "idle_todo: enqueue_go"
        );
        true
    }

    /// Push `Wait` onto the action queue.
    pub(crate) fn enqueue_creature_wait(&mut self, cid: CreatureId, delay_ms: u64) -> bool {
        let Some(k) = self.creatures.get_mut(cid) else {
            return false;
        };
        k.base_mut()
            .todo
            .queue
            .push_back(CreatureAction::Wait { delay_ms });
        let name = k.base().name.clone();
        let queue_len = k.base().todo.queue.len();
        tracing::debug!(
            creature = name.as_str(),
            ?cid,
            delay_ms,
            action_queue_len = queue_len,
            "idle_todo: enqueue_wait"
        );
        if chase_debug::chase_path_debug_enabled() {
            chase_debug::log_todo_wait(
                self.chase_trace_tick(),
                cid,
                name.as_str(),
                delay_ms,
                "enqueue",
            );
        }
        true
    }

    /// Push `Attack` if not already queued.
    pub(crate) fn enqueue_creature_attack(&mut self, cid: CreatureId) -> bool {
        let Some(k) = self.creatures.get_mut(cid) else {
            return false;
        };
        if k.base().todo.has_attack() {
            return false;
        }
        k.base_mut().todo.queue.push_back(CreatureAction::Attack);
        tracing::debug!(
            creature = k.base().name.as_str(),
            ?cid,
            action_queue_len = k.base().todo.queue.len(),
            "idle_todo: enqueue_attack"
        );
        true
    }

    /// Schedule the next action wakeup after `delay_ms` logical time.
    pub(crate) fn todo_start_from_action(
        &mut self,
        cid: CreatureId,
        delay_ms: u64,
        tie_policy: crate::todo_queue::WakeupTiePolicy,
    ) {
        if delay_ms == 0 {
            self.schedule_creature_wakeup(cid, self.server_ms, tie_policy);
        } else {
            self.schedule_creature_wakeup(cid, self.server_ms.saturating_add(delay_ms), tie_policy);
        }
    }

    /// C++ `TDAttack` branch in `ToDoStart` — `cract.cc:909-918`.
    pub(crate) fn todo_attack_delay_ms(&self, cid: CreatureId) -> u64 {
        let earliest_spell_ms = 0u64;
        self.creatures
            .get(cid)
            .map(|k| {
                let base = k.base();
                base.earliest_attack_ms
                    .max(earliest_spell_ms)
                    .saturating_sub(self.server_ms)
            })
            .unwrap_or(0)
    }

    /// Enqueue Wait and arm an immediate execute wakeup (`cract.cc` `ToDoStart`).
    pub(crate) fn idle_enqueue_wait_and_start(&mut self, cid: CreatureId, delay_ms: u64) {
        if !self.enqueue_creature_wait(cid, delay_ms) {
            return;
        }
        trace_creature_todo(self, cid, "idle_enqueue_wait");
        self.schedule_immediate_todo_wakeup(cid);
    }

    fn idle_todo_go_trace_contract(
        &self,
        cid: CreatureId,
        todo_via: Option<&str>,
    ) -> Option<(Position, Position, bool, i32)> {
        let k = self.creatures.get(cid)?;
        let from = k.position();
        let follow_id = k.base().follow_target;
        let walk_step_dest = k.base().walk_queue.front().map(|dir| from.offset(*dir));
        let is_dance = todo_via == Some("idle_dance");
        let is_flee = todo_via == Some("idle_flee");

        let (dest, must_reach, max_steps) = if is_dance {
            (walk_step_dest.unwrap_or(from), true, i32::MAX)
        } else if is_flee {
            // 772 `SearchFlightField` follow-up `ToDoGo(must:true, INT_MAX)` (`crnonpl.cc:2680,2762`).
            (walk_step_dest.unwrap_or(from), true, i32::MAX)
        } else if let Some(follow_id) = follow_id {
            let dest = self
                .creatures
                .get(follow_id)
                .map(|t| t.position())
                .unwrap_or(from);
            let max_steps = match self.creatures.get(cid) {
                Some(CreatureKind::Monster(m)) => {
                    let target_distance = self.monster_effective_target_distance(m.target_distance);
                    let cheb = chebyshev(from, dest);
                    let uses_dist_branch = self.monster_idle_uses_dist_branch(
                        cid,
                        from,
                        follow_id,
                        target_distance,
                    );
                    let is_dist_chase = uses_dist_branch && cheb > target_distance;
                    let is_melee_chase = !uses_dist_branch && cheb > 1;
                    let (budget, _) = monster_idle_chase_step_budget(
                        is_melee_chase,
                        is_dist_chase,
                        cheb,
                        target_distance,
                    );
                    budget as i32
                }
                _ => CHASE_PATH_MAX_STEPS as i32,
            };
            (dest, false, max_steps)
        } else {
            (from, false, CHASE_PATH_MAX_STEPS as i32)
        };

        Some((from, dest, must_reach, max_steps))
    }

    /// Enqueue Go (and optional trailing Wait), then schedule the Go wakeup.
    pub(crate) fn idle_enqueue_paced_go(
        &mut self,
        cid: CreatureId,
        first_step: bool,
        todo_via: Option<&str>,
        wait_after_ms: Option<u64>,
    ) {
        if !self.enqueue_creature_go(cid)
            && !self
                .creatures
                .get(cid)
                .is_some_and(|k| k.base().todo.has_go())
        {
            return;
        }
        if let Some(ms) = wait_after_ms {
            self.enqueue_creature_wait(cid, ms);
        }
        if self.beat_driven_loop && chase_debug::chase_path_debug_enabled() {
            if let Some(k) = self.creatures.get(cid) {
                let follow_id = k.base().follow_target;
                let is_dance = todo_via == Some("idle_dance");
                let is_flee = todo_via == Some("idle_flee");

                if let Some((from, dest, must_reach, max_steps)) =
                    self.idle_todo_go_trace_contract(cid, todo_via)
                {
                    let arm = todo_via.filter(|v| *v != "roam");
                    if is_dance || is_flee || follow_id.is_some() {
                        chase_debug::log_todo_go_aligned(
                            self.chase_trace_tick(),
                            cid,
                            k.base().name.as_str(),
                            from,
                            dest,
                            must_reach,
                            max_steps,
                            arm,
                        );
                    } else if todo_via == Some("roam") {
                        chase_debug::log_todo_go(
                            self.chase_trace_tick(),
                            cid,
                            k.base().name.as_str(),
                            "enter",
                            from,
                            from,
                            false,
                            1,
                            Some("roam"),
                        );
                    }
                }
            }
        }
        trace_creature_todo(self, cid, "idle_enqueue_go");
        if self.beat_driven_loop {
            let _ = self.todo_start_go_delay(cid, first_step);
        } else if self.todo_start_go_delay(cid, first_step) {
            self.schedule_immediate_todo_wakeup(cid);
        }
    }

    /// Enqueue Go and schedule its wakeup when idle decides movement is needed.
    pub(crate) fn idle_enqueue_go_and_start(
        &mut self,
        cid: CreatureId,
        first_step: bool,
        todo_via: Option<&str>,
    ) {
        self.idle_enqueue_paced_go(cid, first_step, todo_via, None);
    }

    /// Arm the next todo step on the heap without synchronous re-entry (avoids stack overflow).
    pub(crate) fn schedule_immediate_todo_wakeup(&mut self, cid: CreatureId) {
        self.schedule_creature_wakeup(
            cid,
            self.server_ms.saturating_add(1),
            self.harness_go_wakeup_tie_policy(cid),
        );
    }

    /// C++ `TCreature::ToDoYield` — `cract.cc:1001` (`ToDoWait(0)` + `ToDoStart` when not `LockToDo`).
    pub(crate) fn creature_todo_yield(&mut self, cid: CreatureId) {
        if !self.beat_driven_loop {
            return;
        }
        let locked = self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().todo.locked);
        if locked {
            return;
        }
        if !self.enqueue_creature_wait(cid, 0) {
            return;
        }
        trace_creature_todo(self, cid, "todo_yield");
        // C++ `ToDoWait(0)` — wakeup at `ServerMilliseconds`, not +1 (`cract.cc:1008`).
        self.todo_start_from_action(
            cid,
            0,
            crate::todo_queue::WakeupTiePolicy::HarnessAppearIdle,
        );
    }

    pub(crate) fn creature_uses_todo_execute(&self, cid: CreatureId) -> bool {
        self.beat_driven_loop
            && self
                .creatures
                .get(cid)
                .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
    }
}

#[cfg(test)]
mod tests {
    use tfs_rust_common::Position;

    use crate::creature::CreatureKind;
    use crate::creature_todo::{CreatureAction, MONSTER_IDLE_WAIT_MS};
    use crate::test_world::support::{
        dist_idle_monster_config, beat_driven_test_world, ensure_walkable_tile,
        insert_monster, insert_monster_with_config, insert_player, minimal_world, test_player,
        TEST_SYNTHETIC_GROUND_WP,
    };

    fn beat_driven_test_world_at(ms: u64) -> crate::game_world::GameWorld {
        let mut world = beat_driven_test_world();
        world.server_ms = ms;
        world
    }

    #[test]
    fn wait_action_queues_go_then_wait() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Rat", pos, 200);

        world.idle_enqueue_paced_go(monster, true, Some("roam"), Some(MONSTER_IDLE_WAIT_MS));

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert_eq!(todo.queue.len(), 2);
        assert!(matches!(todo.queue[0], CreatureAction::Go));
        assert!(matches!(
            todo.queue[1],
            CreatureAction::Wait {
                delay_ms: MONSTER_IDLE_WAIT_MS
            }
        ));
    }

    #[test]
    fn wait_execute_schedules_wakeup_at_delay() {
        let mut world = beat_driven_test_world_at(500);
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Rat", pos, 200);

        world.idle_enqueue_wait_and_start(monster, MONSTER_IDLE_WAIT_MS);
        world.run_monster_todo_execute(monster);

        assert!(world.creatures.get(monster).unwrap().base().todo.is_empty());
        assert_eq!(
            world.creatures.get(monster).unwrap().base().next_wakeup,
            Some(500 + MONSTER_IDLE_WAIT_MS)
        );
    }

    #[test]
    fn idle_todo_go_trace_contract_uses_dist_chase_step_budget() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(106, 100, 7);
        for x in 100..=106u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Hunter",
            mpos,
            200,
            dist_idle_monster_config(4),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }

        let (_, dest, must_reach, max_steps) = world
            .idle_todo_go_trace_contract(monster, Some("idle_drain"))
            .expect("trace contract");
        assert_eq!(dest, ppos);
        assert!(!must_reach);
        assert_eq!(max_steps, 2, "dist chase at cheb=6 band=4 must log max=2");
    }

    #[test]
    fn idle_todo_go_trace_contract_uses_single_step_flee_contract() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, Position::new(101, 100, 7), 150);

        let monster = insert_monster(&mut world, "Hunter", mpos, 200);
        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut()
                .walk_queue
                .push_back(tfs_rust_common::enums::Direction::East);
        }

        let (_, dest, must_reach, max_steps) = world
            .idle_todo_go_trace_contract(monster, Some("idle_flee"))
            .expect("trace contract");
        assert_eq!(dest, Position::new(101, 100, 7));
        assert!(must_reach);
        assert_eq!(
            max_steps,
            i32::MAX,
            "flee trace must mirror ToDoGo(must=true,max=INT_MAX)"
        );
    }
}
