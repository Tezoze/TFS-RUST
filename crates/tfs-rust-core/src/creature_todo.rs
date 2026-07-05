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
use tfs_rust_common::{ConnId, Position};

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
    pub(crate) fn validate_action_object_ref(
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
    pub(crate) fn validate_move_object_ref(
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

    /// F8 D2/D6 — `ObjectInRange` z-floor gate shared by `ToDoUse`/`ToDoMove`/
    /// `ToDoTurn` (`cract.cc:1131-1135/1272-1276/1332-1336`). For map-tile sources
    /// (`obj.pos.x != 0xFFFF`), throws `UPSTAIRS`/`DOWNSTAIRS` (Rust
    /// `FirstGoUpStairs`/`FirstGoDownStairs`) when the player and object are on
    /// different floors — before any walk attempt. Mirrors `info.cc:233-257`
    /// `ObjectInRange`'s `posz == ObjZ` precondition; the same-z Chebyshev
    /// `|dx|<=1 && |dy|<=1` reach test lives in the execute arms (D6).
    /// Inventory/container sources (`0xFFFF`) skip the check (always "adjacent").
    pub(crate) fn validate_action_object_z_floor(
        &self,
        cid: CreatureId,
        obj: ActionObjectRef,
    ) -> Result<(), ReturnValue> {
        if obj.pos.x == 0xFFFF {
            return Ok(());
        }
        let Some(k) = self.creatures.get(cid) else {
            return Err(ReturnValue::NotPossible);
        };
        let pp = k.position();
        // C++ `posz > ObjZ → UPSTAIRS` (player deeper than object → go up),
        // `posz < ObjZ → DOWNSTAIRS` (player above object → go down).
        if pp.z > obj.pos.z {
            Err(ReturnValue::FirstGoUpStairs)
        } else if pp.z < obj.pos.z {
            Err(ReturnValue::FirstGoDownStairs)
        } else {
            Ok(())
        }
    }

    /// F8 S2 — `ToDoUse` builder (`cract.cc:1258-1296`). Resolves both objects now
    /// (mirroring `GetObject`'s `throw RESULT` on failure), applies the D2/D6
    /// z-floor gate (`cract.cc:1272-1276`), prepends `Wait{100}`
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
        // D2/D6 — `UPSTAIRS`/`DOWNSTAIRS` z-floor before walk/wait (`cract.cc:1272-1276`).
        self.validate_action_object_z_floor(cid, obj1)?;
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
    /// `receiving.cc:233`). Resolves the source object now, applies the D2/D6
    /// z-floor gate (`cract.cc:1131-1135`), prepends `Wait{100}` (`cract.cc:1155`
    /// `int Delay = 100;` → `this->ToDoWait(Delay)` at `cract.cc:1165`), and
    /// enqueues `Move`. The `CMoveObject` handler itself adds no leading
    /// `ToDoWait` — but `ToDoMove` **always** does, so the resulting queue for an
    /// adjacent map item is `[Wait{100}, Move]`. The creature-container branch
    /// (`Delay = 1000` + `BANK` dest check, `cract.cc:1156-1163`) is not ported
    /// yet (D9 — creature push is out of scope); when it lands, the delay must
    /// be selected per source kind. `dest` is the throw destination
    /// (map/inventory/container encoded in `Position`), `count` is the stack
    /// count. Maps to Rust `GamePacket::Throw` (not `MoveObject` — F8 §0.1 F5).
    pub(crate) fn enqueue_player_move(
        &mut self,
        cid: CreatureId,
        obj: ActionObjectRef,
        dest: Position,
        count: u8,
    ) -> Result<(), ReturnValue> {
        self.validate_move_object_ref(cid, obj)?;
        // D2/D6 — `UPSTAIRS`/`DOWNSTAIRS` z-floor before walk/wait (`cract.cc:1131-1135`).
        self.validate_action_object_z_floor(cid, obj)?;
        // D1 — `ToDoMove` always calls `this->ToDoWait(Delay)` with `Delay = 100`
        // for the non-creature-container path (`cract.cc:1155,1165`). Without this
        // the throw executes on the next beat (~1 ms) instead of ~100 ms out, so
        // items move faster than the reference and rapid move packets have no
        // pacing. Creature-container push (Delay = 1000) is D9, not yet ported.
        self.enqueue_creature_wait(cid, 100);
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
    /// the object now, applies the D2/D6 z-floor gate (`cract.cc:1334-1338`),
    /// prepends `Wait{100}` (`receiving.cc:549`), enqueues `Turn`. The executor
    /// is new code (S4 — nothing exists to reuse, F8 §0.1 F2).
    ///
    /// F8 D3 — the C++ builder's `ObjectInRange(1)` → `ToDoGo(...)` walk-to-reach
    /// (`cract.cc:1340-1341`) is **not** enqueued here; it is deferred to the
    /// `Turn` execute arm in `idle_stimulus.rs` (S5 `Go`-prepend pattern, same
    /// shape as `Use`/`Move`).
    pub(crate) fn enqueue_player_turn(
        &mut self,
        cid: CreatureId,
        obj: ActionObjectRef,
    ) -> Result<(), ReturnValue> {
        self.validate_action_object_ref(cid, obj)?;
        // D2/D6 — `UPSTAIRS`/`DOWNSTAIRS` z-floor before walk/wait (`cract.cc:1334-1338`).
        self.validate_action_object_z_floor(cid, obj)?;
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
    /// use); single-object use is ungated (delay 0). Called by the execute-arm gate check
    /// (action already popped — passes `obj2.is_some()` directly). The S6 handler does NOT
    /// call this — the `Wait{100}` prefix drives the initial `ToDoStart` delay, and the
    /// multiuse gate is applied here when the `Use` action reaches the front during the
    /// execute drain (mirroring C++ `Execute` re-running `CalculateDelay` per entry).
    pub(crate) fn multiuse_gate_delay_ms(&self, cid: CreatureId, has_obj2: bool) -> u64 {
        if !has_obj2 {
            return 0;
        }
        self.creatures
            .get(cid)
            .map(|k| k.base().earliest_multiuse_server_ms.saturating_sub(self.server_ms))
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
        if chase_debug::chase_path_debug_enabled() {
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
        // Phase 4: 1098 arm deleted — both eras use the beat `todo_start_go_delay` path.
        let _ = self.todo_start_go_delay(cid, first_step);
    }

    /// Arm the next todo step on the heap without synchronous re-entry (avoids stack overflow).
    pub(crate) fn schedule_immediate_todo_wakeup(&mut self, cid: CreatureId) {
        self.schedule_creature_wakeup(cid, self.server_ms.saturating_add(1));
    }

    /// C++ `TCreature::ToDoYield` — `cract.cc:1001` (`ToDoWait(0)` + `ToDoStart` when not `LockToDo`).
    pub(crate) fn creature_todo_yield(&mut self, cid: CreatureId) {
        // Phase 4: 1098 defer deleted — both eras use ToDo yield.
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
    /// Phase 6: `beat_driven_loop` collapsed — both eras unconditionally use the ToDo path.
    pub(crate) fn creature_uses_todo_execute(&self, cid: CreatureId) -> bool {
        self.creatures.get(cid).is_some_and(|k| {
            matches!(k, CreatureKind::Monster(_) | CreatureKind::Player(_))
        })
    }

    /// F8 S4 — C++ `Execute` `RESULT` catch — `cract.cc:870-889`.
    ///
    /// Called by the `Use`/`Move`/`Turn` execute arms when the executor returns
    /// `Err(rv)`. Mirrors the C++ catch:
    /// 1. `ToDoClear()` — clear the queue; `had_pending_go` drives the snapback decision
    ///    (`cract.cc:871`, `:953-989`).
    /// 2. `EXHAUSTED` → `ToDoWait(1000)` + `ToDoStart` (`cract.cc:872-874`).
    ///    Else → `ToDoYield` = `ToDoWait(0)` + `ToDoStart` (`cract.cc:875-877`, `:1026-1031`).
    /// 3. Player-only: `SendResult` (= `send_cancel_message`) + conditional `SendSnapback`
    ///    (`cract.cc:879-886`). Snapback is skipped for `MOVENOTPOSSIBLE` / `NOTINVITED` /
    ///    `ENTERPROTECTIONZONE` (772 RESULT codes 52/50/48, `enums.hh:440-444`).
    ///
    /// 772 RESULT → Rust `ReturnValue` mapping (approximate — no exact variants for
    /// `MOVENOTPOSSIBLE`/`NOTTURNABLE`/`DESTROYED`; `NotPossible`/`ThereIsNoWay` are the
    /// existing convention per `walk/mod.rs:1506`):
    /// - `EXHAUSTED` (49) → `YouAreExhausted`
    /// - `NOTINVITED` (50) → `PlayerIsNotInvited`
    /// - `ENTERPROTECTIONZONE` (48) → `ActionNotPermittedInProtectionZone`
    /// - `MOVENOTPOSSIBLE` (52) → `ThereIsNoWay` (closest — used when no path)
    pub(crate) fn apply_todo_result_catch(&mut self, cid: CreatureId, rv: ReturnValue) {
        // `ToDoClear` — clear the queue. `player_todo_clear` also clears walk state
        // (broader than C++ `ToDoClear`, but correct for a failed action restart).
        let had_pending_go = self.player_todo_clear(cid);

        if rv == ReturnValue::YouAreExhausted {
            // `EXHAUSTED` → `ToDoWait(1000)` + `ToDoStart` (`cract.cc:872-874`).
            self.enqueue_creature_wait(cid, 1000);
            self.todo_start_from_action(cid, 1000);
            trace_creature_todo(self, cid, "result_catch_exhausted");
        } else {
            // `ToDoYield` = `ToDoWait(0)` + `ToDoStart` (`cract.cc:875-877`, `:1026-1031`).
            self.enqueue_creature_wait(cid, 0);
            self.todo_start_from_action(cid, 0);
            trace_creature_todo(self, cid, "result_catch_yield");
        }

        // Player-only: `SendResult` + conditional `SendSnapback` (`cract.cc:879-886`).
        if let Some(conn) = self.conn_for_creature(cid) {
            self.send_result_player(conn, cid, rv, had_pending_go);
        }
    }

    /// `SendResult` + conditional `SendSnapback` — the player tail of the `RESULT` catch
    /// (`cract.cc:879-886`). Split out so `apply_todo_result_catch` stays readable.
    fn send_result_player(&mut self, conn: ConnId, cid: CreatureId, rv: ReturnValue, snapback: bool) {
        // `SendResult` — `sending.cc:285-357`: text via `SendMessage(TALK_FAILURE_MESSAGE, ...)`.
        self.send_cancel_message(conn, rv);
        // `SendSnapback` — skip for `MOVENOTPOSSIBLE` / `NOTINVITED` / `ENTERPROTECTIONZONE`
        // (`cract.cc:882-884`). Only sent when `SnapbackNecessary` (a pending `TDGo` was cleared).
        let snapback_exempt = matches!(
            rv,
            ReturnValue::PlayerIsNotInvited               // NOTINVITED (50)
                | ReturnValue::ActionNotPermittedInProtectionZone // ENTERPROTECTIONZONE (48)
                | ReturnValue::ThereIsNoWay               // MOVENOTPOSSIBLE (52) — closest
        );
        if snapback && !snapback_exempt {
            let dir_byte = self
                .creatures
                .get(cid)
                .map(|k| k.base().direction as u8)
                .unwrap_or(0);
            self.enqueue_encoded(conn, self.codec.encode_cancel_walk(dir_byte));
        }
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
    fn enqueue_player_move_prepends_wait_then_move() {
        // D1 — `ToDoMove` always calls `this->ToDoWait(100)` (`cract.cc:1155,1165`),
        // so the builder queue is `[Wait{100}, Move]`, matching the reference's
        // ~100 ms throw floor (not the next-beat ~1 ms execution the old code had).
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
        assert_eq!(todo.queue.len(), 2, "Move → [Wait{{100}}, Move]");
        assert!(
            matches!(todo.queue[0], CreatureAction::Wait { delay_ms: 100 }),
            "front = Wait{{100}}"
        );
        match todo.queue[1] {
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

    // === F8 S4 — Turn executor + RESULT catch tests ===
    // C++ ref: `operate.cc:2562-2583` `Turn`, `cract.cc:870-889` RESULT catch,
    //          `cract.cc:1026-1031` `ToDoYield`, `enums.hh:390-453` RESULT codes.

    /// Helper: register a rotatable item type (server id `id`, rotates to `to`) in the
    /// test `items_db` and return the registered `ItemType`. The default
    /// `beat_driven_test_world` items_db only has bag (1987) + gold (2148); rotatable
    /// items need `rotatable()` + `rotate_to` set.
    fn register_rotatable_item(
        world: &mut crate::game_world::GameWorld,
        server_id: u16,
        rotate_to: u16,
    ) {
        use tfs_rust_content::otb::ItemType;
        let mut it = ItemType {
            server_id,
            ..Default::default()
        };
        // `FLAG_ROTATABLE` = `1 << 15` (`otb.rs:427`) — private, so set the bit directly.
        it.flags |= 1u32 << 15;
        it.rotate_to = rotate_to;
        // Rebuild the items_db Arc with the new entry added. Tests own the only ref.
        let mut items_map = world.items_db.items.clone();
        items_map.insert(server_id, it);
        world.items_db = std::sync::Arc::new(tfs_rust_content::items::ItemDatabase {
            items: items_map,
            client_to_server: std::collections::HashMap::new(),
        });
    }

    /// Place a rotatable item on a tile and return its `ActionObjectRef`.
    fn place_rotatable_on_tile(
        world: &mut crate::game_world::GameWorld,
        pos: Position,
        server_id: u16,
        rotate_to: u16,
    ) -> ActionObjectRef {
        register_rotatable_item(world, server_id, rotate_to);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let item_id = world
            .items
            .insert(crate::item::Item::new_single(server_id));
        world
            .map
            .get_tile_mut(pos)
            .expect("tile just inserted")
            .add_item(item_id);
        ActionObjectRef {
            pos,
            stack_pos: 0,
            sprite_id: 0, // default client_id=0 in test items_db
        }
    }

    /// `Turn` executor: rotatable item transforms to `rotate_to` on success.
    /// C++ ref: `operate.cc:2577-2583` `Change(Obj, RotateTarget, 0)`.
    #[test]
    fn player_rotate_item_transforms_rotatable_item() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj = place_rotatable_on_tile(&mut world, item_pos, 5001, 5002);

        let item_id = world
            .resolve_rotate_item_id(cid, obj)
            .expect("rotatable item resolves");
        assert_eq!(
            world.items.get(item_id).unwrap().item_type,
            5001,
            "item starts at original type"
        );

        world
            .player_rotate_item(cid, obj)
            .expect("rotatable item rotates");

        assert_eq!(
            world.items.get(item_id).unwrap().item_type,
            5002,
            "item transformed to rotate_to"
        );
    }

    /// `Turn` executor: non-rotatable item → `Err(NotPossible)` (C++ `NOTTURNABLE`).
    #[test]
    fn player_rotate_item_fails_on_non_rotatable() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        // Bag (1987) is a container, not rotatable.
        let obj = place_bag_on_tile(&mut world, item_pos);

        let result = world.player_rotate_item(cid, obj);
        assert_eq!(
            result,
            Err(crate::return_value::ReturnValue::NotPossible),
            "non-rotatable item → NOTTURNABLE → NotPossible"
        );
    }

    /// `Turn` executor: out-of-range map tile → `Err(NotPossible)` (C++ `NOTACCESSIBLE`).
    #[test]
    fn player_rotate_item_fails_when_out_of_range() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let far_pos = Position::new(110, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj = place_rotatable_on_tile(&mut world, far_pos, 5001, 5002);

        let result = world.player_rotate_item(cid, obj);
        assert_eq!(
            result,
            Err(crate::return_value::ReturnValue::NotPossible),
            "out-of-range item → NOTACCESSIBLE → NotPossible"
        );
    }

    /// `Turn` executor: absent item → `Err(NotPossible)` (C++ `DESTROYED`).
    #[test]
    fn player_rotate_item_fails_on_absent_object() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let absent = ActionObjectRef {
            pos: Position::new(200, 200, 7),
            stack_pos: 0,
            sprite_id: 0,
        };

        let result = world.player_rotate_item(cid, absent);
        assert_eq!(
            result,
            Err(crate::return_value::ReturnValue::NotPossible),
            "absent item → DESTROYED → NotPossible"
        );
    }

    /// `Turn` executor: `rotate_to == 0` → `Err(NotPossible)` (no rotation target).
    #[test]
    fn player_rotate_item_fails_when_rotate_to_zero() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj = place_rotatable_on_tile(&mut world, item_pos, 5001, 0);

        let result = world.player_rotate_item(cid, obj);
        assert_eq!(
            result,
            Err(crate::return_value::ReturnValue::NotPossible),
            "rotate_to=0 → no rotation target → NotPossible"
        );
    }

    /// RESULT catch: `EXHAUSTED` → `ToDoWait(1000)` + `ToDoStart` + queue cleared.
    /// C++ ref: `cract.cc:872-874`.
    #[test]
    fn apply_todo_result_catch_exhausted_clears_queue_and_waits_1000() {
        let mut world = beat_driven_test_world_at(5000);
        let player_pos = Position::new(100, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        // Pre-populate the queue with a Turn so we can verify clear.
        let obj = place_rotatable_on_tile(
            &mut world,
            Position::new(101, 100, 7),
            5001,
            5002,
        );
        world
            .enqueue_player_turn(cid, obj)
            .expect("rotatable resolves");
        assert!(
            !world.creatures.get(cid).unwrap().base().todo.is_empty(),
            "queue populated before catch"
        );

        world.apply_todo_result_catch(cid, crate::return_value::ReturnValue::YouAreExhausted);

        let base = world.creatures.get(cid).unwrap().base();
        // `ToDoClear` wipes the queue, then `ToDoWait(1000)` enqueues a single
        // `Wait{1000}`. The original Turn/Wait entries are gone; only the catch's
        // Wait remains (`cract.cc:872-874`).
        assert_eq!(
            base.todo.queue.len(),
            1,
            "EXHAUSTED catch → ToDoClear + ToDoWait(1000) → [Wait{{1000}}]"
        );
        assert!(matches!(
            base.todo.queue[0],
            CreatureAction::Wait { delay_ms: 1000 }
        ));
    }

    /// RESULT catch: non-exhausted error → `ToDoYield` = `ToDoWait(0)` + `ToDoStart`.
    /// C++ ref: `cract.cc:875-877`, `:1026-1031`.
    #[test]
    fn apply_todo_result_catch_non_exhausted_yields_wait_zero() {
        let mut world = beat_driven_test_world_at(5000);
        let player_pos = Position::new(100, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj = place_rotatable_on_tile(
            &mut world,
            Position::new(101, 100, 7),
            5001,
            5002,
        );
        world
            .enqueue_player_turn(cid, obj)
            .expect("rotatable resolves");

        world.apply_todo_result_catch(cid, crate::return_value::ReturnValue::NotPossible);

        let base = world.creatures.get(cid).unwrap().base();
        assert_eq!(
            base.todo.queue.len(),
            1,
            "non-exhausted catch → [Wait{{0}}] (ToDoYield)"
        );
        assert!(matches!(
            base.todo.queue[0],
            CreatureAction::Wait { delay_ms: 0 }
        ));
    }

    /// RESULT catch: `had_pending_go` is returned by `player_todo_clear` and would
    /// trigger `SendSnapback` for non-exempt errors. This test verifies the
    /// snapback-exempt set (`ThereIsNoWay` = `MOVENOTPOSSIBLE`) does **not** panic
    /// and leaves the queue in the yield state — the actual snapback packet requires
    /// a conn and is exercised in integration tests.
    #[test]
    fn apply_todo_result_catch_snapback_exempt_does_not_panic() {
        let mut world = beat_driven_test_world_at(5000);
        let player_pos = Position::new(100, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        // Enqueue a Go so `had_pending_go` is true.
        ensure_walkable_tile(
            &mut world.map,
            Position::new(102, 100, 7),
            TEST_SYNTHETIC_GROUND_WP,
        );
        if let Some(k) = world.creatures.get_mut(cid) {
            k.base_mut().todo.queue.push_back(CreatureAction::Go);
        }

        // `ThereIsNoWay` is snapback-exempt (MOVENOTPOSSIBLE) — no panic, queue cleared.
        world.apply_todo_result_catch(cid, crate::return_value::ReturnValue::ThereIsNoWay);

        let base = world.creatures.get(cid).unwrap().base();
        assert_eq!(base.todo.queue.len(), 1, "yield enqueued Wait{{0}}");
        assert!(matches!(
            base.todo.queue[0],
            CreatureAction::Wait { delay_ms: 0 }
        ));
    }

    /// `Use` execute arm: single-object use on a bag opens the container (success).
    /// Verifies the execute dispatch reaches `player_use_item` and returns `Ok(())`.
    #[test]
    fn execute_player_use_single_object_opens_container() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj1 = place_bag_on_tile(&mut world, item_pos);

        // `execute_player_use` re-validates + dispatches to `player_use_item`.
        // No conn registered → returns `Ok(())` (no-op, no panic).
        let result = world.execute_player_use(cid, obj1, None, 0);
        assert!(result.is_ok(), "single-object use dispatches without conn");
    }

    /// `Move` execute arm: re-validation failure (absent object) → `Err(NotPossible)`.
    /// The `RESULT` catch is applied by the caller (`execute_creature_todo_action`);
    /// here we verify the executor returns the error.
    #[test]
    fn execute_player_move_fails_on_absent_object() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let absent = ActionObjectRef {
            pos: Position::new(200, 200, 7),
            stack_pos: 0,
            sprite_id: 0,
        };

        let result = world.execute_player_move(cid, absent, Position::new(105, 100, 7), 1);
        assert_eq!(
            result,
            Err(crate::return_value::ReturnValue::NotPossible),
            "absent object → re-validation fails → NotPossible"
        );
    }

    // === F8 S5 — Go-prepend walk-to-reach tests ===
    // C++ ref: `cract.cc:600-760` `Use` executor — if the target isn't reachable,
    // prepend `ToDoGo(dest)` + re-enqueue `ToDoUse`/`ToDoMove` + `ToDoStart`.

    /// Helper: lay a walkable corridor from `start` to `end` (inclusive, X axis).
    fn ensure_walkable_corridor_x(
        world: &mut crate::game_world::GameWorld,
        start_x: u16,
        end_x: u16,
        y: u16,
        z: u8,
    ) {
        for x in start_x..=end_x {
            ensure_walkable_tile(&mut world.map, Position::new(x, y, z), TEST_SYNTHETIC_GROUND_WP);
        }
    }

    /// S5: `Use` execute arm with a not-adjacent map tile → `Go`-prepend.
    /// The queue becomes `[Go, Use]` and `walk_queue` is populated — mirroring the
    /// C++ `Use` executor's `ToDoGo(dest)` + re-enqueue `ToDoUse` (`cract.cc:600-760`).
    #[test]
    fn s5_use_not_adjacent_prepends_go_and_re_enqueues_use() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(105, 100, 7);
        ensure_walkable_corridor_x(&mut world, 100, 105, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj1 = place_bag_on_tile(&mut world, item_pos);

        world
            .enqueue_player_use(cid, obj1, None, 0)
            .expect("bag on tile resolves");
        // Pop the `Wait{100}` so the front is the `Use` action.
        if let Some(k) = world.creatures.get_mut(cid) {
            k.base_mut().todo.queue.pop_front();
        }

        // Execute the `Use` — should detect non-adjacency and Go-prepend.
        let kind = world.execute_creature_todo_action(cid);
        assert!(
            matches!(kind, Some(crate::idle_stimulus::TodoExecuteKind::Deferred)),
            "not-adjacent Use → Deferred (Go-prepend)"
        );

        let base = world.creatures.get(cid).unwrap().base();
        assert_eq!(
            base.todo.queue.len(),
            2,
            "queue → [Go, Use]"
        );
        assert!(matches!(base.todo.queue[0], CreatureAction::Go), "front = Go");
        assert!(
            matches!(base.todo.queue[1], CreatureAction::Use { .. }),
            "second = Use"
        );
        assert!(
            !base.walk_queue.is_empty(),
            "walk_queue populated for walk-to-reach"
        );
    }

    /// S5: `Use` execute arm with an adjacent map tile → no `Go`-prepend.
    /// Dispatches to the executor directly (queue empty after execute).
    #[test]
    fn s5_use_adjacent_does_not_go_prepend() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        ensure_walkable_corridor_x(&mut world, 100, 101, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj1 = place_bag_on_tile(&mut world, item_pos);

        world
            .enqueue_player_use(cid, obj1, None, 0)
            .expect("bag on tile resolves");
        // Pop the `Wait{100}`.
        if let Some(k) = world.creatures.get_mut(cid) {
            k.base_mut().todo.queue.pop_front();
        }

        let kind = world.execute_creature_todo_action(cid);
        // Adjacent → dispatches to executor. No conn → Ok(()). Kind = Wait.
        assert!(
            matches!(kind, Some(crate::idle_stimulus::TodoExecuteKind::Wait)),
            "adjacent Use → Wait (no Go-prepend)"
        );
        let base = world.creatures.get(cid).unwrap().base();
        assert!(
            base.todo.queue.is_empty(),
            "queue drained after adjacent Use execute"
        );
        assert!(
            base.walk_queue.is_empty(),
            "no walk_queue for adjacent Use"
        );
    }

    /// S5: `Use` execute arm with an inventory source → no `Go`-prepend.
    /// Inventory/container sources are always "adjacent" (no walk needed).
    #[test]
    fn s5_use_inventory_source_does_not_go_prepend() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, player_pos, TEST_SYNTHETIC_GROUND_WP);
        let cid = insert_test_player(&mut world, player_pos);
        // Inventory slot encoding: pos.x = 0xFFFF, pos.y = 0x40 | slot.
        let inv_obj = ActionObjectRef {
            pos: Position::new(0xFFFF, 0x40 | 1, 7),
            stack_pos: 0,
            sprite_id: 0,
        };

        // Enqueue — validate_action_object_ref may fail for an empty inventory slot,
        // so we push the Use directly (bypassing the builder's validation) to test
        // the execute arm's adjacency logic only.
        if let Some(k) = world.creatures.get_mut(cid) {
            k.base_mut().todo.queue.push_back(CreatureAction::Use {
                obj1: inv_obj,
                obj2: None,
                open_index: 0,
            });
        }

        let kind = world.execute_creature_todo_action(cid);
        // Inventory source → no Go-prepend → dispatches to executor.
        // No conn + no item at that slot → executor returns Err(NotPossible) → RESULT catch.
        // The catch enqueues Wait{0} (ToDoYield). Kind = Wait.
        assert!(
            matches!(kind, Some(crate::idle_stimulus::TodoExecuteKind::Wait)),
            "inventory Use → Wait (no Go-prepend)"
        );
        let base = world.creatures.get(cid).unwrap().base();
        assert!(
            !base.todo.queue.iter().any(|a| matches!(a, CreatureAction::Go)),
            "no Go enqueued for inventory source"
        );
        assert!(
            base.walk_queue.is_empty(),
            "no walk_queue for inventory source"
        );
    }

    /// S5: `Move` execute arm with a not-adjacent map tile → `Go`-prepend.
    /// The queue becomes `[Go, Move]` and `walk_queue` is populated. D1 added a
    /// `Wait{100}` prefix from the builder (`cract.cc:1165`), so the test drains
    /// it first (one `execute_creature_todo_action` returning `Wait`) before the
    /// `Move` arm runs the `Go`-prepend.
    #[test]
    fn s5_move_not_adjacent_prepends_go_and_re_enqueues_move() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(105, 100, 7);
        let dest = Position::new(110, 100, 7);
        ensure_walkable_corridor_x(&mut world, 100, 110, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj = place_gold_on_tile(&mut world, item_pos);

        world
            .enqueue_player_move(cid, obj, dest, 1)
            .expect("gold on tile resolves");

        // D1: drain the builder's `Wait{100}` floor first.
        let drain_kind = world.execute_creature_todo_action(cid);
        assert!(
            matches!(drain_kind, Some(crate::idle_stimulus::TodoExecuteKind::Wait)),
            "builder Wait{{100}} drains first"
        );

        // Execute the `Move` — should detect non-adjacency and Go-prepend.
        let kind = world.execute_creature_todo_action(cid);
        assert!(
            matches!(kind, Some(crate::idle_stimulus::TodoExecuteKind::Deferred)),
            "not-adjacent Move → Deferred (Go-prepend)"
        );

        let base = world.creatures.get(cid).unwrap().base();
        assert_eq!(
            base.todo.queue.len(),
            2,
            "queue → [Go, Move]"
        );
        assert!(matches!(base.todo.queue[0], CreatureAction::Go), "front = Go");
        assert!(
            matches!(base.todo.queue[1], CreatureAction::Move { .. }),
            "second = Move"
        );
        assert!(
            !base.walk_queue.is_empty(),
            "walk_queue populated for walk-to-reach"
        );
    }

    /// S5: `Use` execute arm with no path to target → `Err(ThereIsNoWay)` → RESULT catch.
    /// The catch clears the queue and enqueues `Wait{0}` (ToDoYield).
    #[test]
    fn s5_use_no_path_to_target_applies_result_catch() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        // Item on an isolated tile — no walkable path from player.
        let item_pos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, player_pos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, item_pos, TEST_SYNTHETIC_GROUND_WP);
        // No corridor between (100,100) and (105,100) — tiles 101-104 are missing.
        let cid = insert_test_player(&mut world, player_pos);
        let obj1 = place_bag_on_tile(&mut world, item_pos);

        world
            .enqueue_player_use(cid, obj1, None, 0)
            .expect("bag on tile resolves");
        // Pop the `Wait{100}`.
        if let Some(k) = world.creatures.get_mut(cid) {
            k.base_mut().todo.queue.pop_front();
        }

        let kind = world.execute_creature_todo_action(cid);
        assert!(
            matches!(kind, Some(crate::idle_stimulus::TodoExecuteKind::Wait)),
            "no-path Use → Wait (RESULT catch applied)"
        );
        let base = world.creatures.get(cid).unwrap().base();
        // RESULT catch: ToDoClear + ToDoYield (Wait{0}).
        assert_eq!(
            base.todo.queue.len(),
            1,
            "RESULT catch → [Wait{{0}}] (ToDoYield)"
        );
        assert!(
            matches!(base.todo.queue[0], CreatureAction::Wait { delay_ms: 0 }),
            "yield enqueued Wait{{0}}"
        );
    }

    /// S5: `setup_player_walk_to_target` sets up the walk queue without clearing ToDo.
    /// Verifies the helper populates `walk_queue`/`walk_destinations` and does NOT
    /// touch the ToDo action queue (unlike `player_auto_walk_path` which clears).
    #[test]
    fn s5_setup_player_walk_to_target_populates_walk_queue_without_clearing_todo() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let target = Position::new(103, 100, 7);
        ensure_walkable_corridor_x(&mut world, 100, 103, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);

        // Pre-populate the ToDo queue to verify it's NOT cleared.
        if let Some(k) = world.creatures.get_mut(cid) {
            k.base_mut()
                .todo
                .queue
                .push_back(CreatureAction::Wait { delay_ms: 500 });
        }

        let now = std::time::Instant::now();
        let result = world.setup_player_walk_to_target(cid, target, now);
        assert!(result.is_ok(), "path to target exists");

        let base = world.creatures.get(cid).unwrap().base();
        assert!(
            !base.walk_queue.is_empty(),
            "walk_queue populated"
        );
        assert!(
            !base.walk_destinations.is_empty(),
            "walk_destinations populated"
        );
        assert_eq!(
            base.todo.queue.len(),
            1,
            "ToDo queue unchanged (Wait{{500}} still there)"
        );
        assert!(
            matches!(base.todo.queue[0], CreatureAction::Wait { delay_ms: 500 }),
            "ToDo queue not cleared by setup_player_walk_to_target"
        );
    }

    /// S5/D3: `Turn` execute arm with a not-adjacent map tile → `Go`-prepend.
    /// The queue becomes `[Go, Turn]` and `walk_queue` is populated. The builder
    /// prepends `Wait{100}` (`cract.cc:1345`), so the test drains it first (one
    /// `execute_creature_todo_action` returning `Wait`) before the `Turn` arm runs
    /// the `Go`-prepend — same shape as `s5_move_not_adjacent_prepends_go_and_re_enqueues_move`.
    /// C++ ref: `cract.cc:1340-1341` `ObjectInRange(1)` → `ToDoGo(...)`.
    #[test]
    fn s5_turn_not_adjacent_prepends_go_and_re_enqueues_turn() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(105, 100, 7);
        ensure_walkable_corridor_x(&mut world, 100, 105, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj = place_rotatable_on_tile(&mut world, item_pos, 5001, 5002);

        world
            .enqueue_player_turn(cid, obj)
            .expect("rotatable on tile resolves");

        // Drain the builder's `Wait{100}` floor first (`cract.cc:1345`).
        let drain_kind = world.execute_creature_todo_action(cid);
        assert!(
            matches!(drain_kind, Some(crate::idle_stimulus::TodoExecuteKind::Wait)),
            "builder Wait{{100}} drains first"
        );

        // Execute the `Turn` — should detect non-adjacency and Go-prepend.
        let kind = world.execute_creature_todo_action(cid);
        assert!(
            matches!(kind, Some(crate::idle_stimulus::TodoExecuteKind::Deferred)),
            "not-adjacent Turn → Deferred (Go-prepend)"
        );

        let base = world.creatures.get(cid).unwrap().base();
        assert_eq!(
            base.todo.queue.len(),
            2,
            "queue → [Go, Turn]"
        );
        assert!(matches!(base.todo.queue[0], CreatureAction::Go), "front = Go");
        assert!(
            matches!(base.todo.queue[1], CreatureAction::Turn { .. }),
            "second = Turn"
        );
        assert!(
            !base.walk_queue.is_empty(),
            "walk_queue populated for walk-to-reach"
        );
    }

    /// S5/D3: `Turn` execute arm with an adjacent map tile → no `Go`-prepend.
    /// Dispatches to `player_rotate_item` directly; the rotatable item transforms
    /// to `rotate_to` (mirrors `s5_use_adjacent_does_not_go_prepend`).
    #[test]
    fn s5_turn_adjacent_does_not_go_prepend() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        ensure_walkable_corridor_x(&mut world, 100, 101, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj = place_rotatable_on_tile(&mut world, item_pos, 5001, 5002);

        let item_id = world
            .resolve_rotate_item_id(cid, obj)
            .expect("rotatable item resolves");

        world
            .enqueue_player_turn(cid, obj)
            .expect("rotatable on tile resolves");
        // Drain the `Wait{100}`.
        if let Some(k) = world.creatures.get_mut(cid) {
            k.base_mut().todo.queue.pop_front();
        }

        let kind = world.execute_creature_todo_action(cid);
        // Adjacent → dispatches to `player_rotate_item` → Ok. Kind = Wait.
        assert!(
            matches!(kind, Some(crate::idle_stimulus::TodoExecuteKind::Wait)),
            "adjacent Turn → Wait (no Go-prepend)"
        );
        let base = world.creatures.get(cid).unwrap().base();
        assert!(
            base.todo.queue.is_empty(),
            "queue drained after adjacent Turn execute"
        );
        assert!(
            base.walk_queue.is_empty(),
            "no walk_queue for adjacent Turn"
        );
        // Verify the rotate actually fired (item transformed to `rotate_to`).
        assert_eq!(
            world.items.get(item_id).unwrap().item_type,
            5002,
            "adjacent Turn rotated the item"
        );
    }

    /// S5/D3: `Turn` execute arm with no path to target → `Err(ThereIsNoWay)` →
    /// RESULT catch. The catch clears the queue and enqueues `Wait{0}` (ToDoYield).
    /// Mirrors `s5_use_no_path_to_target_applies_result_catch`.
    #[test]
    fn s5_turn_no_path_to_target_applies_result_catch() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        // Rotatable on an isolated tile — no walkable path from player.
        let item_pos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, player_pos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, item_pos, TEST_SYNTHETIC_GROUND_WP);
        // No corridor between (100,100) and (105,100) — tiles 101-104 are missing.
        let cid = insert_test_player(&mut world, player_pos);
        let obj = place_rotatable_on_tile(&mut world, item_pos, 5001, 5002);

        world
            .enqueue_player_turn(cid, obj)
            .expect("rotatable on tile resolves");
        // Drain the `Wait{100}`.
        if let Some(k) = world.creatures.get_mut(cid) {
            k.base_mut().todo.queue.pop_front();
        }

        let kind = world.execute_creature_todo_action(cid);
        assert!(
            matches!(kind, Some(crate::idle_stimulus::TodoExecuteKind::Wait)),
            "no-path Turn → Wait (RESULT catch applied)"
        );
        let base = world.creatures.get(cid).unwrap().base();
        // RESULT catch: ToDoClear + ToDoYield (Wait{0}).
        assert_eq!(
            base.todo.queue.len(),
            1,
            "RESULT catch → [Wait{{0}}] (ToDoYield)"
        );
        assert!(
            matches!(base.todo.queue[0], CreatureAction::Wait { delay_ms: 0 }),
            "yield enqueued Wait{{0}}"
        );
    }

    // === F8 D2/D6 — z-floor gate tests ===
    // C++ ref: `cract.cc:1131-1135/1272-1276/1332-1336` `UPSTAIRS`/`DOWNSTAIRS`;
    //          `info.cc:233-257` `ObjectInRange` `posz == ObjZ` precondition.

    /// D2/D6 — `enqueue_player_use` rejects a map-tile source below the player
    /// (player z=6, item z=7 → player above object → `FirstGoDownStairs`).
    /// C++ ref: `cract.cc:1276-1278` `if(this->posz < ObjZ) throw DOWNSTAIRS`.
    #[test]
    fn enqueue_player_use_rejects_cross_floor_below_with_downstairs() {
        let mut world = beat_driven_test_world();
        // player z=6 < item z=7 → player above object → DOWNSTAIRS.
        let player_pos = Position::new(100, 100, 6);
        let item_pos = Position::new(101, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj1 = place_bag_on_tile(&mut world, item_pos);

        let result = world.enqueue_player_use(cid, obj1, None, 0);
        assert_eq!(
            result,
            Err(crate::return_value::ReturnValue::FirstGoDownStairs),
            "player above object (z=6 < z=7) → DOWNSTAIRS"
        );
        assert!(
            world.creatures.get(cid).unwrap().base().todo.is_empty(),
            "z-floor reject must not enqueue anything"
        );
    }

    /// D2/D6 — `enqueue_player_use` rejects a map-tile source above the player
    /// (player z=7, item z=6 → player deeper than object → `FirstGoUpStairs`).
    /// C++ ref: `cract.cc:1272-1276` `if(this->posz > ObjZ) throw UPSTAIRS`.
    #[test]
    fn enqueue_player_use_rejects_cross_floor_above_with_upstairs() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 6);
        let cid = insert_test_player(&mut world, player_pos);
        let obj1 = place_bag_on_tile(&mut world, item_pos);

        let result = world.enqueue_player_use(cid, obj1, None, 0);
        assert_eq!(
            result,
            Err(crate::return_value::ReturnValue::FirstGoUpStairs),
            "player deeper than object (z=7 > z=6) → UPSTAIRS"
        );
        assert!(
            world.creatures.get(cid).unwrap().base().todo.is_empty(),
            "z-floor reject must not enqueue anything"
        );
    }

    /// D2/D6 — `enqueue_player_use` accepts a same-z map-tile source (the
    /// existing happy path; this guards against the z-gate over-firing).
    #[test]
    fn enqueue_player_use_accepts_same_floor_map_tile() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj1 = place_bag_on_tile(&mut world, item_pos);

        world
            .enqueue_player_use(cid, obj1, None, 0)
            .expect("same-z map tile passes the z-floor gate");
        assert_eq!(
            world.creatures.get(cid).unwrap().base().todo.queue.len(),
            2,
            "same-z → [Wait{{100}}, Use]"
        );
    }

    /// D2/D6 — `enqueue_player_move` rejects a cross-floor map-tile source.
    /// C++ ref: `cract.cc:1131-1135`.
    #[test]
    fn enqueue_player_move_rejects_cross_floor_with_upstairs() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 6);
        let dest = Position::new(105, 100, 6);
        let cid = insert_test_player(&mut world, player_pos);
        let obj = place_gold_on_tile(&mut world, item_pos);

        // player z=7 > item z=6 → UPSTAIRS.
        let result = world.enqueue_player_move(cid, obj, dest, 1);
        assert_eq!(
            result,
            Err(crate::return_value::ReturnValue::FirstGoUpStairs),
            "Move cross-floor source → UPSTAIRS"
        );
        assert!(
            world.creatures.get(cid).unwrap().base().todo.is_empty(),
            "z-floor reject must not enqueue anything"
        );
    }

    /// D2/D6 — `enqueue_player_move` rejects a cross-floor source below the player.
    #[test]
    fn enqueue_player_move_rejects_cross_floor_with_downstairs() {
        let mut world = beat_driven_test_world();
        // player z=6, item z=7 → player above object → DOWNSTAIRS.
        let player_pos = Position::new(100, 100, 6);
        let item_pos = Position::new(101, 100, 7);
        let dest = Position::new(105, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj = place_gold_on_tile(&mut world, item_pos);

        let result = world.enqueue_player_move(cid, obj, dest, 1);
        assert_eq!(
            result,
            Err(crate::return_value::ReturnValue::FirstGoDownStairs),
            "Move cross-floor source → DOWNSTAIRS"
        );
        assert!(
            world.creatures.get(cid).unwrap().base().todo.is_empty(),
            "z-floor reject must not enqueue anything"
        );
    }

    /// D2/D6 — `enqueue_player_turn` rejects a cross-floor map-tile source.
    /// C++ ref: `cract.cc:1334-1338`.
    #[test]
    fn enqueue_player_turn_rejects_cross_floor_with_upstairs() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 6);
        let cid = insert_test_player(&mut world, player_pos);
        let obj = place_rotatable_on_tile(&mut world, item_pos, 5001, 5002);

        let result = world.enqueue_player_turn(cid, obj);
        assert_eq!(
            result,
            Err(crate::return_value::ReturnValue::FirstGoUpStairs),
            "Turn cross-floor source → UPSTAIRS"
        );
        assert!(
            world.creatures.get(cid).unwrap().base().todo.is_empty(),
            "z-floor reject must not enqueue anything"
        );
    }

    /// D2/D6 — `enqueue_player_turn` rejects a cross-floor source below the player.
    #[test]
    fn enqueue_player_turn_rejects_cross_floor_with_downstairs() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 6);
        let item_pos = Position::new(101, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        let obj = place_rotatable_on_tile(&mut world, item_pos, 5001, 5002);

        let result = world.enqueue_player_turn(cid, obj);
        assert_eq!(
            result,
            Err(crate::return_value::ReturnValue::FirstGoDownStairs),
            "Turn cross-floor source → DOWNSTAIRS"
        );
        assert!(
            world.creatures.get(cid).unwrap().base().todo.is_empty(),
            "z-floor reject must not enqueue anything"
        );
    }

    /// D2/D6 — inventory/container sources (`pos.x == 0xFFFF`) skip the z-gate.
    /// The z-floor only applies to map-tile sources (`cract.cc:1131` `if(ObjX != 0xFFFF)`).
    #[test]
    fn validate_action_object_z_floor_skips_inventory_source() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let cid = insert_test_player(&mut world, player_pos);
        // Inventory slot encoding: pos.x = 0xFFFF — z is irrelevant.
        let inv_obj = ActionObjectRef {
            pos: Position::new(0xFFFF, 0x40 | 1, 99),
            stack_pos: 0,
            sprite_id: 0,
        };

        let result = world.validate_action_object_z_floor(cid, inv_obj);
        assert!(
            result.is_ok(),
            "inventory source skips z-floor gate regardless of z"
        );
    }
}
