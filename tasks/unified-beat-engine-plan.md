# Unified Beat Engine — Make the CipSoft ToDo Simulation the Sole Engine

**Date:** 2026-07-02
**Author:** planning pass (pre-refactor)
**Status:** ⬜ PLAN ONLY — not started. Do not begin implementation until the open questions
(§10) are answered and the parity gate (§8) is agreed.

## 0. Goal (in the user's words)

Make the **beat-driven ToDo engine the single source of truth** for server mechanics — game
loop, scheduling, creature `Execute`/`IdleStimulus`, monster AI, chase/follow. Every era (772
today, 1098, and any future codec) runs on **this one engine**. Version differences branch off
**only** in two sanctioned places:

- **`MechanicsProfile`** (+ `data/formulas/<v>.lua`) — era-tuned numbers/curves/cadences.
- **`ProtocolCodec`** (`tfs-rust-net`) — wire bytes / opcodes / transport caps.

Nothing else. No parallel loop, no `beat_driven_loop` boolean, no reactive 1098 walk/follow
machinery.

## 1. Relationship to existing plans (read these first)

| Doc | What it covers | This plan's relationship |
|-----|----------------|--------------------------|
| `docs/GAME_LOOP_ARCHITECTURE.md` | Both loops as they exist today (dual) | This plan **retires §2 (1098 reactive loop)** and generalizes §3 (772 beat loop) to all eras |
| `tasks/walk-engine-unification.md` | Unifies the **walk/chase** engine; Phase 1 (772 players) DONE, **Phase 2 (1098) DEFERRED** | This plan is the **superset**: it executes that Phase 2 and extends it to think/AI/conditions/skills/flush/loop |
| `docs/REFACTOR_AUDIT.md` Phase 3 | Rename `_772` core fns by *behavior* (`advance_beat_772` → `advance_beat`) | **Coordinate** — see §7. The renames should assume the beat path is *canonical*, not one era's variant |
| `tasks/player-walk-audit.md` | Per-bug walk findings; **P8/P9 are 1098-only** and "dissolve" under unification | This plan is the "ARCHITECTURE DECISION" it references — P8/P9 become `CanToDoAttack`/`EarliestWalkTime`, not 1098 special cases |

**This plan does NOT replace those docs.** It sequences and extends them. Walk-engine Phase 2 is
subsumed into Phase B/C below.

## 2. The hard truth: this is NOT behavior-preserving for 1098

The `REFACTOR_AUDIT.md` phases are all *structural* (behavior-frozen). **This effort is
different.** It changes 1098's *timing model*:

- Today 1098 runs on a **wall-clock reactive** loop: `tokio::select!`, per-creature
  `sleep_until` walk timers (sub-ms precision), `Instant::now()` clock, immediate movement
  flushes, 50 ms bucketed `checkCreatures`.
- After: 1098 runs on the **logical beat clock** (`server_ms` advanced in `beat_ms` steps),
  the global `ToDoQueue` min-heap, `IdleStimulus`-driven AI, and consolidated per-beat flush.

This is an **observable change** for 1098 (step cadence, follow re-path latency, flush timing,
client-side prediction feel). Per `tfs-core.md`, observable changes require explicit approval
and a parity gate. **The whole effort is gated by §8 (live 10.98 client QA).** Treat "no test
delta" as necessary-but-not-sufficient; the real bar is client feel.

> **Mitigation lever:** `beat_ms` is a profile knob. 1098 can run the shared engine at a
> **smaller beat** (e.g. 50 ms) to preserve retail feel while sharing all logic. The engine is
> beat-driven; the beat *size* is per-era. See Phase A.

## 3. Current state (measured 2026-07-02)

Dual-loop split is gated by one boolean, `GameWorld::beat_driven_loop`
(`= profile.step_speed == StepSpeedModel::LinearGo`):

- **186** non-comment `beat_driven_loop` branches across **25** production files.
- Two loop entry points: `run_game_loop_1098` (reactive) and `run_game_loop_772` (beat) in
  `game_loop.rs`; selected in `run_server.rs`.
- 1098-only machinery still live: `walk_wake_tx`/`walk_wake_rx` + `sleep_until`,
  `process_walk_deadlines`, `on_tick`/`check_creatures` (50 ms bucketed), `go_to_follow_creature`
  + `onThink` follow poll + `onCreatureMove` re-path, `walk_action_due` dual path,
  `FlushPolicy::ImmediateOnMovement`, `Instant`-based `walk_timer`/`next_walk_check`.
