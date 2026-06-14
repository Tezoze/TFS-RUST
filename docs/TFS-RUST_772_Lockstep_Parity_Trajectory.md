# TFS-RUST 772 — Lockstep Parity Trajectory

**Date:** 2026-06-14  
**Status:** active plan — post §21 closeout (`4874932`)  
**Audience:** implementing engineer  
**Goal:** reach **6/6 lockstep PASS** on the synthetic sim battery, then hold the gate as scenarios expand.

**Related docs**

| Doc | Role |
|-----|------|
| [TFS-RUST_772_Sim_Divergence_Report.md](TFS-RUST_772_Sim_Divergence_Report.md) | Historical A/B metrics, §21 baseline |
| [TFS-RUST_772_Sim_Coverage_Matrix.md](TFS-RUST_772_Sim_Coverage_Matrix.md) | Event-type coverage map |
| [TFS-RUST_772_Monster_Combat_Integration_Plan.md](TFS-RUST_772_Monster_Combat_Integration_Plan.md) | E0–E6 combat integration (largely done) |

---

## 0. Executive summary

| Metric | Today (§21) | Target |
|--------|-------------|--------|
| Lockstep gate | **1 / 6 PASS** (17%) | **6 / 6 PASS** (100%) |
| Stand dance content | 100% pairwise on chase arms | + tick bucket alignment |
| Panic E5 trace | 100% on dance + `damage_stimulus` | + tick bucket alignment |
| Cyclops quad | `combat_state` 4/4; `shortway` 0/4 | chase @ ref tick + path shape |
| Kill E6 | **PASS** | maintain |

**Bottom line:** observable *semantics* are largely correct on stand/panic/kill. The remaining work is **when** (tick cadence / appear drain), **where** (path walk-back on cyclops), and **harness gaps** (kite time advancement, cobra E4 tuning). This is engineering backlog, not a fundamental parity ceiling.

---

## 1. What “100% parity” means

### 1.1 Lockstep gate (primary metric)

The gate is enforced by `scripts/summarize_chase_gaps.py --lockstep`:

1. Run the same scenario on C++ harness and Rust `chase_kite_sim` with `TFS_SIM_SEED=772`.
2. Compare JSONL traces within `--max-tick` for the scenario.
3. **PASS** iff every `mismatch_counts` key is **0** across all event types:

   `branch`, `todo_go`, `shortway`, `go_exec`, `combat_state`, `attack_enqueue`, `melee_hit`, `ranged_hit`, `spell_cast`, `damage_stimulus`, `creature_death`.

Comparison is **tick-bucketed** (2000 ms windows): events must match in **count per tick** and **semantic payload** (arm, dest tile, via, step list, damage, etc.). Same content at the wrong tick is **FAIL**.

Battery orchestrator: `scripts/run_sim_battery.py --synthetic`.

### 1.2 What 100% does *not* require

- Line-for-line Rust ↔ C++ structure (idiomatic Rust is mandatory).
- Byte-identical creature IDs in JSONL (compare normalizes semantic roles).
- Parity on scenarios not in the battery (expanded coverage is a follow-on).

### 1.3 Soft metrics (progress signals, not the gate)

| Metric | Use |
|--------|-----|
| Pairwise match % | Content alignment when tick buckets differ |
| Event count delta | Smoke for missing/extra hooks |
| Diagonal `go_exec` ratio | Path shape health |
| Parity scorecard (divergence report) | Layer-level estimates |

Do **not** relax the gate (e.g. ignore tick buckets) to inflate pass rate. Fix cadence instead.

---

## 2. Current baseline (§21)

Battery: `TFS_SIM_SEED=772`, `--synthetic`, QM on `127.0.0.1:7173`.

| Scenario | Lockstep | ref / rust events | Primary blocker |
|----------|----------|-------------------|-----------------|
| **kill** | **PASS** | 2 / 2 | — |
| stand | FAIL | 11 / 12 | tick bucket: rust@2000 vs ref@4000 on chase arms |
| panic | FAIL | 12 / 14 | same tick shift; combat counts (`melee_hit` ref=2 rust=4) |
| cyclops | FAIL | 20 / 18 | tick=0 `melee_dance`×2; `shortway` 0/4; diagonal first step |
| kite | FAIL | 8 / 17 | C++ **0 events** — `advance_ms 0` never advances time |
| cobra | FAIL | 15 / 20 | E4 spell-cast timing / scenario tuning |

