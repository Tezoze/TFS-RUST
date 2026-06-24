# TFS-RUST 772 — Real-Map Scenario Proposal

**Date:** 2026-06-23  
**Status:** proposal — decision pending

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

---

## The case for real-map scenarios

### 1. Synthetic arena is now unnecessary overhead

The FillMap walkability parity work (§25) is closed. Rust and C++ now agree on
walkability for mixed terrain (grass over fir tree, synthetic overlay semantics).
The synthetic arena was a workaround for a known conversion gap — that gap is fixed.
Keeping it narrows the test surface without buying anything.

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

### 4. Same code path, no sim changes needed

The Rust sim already loads `forgotten.otbm` for every scenario — the arena overlay
is applied on top of it. Removing `arena_synthetic 1` from a scenario file means the
sim uses the real tiles underneath. The C++ side running without
`TFS_KITE_SYNTHETIC_ARENA=1` uses its sector files directly. Both arrive at the same
world from the same source data.

---

## What changes in practice

| Item | Synthetic arena | Real-map scenario |
|------|----------------|-------------------|
| Scenario file | `arena_synthetic 1` present | `arena_synthetic 1` removed |
| Battery flag | `--synthetic` passed | `--synthetic` omitted |
| C++ terrain | Flat overlay tiles | Native sector file tiles |
| Rust terrain | Flat overlay tiles | OTBM tiles (converted from sectors) |
| Walkability source | Harness-injected | Real conversion |
| Conversion bugs | Hidden | Exposed as lockstep mismatch |
| Terrain coverage | 16-tile flat slab | Real world geometry |

---

## Good candidate areas

| Area type | Value |
|-----------|-------|
| Open flat field (existing arena coords) | Closest to current baseline — easy first step |
| Rat cave / cellar corridor | Exercises constrained pathfinding; matches `rat` monsters |
| Hunter spawn area | Matches `hunter` scenarios; open ground with scatter obstacles |
| Dragon lair entrance | Narrow approach corridor; tests flee pathing on real geometry |
| Any chokepoint / doorway | Stress-tests TShortway and A\* on geometry that matters in-game |

---

## Open question before proceeding

**Where does the C++ binary locate its sector files when running headless `chase-scenario`?**

The C++ game binary runs with `cwd = reference/cipsoft-772/runtime/`. It presumably
reads sector files from a subdirectory there. We need to confirm:

1. What path the C++ binary resolves sector files from in headless mode.
2. Whether those sector files are the same set that `forgotten.otbm` was converted
   from, or a different revision.

If both are from the same sector file set, removing `arena_synthetic 1` should work
without any other changes. If the sector files differ (e.g. a different map revision
was used for the OTBM conversion), there would be coordinate-level tile disagreements
that are source divergence, not conversion bugs.

---

## Decision checklist

- [ ] Confirm C++ sector file path in headless mode
- [ ] Confirm sector files match the OTBM conversion source
- [ ] If both confirmed: create one real-map scenario as a pilot (suggested: rat cave
      corridor or hunter spawn area)
- [ ] Run pilot without `arena_synthetic 1`; compare lockstep result against synthetic baseline
- [ ] If pilot passes: migrate extended battery scenarios to real-map; retire
      `arena_synthetic` as the default
