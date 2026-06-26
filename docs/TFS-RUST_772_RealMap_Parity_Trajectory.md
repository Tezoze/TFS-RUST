# TFS-RUST 772 — Real-Map Parity Trajectory

**Date:** 2026-06-26  
**Pilot:** `kite_cyclops_six_real` (first real-map lockstep run)  
**Related:** [`TFS-RUST_772_RealMap_Scenario_Proposal.md`](TFS-RUST_772_RealMap_Scenario_Proposal.md), [`TFS-RUST_772_Real_Map_Kite_Sim_Plan.md`](TFS-RUST_772_Real_Map_Kite_Sim_Plan.md), [`TFS-RUST_772_Sim_Divergence_Report.md`](TFS-RUST_772_Sim_Divergence_Report.md)

---

## 1. Executive summary

**Pre-P3 pilot** (`kite_cyclops_six_real`, no `NO_WILD`): infrastructure proven, lockstep **FAIL**
(ref 230 vs rust 49) — see §3 (historical).

**Post-P3** (`kite_cyclops_one_real` / `six_real`, `TFS_KITE_NO_WILD=1`, `wall_ms=5000`):
harness and minimal control are **close but not lockstep-clean**.

| Dimension | Pre-P3 pilot | Post-P3 (`one_real`, `NO_WILD`) |
|-----------|--------------|-----------------------------------|
| Harness / scenario | PASS | **PASS** — both stacks complete |
| Map / walkability | PASS | **PASS** — route audit 0 FAIL |
| `player_walk` tiles | (not isolated) | **PASS** — `harness_player_step` **5/5** |
| Event volume | ref 230 / rust 49 | **10 / 10** |
| `go_exec` pairwise (ordered) | 0% | **3/3 = 100%** |
| Lockstep gate | FAIL | **FAIL** — see §12 remaining divergences |
| vs synthetic quad | 20/20 PASS | Real-map still **not** synthetic gate |

**Bottom line:** P3 closed spawn placement, appear-idle timing, and map-spawn noise.
**Remaining gap:** trace-field / tick-phasing mismatches on `todo_go`/`shortway` comparators,
`go_exec` step **timing** (same tiles, different ticks), FillMap creature-occupation tiles,
and `attack_enqueue` index pairing (1/2). Not a different chase model — scheduling and
compare semantics.

---

## 2. What was run

### Scenario

`scripts/scenarios/kite_cyclops_six_real.scenario`

| Field | Value |
|-------|-------|
| Area | Cyclops gravel bowl `(32451, 32065, 7)` |
| Player | `player_start` + **5× `player_walk`** (200 ms) + 2× `advance_ms 2000` tail (`wall_ms=5000`) |
| Monsters | **1** (ramp to 6 — same bowl, east-adjacent spawn) |
| Arena | `32451 32068 4` (metadata; C++ skips full-grid check when `!ArenaSynthetic`) |
| Terrain | Rust `data/world/forgotten.otbm`; C++ `runtime/map/*.sec` |
| Synthetic | **none** (`arena_synthetic` omitted, no `--synthetic`) |

### Commands

```bash
# Rust-only (walkability + parse)
TFS_SIM_SEED=772 cargo run -p tfs-rust-core --bin chase_kite_sim -- \
  scripts/scenarios/kite_cyclops_six_real.scenario

# Full lockstep (QM required)
TFS_SIM_SEED=772 python3 scripts/run_realmap_sim_battery.py
# or single:
TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --real-map \
  scripts/scenarios/kite_cyclops_six_real.scenario
```

### Artifacts

| Artifact | Path |
|----------|------|
| Gap summary (post-P3) | `log/summary_realmap_cyclops_one_real.txt` |
| C++ trace (post-P3) | `log/chase_path_cip_realmap_cyclops_one_real.log` |
| Rust trace (post-P3) | `log/chase_path_rust_realmap_cyclops_one_real.log` |
| Pre-P3 pilot summary | `log/summary_realmap_cyclops_six_real.txt` (early run; see §3) |

---

## 3. Lockstep results — pre-P3 pilot (historical)

