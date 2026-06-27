# TFS-RUST 772 — Real-Map Parity Trajectory

**Date:** 2026-06-26  
**Pilot:** `kite_cyclops_six_real` (first real-map lockstep run)  
**Related:** [`TFS-RUST_772_RealMap_Scenario_Proposal.md`](TFS-RUST_772_RealMap_Scenario_Proposal.md), [`TFS-RUST_772_Real_Map_Kite_Sim_Plan.md`](TFS-RUST_772_Real_Map_Kite_Sim_Plan.md), [`TFS-RUST_772_Sim_Divergence_Report.md`](TFS-RUST_772_Sim_Divergence_Report.md)

---

## 1. Executive summary

**Pre-P3 pilot** (`kite_cyclops_six_real`, no `NO_WILD`): infrastructure proven, lockstep **FAIL**
(ref 230 vs rust 49) — see §3 (historical).

**Post-P5** (`kite_cyclops_one_real`, narrow 4-event gate): **lockstep PASS** (§14).

**Post-P6** (expanded JSONL + chase/face inline repath, 2026-06-27): **lockstep FAIL** on
15-event gate — core geometry still green; volume/scheduler gaps exposed (§15).

| Dimension | Pre-P3 pilot | Post-P4 | Post-P5 (narrow gate) | Post-P6 (expanded gate) |
|-----------|--------------|---------|-------------------------|-------------------------|
| Harness / scenario | PASS | **PASS** | **PASS** | **PASS** |
| Map / walkability | PASS | **PASS** | **PASS** | **PASS** |
| `player_walk` tiles | — | **5/5** | **5/5** | **5/5** |
| Event volume | 230 / 49 | **10 / 10** | **10 / 10** | **19 / 109** |
| `todo_go` index pairwise | 0% | **1/1** | **1/1** | **1/1** (count 1 vs 6) |
| `go_exec` index pairwise | 0% | **3/3** | **3/3** | **3/3** (tick buckets differ) |
| `go_exec` tick buckets | — | 400/2000/4000 | 400/2000/4000 | ref 400/2000/4000; rust 400/4000/5000 |
| `melee_hit` | — | dmg skew | **52 @4000** | **52** but rust @5000 |
| Lockstep gate | FAIL | FAIL | **PASS** (4 evt types) | **FAIL** (15 evt types) |

**Bottom line:** P3–P5 closed spawn, drain order, combat tail, and narrow-gate lockstep.
P6 expanded tracing shows Rust **inline repath on every `player_walk`** (todo_go 1→6,
idle_stimulus 2→9) and **+1000ms go_exec/melee phasing** — not a map regression; harness
player tiles and index-aligned chase arms still match.

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
| Gap summary (post-P6) | `log/summary_realmap_cyclops_one_real.txt` (gap table appended) |
| C++ trace (post-P6) | `log/chase_path_cip_realmap_cyclops_one_real.log` |
| Rust trace (post-P6) | `log/chase_path_rust_realmap_cyclops_one_real.log` |
| Gap summary (post-P5 narrow gate) | archived `log/realmap_pilot_20260626_*` |
| C++ trace (post-P3/P4) | `log/chase_path_cip_realmap_cyclops_one_real_nowild.log` |
| Rust trace (post-P4) | `log/chase_path_rust_realmap_p3.out` |

---

## 3. Lockstep results — pre-P3 pilot (historical)

> **Superseded by §13** for current `kite_cyclops_one_real` with `TFS_KITE_NO_WILD=1`.
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

| Metric | Synthetic `kite_cyclops_quad_chase` | Pre-P3 `six_real` | Post-P4 `one_real` (`NO_WILD`) |
|--------|-------------------------------------|-------------------|--------------------------------|
| Arena | `arena_synthetic 1`, uniform wp=150 | Native OTBM / `.sec` | Native OTBM / `.sec` |
| Monsters | 4 | 1 (map spawns leaked) | 1 (harness only) |
| Player move | `player_pos` teleports | `player_walk` legal steps | `player_walk` legal steps |
| Total events | ref=20, rust=20 | ref=230, rust=49 | ref=**10**, rust=**10** |
| `go_exec` pairwise | **4/4 = 100%** | **0/13 = 0%** | **3/3 = 100%** (tiles + ticks) |
| Lockstep gate | **PASS** | **FAIL** | **FAIL** (combat tail only — §13) |