- 772/beat machinery to become canonical: `advance_beat_772`, `ToDoQueue`, `drain_todo_queue`,
  `subsystem_counters_772`, `IdleStimulus`/`Execute`, `server_ms`, `next_wakeup`,
  `FlushPolicy::BeatEndOnly`, `process_connections_772`, `tick_ambiente_light_772`.

## 4. Target invariants (definition of done)

1. **One loop.** `run_game_loop` only. Beat size = `profile.beat_ms`. No `run_game_loop_1098`.
2. **One clock.** `now_ms()` == `server_ms` for all eras. No `Instant`-based scheduling in core
   game logic (Tokio timers stay in I/O/loop only).
3. **One scheduler.** `ToDoQueue` + `IdleStimulus`/`Execute` for every creature, every era. No
   `walk_wake_tx`, no `process_walk_deadlines`, no per-creature `sleep_until`.
4. **`beat_driven_loop` is gone.** Every current branch is resolved to: (a) unified code, or
   (b) a `MechanicsProfile` knob when the *outcome* genuinely differs by era.
5. **Era = profile + codec, nothing else.** Adding a new era = add a `MechanicsProfile` +
   `data/formulas/<v>.lua` + a `Codec`. Zero new core branches.
6. **Green + parity.** `cargo test` green (counts change deliberately here — this is not
   behavior-frozen); §8 parity QA signed off for both 772 and 1098.

## 5. Taxonomy of the 186 branches (do this classification FIRST)

Before touching code, produce a spreadsheet/table classifying every `beat_driven_loop` site
into one of four fates. This is the single most important de-risking step — it converts a scary
186-site refactor into a mechanical checklist.

| Fate | Meaning | Example sites |
|------|---------|---------------|
| **U — Unify** | The `else`/1098 arm is dead once 1098 is on the beat engine; delete it, keep the beat arm unconditionally | `creature_uses_todo_execute`, `creature_todo_yield`, `now_ms`, walk-action deferral, `monster_targets` idle-stimulus arms |
| **K — Profile Knob** | Outcome genuinely differs by era → move the difference into `MechanicsProfile` (or an existing knob), delete the boolean check | step-duration (`step_speed` already), think cadence, condition tick interval, `parity_random` stream (RNG source) |
| **C — Clock adapter** | Difference is *only* wall-clock vs logical time | `walk_timer_idle`, `next_walk_check` vs `next_wakeup`, `player_ping` timing |
| **X — Codec/transport** | Belongs in `tfs-rust-net`, not core | `player_ping` `0x1E` vs `0x1D`, any flush-shape difference tied to wire |

Deliverable: `tasks/beat-unify-branch-inventory.md` with all 186 rows tagged U/K/C/X + target.
**No code changes in this step.** Review the K rows with the user — those are the only places a
real era difference survives, and each one is a decision.

## 6. Phased execution

Ordered so risk-per-step descends and each phase leaves the tree green. Gate every step with
`rtk cargo check && rtk cargo clippy --all-targets && rtk cargo test`.

### Phase A — Decouple "beat size" from "which era" (low risk, enabling)
**Goal:** make the beat engine runnable at any beat, so 1098 can adopt it without inheriting
200 ms.
- [ ] Confirm `beat_ms` / `step_beat_ms` / `step_speed` are independent knobs (they already are
      in `MechanicsProfile`). Add explicit profile fields for the currently-hardcoded era splits
      the taxonomy (§5, K rows) surfaces: **think cadence** (772 ~1000 ms staggered vs 1098
      50 ms bucketed), **flush policy** (beat-end vs immediate-on-movement), **condition/skill
      tick interval**.
- [ ] Introduce a `LoopProfile`/`SchedulingProfile` sub-struct (or fields on `MechanicsProfile`)
      so the loop reads cadence/flush/beat from the profile, not from `beat_driven_loop`.
- [ ] Keep both loops running for now; 772 unchanged. 1098 unchanged. Pure plumbing.

**Exit:** the beat loop can be instantiated with `beat_ms = 50`, immediate-ish flush, and 50 ms
think cadence and produce a *1098-flavored* run (even if not yet wired as the default).

### Phase B — Route 1098 creatures onto ToDo/`Execute`/`IdleStimulus` (medium/high)
This is `walk-engine-unification.md` Phase 2, executed. Do **monsters first, players second**
(players are the higher-risk, more-visible path).
- [ ] Implement the continuous/small-beat side of the Phase 0 clock seam so `server_ms` drives
      1098 scheduling.