> **Superseded by §12** for current `kite_cyclops_one_real` with `TFS_KITE_NO_WILD=1`.
> Kept as baseline for the first `six_real` run before spawn isolation and P3 fixes.

### Scenario: `kite_cyclops_six_real` (no `NO_WILD`, early wall budget)

### Event totals

| Event | C++ (ref) | Rust | Δ |
|-------|-----------|------|---|
| `branch` | 19 | 0 | −19 |
| `todo_go` | 49 | 12 | −37 |
| `shortway` | 30 | 12 | −18 |
| `go_exec` | 68 | 13 | −55 |
| `combat_state` | 31 | 6 | −25 |
| `attack_enqueue` | 32 | 6 | −26 |
| `melee_hit` | 1 | 0 | −1 |
| **all** | **230** | **49** | **−181** |

### Pairwise sequence match (index-aligned)

| Event | Match |
|-------|-------|
| `todo_go` | 2/12 = **16.7%** |
| `shortway` | 0/12 = **0%** |
| `go_exec` | 0/13 = **0%** |
| `combat_state` | 6/6 = **100%** (paired prefix only — counts still diverge) |
| `attack_enqueue` | 6/6 = **100%** (paired prefix only) |

Diagonal `go_exec`: ref **0/68**, rust **0/13** (both 0% — cardinal-only on this trace).

### First divergence (positions)

| Event | C++ (ref) | Rust |
|-------|-----------|------|
| `todo_go[0]` | `enter` @ `(32451, 32065, 7)` | `enter` @ `(32450, 32069, 7)` |
| `shortway[0]` | from `(32451, 32065, 7)` → path toward `(32457, 32067, 7)…` | from `(32450, 32069, 7)` → `(32456, 32068, 7)…` |
| `go_exec[0]` | `(32456, 32071)` → `(32456, 32070)` | `(32455, 32072)` → `(32455, 32071)` |

C++ emits heavy early activity (`branch` roam @ tick 200: ref=5, rust=0; `go_exec` @400: ref=12, rust=0).

### Branch mix (C++ only)

| Arm | ref | rust |
|-----|-----|------|
| `roam` | 18 | 0 |
| `melee_dance` | 1 | 0 |

Rust logged **no** `branch` events on this run — idle/roam path may differ or trace hooks may not fire the same arms on real terrain.

---

## 4. Comparison: real-map vs synthetic cyclops

| Metric | Synthetic `kite_cyclops_quad_chase` | Pre-P3 `six_real` (no `NO_WILD`) | Post-P3 `one_real` (`NO_WILD`) |
|--------|-------------------------------------|----------------------------------|--------------------------------|
| Arena | `arena_synthetic 1`, uniform wp=150 | Native OTBM / `.sec` | Native OTBM / `.sec` |
| Monsters | 4 | 1 (pilot; map spawns leaked) | 1 (harness only) |
| Player move | `player_pos` teleports | `player_walk` legal steps | `player_walk` legal steps |
| Total events | ref=20, rust=20 | ref=230, rust=49 | ref=**10**, rust=**10** |
| `go_exec` pairwise | **4/4 = 100%** | **0/13 = 0%** | **3/3 = 100%** |
| Lockstep gate | **PASS** | **FAIL** | **FAIL** (tick/field residuals) |

Real-map exposes terrain-dependent pathfinding and longer combat tails that the synthetic slab
hides. Post-P3 minimal control matches synthetic on **ordered `go_exec` tiles** but not on
full lockstep gate (§12).

---

## 5. Map parity (confirmed)

| Check | Status |
|-------|--------|
| OTBM converted from `runtime/map/` `.sec` | **Confirmed** (author) |
| Spot-check `(32451, 32065, 7)` | Same semantics: ground wp/speed **150**, 3-item stack (ground + 2 overlays) |
| RME vs `.sec` id line | Different numeric variants (`4573/4605/477` OTBM vs `4562/4594/602` `.sec`) — normal for 772 gravel families |
| Rust `validate_positions_walkable` | **PASS** on full scenario route |