Real-map exposes terrain-dependent pathfinding and longer combat tails that the synthetic slab
hides. Post-P4 minimal control matches synthetic on **movement/chase scheduling**; full lockstep
still blocked by combat-tail residuals (§13).

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

### Closed in P4

| # | Hypothesis | Resolution |
|---|------------|------------|
| A | **`go_exec` tick phasing** | **Closed** — first `go_exec` @**400** both sides; harness first-step delay uses `sim_harness_segment_ms`; idle defers `Go` on drain tick (`walk/mod.rs` `todo_start_go_delay`) |
| B | **`todo_go` / `shortway` compare field** | **Closed** — summarize compares `from`/`start` + steps, not chase-target `dest`; harness drains **before** `player_walk` so `dest` snapshot matches C++ @200 |
| D | **FillMap creature occupation** | **Closed (self-tile)** — `monster_tshortway_fill_walkable` keeps terrain wp on own tile (`crnonpl.cc` `MovePossible Execute=false`); occupation diffs on priority tiles were dump-timing skew (pre-walk vs post-walk) |

### Still open (post-P5 ramp)

| # | Hypothesis | Evidence | Next action |
|---|------------|----------|-------------|
| E | **Idle / roam branches** | 0 `branch` on minimal control (both sides) — unproven on multi-monster / longer kite | Re-test when ramping `six_real` to 6 monsters |
| F | **OTBM vs `.sec` id variants** | Route audit WARN only; no FAIL | Monitor if path geometry diverges on cliff-edge tiles |

~~C — `attack_enqueue` pairing~~ — **closed P5** (§14).  
~~G — `melee_hit` damage roll~~ — **closed P5** (§14).

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

### P4 — Movement/chase scheduling (2026-06-26)

- [x] Align `go_exec` tick scheduling — harness drain **before** `player_walk`; first-step delay =
  `sim_harness_segment_ms` at wall tick; pull appear-idle wakeup forward in
  `clear_harness_appear_idle_defer`.
- [x] Fix `summarize_chase_gaps.py` / `compare_chase_live_logs.py` — `todo_go` uses `from`,
  `shortway` uses `start` + steps (not chase-target `dest`).
- [x] FillMap self-tile occupation — own monster tile keeps terrain wp (`monster_ai.rs`).
- [x] Lockstep **PASS** on `kite_cyclops_one_real` — P5 combat tail (§14).
- [ ] Optional: live repro at bowl coords + `compare_chase_live_logs.py`.

### P5 — Combat tail lockstep (2026-06-26) — **DONE**

- [x] Close `attack_enqueue` second-index pairing (`idle_tail` vs `skipped` @4000) — `idle_stimulus.rs`.
- [x] Close `melee_hit` damage roll parity with `TFS_SIM_SEED=772` — harness RNG realign in `monster_do_attacking`.
- [x] Lockstep **PASS** on `kite_cyclops_one_real` (`summarize_chase_gaps.py --lockstep` exit 0).
- [x] Archive dated logs (`log/realmap_pilot_20260626_*`).
- [x] §33 real-map lockstep closeout in divergence report.

### P6 — Suite expansion (after one_real lockstep PASS)

- [ ] Ramp `kite_cyclops_six_real` to 6 monsters (verify branch/roam under load).
- [ ] Compare real-map summary vs synthetic quad in divergence report.
- [ ] Second real-map scenario (e.g. Thais flat `1011-1009-07` control from proposal).
- [ ] Do **not** add real-map rows to synthetic CI gate until six-monster ramp validated.

---

## 9. Trajectory scorecard

