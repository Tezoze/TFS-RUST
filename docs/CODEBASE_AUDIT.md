# TFS-RUST Codebase Audit

**Date:** 2026-06-06  
**Scope:** Game loop architecture, era split (772 vs 1098), mechanics profiles, refactoring doc claims, test/build health.

---

## Executive Summary

The codebase is **architecturally sound** for an 1098-first port with 772 mechanics layered via `MechanicsProfile`. It is **not yet correct** as a complete dual-loop, dual-era server.

| Area | Status |
|------|--------|
| Threading model (I/O vs game thread) | **Correct** |
| 1098 game loop (`run_game_loop`) | **Correct** — matches TFS reactive model |
| 772 game loop (beat-driven / ToDoQueue) | **Implemented (P2 MVP)** — `run_game_loop_772`; hybrid 50 ms `on_tick` for subsystems remains |
| 1098 mechanics / formulas | **Correct** — tests pass |
| 772 mechanics / formulas | **Broken in working tree** — `772.lua` drift |
| Walk timing code structure | **Correct** — profile-driven `step_speed` + `step_beat_ms` |
| Refactoring doc claims | **Mostly accurate** — minor line-count drift |
| Workspace compile | **Passes** (9 warnings in core) |
| Core unit tests | **187/187 pass** |
| Shipped formulas tests | **1/2 pass** — 772 fails |

**Three takeaways:**

1. **One active bug:** modified `data/formulas/772.lua` breaks formulas parity and walk timing.
2. **772 loop (P2):** `run_game_loop_772` + `ToDoQueue` + beat-end flush ship for `clientVersion = 772`; staggered subsystem counters (§3.4) still deferred.
3. **Docs may lag:** refresh `GAME_LOOP_ARCHITECTURE.md` §3 status when follow-ups land (staggered counters, multi-beat catch-up).

---

## 1. Critical: `772.lua` Drift

### Finding

Uncommitted change in `data/formulas/772.lua`:

```diff
-  stepBeatMs = 50,              -- TVP walk quantizer (`gameserver/src/creature.cpp`), not CipSoft Beat 200
+  stepBeatMs = 200,              -- TVP walk quantizer (`gameserver/src/creature.cpp`), not CipSoft Beat 200
```

### Impact

- Test `shipped_772_formulas_match_cipsoft_defaults` **fails** (expected `step_beat_ms: 50`, loaded `200`).
- A live 772 server loads this Lua file via `run_server.rs` → `load_mechanics`, so walk step durations quantize to **200 ms** instead of the intended **50 ms** (TVP `gameserver` authority).
- Example: wolf step duration shifts 950 ms → 1000 ms — small but observable.
- The comment contradicts the value: TVP `gameserver/src/creature.cpp` uses **50**, not 200.

### Action

**P0:** Revert `stepBeatMs` to `50`, **or** deliberately update built-in defaults, unit tests, and walk expectations if 200 ms CipSoft Beat quantization is the new parity target (that would be an explicit parity decision, not a typo fix).

---

## 2. Game Loop: Documentation vs Implementation

`docs/GAME_LOOP_ARCHITECTURE.md` line 6 states **“One binary, two loop modes”** — that is the **target**, not current reality.

| Claim (doc) | Code reality |
|-------------|--------------|
| `run_game_loop_1098` / `run_game_loop_772` | **Both exist** — `run_server.rs` branches on `StepSpeedModel` |
| 772: 200 ms beat timer | **`beat_ms` from profile** drives `run_game_loop_772` interval |
| 772: `ToDoQueue` + logical `server_ms` | **`todo_queue.rs` + `GameWorld::server_ms`** |
| 772: `walk_wake_tx = None` | **`None` for CipSoft** in `run_server.rs` |
| 772: beat-end-only flush (`SendAll`) | **`FlushPolicy::BeatEndOnly`** on 772 loop |
| 772: staggered ~1000 ms subsystem counters | **Not yet** — hybrid 50 ms `on_tick` still runs subsystems |

### What is correct today

**Section 2 (1098 reactive loop)** matches production code:

- `tokio::select! { biased; cmd, walk_wake, tick }`
- Per-creature `tokio::spawn` + `sleep_until` for walk deadlines (`walk.rs`)
- Immediate movement flush via `game_packet_needs_immediate_flush`
- 50 ms world tick with `GameWorld::on_tick`

This loop runs for **`clientVersion = 1098`** only (`StepSpeedModel::TfsLog`).

### What is implemented (P2 MVP — §3 core)

**Section 3 (772 beat-driven loop)** — shipped 2026-06-06:

- 200 ms beat timer (`profile.beat_ms`)
- Global `ToDoQueue` min-heap keyed on logical `server_ms`
- Consolidated output flush once per beat (`FlushPolicy::BeatEndOnly`)
- No per-creature Tokio walk timers on 772 (`walk_wake_tx = None`)

**Still deferred (post-P2):**

- Staggered subsystem counters (~1000 ms) — hybrid 50 ms `on_tick` remains
- Multi-beat lag catch-up when alarms pile up

### Recommended doc fix

Add a status banner to `GAME_LOOP_ARCHITECTURE.md`:

