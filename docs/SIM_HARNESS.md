# Simulation harness — current state and implementation plan

**Status:** implementation plan (inventory 2026-09-10)
**Related:** `docs/REFACTOR_AUDIT.md` Phase 2 (quarantine done; crate extract was the unfinished stretch);
`tasks/lessons.md` §70, §100, §127, §232; `docs/772_PLAYER_COMBAT_AUDIT.md` (B4 / sim battery fragility);
`tasks/todo.md` (Sim harness extract)

---

## 1. Intent

Run the **same scenario scripts** against:

| Side | Driver | Reference |
|------|--------|-----------|
| **C++** | `chase_kite_scenario.cc` (tibia-game-master) | Outcomes + draw order for 772 |
| **Rust** | Headless scenario runner | Must match C++ logs under a fixed seed |

Purpose:

- Tune chase / AI / combat **identically** on both stacks
- Diff JSONL path / combat / state traces
- Keep production `tfs-rust-core` free of harness weight — core must not know it is “in sim mode”

Non-goals:

- Replacing unit tests inside crates
- Shipping a dedicated chase JSONL tracer in the live server binary
- Being the primary vehicle for every combat audit finding (unit tests + profile formulas cover most of that)
- Routing scenarios through live `AdvanceGame` (`advance_beat`) — C++ `chase_kite_scenario` drives `MoveCreatures` (clock + `DrainTodoQueue`), not the full beat. Matching that reduced loop is the parity contract.

---

## 2. What exists today

### 2.1 Outer loop (keep)

```
scripts/scenarios/*.scenario
        │
        ▼
scripts/run_kite_scenario.py  ──►  C++ chase_kite_scenario
        │                     ──►  Rust chase_kite_sim (--features sim)
        ▼
log/chase_path_{cip,rust}_*.log  →  summarize_chase_gaps.py
scripts/run_sim_battery.py / run_realmap_sim_battery.py
```

Shared scenarios, dual runners, diff tools. The weight problem is not here.

### 2.2 Inner loop (injected into core)

| Artifact | Location | Actual problem |
|----------|----------|----------------|
| `sim_harness.rs` (~1932 lines) | `tfs-rust-core`, `cfg(any(test, feature = "sim"))` | **Two modules fused:** in-crate test fixtures **and** scenario/parity harness. `test_world.rs` re-exports the whole file; **~1,116 / 1,398** core `#[test]` fns import it. |
| `chase_kite_sim` bin | `tfs-rust-core/src/bin/` | Correct idea, wrong crate. Relies on harness `pub` wrappers over `pub(crate)` world internals. |
| `path_compare` bin | same `src/bin/` | **Clean** — public pathfinding only, no `sim` feature. Leave in core. |
| `chase_debug` | Always compiled: stubs in prod, JSONL under `sim`/`test` | 52 production call sites in 6 files. Stubs are free at runtime; the module + `cfg` still couple core to the harness. |
| `sim_glibc_rand` | Always: `GlibcRngState`; under sim: process-global glibc | Dual stream. Combat/AI branches on `sim_glibc_rng_enabled()`. Free `parity_random()` falls back to `thread_rng` when sim is off (third stream). |
| `GameWorld::parity_random` | `game_world.rs:599` | Prefers the global sim stream when enabled. `init_sim_rng_from_env` reads `TFS_SIM_SEED` inside core. |
| `Player::sim_melee_*` | Always on `Player` | **Not harness state.** Race-data fist fallback (`human.mon` Attack=7 / Defend=5, `crcombat.cc:183`). Set in `login.rs`. Misnamed. |
| `Monster::harness_preserve_sleep` | Always on `Monster` | Genuine harness leak (appear-defer). |
| `feature = "sim"` | `tfs-rust-core/Cargo.toml` | Empty feature used only as a cfg switch. 80 `cfg(any(test, feature = "sim"))` sites in 9 files. |

Phase 2 (`REFACTOR_AUDIT`) cfg-quarantined this so **default production builds compile stubs**. The stretch — move to `tfs-rust-sim` — was never done. Result: “off in the binary, still in the architecture.”

