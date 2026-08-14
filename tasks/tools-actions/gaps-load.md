# Gaps 1, 2, 5 — the load pipeline

Index: [README.md](README.md) · missing API surface: [gaps-lua-api.md](gaps-lua-api.md) · class globals: [gap7-class-globals.md](gap7-class-globals.md)

## Gap 1 — Load-time: `Action:allowFarUse` not registered ✅ DONE (2026-08-14)

`fishing_rod.lua:83` calls `action:allowFarUse(true)`. The `Action` table constructor only registered `id` / `aid` / `register`.

**Fix (applied):**
- `Action:allowFarUse(bool)` stores `_allow_far_use` on the instance table (`runtime.rs`; C++ `luaActionAllowFarUse`).
- Drain `PendingAction` → `ActionDef` → `ActionEntry.allow_far_use`.
- `EventDispatcher::action_allows_far_use` → ToDo Use Obj2 arm treats it like rune `allowFarUse`: TFS `canUseFar` `areInRange<7,5>` + default `checkFloor`, no walk (`idle_stimulus.rs`). 772 fishing rod (`objects.srv` TypeID 3483) has no `DistUse`.
- C++ reference: `actions.h` `Action::allowFarUse` / `actions.cpp` `Actions::canUseFar` / `Action::canExecuteAction`.

Tests: `tools_scripts_load_and_register` (9 files, id 2580, flag set) + `action_allow_far_use_drains_onto_pending`.

## Gap 2 — `functions.lua` not loaded before action scripts ✅ DONE (2026-08-09)

`rope.lua`, `shovel.lua`, `pick.lua`, `knife.lua`, `machete.lua`, `scythe.lua`, `crowbar.lua` all call global helpers defined in `data/scripts/functions.lua`:

- `onUseRope`, `onUseShovel`, `onUsePick`, `onUseKnife`, `onUseMachete`, `onUseScythe`
- `destroyItem` (crowbar fallback)
- `table.contains` (used by `functions.lua` itself)
- `pickGrounds`, `ropeSpots`, `holeId`, `holes`, `sandIds`, `jungleGrass` tables

These scripts **load fine** (the calls are inside `onUse`, deferred to use-time) but **fail at runtime** — the globals are `nil`.

`functions.lua` is currently only loaded inside the **spell** path (`crates/tfs-rust-lua/src/combat_scripts.rs:210-228`). `run_server.rs` loaded action scripts (pre-Gap 2) with only `inject_door_tables_from_global` — no `functions.lua`, no `data/lib/core/*.lua`, no `compat.lua`.

**Fix:**
- Load `data/scripts/functions.lua` (and its `data/lib/core/*.lua` deps, notably `actionids.lua`) before `load_action_scripts` in `run_server.rs`, mirroring `combat_scripts.rs`.
- Extract a shared `load_data_lib(runtime, data_dir)` helper to avoid the third copy of this pattern (already flagged by `TFS-code-hygiene` rule).
- C++ reference: `luascript.cpp` `loadScripts` recursive lib load.

Unblocks 7 of 9 scripts at runtime with zero new Rust API code.

## Gap 5 — Load contract is implicit and fails silently ✅ DONE (2026-08-13, 5a closed remaining hole)

The load order was incidental rather than declared, and every dependency failure was a `nil` global discovered at use-time, not at boot:

- `functions.lua` was only reachable via the spell path before Gap 2 — an accident, not a design.
- `functions.lua` `onUseShovel` calls `checkScarabTile`, which lives in the **sibling** `data/scripts/scarab_tiles.lua`. Cross-file global dependency with no declared ordering.
- `functions.lua` itself needs `table.contains` and the `actionIds` table from `data/lib/core/`.
- `actionids.lua` lives in `data/lib/core/` but is NOT in `core.lua`'s dofile chain — stock TFS picks it up via the recursive `data/lib/` scan; our hardcoded `CORE_FILES` list missed it.

**Fix (applied — final form):**
- `load_data_lib` (`crates/tfs-rust-lua/src/actions.rs`) now uses **recursive directory scans with no hardcoded file lists**, matching TVP's `Scripts::loadScripts` model (`script.cpp:24-83`):
  - `data/lib/core/**/*.lua` — recursive scan, sorted (replicates `data/lib/lib.lua` → `core.lua` dofile chain without needing `dofile` wired)
  - `data/scripts/lib/**/*.lua` — recursive scan, sorted (matches TVP's `loadScripts("scripts/lib", true, false)`; picks up `create_functions.lua`, `defaults_move_event.lua`, `event_callbacks.lua`, `helper_constructors.lua`, `register_monster_type.lua` — previously not loaded at all)
  - `data/scripts/*.lua` top-level (non-recursive) — picks up `functions.lua` + `scarab_tiles.lua` (the part of TVP's `loadScripts("scripts", false, false)` that no per-subsystem loader covers)
- No individual script filenames are hardcoded. The scans pick up whatever the data pack contains. Alphabetical sort order is safe — no `data/lib/core/*.lua` or `data/scripts/lib/*.lua` file references another at load time (cross-file calls are inside `onUse*` bodies, deferred to use-time). Verified by grepping for cross-file refs.
- Added `assert_required_data_globals(runtime)` (`actions.rs`) — checks a declared list (`onUseRope`, `onUsePick`, `onUseShovel`, `onUseScythe`, `onUseMachete`, `onUseKnife`, `destroyItem`, `checkScarabTile`, `table.contains`, `actionIds`) resolves to the right kind (function/table). New `LuaError::MissingGlobals(Vec<String>)` variant carries the named list.
- `run_server.rs` calls the assertion after `load_data_lib` + `inject_door_tables_from_global` and `anyhow::bail!`s on failure — boot aborts with the missing names instead of producing `nil` globals at use-time.
- Regression test `required_data_globals_present_after_lib_load` in `actions.rs::tests` guards against the load order silently regressing again. The assertion caught the missing `actionIds` on first run (before the recursive scan replaced the hardcoded list).

This gives Rust-style fail-fast at the boundary while keeping the data pack overridable. It is the actual mitigation for the concern that motivates "just put it in Rust."

**Gap 5a — lib-stage fatal ✅ DONE (2026-08-13)**

The recursive scan is correct and stays. `load_data_lib` no longer `tracing::warn!`s and continues: every IO/exec failure across `data/lib/core/**` (minus `core.lua`/`lib.lua` dispatchers), `data/scripts/lib/**`, and top-level `data/scripts/*.lua` is collected into `LuaError::LibStageFailures`. `run_server.rs` `anyhow::bail!`s on that error (same pattern as the Gap 5 globals assertion). Content-stage loaders (`load_action_scripts`, spell/weapon scans) stay warn-and-continue.

`core.lua` / `lib.lua` are skipped so the recursive scan does not double-load every core file and so CWD-relative `dofile` cannot brick boot outside the repo root.

`assert_required_data_globals` stays as a cheap extra guard on the tools contract; `lib_stage_loads_with_zero_failures` is the primary load-stage test. `lib_stage_failures_are_fatal_and_aggregated` locks the policy (two broken files → one error listing both; dispatchers skipped).

**Order note:** Gap 7a/7b/7c/5a/1 are done. Order now: **`new_for_test()` → Gap 4 → Gap 3**.
