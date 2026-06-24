# TFS-RUST 772 — Lockstep Parity Trajectory

**Date:** 2026-06-24  
**Status:** active plan — post §30 X2 follow-up  
**Audience:** implementing engineer  
**Goal:** reach **6/6 lockstep PASS** on the synthetic sim battery, then extend the gate to the hunter/dragon scenarios.

**Related docs**

| Doc | Role |
|-----|------|
| [TFS-RUST_772_Sim_Divergence_Report.md](TFS-RUST_772_Sim_Divergence_Report.md) | Historical A/B metrics, §21 baseline |
| [TFS-RUST_772_Sim_Coverage_Matrix.md](TFS-RUST_772_Sim_Coverage_Matrix.md) | Event-type coverage map |
| [TFS-RUST_772_Monster_Combat_Integration_Plan.md](TFS-RUST_772_Monster_Combat_Integration_Plan.md) | E0–E6 combat integration (largely done) |

---

## 0. Executive summary

| Metric | Today (§30) | Target |
|--------|-------------|--------|
| Base lockstep gate | **5 / 6 PASS** (83%) | **6 / 6 PASS** (100%) |
| Extended lockstep gate | **0 / 3 PASS** (0%) | **3 / 3 PASS** (100%) |
| Hunter dist_chase first go_exec cadence | **fixed** (X1 closed) | match ref @t=4000 |
| Hunter `todo_go` enter contract | **fixed** (X2 closed) | ref `max` matches Rust |
| Dragon melee_dance → flee sequence | Rust skips melee_dance (X3) | melee_dance then flee |
| Cobra E4 spell delay | FAIL (Phase 4 open) | spell_cast timing aligned |

**Bottom line:** stand/panic/kill/cyclops/kite lockstep **PASS** (5/6). Phase 3 **closed** (§27). Phase 4 (cobra) and Phase 5 (extended: hunter + dragon) are open. No regression since §27 — the earlier 1/6 result was a stale C++ PID file + missing `--synthetic` flag.

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

## 2. Current baseline (§28)

Battery: `TFS_SIM_SEED=772`, `--synthetic`, QM on `127.0.0.1:7173`.

**Base battery:**

| Scenario | Lockstep | ref / rust events | Primary blocker |
|----------|----------|-------------------|----------------|
| **kill** | **PASS** | 2 / 2 | — |
| **stand** | **PASS** | — | — |
| **panic** | **PASS** | — | — |
| **cyclops** | **PASS** | — | — |
| **kite** | **PASS** | 8 / 8 | — |
| cobra | FAIL | — | E4 spell-cast timing / scenario tuning |

**Extended battery (first run — §28):**

| Scenario | Lockstep | ref events | rust events | Primary blocker |
|----------|----------|------------|-------------|----------------|
| hunter_chase | FAIL | 6 | 14 | X4: combat_state inflation / early dist-flee follow-on |
| hunter_dist_flee | FAIL | 11 | 14 | X4 + X5: combat_state inflation + spell tag normalization (X2 closed) |
| dragon_lowhp_flee | FAIL | 9 | 6 | X3: melee_dance arm skipped before flee |

**Known regression to avoid:** batched appear `ToDoYield` without per-monster idle defer → cyclops tick=0 dance or silenced JSONL.

---

## 3. Trajectory overview

Phases are ordered by **lockstep ROI** (scenarios flipped per unit effort). Each phase ends with a battery rerun and a new divergence-report section.

```
Phase 0 ──► 1/6   kill PASS                      [DONE §21]
Phase 1 ──► 3/6   + stand, panic                 [DONE §22]
Phase 2 ──► 4/6   + cyclops                      [DONE §26]
Phase 3 ──► 5/6   + kite                         [DONE §27]
Phase 4 ──► 6/6   + cobra                        [E4 spell delay — open]
Phase 5 ──► 6/6 + extend  hunter/dragon baseline [0/3 §28 — open]
```