### 2.3 What the battery validates well

- Monster chase / kite / stand / panic / flee / dance timing
- Appear batch + harness wall clock vs C++ scenario clock
- Seeded glibc draw *order* when both sides stay aligned

### 2.4 What it does poorly / falsely suggests

- **General player combat oracle** — weapon resolution, skills, DoTs, combat list. Those need focused unit tests, not chase scenarios.
- **Confidence after RNG desync** — one extra `rand()` poisons the rest of a run; the battery then measures stream drift, not mechanics.
- **Separation** — `cargo test` enables full sim modules via `cfg(test)`, so everyday tests sit on the injected surface.
- **`sim_melee_*` as coupling** — renaming them is hygiene; deleting them breaks unarmed combat (lesson 127).

---

## 3. Target architecture

### 3.1 Dependency rule (hard)

```
tfs-rust-sim  ──depends on──►  tfs-rust-core
     ▲
     │  never
tfs-rust-core  ──✗──►  tfs-rust-sim / sim_glibc_rng_enabled / chase_debug / feature = "sim"
```

Core must not:

- Import harness / scenario modules
- Branch on “sim mode” or read `TFS_SIM_SEED` inside combat / `GameWorld`
- Carry harness fields (`harness_preserve_sleep`, `sim_*` names)
- Own a chase JSONL writer

Core **does**:

- Own one per-world `GlibcRngState` (`parity_rng`) used by live and headless
- Own in-crate **test fixtures** under `#[cfg(test)]` (`test_world` / `test_support`) — these are not the sim crate
- Emit structured `tracing` events on `target = "chase"` (compile-time filterable)
- Expose a small **public** `MoveCreatures`-shaped clock API (see §3.3)

### 3.2 Who owns what

| Concern | Owner |
|---------|--------|
| Game rules, `GameWorld`, combat, AI | `tfs-rust-core` |
| Per-world deterministic RNG (`GlibcRngState` / `parity_rng`) | `tfs-rust-core` — seed from caller (`seed_parity_rng`), never from env inside combat |
| In-crate unit-test fixtures (`minimal_world`, `insert_monster`, …) | `tfs-rust-core` `#[cfg(test)]` (`test_world`) |
| Scenario parse, OTBM/synthetic arena, hero/monster scenario fixtures, wall clock, appear defer | `tfs-rust-sim` |
| JSONL chase traces (subscriber), gap summarize glue | `tfs-rust-sim` + `scripts/` |
| C++ scenario runner | `reference/.../chase_kite_scenario.cc` |
| Battery / dual-run orchestration | `scripts/run_kite_scenario.py`, `run_sim_battery.py`, `run_realmap_sim_battery.py` |
| Pathfinding CLI | `tfs-rust-core` `path_compare` (unchanged) |

### 3.3 How Rust sim drives core (without injection)

Headless runner builds a normal `GameWorld`, then:

1. **Seed** `world.seed_parity_rng(seed)` once at scenario start, and again at documented resync points that mirror C++ `ResyncHarnessRng`. The **sim crate** reads `TFS_SIM_SEED` and calls the method. Core never reads the env var.
2. **Place** creatures through public spawn/appear APIs (or a small public `place_creature_login` that is the real login-shaped path, not a cfg-gated `pub(crate)` fork). Hero fist attack/defend are `Player::fist_attack` / `fist_defense` (race data), set the same way login sets them.
3. **Advance** time via public `MoveCreatures` APIs on `GameWorld` (clock + due-todo drain). Wall-ms clamp and `run_sim_tick` loops live in the sim crate as wrappers. Do **not** call `advance_beat` from scenarios — that is full `AdvanceGame` and would desync vs C++.
4. **Observe** via `tracing::trace!(target: "chase", …)` already in core AI/combat/walk. The sim binary installs a subscriber that writes C++-schema JSONL. Production default log filter does not enable `chase`. No `SimObserver` trait, no `Option<&dyn …>` on hot paths.

C++ keeps its own harness. Parity contract is **scenario file + seed + log schema**, not shared process globals.

### 3.4 RNG contract