| Milestone | Target | Current |
|-----------|--------|---------|
| Harness runs real terrain | Both stacks complete scenario | **Done** |
| Map loader agreement | Route walkable both sides | **Done** (`audit_realmap_route.py`: 0 FAIL, id variants WARN only) |
| `player_walk` parity | Same player tile @ same tick | **Done** — `harness_player_step` **5/5** with `TFS_KITE_NO_WILD=1` |
| FillMap real terrain | Viewport match cyclops bowl | **Done (route + self-tile)** — gravel corridor PASS; self-tile wp matches C++ `MovePossible` |
| `todo_go` / `shortway` pairwise | 1/1 on minimal control | **Done** — 1/1 @200 (`from`/`start` + steps) |
| `go_exec` pairwise | ≥1/1 on minimal control | **Done** — 3/3 tiles + ticks @ 400/2000/4000 |
| Full kite lockstep | Summarize gate PASS | **PASS** — `one_real` + `six_real` (§14) |

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

# Rust-only quick check (no QM) — post-P4 movement parity
TFS_SIM_SEED=772 cargo run -p tfs-rust-core --bin chase_kite_sim -- \
  scripts/scenarios/kite_cyclops_one_real.scenario \
  --log log/chase_path_rust_p4_test.log

python3 scripts/summarize_chase_gaps.py \
  --ref log/chase_path_cip_realmap_cyclops_one_real.log \
  --rust log/chase_path_rust_p4_test.log \
  --monster cyclops --lockstep