**Recently closed (§21):** `DANCE_DIR_ORDER` N/S fix, `ToDoWait(0)`, chase step reverse removal, kill armor + stimulus order, `harness_preserve_sleep`.

**Known regression to avoid:** batched appear `ToDoYield` without per-monster idle defer → cyclops tick=0 dance or silenced JSONL.

---

## 3. Trajectory overview

Phases are ordered by **lockstep ROI** (scenarios flipped per unit effort). Each phase ends with a battery rerun and a new divergence-report section.

```
Phase 0 ──► 1/6   kill PASS                    [DONE §21]
Phase 1 ──► 3/6   + stand, panic               [DONE §22]
Phase 2 ──► 4/6   + cyclops                    [appear yield + path shape]
Phase 3 ──► 5/6   + kite                       [scenario time model]
Phase 4 ──► 6/6   + cobra                      [E4 spell delay]
Phase 5 ──► hold  expand battery + live replay [ongoing]
```

| Phase | Gate | Est. effort | Depends on |
|-------|------|-------------|------------|
| 0 | 1/6 | — | §21 shipped |
| 1 | 3/6 | Medium | Phase 0 |
| 2 | 4/6 | Medium–high | Phase 1 cadence patterns |
| 3 | 5/6 | Low | Harness only |
| 4 | 6/6 | Medium | E4 combat path stable |
| 5 | 6/6 + | Ongoing | All phases |

---

## 4. Phase 1 — Stand + panic (3/6)

**Target:** `stand` and `panic` lockstep PASS.

### 4.1 Problem statement

Chase arms already **100% content match** on `branch` / `todo_go` / `go_exec` / `damage_stimulus`. Failures are **tick-bucket only**:

```
branch  tick=2000  ref=0 rust=1
branch  tick=4000  ref=1 rust=0
```

Rust first idle fires **one 2000 ms bucket too early** vs C++.

### 4.2 Hypothesis chain

1. C++ `SpawnMonsterAppear` sets target then `ToDoYield()` on every monster before a single `DrainTodoQueue` (`chase_kite_scenario.cc`).
2. Rust per-monster `kite_monster_appear` indirectly schedules idle via `request_idle_stimulus` on `monster_set_follow_creature` — wakeup lands before the first `advance_ms 2000` drain window C++ uses.
3. `creature_todo_yield` now uses `ToDoWait(0)` at `server_ms` (§21) — partial fix, not sufficient alone.

### 4.3 Work items

| ID | Task | Files / refs | Done when |
|----|------|--------------|-----------|
| P1.1 | Trace glibc + todo wakeup from appear → first `branch` on both sides (`TFS_SIM_RNG_TRACE=1`) | harness, C++ scenario | Draw order documented; delta identified |
| P1.2 | Align first idle wakeup to post-appear drain tick (single-monster path first) | `sim_harness.rs`, `chase_kite_sim.rs`, `creature_todo.rs`, `idle_stimulus.rs` | stand `branch` @4000 ref = rust @4000 |
| P1.3 | Re-check combat trace counts after cadence fix | `idle_stimulus.rs`, combat path | `combat_state` / `melee_hit` counts match or explain via RNG trace |
| P1.4 | Extend fix to panic scenario (same cadence, + low-HP dance path) | same | panic lockstep PASS |

### 4.4 Verification

```bash
TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --synthetic \
  scripts/scenarios/kite_rat_stand_melee.scenario
python3 scripts/summarize_chase_gaps.py --ref log/chase_path_cip.log \
  --rust log/chase_path_rust.log --monster rat --max-tick 6000 --lockstep
# repeat for kite_rat_panic.scenario
```

**Exit criteria:** stand + panic `mismatch_counts` all zero; divergence report §22.

---

## 5. Phase 2 — Cyclops quad (4/6)

**Target:** `cyclops` lockstep PASS.

### 5.1 Problem statement

