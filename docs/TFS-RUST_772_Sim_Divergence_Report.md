# TFS-RUST 772 — Simulation Divergence Report (Rust vs C++)

**Date:** 2026-06-14 (last updated §18 gap closeout)  
**Scope:** Movement parity + combat E0–E3 (implemented), via headless kite harness  
**Scenarios run:** `kite_rat_stand_melee.scenario`, `kite_rat_melee.scenario`, `kite_cyclops_quad_chase.scenario`  
**Seed:** `TFS_SIM_SEED=772` (default in `run_kite_scenario.py`)  
**Arena:** `--synthetic` / `arena_synthetic 1` (uniform wp=150 grass, both stacks)  
**Prerequisite:** query manager on `127.0.0.1:7173` (`scripts/tibia_game_dev.sh run-qm`)  
**Logs (2026-06-14 §16 rerun):**

| Scenario | C++ (`log/chase_path_cip_*.log`) | Rust (`log/chase_path_rust_*.log`) |
|----------|----------------------------------|-------------------------------------|
| Stand melee | `_stand` — 13 rat-filtered events (45 lines raw) | `_stand` — 17 events (17 lines) |
| Kite melee | `_kite` — **0 events** (no `chase_ai.jsonl`; C++ time budget) | `_kite` — 4 events (appear drain only) |
| Cyclops quad | `_cyclops` — 20 events (4 monsters) | `_cyclops` — 30 events (4 monsters) |

**Re-run (§16 — QM required, synthetic arena):**

```bash
# terminal 1
scripts/tibia_game_dev.sh run-qm

# terminal 2 — all three scenarios
TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --synthetic scripts/scenarios/kite_rat_stand_melee.scenario
cp log/chase_path_cip.log log/chase_path_cip_stand.log && cp log/chase_path_rust.log log/chase_path_rust_stand.log

TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --synthetic scripts/scenarios/kite_rat_melee.scenario
cp log/chase_path_cip.log log/chase_path_cip_kite.log && cp log/chase_path_rust.log log/chase_path_rust_kite.log

TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --synthetic scripts/scenarios/kite_cyclops_quad_chase.scenario
cp log/chase_path_cip.log log/chase_path_cip_cyclops.log && cp log/chase_path_rust.log log/chase_path_rust_cyclops.log
```

Summaries saved under `log/summary_{stand,kite,cyclops}.txt`. `run_kite_scenario.py` checks QM before C++ and auto-detects `--monster` from the scenario file.

**Historical logs (§13 baseline):**

| Scenario | C++ (§13 baseline) | Rust (P2) |
|----------|-------------------|-----------|
| Stand melee | `log/chase_path_cip_stand.log` (680 rat events) | `log/chase_path_rust_stand.log` (2,048 rat events) |
| Kite melee | `log/chase_path_cip_kite.log` (212 rat events) | `log/chase_path_rust_kite.log` (601 rat events) |

`run_kite_scenario.py` truncates `reference/cipsoft-772/runtime/log/chase_ai.jsonl` before each C++ run and passes `TFS_SIM_SEED` to both stacks.

> **Historical sections:** §3–§9 and §5 root-cause taxonomy describe the **pre-fix** baseline. §12–§14 are the authoritative post-fix timeline.

---

## 1. Executive summary

Both stacks emit a shared JSONL trace (`branch` → `todo_go` → `shortway` → `go_exec` + `combat_state` / `attack_enqueue` / `melee_hit`).

### Current status (§18 — synthetic, seed 772)

**Parity scorecard (see §18.7):** harness RNG isolation **~95%**; stand trace **~50–55%**; cyclops AI arms **~85%**; lockstep gate **0% pass**.

| Finding | Stand rat | Kite rat | Cyclops quad |
|---------|-----------|----------|--------------|
| Lockstep gate | **FAIL** | **FAIL** | **FAIL** |
| C++ wild/map monsters | **closed** (purge + spawn skip) | closed | closed |
| C++ JSONL on kite | n/a | **closed** (`wall_ms=6000`) | n/a |
| Pairwise movement | dance dest N vs S | chase tick cadence | `todo_go` **4/4**; `shortway`/`go_exec` **0/4** |
| Combat trace | `combat_state` 1/1; `melee_hit` timing differs | comparable | `combat_state` **4/4 (100%)** |

**Bottom line:** §17.6 harness gaps largely closed. Remaining lockstep blockers: first-idle RNG re-seed alignment (stand), chase tick cadence (kite/cyclops), C++ synthetic overlay still leaks native `min_wp=120` into pathfinder viewport.

### Current status (§17 — synthetic, seed 772)

**Parity scorecard (see §17.7):** harness/ToDo **~85–90%**; §16 idle preamble **~90–95%**; stand lockstep trace **~45–55%**; lockstep gate **0% pass** (still FAIL on all three scenarios).

| Finding | Stand rat | Kite rat | Cyclops quad |
|---------|-----------|----------|--------------|
| Lockstep gate | **FAIL** | **FAIL** | **FAIL** |
| tick=0 appear dance | **closed** (§17) | Rust-only tick=0 gone | **closed** (§17) |
| Pairwise movement | `branch`/`todo_go`/`go_exec` **0%** on first paired events | n/a (C++ silent) | `todo_go` **100%**; `shortway`/`go_exec` **0%** |
| Combat trace | `combat_state` 1/1 paired; `melee_hit` diverges after RNG desync | n/a | `combat_state` **4/4 (100%)** |

**Bottom line:** §16 structural blockers are closed (defer `ToDoYield`, LoseTarget draw, talk gate). Remaining gaps are **global RNG isolation** (C++ wild-map idles advance `rand()` before scenario monsters), **first-idle dance when `rand()%5` picks a blocked tile**, and **path geometry** (`min_wp` 120 vs 150 on cyclops). Kite rat remains **not comparable** until the scenario gets non-zero `advance_ms`.

### Current status (§16 rerun — synthetic, seed 772)

| Finding | §15 stand | §16 stand | §16 kite | §16 cyclops quad |
|---------|-----------|-----------|----------|------------------|
| Lockstep gate | **FAIL** | **FAIL** | **FAIL** | **FAIL** |
| C++ events (filtered) | 11 | 13 | **0** | 20 |
| Rust events (filtered) | 17 | 17 | 4 | 30 |
| `combat_state` pairwise | 1/1 | 1/1 | n/a | **4/4 (100%)** |
| `melee_hit` | 2 vs 3 | 2 vs 3 | 0 vs 0 | 0 vs 0 (no contact) |
| First movement diverge | tick=0 Rust dance | tick=2000 dest tile | C++ silent | tick=2000 path shape |

**Bottom line:** Synthetic arena + QM + multi-monster spawn work end-to-end. Stand combat still diverges on dance dest + hit damage once RNG stream desyncs. **Kite rat is not comparable on C++ today** — scenario `wall_ms=0` (all `advance_ms 0`) so C++ never schedules idle/attack todos and writes no JSONL. Cyclops quad proves 4-monster fan-out (`combat_state` 4/4) but movement paths diverge immediately (cheb chase vs appear-drain dance).

### Current status (P2 applied — §14)

| Finding | Pre-fix | After P0/P1 (§13) | After P2 (§14) |
|---------|---------|-------------------|----------------|
| Rust `melee_hit` all zeros | **Blocker** | Fixed (38–40 hits) | **Fixed** — 166 stand / 81 kite; rolls 2–7 |
| C++ event flood | 3,492–6,901 rat events | 680 stand / 212 kite | Same (C++ not re-run with P2 yet) |
| Tick alignment | C++ `tick=0`; Rust sub-beat | C++ `2000…`; Rust `0,40,80…` | **Both use ms clock** — combat at `2001,4001…` |
| Rust `combat_state` volume | Lower than C++ | Inverted (368 vs 227) | Deduped; still higher when dance logged |
| Combat hit count parity | 6–8× C++ excess | Stand 226 vs 40 | Stand 226 vs 166 (Rust runs longer per drain) |
| Pairwise movement parity | 8–25% | Poor (unseeded RNG) | Seeded (`TFS_SIM_SEED=772`); compare uses tick buckets |
| Scenario time budget | n/a | ~6 s intended | Rust drain reaches **tick ~102k** (player dies) |

**Bottom line:** Harness blockers are cleared; combat math works on both sides; ticks align at millisecond scale; RNG is seedable. Remaining gaps are **harness cadence** (Rust `DrainTodoQueue(64)` fast-forwards through many attack cycles per `sim_tick`), **movement logging** (Rust stand logs dance; C++ §13 stand did not), and **`shortway` path shape** at kite coords.

