# TFS-RUST 7.72 Monster AI — Dance & Chase Parity Audit

**Project**: CipSoft 7.72 monster movement feel (melee dance, chase repath, diagonal steps)  
**Date**: 2026-06-06  
**Scope**: `monster_ai.rs`, `walk/mod.rs`, `idle_stimulus.rs`, `creature_todo.rs`, `pathfinding.rs`  
**References**: CipSoft `crnonpl.cc` (~2731–2753 dance), `cract.cc` (`ToDoStart`, `NotifyGo`, `TShortway`)

---

## Executive Summary

Observed in-game symptoms:

1. **`rand() % 5` melee dance fires too often** while a monster is on a player.
2. **Diagonal chase steps happen easily** when the player moves away.
3. **Diagonal steps look fast** (instant snap rather than slow 3× waypoint stride).

Root cause is **not** the dance direction selection logic — the 772 cardinal `rand() % 5` block is implemented correctly for **one narrow case** (melee adjacent). The server does **not** flip back to 1098 at runtime; `beat_driven_loop` stays true for 772. Instead, movement is a **hybrid stack**: a thin CipSoft layer (`idle_stimulus`, `rand() % 5`) sits on top of shared TFS ports (`go_to_follow_creature`, `on_walk`, `add_event_walk`). Anything not explicitly gated with `if !beat_driven_loop` still runs TFS logic on every 772 tick.

The gaps are in **scheduling and retry cadence**: 772 monsters re-poll dance on stand-still every ~1 logical ms instead of once per full step delay, and chase repaths fire the first queued step immediately via `add_event_walk`, bypassing the todo/`ToDoStart` delay model.

| Symptom | Root cause | CipSoft expectation |
|---------|------------|---------------------|
| Dance too frequent | `request_idle_stimulus` + `schedule_immediate_todo_wakeup` on stand-still | One attempt per full step delay (~500–600 ms for typical rat) |
| Diagonal on player move | Instant first step after repath + A* `allow_diagonal: true` | `ToDoStart` first-step delay; diagonals rare (3× path cost) |
| Diagonal feels fast | No pre-step wait on repath; jittery rapid re-polls | Full diagonal duration (3× waypoints before beat ceil) |

**Priority fixes**: (1) dance retry delay, (2) route 772 chase through todo queue, (3) first-step vs continuing-step timing.

Related: [`TFS-RUST_772_Pathfinding_Creature_AI_Analysis.md`](TFS-RUST_772_Pathfinding_Creature_AI_Analysis.md), [`IDLE_STIMULUS.md`](IDLE_STIMULUS.md), [`772_MONSTER_AI_TARGETING_PATHING_GAP_AUDIT.md`](772_MONSTER_AI_TARGETING_PATHING_GAP_AUDIT.md).

---

## Why It Keeps Falling Back to TFS

772 does **not** disable TFS monster AI and swap in a parallel CipSoft module. The port added **spot gates** (`if !beat_driven_loop`) inside shared functions. Un-gated code paths run identically on both eras.

### Era flag (always on for 772)

```rust
// game_world.rs — set once at startup, never toggled per-monster
let beat_driven_loop =
    mechanics.profile.step_speed == StepSpeedModel::CipSoft;
```

When `clientVersion = 772`, this is always `true`. Fallback to TFS is **architectural**, not a config mistake.

### Hybrid stack (mental model)

```
772 decision layer (partial)     idle_stimulus, rand()%5 melee dance
         ↓ calls
TFS-shaped chase API (shared)    go_to_follow_creature, monster_next_walk_step
         ↓ calls
TFS walk scheduler (shared)      add_event_walk, on_walk, creature_start_chase_auto_walk
         ↓ calls
Era-mixed pathfinding            Reverse TShortway → forward TFS A* fallback
```

772 added a decision trigger (`IdleStimulus` on todo drain) and one dance selector (`rand() % 5`). It did **not** fork walk execution, chase repath, or pathfinding into separate 772 modules.

### Every TFS path still reachable on 772

| Layer | Shared TFS code | Gated for 772? | Notes |
|-------|-----------------|----------------|-------|
| Walk step machine | `on_walk`, `internal_move_creature_step` | **No** | All steps go through `creature.cpp` port |
| Chase repath arm | `creature_start_chase_auto_walk` → `add_event_walk` | **No** | Immediate first step on every A* repath |
| Chase API | `go_to_follow_creature` | **Partial** | Only `get_distance_step` / greedy chase gated |
| A* relaxed retry | `monster_try_apply_chase_path` relaxed FPP | **No** | TFS-style second-chance search |
| A* forward fallback | `path_matching_forward` after reverse fail | **No** | TFS `dirNeighbors` expansion |
| Melee dance | `rand() % 5` | **772 only** | Only when `dance_range == 1 && dist == 1` |
| Keep-distance dance | `get_dance_step` | **No** | Fall-through when `dance_range > 1` |
| Flee dance | `get_dance_step` | **No** | `if fleeing` block has no 772 gate |
| Retarget / look | `monster_on_think_target`, `updateLookDirection` | **No** | Called from `idle_stimulus`; TFS cadence |
| Stand-still retry | `request_idle_stimulus` in `on_walk` | **772-only bug** | 1098 uses `schedule_walk_followup_deadline` instead |

### `get_dance_step` fall-through (not fully replaced)

The 772 `rand() % 5` block sits inside `monster_next_walk_step` but only handles **melee adjacent**:

```rust
if self.beat_driven_loop {
    if dance_range == 1 && dist == 1 {
        // cardinal rand() % 5 — CipSoft
        return None; // or Some(cardinal)
    }
}
if fleeing {
    return get_dance_step(...);  // TFS — always
}
if static_attack_chance < roll {
    return get_dance_step(...);  // TFS — always (keep-distance at band, etc.)
}
```

