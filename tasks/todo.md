# Capacity on level up/down → 0 — 2026-08-01

**Bug:** Leveling overwrites `Player.capacity` with oz-scale vocation formula; runtime is centi-oz → free cap shows 0.

- [x] `VocationProfile::from_def` / `none_vocation`: store `base_cap`/`gain_cap` as centi-oz (×100), matching TFS `vocation.cpp`
- [x] Update vitals tests + level-up/down capacity assertions
- [x] Lesson 293; targeted `cargo test` pass