> **§2 = implemented (both eras today). §3 = target architecture (772 loop not yet built).**

---

## 3. Walk Timing and Beat Fields

`MechanicsProfile` correctly separates two concepts (`formulas.rs`):

| Field | 772 default | Used at runtime? | Purpose |
|-------|-------------|------------------|---------|
| `beat_ms` | 200 | **Yes** — `run_game_loop_772` beat interval | 772 main loop timer |
| `step_beat_ms` | 50 | **Yes** — `walk.rs:277` | Walk step duration quantization |

Walk timing branches correctly on `step_speed`:

```rust
// walk.rs — get_step_duration
let beat = mech.profile.step_beat_ms.max(1) as i64;
match mech.profile.step_speed {
    StepSpeedModel::CipSoft => {
        let delay = (gs as i64 * 1000) / i64::from(eff.max(1));
        ((delay + beat - 1) / beat) * beat   // ceil to step_beat_ms
    }
    StepSpeedModel::TfsLog => { /* TFS log formula + ceil to step_beat_ms */ }
}
```

### Authority split

| Source | Walk quantizer | Value |
|--------|----------------|-------|
| TVP `gameserver/src/creature.cpp` | `step_beat_ms` | **50 ms** (code default + intended shipped lua) |
| CipSoft decompile (`cract.cc` NotifyGo) | global `Beat` | **200 ms** |

The code intentionally follows **TVP gameserver** for walk quantization, not CipSoft decompile Beat. Stale internal docs (`tasks/lessons.md`, `tasks/todo.md`) still reference `beat_ms` for walk quantization — those should be updated to `step_beat_ms`.

---

## 4. Era Gating and Protocol Versioning

Per `.cursor/rules/TFS-protocol-versioning.mdc`, core mechanics should use `MechanicsProfile`, not scattered codec/version checks.

| Location | Issue | Severity |
|----------|-------|----------|
| `monster_ai.rs:919` | `is_772 = matches!(self.codec, Codec::V772(_))` for follow repath | **Medium** — should be a profile flag (e.g. `follow_repath_without_path`) |
| `walk.rs` | `clear_todo_772`, autowalk `first_only` gated on codec | **Borderline** — wire/client behavior, but lives in core walk scheduler |
| `creature_think.rs` | Same TFS 1 s bucketed think for both eras | **Medium gap** vs CipSoft IdleStimulus / ToDo model |
| `game_packet_needs_immediate_flush` | No era branch | **OK for now** — 1098 flush policy applies to both eras until 772 loop lands |

**Good:** No scattered `if version == 772` literals in core outside the profile loader (`formulas.rs`) and config parsing.

**Wire boundary (correct):** `login_out.rs`, codec modules, stackpos quirks — codec-gated as expected.

---

## 5. Refactoring Doc Verification

Audit of `docs/TFS-RUST_refactoring_opportunities.md`:

### Confirmed

| Claim | Live value |
|-------|------------|
| `monster_ai.rs` | 2,649 lines |
| `walk.rs` | 2,285 lines (doc says 2,257 — minor drift) |
| `items.rs` | 1,336 lines |
| `outgoing_extra.rs` | 1,048 lines |
| `monster_ai` > `game_world.rs` | 2,649 vs 2,477 |
| Duplicate `distance_x/y`, `offset_x/y` | Present in both `monster_ai.rs` and `monster_distance_step.rs` |
| Proposed split function names | All exist |
| Reference shapes (`container_ops.rs`, `monster_distance_step.rs`) | Accurate |

### Corrections needed in refactoring doc

- **`offset_x/y` dedupe is not “zero risk”** — the two files use **opposite sign conventions** (`to - from` vs `creature - target`). Requires argument swap or negation, not blind delete.
- **`player_inventory_query_add.rs` (1,140 lines)** should be on the list — 4th-largest crate file, omitted.
- **`game_world.rs` (2,477 lines)** is the next split target after `monster_ai`.
- **`walk_tile.rs` estimate (~300 lines)** is low — live block is ~440 lines.
- **`monster_targets.rs` estimate (~400 lines)** is low — live block is ~650–700 lines.

### Refactoring vs game loop

File-layout refactors (`monster_ai` split, distance helper dedupe, `walk.rs` split) **do not change** game loop architecture or runtime behavior.

---

## 6. Test and Build Health

```
cargo check --workspace              ✅ passes (9 warnings in tfs-rust-core)
cargo test -p tfs-rust-core --lib    ✅ 187/187 pass
cargo test -p tfs-rust-net             ✅ all pass
cargo test -p tfs-rust-core --test mechanics_formulas
  shipped_1098_formulas_match_era_defaults  ✅
  shipped_772_formulas_match_cipsoft_defaults  ❌ (772.lua drift)
```

Warnings are minor: unused variables in `monster_ai.rs`, dead code in `creature/kind.rs`.

---

## 7. What Is Correct and Should Stay As-Is