| Situation on 772 | Dance logic used |
|------------------|------------------|
| Melee adjacent (`dist == 1`, `dance_range == 1`) | CipSoft `rand() % 5` |
| Keep-distance at band (`dist == targetDistance`, e.g. 4) | **TFS `get_dance_step`** (diagonals allowed) |
| Fleeing while adjacent | **TFS `get_dance_step`** |
| Any non-melee dance after `staticAttackChance` roll | **TFS `get_dance_step`** |

### `go_to_follow_creature` — gated vs shared arms

The whole function is TFS `creature.cpp` ~1011 shape. Only some arms check era:

| Step | 772 |
|------|-----|
| `get_distance_step` (diagonal chase/flee one-step) | **Off** (`!beat_driven_loop`) |
| `monster_try_apply_chase_path` (A* primary + relaxed) | **On** — shared |
| `creature_start_chase_auto_walk` after A* | **On** — TFS scheduler, **not gated** |
| `monster_try_any_closer_step` | **On** — cardinal-only on 772 |
| `monster_try_greedy_chase_step` | **Off** (`!beat_driven_loop`) |
| A* failure → clear path | **On** — no CipSoft `NOWAY` stop |

### Pathfinding silent TFS fallback

772 primary search is CipSoft reverse `TShortway`. When origin is unreachable (tree/wall between monster and player):

```rust
// pathfinding.rs — after reverse returns empty
path_matching_forward(...)  // TFS dirNeighbors expansion
```

Any blocked-geometry chase can silently swap from CipSoft reverse to TFS forward A* without logging or era check.

### End-to-end flow: player moves, rat on you (melee)

```
Player moves
  → monster_on_follow_creature_moved (772 hysteresis)
  → request_idle_stimulus
  → idle_stimulus → go_to_follow_creature          [TFS-shaped API]
      → monster_try_apply_chase_path               [A* shared]
      → creature_start_chase_auto_walk             [TFS scheduler — NOT gated]
      → add_event_walk(first=true) → on_walk       [TFS step machine]
          → pop queue step OR monster_next_walk_step
              → rand()%5 if melee adjacent         [CipSoft — narrow gate]
              → else get_dance_step                [TFS]
          → stand still?
              → request_idle_stimulus (772)        [wrong retry path]
              → NOT schedule_walk_followup_deadline [1098 would do this]
```

### What stops the constant fallback pattern

Not more scattered `if 772` checks — **route 772 through one pipeline**:

1. Chase steps only via todo `Go` + `todo_start_go_delay` (drop `creature_start_chase_auto_walk` on beat-driven).
2. Extend cardinal dance / CipSoft flee to **all** 772 dance paths (remove `get_dance_step` fall-through).
3. Stand-still → `schedule_walk_followup_deadline` (same as 1098 dance-alive path).
4. Optional: no forward A* fallback on 772 (CipSoft `NOWAY`); `allow_diagonal: false` for melee FPP.

---

## What Is Already Correct (772)

### Melee dance selection — cardinal `rand() % 5`

For `beat_driven_loop` + adjacent melee (`dance_range == 1`, `dist == 1`), `monster_next_walk_step` uses CipSoft-style cardinal-only selection:

```rust
// monster_ai.rs — mirrors crnonpl.cc:2731–2753
let choice = rng.gen_range(0..5);
// West, East, North, South, stand still (20% each)
```

Test: `test_772_melee_dance_only_cardinal` — 100 samples, all steps cardinal or None.

### TFS diagonal fallbacks — partially gated

| Path | 772 status |
|------|------------|
| `get_distance_step` in `go_to_follow_creature` | Gated: `!beat_driven_loop` |
| `monster_try_greedy_chase_step` | Gated: `!beat_driven_loop` |
| `monster_try_any_closer_step` | Cardinal-only when `beat_driven_loop` |
| TFS `get_dance_step` at **melee adjacent** (`dist==1`, `dance_range==1`) | Replaced by `rand() % 5` |
| TFS `get_dance_step` keep-distance / flee / post-`staticAttackChance` | **Still active** on 772 |
| `creature_start_chase_auto_walk` / `add_event_walk` | **Not gated** — TFS scheduler on all repaths |
| Forward A* fallback after reverse fail | **Not gated** — TFS pathfinder |

### Pathfinding core

Reverse A* (`TShortway`), terrain-weighted costs, diagonal ×3 in expansion — see pathfinding analysis doc (~95% faithful). On uniform terrain, chase paths are cardinal-only (`test_772_path_to_diagonal_target`).

### Walk timing math

CipSoft diagonal duration uses 3× waypoints **before** quantizer ceil (`walk_timing.rs`). Example: ground 150 → cardinal ~950 ms, diagonal ~2750 ms (not TFS post-ceil ×3).

---

## Gap 1 — Dance Retry Cadence (Critical)

### Observed behavior

Monster adjacent to player appears to sidestep or twitch constantly. `rand() % 5` feels like it fires every frame rather than ~20% per walk beat.

### Mechanism

**Correct dance logic, wrong retry path.**

When dance returns `None` (stand still — 80% of rolls — or blocked direction), `on_walk` takes the 772 branch:

```
on_walk → getNextStep false
       → request_idle_stimulus (immediate)
       → idle_stimulus → enqueue Go
       → todo_start_go_delay(first_step=true)
       → ticks == 1 when walk_delay == 0
       → schedule_immediate_todo_wakeup (server_ms + 1)
       → new rand() % 5 roll ~1 ms later
```

**1098 uses the correct pattern** for the same situation:

```
on_walk → getNextStep false
       → schedule_walk_followup_deadline
       → get_event_step_ticks(false, ...) with full step duration
```

### Code locations

