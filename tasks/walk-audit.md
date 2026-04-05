# Walk Smoothness Audit — Rust Port vs C++ TFS 1.4.2

**Date:** 2025-04-05  
**Symptom:** Stuttering, delays, and desync during player walking.

---

## Architecture Overview

### C++ (Reference)
- **3 threads:** Dispatcher (game logic), Scheduler (timer → dispatcher), Network I/O.
- **Scheduler** (`scheduler.cpp`): Each walk event gets a **dedicated `boost::asio::steady_timer`**. When the timer fires, it posts the callback to the Dispatcher immediately (`g_dispatcher.addTask`). The Dispatcher thread drains tasks as fast as they arrive (condition-variable wake, no fixed tick).
- **Packet send** (`connection.cpp`): `Connection::send()` calls `async_write` **immediately** on the IO thread — packets hit the wire the instant the Dispatcher executes `sendMoveCreature`.
- **Walk chain:** `addEventWalk` → `scheduler.addEvent(ticks, checkCreatureWalk)` → timer fires → dispatcher runs `onWalk` → move + send packet → `onWalk` tail calls `addEventWalk()` for next step.

### Rust (Port)
- **Single-threaded game loop** on a Tokio `LocalSet` (`game_loop.rs`).
- **`tokio::select!`** multiplexes three sources: command channel (`cmd_rx`), walk-wake channel (`walk_wake_rx`), and a 50ms tick timer.
- **Walk timer:** `sync_walk_timer_arm` spawns a `tokio::time::sleep_until` task that sends `CreatureId` on an unbounded channel when the deadline fires.
- **Packet send:** Packets are **buffered** in `pending_outgoing` (`HashMap<ConnId, Vec<Vec<u8>>>`) and only flushed to IO after each `select!` branch completes (`flush_pending_outgoing`).

---

## Identified Issues

### Issue 1 — `tokio::select!` Branch Starvation / Priority Inversion (HIGH)

**C++ behavior:** The Dispatcher is a simple FIFO — scheduler callbacks and network-originated tasks run in arrival order with no polling interval. A walk timer firing at exactly 250ms runs at ~250ms.

**Rust behavior:** `tokio::select!` picks **one** ready branch per iteration. If a `cmd` (packet from client) and a `walk_wake` fire simultaneously, only one runs. The other waits for the next loop iteration. Worse, the `tick_timer` (50ms) can preempt both — when the tick branch wins, walk wakes and incoming commands are delayed until the tick completes.

**Impact:** Walk steps can be delayed by up to one full select iteration (~0–50ms jitter on top of the intended delay). This is the most likely cause of **stuttering**.

**C++ equivalent:** There is no contention — scheduler timer fires → dispatcher runs callback → done. Sub-millisecond latency.

### Issue 2 — Packet Flush is Batched, Not Immediate (MEDIUM-HIGH)

**C++ behavior:** `ProtocolGame::sendMoveCreature` → `Connection::send` → `boost::asio::async_write` fires **immediately** on the IO thread. The client receives the move packet within the same millisecond the game logic executes it.

**Rust behavior:** `enqueue_outgoing` buffers into `pending_outgoing`. The buffer is only drained at the end of the `select!` branch via `flush_pending_outgoing`. If multiple walk steps resolve in the same branch (chained `ticks == 1` paths), all their packets batch together — the client receives them in a burst rather than spaced out.

**Impact:** The client gets move packets **later** than the server intended. OTClient's walk animation is timed from packet arrival. Late packets = visual stutter / snap. Burst packets = animation overlap.

### Issue 3 — `schedule_walk_followup_deadline` Deviates from C++ Reschedule Logic (MEDIUM)

**C++ behavior (`creature.cpp` ~228–233):**
```cpp
// In onWalk(), at the end:
if (eventWalk != 0) {
    eventWalk = 0;
    addEventWalk();  // reschedule from current state
}
```
`addEventWalk()` (no args) calls `getEventStepTicks(false)` which uses `getWalkDelay()` — the remaining time from `lastStep`. If `lastStep` was just set, walk delay = full step duration. The scheduler event fires after that delay.

**Rust behavior (`walk.rs` ~689–719):**  
`schedule_walk_followup_deadline` recomputes `get_event_step_ticks(false)` but anchors the deadline to `scheduling_base` (the instant the *previous* deadline fired), not to `Instant::now()`. This drift-free chaining is an **improvement** in theory, but differs from C++ where each `addEvent` delay is relative to "now" (the moment the Dispatcher runs the callback). If the Rust game loop ran late (e.g., a tick preempted it), the next deadline could be set **in the past**, causing an immediate re-fire and a too-fast step.

**Impact:** Walk rhythm drift — steps can bunch together or have uneven spacing vs C++.

### Issue 4 — `WALK_DEADLINE_GRACE` (2ms) Causes Re-buffering Delays (LOW-MEDIUM)