- [ ] Widen every `creature_uses_todo_execute` / `request_idle_stimulus` / idle-stimulus arm
      that is currently `beat_driven_loop`-gated to run for 1098 too (many are already
      `creature_uses_todo_execute`-based — those just need the flag to flip true).
- [ ] Route 1098 monster AI (`monster_ai.rs` `on_think`/`do_attacking`/follow) through the same
      `IdleStimulus`/`CanToDoAttack` path monsters use on 772. Replace `go_to_follow_creature`
      + `onThink` follow poll + `onCreatureMove` re-path with `CanToDoAttack` on the attack beat
      (P8 dissolves).
- [ ] Route 1098 players through ToDo (mirror the completed 772 player work in
      `player_combat.rs` / `walk/mod.rs`). The `nextAction` lockout becomes `EarliestWalkTime`
      in the ToDo delay (P9 dissolves).

**Exit:** with 1098 config + `beat_driven_loop` *temporarily forced true*, monsters and players
walk/chase/follow via ToDo; behavioral spot-checks (§8) pass in a dev harness.

### Phase C — Retire the 1098 reactive machinery (medium)
Delete, don't gate. After Phase B nothing should call these on 1098:
- [ ] `run_game_loop_1098` walk-wake branch + `walk_wake_tx`/`walk_wake_rx` + `sleep_until`
      scheduling; `GameWorld::walk_wake_tx` field.
- [ ] `process_walk_deadlines`, `schedule_walk_followup_deadline`, `commit_next_walk_deadline`.
- [ ] `go_to_follow_creature` and the `onThink` follow-poll / `onCreatureMove` re-path.
- [ ] `walk_action_due` dual path (`walk_action.rs`) → single ToDo-based path.
- [ ] `Instant`-based `walk_timer` / `next_walk_check` on `CreatureBase` → `next_wakeup` only.
- [ ] `FlushPolicy::ImmediateOnMovement` → profile-driven flush (Phase A knob).

**Exit:** `grep` for the above symbols returns only removed/renamed results; both eras still run
(772 via beat, 1098 via the now-shared beat engine with 1098 profile).

### Phase D — Collapse the `beat_driven_loop` flag (mechanical, guided by §5)
- [ ] Walk the §5 inventory. For every **U** row: delete the `else`, keep the beat arm
      unconditionally. For every **K** row: replace the boolean with the profile read. For **C**:
      route through the clock seam. For **X**: push to codec.
- [ ] Remove the `beat_driven_loop` field from `GameWorld` and its constructor wiring.
- [ ] `now_ms()` unconditionally returns `server_ms`.

**Exit:** `grep -rn "beat_driven_loop" crates/tfs-rust-core/src` → **zero** production hits
(tests may reference the removed field until updated; update them in this phase).

### Phase E — Single loop entry point (low, once B–D land)
- [ ] Merge `run_game_loop_772` into `run_game_loop`; delete `run_game_loop_1098` and the
      back-compat alias. `run_server.rs` picks beat size + flush/cadence from the profile — no
      `if beat_driven` fork.
- [ ] Update `docs/GAME_LOOP_ARCHITECTURE.md`: collapse §2/§3 into "one beat engine, per-era
      beat size + cadence via profile"; keep the C++ reference index (both eras still cite their
      sources).

**Exit:** one loop function; `run_server.rs` has no loop-selection branch.

### Phase F — Naming reconciliation with REFACTOR_AUDIT Phase 3 (mechanical)
The audit's Phase 3 renames `advance_beat_772` → `advance_beat`, `process_creatures_772` →
`process_creatures_beat`, etc. Under unification these are **the** canonical functions, not a
"beat variant."
- [ ] Rename by canonical role, dropping era/variant qualifiers where the beat path is now the
      only path: `advance_beat_772` → `advance`, `process_creatures_772` → `process_creatures`,
      `process_connections_772` → `process_connections`, `tick_ambiente_light_772` →
      `tick_ambient_light`, `subsystem_counters_772` → `subsystem_counters`, etc.
- [ ] Coordinate ordering: if REFACTOR_AUDIT Phase 3 runs **before** this effort, it will pick
      `*_beat` behavior-suffixes; re-simplify them here. If this effort runs first, do the
      canonical rename directly and mark audit Phase 3 satisfied for these symbols. **Recommend:
      do the unification first, then the rename**, so names reflect the final single-engine
      reality.

**Exit:** `grep -rn "fn .*_772" crates/tfs-rust-core/src` returns only config/test items
(per audit Phase 3 exit criteria).

## 7. How a new era/codec plugs in afterward (the payoff)

