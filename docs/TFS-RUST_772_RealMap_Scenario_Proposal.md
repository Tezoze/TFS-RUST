# TFS-RUST 772 — Real-Map Scenario Proposal

**Date:** 2026-06-25 (updated)  
**Status:** pilot run complete — see [`TFS-RUST_772_RealMap_Parity_Trajectory.md`](TFS-RUST_772_RealMap_Parity_Trajectory.md)  
**Implementation plan:** [`TFS-RUST_772_Real_Map_Kite_Sim_Plan.md`](TFS-RUST_772_Real_Map_Kite_Sim_Plan.md)

---

## Background

All current sim battery scenarios use `arena_synthetic 1`, which overlays a flat
slab of walkable tiles over a 16-tile radius of the real OTBM map before running
the AI comparison. The C++ game receives `TFS_KITE_SYNTHETIC_ARENA=1` via
environment and lays the same synthetic arena from its side.

This overlay was introduced as a safety net: during the §25 P2.5 FillMap parity
work, a fir tree (TypeID 3682) sitting under a synthetic grass tile caused a
walkability disagreement between C++ and Rust. Synthetic tiles eliminated terrain
as a variable while that was being debugged.

FillMap parity is now closed (§25). Synthetic scenarios still pass lockstep on the
base battery (~5/6 base + extended rows still diverge on hunter/dragon). The next
step is a **separate real-map battery** that exercises terrain the synthetic slab
hides — without retiring the synthetic gate.

---

## Map format architecture

```
772 sector files  ──►  C++ game (reads sector files natively)
      │
      └──► OTBM conversion  ──►  Rust sim (loads data/world/forgotten.otbm)
```

The `forgotten.otbm` loaded by the Rust sim was converted from the same 772 sector
files the C++ reference reads. They represent the same world. They are not the same
file format, but they should encode the same tile data for every coordinate.

| Stack | Terrain loader | Default path |
|-------|----------------|--------------|
| C++ headless `chase-scenario` | `.sec` sectors | `reference/cipsoft-772/runtime/map/*.sec` (`MAPPATH` in runtime config; cwd = `runtime/`) |
| Rust `chase_kite_sim` | OTBM + `objects.srv` | `data/world/forgotten.otbm` |

`scripts/tibia_game_dev.sh` rewrites `MAPPATH` / `ORIGMAPPATH` under the runtime
tree. `origmap/` holds pristine sectors; `map/` is the live runtime copy. Scenario
coordinates must be walkable on **both** loaders at the same `(x, y, z)`.

---

## The case for real-map scenarios

### 1. Synthetic arena is now unnecessary overhead for lockstep gating

The FillMap walkability parity work (§25) is closed. Rust and C++ now agree on
walkability for mixed terrain (grass over fir tree, synthetic overlay semantics).
Keeping synthetic as the **regression gate** is still valuable; using it as the
**only** terrain surface is not.

### 2. Any remaining conversion gap becomes a detectable bug

If the OTBM conversion missed a tile attribute — an impassable item, a different
move cost, a blocking flag — a real-map scenario will expose it as a lockstep
mismatch. With synthetic tiles those bugs are invisible because the overlay
replaces the real tile data anyway. Real-map scenarios make conversion fidelity a
first-class test concern.

### 3. In-game feel is on real terrain, not a flat arena

The whole point of the sim is to verify that the AI feels the same in-game. In-game
the AI runs on real terrain — corridors, walls, obstacles. A hunter chasing a player
in its actual spawn area, on actual tiles, is a more meaningful parity test than the
same hunter on a flat slab.

**Observed gap (2026-06):** synthetic `kite_cyclops_quad_chase` locksteps, but
kiting **six** cyclops live on real terrain still feels wrong. That is expected until
we run the same geometry in a real-map scenario and compare traces — not eyeballing.

### 4. Same harness, minimal change for v0 pilot