| Phase | Gate | Est. effort | Depends on |
|-------|------|-------------|------------|
| 0 | 1/6 | — | §21 shipped |
| 1 | 3/6 | Medium | Phase 0 |
| 2 | 4/6 | Medium–high | Phase 1 cadence patterns |
| 3 | 5/6 | Low | Harness only |
| 4 | 6/6 | Medium | E4 combat path stable |
| 5 | 6/6 base + 3/3 ext | Medium | X1 dist_chase cadence; X3 dragon idle priority |

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

## 5. Phase 2 — Cyclops quad (4/6) — partial

**Target:** `cyclops` lockstep PASS. **Battery today:** 3/6 (kill + stand + panic PASS).

### 5.1 Closed (P2.1–P2.4, P2.5a–d)

| Symptom | ref | rust | Status |
|---------|-----|------|--------|
| `branch` @ tick=0 | 0 | 0 | **PASS** |
| Events | 20 | 20 | **PASS** |
| `todo_go` @2000 | 4× `enter` | 4× `enter` | **PASS** |
| `combat_state` / `attack_enqueue` | 4/4 | 4/4 | **PASS** |
| All four `shortway` @2000 | NW diag + far-N north | same | **PASS** (4/4) |
| Fill-map priority tiles | `(32359,32290)` etc. | match | **PASS** |

Shipped: `harness_defer_appear_idle` through kite window, signed TShortway heuristic, grass TypeID 102 synthetic, `monster_tshortway_fill_walkable`, walk_queue LIFO push, `Waypoints==-1` relax (`cract.cc:158-202`), **`overlay_synthetic_ground_in_arena`** (OTBM items preserved), `scripts/compare_fill_walkable.py`.

### 5.2 Closed — P2.5 root cause

| Finding | Detail |
|---------|--------|
| Not expand math | Python `cract.cc` port matched Rust pre-fix; live C++ differed on walkability |
| Not creature gates alone | C++ stack probe: **fir tree 3682** (`Unpass`) on `(32359,32290)` under grass |
| Fix | OTBM + grass overlay — same as C++ `LaySyntheticArena` semantics |

### 5.3 Closed — P2.5e (`go_exec` execution)

| Metric | ref | rust |
|--------|-----|------|
| `shortway` pairwise | — | **4/4** |
| `go_exec` pairwise | — | **4/4** |
| Diagonal `go_exec` (NW) | 1/4 @4000 | **1/4** @4000 |

**Fix:** `earliest_walk_server_ms` + `todo_start_go_delay` mirrors `CalculateDelay(TDGo)` / `ToDoStart` / `NotifyGo` (`cract.cc`). First `Go` arms @`server_ms+1` @2000; batch drain @4000 executes diagonal step.

### 5.4 Fresh oracle (June 2026)

Re-run after freeing port 7172 (`stop tfs-rust`; QM on 7173):

```bash
TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --synthetic \
  scripts/scenarios/kite_cyclops_quad_chase.scenario
```

**Result:** C++ paths **identical** to prior oracle — authoritative. Post-P2.5e Rust **shortway** + **go_exec** match on NW diagonal @4000.

### 5.5 Next steps (P2.5g → Phase 2 exit)

| ID | Task | Done when |
|----|------|-----------|
| P2.5f | Battery rerun + divergence §25 closeout | **done** — 3/6; cyclops `go_exec` 4/4 counts, 2/4 pairwise |
| P2.5g | Multi-monster `go_exec` drain order @same tick | **done** — cyclops lockstep PASS; battery **4/6** |

### 5.6 Verification

```bash
# QM on 7173; game port 7172 free (not tfs-rust)
TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --synthetic \
  scripts/scenarios/kite_cyclops_quad_chase.scenario
python3 scripts/summarize_chase_gaps.py \
  --ref log/chase_path_cip_cyclops.log \
  --rust log/chase_path_rust_cyclops.log --lockstep
```

**Exit criteria (Phase 2):** cyclops lockstep PASS (`shortway` + `go_exec` 4/4 pairwise); battery **4/6**. **Done** — §26 P2.5g closeout.

---