- **Hybrid threading:** Tokio I/O + single-threaded `GameWorld` via mpsc — correct and idiomatic Rust.
- **1098 loop:** cmd-first `biased` select, Turn→Move coalescing, walk wake timers — correct TFS parity.
- **Entity storage:** `SlotMap` + typed IDs throughout — correct.
- **MechanicsProfile:** one binary, era via profile not forked modules — correct pattern.
- **772 walk math:** CipSoft speed model + `step_beat_ms` quantization — correct structure (when `step_beat_ms = 50`).
- **Output queuing:** game thread buffers, I/O thread drains — correct for current implementation.

---

## 8. 772 Loop Design Notes (When Implemented)

The planned 772 loop in `GAME_LOOP_ARCHITECTURE.md` §3 is the efficient Rust mapping of CipSoft’s design:

| CipSoft behavior | Planned Rust approach |
|------------------|----------------------|
| SIGUSR1 / SIGALRM wake | `tokio::select! { biased; cmd, beat }` |
| `ToDoQueue` min-heap | `BinaryHeap<Reverse<ToDoEntry>>` on game thread |
| `ServerMilliseconds` | `server_ms: u64` logical clock |
| `SendAll` once per beat | `flush_pending_outgoing` + `game_packet_needs_immediate_flush() → false` |
| Input between beats | Process immediately; flush only at beat end |

**Implementation refinements beyond the doc pseudocode:**

1. **Drain all input on wake** — `try_recv` loop after cmd branch (CipSoft `ReceiveData` drains all).
2. **Catch up missed beats** — C++ `AdvanceGame(NumBeats * Beat)` when alarms pile up.
3. **Stale heap entry guards** — compare popped `execution_time` to creature’s current `next_wakeup`.
4. **Explicit tie-breaking** — `(execution_time, creature_id)` ordering in `ToDoEntry`.
5. **Typed creature ToDo queue** — enum-based action list, not C++ vector transcription.
6. **Shared command dispatch** — extract `process_game_command`; do not duplicate the `GameCommand` match.
7. **Use `beat_ms` for main loop timer**, `step_beat_ms` for walk quantization — do not conflate.

**Anti-patterns to avoid:**

- Keeping `walk_wake_tx` **and** `ToDoQueue` for 772 (dual schedulers).
- Spawning Tokio tasks inside the 772 loop.
- O(n) creature scan for due walks instead of heap drain.
- Immediate movement flushes in 772 mode (wrong parity and extra I/O).

---

## 9. Priority Action List

| Priority | Item | Why |
|----------|------|-----|
| **P0** | Resolve `772.lua` `stepBeatMs` drift | Active test failure + wrong live server walk timing |
| **P1** | Add status banner to `GAME_LOOP_ARCHITECTURE.md` | Prevents reading target architecture as shipped |
| **P2** | ~~Implement 772 loop (`run_game_loop_772`)~~ | **Done** — beat timer, ToDoQueue, beat-end flush, walk scheduling |
| **P3** | Move `monster_ai` codec check to profile knob | Protocol versioning hygiene |
| **P4** | Staggered 772 subsystem counters (~1000 ms) | Replace hybrid 50 ms `on_tick` on beat loop |
| **P5** | Update refactoring doc corrections | Minor accuracy fixes |
| **P6** | Update stale docs referencing `beat_ms` for walk quant | `tasks/lessons.md`, `tasks/todo.md` |

---

## 10. File Reference Index

| Path | Relevance |
|------|-----------|
| `crates/tfs-rust-core/src/game_loop.rs` | `run_game_loop_1098` + `run_game_loop_772` + shared dispatch |
| `crates/tfs-rust-core/src/todo_queue.rs` | Min-heap ToDoQueue for 772 walk scheduling |
| `crates/tfs-rust-core/src/run_server.rs` | Era branch: `walk_wake_tx` + loop selection |
| `crates/tfs-rust-core/src/walk.rs` | Step duration, wake timers, CipSoft/TFS speed models |
| `crates/tfs-rust-core/src/formulas.rs` | `beat_ms` / `step_beat_ms` definitions and loader |
| `crates/tfs-rust-core/src/game_world.rs` | `on_tick` pipeline |
| `crates/tfs-rust-core/src/creature_think.rs` | Think bucket timing (shared both eras) |
| `crates/tfs-rust-core/src/monster_ai.rs` | Codec-gated 772 follow repath |
| `data/formulas/772.lua` | `stepBeatMs` drift (200 vs expected 50) |
| `data/formulas/1098.lua` | Matches built-in defaults |
| `docs/GAME_LOOP_ARCHITECTURE.md` | §2 current + §3 target (mixed if read as one) |
| `docs/TFS-RUST_refactoring_opportunities.md` | Refactoring backlog (mostly accurate) |
| `crates/tfs-rust-core/tests/mechanics_formulas.rs` | Shipped formulas parity tests |

---

## Related Documents

- [`GAME_LOOP_ARCHITECTURE.md`](GAME_LOOP_ARCHITECTURE.md) — threading model and loop design (§2 implemented, §3 target)
- [`TFS-RUST_refactoring_opportunities.md`](TFS-RUST_refactoring_opportunities.md) — file-size refactoring backlog
- [`PROTOCOL_VERSIONING.md`](PROTOCOL_VERSIONING.md) — wire vs mechanics axes, era authorities