The Rust sim already loads `forgotten.otbm` when `arena_synthetic` is omitted.
Removing `arena_synthetic 1` and `--synthetic` uses real tiles underneath. C++ uses
native `.sec` tiles when `TFS_KITE_SYNTHETIC_ARENA` is unset. No new map loader is
required for the first pilot.

Proposed DSL additions (`mode real_map`, `player_walk`, …) are documented in the
implementation plan; **v0 pilot can use today's verbs** (`arena`, `player_start`,
`monster`, `player_pos`, `advance_ms`, `sim_tick`) without synthetic overlay.

---

## What changes in practice

| Item | Synthetic arena | Real-map scenario |
|------|----------------|-------------------|
| Scenario file | `arena_synthetic 1` present | `arena_synthetic 1` **removed** |
| Battery flag | `--synthetic` passed | `--synthetic` **omitted** |
| C++ terrain | Flat overlay tiles | Native sector file tiles |
| Rust terrain | Flat overlay tiles | OTBM tiles (converted from sectors) |
| Walkability source | Harness-injected | Real conversion |
| Conversion bugs | Hidden | Exposed as lockstep mismatch |
| Terrain coverage | 16-tile flat slab | Real world geometry |
| Battery role | Canonical regression gate | Additional parity suite |

---

## Finding coordinates for a scenario

Real-map authoring needs **world** `(x, y, z)` — the same numbers in `.scenario`,
OTBM, and `.sec`.

### From `.sec` sector files

Filename `SSSS-YYYY-ZZ.sec` encodes sector indices `(SectorX, SectorY, Z)`.
Each sector is **32×32** tiles. Line `LX-LY: Content={…}` is a local offset.

```
world_x = sector_x * 32 + local_x
world_y = sector_y * 32 + local_y
world_z = sector_z
```

Example: tile `15-20` in `0996-0989-07.sec` → **(31887, 31668, 7)**.

Reverse lookup for world `(x, y, z)`:

```
sector_x = x / 32,  sector_y = y / 32
local_x  = x % 32,  local_y  = y % 32
→ open {sector_x:04d}-{sector_y:04d}-{z:02d}.sec
```

`.sec` files contain **terrain/items only** — not creature spawns. Use them to
audit walkability and item stacks, not to place monsters.

### From `spawns.xml` / RME

Monster spawn **zones** live in `data/world/spawns.xml` (`<spawn centerx centery
centerz radius>` + child `<monster x y name>` offsets). World position:

```
world_x = centerx + offset_x
world_y = centery + offset_y
```

`scripts/spread_spawn_offsets_for_rme.py` fixes duplicate `(0,0)` offsets for RME
editing; 772 Rust `Classic772Bfs` still searches from zone center, but distinct
offsets help pick visible tiles in the editor.

### From in-game / existing synthetic baseline

Current synthetic cyclops quad uses **32360, 32290, 7** (Thais lab area) — sector
file **`1011-1009-07.sec`**. A real-map pilot can start here (flat-ish, already
validated on OTBM) before moving to chokepoints or spawn areas.

### Required `.scenario` fields (v0)

| Field | Purpose |
|-------|---------|
| `z` | Floor |
| `arena cx cy radius` | Validation window: spawns + all `player_pos` + ~10 tile margin |
| `player_start x y` | Player spawn |
| `monster <label> x y` | One line per monster; **order matters** (idle drain / `harness_spawn_order`) |
| `player_pos x y` + `advance_ms` / `sim_tick` | Scripted kite path |
| *(omit)* `arena_synthetic 1` | Use real terrain |
| *(omit)* `--synthetic` on runner | Same |

Rust fails fast via `validate_positions_walkable` / `validate_arena_walkable` on
OTBM before sim starts.

---

## Good candidate areas