| File | Symbol | Issue |
|------|--------|-------|
| `walk/mod.rs` | `on_walk` ~1237–1243 | 772 always `request_idle_stimulus` instead of `schedule_walk_followup_deadline` |
| `creature_todo.rs` | `schedule_immediate_todo_wakeup` | Fires at `server_ms + 1` when `ticks == 1` |
| `idle_stimulus.rs` | `idle_enqueue_go_and_start(cid, true)` | Always passes `first_step=true` for dance continuations |

### CipSoft reference

- `IdleStimulus` enqueues `ToDoGo` once per drain cycle.
- `ToDoStart` / `CalculateDelay` (`cract.cc:846`, `cract.cc:955`) separates first-step vs continuing-step delay.
- Dance attempt rate ≈ **1 / step_duration**, not ~1000/sec.

### Recommended fix

1. When 772 monster is in dance/chase-alive state and `getNextStep` returns false, call `schedule_walk_followup_deadline` (same as 1098 `monster_should_keep_dance_walk_alive` path).
2. Use `first_step=false` for dance continuation enqueues from idle.
3. Do **not** call `request_idle_stimulus` synchronously from `on_walk` when the only outcome was stand-still — defer to the scheduled wakeup.

---

## Gap 2 — Chase First Step Bypasses Todo Queue (High)

### Observed behavior

When the player moves while a monster is on them, the monster snaps diagonally (or steps immediately) without the expected pre-move pause.

### Mechanism

`go_to_follow_creature` → `monster_try_apply_chase_path` → `creature_start_chase_auto_walk`:

```rust
// walk/mod.rs
pub(crate) fn creature_start_chase_auto_walk(&mut self, cid: CreatureId) {
    self.add_event_walk(cid, true, Instant::now());  // immediate first step
}
```

On 772, this bypasses the per-creature todo action queue (`CreatureAction::Go` → `Execute` → `ToDoStart`). The first repath step runs synchronously via `check_creature_walk_from_add_event_walk` with `first_step=true` and `ticks == 1`.

### CipSoft reference

All movement intents go through `ToDoGo` + `ToDoStart`. First step after a new path has a distinct delay from continuing steps on an existing path.

### Recommended fix

For `beat_driven_loop` monsters, replace `creature_start_chase_auto_walk` with `idle_enqueue_go_and_start` (or equivalent) so repath steps respect `todo_start_go_delay`. Wire `first_step` correctly: `true` for new path, `false` for queue continuations.

---

## Gap 3 — Diagonal Steps in Chase (Medium)

### Observed behavior

Monsters take diagonal steps more often than remembered from real 7.72, especially when the player kites diagonally.

### Sources (ranked by visual impact)

| Source | Frequency | Diagonal bias | 772 status |
|--------|-----------|---------------|------------|
| Instant repath first step | Every target move off-band | Medium | Active — Gap 2 |
| A* with `allow_diagonal: true` | Every path calc | Low–medium | Active — CipSoft allows but 3× cost |
| Forward A* fallback | When reverse search fails | Medium | Active |
| TFS `get_dance_step` | Keep-distance / ranged at band | High | **Still active** when `dance_range > 1` |
| Hysteresis stale queue | Player moves within Chebyshev 1 | Medium | May execute wrong cardinal step before repath |

### A* diagonals vs CipSoft

CipSoft `TShortway::Expand` **does** expand diagonals with 3× waypoint cost (`cract.cc:136–155`). Diagonal chase steps are valid but should be **rare** on open terrain because cardinal routes cost less. On uniform ground, Rust paths are cardinal-only (verified by test).

Diagonals appear more often when:

- Reverse search fails → forward fallback with TFS `dirNeighbors` bias
- Geometry makes diagonal tie or shortcut
- Player move triggers instant first step before timing settles

### Optional tightening

Gate `allow_diagonal: false` in `monster_path_search_params` when `beat_driven_loop && target_distance <= 1` for stricter cardinal-only melee chase (may diverge from CipSoft A* but matches observed 7.72 feel).

---

## Gap 4 — Keep-Distance & Flee Dance Still Use TFS (Medium)

For ranged / keep-distance monsters (`target_distance > 1`, `dist == band`), the 772 `rand() % 5` guard fails (`dance_range == 1` is false), so execution **falls through** to TFS `get_dance_step` via the `staticAttackChance` roll — lateral and diagonal fallback steps included.

