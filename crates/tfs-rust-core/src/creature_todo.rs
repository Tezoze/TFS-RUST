//! Per-creature ToDo action queue for CipSoft 772 idle-driven AI.
//!
//! - CipSoft `TCreature::Execute` / `ToDoList` — `cract.cc:728`.
//! - Global wakeup heap: [`ToDoQueue`](crate::todo_queue::ToDoQueue) + `next_wakeup`.
//!
//! Phase A: `Go` only. Attack/Wait deferred to Phase B/C.

use std::collections::VecDeque;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

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
}

/// CipSoft ToDo action kinds — Rust enum instead of C++ `void*` task list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureAction {
    /// `TDGo` — execute one walk step from `listWalkDir`.
    Go,
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
}

impl GameWorld {
    pub(crate) fn creature_todo_queue_empty(&self, cid: CreatureId) -> bool {
        self.creatures
            .get(cid)
            .is_some_and(|k| k.base().todo.is_empty())
    }

    /// Push `Go` if not already queued — avoids duplicate action storms.
    pub(crate) fn enqueue_creature_go(&mut self, cid: CreatureId) -> bool {
        let Some(k) = self.creatures.get_mut(cid) else {
            return false;
        };
        if k.base().todo.has_go() {
            return false;
        }
        k.base_mut().todo.queue.push_back(CreatureAction::Go);
        tracing::debug!(
            creature = k.base().name.as_str(),
            ?cid,
            action_queue_len = k.base().todo.queue.len(),
            "idle_todo: enqueue_go"
        );
        true
    }

    /// Schedule the next action wakeup after `delay_ms` logical time.
    pub(crate) fn todo_start_from_action(&mut self, cid: CreatureId, delay_ms: u64) {
        if delay_ms == 0 {
            self.schedule_creature_wakeup(cid, self.server_ms);
        } else {
            self.schedule_creature_wakeup(cid, self.server_ms.saturating_add(delay_ms));
        }
    }

    /// Enqueue Go and schedule its wakeup when idle decides movement is needed.
    pub(crate) fn idle_enqueue_go_and_start(&mut self, cid: CreatureId, first_step: bool) {
        if !self.enqueue_creature_go(cid) {
            return;
        }
        if self.todo_start_go_delay(cid, first_step) {
            self.schedule_immediate_todo_wakeup(cid);
        }
    }

    /// Arm the next todo step on the heap without synchronous re-entry (avoids stack overflow).
    pub(crate) fn schedule_immediate_todo_wakeup(&mut self, cid: CreatureId) {
        self.schedule_creature_wakeup(cid, self.server_ms.saturating_add(1));
    }

    pub(crate) fn clear_creature_todo(&mut self, cid: CreatureId) {
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().todo.queue.clear();
            k.base_mut().todo.locked = false;
        }
    }

    pub(crate) fn creature_uses_todo_execute(&self, cid: CreatureId) -> bool {
        self.beat_driven_loop
            && self
                .creatures
                .get(cid)
                .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
    }
}
