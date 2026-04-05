# Walk Fix TODO — Stuttering, Delays & Desync

Ordered by impact and dependency. Each fix references the audit in `tasks/walk-audit.md`.

---

## Phase 1 — Immediate Timing Fixes (Stutter / Delay) — complete

- [x] **1. Use `biased` `tokio::select!` — give walk wakes priority**
  - **File:** `crates/tfs-rust-core/src/game_loop.rs` (~line 111)
  - **Why:** `tokio::select!` randomly picks a ready branch. Walk timer wakes compete with the 50ms tick and command channel, adding 0–50ms jitter per step. `biased` mode checks branches top-to-bottom — put `walk_wake_rx` first.
  - **Audit ref:** Issue 1

- [x] **2. Flush packets immediately after walk wake, not just at branch end**
  - **File:** `crates/tfs-rust-core/src/game_loop.rs` (~line 246–248)
  - **Why:** Move packets sit in `pending_outgoing` until `flush_pending_outgoing` runs at the end of the `select!` branch. C++ calls `async_write` inline — the client gets the packet in the same ms the move executes.
  - **Change:** Already flushing after walk wake — verify no extra buffering layer exists between `flush_pending_outgoing` and the TCP write. Check `OutRegistry` → per-connection writer latency.
  - **Audit ref:** Issue 2

- [x] **3. Remove or reduce `WALK_DEADLINE_GRACE` from 2ms to 0ms**
  - **File:** `crates/tfs-rust-core/src/walk.rs` (~line 29)
  - **Why:** When Tokio's `sleep_until` fires 1–2ms early (normal jitter), the grace check re-queues the walk instead of executing it. This adds a full extra round-trip (timer → channel → select → timer again). C++ has no grace period.
  - **Audit ref:** Issue 4

- [x] **4. Align reschedule anchor to wall clock (`Instant::now()`) instead of `scheduling_base`**
  - **File:** `crates/tfs-rust-core/src/walk.rs` — `schedule_walk_followup_deadline` (~line 717), `add_event_walk` (~line 775–778)
  - **Why:** C++ `addEventWalk` schedules relative to `OTSYS_TIME()` at the moment the Dispatcher runs the callback. Rust anchors to the prior deadline — if the game loop ran late, the next deadline is in the past, causing an immediate re-fire (too-fast step) or bunching.
  - **Change:** Replace `scheduling_base + Duration::from_millis(delay_ms)` with `Instant::now() + Duration::from_millis(delay_ms)` in the non-`first_step` path to match C++ semantics.
  - **Audit ref:** Issue 3

---

## Phase 2 — Correctness Fixes (Desync) — complete

- [x] **5. Add `onWalkComplete` callback when walk queue empties**
  - **File:** `crates/tfs-rust-core/src/walk.rs` — `on_walk` (~line 936)
  - **C++ ref:** `src/creature.cpp` ~215–219: `stopEventWalk` / `onWalkComplete()` when `getNextStep` returns false and queue empty.
  - **Why:** Missing callback means Lua `onWalkComplete` events never fire. Not a stutter cause now, but will be needed for scripting compatibility.

- [x] **6. Implement `forceUpdateFollowPath` on blocked walk instead of clearing queue**
  - **File:** `crates/tfs-rust-core/src/walk.rs` — `on_walk` error branch (~line 908–909)
  - **C++ ref:** `src/creature.cpp` ~213: `forceUpdateFollowPath = true` when `internalMoveCreature` returns not `RETURNVALUE_NOERROR`.
  - **Why:** Clearing the queue prevents path retry. Will be critical for monster following.

- [x] **7. Add `cleanup()` pass after walk step**
  - **File:** `crates/tfs-rust-core/src/walk.rs` — after `on_walk` completes
  - **C++ ref:** `src/game.cpp` ~3773–3778: `cleanup()` after `creature->onWalk()`.
  - **Why:** Stale creature references from death/removal during walk callbacks.

---

## Phase 3 — Monster / NPC Walking Foundation

- [ ] **8. Extend `add_event_walk` / `on_walk` / `check_creature_walk` to handle `CreatureKind::Monster` and `CreatureKind::Npc`**
  - **Files:** `crates/tfs-rust-core/src/walk.rs` — all `GameWorld` impl methods
  - **C++ ref:** `creature.cpp` — the entire walk chain is creature-generic, not player-specific.
  - **Why:** Currently all walk logic early-returns on non-Player creatures. Monsters and NPCs cannot walk.

- [ ] **9. Port `Creature::onCreatureMove` map cache shifting**
  - **C++ ref:** `creature.cpp` ~524–603 — `localMapCache` memcpy shifts on cardinal move, full rebuild on teleport/floor change.
  - **Why:** Monster pathfinding (`getPathTo`) uses `localMapCache` for `getWalkCache`. Without it, monsters will pathfind blind.

- [ ] **10. Port follow-creature walk update on target move**
  - **C++ ref:** `creature.cpp` ~619–656 — `onCreatureMove` recalculates follow path when target or self moves.
  - **Why:** Monsters chasing players need instant path updates when the player moves.

---

## Phase 4 — Condition Interaction with Walking

- [ ] **11. Port deferred condition add/remove on walk delay (paralyze ↔ haste)**
  - **C++ ref:** `creature.cpp` ~1278–1287 (haste deferred while paralyzed + walking), ~1330–1336 (paralyze removal deferred while walking).
  - **Why:** Without this, applying haste while paralyzed and walking causes an immediate speed jump instead of waiting for the current step to finish. Visible desync.

---

## Instrumentation (Optional, for Validation)

- [ ] **12. Add walk timing telemetry**
  - Log `fired_deadline`, `Instant::now()`, `step_duration`, `walk_delay` at each `on_walk` execution.
  - Compare actual step intervals to expected `getStepDuration * lastStepCost`.
  - Use to validate fixes 1–4 before/after.