```
TFS_SIM_SEED=N          # read only by chase_kite_sim / C++ runner / battery scripts
  → seed_parity_rng(N) at the same scenario milestones
  → every combat/AI draw comes from that world's GlibcRngState only
  → no libc::srand / process-global rand()
  → no thread_rng fallback on the parity helpers
```

Unit tests that need determinism call `seed_parity_rng` explicitly. They do not require `feature = "sim"` or env vars.

After this change, the free functions `parity_random` / `parity_rand_mod` in `sim_glibc_rand.rs` that use `thread_rng` are deleted or become inherent methods on `GlibcRngState` only. All live draws go through `GameWorld::parity_*`.

### 3.5 Observe via `tracing`, not a trait

There are 52 `chase_debug::` references in always-compiled code. Threading `Option<&dyn SimObserver>` through them adds a parameter and dyn dispatch on the hottest AI paths, and the existing stubs already cost zero in production — churn without a win.

`tracing` is already the stack’s logging layer:

```rust
tracing::trace!(
    target: "chase",
    event = "shortway",
    tick,
    cid = ?cid,
    from = %from,
    dest = %dest,
);
```

- Core deletes `chase_debug.rs` (stubs + JSONL writer).
- Default `RUST_LOG` / production max-level leaves `trace` off.
- `chase_kite_sim` installs a layer that captures `target == "chase"` and writes the existing JSONL schema so `compare_chase_live_logs.py` / `summarize_chase_gaps.py` stay unchanged.

### 3.6 Scenario surface (keep / extend)

Keep `.scenario` files as the shared language. Expand only when both C++ and Rust runners implement the step:

- Movement / kite / teleport / wall ms (today)
- Optional later: scripted player strikes, wand/ammo setups, poison assert points — **only if** C++ harness gains the same steps; otherwise use Rust unit tests for one-sided checks

---

## 4. Why “new crate first” fails

`sim_harness.rs` cannot move to a downstream crate on `pub` APIs today.

It reimplements C++ `MoveCreatures` / `DrainTodoQueue` by writing `GameWorld.server_ms` (`pub(crate)`), peeking `todo_queue` (`pub(crate)`), and calling ~15 `pub(crate)` methods (`harness_place_creature_login`, `try_creature_walk_step`, `combat_execute_with_stimulus`, `creature_todo_yield`, `monster_is_target`, `roll_monster_spawn_loot`, `assign_creature_wire_id`, …). Extracting first forces those internals public — trading a cfg coupling for a permanently widened core API.

Worse: **~80% of core unit tests** import the same file via `test_world::support`. A crate extract that takes `sim_harness.rs` wholesale breaks them, because they cannot depend on a downstream crate for `pub(crate)` helpers.

So the crate move is the **last** win. First: one RNG stream, tracing instead of `chase_debug`, split fixtures from scenarios, then a public `MoveCreatures` surface. After that the scenario half has almost nothing private left to reach.

---

## 5. Implementation plan

Ordered to remove weight without breaking the battery overnight. Each phase is a reviewable change. Battery stays green, or is **intentionally re-baselined in the same change**.

Inventory (old sketch step 1) is done — tables in §2 and the 2026-09-10 audit.

### Phase 0 — Rename `sim_melee_*` → fist race fields

**Why first:** mechanical, no behavior change, kills the false “harness leaked into Player” signal. Lesson 127: these are `RaceData[Race].Attack/Defend`.

| Change | Files |
|--------|--------|
| `Player.sim_melee_attack` → `fist_attack` | `creature/player.rs` |
| `Player.sim_melee_defense` → `fist_defense` | same |
| Login defaults stay 7 / 5 | `login.rs` |
| Combat readers | `player/combat/values.rs`, `creature/monster_combat.rs` |
| Struct literals / test overrides | `sim_harness.rs`, `player/combat/strike.rs`, `spell_tests.rs`, `player/inventory/notifications.rs`, `tests/arena.rs` |

Do **not** delete the fields. Do **not** introduce a sim-only wrapper type.

**Exit:** `rg sim_melee_` in `crates/` is empty. Unarmed strike tests still pass.

