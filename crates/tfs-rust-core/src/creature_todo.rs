//! Per-creature ToDo action queue for 772 idle-driven AI.
//!
//! - 772 `TCreature::Execute` / `ToDoList` — `cract.cc:728`.
//! - `ToDoWait` — `cract.cc:1008`; idle pacing — `crnonpl.cc:2693–2807,2852`.
//! - Global wakeup heap: [`ToDoQueue`](crate::todo_queue::ToDoQueue) + `next_wakeup`.
//!
//! # Clock seam (Phase 0 — documented, 1098 side not yet implemented)
//!
//! The ToDo engine is clock-agnostic by design. The seam is two methods on [`GameWorld`]:
//!
//! - **`now_beat()`** — current logical time. On 772 this is `server_ms` (Beat-quantized:
//!   `EarliestWalkTime = now + ceil(Delay/Beat)*Beat`, `cract.cc:1530`). On 1098 it will be
//!   a pass-through `Instant`-derived ms (no Beat quantization).
//! - **`schedule_at(cid, time)`** — insert into the global wakeup heap at `time`. Currently
//!   [`GameWorld::schedule_creature_wakeup`]; the 1098 adapter will wrap the same heap with
//!   a wall-clock → logical-ms conversion.
//!
//! **772 (current):** `server_ms` + `ToDoQueue` (Beat-quantized). All references to
//! `server_ms` / `next_wakeup` / `Beat` in this module are the 772 realization of this seam.
//! **1098 (future Phase 2):** continuous `Instant` → `server_ms` mapping, same heap, no
//! Beat quantization. The API contract (`schedule_at` / `now_beat`) does not change.

use std::collections::VecDeque;

use crate::chase_debug;
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::monster_ai::{chebyshev, monster_idle_chase_step_budget};
use crate::pathfinding::CHASE_PATH_MAX_STEPS;
use crate::return_value::ReturnValue;
use crate::thing::Thing;
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

/// Resolved-at-enqueue object identity for `Use`/`Move`/`Turn` actions.
///
/// Rust analog of the decompile's `Object` handle resolved by `GetObject` in
/// `ToDoUse`/`ToDoMove`/`ToDoTurn` (`cract.cc:1260/1125/1327`). Carries enough
/// to re-locate + re-validate the item at execute time (mirroring `Obj.exists()`
/// in the `Execute` drain, `cract.cc:783-898`). **Does not cache a raw `ItemId`** —
/// the SlotMap could reuse a freed slot's generation, so the executor must re-resolve
/// via `resolve_item_at_position` + `validate_item_sprite` and return `NOTACCESSIBLE`
/// on mismatch (F8 §7 risk note).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActionObjectRef {
    /// Wire `Position` — encodes map tile / inventory slot / container slot via the
    /// `0xFFFF` / `0x40` conventions (same encoding `resolve_item_at_position` reads).
    pub pos: Position,
    /// Wire `stack_pos` / `RNum` — tile stack index or container slot index.
    pub stack_pos: u8,
    /// Expected client sprite id — re-validated at execute time via
    /// `validate_item_sprite` (mirrors `Obj.exists()` type check).
    pub sprite_id: u16,
}