```

Expect **lockstep PASS** on `one_real` (§14). Movement/chase (`todo_go`, `shortway`, `go_exec`)
and combat tail (`attack_enqueue`, `melee_hit`) should be **100%** pairwise. Synthetic battery
remains the merge gate.

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

> **Historical** — pre-P4 movement scheduling. Superseded by §13 for current `one_real` state.

Artifacts: `log/summary_realmap_cyclops_one_real.txt`, `log/summary_realmap_cyclops_six_real.txt`,
`log/chase_path_cip_realmap_cyclops_one_real.log`, `log/chase_path_rust_realmap_cyclops_one_real.log`,
`log/fill_walkable_rust_cyclops_bowl.json`.

Conditions: `TFS_KITE_NO_WILD=1`, `TFS_SIM_SEED=772`, `wall_ms=5000`, scenario tail
`1000 + 2000 + 1000` ms advances after U-loop.

### 12.1 What matches (still valid post-P4)

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

### 12.2 Remaining divergences (pre-P4 — all closed in §13 except combat tail)

#### B — `todo_go` / `shortway` comparator (closed P4)

Pre-P4 summarize paired on **dest** while Rust chased after `player_walk`. Fixed by harness
drain-before-walk + compare `from`/`start`.

#### C — `go_exec` tick phasing (closed P4)

Pre-P4 Rust fired first `go_exec` @200; C++ @400. Fixed by appear-idle wakeup pull-forward,
harness-wall first-step segment delay, and idle `Go` deferral on drain tick.

#### D — `attack_enqueue`: 1/2 pairwise (still open — §13)

#### E — FillMap occupation (closed P4 for self-tile; dump-timing explained remaining diffs)

Pre-P4 priority-tile diffs were largely **snapshot timing** (chase before vs after walk) plus
Rust blocking own origin tile (stricter than C++ `MovePossible Execute=false`).

---

## 13. Post-P4 lockstep (`kite_cyclops_one_real`, 2026-06-26)

Artifacts: `log/chase_path_cip_realmap_cyclops_one_real.log`, `log/chase_path_rust_p4_test.log`.

Conditions: `TFS_KITE_NO_WILD=1`, `TFS_SIM_SEED=772`, `wall_ms=5000`.

### 13.1 What matches

| Check | Result |
|-------|--------|
| Event totals | **10 / 10** |
| `harness_player_step` | **5/5** @ 200…1000 |
| Event order @200 | chase (`combat_state`, `shortway`, `todo_go`, `attack_enqueue`) **then** `harness_player_step` — matches C++ |
| `todo_go[0]` dest @200 | **`(32451, 32065, 7)`** both sides |
| `shortway` path steps | **Identical** south hook |
| `todo_go` pairwise | **1/1 = 100%** |
| `shortway` pairwise | **1/1 = 100%** |
| `go_exec` pairwise | **3/3 = 100%** — tiles **and** per-tick buckets |
| `go_exec` ticks | **400, 2000, 4000** both sides |
| `combat_state` | **2/2 = 100%** |
| `go_exec` per-tick mismatch volume | **0** (was 5 pre-P4) |

### 13.2 Remaining divergences

#### A — Lockstep gate: FAIL (combat tail only)

`summarize_chase_gaps.py --lockstep` returns non-zero. Movement events: **0 mismatches**.

#### B — `attack_enqueue`: 1/2 pairwise

| Index | C++ (ref) | Rust |
|-------|-----------|------|
| `[0]` @200 | `idle_tail` | `idle_tail` — **match** |
| `[1]` @4000 | `idle_tail` | `skipped` — **mismatch** |

#### C — `melee_hit`: damage roll

| Field | C++ (ref) | Rust |
|-------|-----------|------|
| tick | 4000 | 4000 |
| damage | 52 | 42 |
| attack | 54 | 47 |
| defense | 2 | 5 |

Tile positions at `go_exec` @4000 now align; damage skew is combat formula / strike-context,
not chase geometry.

### 13.3 P4 code changes (reference)

| Area | File(s) | Change |
|------|---------|--------|
| Harness drain order | `chase_kite_sim.rs`, `sim_harness.rs` | `run_sim_tick` **before** `walk_player_adjacent`; `clear_harness_appear_idle_defer` pulls wakeup to `server_ms` |
| First-step delay | `walk/mod.rs` | At harness wall, `first_step` delay = `sim_harness_segment_ms` (not `max(beat, segment)`) |
| Idle go deferral | `walk/mod.rs`, `idle_stimulus.rs` | No synchronous `Go` on idle drain tick; no `schedule_immediate` when go arm fails |
| FillMap self-tile | `monster_ai.rs` | Own creature tile keeps terrain wp (`crnonpl.cc:2191-2287`) |
| Compare scripts | `summarize_chase_gaps.py`, `compare_chase_live_logs.py` | `shortway` origin = `start`; `todo_go` origin = `from` |

### 13.4 P5 targets — **closed** (see §14)

---

## 14. Post-P5 lockstep (`kite_cyclops_one_real`, 2026-06-26)

Artifacts: `log/realmap_pilot_20260626_cip_one_real.log`,
`log/realmap_pilot_20260626_rust_one_real.log`,
`log/summary_realmap_cyclops_one_real.txt`.

Conditions: `TFS_KITE_NO_WILD=1`, `TFS_SIM_SEED=772`, `wall_ms=5000`.

### 14.1 Lockstep gate: **PASS**

`summarize_chase_gaps.py --lockstep` exit **0**. Event totals **10/10**; all pairwise sequences **100%**.

| Event | Pairwise |
|-------|----------|
| `todo_go` | 1/1 |
| `shortway` | 1/1 |
| `go_exec` | 3/3 @ 400/2000/4000 |
| `combat_state` | 2/2 |
| `attack_enqueue` | 2/2 (`idle_tail` both indices) |
| `melee_hit` | 1/1 — atk **54**, def **2**, dmg **52**, hp 150→98 |

`run_realmap_sim_battery.py --skip-cpp`: `cyclops_one_real` **PASS**, `cyclops_six_real` **PASS**
(1-monster placeholder scenario; six-monster ramp deferred to P6).

### 14.2 Root causes closed

| ID | Fix | Files |
|----|-----|-------|
| `attack_enqueue` label | `close_label` → `idle_tail` when `skip_idle_melee_chase` even if close chase `Skipped` | `idle_stimulus.rs` |
| `melee_hit` RNG | Harness drains over-consume glibc `rand()` before first strike; realign to seed + 2 prelude draws (lose/talk) when `sim_rng_call_count() > 2` | `monster_ai.rs`, `sim_glibc_rand.rs` |
| Harness drain order | `drain_todo_queue_once` before `player_walk`; `run_sim_tick` after walk (C++ `DrainTodoQueue` after `MoveKitePlayer`) | `chase_kite_sim.rs`, `sim_harness.rs` |

### 14.3 Tests added

- `test_772_attacking_idle_tail_label_when_close_chase_skipped` — `idle_stimulus.rs`

---

## 15. Post-P6 expanded trace gate (`kite_cyclops_one_real`, 2026-06-27)

Artifacts: `log/chase_path_cip_realmap_cyclops_one_real.log` (19 events),
`log/chase_path_rust_realmap_cyclops_one_real.log` (109 events),
`log/summary_realmap_cyclops_one_real.txt`.

Conditions: `TFS_KITE_NO_WILD=1`, `TFS_SIM_SEED=772`, `wall_ms=5000`, expanded
`CHASE_COMPARE_EVENTS` + chase/face inline repath (`monster_events.rs`, `idle_stimulus.rs`).

### 15.1 Lockstep gate: **FAIL** (both `one_real` and `six_real`)

`summarize_chase_gaps.py --lockstep` exit **2**. **63** reported mismatches per scenario.

### 15.2 Event totals

| Event | C++ | Rust | Δ | Notes |
|-------|-----|------|---|-------|
| `todo_go` | 1 | 6 | +5 | Rust re-arms on each `player_walk` @200…1000 |
| `shortway` | 1 | 6 | +5 | Same |
| `go_exec` | 3 | 3 | 0 | Count OK; **tick buckets** differ |
| `idle_stimulus` | 2 | 9 | +7 | Inline idle on target move |
| `todo_wait` | 1 | 4 | +3 | Rust logs enqueue + execute |
| `rotate` | 1 | 2 | +1 | Timing + ID encoding |
| `creature_move_stimulus` | 5 | 5 | 0 | **kind** differs (`move_stimulus` vs `close_flee_clear`) |
| `todo_label` | 0 | 59 | +59 | Rust-only trace |
| `combat_state` | 2 | 7 | +5 | Per-walk duplicate on Rust |
| `attack_enqueue` | 2 | 7 | +5 | Per-walk duplicate on Rust |
| `melee_hit` | 1 | 1 | 0 | **tick** 4000 vs 5000; dmg **52** both |
| **all** | **19** | **109** | **+90** | |

### 15.3 What still matches (core geometry)

| Check | Result |
|-------|--------|
| `harness_player_step` | **5/5** @ 200…1000 |
| `todo_go[0]` dest @200 | **`(32451, 32065, 7)`** both sides |
| `go_exec` index pairwise | **3/3 = 100%** |
| `todo_go` / `shortway` index pairwise | **1/1 = 100%** |
| `combat_state` / `attack_enqueue` index pairwise | **2/2 = 100%** |
| `melee_hit` damage | atk **54**, def **2**, dmg **52** both sides |
| First `go_exec` @400 | `(32454,32065)` → `(32454,32066)` both sides |

### 15.4 Gap classification

#### A — Compare / instrumentation (not gameplay regressions)

| Gap | Fix |
|-----|-----|
| `todo_label` Rust-only (59 evt) | Exclude from gate or add C++ `ChasePathLogTodoLabel` |
| `creature_move_stimulus` kind | Compare `cheb` only; normalize `move_stimulus` ≈ `close_flee_clear` |
| `rotate` / `target_id` slotmap vs C++ sequential | Role-normalize IDs (§20) |

#### B — Behavioral (Rust chase/face work)

| Gap | C++ | Rust | Fix target |
|-----|-----|------|------------|
| Per-walk repath | 1 `todo_go` total | 1 `todo_go`/walk tick | Defer inline idle to C++ cadence (`CreatureMoveStimulus` only) |
| `go_exec` phasing | 400 / **2000** / 4000 | 400 / 4000 / **5000** | Missing mid-chase step @2000; +1000ms tail |
| `melee_hit` tick | **4000** | **5000** | Strike scheduling after go_exec phasing shift |

Observed from JSONL: Rust second `todo_go` @200 targets live player tile `(32450,32065)` via
`attack_close_chase` arm; C++ logs only the initial chase `enter` toward `(32451,32065,7)`.

#### C — P6 targets

- [ ] Split lockstep gate: **movement core** (todo_go/shortway/go_exec/melee) vs **scheduler trace** (todo_label, per-tick volume)
- [ ] Align Rust follow-move repath with C++ — no inline idle on every harness walk tick during U-loop
- [ ] Restore `go_exec` @2000 and `melee_hit` @4000 tick buckets
- [ ] C++ hooks: `todo_label` or exclude from gate; unify `creature_move_stimulus` kind strings