## 6. Phase 3 — Kite rat (5/6) — **DONE**

**Target:** `kite` lockstep PASS. **Status:** **PASS** (§27, battery 5/6).

### 6.1 Closed

| ID | Task | Status |
|----|------|--------|
| P3.1 | `advance_ms 2000` between kite steps (`wall_ms=6000`) | done |
| P3.2 | Harness drain-before-teleport + `CreatureMoveStimulus` `LockToDo` gate | done |
| P3.3 | Lockstep compare through `wall_ms=6000` | **PASS** |

**Exit criteria met:** kite lockstep PASS; scenario time model documented in §27.

---

## 7. Phase 4 — Cobra poison (6/6)

**Target:** `cobra` lockstep PASS — completes base battery.

---

## 7b. Phase 5 — Extended battery: hunter dist-chase/flee + dragon low-HP flee (§28 baseline)

**Target:** all 3 extended scenarios lockstep PASS (`--synthetic --extended`).

### 7b.1 Problem statements

**X1 — dist_chase go_exec 1 beat early (HIGH — hunter_chase, hunter_dist_flee) — CLOSED (2026-06-23 rerun)**

Fixed in `walk/mod.rs` (`process_creature_todo`) by preventing same-drain `Go` execute when `IdleStimulus` already armed `next_wakeup > server_ms`. First hunter `go_exec` now starts at t=4000 (not 2000), matching reference cadence. Remaining hunter drift is tracked under X2/X4/X5.

**X2 — todo_go enter step count off by 1 (MEDIUM — hunter_dist_flee) — CLOSED (2026-06-24)**

Closed in `creature_todo.rs`: the queued Rust path was already correct (`shortway.max=2`), but the 772 `todo_go` event was logging the generic follow-target chase budget (`CHASE_PATH_MAX_STEPS`) instead of the actual idle-arm contract. Dist-chase now logs `monster_idle_chase_step_budget(...)`; `idle_flee` logs the single-step `SearchFlightField` contract (`must=true`, `max=INT_MAX`).

**X3 — Dragon melee_dance arm skipped before runonhealth flee (HIGH — dragon_lowhp_flee)**

Ref at tick=2000: dragon still at melee range → idle picks `melee_dance` → moves to (32361,32291). Then at tick=4000 (next idle cycle, HP still low) → picks `flee`. Rust skips straight to `flee` at tick=2000. Root: `run_on_health_threshold` condition wins before the melee-range `melee_dance` guard in `idle_stimulus.rs`. C++ evaluates melee_dance eligibility before checking runonhealth flee.

### 7b.2 Work items

| ID | Task | Files | Done when |
|----|------|-------|-----------|
| X1 ✅ | Align dist_chase `TDoIdleChase` first idle wakeup cadence — defer first go_exec to post-appear drain window (same as §22 P1.2 fix, but for dist_chase arm) | `walk/mod.rs` | hunter_chase + hunter_dist_flee first `go_exec` @t=4000 (matched in §29 rerun) |
| X2 ✅ | Align `todo_go` trace contract to the active 772 idle arm: dist-chase logs `monster_idle_chase_step_budget`, flee logs `SearchFlightField` single-step contract | `creature_todo.rs` | `hunter_dist_flee` `todo_go` pairwise 2/2; first event now `max=2` |
| X3 | Guard `run_on_health_threshold` flee branch: skip if monster is at melee distance and melee_dance is eligible — match C++ idle priority order | `idle_stimulus.rs` | dragon `branch[0]` = `melee_dance` @t=2000 |
| X4 | Recheck `combat_state` gate after X1/X2 | `monster_ai.rs` | hunter `combat_state` ref=0 rust=0 |
| X5 | Normalize `spell_cast` type tag in Rust sim log: strip `:Physical` suffix | `sim_harness.rs` or compare script | `spell_cast[0]` type matches ref |
| X6 | Wire `player_damage` harness command to `damage_stimulus` JSONL event; suppress spurious dragon spell during flee | `sim_harness.rs`, `idle_stimulus.rs` | dragon `damage_stimulus` ref=rust; `spell_cast` ref=0 rust=0 |