| Area type | Value |
|-----------|-------|
| Open flat field (32360, 32290 lab) | Closest to synthetic baseline — **first pilot** |
| Cyclops spawn near Thais / real field | Six-monster kite matching live repro |
| Rat cave / cellar corridor | Constrained pathfinding; matches `rat` monsters |
| Hunter spawn area | Matches `hunter` scenarios; open ground with scatter obstacles |
| Dragon lair entrance | Narrow approach corridor; flee pathing on real geometry |
| Any chokepoint / doorway | Stress-tests reverse terrain path and A* on geometry that matters in-game |

**Suggested pilot:** `kite_cyclops_six_real` — extend quad chase to six cyclops on
real tiles at the lab coords (or user-provided live coords), same `player_pos`
script as `kite_cyclops_quad_chase.scenario`.

---

## Sim vs live — why synthetic pass ≠ live feel

| Factor | Headless sim | Live server |
|--------|--------------|-------------|
| Terrain | Synthetic slab or fixed OTBM slice | Full map, ambient creatures/items |
| Player movement | `player_pos` teleports (v0) | Client input, walk beats, latency |
| Monster count | Scripted (4 in quad chase) | User repro (e.g. 6 cyclops) |
| Todo drain | Harness `sim_tick` ordering | FIFO idle queue under load |
| RNG / scheduling | `TFS_SIM_SEED=772` pinned | glibc draw order + async stimuli |
| Comparison | `compare_chase_live_logs.py` lockstep | Same tool with `TFS_CHASE_PATH_DEBUG` / `TIBIA_CHASE_PATH_DEBUG` |

Real-map sim closes the **terrain** column. It does not replace live replay — run
both: sim lockstep on a frozen route, then live logs on the same coords.

**772 AI hygiene (2026-06):** several 1098-only paths (`monster_on_think_target`,
synchronous `searchTarget` on damage, forced `updateLookDirection` on target
select) were gated behind `beat_driven_loop` / `MechanicsProfile` — not
`clientVersion == 772`. That improves live 772 feel independently of real-map
scenarios; real-map tests still needed to catch pathfinder / map conversion bugs.

---

## Open questions (remaining)

1. **OTBM vs `map/` revision drift** — confirm `forgotten.otbm` was built from the
   same sector set as `runtime/map/` (not only `origmap/`). Mismatches show as tile
   disagreements, not AI bugs.
2. **`player_pos` on real terrain** — teleports can skip collision semantics; plan
   adds `player_walk` for v1. Acceptable for pilot if every tile is validated
   walkable and route is short.
3. **Ambient map state on C++** — headless harness should spawn only scenario
   monsters (same lesson as synthetic §28).

---

## Decision checklist

- [x] Confirm C++ sector file path in headless mode → `runtime/map/` via `MAPPATH`, cwd `runtime/`
- [x] Confirm `forgotten.otbm` matches `runtime/map/` sector revision (OTBM converted from `.sec`; spot-check `(32451,32065,7)` — same ground wp=150 + 3-item stack)
- [x] Author `scripts/scenarios/kite_cyclops_six_real.scenario` (no `arena_synthetic`, `player_walk` route)
- [x] Run Rust pilot: `TFS_SIM_SEED=772 cargo run -p tfs-rust-core --bin chase_kite_sim -- scripts/scenarios/kite_cyclops_six_real.scenario`
- [x] Run lockstep pilot: `TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --real-map scripts/scenarios/kite_cyclops_six_real.scenario` (baseline: `log/summary_realmap_cyclops_six_real.txt`; divergence expected — not synthetic gate)
- [ ] Compare lockstep vs synthetic quad baseline; file divergence in [`TFS-RUST_772_RealMap_Parity_Trajectory.md`](TFS-RUST_772_RealMap_Parity_Trajectory.md) §4
- [ ] Optional: live repro at same coords with chase debug + `compare_chase_live_logs.py`
- [x] Add `--real-map` battery slice (`scripts/run_realmap_sim_battery.py`); synthetic remains default gate