/// 772 ToDo action kinds — Rust enum instead of C++ `void*` task list.
///
/// Mirrors the `TToDoEntry::Code` discriminator in `cract.cc:812-868` (`TDGo`, `TDAttack`,
/// `TDWait`, …). `TDRotate` (`cract.cc:818`) is **not** modeled as a queued action: the 772
/// idle combat tail calls `Rotate(Target)` directly (`crnonpl.cc:2872-2873`), so the 0x6B turn
/// broadcast lands in the same beat as the first `TDGo` move packet — making the turn
/// imperceptible. Enqueuing it caused a visible "turn on the spot" defect (audit: turn-on-spot).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureAction {
    /// `TDGo` — execute one walk step from `listWalkDir`.
    Go,
    /// `TDWait` — logical delay before the next action (`cract.cc:1008`).
    Wait { delay_ms: u64 },
    /// `TDAttack` — melee/ranged strike (`cract.cc:1325`); execute stub until Phase E2.
    Attack,
    /// `TDTalk` — speak text on the next ToDo execute (`cract.cc:848`, `:1367-1390`).
    /// `text` is `&'static str` to avoid allocation in the hot walk path (drunk "Hicks!").
    Talk { text: &'static str },
    /// `TDUse` — `cract.cc:1258-1296` `ToDoUse`. `obj2.is_none()` = single-object use
    /// (`CUseObject` `receiving.cc:384`); `obj2.is_some()` = two-object use
    /// (`CUseTwoObjects` `receiving.cc:430`), gated by `earliest_multiuse_server_ms` in
    /// `CalculateDelay` (`cract.cc:925`). `open_index` is the client's preferred container
    /// cid (`UseItemPayload.index`, `cract.cc:1294` "next free container index"); 0 for
    /// `UseItemEx` (which has no index byte).
    Use {
        obj1: ActionObjectRef,
        obj2: Option<ActionObjectRef>,
        open_index: u8,
    },
    /// `TDMove` — `cract.cc:1123-1172` `ToDoMove` (`CMoveObject` `receiving.cc:233`).
    /// `obj` = source, `dest` = throw destination (map/inventory/container encoded in
    /// `Position`), `count` = stack count. Maps to Rust `GamePacket::Throw` (not
    /// `MoveObject` — F8 §0.1 F5).
    Move {
        obj: ActionObjectRef,
        dest: Position,
        count: u8,
    },
    /// `TDTurn` — `cract.cc:1326-1351` `ToDoTurn` (rotate a rotatable *item*,
    /// `CTurnObject` `receiving.cc:549`). **Not** `CRotate` (player facing) — that's
    /// `GamePacket::Turn` and stays immediate (`receiving.cc:213`). No `Rotate` variant
    /// on this enum (§3 note); this `Turn` variant is the item-rotate action only.
    Turn { obj: ActionObjectRef },
}

/// Per-creature action queue paired with the global wakeup heap.
#[derive(Debug, Clone, Default)]
pub struct CreatureTodo {
    pub queue: VecDeque<CreatureAction>,
    /// C++ `LockToDo` while an action is executing.
    pub locked: bool,
    /// C++ `Stop` flag — set by `ToDoStop` when `LockToDo` is true (`cract.cc:1002-1008`).
    /// The in-flight step lands on the next beat, then `Execute` checks `Stop` and does
    /// `ToDoClear + SendSnapback` (`cract.cc:891-897`, `:797-801`). Player-only semantics.
    pub todo_stop: bool,
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

    pub fn has_talk(&self) -> bool {
        self.queue
            .iter()
            .any(|a| matches!(a, CreatureAction::Talk { .. }))
    }

    pub fn has_use(&self) -> bool {
        self.queue
            .iter()
            .any(|a| matches!(a, CreatureAction::Use { .. }))
    }

    pub fn has_move(&self) -> bool {
        self.queue
            .iter()
            .any(|a| matches!(a, CreatureAction::Move { .. }))
    }