### Post-fix status (P0/P1 — §12–13, superseded by §14 for Rust metrics)

### Pre-fix findings (historical — for context)

| Finding | Severity | Category |
|---------|----------|----------|
| Rust `melee_hit` always `attack=0`, `damage=0` | ~~Blocker~~ Fixed | Harness |
| C++ emits ~5–10× more rat events | ~~High~~ Much reduced | Harness drain + log append |
| C++ `tick` always `0` | ~~High~~ Fixed | Instrumentation |
| `todo_go` `must`/`max` differ | **Medium** | Semantic |
| Movement dest diverges from event 0 | **Expected** | RNG dance |

---

## 2. Test setup

### 2.1 Stacks

| | Rust (`chase_kite_sim`) | C++ (`build/game chase-scenario`) |
|--|-------------------------|-------------------------------------|
| Map | OTBM `data/world/forgotten.otbm` + `objects.srv` overlay | `.sec` sectors `reference/cipsoft-772/runtime/map/` |
| Coords | `32360,32290` z=7 (shared) | Same |
| Debug | `TFS_CHASE_PATH_DEBUG=1` → `log/chase_path_rust.log` | `TIBIA_CHASE_PATH_DEBUG=1` → `log/chase_ai.jsonl` (copied to `log/chase_path_cip.log`) |
| Player | `sim_hero_player` (150 HP) | `TKiteSimPlayer` human race from `.mon` data (150 HP) |
| RNG seed | `TFS_SIM_SEED` → `GameWorld::ai_rng` | `TFS_SIM_SEED` → `srand()` in `chase_kite_scenario.cc` |
| Time advance | `move_creatures(ms)` mirrors C++ `MoveCreatures` | `MoveCreatures(ms)` in scenario steps |
| Drain | `run_sim_tick` → `DrainTodoQueue(64)` | `DrainTodoQueue(64)` per `sim_tick` / `monster_appear` / `player_pos` |

### 2.2 Scenarios

**`kite_rat_stand_melee`** — rat at `(32361,32290)`, player at `(32360,32290)`, no kite movement. Three `sim_tick` drains with `advance_ms 2000` between. Intended to exercise **E2 melee cadence** and **E3 ATTACKING** at cheb=1.

**`kite_rat_melee`** — same spawn, player kites east/north (`32362` → `32363` → `32363,32292`). Intended to exercise **melee_dance** while moving. **Note (§16):** current script has `wall_ms=0`; C++ emits no JSONL until non-zero `advance_ms` steps are added.

**`kite_cyclops_quad_chase`** — four cyclops at N/E/S/W of `(32360,32290)`, player kites NE. Exercises **multi-monster spawn**, shared `CreatureMoveStimulus`, and dist chase (`wall_ms=4000`).

### 2.3 Filter

All counts below filter JSONL lines where `name` contains `rat` (matches C++ `"a rat"` and Rust `"Rat"`).

---

## 3. Scenario A — `kite_rat_stand_melee` (combat focus)

### 3.1 Event volume

| Event | C++ ref | Rust | Δ | Notes |
|-------|--------:|-----:|--:|-------|
| **Total (rat)** | 3,492 | 659 | +2,833 | C++ ~5.3× more |
| `branch` | 30 | 44 | +14 | Both 100% `melee_dance` |
| `todo_go` | 65 | 44 | −21 | C++ more path enqueues |
| `shortway` | 5 | 0 | −5 | Rust adjacent dance = single-step, no TShortway log |
| `go_exec` | 37 | 44 | +7 | |
| `combat_state` | 1,678 | 264 | −1,414 | C++ logs **twice per idle** (`none` then `close`) |
| `attack_enqueue` | 839 | 132 | −707 | ~1:1 with melee_hit on C++ |
| `melee_hit` | 838 | 131 | −707 | |

### 3.2 Pairwise match rates (index-aligned)

| Stage | Matched | Rate |
|-------|--------:|-----:|
| `branch` | 5/30 | **16.7%** |
| `todo_go` | 0/44 | **0%** |
| `shortway` | 0/0 | n/a (Rust has none) |
| `go_exec` | 3/37 | **8.1%** |
| `combat_state` | 2/264 prefix then diverge | **<1%** effective |
| `melee_hit` | 0/131 damage match | **0%** |

### 3.3 First divergence (where sequences split)

**`branch[3]`** (after three matching dance dests at spawn):

- C++: `melee_dance` → dest `(32361,32290)`
- Rust: `melee_dance` → dest `(32360,32291)`

Same arm, different lateral tile — `rand()%5` dance pick (`crnonpl.cc:2739`, `monster_idle_dance_step`).

**`todo_go[0]`**:

- C++: `via=enter`, dest `(32361,32291)`, `must=1`, `max=2147483647`
- Rust: `via=single`, dest `(32360,32290)`, `must=0`, `max=3`, `arm=idle_dance`

This is **not** a pure naming mismatch: C++ dance uses `ToDoGo(must:true, INT_MAX)`; Rust idle dance enqueues a single paced step with `must=false`, `max=3`.

**`go_exec[3]`**:

- C++: `(32361,32291)→(32361,32290)`
- Rust: `(32361,32291)→(32360,32291)`

**`melee_hit[2]+`**:

- C++: `attack` 2–6, `damage` 0–5, `hp_before` varies (150 down to low values)
- Rust: **every** hit: `attack=0`, `defense=0`, `armor=0`, `damage=0`, `hp_before=100`, `hp_after=100`

### 3.4 Combat deep stats (stand)

| Metric | C++ | Rust |
|--------|-----|------|
| `melee_hit` count | 838 | 131 |
| Nonzero damage hits | **635** (76%) | **0** (0%) |
| Top attack rolls | 4 (192×), 3 (145×), 5 (143×), 2 (119×) | 0 (131×) |
| `hp_before` values | 27 distinct (150, 143, 137, … 0) | **{100} only** |
| `earliest_attack_ms` range | 2200 → 1,677,455 | 2001 → 262,002 |
| `attack_enqueue wait_ms` | 0 (839×) | 0 (132×) |
| Unique `tick` in log | **1** (all tick=0) | **4** (0, 2000, 4000, 6000) |
| Diagonal `go_exec` | 0/37 | 0/44 |

### 3.5 What stand scenario tells us

**Aligned (qualitatively):**

- Both enter `melee_dance` only (no `melee_chase` at cheb=1 — correct for E3).
- Both emit `combat_state` → `attack_enqueue` → `melee_hit` chains.
- C++ attack cadence bumps `earliest_attack_ms` by ~2000 ms per strike (E2 shape present).
- Rust also advances `earliest_attack_ms` (~2001 ms steps) even when damage is zero.
- Zero diagonal steps in both (cardinal dance/kite only).

**Divergent (action required):**

1. **Rust deals no damage** — harness sets `monster_melee_skill=15` but **not** `melee_attack=7` (rat TVP stats). `weapon_damage(skill, attack=0)` → `attack=0` in log.
2. **C++ event flood** — `DrainTodoQueue(256)` per `sim_tick`/`player_pos` step re-enters idle hundreds of times; `combat_state` hook fires at `SetChaseMode(NONE)` and again at `SetChaseMode(CLOSE)` → **2× state lines per cycle**.
3. **C++ tick field useless** — all events at `tick:0`; compare cannot time-align.
4. **Player stats differ** — C++ Hero 150 HP with human defense; Rust test player 100 HP, likely zero effective defense in snapshot.
5. **Rust no `shortway`** in stand — expected when dance is Manhattan-1 single step; C++ still logs 5 shortway (from chase repaths during drain).

---

## 4. Scenario B — `kite_rat_melee` (movement + combat)

### 4.1 Event volume (rat-filtered)

| Event | C++ ref | Rust | Δ |
|-------|--------:|-----:|--:|
| **Total (rat)** | 6,901 | 486 | +6,415 |
| `branch` | 74 | 33 | −41 |
| `todo_go` | 154 | 35 | −119 |
| `shortway` | 6 | 2 | −4 |
| `go_exec` | 83 | 37 | −46 |
| `combat_state` | 3,294 | 191 | −3,103 |
| `attack_enqueue` | 1,648 | 95 | −1,553 |
| `melee_hit` | 1,642 | 93 | −1,549 |

**Full C++ log scale:** 40,790 JSONL lines total — only **6,901** (17%) are the scenario rat (`"a rat"`). The rest are other map creatures whose AI runs during `DrainTodoQueue` while the harness advances time. Rust logs **only** the spawned rat + player (486 lines).