See [`772_OBJECTS_SRV_TO_OTB_LOOKUP.md`](772_OBJECTS_SRV_TO_OTB_LOOKUP.md) for id-namespace rules.

---

## 6. Infrastructure delivered (pilot PR)

| Item | Status |
|------|--------|
| `player_walk` Rust (`walk_player_adjacent`, `chase_kite_sim`) | Done |
| `player_walk` C++ (`chase_kite_scenario.cc`, relaxed `ValidateArena`) | Done |
| `kite_cyclops_six_real.scenario` | Done |
| `run_kite_scenario.py --real-map` | Done |
| `run_realmap_sim_battery.py` | Done |
| Proposal / kite plan docs | Updated |

---

## 7. Divergence hypothesis (ordered)

### Closed in P3

| # | Hypothesis | Resolution |
|---|------------|------------|
| 1 | Player `player_walk` tick alignment | **Closed** — `harness_player_step` **5/5** |
| 2 | Map spawn noise (wild cyclops) | **Closed** — `TFS_KITE_NO_WILD=1`; 1 harness monster |
| 3 | Spawn tile `(32453)` vs C++ `SetOnMap` | **Closed** — Rust `harness_place_creature_login` → `(32454,32065)` |
| 4 | Appear-idle defer (Rust @3000 vs C++ @200) | **Closed** — `clear_harness_appear_idle_defer` on `player_walk` |
| 5 | FillMap dump never emitted (C++) | **Closed** — one-shot first `fill_map` @ tick 200 |

### Still open (P4+)

| # | Hypothesis | Evidence | Next action |
|---|------------|----------|-------------|
| A | **`go_exec` tick phasing** | Same step tiles; Rust first `go_exec` @**200**, C++ @**400**; per-tick counts differ @200/400/1000/2000/4000 | Align walk-step drain vs `NotifyGo` delay (`cract.cc` step scheduling) |
| B | **`todo_go` / `shortway` compare field** | Summarize pairs on **dest** tuple: C++ `(32451,…)` vs Rust `(32450,…)` @200 while **path steps match** (south hook); trace `start`/`from` both `(32454,32065)` | Fix comparator to use monster `from`/`start`, not player dest; or align chase-target tile snapshot timing |
| C | **`attack_enqueue` pairing** | **1/2** index-aligned despite equal counts (2/2) | Trace second enqueue timing / `idle_tail` vs `CreatureMoveStimulus` re-arm |
| D | **FillMap creature occupation** | Gravel route tiles PASS; `(32451,32065)` / `(32454,32065)` / `(32450,32065)` differ on `walkable`/`wp=-1` between stacks | Align `dump_tshortway_fill_walkable_viewport` creature blocking with C++ `FillMap` |
| E | **Idle / roam branches** | 0 `branch` on minimal control (both sides) — unproven on multi-monster / longer kite | Re-test when ramping `six_real` to 6 monsters |
| F | **OTBM vs `.sec` id variants** | Route audit WARN only; no FAIL | Monitor if path geometry diverges on cliff-edge tiles |

---

## 8. Next steps

### P0 — Baseline hygiene

- [x] Re-run lockstep and archive logs under a dated name (`log/realmap_pilot_20260626_*`) so future diffs have a fixed baseline.
- [x] Add §**Real-map pilot** entry to [`TFS-RUST_772_Sim_Divergence_Report.md`](TFS-RUST_772_Sim_Divergence_Report.md) (separate from synthetic 4/6 gate).

### P1 — Isolate divergence class

- [x] **Real-map 1-cyclops baseline** — `32453 32065` spawn, short U-loop; re-run lockstep before adding monsters back (`kite_cyclops_one_real.scenario`).
- [x] **FillMap probe** at `(32451, 32065, 7)` and one cliff-edge tile (`1099` bank) — compare rust vs C++ JSON dumps (`scripts/probe_realmap_fillmap.py`, `cyclops_bowl_real_fill_walkable_dump_at_tick_2000`).
- [x] Tick-alignment table: log player tile after each `player_walk` on both stacks (first 5 steps) — `harness_player_step` JSONL + `scripts/compare_harness_player_walk.py`.

### P2 — Tooling