**Verify:**

```
/home/jessec/.local/bin/rtk cargo test -p tfs-rust-core --lib player::combat
/home/jessec/.local/bin/rtk cargo check -p tfs-rust-core
```

---

### Phase 1 — Collapse dual RNG (highest value)

**Goal:** one `GlibcRngState` per `GameWorld`. Zero `sim_glibc_rng_enabled()` branches in production code. Core never reads `TFS_SIM_SEED`.

| Delete / stop | Replace with |
|---------------|--------------|
| Process-global glibc (`libc::srand` / `sim_random` / `enable_sim_glibc_rng`) | `world.parity_rng` only |
| `sim_glibc_rng_enabled` checks in `game_world.rs`, `combat/rng.rs`, `combat/math.rs`, `creature/monster_combat.rs` | Unconditional `self.parity_rng.*` |
| Free `parity_random` / `parity_rand_mod` `thread_rng` fallback (`sim_glibc_rand.rs:258`) | Callers use `GameWorld::parity_*` or `GlibcRngState` methods |
| `GameWorld::init_sim_rng_from_env` | Sim bin / test helper reads env, calls `seed_parity_rng` |
| `GameWorld::resync_sim_glibc_rng` | Sim crate calls `seed_parity_rng(seed)` at the same milestones `kite_monsters_appear_batch` does today |

Keep: `GlibcRngState`, `DANCE_DIR_ORDER`, `seed_parity_rng`, `parity_random` / `parity_rand_mod` / `parity_random_shuffle` **as inherent `GameWorld` methods with no cfg**.

Rename `sim_dance_choice` → `dance_choice` in the same change if the diff stays small.

**Resync milestones to document in this file when the call sites move (Phase 1 or 5):**

| Milestone | Today | After |
|-----------|--------|--------|
| World construction | `init_sim_rng_from_env` in beat-driven builders | sim crate / test fixture: `seed_parity_rng` |
| After spawn loot / appear batch | `resync_sim_glibc_rng` in `kite_monsters_appear_batch` | sim crate: `seed_parity_rng` again (mirrors C++ `ResyncHarnessRng`) |

**This phase will change draw order** vs the global `libc::rand` stream. Re-baseline `run_sim_battery.py` JSONL in the **same** commit. Do not land the code change with a red battery “to fix later.”

**Exit:** `rg sim_glibc_rng_enabled|enable_sim_glibc_rng|init_sim_rng_from_env|resync_sim_glibc_rng` empty in core. `cargo test -p tfs-rust-core` green. Battery green (new baseline).

**Verify:**

```
/home/jessec/.local/bin/rtk cargo test -p tfs-rust-core
/home/jessec/.local/bin/rtk cargo check -p tfs-rust-core --features sim
python3 scripts/run_sim_battery.py
```

---

### Phase 2 — Replace `chase_debug` with `target = "chase"` tracing

**Goal:** delete `chase_debug.rs` (~790 lines, 41 cfg attributes). Production files keep a one-line `tracing::trace!(target: "chase", event = "…", …)` at each of the 52 sites.

| File (call sites today) | Count |
|-------------------------|-------|
| `idle_stimulus.rs` | 18 |
| `monster_ai.rs` | 18 |
| `creature_todo.rs` | 9 |
| `monster_events.rs` | 3 |
| `walk/mod.rs` | 2 |
| `game_world_lifecycle.rs` | 2 |

JSONL writer moves into `chase_kite_sim` (still in core this phase) as a `tracing` layer that emits the **existing** event names (`branch`, `todo_go`, `shortway`, `go_exec`, `idle_stimulus`, `todo_wait`, `rotate`, `creature_move_stimulus`, `todo_label`, `parked`, `combat_state`, `attack_enqueue`, `melee_hit`, `ranged_hit`, `spell_cast`, `damage_stimulus`, `creature_death`, `harness_player_step`, `fill_map`, `rng_trace`, `rng_resync`) so Python diffs do not change.

`TFS_CHASE_PATH_DEBUG=1` / `TFS_CHASE_PATH_LOG` become subscriber install flags on the sim bin (and optionally on a test helper), not env checks inside AI.