| Symptom | ref | rust |
|---------|-----|------|
| `branch` at tick=0 | 0 | 2 (`melee_dance`) |
| `todo_go` @2000 | 4× `enter` | 0 |
| `shortway` | 4 @2000 | 0 |
| `go_exec[0]` diagonal | diag=1 | diag=0 |

§20 had better cyclops counts (20/20, chase ref@2000 vs rust@4000). §21 batch-appear experiment regressed to tick=0 dance.

### 5.2 Work items

| ID | Task | Files / refs | Done when |
|----|------|--------------|-----------|
| P2.1 | **Safe multi-monster appear yield** — appear all → single batched yield → one drain; no tick=0 dance, no 0-event trace | `chase_kite_sim.rs`, `sim_harness.rs`, `game_world.rs` (`batch_appear_defer_idle`), `creature_todo.rs` | 0 branch @ tick=0; JSONL non-empty |
| P2.2 | Chase fires @ tick=2000 on all 4 monsters | same + `monster_events.rs` | `todo_go via=enter`×4 @2000 |
| P2.3 | **Diagonal first step** — `path_matching_tshortway` walk-back + `truncate_cipsoft_chase_queue` | `pathfinding.rs`, `monster_ai.rs`; C++ `cract.cc` TShortway | `go_exec[0] diag` matches ref |
| P2.4 | `shortway` step lists 4/4 pairwise | pathfinding + chase debug | `shortway` mismatch_counts = 0 |

### 5.3 Design constraint

Reuse `batch_appear_defer_idle` infrastructure from §21, but **do not** restore the regression path:

- Per-monster inline `creature_todo_yield` in appear → 0 JSONL (4 monsters).
- Batch yield without defer → tick=0 idle dance.

Required pattern (C++ parity): set all targets → yield all → **one** `DrainTodoQueue` / `run_sim_tick` after first `advance_ms 2000`.

### 5.4 Verification

```bash
TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --synthetic \
  scripts/scenarios/kite_cyclops_quad_chase.scenario
python3 scripts/summarize_chase_gaps.py --ref log/chase_path_cip.log \
  --rust log/chase_path_rust.log --monster cyclops --max-tick 4000 --lockstep
```

**Exit criteria:** cyclops lockstep PASS; restore 20/20 event alignment; divergence report §23.

---

## 6. Phase 3 — Kite rat (5/6)

**Target:** `kite` lockstep PASS.

### 6.1 Problem statement

C++ produces **zero** JSONL events: every scenario step uses `advance_ms 0`, so `ServerMilliseconds` never advances and scheduled todos never drain. Rust still logs tick=0 appear events.

This is primarily a **harness/scenario** gap, not core AI impossibility.

### 6.2 Work items

| ID | Task | Files | Done when |
|----|------|-------|-----------|
| P3.1 | Add non-zero `advance_ms` between kite steps (or explicit wall budget) | `scripts/scenarios/kite_rat_melee.scenario` | C++ JSONL non-empty |
| P3.2 | Align kite movement trace after time model fixed | movement + idle path | comparable event counts |
| P3.3 | Lockstep compare through `wall_ms=6000` | battery | kite PASS |

**Exit criteria:** kite lockstep PASS; document C++ time-advance requirement in scenario header comment.

---

## 7. Phase 4 — Cobra poison (6/6)

**Target:** `cobra` lockstep PASS — completes battery.

### 7.1 Problem statement

Cobra closes to melee before E4 spell delay fires. Rust logs extra `melee_dance` @ tick=0; `todo_go` / combat counts diverge. Path + chase arms otherwise strong (`todo_go`/`shortway`/`go_exec` 100% on paired prefix in §21).

### 7.2 Work items

| ID | Task | Files / refs | Done when |
|----|------|--------------|-----------|
| P4.1 | Tune scenario positions/timing so poison cast window opens | `kite_cobra_poison.scenario` | ref `spell_cast` or agreed E4 signal fires |
| P4.2 | Align `DistanceAttack` / spell enqueue cadence with idle drain | `monster_combat.rs`, `idle_stimulus.rs` | combat trace counts match |
| P4.3 | Inherit appear cadence fix from Phase 1/2 if cobra shows tick=0 dance | harness | no spurious tick=0 branch |