Adding e.g. 860 or 1200:
1. Add a `Codec` impl in `tfs-rust-net/src/codec/` (wire bytes, opcodes, caps).
2. Add `MechanicsProfile::for_version(860)` + `data/formulas/860.lua` (beat size, step curve,
   condition ticks, cadences, AI knobs).
3. **Write zero core branches.** The engine, AI, scheduler, and loop are already shared.

If step 3 ever requires a core `if version == …`, that is a signal the difference belongs in the
profile or the codec — stop and re-classify per §5.

## 8. Parity gate (mandatory — this is the real acceptance test)

Because 1098 behavior changes, "tests green" is not enough. Before removing the old 1098 loop
(end of Phase C) **and** before shipping:

- [ ] **772:** re-run the behavioral checks in `tasks/player-walk-audit.md` §Verification and the
      `chase_kite_sim` harness — outcomes unchanged from current 772 (this era already uses the
      engine, so it must be byte-for-byte stable).
- [ ] **1098:** validate against a **live 10.98 client**: single-step + diagonal walk cadence,
      autowalk, follow re-path latency, monster chase/kite feel, attack beat, client-side
      prediction (no rubber-banding), light/ambient, ping. Compare side-by-side with the current
      `run_game_loop_1098` build.
- [ ] Decide the 1098 `beat_ms` empirically here (50 ms vs larger) — this is the primary feel
      knob. Record the choice + rationale in `docs/GAME_LOOP_ARCHITECTURE.md`.

If 1098 feel regresses and can't be recovered via `beat_ms`/cadence knobs, **stop** — the shared
engine may need a finer sub-beat scheduling mode (a profile-selected "continuous" drain) rather
than forcing beat quantization on 1098. Flag that as a design escalation, not a silent
workaround.

## 9. Risks

- **1098 client prediction / rubber-banding** — biggest risk. Beat-quantized walk timing may
  desync from the 10.98 client's local movement prediction. Mitigated by small `beat_ms` and the
  §8 gate; escalate to a continuous-drain profile mode if unrecoverable.
- **186-site collapse churn** — mitigated by the §5 inventory-first approach (classify before
  editing) and by flipping the flag `true` (Phase B) before deleting the `else` arms (Phase D),
  so behavior and deletion are separate, individually-revertable steps.
- **Test suite assumes 1098 reactive paths** — many tests set `walk_wake_tx = None` /
  `process_walk_deadlines()` / `go_to_follow_creature()` directly. These must migrate to the ToDo
  path; expect deliberate test rewrites (not "adjust to pass" — rewrite to assert the new model).
- **Lua timing visibility** — the immediate-mutation contract (`tfs-lua-boundaries.md`) is clock
  independent, but scripts using `addEvent` and think cadence may observe different tick spacing
  on 1098. Verify Tier-2 hook timing under the new cadence.
- **Not behavior-preserving** — unlike REFACTOR_AUDIT, so it cannot ride the "no test delta"
  safety net. §8 is the net.

## 10. Open questions for the user (answer before Phase A)

1. **1098 target beat:** OK to run 1098 on the shared beat engine at a **small `beat_ms`
   (~50 ms)** to preserve feel? Or must 1098 keep true sub-ms/continuous scheduling (which would
   mean a profile-selected "continuous drain" mode on the *same* engine rather than a fixed
   beat)?
2. **Sequencing vs REFACTOR_AUDIT Phase 3:** do the unification first then the `_772` rename
   (recommended, §6 Phase F), or are you mid-rename and want the plan to assume the renames land
   first?
3. **Scope of "monster AI etc.":** confirm the intended surface — walk/chase/follow + think +
   conditions + skills + spawn/respawn cadence + flush. Anything explicitly *out* of scope for
   this pass (e.g. leave 1098 combat timing alone for now)?
4. **Parity bar owner:** who runs the live 10.98 client QA (§8), and is a temporary
   feature-flagged "1098-on-beat" build (both loops compiled, chosen by config) acceptable during
   Phase B/C so you can A/B against the current loop?

## 11. Verification commands (every step)

```bash
rtk cargo check
rtk cargo clippy --all-targets
rtk cargo test -p tfs-rust-core
rtk cargo test -p tfs-rust-net
# 772 parity harness (must stay stable):
rtk cargo run --bin chase_kite_sim --features sim   # if used for the parity check
```
Watch suites: `idle_stimulus` (`test_phase1_*`), `creature_todo`, `walk/mod` step-speed,
`monster_ai_world_tests`, `subsystem_counters_772`, `game_world_tick`.