**Note:** C++ has *more* movement events than Rust (83 vs 37 `go_exec`) because each `player_pos` kite step triggers `DrainTodoQueue(256)`; Rust `sim_tick` drains one creature-todo pass per call.

### 4.2 Pairwise match rates

| Stage | Matched | Rate |
|-------|--------:|-----:|
| `branch` | 7/33 | **21.2%** |
| `todo_go` | 0/35 | **0%** |
| `shortway` | 0/2 | **0%** |
| `go_exec` | 5/37 | **13.5%** |

### 4.3 First divergence

**`branch[5]`** (first five match spawn dance pattern):

- C++: dest `(32361,32290)`
- Rust: dest `(32362,32291)` — player has kited; rat dances toward new geometry.

**`shortway[0]`** (first path query after kite):

- C++: dest `(32363,32290)`, steps `[(32362,32290)]`, `min_wp=120`, ok=true
- Rust: dest `(32363,32292)`, steps `[(32363,32290),(32363,32291)]`, `min_wp=120`, ok=true

Same `min_wp` (120) in this run — OTBM overlay matched `.sec` at this coord. **Path shape differs**: 1-step vs 2-step to different dest (player at `32363,32292`).

**`go_exec[5]`** onward: rat chase positions diverge — C++ oscillates around `(32361-32363,32289-32293)`; Rust pushes east `(32362→32364,32291)`.

### 4.4 Combat during kite

| Metric | C++ | Rust |
|--------|-----|------|
| `melee_hit` count | 1,642 | 93 |
| Nonzero `melee_hit` | **1,112/1,642 (68%)** | **0/93 (0%)** |
| Top C++ attack rolls | 4 (374×), 3 (292×), 5 (269×) | 0 (93×) |
| `hp_before` distinct values | 88 (150 down to 0) | **{100} only** |
| `todo_go.arm=attack_close_chase` | 0 (no arm field) | **1** (Rust tags close chase once) |
| Diagonal `go_exec` | 0/83 | 0/37 |

Rust logged one `attack_close_chase` arm during kite (player left melee band briefly); C++ close walk goes through `CanToDoAttack` without arm annotation. Neither side fired `melee_chase` branch events — E3 gating appears to work (ATTACKING skips idle chase).

---

## 5. Divergence taxonomy (root causes)

### 5.1 Harness / instrumentation (fix before blaming E2/E3 code)

#### H1 — Rust spawn missing `melee_attack` (BLOCKER)

`chase_kite_sim.rs` sets:

```rust
config.melee_skill = scenario.monster_melee_skill;  // 15 from scenario
// melee_attack NOT set — defaults to 0
```

Rat TVP stats (E0): `skill=15`, `attack=7`. C++ loads full race from `.mon`. Rust probe roll with `attack=0` yields **`melee_hit.attack=0`, `damage=0` always**.

**Fix:** Add `monster_melee_attack` scenario verb (default 7 for rat) or load from `MonsterDatabase` by label.

#### H2 — C++ todo drain volume (HIGH)

`chase_kite_scenario.cc` `DrainTodoQueue(256)` runs up to 256 `MoveCreatures` iterations **per** `sim_tick`, `monster_appear`, and `player_pos`. A single stand scenario produces **839 attack enqueues** vs Rust **132**.

Rust `run_sim_tick` drains one creature-todo pass per call.

**Fix:** Cap drain to scenario-equivalent work (e.g. match Rust tick budget, or drain until one monster idle cycle completes).

#### H3 — C++ `combat_state` double logging (MEDIUM)

Hooks at `crnonpl.cc:2709` (`chase_mode=none`) and `:2726` (`chase_mode=close`) fire **every idle pass** → 2× `combat_state` per cycle. Explains ref 1678 vs rust 264 (ratio ~6.4× before other factors).

**Fix:** Log only on **transition** or once per idle tail.

#### H4 — C++ `tick` field (MEDIUM)

`ChasePathLog*` uses `GetRoundNr()` which stays **0** in `chase-scenario` mode. Rust uses `tick_counter` / `server_ms` (0, 2000, 4000… in stand).

**Fix:** Log `ServerMilliseconds` as `tick` in C++ debug.

#### H5 — Player stat mismatch (MEDIUM)

| | C++ Hero | Rust `test_player` |
|--|----------|---------------------|
| HP | 150 | 100 |
| Defense | Human race (~4 in logs) | Likely 0 in `melee_defense_snapshot` |
| Name | `"Hero"` | `"Hero"` |

Damage parity cannot be assessed until Rust player carries era-correct defense.

#### H6 — C++ log file ambient noise (HIGH)

| Run | Total JSONL lines | Rat-filtered events | Rat % |
|-----|------------------:|--------------------:|------:|
| Stand | 17,516 | 3,492 | 20% |
| Kite | 40,790 | 6,901 | 17% |

Other monsters on the loaded `.sec` map run AI during every `DrainTodoQueue` call. Compare scripts filter by `rat`, but **volume ratios** (combat_state 3294 vs 191) still reflect C++ over-sampling, not just ambient creatures.

#### H7 — C++ runtime log not truncated by orchestrator (MEDIUM)

`run_kite_scenario.py` deletes `log/chase_path_cip.log` in the repo but **does not** clear `reference/cipsoft-772/runtime/log/chase_ai.jsonl` before invoking `game chase-scenario`. Back-to-back runs without manual deletion **append** scenarios.

**Fix:** `unlink(runtime/log/chase_ai.jsonl)` in `run_kite_scenario.py` before C++ run; optionally log creature id and filter compare by id.

---

### 5.2 Semantic / behavioral (real parity work)

#### S1 — Dance RNG (EXPECTED)

`melee_dance` uses `rand()%5` cardinal + no-step. First divergence at `branch[3]` with **matching** first three dests proves spawn alignment; divergence after is RNG unless seeded.

#### S2 — `todo_go` must/max contract (MEDIUM)

| Path | C++ | Rust |
|------|-----|------|
| Dance `ToDoGo` | `must=1`, `max=INT_MAX` | `must=0`, `max=3` via idle enqueue |
| Attack close chase | `must=0`, `max=3` (via `CanToDoAttack`) | `must=0`, `max=3`, `arm=attack_close_chase` |

Rust aligned `via` to `enter`/`single` but **must/max** still differ on dance path.

#### S3 — `shortway` presence (LOW–MEDIUM)

Stand: Rust 0, C++ 5. Rust single-step dance skips `log_shortway`; C++ still runs TShortway on some drain cycles.

Kite: Rust 1 multi-step, C++ 1 single-step to different dest — pathfinder agrees on `min_wp=120` but **steps differ** (map walkability at `32363,32290` vs `32363,32292`).

#### S4 — E3 `attack_close_chase` visibility (LOW)

Rust tags `todo_go.arm=attack_close_chase` once during kite. C++ close walk goes through `CanToDoAttack` inside `ToDoAttack` without arm annotation. Behavior may match; **log schema differs**.

#### S5 — No `melee_chase` branch in either run (GOOD)

At cheb=1 (stand) or during ATTACKING (kite), neither side logged idle `melee_chase` branches. Consistent with E3 gating intent.

---

### 5.3 Not yet in sim (E4–E6)

| Phase | Missing from trace |
|-------|-------------------|
| E4 | `spell_cast`, ranged hit events |
| E5 | `damage_stimulus`, PANIC transition |
| E6 | `death`, exp, loot |

---

## 6. Event schema alignment status

| Event | C++ emits? | Rust emits? | Comparable? |
|-------|:----------:|:-----------:|:-----------:|
| `branch` | ✓ | ✓ | Partial — dest RNG |
| `todo_go` | ✓ | ✓ | Partial — `via` aligned; `must`/`max`/`arm` differ |
| `shortway` | ✓ | ✓ | Partial — count + steps |
| `go_exec` | ✓ | ✓ | Partial — positions |
| `combat_state` | ✓ (transition) | ✓ (transition dedupe) | Partial — volume still differs when Rust logs dance |
| `attack_enqueue` | ✓ | ✓ | Partial — volume |
| `melee_hit` | ✓ | ✓ | **Comparable** — nonzero damage, rolls 2–7 |
| `parked` | — | ✓ | Rust only |

---

## 7. Recommended fix order (before E4)

### P0 — Unblock combat compare

1. **`chase_kite_sim`**: set `config.melee_attack` (scenario field, default 7 for rat), `config.defense`/`armor` from monster type if available.
2. **`test_player` / sim hero**: match 772 human HP (150) and defense skill for damage rolls.
3. **Verify** stand scenario: Rust `melee_hit` nonzero damage, ~2000 ms `earliest_attack_ms` steps.