    pub fn has_turn(&self) -> bool {
        self.queue
            .iter()
            .any(|a| matches!(a, CreatureAction::Turn { .. }))
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

    /// C++ `TCreature::ToDoTalk` — `cract.cc:1367-1390`: enqueue `TDTalk` at the back.
    pub(crate) fn enqueue_creature_talk(&mut self, cid: CreatureId, text: &'static str) -> bool {
        let Some(k) = self.creatures.get_mut(cid) else {
            return false;
        };
        k.base_mut().todo.queue.push_back(CreatureAction::Talk { text });
        tracing::debug!(
            creature = k.base().name.as_str(),
            ?cid,
            action_queue_len = k.base().todo.queue.len(),
            "idle_todo: enqueue_talk"
        );
        true
    }

    /// F8 S2 — validate that an item exists at the wire location with the expected
    /// sprite, using the **Use/Turn** resolution path (`resolve_item_at_position` +
    /// `find_tile_item_by_client_sprite` fallback for map tiles). Mirrors the
    /// decompile's `GetObject` (`cract.cc:1260/1327`) which throws `RESULT` at
    /// enqueue time if the object can't be resolved. Returns the `ActionObjectRef`
    /// (wire identity triple, unchanged) on success; `Err(NotPossible)` on failure —
    /// C++ `NOTACCESSIBLE` maps to `ReturnValue::NotPossible` (matching
    /// `walk/mod.rs:1506`'s `NOTACCESSIBLE` → `NotPossible` convention).
    fn validate_action_object_ref(
        &self,
        cid: CreatureId,
        obj: ActionObjectRef,
    ) -> Result<ActionObjectRef, ReturnValue> {
        let is_map_tile = obj.pos.x != 0xFFFF;
        // C++ ref: `Game::internalGetThing` — `resolve_item_at_position` mirrors the
        // `STACKPOS_USEITEM` path; `find_tile_item_by_client_sprite` is the sprite-id
        // fallback for map tiles (same logic as `player_use_item`, `container_ui.rs:518-525`).
        let item_id = if let Some(id) = self.resolve_item_at_position(cid, obj.pos, obj.stack_pos) {
            Some(id)
        } else if is_map_tile {
            self.find_tile_item_by_client_sprite(obj.pos, obj.sprite_id)
        } else {
            None
        };
        let Some(item_id) = item_id else {
            return Err(ReturnValue::NotPossible);
        };
        // Re-validate sprite — mirrors `Obj.exists()` type check (`cract.cc:783-898`).
        if !self.validate_item_sprite(item_id, obj.sprite_id) {
            return Err(ReturnValue::NotPossible);
        }
        Ok(obj)
    }

    /// F8 S2 — validate that a moveable item exists at the wire location, using the
    /// **Move** resolution path (`internal_get_thing_move` — `STACKPOS_MOVE`). Same
    /// `GetObject` → `throw RESULT` contract as [`validate_action_object_ref`], but
    /// the Move executor (`player_move_item`) resolves via `internal_get_thing_move`
    /// (moveable-priority stack walk), not `resolve_item_at_position`
    /// (container-priority). The builder validates with the same path its S4
    /// executor will re-validate with.
    fn validate_move_object_ref(
        &self,
        cid: CreatureId,
        obj: ActionObjectRef,
    ) -> Result<ActionObjectRef, ReturnValue> {
        let thing = self.internal_get_thing_move(cid, obj.pos, obj.stack_pos);
        let item_id = match thing {
            Some(Thing::Item(id)) => Some(id),
            _ => None,
        };
        let Some(item_id) = item_id else {
            return Err(ReturnValue::NotPossible);
        };
        if !self.validate_item_sprite(item_id, obj.sprite_id) {
            return Err(ReturnValue::NotPossible);
        }
        Ok(obj)
    }

    /// F8 S2 — `ToDoUse` builder (`cract.cc:1258-1296`). Resolves both objects now
    /// (mirroring `GetObject`'s `throw RESULT` on failure), prepends `Wait{100}`
    /// (`receiving.cc:384/430`), and enqueues `Use`. `obj2.is_none()` = single-object
    /// `CUseObject` (`receiving.cc:384`); `obj2.is_some()` = two-object
    /// `CUseTwoObjects` (`receiving.cc:430`). `open_index` is the client's preferred
    /// container cid (`UseItemPayload.index`); 0 for `UseItemEx` (no index byte).
    pub(crate) fn enqueue_player_use(
        &mut self,
        cid: CreatureId,
        obj1: ActionObjectRef,
        obj2: Option<ActionObjectRef>,
        open_index: u8,
    ) -> Result<(), ReturnValue> {
        self.validate_action_object_ref(cid, obj1)?;
        if let Some(o2) = obj2 {
            self.validate_action_object_ref(cid, o2)?;
        }
        // `ToDoWait(100)` then `ToDoUse(...)` — `receiving.cc:384/430`.
        self.enqueue_creature_wait(cid, 100);
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().todo.queue.push_back(CreatureAction::Use {
                obj1,
                obj2,
                open_index,
            });
        }
        trace_creature_todo(self, cid, "enqueue_player_use");
        Ok(())
    }