- [x] `scripts/audit_realmap_route.py` — for each `player_walk` / `monster` coord: `.sec` first `Content` id, OTBM ground id, walkability flag (from kite plan §4).
- [x] Extend `compare_fill_walkable.py` preset for cyclops bowl viewport (not only synthetic NW coords).

### P3 — Parity work (after classification)

- [x] **Harness: map spawn isolation** — `TFS_KITE_NO_WILD=1` on real-map C++ runs (wired in `run_kite_scenario.py` / `probe_realmap_fillmap.py`; explicit in `run_realmap_sim_battery.py`).
- [x] **Spawn coord** — Rust `harness_place_creature_login` mirrors C++ `SetOnMap` → `(32454,32065)` from scenario `(32453,32065)`.
- [x] **Appear-idle defer** — `clear_harness_appear_idle_defer` on `player_walk`; first chase @200 both sides.
- [x] **FillMap dump tick** — C++ one-shot first `fill_map`; probe `--start 32454 32065 7`; route tiles PASS (occupation diffs on player/cyclops tiles).
- [x] Fix highest-signal path bucket after above — `go_exec` **3/3** pairwise on `one_real`; `shortway` path steps match (south hook).
- [x] Re-run `run_realmap_sim_battery.py`; event totals **10/10** on `one_real`.
- [ ] Optional: live repro at bowl coords + `compare_chase_live_logs.py`.

### P4 — Suite expansion (only after cyclops real-map trends better)

Target §12.2 divergences first (`go_exec` tick phasing, compare-field semantics, FillMap occupation).

- [ ] Align `go_exec` tick scheduling (Rust step delay vs C++ drain).
- [ ] Fix `summarize_chase_gaps.py` todo_go/shortway tuple or chase-target snapshot timing.
- [ ] FillMap creature-blocking parity on occupied priority tiles.
- [ ] Lockstep **PASS** on `kite_cyclops_one_real` before monster ramp.
- [ ] Compare real-map summary vs synthetic quad in divergence report.
- [ ] Second real-map scenario (e.g. Thais flat `1011-1009-07` control from proposal).
- [ ] Do **not** add real-map rows to synthetic CI gate until lockstep PASS on minimal control.

---

## 9. Trajectory scorecard

| Milestone | Target | Current |
|-----------|--------|---------|
| Harness runs real terrain | Both stacks complete scenario | **Done** |
| Map loader agreement | Route walkable both sides | **Done** (`audit_realmap_route.py`: 0 FAIL, id variants WARN only) |
| `player_walk` parity | Same player tile @ same tick | **Done** — `harness_player_step` **5/5** with `TFS_KITE_NO_WILD=1` |
| FillMap real terrain | Viewport match cyclops bowl | **Partial** — `fill_map` emits; route tiles PASS; creature-occupied priority tiles differ |
| `go_exec` pairwise | ≥1/1 on minimal control | **3/3** on `one_real` (`NO_WILD`) |
| Full kite lockstep | Summarize gate PASS | **FAIL** — ordered `go_exec` 3/3 but tick/field residuals (§12) |

---

## 10. Re-run checklist

```bash
# terminal 1
scripts/tibia_game_dev.sh run-qm

# terminal 2 — route audit (no QM)
python3 scripts/audit_realmap_route.py scripts/scenarios/kite_cyclops_one_real.scenario

# FillMap probe (QM + TFS_KITE_NO_WILD=1 via probe script)
TFS_SIM_SEED=772 python3 scripts/probe_realmap_fillmap.py

# 1-cyclops lockstep (run_kite_scenario.py sets TFS_KITE_NO_WILD=1 for --real-map)
TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --real-map \
  scripts/scenarios/kite_cyclops_one_real.scenario

# Full battery
TFS_SIM_SEED=772 python3 scripts/run_realmap_sim_battery.py
```

Expect **lockstep FAIL** until §12 divergences close (primarily `go_exec` tick phasing and
compare-field semantics). Synthetic battery remains the merge gate.

---

## 11. P2 classification run (`kite_cyclops_one_real`, 2026-06-26)