### 7b.3 Verification

```bash
# After each fix:
TFS_SIM_SEED=772 python3 scripts/run_sim_battery.py --synthetic --extended
# Inspect per-scenario summaries:
cat log/summary_hunter_chase.txt log/summary_hunter_dist_flee.txt log/summary_dragon_lowhp_flee.txt
```

**Status after X2 follow-up (§30):** X1 and X2 closed; X3–X6 remain.

**Exit criteria:** all three extended scenarios lockstep PASS; divergence report §30; trajectory §12 updated.

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

## 8. Phase 6 — Hold and expand

After 6/6 base + 3/3 extended:

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

**Base battery:**

| Phase | Gate | Stand | Panic | Cyclops | Kite | Cobra | Kill | Date |
|-------|------|-------|-------|---------|------|-------|------|------|
| §21 | 1/6 | FAIL | FAIL | FAIL | FAIL | FAIL | **PASS** | 2026-06-14 |
| §22 | 3/6 | **PASS** | **PASS** | FAIL | FAIL | FAIL | **PASS** | 2026-06-14 |
| §23 | 3/6 | **PASS** | **PASS** | FAIL | FAIL | FAIL | **PASS** | 2026-06-14 |
| §24 | 3/6 | **PASS** | **PASS** | FAIL (2/4 path) | FAIL | FAIL | **PASS** | 2026-06-14 |
| §25 | 3/6 | **PASS** | **PASS** | FAIL (4/4 sw, 3/4 go) | FAIL | FAIL | **PASS** | 2026-06-15 |
| §26 | **4/6** | **PASS** | **PASS** | **PASS** | FAIL | FAIL | **PASS** | 2026-06-16 |
| §27 | **5/6** | **PASS** | **PASS** | **PASS** | **PASS** | FAIL | **PASS** | 2026-06-16 |
| §28 | **5/6** | **PASS** | **PASS** | **PASS** | **PASS** | FAIL | **PASS** | 2026-06-23 |
| Target | 6/6 | | | | | | | |

**Extended battery (`--extended`, first run §28):**

| Phase | Gate | Hunter Chase | Hunter Dist-Flee | Dragon Low-HP | Date |
|-------|------|-------------|-----------------|---------------|------|
| §28 | 0/3 | FAIL | FAIL | FAIL | 2026-06-23 |
| Target | 3/3 | | | | |

---

## Changelog

| Date | Change |
|------|--------|
| 2026-06-14 | Initial trajectory doc — post §21 baseline (1/6), phases 0–5 scoped |
| 2026-06-14 | §22 milestone: stand + panic lockstep PASS (3/6) |
| 2026-06-14 | §23 Phase 2 partial: cyclops cadence 20/20; path pipeline fixes; shortway 2/4 — lockstep still 3/6 |
| 2026-06-14 | §24 fresh C++ oracle refresh; FillMap/fill_walkable dump scoped as P2.5 next step |
| 2026-06-15 | §25.4 P2.5e closeout: `EarliestWalkTime`/`ToDoStart`; go_exec 4/4; P2.5f battery A/B scoped |
| 2026-06-15 | §25.7 P2.5f battery rerun: 3/6; cyclops go_exec order swap @4000; P2.5g scoped |
| 2026-06-16 | §26 P2.5g closeout: `WakeupTiePolicy` appear LIFO + go-step tie; cyclops **4/4** go_exec; battery **4/6**; Phase 2 done |
| 2026-06-16 | §27 Phase 3 closeout: kite harness drain-before-teleport + `CreatureMoveStimulus` `LockToDo`; battery **5/6** |
| 2026-06-23 | §28 extended battery baseline (`--synthetic --extended`): base 5/6 confirmed (no regression); extended 0/3 first run; X1–X6 root causes scoped; Phase 5 opened |
| 2026-06-24 | §30 X2 follow-up: `todo_go` trace contract aligned to actual 772 idle arm; X1/X2 closed, X3–X6 remain |