**Do not** add `Option<&dyn SimObserver>` to `GameWorld` or monster think.

**Exit:** `rg chase_debug` empty. `lib.rs` no longer `mod chase_debug`. Battery JSONL comparable (schema-stable; re-baseline only if field names were stub-divergent).

**Verify:** same battery + a single scenario diff against C++.

---

### Phase 3 — Split `sim_harness.rs` inside core (precondition for extract)

**Goal:** two files, still in `tfs-rust-core`. No new crate yet. Tests stay green.

| Stay (`#[cfg(test)]`, re-exported by `test_world`) | Leave as scenario module (`cfg(any(test, feature = "sim"))` until Phase 5) |
|---------------------------------------------------|--------------------------------------------------------------------------|
| `minimal_world`, `beat_driven_world`, `beat_driven_test_world`, `beat_driven_world_with_synthetic_ground` | `SimMapConfig`, `default_sim_map_config`, `beat_driven_world_from_map`, `beat_driven_world_for_kite_synthetic` |
| `test_config`, `test_player`, `minimal_player`, `insert_player` / `insert_monster*` / `insert_npc` / `insert_spectator` | OTBM audit (`audit_otbm_route_tiles`, `write_audit_route_json`, fill-walkable dump) |
| Tile helpers (`ensure_walkable_tile`, `lay_arena_tiles`, synthetic ground types) | `kite_monsters_appear_batch`, `kite_monster_appear`, `harness_place_creature_login` wrapper, `teleport_player`, `walk_player_adjacent` |
| `sim_player_damage_monster` if tests use it | `setup_cyclops_*`, `setup_kite_rat_*` presets |
| Time helpers **once they are `GameWorld` methods** (Phase 4) | Wall clock (`set_sim_harness_wall_ms`), `run_sim_tick` loop, `HarnessScenarioClock` |

Suggested names: `src/test_support.rs` (or keep growing `test_world.rs`) vs `src/sim_scenario.rs`.

`sim_harness_tests.rs` splits with the code it tests: fixture tests stay; appear/wall/OTBM tests travel with the scenario module (and later the sim crate).

**Exit:** `test_world` no longer `pub use crate::sim_harness::*`. Core tests compile without importing scenario presets. `chase_kite_sim` imports only the scenario module.

**Verify:**

```
/home/jessec/.local/bin/rtk cargo test -p tfs-rust-core
/home/jessec/.local/bin/rtk cargo check -p tfs-rust-core --features sim --bins
```

---

### Phase 4 — Public `MoveCreatures` surface; drop cfg-gated `pub(crate)` hooks

**Goal:** scenario code (still in-tree) no longer writes `pub(crate)` fields. C++ contract stays `MoveCreatures`, not `AdvanceGame`.

Promote on `GameWorld` (always compiled, small):

```rust
pub fn server_ms(&self) -> u64;
/// C++ `MoveCreatures` (`crmain.cc:1106`): advance `ServerMilliseconds`, tick counter, drain due todos.
pub fn move_creatures(&mut self, delay_ms: u64);
pub fn next_todo_execution_ms(&self) -> Option<u64>;
```

Scenario `run_sim_tick` / wall clamp become wrappers in the scenario module using only those methods.

Then delete or un-gate:

| Hook | Action |
|------|--------|
| `try_creature_walk_step` (`walk/mod.rs`, cfg-gated) | Use the production walk entry the live player uses, or a public `try_walk` if that **is** the production path |
| `harness_place_creature_login` (`spawn_placement.rs`) | Promote to public `place_creature_login` **if** it is login-shaped and reusable; otherwise scenario uses public login/spawn |
| `Monster::harness_preserve_sleep` | Appear-batch parameter / method on the public appear path, not a domain field |
| Direct `server_ms` / `todo_queue` writes from scenario code | Gone |

Do **not** blindly `pub` the remaining `pub(crate)` helpers (`combat_execute_with_stimulus`, `creature_todo_yield`, …). Either the scenario stops needing them (drive appear/combat through public events) or a **named** public method is added with a C++ citation — no kitchen-sink `pub use`.