Artifacts: `log/realmap_fillmap_probe_20260626.out`, `log/realmap_one_real_nowild_20260626.out`,
`log/chase_path_cip_realmap.log`, `log/chase_path_rust_realmap.log`, `log/fill_walkable_rust_cyclops_bowl.json`.

### 11.1 Route audit (`audit_realmap_route.py`)

| Result | Detail |
|--------|--------|
| **0 FAIL** | Walkability + wp agree on all route tiles |
| **5 WARN** | OTBM ground id ≠ `.sec` first `Content` id (gravel family variants, e.g. 4573 vs 4562) — expected |
| **1 PASS** | Monster tile `(32453,32065)` — ids match (`104` sand bank, wp 160) |

Terrain semantics are aligned; numeric id families differ only on gravel overlays.

### 11.2 Player walk + map population

| Check | Without `TFS_KITE_NO_WILD` | With `TFS_KITE_NO_WILD=1` |
|-------|---------------------------|---------------------------|
| `harness_player_step` | **5/5** match | **5/5** match |
| C++ monsters in trace | **9** distinct `todo_go` origins (map bowl spawns) | **1** cyclops |
| Total chase events | ref **85** / rust **13** | ref **10** / rust **8** |
| `todo_go` pairwise | 1/1 prefix only (noise from extra monsters) | **1/1** |
| `melee_hit` pairwise | 0/1 (damage skew from wrong pull-in) | **1/1** (52 dmg both sides @5000) |

**P3 action (harness):** `run_kite_scenario.py --real-map` now sets `TFS_KITE_NO_WILD=1` for C++
(`crmain.cc` `ChaseHarnessSkipsWildCreatures` / `PurgeAllMonstersForChaseHarness`). Required for
any meaningful real-map lockstep — without it C++ loads native cyclops from `.sec` spawns.

### 11.3 FillMap compare (`probe_realmap_fillmap.py`)

| Side | Status (post-P3) |
|------|------------------|
| Rust JSON dump | **Written** — `setup_cyclops_bowl_real_first_shortway` @ tick **200**, monster @ `(32454,32065)` |
| C++ `fill_map` JSONL | **Written** — one-shot first `FillMap` @ tick **200** (`cract.cc`) |
| Compare `--start` | **(32454, 32065, 7)** — post-`SetOnMap` tile, not scenario `(32453,…)` |
| Route gravel tiles | **PASS** — U-loop corridor tiles match |
| Occupied tiles | **FAIL** — player/cyclops tiles differ on `walkable` / `wp=-1` (§12.2 E) |

### 11.4 Path divergence — pre-P3 (`NO_WILD`, 1 cyclops)

> **Historical** — before spawn login + appear-defer fixes. See §12 for current state.

| Event | C++ (ref) | Rust |
|-------|-----------|------|
| `shortway[0]` | from `(32454,32065)` → south hook | from `(32453,32065)` → west 1-step |
| `go_exec[0]` | `(32454,32065)→(32454,32066)` @400 | `(32453,32065)→(32452,32065)` @5000 |

Root causes (all addressed in P3): spawn `SetOnMap`, appear-idle defer, map spawn noise.

### 11.5 P3 checklist

- [x] Rust spawn mirrors C++ `SetOnMap` → `(32454,32065)`.
- [x] Appear-idle defer parity (first chase tick @200).
- [x] FillMap dump alignment + route-tile compare on `cyclops-bowl` preset.
- [x] `go_exec` pairwise **3/3** on `kite_cyclops_one_real`.
- [x] Re-run six_real battery with `NO_WILD`; event totals **10/10**.

---

## 12. Post-P3 lockstep (`kite_cyclops_one_real`, 2026-06-26)

Artifacts: `log/summary_realmap_cyclops_one_real.txt`, `log/summary_realmap_cyclops_six_real.txt`,
`log/chase_path_cip_realmap_cyclops_one_real.log`, `log/chase_path_rust_realmap_cyclops_one_real.log`,
`log/fill_walkable_rust_cyclops_bowl.json`.