**C++ behavior:** No grace period. When the scheduler timer fires, the callback runs immediately.

**Rust behavior (`walk.rs` ~810–813):**
```rust
if fired_deadline > now + WALK_DEADLINE_GRACE {
    self.commit_next_walk_deadline(cid, Some(fired_deadline));
    return;
}
```
If the Tokio timer fires 2–3ms early (common with `sleep_until`), the walk is **re-queued** instead of executed. This adds an extra round-trip through the timer → channel → select loop, adding 1–50ms of unintended delay.

**Impact:** Sporadic extra delays on individual steps. Feels like micro-stutter.

### Issue 5 — `on_walk` Error Path Clears Queue Instead of Setting `forceUpdateFollowPath` (LOW)

**C++ behavior (`creature.cpp` ~207–213):**
```cpp
if (ret != RETURNVALUE_NOERROR) {
    player->sendCancelMessage(ret);
    player->sendCancelWalk();
    forceUpdateFollowPath = true;  // <-- retry path
}
```

**Rust behavior (`walk.rs` ~908–909):**
```rust
p.base.walk_queue.clear();
```
No `forceUpdateFollowPath` equivalent. For players this is less critical (no follow path), but if monsters use this path later, it will break chasing.

**Impact:** Low for players now; will be HIGH for monster walking.

### Issue 6 — `process_walk_deadlines` Polling Fallback Has Resolution Issues (LOW)

When `walk_wake_tx` is `None` (tests/fallback), `process_walk_deadlines` polls all creatures every 50ms tick. This means walk deadlines can be up to 50ms late. However, with `walk_wake_tx` set (production), this path is skipped — the Tokio timer path is used instead.

**Impact:** Low in production. High in test harnesses if walk timing is tested.

### Issue 7 — No `cleanup()` After Walk (LOW)

**C++ behavior (`game.cpp` ~3778):**
```cpp
void Game::checkCreatureWalk(uint32_t creatureId) {
    Creature* creature = getCreatureByID(creatureId);
    if (creature && creature->getHealth() > 0) {
        creature->onWalk();
        cleanup();  // <-- remove dead/removed creatures
    }
}
```

**Rust behavior:** `check_creature_walk` / `on_walk` have no cleanup pass.

**Impact:** Stale creature references could accumulate. Not directly a stutter cause but could lead to subtle bugs.

---

## Gaps (Missing from Rust Port)

| Feature | C++ Location | Status in Rust |
|---|---|---|
| `Creature::onCreatureMove` — map cache shift | `creature.cpp` ~524–603 | **Missing** — no `localMapCache` for monsters/NPCs |
| `Creature::onCreatureMove` — follow path instant recalc | `creature.cpp` ~619–656 | **Missing** — no follow creature walk update on move |
| `Game::cleanup()` after walk | `game.cpp` ~3778 | **Missing** |
| `forceUpdateFollowPath` on blocked walk | `creature.cpp` ~213 | **Missing** — queue cleared instead |
| `Creature::onWalkComplete` callback | `creature.cpp` ~219 | **Missing** |
| `Creature::onWalkAborted` on cancel | `creature.cpp` ~226–227 | **Partial** — `sendCancelWalk` sent but no Lua event |
| Monster / NPC walking | `creature.cpp` entire walk chain | **Missing** — `add_event_walk` / `on_walk` only handle `CreatureKind::Player` |
| `Condition` speed changes during walk (paralyze/haste swap with walk delay) | `creature.cpp` ~1278–1287, ~1330–1336 | **Missing** — no deferred condition add/remove on walk delay |

---

## Root Cause Summary (Ranked)

1. **`tokio::select!` contention** — Walk wakes compete with commands and ticks. C++ has no such contention.
2. **Batched packet flush** — Move packets don't hit the wire immediately; client animation timing is off.
3. **Grace period re-buffering** — Early timer wakes add extra delays.
4. **Drift-free chaining mismatch** — Anchoring to `scheduling_base` instead of wall clock can cause step bunching when the game loop is late.

## Recommended Investigation Order

1. **Instrument walk timing** — Log `Instant::now()` at each step in `on_walk` vs the intended `fired_deadline`. Measure actual step intervals vs expected `getStepDuration * lastStepCost`.
2. **Try immediate flush after walk wake** — Instead of batching, flush `pending_outgoing` for the walking player immediately after `on_walk`.
3. **Reduce or eliminate `WALK_DEADLINE_GRACE`** — Set to 0ms or 1ms and measure.
4. **Consider biased `select!`** — Give `walk_wake_rx` priority over `tick_timer` using `tokio::select! { biased; ... }`.
5. **Align reschedule to wall clock** — In `schedule_walk_followup_deadline`, use `Instant::now() + delay` instead of `scheduling_base + delay` to match C++ behavior exactly.