    /// F8 S2 — `ToDoMove` builder (`cract.cc:1123-1172`, `CMoveObject`
    /// `receiving.cc:233`). Resolves the source object now, enqueues `Move` with
    /// **no** `Wait` prefix (the decompile's `CMoveObject` handler calls
    /// `ToDoMove` + `ToDoStart` directly — no `ToDoWait`). `dest` is the throw
    /// destination (map/inventory/container encoded in `Position`), `count` is the
    /// stack count. Maps to Rust `GamePacket::Throw` (not `MoveObject` — F8 §0.1 F5).
    pub(crate) fn enqueue_player_move(
        &mut self,
        cid: CreatureId,
        obj: ActionObjectRef,
        dest: Position,
        count: u8,
    ) -> Result<(), ReturnValue> {
        self.validate_move_object_ref(cid, obj)?;
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().todo.queue.push_back(CreatureAction::Move {
                obj,
                dest,
                count,
            });
        }
        trace_creature_todo(self, cid, "enqueue_player_move");
        Ok(())
    }

    /// F8 S2 — `ToDoTurn` builder (`cract.cc:1326-1351`, `CTurnObject`
    /// `receiving.cc:549`). Rotates a rotatable *item* (wall torch/rope) — **not**
    /// `CRotate` (player facing, `receiving.cc:213`, already immediate). Resolves
    /// the object now, prepends `Wait{100}` (`receiving.cc:549`), enqueues `Turn`.
    /// The executor is new code (S4 — nothing exists to reuse, F8 §0.1 F2).
    pub(crate) fn enqueue_player_turn(
        &mut self,
        cid: CreatureId,
        obj: ActionObjectRef,
    ) -> Result<(), ReturnValue> {
        self.validate_action_object_ref(cid, obj)?;
        // `ToDoWait(100)` then `ToDoTurn(...)` — `receiving.cc:549`.
        self.enqueue_creature_wait(cid, 100);
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().todo.queue.push_back(CreatureAction::Turn { obj });
        }
        trace_creature_todo(self, cid, "enqueue_player_turn");
        Ok(())
    }

    /// Schedule the next action wakeup after `delay_ms` logical time.
    pub(crate) fn todo_start_from_action(&mut self, cid: CreatureId, delay_ms: u64) {
        // C++ `ToDoStart` clamps `Delay < 1` to `1` (`cract.cc:1016`), so a re-insertion is always
        // at least `server_ms + 1` — strictly future, so it cannot be re-drained in the same beat.
        // This is the engine's anti-re-entrancy guarantee (audit Finding 17 / Phase 2).
        let delay = delay_ms.max(1);
        self.schedule_creature_wakeup(cid, self.server_ms.saturating_add(delay));
    }

    /// C++ `TDAttack` branch in `ToDoStart` — `cract.cc:909-918`.
    pub(crate) fn todo_attack_delay_ms(&self, cid: CreatureId) -> u64 {
        self.creatures
            .get(cid)
            .map(|k| {
                let base = k.base();
                base.earliest_attack_ms
                    .max(base.earliest_spell_server_ms)
                    .saturating_sub(self.server_ms)
            })
            .unwrap_or(0)
    }

    /// F8 S3 — C++ `CalculateDelay(TDUse)` core — `cract.cc:925-932`. Returns
    /// `EarliestMultiuseTime - ServerMilliseconds` **only if `has_obj2`** (two-object
    /// use); single-object use is ungated (delay 0). Shared by the peek-based
    /// [`todo_use_delay_ms`] (S6 routing — action at front of queue) and the execute-arm
    /// gate check (action already popped — passes `obj2.is_some()` directly).
    pub(crate) fn multiuse_gate_delay_ms(&self, cid: CreatureId, has_obj2: bool) -> u64 {
        if !has_obj2 {
            return 0;
        }
        self.creatures
            .get(cid)
            .map(|k| k.base().earliest_multiuse_server_ms.saturating_sub(self.server_ms))
            .unwrap_or(0)
    }

    /// F8 S3 — C++ `CalculateDelay(TDUse)` — `cract.cc:925-932`. Returns the multiuse gate
    /// delay for the front `Use` action: `EarliestMultiuseTime - ServerMilliseconds` **only
    /// if `obj2.is_some()`** (two-object use); single-object use is ungated (delay 0). The
    /// `default` case (`TDMove`/`TDTurn`/`TDTalk`/…) is delay 0 (`cract.cc:946-948`), so this
    /// helper returns 0 for any non-`Use` front action — Move/Turn are ungated by design.
    /// Used by S6 handler routing (action is at the front after enqueue). The execute arm
    /// calls [`multiuse_gate_delay_ms`] directly with the popped action's `obj2.is_some()`.
    pub(crate) fn todo_use_delay_ms(&self, cid: CreatureId) -> u64 {
        let has_obj2 = self
            .creatures
            .get(cid)
            .and_then(|k| k.base().todo.queue.front())
            .is_some_and(|a| matches!(a, CreatureAction::Use { obj2: Some(_), .. }));
        self.multiuse_gate_delay_ms(cid, has_obj2)
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
        self.schedule_creature_wakeup(cid, self.server_ms.saturating_add(1));
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
        // C++ `ToDoWait(0)` → `ToDoStart` clamps `Delay<1` to 1 ⇒ wakeup at `server_ms + 1`
        // (next beat), the oracle's yield-defers-a-beat contract (`cract.cc:1016`, audit Finding 17).
        self.todo_start_from_action(cid, 0);
    }

    /// Whether `cid` runs on the unified 772 ToDo/`Execute`/`IdleStimulus` path.
    ///
    /// Phase 0 walk-engine unification: widened from monster-only to include **players** so both
    /// share `Execute` → `Go`/`Attack` → `IdleStimulus` → `Combat.CanToDoAttack`
    /// (`cract.cc:783`, `crplayer.cc:388`). NPCs remain excluded (no ToDo-driven behavior).
    pub(crate) fn creature_uses_todo_execute(&self, cid: CreatureId) -> bool {
        self.beat_driven_loop
            && self.creatures.get(cid).is_some_and(|k| {
                matches!(k, CreatureKind::Monster(_) | CreatureKind::Player(_))
            })
    }
}