**Exit:** scenario module compiles against `pub` `GameWorld` APIs only (treat as a dry-run for the crate). `rg harness_preserve_sleep` empty.

**Verify:** battery + `cargo test -p tfs-rust-core`.

---

### Phase 5 — New crate `tfs-rust-sim`; drop `feature = "sim"`

**Goal:** scenario module + `chase_kite_sim` + its 9 bin tests + tracing JSONL layer live in `crates/tfs-rust-sim`. Depends on `tfs-rust-core`. Core default build has **zero** sim cfg.

| Move | Stay |
|------|------|
| `sim_scenario.rs`, `chase_kite_sim.rs`, JSONL subscriber | `test_support` / `test_world`, `GlibcRngState`, public `move_creatures`, `path_compare` |
| Battery scripts’ cargo invocation | `scripts/scenarios/*.scenario`, Python diffs |

Workspace `Cargo.toml` members += `crates/tfs-rust-sim`. Core `Cargo.toml` drops `sim` feature and the `chase_kite_sim` `[[bin]]`.

Script change: `scripts/run_kite_scenario.py` / `run_sim_battery.py` call `cargo run -p tfs-rust-sim --bin chase_kite_sim` (no `--features sim`).

**Exit (hard):**

```
/home/jessec/.local/bin/rtk rg -n 'feature = "sim"|sim_glibc_rng_enabled|chase_debug::' crates/tfs-rust-core/src
# zero hits
/home/jessec/.local/bin/rtk cargo check -p tfs-rust-core
/home/jessec/.local/bin/rtk cargo run -p tfs-rust-sim --bin chase_kite_sim -- --help
python3 scripts/run_sim_battery.py
```

---

## 6. Success criteria

- Default `cargo check -p tfs-rust-core`: no `sim_harness` / `sim_scenario`, no `chase_debug`, no `sim_glibc_rng_enabled`, no `feature = "sim"`.
- `rg sim_melee_` empty; fist values still 7/5 on login.
- `cargo run -p tfs-rust-sim --bin chase_kite_sim` + C++ scenario produce comparable logs under `TFS_SIM_SEED`.
- Tuning chase/combat means changing **core mechanics once**; both harnesses only re-run scenarios.
- Player-combat audit work stays on **unit tests** in core; the dual harness stays the integration oracle for multi-second chase scenarios.
- Core `cargo test` does not enable a `sim` feature and does not compile scenario/OTBM harness code.

---

## 7. Verification (every phase)

```
/home/jessec/.local/bin/rtk cargo check -p tfs-rust-core
/home/jessec/.local/bin/rtk cargo clippy -p tfs-rust-core --all-targets -- -D warnings
/home/jessec/.local/bin/rtk cargo test -p tfs-rust-core
/home/jessec/.local/bin/rtk cargo check -p tfs-rust-core --features sim   # until Phase 5 drops it
/home/jessec/.local/bin/rtk rg -n 'feature = "sim"|sim_glibc_rng_enabled|chase_debug::|sim_melee_' crates/tfs-rust-core/src
python3 scripts/run_sim_battery.py
```

Phase 1 and Phase 5 are the likely JSONL re-baseline points. Capture old vs new in the PR description; do not mix a mechanics change with a baseline in the same commit if it can be avoided.

Tests: Phase 0 updates struct literals. Phase 1 may need tests that currently rely on `TFS_SIM_SEED` in the environment to call `seed_parity_rng` instead (lesson 70). Phase 3 must not drop the 1,116 fixture-backed tests. New tests: `GameWorld::move_creatures` clock/todo drain (Phase 4); subscriber round-trip of one `chase` event to JSONL (Phase 2/5).

---

## 8. One-line summary

**Today:** dual scenario runners exist, but the Rust side is a cfg-quarantined parasite inside `tfs-rust-core`, fused with the unit-test fixture library.

**Target:** shared scenarios + seed + log schema; C++ and Rust harnesses outside core; core exposes normal world APIs, in-crate test fixtures, one per-world RNG stream, and `tracing` chase events.