### P1 — Make counts comparable

4. **C++ `DrainTodoQueue`**: cap per `sim_tick` to Rust-equivalent budget (e.g. 1–4 `MoveCreatures` rounds, not 256).
5. **C++ `combat_state`**: log on transition only.
6. **C++ `tick`**: use `ServerMilliseconds`.

### P2 — Tighten movement compare

7. ~~**Seed RNG**~~ in both harnesses (`TFS_SIM_SEED=772`) — done §14.
8. **Align dance `must/max`** on Rust idle dance to C++ `must:true, INT_MAX` (or document intentional diff) — open §14 P3.
9. **Cap scenario drain** to wall-clock window — open §14 P3.

### P3 — E4 prep

9. Add `spell_cast` event to schema before cobra scenario.

---

## 8. Raw compare tool output (stand)

```
Event totals (rat-filtered)
  branch         30     44
  todo_go        65     44
  shortway        5      0
  go_exec        37     44
  combat_state   1678   264
  attack_enqueue  839   132
  melee_hit       838   131
  all events: ref=3492  rust=659

Pairwise: branch 16.7% | todo_go 0% | shortway n/a | go_exec 8.1%
Diagonal go_exec: ref 0/37, rust 0/44

melee_hit nonzero: C++ 635/838 (76%), Rust 0/131 (0%)
C++ hp_before: 27 distinct values (150→0); Rust: {100} only
C++ ticks: all 0; Rust ticks: 0, 40, 80, 120 (beat sub-steps)
```

## 8.1 Sample mismatch excerpts (stand)

**`combat_state[2+]`** — alternating pattern on C++:

- C++: `('attacking', 'none')` then `('attacking', 'close')` every idle cycle (double log)
- Rust: `('attacking', 'close')` only (single log per cycle)

**`melee_hit[2]`** — first damage divergence:

- C++: `attack=5`, `defense=4`, `damage=1`, `hp_before=150`, `hp_after=149`
- Rust: `attack=0`, `defense=0`, `damage=0`, `hp_before=100`, `hp_after=100`

## 9. Raw compare tool output (kite)

```
Event totals (rat-filtered)
  branch         74     33
  todo_go       154     35
  shortway        6      2
  go_exec        83     37
  combat_state   3294   191
  attack_enqueue 1648    95
  melee_hit      1642    93
  all events: ref=6901  rust=486

Pairwise: branch 21.2% | todo_go 0% | shortway 0% | go_exec 13.5%
Diagonal go_exec: ref 0/83, rust 0/37

melee_hit nonzero: C++ 1112/1642 (68%), Rust 0/93 (0%)
```

## 9.1 Sample mismatch excerpts (kite, first divergence)

**`shortway[0]`** — same dest, different step shape:

- C++: dest `(32363,32290)`, steps `[(32362,32290)]`, `min_wp=120`
- Rust: dest `(32363,32290)`, steps `[(32362,32291),(32362,32290)]`, `min_wp=120`

**`shortway[1]`**:

- C++: `[(32363,32289),(32363,32290),(32363,32291)]`
- Rust: `[(32362,32291),(32363,32291)]`

**`go_exec[3]`** onward — rat position streams diverge after player kite; C++ oscillates in `32289–32294` band east of spawn; Rust pushes to `(32364,32291)`.

---

## 10. Conclusion

The simulation harness **successfully runs both stacks** on shared coordinates and captures the **full E0–E3 event chain** (movement + combat).

**Resolved (P0–P2):**

- Rust combat math exercised (`melee_attack=7`, hero 150 HP, real damage rolls).
- C++ tick field uses `ServerMilliseconds`; Rust trace uses `server_ms` via `chase_trace_tick()`.
- C++ `combat_state` no longer double-logs `none` then `close` every cycle.
- Deterministic compare path: `TFS_SIM_SEED=772` + tick-bucket pairing in `compare_chase_live_logs.py`.
- Rust harness mirrors C++ `MoveCreatures` / `DrainTodoQueue(64)`.

**Still open (P3):**

- **Full A/B rerun** with rebuilt C++ `game` + query manager (§13 C++ logs predate P2 `srand`).
- **Scenario time budget** — each `sim_tick` can fast-forward to tick ~100k+ within a scenario that only `advance_ms 6000` explicitly; cap drain or split compare windows.
- **C++ stand movement logging** — §13 C++ stand had zero `branch`/`go_exec`; Rust P2 stand logs 208 dance events.
- **`shortway` step shape** at kite coords — OTBM vs `.sec` tile audit.
- **Semantic:** dance `todo_go` `must/max` still differs (S2); optional alignment or document as intentional.

No confirmed E2/E3 logic bugs from harness-corrected traces; remaining diffs are cadence, logging visibility, map path shape, and RNG-dependent dance dests (now seedable).

---

## 11. Related docs

- [`TFS-RUST_772_Sim_Coverage_Matrix.md`](TFS-RUST_772_Sim_Coverage_Matrix.md) — which E0–E6 events the harness emits
- [`TFS-RUST_772_Monster_Combat_Integration_Plan.md`](TFS-RUST_772_Monster_Combat_Integration_Plan.md) — E0–E3 implementation status
- [`TIBIA_GAME_MASTER_DEV.md`](TIBIA_GAME_MASTER_DEV.md) — build/run commands for C++ harness

---

## 12. Fixes applied (2026-06-14 rerun)

### Rust (P0)

| Change | File |
|--------|------|
| `sim_hero_player` — 150 HP human hero | `sim_harness.rs` |
| `monster_melee_attack`, `monster_armor`, `monster_defense` scenario verbs + defaults (7/1/3) | `chase_kite_sim.rs`, `*.scenario` |
| Spawn sets `melee_attack`, `armor`, `defense` on `MonsterAiConfig` | `chase_kite_sim.rs` |

### C++ (P1)

| Change | File |
|--------|------|
| `DrainTodoQueue(256)` → `64` (match Rust `MAX_ROUNDS`) | `chase_kite_scenario.cc` |
| `tick` = `ServerMilliseconds` (was `GetRoundNr()`) | `chase_path_debug.cc` |
| Remove `combat_state` log after `CHASE_MODE_NONE` | `crnonpl.cc` |
| `ChasePathResetLog()` truncates JSONL at scenario start | `chase_path_debug.cc`, `chase_kite_scenario.cc` |
| Ignore Rust-only scenario verbs (`monster_melee_attack`, etc.) | `chase_kite_scenario.cc` |

### Orchestrator (P1)

| Change | File |
|--------|------|
| `unlink(runtime/log/chase_ai.jsonl)` before C++ run | `run_kite_scenario.py` |

### Rust + C++ + orchestrator (P2)

| Change | File |
|--------|------|
| Trace `tick` = `server_ms` (`chase_trace_tick()`) | `game_world.rs`, `monster_ai.rs`, `idle_stimulus.rs`, `creature_todo.rs`, `walk/mod.rs` |
| `move_creatures()` + C++-aligned `run_sim_tick` | `sim_harness.rs` |
| Scenario `advance_ms` uses `move_creatures` not `advance_beat_772` | `chase_kite_sim.rs` |
| `combat_state` transition dedupe (`Monster::last_combat_trace`) | `idle_stimulus.rs`, `creature/monster.rs` |
| Seedable `ai_rng` + `TFS_SIM_SEED` | `game_world.rs`, `monster_ai.rs` |
| C++ `srand(TFS_SIM_SEED)` | `chase_kite_scenario.cc` |
| Compare by `(evt, tick)` bucket | `compare_chase_live_logs.py` |
| Default `TFS_SIM_SEED=772` in orchestrator | `run_kite_scenario.py` |
| `path_compare` `stop_at_cheb=1` arg | `src/bin/path_compare.rs` |

---

## 13. Post-fix metrics (P0/P1 — rat-filtered)

> **Superseded for Rust:** §14 has P2 Rust metrics. C++ columns below are the §13 baseline (still valid until P2 A/B rerun).

### Stand melee

| Event | C++ | Rust | Δ |
|-------|----:|-----:|--:|
| **Total** | 680 | 748 | +68 |
| `combat_state` | 227 | 368 | +141 |
| `attack_enqueue` | 227 | 184 | −43 |
| `melee_hit` | 226 | 40 | −186 |
| `branch` / `go_exec` | **0** / **0** | 52 / 52 | C++ stand emits combat-only |

**Combat quality:**