#[cfg(test)]
mod tests {
    use tfs_rust_common::Position;

    use crate::creature::CreatureKind;
    use crate::creature_todo::{ActionObjectRef, CreatureAction, CreatureTodo, MONSTER_IDLE_WAIT_MS};
    use crate::ids::CreatureId;
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

    #[test]
    fn has_use_move_turn_helpers_detect_queued_variants() {
        let obj = ActionObjectRef {
            pos: Position::new(100, 100, 7),
            stack_pos: 0,
            sprite_id: 100,
        };
        let mut todo = CreatureTodo::default();
        assert!(!todo.has_use());
        assert!(!todo.has_move());
        assert!(!todo.has_turn());

        todo.queue.push_back(CreatureAction::Use {
            obj1: obj,
            obj2: None,
            open_index: 0,
        });
        assert!(todo.has_use());
        assert!(!todo.has_move());
        assert!(!todo.has_turn());

        todo.queue.push_back(CreatureAction::Move {
            obj,
            dest: Position::new(101, 100, 7),
            count: 1,
        });
        assert!(todo.has_use());
        assert!(todo.has_move());
        assert!(!todo.has_turn());

        todo.queue.push_back(CreatureAction::Turn { obj });
        assert!(todo.has_use());
        assert!(todo.has_move());
        assert!(todo.has_turn());
    }

    // === F8 S2 — builder queue-shape + failure tests ===
    // C++ ref: `cract.cc:1258-1296` `ToDoUse`, `:1123-1172` `ToDoMove`,
    //          `:1326-1351` `ToDoTurn`; `receiving.cc:384/430/233/549` handlers.
    // The `beat_driven_test_world` items_db registers bag (1987, GROUP_CONTAINER)
    // and gold (2148, pickupable) with `client_id=0` (default), so `sprite_id=0`
    // validates via `validate_item_sprite` (`client_id_for_server == 0`).

    /// Place a bag (container) item on a tile and return its `ActionObjectRef`.
    /// Used by the Use/Turn success tests — `item_id_for_tile_use` finds
    /// containers via `is_container` priority.
    fn place_bag_on_tile(
        world: &mut crate::game_world::GameWorld,
        pos: Position,
    ) -> ActionObjectRef {
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let item_id = world.items.insert(crate::item::Item::new_single(1987));
        world
            .map
            .get_tile_mut(pos)
            .expect("tile just inserted")
            .add_item(item_id);
        ActionObjectRef {
            pos,
            stack_pos: 0,
            sprite_id: 0, // matches default `client_id=0` in test items_db
        }
    }

    /// Place a gold (pickupable, moveable) item on a tile and return its
    /// `ActionObjectRef`. Used by the Move success test —
    /// `internal_get_thing_move` finds moveable down-items via `get_top_down_item`.
    fn place_gold_on_tile(
        world: &mut crate::game_world::GameWorld,
        pos: Position,
    ) -> ActionObjectRef {
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let item_id = world.items.insert(crate::item::Item::new_single(2148));
        world
            .map
            .get_tile_mut(pos)
            .expect("tile just inserted")
            .add_item(item_id);
        ActionObjectRef {
            pos,
            stack_pos: 0,
            sprite_id: 0,
        }
    }

    fn insert_test_player(
        world: &mut crate::game_world::GameWorld,
        pos: Position,
    ) -> CreatureId {
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let cid = world
            .creatures
            .insert(CreatureKind::Player(test_player("S2Hero", pos)));
        world.map.register_creature_at(pos, cid);
        cid
    }