Conditions: `TFS_KITE_NO_WILD=1`, `TFS_SIM_SEED=772`, `wall_ms=5000`, scenario tail
`1000 + 2000 + 1000` ms advances after U-loop.

### 12.1 What matches

| Check | Result |
|-------|--------|
| Event totals | **10 / 10** (both `one_real` and `six_real`) |
| `harness_player_step` | **5/5** @ 200…1000 |
| Monster spawn (trace `from`/`start`) | **(32454, 32065, 7)** both sides |
| First `shortway` tick | **200** both sides |
| `shortway` path steps | **Identical** south hook: `(32454,32066)→(32453,32066)→(32452,32066)` |
| `go_exec` ordered tiles | **3/3 = 100%** — same from/to pairs in sequence |
| `combat_state` | **2/2 = 100%** |
| `melee_hit` | **1/1** — 52 dmg @ tick 4000 |
| FillMap emit | Both sides log `fill_map` @ tick 200 |
| FillMap gravel route | `(32450,32066)`, `(32451,32066)`, `(32452,32066)` **PASS** |

### 12.2 Remaining divergences

#### A — Lockstep gate: FAIL

`summarize_chase_gaps.py --lockstep` returns non-zero despite matching event counts.
Battery: `cyclops_one_real` lockstep=**FAIL**, `cyclops_six_real` lockstep=**FAIL**.

#### B — `todo_go` / `shortway` pairwise: 0/1 (comparator)

Summarize reports first mismatch on **dest** position in the comparison tuple:

| Event | C++ (ref) | Rust |
|-------|-----------|------|
| `todo_go[0]` @200 | dest `(32451, 32065, 7)` | dest `(32450, 32065, 7)` |
| `shortway[0]` @200 | compare tuple start `(32451, 32065, 7)` | compare tuple start `(32450, 32065, 7)` |

**Trace ground truth** (JSONL `start`/`from` fields): both monsters at `(32454,32065)`;
path steps **match**. Divergence is **compare semantics / chase-target tile snapshot**
(player @ `32450` after walk; C++ still paths toward `32451` in `dest` field), not
different `TShortway` geometry.

#### C — `go_exec` tick phasing

Same step **tiles** in ordered pairwise (3/3), but **per-tick event counts** differ:

| Tick | C++ `go_exec` | Rust `go_exec` |
|------|---------------|----------------|
| 200 | 0 | 1 — `(32454,32065)→(32454,32066)` |
| 400 | 1 — same step | 0 |
| 1000 | 0 | 1 |
| 2000 | `(32454,32066)→(32453,32066)` | `(32453,32066)→(32452,32066)` — **one step behind** |
| 4000 | 1 + `melee_hit` | 0 |

Rust issues the first walk step on the same tick as `shortway`; C++ defers `go_exec` to
the next drain (+200 ms). Later steps accumulate ~1-tile phase offset.

#### D — `attack_enqueue`: 1/2 pairwise

Counts match (2/2); index-aligned pairing fails on second enqueue (timing / `idle_tail` arm).

#### E — FillMap: creature-occupied priority tiles

`compare_fill_walkable.py --preset cyclops-bowl` — route gravel **PASS**; priority **FAIL**
on tiles with creatures:

| Tile | Rust | C++ | Notes |
|------|------|-----|-------|
| `(32451, 32065, 7)` | walkable, wp=150 | wp=-1 | player_start; occupation differs |
| `(32454, 32065, 7)` | walkable, wp=160 | wp=-1 | cyclops tile |
| `(32450, 32065, 7)` | wp=-1 | walkable, wp=150 | player stand tile @ dump |

Viewport: ~46/462 mismatches (mostly viewport edge + occupation).

### 12.3 P4 targets (from open hypotheses §7)

1. Align `go_exec` tick scheduling (Rust `NotifyGo` / step delay vs C++ `DrainTodoQueue`).
2. Fix `summarize_chase_gaps.py` shortway/todo_go tuple to compare monster origin + steps,
   not stale player dest — or align target tile in chase trace @ path time.
3. FillMap creature blocking parity on occupied tiles.
4. Re-run battery; target lockstep **PASS** on `one_real` before six-monster ramp.