**Exit criteria:** cobra lockstep PASS; divergence report §24; **battery summary 6/6 PASS**.

---

## 8. Phase 5 — Hold and expand

After 6/6:

| Track | Action |
|-------|--------|
| **Regression gate** | Run `run_sim_battery.py --synthetic` on PRs touching idle/todo/path/combat |
| **New scenarios** | Add rows to [Coverage Matrix](TFS-RUST_772_Sim_Coverage_Matrix.md) before claiming layer parity |
| **Live replay** | Snake / live map traces via `compare_chase_live_logs.py` (orthogonal to synthetic gate) |
| **Compare hygiene** | Resolve known logging diffs (e.g. C++ `melee_hit armor=0`) — tooling or C++ hook, not gate relaxation |
| **RNG audit** | Maintain glibc draw-order documentation for spawn → first combat event |

---

## 9. Cross-cutting workstreams

These span multiple phases:

### 9.1 Glibc RNG draw order

Every `random()` / `parity_random()` between spawn resync and first combat event must match C++ consumption order.

- Tools: `TFS_SIM_RNG_TRACE=1`, `sim_glibc_rand.rs`
- Blocks: stand/panic dance *timing* if preamble draws differ; combat damage rolls

### 9.2 Combat trace alignment

Even when movement locksteps, extra `melee_hit` / `attack_enqueue` fail the gate.

- Likely resolves after tick cadence (Phase 1) — attacks firing on wrong beats
- If not: audit `DelayAttack` / `EarliestAttackTime` vs C++ `crcombat.cc`

### 9.3 Compare script

- Semantic ID roles (attacker/target/self) — done §20
- Optional: ignore C++ `melee_hit.armor=0` artifact if behavior matches
- Do **not** add tick-tolerance without explicit user approval

---

## 10. Risks and non-goals

| Risk | Mitigation |
|------|------------|
| Batch appear yield silences trace | Test cyclops event count > 0 before declaring win |
| Fixing cadence breaks kill sleeping posture | Re-run kill scenario every phase |
| Path fix without cadence masks wrong tick | Fix cadence first on cyclops |
| Scenario tuning passes gate but misses live maps | Phase 5 live replay track |

**Non-goals for this trajectory doc:**

- 1098 mechanics parity
- Wire/protocol parity (separate axis per `PROTOCOL_VERSIONING.md`)
- Relaxing lockstep gate definition

---

## 11. Verification checklist (every phase)

```bash
# Unit tests
/home/jessec/.local/bin/rtk cargo test -p tfs-rust-core

# Rebuild C++ harness (when C++ side changed)
scripts/tibia_game_dev.sh build

# Query manager (terminal 1)
scripts/tibia_game_dev.sh run-qm

# Full battery
TFS_SIM_SEED=772 python3 scripts/run_sim_battery.py --synthetic

# Inspect summaries
ls log/summary_*.txt
```

Record results in `docs/TFS-RUST_772_Sim_Divergence_Report.md` (next §22+).

---

## 12. Milestone scorecard (living)

Update this table at each phase closeout.

| Phase | Gate | Stand | Panic | Cyclops | Kite | Cobra | Kill | Date |
|-------|------|-------|-------|---------|------|-------|------|------|
| §21 | 1/6 | FAIL | FAIL | FAIL | FAIL | FAIL | **PASS** | 2026-06-14 |
| §22 | 3/6 | **PASS** | **PASS** | FAIL | FAIL | FAIL | **PASS** | 2026-06-14 |
| §23 | 3/6 | **PASS** | **PASS** | FAIL | FAIL | FAIL | **PASS** | 2026-06-14 |
| §24 | 5/6 | | | | | | | |
| §25 | 6/6 | | | | | | | |

---

## Changelog

| Date | Change |
|------|--------|
| 2026-06-14 | Initial trajectory doc — post §21 baseline (1/6), phases 0–5 scoped |
| 2026-06-14 | §22 milestone: stand + panic lockstep PASS (3/6) |
| 2026-06-14 | §23 Phase 2 partial: cyclops cadence 20/20; path pipeline fixes; shortway 2/4 — lockstep still 3/6 |
