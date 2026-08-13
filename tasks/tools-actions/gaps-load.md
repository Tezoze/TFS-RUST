# Gaps 1, 2, 5 — the load pipeline

Index: [README.md](README.md) · missing API surface: [gaps-lua-api.md](gaps-lua-api.md) · class globals: [gap7-class-globals.md](gap7-class-globals.md)

## Gap 1 — Load-time: `Action:allowFarUse` not registered

`fishing_rod.lua:83` calls `action:allowFarUse(true)`. The `Action` table constructor only registers `id` / `aid` / `register` (`runtime.rs:1403-1452`).

**Fix:**
- Add `allowFarUse(bool)` setter on the `Action` constructor that stores `_allow_far_use` on the table.
- Plumb through `PendingAction` → `ActionDef` → `ActionRegistry` so far-use range checks honor it.
- C++ reference: `actions.h` `Action::allowFarUse` / `actions.cpp` `Actions::canUseFar`.

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

## Gap 5 — Load contract is implicit and fails silently ⚠️ PARTIAL (2026-08-10)

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

**Gap 5a — remaining work: warn-and-continue defeats the assertion (verified 2026-08-10)**

The recursive scan is correct and should stay. But `load_data_lib` logs lib-stage load errors as `tracing::warn!` and continues (`actions.rs:117-119`, `:142-144`, `:173-175`), and the assertion only checks 10 hand-listed names. Probe result — **9 of 17 `data/lib/core/*.lua` files fail to load, and `required_data_globals_present_after_lib_load` still passes:**

```
OK   achievements  actionids  constants  container  creature  game  player  storages
FAIL combat.lua    :1   attempt to index global 'Combat' (a function value)
FAIL tile.lua      :1   attempt to index global 'Tile' (a function value)
FAIL position.lua  :10  attempt to index global 'Position' (a function value)
FAIL itemtype.lua  :14  attempt to index global 'ItemType' (a function value)
FAIL item.lua      :111 (same — ItemType.getNameDescription)
FAIL party.lua     :1   'Party' (a nil value)
FAIL teleport.lua  :1   'Teleport' (a nil value)
FAIL vocation.lua  :1   'Vocation' (a nil value)
FAIL core.lua      cannot open data/lib/... (dofile, CWD-relative)
```

Over half the core lib is missing at runtime and boot is green — exactly the silent-`nil` failure mode Gap 5 was written to eliminate. A curated 10-name allowlist cannot scale to cover the data pack; the load itself must be the guard.

**Fix:**
- Make lib-stage load errors **fatal**: collect failures across the scan and return an aggregated `LuaError` listing every file + error, instead of `tracing::warn!` + continue. The data pack is a build artifact of this repo — a lib file that does not parse is a boot-blocking defect, not a warning. (Per-script *revscript* loads under `data/scripts/actions/**` may stay warn-and-continue; a broken shard script should not brick the server. The distinction is lib stage vs content stage.)
- Skip `core.lua` and `lib.lua` in the `data/lib/core` scan — they are `dofile` dispatchers, redundant under a recursive scan, and their CWD-relative `dofile` fails outside the repo root. (Gap 7 has landed, so loading them for real is now possible — see step 11 in the [implementation order](README.md#suggested-implementation-order).)
- Keep `assert_required_data_globals` as a cheap extra guard on the specific tools contract, but it is no longer the primary defense.
- **Was blocked on Gap 7a** for the `data/lib/core` stage — those 9 failures are gone (0 of 17 fail as of 2026-08-13). **Still blocked on Gap 7c** for the `data/scripts/lib` stage: `create_functions.lua`, `helper_constructors.lua`, `register_monster_type.lua` fail today, so flipping to fatal now aborts boot. See [*Re-audit 2026-08-13*](re-audit-2026-08-13.md).

**Order note:** Gap 7a/7b were prerequisites for the `lib/core` half of Gap 5a; **Gap 7c is the prerequisite for the rest**. Order: 7c → 5a → 3.