| Metric | C++ | Rust |
|--------|-----|------|
| Nonzero `melee_hit` | 156/226 (69%) | **38/40 (95%)** |
| Attack roll range | 2–6 | 2–6 |
| `hp_before` | 150 → 0 (ambient bleed) | 150 → 50 |
| `tick` values | 2000, 4000, 6000… | 0, 40, 80, 120 (sub-beat) |

### Kite melee

| Event | C++ | Rust | Δ |
|-------|----:|-----:|--:|
| **Total** | 212 | 418 | +206 |
| `branch` | 4 | 28 | +24 |
| `go_exec` | 9 | 30 | +21 |
| `melee_hit` | 57 | 39 | −18 |
| `combat_state` | 61 | 215 | +154 |

**Combat quality:**

| Metric | C++ | Rust |
|--------|-----|------|
| Nonzero `melee_hit` | 27/57 (47%) | **38/39 (97%)** |
| Diagonal `go_exec` | 2/9 (22%) | 0/30 |

### Remaining work (P2 → moved to §14 P3)

See §14 **Remaining work (P3)** for current open items.

---

## 14. P2 closeout (2026-06-14)

### Rust P2 metrics (`TFS_SIM_SEED=772`, rat-filtered)

| Event | Stand | Kite |
|-------|------:|-----:|
| **Total** | 2,048 | 601 |
| `combat_state` | 546 | 216 |
| `attack_enqueue` | 712 | 150 |
| `melee_hit` | 166 | 81 |
| `branch` / `go_exec` | 208 / 208 | 49 / 52 |
| `shortway` | 0 | 2 |
| Nonzero `melee_hit` | **161/166 (97%)** | **79/81 (97%)** |
| Attack roll range | 2–7 | 2–7 |
| Combat tick sample | 2001, 4001, 6001… | 2001, 4001, 6001… |
| Move tick sample | 0, 1, 40, 80, 120 | 0, 1 |
| Max `tick` in log | **102,362** | **88,204** |

> **Post–wall-cap (harness v2):** stand Rust log **8 events**, `max_tick=6000`, `over_budget=0`. Kite Rust **5 events**, `max_tick=0` (scenario has no `advance_ms` budget — all steps are `advance_ms 0`).

**Combat quality (stand samples — pre–wall-cap baseline):**

```
tick=0    attack=6 defense=0 damage=6  hp 150→144
tick=0    attack=5 defense=0 damage=5  hp 144→139
tick=0    attack=2 defense=0 damage=2  hp 139→137
…
tick=82002 attack=7 damage=7 hp 1→0     (hero killed)
```

Hero starts at 150 HP; player dies during drain — scenario runs far beyond the intended ~6 s wall time (see **Scenario time budget** below).

### P2 Rust vs §13 C++ baseline (C++ not re-run with P2 `srand` yet)

| Metric | Stand C++ (§13) | Stand Rust (P2) | Kite C++ (§13) | Kite Rust (P2) |
|--------|----------------:|------------------:|---------------:|---------------:|
| Total events | 680 | 2,048 | 212 | 601 |
| `melee_hit` | 226 | 166 | 57 | 81 |
| `branch`/`go_exec` | **0 / 0** | 208 / 208 | 4 / 9 | 49 / 52 |
| Nonzero damage | 69% | **97%** | 47% | **97%** |
| Combat ticks | 2000, 4000… | 2001, 4001… | 2000, 4000… | 2001, 4001… |

### Tick-bucket compare (§13 C++ stand vs P2 Rust stand)

`compare_chase_live_logs.py` with tick-bucket pairing (no index cascade):

```
Reference: 680 events  (combat-only — zero branch/go_exec)
Rust:      2048 events (combat + 208 dance movement events)
171 mismatches — predominantly "branch tick=X count ref=0 rust=N"
```

Interpretation: C++ §13 stand logged **no movement** during combat; Rust P2 stand logs idle **melee_dance** (`branch`/`todo_go`/`go_exec`). This is a harness logging / drain visibility gap, not necessarily an E3 logic divergence.

### Scenario time budget (harness v2 — fixed)

The stand scenario script only advances **6,000 ms** explicitly (`advance_ms 2000` × 3). Pre–wall-cap, each `sim_tick` called `DrainTodoQueue(64)` which could fast-forward through many 2000 ms attack cycles — Rust logs reached `tick ≈ 102,362` and killed the hero.

**Harness v2 fixes (Rust + C++ mirror):**

| Mechanism | Behavior |
|-----------|----------|
| `SimClock` / `g_ScenarioWallMs` | Cumulative `advance_ms` budget caps drain fast-forward |
| `move_creatures` vs `move_creatures_explicit` | Scenario `advance_ms` always applies full delay; drain rounds clamp via `harness_clamp_delay` |
| `run_sim_tick` | Drains due todos at current `server_ms` even when at wall; only blocks *future* fast-forward |
| `chase_path_reset_log` | Truncate JSONL each run (C++ `ChasePathResetLog`) |
| `--max-tick` compare filter | `run_kite_scenario.py` auto-passes scenario wall to compare |

**Post–wall-cap stand (`TFS_SIM_SEED=772`):** 8 events — `combat_state=1`, `attack_enqueue=4`, `melee_hit=3`, `max_tick=6000`.

**Implication:** §13 C++ event counts (680 stand) were measured with uncapped drain; compare should use `--max-tick 6000` or re-run C++ with harness v2 wall cap for apples-to-apples counts.

### Remaining work (P3)

| # | Item | Status |
|---|------|--------|
| 1 | Full A/B rerun with QM + rebuilt C++ `game` (includes `srand` + wall cap) | **Done** (2026-06-14) |
| 2 | Cap `sim_tick` drain to scenario wall time | **Done** (harness v2) |
| 3 | Stand melee dance logging parity | **Partial** — Rust now logs dance; dest/tick differ (RNG stream + map) |
| 4 | `shortway` step shape — OTBM vs `.sec` at kite tiles | **Partial** — kite lab walkability test + `kite_lab_shortway.scenario` |
| 5 | Dance `todo_go` `must/max` alignment (S2) | **Done** — `must=1`, `max=INT_MAX`, dest from walk queue |
| 6 | E4 prep — `spell_cast` event before cobra scenario | **Done** — `chase_debug::log_spell_cast` |

### P3 closeout (2026-06-14)

**Stand A/B (`TFS_SIM_SEED=772`, `--max-tick 6000`):**

| Metric | C++ (rebuilt) | Rust (P3) |
|--------|--------------:|----------:|
| Total events | 16 | 17 |
| `branch` / `go_exec` | 2 / 2 | 3 / 3 |
| `melee_hit` | 2 | 3 |
| `max_tick` (filtered) | ≤6000 | ≤6000 |

C++ wall cap confirmed after rebuild (was 100k+ ticks pre-rebuild). Rust stand now emits melee_dance (`branch`/`todo_go`/`go_exec`) with C++-aligned `must/max`. Remaining compare diffs: dance **dest** tiles (OTBM vs `.sec` sidestep walkability), Rust `via=single` vs C++ duplicate `enter`+`single` logs, global `rand()` consumption order (dance uses glibc `rand()` when seeded; other rolls may still diverge).

**P3 code changes:**

| Change | File(s) |
|--------|---------|
| glibc `rand()` for seeded dance (`TFS_SIM_SEED`) | `sim_glibc_rand.rs`, `game_world.rs`, `monster_ai.rs` |
| C++ dance dir order W,E,S,N,hold | `sim_glibc_rand.rs` `DANCE_DIR_ORDER` |
| Dance `todo_go` `must=1` / `max=INT_MAX` / sidestep dest | `creature_todo.rs` |
| `spell_cast` JSONL event (E4 prep) | `chase_debug.rs`, `idle_stimulus.rs` |
| Kite lab OTBM walkability test | `sim_harness.rs` |
| Path probe scenario | `scripts/scenarios/kite_lab_shortway.scenario` |

### P2 fixes (reference)

| Change | File(s) |
|--------|---------|
| Trace `tick` = `server_ms` | `game_world.rs`, all `chase_debug::log_*` call sites |
| `move_creatures` + C++-aligned `run_sim_tick` | `sim_harness.rs`, `chase_kite_sim.rs` |
| `combat_state` transition dedupe | `idle_stimulus.rs`, `creature/monster.rs` |
| Seedable `ai_rng` (`TFS_SIM_SEED`) | `game_world.rs`, `monster_ai.rs`, `run_kite_scenario.py` |
| C++ `srand(TFS_SIM_SEED)` | `reference/.../chase_kite_scenario.cc` |
| Compare by `(evt, tick)` bucket | `scripts/compare_chase_live_logs.py` |
| `--max-tick` compare filter + scenario wall auto | `compare_chase_live_logs.py`, `run_kite_scenario.py` |
| Harness wall cap + log reset | `sim_harness.rs`, `chase_kite_sim.rs`, `chase_debug.rs`, `chase_kite_scenario.cc` |
| `path_compare` `stop_at_cheb` arg | `src/bin/path_compare.rs` |