**Target fix (P2/P6):** CipSoft cardinal band dance at `dist == monster_effective_target_distance(...)`, not TFS dance. Band comes from XML `targetDistance` (see [§Keep-distance band design](#keep-distance-band-design)) — not hardcoded 4.

Fleeing monsters hit `get_dance_step` directly in the `if fleeing` block with **no** `beat_driven_loop` gate at all.

CipSoft distance-fighters use explicit keep-band logic (`SearchFlightField`, step away/closer, cardinal dance at band). CipSoft flee uses `SearchFlightField`, not TFS `getDanceStep`. See [§Keep-distance band design](#keep-distance-band-design) and [`772_MONSTER_AI_TARGETING_PATHING_GAP_AUDIT.md`](772_MONSTER_AI_TARGETING_PATHING_GAP_AUDIT.md) P1.

Note: `search_flight_field` exists in `monster_distance_step.rs` but is **not wired** into 772 flee/chase yet.

---

## Gap 5 — First-Step vs Continuing-Step Timing (Medium)

Flagged in pathfinding analysis doc. `idle_enqueue_go_and_start` always passes `first_step=true`. CipSoft `CalculateDelay` distinguishes starting a new path from continuing an queued path.

Impact: continuing chase segments and dance retries may get wrong delay (too fast or too slow).

---

## Scheduling Model Comparison

```
CipSoft 7.72:
  ToDo drain → Execute(Go) → step → drain empty → IdleStimulus
           → enqueue ToDoGo → ToDoStart(delay) → heap wakeup

Rust 772 today (dance):
  ToDo drain → Execute(Go) → rand()%5 → stand still
           → request_idle_stimulus (sync, no delay)
           → enqueue Go → wakeup +1ms → repeat

Rust 772 today (chase repath):
  IdleStimulus → go_to_follow_creature → A* queue
              → creature_start_chase_auto_walk (bypass todo)
              → add_event_walk(first=true) → immediate step
              → schedule_walk_followup_deadline for rest

Rust 1098 (reference for dance retry):
  on_walk → getNextStep false → schedule_walk_followup_deadline
           → full step delay before next poll
```

---

## Parity Scorecard

| Area | Fidelity | Notes |
|------|----------|-------|
| `TShortway` reverse A* | ~95% | Excellent |
| Cardinal melee dance **selection** | ~95% | `rand() % 5` correct |
| Dance **frequency** | ~20% | ~1 ms retry — critical gap |
| Chase first-step delay | ~30% | Bypasses todo |
| TFS diagonal fallbacks | ~60% | Only chase one-step helpers gated; dance/path/scheduler not |
| IdleStimulus decision model | ~75% | Drain-triggered idle ✓; chase arm inconsistent |
| Keep-distance / flee dance | ~50% | Still TFS `get_dance_step`; `SearchFlightField` unwired |
| Hybrid architecture clarity | N/A | Single stack with spot gates — see §Why It Keeps Falling Back |
| Overall movement **feel** | ~70–75% | Dominated by Gaps 1–2 |
| **Whole monster stack** (move + target + combat) | ~60% | Targeting P0 gaps; combat loop not wired |

### Parity milestones (projected)

| Milestone | Melee feel | Full movement | Targeting | Whole monster |
|-----------|------------|---------------|-----------|---------------|
| **Today** | ~72% | ~60% | ~60% | ~60% |
| After **P1** (scheduler) | ~85% | ~70% | ~60% | ~62% |
| After **P1–P3** (movement) | ~92% | ~88% | ~60% | ~65% |
| After **P1–P6** (+ todo Attack, targeting, distance) | ~95% | ~95% | ~90% | ~80% |
| After **P1–P8** (combat + soak) | ~98% | ~98% | ~98% | **~98%** |

The last ~2% is decompile ambiguity, map edge cases, and divergences only caught by live soak — not more scattered gates.

---

## Recommended Fix Order

### Immediate (highest feel impact)

1. **Dance retry delay** — Replace 772 `request_idle_stimulus` in `on_walk` stand-still path with `schedule_walk_followup_deadline` when `monster_should_keep_dance_walk_alive`.
2. **Todo-only chase on 772** — Stop calling `creature_start_chase_auto_walk` for beat-driven monsters; enqueue `Go` via idle/todo path.
3. **first_step flag** — `true` for new path intent, `false` for queue continuation and dance retry.

### Medium term

4. Gate `get_dance_step` off 772 entirely; wire `search_flight_field` for flee; cardinal band dance via `targetDistance` (not hardcoded 4).
5. Remove or gate forward A* fallback on 772 (CipSoft `NOWAY` when reverse fails).
6. Optional: `allow_diagonal: false` for 772 melee chase FPP.
7. Hysteresis audit: repath when end-of-queue position no longer minimizes distance to target, not only `monster_at_follow_goal`.

---

## Proper 772 Gating Plan

**Goal:** 772 monsters never execute TFS-only movement/dance/path arms unless CipSoft explicitly matches that behavior.

**Wrong approach:** Add more `if self.beat_driven_loop` at random call sites — this is how the current hybrid got into partial-gate hell.

**Right approach:** Two layers, applied in order:

1. **Route** — On `beat_driven_loop`, all monster walk intent goes through todo (`Go` → `Execute` → `ToDoStart`). No direct `add_event_walk` / `creature_start_chase_auto_walk` for monsters.
2. **Gate** — At the **decision boundary** (where direction/path is chosen), branch to CipSoft helpers or TFS helpers. One gate per concern, not per callsite downstream.

Use existing `beat_driven_loop` (derived from `MechanicsProfile::step_speed == CipSoft`). Do **not** add `*_772` function names or parallel modules — project rules forbid era suffixes on public APIs.

### Layer 1 — Scheduler routing (must fix first)

| Call site | Today | 772 target |
|-----------|-------|------------|
| `monster_try_apply_chase_path` | `creature_start_chase_auto_walk` | `idle_enqueue_go_and_start(cid, first_step)` |
| `monster_start_follow_step` | `creature_start_chase_auto_walk` | same |
| `on_walk` stand-still / dance-alive | `request_idle_stimulus` | `schedule_walk_followup_deadline` |
| `finish_creature_todo_execute` queue drain | `idle_enqueue_go_and_start(..., true)` always | `first_step=false` when queue had steps |
| Monster walk in `on_walk` | shared | unchanged — execution is era-agnostic once scheduled |

After Layer 1, 772 monsters **never** hit `check_creature_walk_from_add_event_walk` from chase repath. They only step when todo wakeup fires and `todo_start_go_delay` allows it.

### Layer 2 — Decision boundaries (gate at source)

| Boundary | TFS (1098) | CipSoft (772) | Gate location |
|----------|------------|---------------|---------------|
| Chase one-step before A* | `get_distance_step` | skip → A* or cardinal fallback | `go_to_follow_creature` ✓ already |
| Greedy chase fallback | `monster_try_greedy_chase_step` | skip | `go_to_follow_creature` ✓ already |
| Brute one-step fallback | 8-dir scan | 4-dir cardinal | `monster_try_any_closer_step` ✓ already |
| Melee dance | `get_dance_step` + `staticAttackChance` | `rand() % 5` cardinal | `monster_next_walk_step` ✓ partial |
| Keep-distance dance | `get_dance_step` | cardinal band dance at `dist == effective targetDistance` | `monster_next_walk_step` ✗ **needs gate** |
| Flee step | `get_dance_step` / `get_distance_step` | `search_flight_field` → A* | `monster_next_walk_step` + `go_to_follow_creature` ✗ **needs gate** |
| Path search direction | `PathSearchModel::Forward` | `PathSearchModel::Reverse` | `MechanicsProfile` ✓ already |
| Forward A* fallback | always try forward | `NOWAY` (return none) or profile flag | `get_path_matching` ✗ **needs gate** |
| Relaxed FPP retry | TFS relaxed band | CipSoft: no relaxed pass, or same reverse | `monster_try_apply_chase_path` ✗ **needs gate** |
| Melee chase diagonals | `allow_diagonal: true` | optional `false` for feel | `monster_path_search_params` ✗ optional |

### Layer 2 implementation sketch (`monster_next_walk_step`)

Replace fall-through to TFS with explicit era dispatch — no shared fall-through:

```rust
if self.beat_driven_loop {
    if fleeing {
        return self.monster_cipsoft_flee_step(...);  // search_flight_field
    }
    if dist <= dance_range {
        return self.monster_cipsoft_dance_step(...);  // rand()%5 or band hold
    }
    return None;
}
// 1098: existing get_dance_step / staticAttackChance path
```

`monster_cipsoft_dance_step` handles both melee (`rand() % 5`) and keep-distance (cardinal lateral at band — not `get_dance_step`).

### Layer 2 implementation sketch (chase path)

```rust
fn monster_start_chase_walk(&mut self, cid: CreatureId, first_step: bool) {
    if self.beat_driven_loop {
        self.idle_enqueue_go_and_start(cid, first_step);
    } else {
        self.creature_start_chase_auto_walk(cid);
    }
}
```

Single choke point — replace all `creature_start_chase_auto_walk` call sites from monster AI with this.

### Layer 2 implementation sketch (pathfinding)

In `get_path_matching`, when `search == Reverse` and reverse returns none/empty:

```rust
if matches!(search, PathSearchModel::Reverse) {
    // 772: CipSoft NOWAY — do not fall through to forward TFS A*
    return None;
}
path_matching_forward(...);  // 1098 only
```

Or gate behind `MechanicsProfile` bool (e.g. `path_forward_fallback: bool`) default `false` for 772, `true` for 1098 — keeps core free of `if version == 772`.

### What stays shared (do not gate)

| Component | Why shared |
|-----------|------------|
| `on_walk` / tile move / push | Same observable step execution |
| `get_walk_delay` / CipSoft quantizer | Already profile-driven via `MechanicsProfile` |
| Reverse `TShortway` | Already selected by `path_search: Reverse` on 772 |
| `walk_queue` LIFO pop | Same as C++ `listWalkDir` |

### Phased rollout (movement — P1–P4)

| Phase | Work | Feel impact |
|-------|------|-------------|
| **P1** | Scheduler routing + dance retry delay | Fixes twitchy melee dance + instant repath snap |
| **P2** | `monster_next_walk_step` full CipSoft dispatch (flee + band dance) | Fixes diagonal dance on hunters / flee |
| **P3** | Path forward-fallback gate + optional melee `allow_diagonal: false` | Fixes diagonal chase on blocked geometry |
| **P4** | Relaxed FPP removal on 772; hysteresis fix | Edge cases / kiting |

Each phase should add/update `test_772_*` tests. 1098 tests must stay green (gate branches, don't change TFS path).

Full roadmap to observable **100%** (P1–P8): see [§Road to 100% Parity](#road-to-100-parity) below.

### Acceptance criteria (772 melee rat on player)

- `rand() % 5` attempt interval ≥ one full step duration (not ~1 ms).
- No `get_dance_step` calls on 772 (`#[cfg(test)]` mock or trace assert).
- Repath after player move: first step delayed by `todo_start_go_delay`.
- No forward A* fallback on open-field chase (reverse path only).

---

## Road to 100% Parity

**Definition:** Observable behavior matches CipSoft 7.72 (`tibia-game-master/src/`) — chase feel, target picks, flee, spacing, step timing, combat outcomes — in idiomatic Rust via `MechanicsProfile` + `beat_driven_loop`. Not line-for-line decompile transcription.

**Target:** ~98% with automated tests + soak; remaining ~2% is ambiguous decompile corners and content-specific edge cases.

### End-state architecture

```
MechanicsProfile (772)
  → IdleStimulus (only AI tick for monsters)
    → ToDo queue (Go / Attack / Wait)
      → ToDoStart delays (first vs continuing step)
        → CipSoft decision fns (TFS helpers unreachable on 772)
          → shared on_walk execution
```

**Invariant on 772:** zero reachable calls to `get_dance_step`, `get_distance_step`, `creature_start_chase_auto_walk` (monsters), forward A* fallback, relaxed FPP — enforced by tests/trace.

### Phase map

#### P1 — Scheduler parity (~72% → ~85% melee feel)

*See [§Proper 772 Gating Plan — Layer 1](#proper-772-gating-plan).*

1. `monster_start_chase_walk` choke point — todo-only on `beat_driven_loop`.
2. Stand-still dance → `schedule_walk_followup_deadline`, not `request_idle_stimulus`.
3. `first_step` wired: `true` new path, `false` queue continuation / dance retry.

| Deliverable | File(s) |
|-------------|---------|
| Chase walk choke point | `monster_ai.rs`, `walk/mod.rs` |
| Dance retry fix | `walk/mod.rs` |
| `first_step` on drain | `idle_stimulus.rs`, `creature_todo.rs` |

**Tests:** dance cadence (≥1 step duration between rolls); repath first-step delay.

---

#### P2 — Movement decision parity (~85% → ~92%)

Replace TFS fall-through in `monster_next_walk_step` with explicit CipSoft dispatch:

| Behavior | CipSoft ref | Rust deliverable |
|----------|-------------|------------------|
| Melee dance | `crnonpl.cc` ~2731 | `rand() % 5` cardinal (done) |
| Band dance (distance fighter) | ~2716 | Cardinal lateral at `dist == band`; band = per-type `targetDistance` |
| Flee step | `SearchFlightField` | Wire `search_flight_field` |
| Flee + blocked | A* away | Flee branch in `go_to_follow_creature`; no TFS distance step |

Remove `staticAttackChance` / `get_dance_step` from the 772 code path entirely.

**Tests:** flee steps away; hunter at band never diagonal-dances; no `get_dance_step` on 772.

---

#### P3 — Path parity (~92% → ~95% movement)

1. Profile gate: `path_forward_fallback: false` on 772 — CipSoft `NOWAY`, no forward TFS A*.
2. Remove relaxed FPP retry on 772 (`monster_try_apply_chase_path`).
3. Hysteresis: repath when queue end tile is wrong for target, not only `monster_at_follow_goal`.
4. Optional: `allow_diagonal: false` for melee chase FPP if soak still shows excess diagonals.

**Tests:** blocked corridor NOWAY; diagonal-kite repath; no forward fallback in traces.

---

#### P4 — IdleStimulus Phase B/C (~95% → ~97% action model)

Today todo has **`Go` only**. CipSoft idle enqueues **Go + Attack + Wait**.

| Action | CipSoft | Work |
|--------|---------|------|
| `Go` | `TDGo` | Phase A (needs P1 routing) |
| `Attack` | strike from idle | `CreatureAction::Attack` + B4 combat delays |
| `Wait` | beat hold at band | Wait actions for distance fighters |

Retire 772 monster attacks from 1 s think bucket; attacks from todo drain. Ref: [`IDLE_STIMULUS.md`](IDLE_STIMULUS.md).

**Tests:** adjacent rat attacks on todo schedule; distance fighter waits at band before ranged strike.

---

#### P5 — Targeting parity (~97% → ~98% whole stack)

From [`772_MONSTER_AI_TARGETING_PATHING_GAP_AUDIT.md`](772_MONSTER_AI_TARGETING_PATHING_GAP_AUDIT.md):

1. Weighted **`Strategy[]`** roulette — nearest / weakest (current HP) / most-damage / random.
2. **`LoseTarget`** per-idle random drop (not TFS cooldown alone).
3. **House zone** exclusion on `monster_is_target`.
4. Retarget on **idle drain**, not accumulated think ticks.

**Tests:** deterministic RNG for strategy weights, lose-target rate, multi-player switches.

---

#### P6 — Distance-fighter regime (keep-distance band)

**Design:** CipSoft **branch shape**, per-type **`targetDistance`** from monster data — **not** a hardcoded 4 in Rust. See [§Keep-distance band design](#keep-distance-band-design).

For each monster, band = `monster_effective_target_distance(m.target_distance)` (default [`DistanceKeep::PerType`](crates/tfs-rust-core/src/formulas.rs) on 772 — reads XML `targetDistance`; most ranged types are 4 in content anyway).

| Condition | CipSoft-style behavior (772) |
|-----------|------------------------------|
| `dist < band` | Step away (`SearchFlightField` / cardinal) |
| `dist > band` | A* / chase closer |
| `dist == band` | Cardinal dance / Wait / Attack (Attack needs combat) |

Optional shard override: `distanceKeep = N` in `data/formulas/772.lua` forces a fixed band for all types — use only when tuning, not the default.

**Tests:** type with `targetDistance=4` holds at 4 and steps back at 3; type with `targetDistance=7` holds at 7; neither melee-chases to 1.

---

#### P7 — Combat loop integration

B4 formulas exist (`combat/math.rs`); live monster hits must use them on 772:

- `attack_speed_ms` / 2000 ms defense gate (`defense_gate_ms`).
- `ProbeValue` damage RNG, randomized armor, fight modes.
- Distance hit `Probe(distance×15, …)` without defense subtract.

Without P7, monsters **move** like 7.72 but **fight** like stubs — caps whole-monster parity ~65% regardless of pathfinding.

**Tests:** golden damage ranges; defense gate timing; distance probe outcomes. Ref: `PROTOCOL_VERSIONING.md` §12.4 / B4.

---

#### P8 — Edge cases & soak (98% → ~100%)

| Edge case | CipSoft behavior |
|-----------|------------------|
| Target on unwalkable / furniture | Goal band / `MustReach` acceptance |
| Creature on chase tile | Occupancy path cost (partial) |
| Target floor change / teleport | Clear follow, lose target |
| Push creatures / items on step | Monster push parity |
| Spawn leash during chase | `MonsterhomeInRange` on path tiles |

**Prove it:** scenario test per row; 30+ min in-game soak (rat pack, hunter kite, flee corridor, multi-player agro).

---

### Scenario checklist (“feels like 7.72”)

| Scenario | Today | After P1–P3 | After P1–P8 |
|----------|-------|-------------|-------------|
| Rat boxing you (melee) | ~70% | ~92% | ~98% |
| Player kites diagonally | ~65% | ~88% | ~98% |
| Hunter at keep-distance band (`targetDistance`, usually 4) | ~50% | ~50% | ~95% |
| Multi-player target switch | ~60% | ~60% | ~95% |
| Fleeing monster | ~55% | ~70% | ~95% |

---

### Verification harness (know when you’re done)

1. **Unit tests** — deterministic RNG; `test_772_*` per decision boundary.
2. **No-TFS invariant** — trace/assert: no `get_dance_step`, `get_distance_step`, monster `add_event_walk` on 772.
3. **Timing tests** — step intervals match CipSoft quantizer; diagonal ≥ ~3× cardinal.
4. **Golden traces** — position + direction every 200 ms vs reference captures on fixed maps.
5. **1098 regression** — TFS branches unchanged; existing tests green.

---

### What not to do

- More scattered `if beat_driven_loop` without choke points (creates hybrid hell).
- Separate `monster_ai_772.rs` fork — use profile + boundary dispatch (project rules).
- Chasing 100% on pathfinding alone — algorithm already ~95%; diminishing returns.
- Claiming 100% without P4 (Attack todo) + P5 (targeting) + P7 (live combat).

---

### Execution order (summary)

```
Now ── P1 ──► ~85% melee feel
      P2 ──► ~92% movement
      P3 ──► ~95% movement
      P4 ──► todo Attack/Wait
      P5 ──► targeting
      P6 ──► distance fighters
      P7 ──► combat loop
      P8 ──► edge cases + soak
           ► ~98% observable monster parity
```

**Highest ROI:** P1 (1–2 PRs). **Required for true 100%:** P4 + P5 + P7.

---

## Keep-distance band design

**Decision (locked):** 772 uses **CipSoft decision shape** with **per-type `targetDistance`** from monster files — not a hardcoded 4-tile constant and not TFS `get_dance_step` lateral/diagonal dance.

### What we match from CipSoft

CipSoft decompile (`crnonpl.cc` ~2716) hardcodes range 4 because most distance-fighting types in original data used that value. We replicate the **three-way branch**, parameterized by band:

```
band = monster_effective_target_distance(m.target_distance)

if dist < band  → step away (SearchFlightField / cardinal)
if dist > band  → chase closer (A* / todo Go)
if dist == band → cardinal dance or Wait (Attack when combat lands)
```

### What we do not do

| Avoid | Why |
|-------|-----|
| `const BAND: i32 = 4` in Rust | Content varies; hunters/dragons may differ in XML |
| TFS `get_dance_step` on 772 | Diagonal/lateral fallback — wrong feel |
| Ignoring `targetDistance` | Loses per-type tuning from `monsters.xml` |

### What already exists in Rust

```rust
// monster_ai.rs — PerType is default for 772 and 1098
pub(crate) fn monster_effective_target_distance(&self, per_type: i32) -> i32 {
    match self.mechanics.profile.distance_keep {
        DistanceKeep::PerType => per_type,   // XML targetDistance
        DistanceKeep::Fixed(n) => n,         // optional shard override in 772.lua
    }
}
```

`monster_at_follow_goal`, `monster_path_search_params`, and `dance_range` already use this effective distance — P2/P6 work is to replace TFS **movement/dance arms** at that band with CipSoft cardinal logic, not to change where band comes from.

### P6 deliverable (movement, pre-combat)

- Flee / too-close: `search_flight_field` (already implemented, needs wiring).
- At-band dance: cardinal lateral via `monster_cipsoft_dance_step` — same family as melee `rand() % 5`, gated on `dist == band` not `dist == 1`.
- Still reads `targetDistance` from type; tests use types with 4 and 7 (or similar) to prove it is not hardcoded.

---

## Activating CipSoft monster logic via formulas (1098 shard)

**Question:** Can a 1098 server use CipSoft monster chase/dance/idle by swapping formula settings, while keeping 1098 wire/protocol?

**Answer:** **Yes, mostly** — by overlaying CipSoft keys into `data/formulas/1098.lua`. There is **no** separate monster-only toggle today; the master switch also enables the beat-driven **simulation loop**.

### How era selection works

| Axis | Selector | What it controls |
|------|----------|----------------|
| **Wire** | `clientVersion` → `Codec1098` / `Codec772` | Packets, opcodes, login — unchanged by formulas |
| **Mechanics** | `data/formulas/{clientVersion}.lua` → `MechanicsProfile` | Pathfinding, walk timing, monster AI gates |
| **Monster AI mode** | `beat_driven_loop` on `GameWorld` | IdleStimulus, todo `Go`, CipSoft dance/chase gates |

Monster CipSoft vs TFS is **not** hard-tied to `clientVersion`. It is gated by:

```rust
// game_world.rs — derived once at startup from MechanicsProfile
beat_driven_loop = (mechanics.profile.step_speed == StepSpeedModel::CipSoft);
```

Formulas load from `data/formulas/1098.lua` when `clientVersion = 1098` — **not** from `772.lua`. Copy relevant keys into `1098.lua`; do not point 1098 at the 772 file.

### Master switch: `stepSpeedModel = "cipsoft"`

Add to `1098.lua` (parser: `formulas.rs` `stepSpeedModel`):

```lua
formulas = {
  stepSpeedModel = "cipsoft",   -- → beat_driven_loop = true
  beatMs = 200,                 -- 772 main loop (1098 default is 50)
  stepBeatMs = 50,              -- walk quantizer (TVP reference)
  pathSearch = "reverse",       -- TShortway
  pathCost = "terrain",         -- terrain waypoints, diagonal ×3
  followRepathWithoutPath = true,
  weakestTargetMetric = "currentHp",
  distanceKeep = "perType",     -- band from each monster's XML targetDistance
  -- optional: damageFormula, armor, fightModes for full 772 combat feel
}
```

When `stepSpeedModel = "cipsoft"`, `run_server.rs` also:

- Runs **`run_game_loop_772`** instead of `run_game_loop_1098`
- Sets **`walk_wake_tx = None`** (no Tokio monster walk timers)
- Enables **IdleStimulus** + todo **`Go`** for monsters (`creature_uses_todo_execute`)

All `if self.beat_driven_loop` monster AI gates activate: `rand() % 5` melee dance, no `get_distance_step`, cardinal fallback, etc.

### What this does **not** do

| Expectation | Reality |
|-------------|---------|
| 1098 wire + CipSoft monsters **only** | **No** — `stepSpeedModel = "cipsoft"` flips the **whole** beat-driven mode (loop, walk timing, scheduling). Players use the 772 loop path too; only monsters use todo execute. |
| Drop in whole `772.lua` on a 1098 server | **No** — loader uses `formulas/1098.lua` for `clientVersion = 1098`. Overlay keys, don't swap files. |
| Instant perfect CipSoft feel | **No** — P1–P6 implementation must land first; config on today's hybrid still inherits hybrid bugs. |
| Hardcoded 4-tile band | **No** — `distanceKeep = "perType"` keeps band from XML `targetDistance` on any era. |

### Partial activation (without `stepSpeedModel`)

These `MechanicsProfile` keys affect monsters on a **1098 TFS-AI** shard (`beat_driven_loop = false`) without IdleStimulus or dance gates:

| Key | Effect without beat-driven |
|-----|----------------------------|
| `pathSearch = "reverse"` | CipSoft reverse A* |
| `pathCost = "terrain"` | Terrain-weighted path costs |
| `followRepathWithoutPath = true` | Repath on target move without `hasFollowPath` |
| `weakestTargetMetric = "currentHp"` | Weakest target by current HP |

These do **not** enable: IdleStimulus, `rand() % 5` dance, TFS dance gating off, todo chase routing.

### Future decoupling (not built)

A dedicated profile axis (e.g. `idleStimulus = true` or `monsterAiModel = "cipsoft" | "tfs"`) decoupled from `step_speed` would allow **1098 players + CipSoft monsters**. Today they share one switch. Worth adding if dual-mode shards become a requirement.

### Recommended 1098 “CipSoft monsters” rollout

1. Implement **P1–P3** on `beat_driven_loop` path first.
2. Test on `clientVersion = 772` until movement feel passes acceptance criteria.
3. Overlay CipSoft keys into **`1098.lua`** for a 1098-protocol test shard.
4. Verify 1098 regression tests still pass with default `1098.lua` (no overlay).

---

## Verification

### Unit tests (existing)

```bash
cargo test -p tfs-rust-core test_772_ -- --nocapture
cargo test -p tfs-rust-core cipsoft_diagonal_step_duration -- --nocapture
```

| Test | Asserts |
|------|---------|
| `test_772_melee_dance_only_cardinal` | Dance steps are N/E/S/W only |
| `test_772_path_to_diagonal_target` | A* paths cardinal on uniform terrain |
| `test_772_walk_queue_hysteresis` | Queue retained when target moves within band |

### Tests to add after fixes

**P1:**
- **Dance cadence**: adjacent monster, stand-still rolls — assert ≥ `step_duration_ms` between `rand() % 5` attempts (deterministic RNG seed).
- **Repath first-step delay**: player moves off-band — assert first chase step not before `todo_start_go_delay` fires.
- **No-TFS invariant**: 772 path never calls `get_dance_step` / monster `creature_start_chase_auto_walk`.

**P2–P3:**
- **Diagonal timing**: diagonal step interval ≥ 2.5× cardinal interval on same tile.
- **Flee / hunter band**: `search_flight_field` wired; no diagonal band dance.
- **NOWAY**: reverse fail does not invoke forward A* on 772.

**P5–P8:**
- Strategy roulette, LoseTarget, damage golden ranges, edge-case scenarios (see [§Road to 100% Parity](#road-to-100-parity)).

### Runtime observation

```bash
RUST_LOG=tfs_rust_core::creature_todo=debug,tfs_rust_core::idle_stimulus=debug cargo run
```

Stand rat on player; count `idle_stimulus_enter` / `enqueue_go` events per second. Before fix: hundreds/sec. After fix: ~1–2/sec (depends on monster speed and ground).

---

## Files Reviewed

| Rust | Role |
|------|------|
| `crates/tfs-rust-core/src/monster_ai.rs` | `monster_next_walk_step`, `go_to_follow_creature`, dance/chase |
| `crates/tfs-rust-core/src/walk/mod.rs` | `on_walk`, `add_event_walk`, `schedule_walk_followup_deadline` |
| `crates/tfs-rust-core/src/idle_stimulus.rs` | Drain-triggered idle, Go enqueue |
| `crates/tfs-rust-core/src/creature_todo.rs` | `Go` action, wakeup scheduling |
| `crates/tfs-rust-core/src/monster_events.rs` | Follow repath hysteresis |
| `crates/tfs-rust-core/src/pathfinding.rs` | `TShortway`, diagonal cost |
| `crates/tfs-rust-core/src/walk/walk_timing.rs` | CipSoft step duration |

| CipSoft (reference) | Role |
|---------------------|------|
| `crnonpl.cc` ~2731–2753 | Melee dance `rand() % 5` |
| `crnonpl.cc` ~2386 | `TMonster::IdleStimulus` |
| `cract.cc` ~846, ~955 | `CalculateDelay`, `ToDoStart` |
| `cract.cc` ~1454–1462 | `NotifyGo` diagonal ×3 waypoints |
| `cract.cc` `TShortway` | Reverse A* |

---

## Conclusion

772 monster **pathfinding** (~95%) and **melee dance selection** (~95%) are strong in isolation. **Movement feel** (~72%) and **whole-monster parity** (~60%) lag because of hybrid scheduling and missing targeting/combat/todo-Attack layers.

1. **Hybrid architecture** — 772 runs on a shared TFS stack with narrow spot gates. Chase scheduling, keep-distance/flee dance, forward A* fallback, and relaxed path retry remain reachable on 772.

2. **Scheduling bugs** — dance stand-still retries fire ~1000× faster than CipSoft; chase repaths skip todo/`ToDoStart` delays.

**Path to ~100%:** P1–P3 closes movement (~95%); P4–P7 closes action model, targeting, and combat (~98% whole stack); P8 + soak closes edge cases. See [§Road to 100% Parity](#road-to-100-parity).

---

*Audit date: 2026-06-06 (updated: hybrid architecture, road-to-100%, keep-distance band design, formulas cross-era activation). Based on code review and unit test verification; CipSoft source cited from project reference paths (`tibia-game-master/src/`).*
