# Simulation harness — current state and target design

**Status:** design note (not an implementation plan commitment)
**Related:** `docs/REFACTOR_AUDIT.md` Phase 2; `tasks/lessons.md` §70, §100, §232;
`docs/772_PLAYER_COMBAT_AUDIT.md` (B4 / sim battery fragility)

---

## 1. Intent (what we want)

Run the **same scenario scripts** against:

| Side | Driver | Reference |
|------|--------|-----------|
| **C++** | `chase_kite_scenario.cc` (tibia-game-master) | Outcomes + draw order for 772 |
| **Rust** | Headless scenario runner | Must match C++ logs under a fixed seed |

Purpose:

- Tune chase / AI / combat **identically** on both stacks
- Diff JSONL (or equivalent) path / combat / state traces
- Keep production `tfs-rust-core` free of harness weight — core should not know it is “in sim mode”

Non-goals for the harness itself:

- Replacing unit tests inside crates
- Shipping diagnostic hooks in the live server binary
- Being the primary vehicle for every combat audit finding (unit tests + profile formulas cover most of that)

---

## 2. What exists today

### 2.1 Outer loop (good shape)

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

This outer loop matches the intent: **shared scenarios, dual runners, diff tools**. The weight problem is not here.

### 2.2 Inner loop (injected into core)

| Artifact | Location | Problem |
|----------|----------|---------|
| `sim_harness.rs` | `tfs-rust-core` behind `cfg(any(test, feature = "sim"))` | Large world-builder / tick API living in the same crate as production |
| `chase_kite_sim` bin | `tfs-rust-core/src/bin/` | Correct idea, wrong crate home |
| `chase_debug` | Stub in prod, full under `sim`/`test` | Production call sites still invoke `log_*` everywhere |
| `sim_glibc_rand` | Split: `GlibcRngState` always; global `SimGlibcRng` under `sim` | Process-global override when `TFS_SIM_SEED` set |
| `GameWorld::parity_random` | Branches to global sim stream if enabled | Live path knows about harness |
| `Player::sim_melee_defense` / `sim_melee_attack` | Always on `Player` | Harness hero stats leaked into domain; login sets them |
| Combat/AI `if sim_glibc_rng_enabled()` | `combat/math.rs`, `monster_combat.rs`, `monster_ai.rs`, … | Core littered with sim branches |

Phase 2 (`REFACTOR_AUDIT`) cfg-quarantined most of this so **default production builds compile stubs**, and left a stretch goal: move to `tfs-rust-sim`. That extract was never done. Result: “off in the binary, still in the architecture.”

### 2.3 What the battery actually validates well

- Monster chase / kite / stand / panic / flee / dance timing
- Appear batch + harness wall clock vs C++ scenario clock
- Seeded glibc draw *order* when both sides stay aligned

### 2.4 What it does poorly / falsely suggests

- **General player combat oracle** — weapon resolution, skills, DoTs, combat list (see player combat audit). Those need focused unit tests, not chase scenarios.
- **Confidence after RNG desync** — one extra `rand()` (e.g. `probe_hit`) poisons the rest of a run; the battery then measures stream drift, not mechanics.
- **Separation** — `cargo test` enables full sim modules via `cfg(test)`, so everyday tests sit on the injected surface.

---

## 3. Target architecture

### 3.1 Dependency rule (hard)

```
tfs-rust-sim  ──depends on──►  tfs-rust-core
     ▲
     │  never
tfs-rust-core  ──✗──►  tfs-rust-sim / sim_* fields / sim_glibc_rng_enabled
```

Core must not:

- Import harness modules
- Branch on “sim mode” / env seed for RNG
- Carry `sim_*` fields on `Player` / `Creature` / `GameWorld`
- Call chase JSONL tracers from production paths (tracing belongs in the sim crate or a thin optional observer)

### 3.2 Who owns what

| Concern | Owner |
|---------|--------|
| Game rules, `GameWorld`, combat, AI | `tfs-rust-core` |
| Per-world deterministic RNG (`GlibcRngState` / `parity_rng`) | `tfs-rust-core` — **one stream for live and headless**; seed from caller, not env magic inside combat |
| Scenario parse, arena build, hero/monster fixtures, wall clock, appear defer | `tfs-rust-sim` |
| JSONL chase traces, gap summarize glue | `tfs-rust-sim` + `scripts/` |
| C++ scenario runner | `reference/.../chase_kite_scenario.cc` (unchanged role) |
| Battery / dual-run orchestration | `scripts/run_sim_battery.py` etc. |