Re-run full A/B:

```bash
TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py scripts/scenarios/kite_rat_stand_melee.scenario
cp log/chase_path_cip.log log/chase_path_cip_stand.log && cp log/chase_path_rust.log log/chase_path_rust_stand.log
TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py scripts/scenarios/kite_rat_melee.scenario
cp log/chase_path_cip.log log/chase_path_cip_kite.log && cp log/chase_path_rust.log log/chase_path_rust_kite.log
```

---

## 15. Lockstep closeout (2026-06-14)

Shared synthetic arena + glibc RNG lockstep plumbing + compare gate for E0–E3.

### Infrastructure shipped

| Area | Change | File(s) |
|------|--------|---------|
| Synthetic arena (Rust) | Pin `default_wp` on ground item + `min_wp` test | `sim_harness.rs`, `chase_kite_sim.rs` |
| Synthetic arena (C++) | `arena_synthetic` verb / `TFS_KITE_SYNTHETIC_ARENA`; lay grass TypeID 102 (wp=150) | `chase_kite_scenario.cc` |
| Orchestrator | `--synthetic` passes env to both stacks; `--lockstep` gate via `summarize_chase_gaps.py` | `run_kite_scenario.py` |
| RNG lockstep | glibc `random()`/`ProbeValue`/`GetArmorStrength`/poison + idle `parity_random` | `sim_glibc_rand.rs`, `combat/math.rs`, `idle_stimulus.rs`, `monster_combat.rs` |
| RNG debug | `rng_trace` JSONL event (`TFS_SIM_RNG_TRACE=1`) | `chase_debug.rs`, `sim_glibc_rand.rs` |
| Compare | Collapse C++ `enter`+`single` `todo_go`; combat match rates in gap summary | `compare_chase_live_logs.py`, `summarize_chase_gaps.py` |
| Scenarios | `arena_synthetic 1` on stand + kite rat scenarios | `kite_rat_*_melee.scenario` |

### A/B rerun (`TFS_SIM_SEED=772`, `--synthetic`, `--max-tick 6000` stand)

| Metric | C++ | Rust |
|--------|----:|-----:|
| `branch` | 1 | 3 |
| `todo_go` | 1 | 3 |
| `go_exec` | 1 | 3 |
| `combat_state` | 3 | 1 |
| `melee_hit` | 2 | 3 |

**Interpretation:** Map-source asymmetry is removed (uniform wp=150 grass). Remaining stand diffs are **RNG draw-order before first logged dance** (C++ pre-idle `rand()` e.g. talk gate) and **Rust logging dance on `monster_appear` drain at tick=0** while C++ defers first movement log to later ticks. Combat rolls now route through glibc but still diverge once the stream desyncs.

Kite synthetic (no explicit `advance_ms` wall): Rust-only tick=0 dance on appear; ref emits no movement — cadence visibility gap only.

### Re-run lockstep gate

```bash
TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --synthetic scripts/scenarios/kite_rat_stand_melee.scenario
python3 scripts/summarize_chase_gaps.py --ref log/chase_path_cip.log --rust log/chase_path_rust.log \
  --monster rat --max-tick 6000 --lockstep
```

### Remaining for full lockstep (post-P3)

1. Audit **pre-dance** glibc consumption in C++ spawn/idle preamble vs Rust (`TFS_SIM_RNG_TRACE=1` on both sides).
2. Align **appear-drain** movement logging cadence (suppress Rust tick=0 dance or log C++ equivalent).
3. C++ `melee_hit` logs `armor=0` always — compare should ignore armor field or align C++ log hook.

---

## 16. Multi-monster rerun (2026-06-14)

Full A/B with query manager, `TFS_SIM_SEED=772`, `--synthetic`. New scenario: **`kite_cyclops_quad_chase`** — 4 cyclops (N/E/S/W of player), player kites NE, `wall_ms=4000`.

### 16.1 Stand melee (`--max-tick 6000`)

| Event | C++ | Rust | Pairwise match |
|-------|----:|-----:|----------------|
| `branch` | 2 | 3 | 50% (1/2) |
| `todo_go` | 2 | 3 | 50% (1/2) |
| `go_exec` | 1 | 3 | 100% on ref prefix; count mismatch |
| `combat_state` | 3 | 1 | 100% (1/1 paired) |
| `attack_enqueue` | 3 | 4 | 0% (0/3) |
| `melee_hit` | 2 | 3 | 0% (0/2) — damage rolls differ |
| **Total** | **13** | **17** | lockstep **FAIL** |

**First divergence:** `branch[1]` — ref dances to `(32360,32291)`, rust to `(32361,32290)` at tick 6000 vs 4000 cadence. Rust still logs tick=0 appear-dance; C++ defers first `branch` to tick=2000.

### 16.2 Kite melee (`wall_ms=0`)

| Event | C++ | Rust |
|-------|----:|-----:|
| All filtered rat events | **0** | 4 |

C++ **`chase_ai.jsonl` not created** — no monster hook fired. Root cause: every script step uses `advance_ms 0`; `ServerMilliseconds` never advances, so scheduled attack/idle todos do not run on C++. Rust `run_sim_tick` still drains appear-time events at tick=0 (`branch`/`todo_go`/`combat_state`/`attack_enqueue`).

**Action:** add non-zero `advance_ms` between kite steps (or explicit wall budget) before expecting C++ kite parity. Until then, kite compare is Rust-only smoke.

### 16.3 Cyclops quad chase (`--max-tick 4000`, filter `cyclops`)

| Event | C++ | Rust | Notes |
|-------|----:|-----:|-------|
| `branch` | 0 | 1 | Rust tick=0 dance on one cyclops |
| `todo_go` | 4 | 6 | C++ `enter` chase; Rust `single` dance + chase mix |
| `shortway` | 4 | 5 | Path steps differ from event 0 |
| `go_exec` | 4 | 7 | ref 1/4 diagonal; rust 1/7 diagonal |
| `combat_state` | 4 | 4 | **4/4 pairwise match** — all 4 monsters ATTACKING |
| `attack_enqueue` | 4 | 7 | Rust extra enqueues from appear drain |
| `melee_hit` | 0 | 0 | Player out of melee range during kite |
| **Total** | **20** | **30** | lockstep **FAIL** |

**First divergence:** `todo_go[0]` — ref `enter` chase toward `(32360,32294)` with `must=0,max=3`; rust `single` dance sidestep on Cyclops 2 at tick=0. Multi-monster `CreatureMoveStimulus` fan-out works on both stacks; movement ordering and tick=0 Rust logging remain the gap.

**Sample go_exec at tick=4000:** four C++ cyclops each step toward player; Rust mix of chase + diagonal dance — paths not lockstep yet.

### 16.4 Lockstep gate summary

| Scenario | `--lockstep` | Primary blocker |
|----------|--------------|-----------------|
| Stand | FAIL | Dance dest + melee_hit damage + tick=0 Rust logs |
| Kite | FAIL | C++ zero events (no time advance) |
| Cyclops quad | FAIL | tick=0 appear drain + chase path shape |

```bash
python3 scripts/summarize_chase_gaps.py \
  --ref log/chase_path_cip_stand.log --rust log/chase_path_rust_stand.log \
  --monster rat --max-tick 6000 --lockstep

python3 scripts/summarize_chase_gaps.py \
  --ref log/chase_path_cip_cyclops.log --rust log/chase_path_rust_cyclops.log \
  --monster cyclops --max-tick 4000 --lockstep
```

---

## 17. §16 closeout — appear defer + RNG preamble (2026-06-14)

Shipped the three Rust-side fixes identified in §16.4. Re-ran synthetic A/B with `TFS_SIM_SEED=772` after rebuilding C++ (`scripts/tibia_game_dev.sh build`) so `monster_talks` is ignored on the reference side.

### 17.1 Fixes shipped