    #[test]
    fn enqueue_player_use_single_prepends_wait_then_use() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj1 = place_bag_on_tile(&mut world, item_pos);

        world
            .enqueue_player_use(cid, obj1, None, 0)
            .expect("bag on tile resolves");

        let todo = &world.creatures.get(cid).unwrap().base().todo;
        assert_eq!(todo.queue.len(), 2, "Use single → [Wait{{100}}, Use]");
        assert!(matches!(
            todo.queue[0],
            CreatureAction::Wait { delay_ms: 100 }
        ));
        match todo.queue[1] {
            CreatureAction::Use {
                obj1: ref o1,
                obj2,
                open_index,
            } => {
                assert_eq!(*o1, obj1);
                assert!(obj2.is_none(), "single-object use has no obj2");
                assert_eq!(open_index, 0);
            }
            ref other => panic!("expected Use, got {other:?}"),
        }
    }

    #[test]
    fn enqueue_player_use_two_object_prepends_wait_then_use_with_obj2() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let item_pos2 = Position::new(102, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj1 = place_bag_on_tile(&mut world, item_pos);
        let obj2 = place_bag_on_tile(&mut world, item_pos2);

        world
            .enqueue_player_use(cid, obj1, Some(obj2), 3)
            .expect("both bags resolve");

        let todo = &world.creatures.get(cid).unwrap().base().todo;
        assert_eq!(todo.queue.len(), 2, "Use two-object → [Wait{{100}}, Use]");
        assert!(matches!(
            todo.queue[0],
            CreatureAction::Wait { delay_ms: 100 }
        ));
        match todo.queue[1] {
            CreatureAction::Use {
                obj1: _,
                obj2: Some(ref o2),
                open_index,
            } => {
                assert_eq!(*o2, obj2);
                assert_eq!(open_index, 3);
            }
            ref other => panic!("expected Use with obj2, got {other:?}"),
        }
    }

    #[test]
    fn enqueue_player_move_enqueues_single_move_no_wait() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let dest = Position::new(105, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj = place_gold_on_tile(&mut world, item_pos);

        world
            .enqueue_player_move(cid, obj, dest, 1)
            .expect("gold on tile resolves");

        let todo = &world.creatures.get(cid).unwrap().base().todo;
        assert_eq!(todo.queue.len(), 1, "Move → single entry, no Wait");
        match todo.queue[0] {
            CreatureAction::Move {
                obj: ref o,
                dest: d,
                count,
            } => {
                assert_eq!(*o, obj);
                assert_eq!(d, dest);
                assert_eq!(count, 1);
            }
            ref other => panic!("expected Move, got {other:?}"),
        }
    }

    #[test]
    fn enqueue_player_turn_prepends_wait_then_turn() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj = place_bag_on_tile(&mut world, item_pos);

        world
            .enqueue_player_turn(cid, obj)
            .expect("bag on tile resolves");

        let todo = &world.creatures.get(cid).unwrap().base().todo;
        assert_eq!(todo.queue.len(), 2, "Turn → [Wait{{100}}, Turn]");
        assert!(matches!(
            todo.queue[0],
            CreatureAction::Wait { delay_ms: 100 }
        ));
        match todo.queue[1] {
            CreatureAction::Turn { obj: ref o } => assert_eq!(*o, obj),
            ref other => panic!("expected Turn, got {other:?}"),
        }
    }

    #[test]
    fn enqueue_player_use_fails_on_absent_object() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let absent = ActionObjectRef {
            pos: Position::new(200, 200, 7), // no tile, no item
            stack_pos: 0,
            sprite_id: 0,
        };

        let result = world.enqueue_player_use(cid, absent, None, 0);

        assert_eq!(result, Err(crate::return_value::ReturnValue::NotPossible));
        assert!(
            world.creatures.get(cid).unwrap().base().todo.is_empty(),
            "failed builder must not enqueue anything"
        );
    }

    #[test]
    fn enqueue_player_turn_fails_on_absent_object() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let absent = ActionObjectRef {
            pos: Position::new(200, 200, 7),
            stack_pos: 0,
            sprite_id: 0,
        };

        let result = world.enqueue_player_turn(cid, absent);

        assert_eq!(result, Err(crate::return_value::ReturnValue::NotPossible));
        assert!(
            world.creatures.get(cid).unwrap().base().todo.is_empty(),
            "failed builder must not enqueue anything"
        );
    }

    #[test]
    fn enqueue_player_move_fails_on_absent_object() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let absent = ActionObjectRef {
            pos: Position::new(200, 200, 7),
            stack_pos: 0,
            sprite_id: 0,
        };

        let result = world.enqueue_player_move(cid, absent, Position::new(105, 100, 7), 1);

        assert_eq!(result, Err(crate::return_value::ReturnValue::NotPossible));
        assert!(
            world.creatures.get(cid).unwrap().base().todo.is_empty(),
            "failed builder must not enqueue anything"
        );
    }

    // === F8 S3 — CalculateDelay multiuse gate tests ===
    // C++ ref: `cract.cc:925-932` `CalculateDelay(TDUse)` — gates only `Obj2 != 0`;
    //          `cract.cc:946-948` `default` → 0 for `TDMove`/`TDTurn`/etc.

    /// Two-object `Use` within the multiuse gate returns the remaining delay
    /// (`EarliestMultiuseTime - ServerMilliseconds`).
    #[test]
    fn todo_use_delay_ms_two_object_within_gate_returns_remaining() {
        let mut world = beat_driven_test_world_at(1000);
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let item_pos2 = Position::new(102, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj1 = place_bag_on_tile(&mut world, item_pos);
        let obj2 = place_bag_on_tile(&mut world, item_pos2);

        world
            .enqueue_player_use(cid, obj1, Some(obj2), 0)
            .expect("both bags resolve");
        // Queue is [Wait{100}, Use{obj2:Some}]. `todo_use_delay_ms` peeks the front
        // (Wait), which is not a `Use` → returns 0. Pop the Wait manually so the
        // front becomes the `Use` we want to gate-check.
        if let Some(k) = world.creatures.get_mut(cid) {
            k.base_mut().todo.queue.pop_front();
        }
        // Arm multiuse exhaustion 500 ms in the future.
        if let Some(k) = world.creatures.get_mut(cid) {
            k.base_mut().earliest_multiuse_server_ms = 1500;
        }

        assert_eq!(
            world.todo_use_delay_ms(cid),
            500,
            "two-object Use within gate → EarliestMultiuseTime - ServerMs"
        );
    }

    /// Single-object `Use` is ungated — delay 0 regardless of `EarliestMultiuseTime`.
    /// C++ only gates `Obj2 != 0` (`cract.cc:926`).
    #[test]
    fn todo_use_delay_ms_single_object_is_ungated() {
        let mut world = beat_driven_test_world_at(1000);
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj1 = place_bag_on_tile(&mut world, item_pos);

        world
            .enqueue_player_use(cid, obj1, None, 0)
            .expect("bag resolves");
        // Pop the Wait so the front is the single-object Use.
        if let Some(k) = world.creatures.get_mut(cid) {
            k.base_mut().todo.queue.pop_front();
        }
        // Arm multiuse exhaustion — single-object Use must still be ungated.
        if let Some(k) = world.creatures.get_mut(cid) {
            k.base_mut().earliest_multiuse_server_ms = 5000;
        }

        assert_eq!(
            world.todo_use_delay_ms(cid),
            0,
            "single-object Use is ungated (C++ only gates Obj2 != 0)"
        );
    }

    /// Two-object `Use` past the multiuse gate returns 0 (no remaining delay).
    #[test]
    fn todo_use_delay_ms_two_object_past_gate_returns_zero() {
        let mut world = beat_driven_test_world_at(2000);
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let item_pos2 = Position::new(102, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj1 = place_bag_on_tile(&mut world, item_pos);
        let obj2 = place_bag_on_tile(&mut world, item_pos2);

        world
            .enqueue_player_use(cid, obj1, Some(obj2), 0)
            .expect("both bags resolve");
        // Pop the Wait so the front is the two-object Use.
        if let Some(k) = world.creatures.get_mut(cid) {
            k.base_mut().todo.queue.pop_front();
        }
        // Gate is in the past (server_ms=2000, gate=1000).
        if let Some(k) = world.creatures.get_mut(cid) {
            k.base_mut().earliest_multiuse_server_ms = 1000;
        }

        assert_eq!(
            world.todo_use_delay_ms(cid),
            0,
            "two-object Use past gate → 0 (saturating_sub clamps to 0)"
        );
    }
}