### 3.3 How Rust sim drives core (without injection)

Headless runner builds a normal `GameWorld`, then:

1. **Seed** `world.seed_parity_rng(seed)` once at scenario start (and at documented resync points that mirror C++ `ResyncHarnessRng` — called from the **sim crate**, not from loot code checking env).
2. **Place** creatures with real inventory / skills / vocation data (or a small fixture helper in the sim crate that sets race-equivalent attack/defend via existing APIs — not `sim_melee_*` fields).
3. **Advance** time via public/test-visible tick APIs (`run_sim_tick` / `move_creatures` move into sim crate as wrappers around core beat APIs).
4. **Observe** via a callback / event sink registered by the sim crate (replace scatter `chase_debug::log_*` in core with an optional `SimObserver` trait object or channel that production leaves as `None`/`Null`).

C++ keeps its own harness. Parity contract is **scenario file + seed + log schema**, not shared process globals.

### 3.4 RNG contract (identical tuning)

```
TFS_SIM_SEED=N
  → both runners srand / seed_parity_rng(N) at the same scenario milestones
  → every combat/AI draw comes from that world's stream only
  → no second “harness global” stream that combat can prefer
```

Unit tests that need determinism call `seed_parity_rng` explicitly; they do not require `feature = "sim"` or env vars.

### 3.5 Scenario surface (keep / extend)

Keep `.scenario` files as the shared language. Expand over time only when both C++ and Rust runners implement the step:

- Movement / kite / teleport / wall ms (today)
- Optional later: scripted player strikes, wand/ammo setups, poison assert points — **only if** C++ harness gains the same steps; otherwise use Rust unit tests for one-sided checks

---

## 4. Migration sketch (when we extract)

Ordered to remove weight without breaking the battery overnight:

1. **Inventory the production touchpoints** — every `sim_glibc_rng_enabled`, `sim_melee_*`, `chase_debug::`, `init_sim_rng_from_env`, `resync_sim_glibc_rng`.
2. **New crate `tfs-rust-sim`** — move `sim_harness.rs`, `chase_kite_sim`, harness tests; depend on `tfs-rust-core`.
3. **Collapse dual RNG** — delete process-global `SimGlibcRng` override path; sim crate only seeds `GameWorld::parity_rng`. Re-baseline battery JSONL once.
4. **Remove `Player::sim_melee_*`** — express hero attack/defend via fixtures (skills + unequipped fist race fallback already in combat values, or a sim-only wrapper type that is not `Player` fields).
5. **Replace `chase_debug` call sites** — `Option<&dyn SimObserver>` or feature-free no-op trait in core; real JSONL writer only in sim crate.
6. **Drop `feature = "sim"` from core** — sim becomes a normal binary crate; core default build has zero sim cfg.
7. **Document resync milestones** — table of “after spawn loot”, “after appear”, etc., matching C++ so both sides stay draw-aligned.

Each step should leave `run_sim_battery.py` green (or intentionally re-baselined in the same change).

---

## 5. Success criteria

- Default `cargo check -p tfs-rust-core`: no `sim_harness`, no `sim_melee_*`, no `sim_glibc_rng_enabled` branches.
- `cargo run -p tfs-rust-sim --bin chase_kite_sim -- …` + C++ scenario produce comparable logs under `TFS_SIM_SEED`.
- Tuning a chase/combat behavior means changing **core mechanics once**; both harnesses only re-run scenarios.
- Player-combat audit work (weapon snapshot, probe RNG, DoTs) is validated primarily by **unit tests** in core; the dual harness stays the integration oracle for multi-second chase scenarios.

---

## 6. One-line summary

**Today:** dual scenario runners exist, but the Rust side is a cfg-quarantined parasite inside `tfs-rust-core`.
**Target:** shared scenarios + seed + log schema; C++ and Rust harnesses outside core; core exposes only normal world APIs and a single per-world RNG stream.