| Fix | C++ reference | Rust change |
|-----|---------------|-------------|
| **A — defer appear idle** | `ToDoYield` schedules `ToDoWait(0)` + wakeup; `IdleStimulus` on drain, not inline (`cract.cc:1001`) | `request_idle_stimulus` → `creature_todo_yield`; `run_monster_todo_execute` chains idle after `Wait{0}` |
| **B — LoseTarget always-draw** | `\|\|` evaluates `random(0,99)` even at 0% (`crnonpl.cc:2381`) | `monster_idle_772_should_lose_target`: draw when `master.is_none()` before compare |
| **C — talk gate** | `rand()%50` every idle; `random(1,Talks)` on hit (`crnonpl.cc:2392`) | `Monster.talks` + `monster_idle_try_talk`; `monster_talks` scenario verb; cyclops `talks=5` |

**Tests:** `rtk cargo test -p tfs-rust-core` — **331 passed**.

### 17.2 Stand melee — before/after (rat-filtered, `--max-tick 6000`)

| Metric | §16 (pre-fix) | §17 (post-fix) |
|--------|---------------|----------------|
| Rust events at **tick=0** | `branch`/`todo_go`/`combat_state` logged | **0** movement/combat JSONL lines |
| First Rust `branch` tick | 0 (appear drain) | **4000** (C++ still **2000**) |
| `branch` / `todo_go` count | ref 2 / rust 3 | ref 2 / rust **1** |
| `go_exec` count | ref 1 / rust 3 | ref 2 / rust **1** |
| `melee_hit` count | ref 2 / rust 3 | ref 2 / rust 3 |
| Lockstep gate | FAIL | **FAIL** (improved cadence; dest still off) |

**§16 blocker closed:** tick=0 appear-dance logging — Rust no longer runs `IdleStimulus` inline on `monster_appear`; first combat log is at tick **2000**.

**Remaining stand divergences:**

1. **First-idle ordering** — C++ tick 2000: `branch`/`todo_go` (dance north to `32291`) then `attack_enqueue`. Rust tick 2000: `melee_hit` only (attack executes in the same drain pass). Root cause: first `rand()%5` dance roll is **0 (West onto player tile)** → Rust `monster_idle_dance_step` rejects; C++ stream lands on a valid sidestep because the reference map still runs **wild creature idles** (snakes) that advance the global `rand()` before the rat’s dance draw.
2. **Dance dest** — first paired `branch[0]`: ref `(32361,32291)` vs rust `(32361,32289)` at tick 4000 vs 2000.
3. **`combat_state` volume** — ref 3 vs rust 1 (dedupe + fewer idle passes logged).

`TFS_SIM_RNG_TRACE=1` on Rust confirms call #1 = LoseTarget, call #2 = dance `%5` (fails west), calls #3–6 = first `melee_hit` probes — no `branch` until the second idle.

### 17.3 Cyclops quad — before/after (`--max-tick 4000`)

| Metric | §16 | §17 |
|--------|-----|-----|
| Rust tick=0 movement | 4× `todo_go`/`branch` on appear | **0** |
| `combat_state` pairwise | 4/4 (100%) | **4/4 (100%)** |
| `todo_go` pairwise | low | **4/4 (100%)** |
| `shortway` / `go_exec` | path shape mismatch | still **0/4** — ref `min_wp=120` (map `.sec`) vs rust `min_wp=150` (synthetic grass); step lists differ |
| Lockstep gate | FAIL | **FAIL** (cadence fixed; path parity open) |

Talk-gate RNG is wired (`monster_talks 5`); cyclops idle now consumes the `%50` draw before targeting when `talks > 0`.

### 17.4 Kite rat

Unchanged vs §16.2 — scenario `wall_ms=0`; C++ writes no JSONL until non-zero `advance_ms` is added (scenario edit, not Rust parity).

### 17.5 Lockstep gate summary (post-§16 fixes)

| Scenario | `--lockstep` | §16 primary blocker | §17 status |
|----------|--------------|---------------------|------------|
| Stand | **FAIL** | tick=0 appear dance | tick=0 **closed**; dance dest + first-idle attack ordering remain |
| Kite | **FAIL** | C++ zero events | unchanged (harness time budget) |
| Cyclops quad | **FAIL** | tick=0 + talk RNG | tick=0 **closed**; talk gate **shipped**; `shortway`/`go_exec` shape open |

```bash
# Reproduce §17 metrics (C++ from runtime cwd; rebuild game first)
scripts/tibia_game_dev.sh build
cd reference/cipsoft-772/runtime
TFS_SIM_SEED=772 TIBIA_CHASE_PATH_DEBUG=1 TFS_KITE_SYNTHETIC_ARENA=1 \
  ../tibia-game-master/build/game chase-scenario ../../../scripts/scenarios/kite_rat_stand_melee.scenario
cp log/chase_ai.jsonl ../../../log/chase_path_cip_stand.log

TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --skip-cpp --synthetic \
  scripts/scenarios/kite_rat_stand_melee.scenario
cp log/chase_path_rust.log log/chase_path_rust_stand.log

python3 scripts/summarize_chase_gaps.py \
  --ref log/chase_path_cip_stand.log --rust log/chase_path_rust_stand.log \
  --monster rat --max-tick 6000 --lockstep
```

### 17.6 Next steps (outside this closeout)

1. **Isolate global RNG in chase harness** — suppress or exclude wild-map creature idles on C++ synthetic runs so rat/cyclops dance draws align with Rust’s scenario-only world (or spawn equivalent noise on Rust).
2. **First-idle dance fallback** — confirm whether C++ retries other directions when `rand()%5` lands on a blocked tile; mirror if so.
3. **C++ `melee_hit` `armor=0`** — compare-side normalization (§16 out of scope).

### 17.7 Parity scorecard (post-§17)

Estimates are for the **headless chase harness** (`TFS_SIM_SEED=772`, `--synthetic`, lockstep compare). They are **not** full-client or full-shard parity percentages.

#### By layer

| Layer | Estimate | Rationale |
|-------|----------|-----------|
| Harness / clock / ToDo model | **~85–90%** | ms ticks, wall gate, `ToDoYield` defer, glibc seed path, JSONL event vocabulary — largely aligned |
| §16 idle preamble (plan items A–C) | **~90–95%** | tick=0 appear dance closed; LoseTarget always-draw; talk gate wired |
| Lockstep trace parity (stand rat) | **~45–55%** | Same event types and regime; first dance tile/tick and combat ordering still diverge |
| Lockstep gate (binary pass/fail) | **0%** | `--lockstep` still **FAIL** on stand, kite, cyclops |

#### By scenario (pairwise, filtered)

| Scenario | Movement / path | Combat / state | Overall read |
|----------|-----------------|----------------|--------------|
| **Stand melee** | **~0%** on first paired `branch`/`todo_go`/`go_exec` (dest + tick) | `combat_state` **100%** on 1/1 overlap; `attack_enqueue`/`melee_hit` **0%** after RNG desync | **~45–55%** trace parity |
| **Cyclops quad** | `todo_go` **100%** (4/4); `shortway`/`go_exec` **0%** (wp + path shape) | `combat_state` **100%** (4/4) | **~65–75%** on AI arms; path geometry open |
| **Kite rat** | Not measurable | Not measurable | **n/a** — C++ emits zero JSONL (`wall_ms=0`) |

#### What “close” means in practice

**Structurally right (~80%):** Rust now runs the same idle pipeline as C++ — appear schedules via yield, first stimulus near tick 2000, same JSONL events, seedable glibc RNG for dance/combat. The §16 plan items are implemented; `rtk cargo test -p tfs-rust-core` passes (331 tests).

**Observably identical on stand (~50%):** A lockstep diff still fails on the first meaningful movement decision: C++ dances north at tick 2000; Rust’s first `rand()%5` is West onto the player tile (rejected), so it attacks first and dances south one beat later. That is mostly **global RNG stream position** (C++ synthetic still drains wild-map creatures) plus **single-try dance** behavior, not a missing idle phase.

**Lockstep pass (0%):** The gate is intentionally strict (ordered, index-aligned events). Until stand `branch[0]` dest + tick align and cyclops `shortway`/`go_exec` match, the gate stays red even when most infrastructure is correct.

#### Expected uplift from next fixes

| Fix | Stand rat | Cyclops quad |
|-----|-------------|--------------|
| RNG world isolation (same creatures consuming `rand()` on both sides) | **~50% → ~70–80%** trace parity | Minor (talk gate already wired) |
| Path wp / terrain alignment (`min_wp` 120 vs 150) | Low impact | **~65–75% → ~85%+** on `shortway`/`go_exec` |
| C++ dance retry on blocked sidestep (if confirmed in reference) | First-idle ordering fix | Low |

#### One-line verdicts

| Question | Answer |
|----------|--------|
| Is the chase AI port structurally correct? | **~80% yes** |
| Would a client see identical monster chase at stand melee today? | **~50%** — same cadence band, wrong first sidestep and hit timing |
| Cyclops multi-monster state machine? | **~70%** on combat/todo arms; path steps still off |
| Full lockstep gate passing? | **No** — 0/3 scenarios |

---

## 18. §17 gap closeout — RNG world isolation + arena + kite budget (2026-06-14)

Implemented the three harness fixes from §17.6 plus a required follow-up: **purge map-embedded monsters** on C++ synthetic runs (`.sec` creature objects survive `LoadMonsterhomes` skip).

### 18.1 Fixes shipped

| Fix | Side | File(s) |
|-----|------|---------|
| Skip `monster.db` wild spawns when `TFS_KITE_SYNTHETIC_ARENA` / `TFS_KITE_NO_WILD` | C++ | `crnonpl.cc` `LoadMonsterhomes` |
| Purge all `MONSTER` creatures before synthetic scenario | C++ | `crmain.cc` `PurgeAllMonstersForChaseHarness`, `chase_kite_scenario.cc` |
| Gate `ProcessMonsterhomes` on synthetic harness | C++ | `crnonpl.cc` |
| Shared env helper `ChaseHarnessSkipsWildCreatures()` | C++ | `crmain.cc`, `cr.hh` |
| Synthetic arena radius **16** (covers pathfinding viewport ±10) | scenarios | `kite_rat_stand_melee.scenario`, `kite_rat_melee.scenario`, `kite_cyclops_quad_chase.scenario` |
| Kite rat non-zero time budget (`advance_ms 2000` × 3 → `wall_ms=6000`) | scenarios | `kite_rat_melee.scenario` |

**Tests:** `rtk cargo test -p tfs-rust-core` — **331 passed** (no Rust logic changes).

**Dance retry (§17.6 item 2):** confirmed **already aligned** — both sides use single `rand()%5` with no retry on blocked sidestep (`monster_ai.rs` / `crnonpl.cc` ~2738). Only roam retries (10×).

### 18.2 A/B rerun (`TFS_SIM_SEED=772`, `--synthetic`)

| Scenario | `--lockstep` | C++ events | Rust events | Key pairwise |
|----------|--------------|------------|-------------|--------------|
| Stand rat (`--max-tick 6000`) | **FAIL** | 13 | 10 | `combat_state` 1/1; `branch`/`go_exec` 0/1 (dest N vs S) |
| Kite rat (`--max-tick 6000`) | **FAIL** | >0 (was **0**) | ~10+ | C++ JSONL restored; dance/chase tick cadence still differs |
| Cyclops quad (`--max-tick 4000`) | **FAIL** | 20 | 20 | `todo_go` **4/4**; `combat_state` **4/4**; `shortway`/`go_exec` **0/4** |

### 18.3 Stand melee — wild RNG isolation closed

**Before (§17):** C++ log included map snakes consuming `rand()` before rat dance; Rust scenario-only.

**After (§18):** C++ log is **rat-only** (`PurgeAllMonstersForChaseHarness` + spawn skip). Example first events:

- C++ tick 2000: `branch` dance → `(32361,32291)` (north), `attack_enqueue`; first `melee_hit` at tick 4000
- Rust tick 2000: `melee_hit` only; first `branch` dance → `(32361,32289)` (south) at tick 4000

Remaining stand divergences: **first-idle ordering** (C++ dance-then-attack vs Rust attack-then-dance when west sidestep blocked on Rust) and **RNG stream position** after re-seed (C++ `srand` in `RunChaseKiteScenario` vs Rust `init_sim_rng_from_env` — preamble draws not yet traced to lockstep).

### 18.4 Kite rat — harness time budget closed

`kite_rat_melee.scenario` now has `advance_ms 2000` between kite steps (`wall_ms=6000`). C++ emits JSONL (no longer silent). Compare is meaningful; lockstep still fails on dance dest + chase tick alignment (`todo_go` ref@2000 vs rust@4000 on first chase).

### 18.5 Cyclops quad — partial path parity

Event counts match (**20/20**). `todo_go` and `combat_state` pairwise **100% (4/4)**.

**Still open:**

1. **`min_wp`:** C++ `shortway` logs **`min_wp=120`** at tick 2000; Rust logs **`min_wp=150`** at tick 4000 — native `.sec` tiles still leak into C++ `TShortway::FillMap` viewport despite scenario `arena … 16` (Rust empty-map synthetic is uniform). Likely needs C++ harness to lay synthetic ground at **`radius + 10`** or clear creature/map objects outside arena.
2. **Tick cadence:** C++ chase/`shortway` at tick **2000** after early `player_pos` steps with `advance_ms 0`; Rust chase at tick **4000** — compare pairs different beats even when dest geometry is close.
3. **`shortway`/`go_exec` shape:** 0/4 index-aligned (diagonal steps on C++ ref 1/4 vs rust 0/4).

### 18.6 Lockstep gate summary (post-§18)

| Scenario | `--lockstep` | §17 blocker | §18 status |
|----------|--------------|-------------|------------|
| Stand | **FAIL** | map RNG + dance dest | **Map RNG closed** (rat-only C++ log); dance dest + hit timing open |
| Kite | **FAIL** | C++ zero events | **Time budget closed**; movement/combat trace open |
| Cyclops | **FAIL** | `min_wp` + path shape | **Counts aligned**; C++ `min_wp` + tick cadence + step shape open |

```bash
# Reproduce §18 metrics
scripts/tibia_game_dev.sh build
TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --synthetic scripts/scenarios/kite_rat_stand_melee.scenario
python3 scripts/summarize_chase_gaps.py --ref log/chase_path_cip.log --rust log/chase_path_rust.log \
  --monster rat --max-tick 6000 --lockstep
```

### 18.7 Parity scorecard (post-§18)

| Layer | Estimate | Change vs §17.7 |
|-------|----------|-----------------|
| Harness / RNG world isolation | **~95%** | Wild spawns + map monsters purged on C++ synthetic |
| Stand lockstep trace | **~50–55%** | Unchanged — dance tile/tick still off |
| Cyclops AI arms (`todo_go` / `combat_state`) | **~85%** | Up from ~70% (4/4 pairwise) |
| Cyclops path geometry | **~40%** | Counts match; steps/`min_wp` still diverge |
| Lockstep gate pass rate | **0%** | Still 0/3 |

### 18.8 Next steps (outside this closeout)

1. Re-seed alignment audit — `TFS_SIM_RNG_TRACE=1` on both stacks from `srand(772)` through first rat idle; align C++ re-seed point with Rust world init.
2. C++ synthetic overlay extent — lay grass at **`arena_radius + REVERSE_PATH_VIEW_RADIUS`** so `FillMap` never sees native `wp=120`.
3. Chase tick cadence — align when first `player_pos` + `sim_tick` with `advance_ms 0` schedules C++ chase vs Rust `CreatureMoveStimulus`.

---

## Changelog

| Date | Change |
|------|--------|
| 2026-06-14 | Initial report (pre-fix) |
| 2026-06-14 | P0/P1 fixes + post-fix rerun metrics (§12–13) |
| 2026-06-14 | P2 closeout: tick/RNG/cadence/dedupe/compare (§14) |
| 2026-06-14 | §14 expanded: P2 vs §13 comparison, time-budget finding, tick-bucket compare output |
| 2026-06-14 | P3 closeout: glibc dance RNG, S2 must/max, spell_cast, C++ rebuild + stand A/B metrics |
| 2026-06-14 | §15 lockstep: synthetic arena, glibc combat RNG, todo_go normalize, lockstep gate |
| 2026-06-14 | §16 rerun: stand + kite + cyclops quad A/B; kite C++ zero-event finding; multi-monster combat_state 4/4 |
| 2026-06-14 | §17 closeout: defer appear idle (`ToDoYield`), LoseTarget always-draw, talk gate; tick=0 appear dance closed; lockstep still FAIL on stand dance dest + map RNG isolation |
| 2026-06-14 | §17.7 parity scorecard: layer/scenario estimates (~80% structural, ~50% stand trace, 0% lockstep pass); executive summary updated |
| 2026-06-14 | §18 gap closeout: C++ wild spawn skip + map monster purge, arena radius 16, kite wall_ms=6000; stand rat-only C++ log; kite comparable; cyclops 20/20 counts, todo_go/combat_state 4/4; lockstep still 0/3 |
