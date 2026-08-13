# Tools Action Scripts — Gap Analysis & Plan

**Scope:** Make every script in `data/scripts/actions/tools/` load and run end-to-end on the existing `Action()` pipeline.
**Companion doc:** `tasks/doors-actions-plan.md` (the `Action()` pipeline itself)
**Date:** 2026-08-09 · updated 2026-08-10 (Gaps 5-6, TVP load model, Gap 7a+7b) · **re-audited 2026-08-13** · Gap 7c done 2026-08-13
**Split into this folder 2026-08-13** — was a single 700-line `tasks/tools-actions-gap.md`.

## Status

| Gap | State |
|---|---|
| Gap 1 — `Action:allowFarUse` | not started — [gaps-load.md](gaps-load.md) |
| Gap 2 — lib load before actions | ✅ done 2026-08-09 — [gaps-load.md](gaps-load.md) |
| Gap 3 — missing Lua API methods | inventory re-audited (9 items); implementation not started — [gaps-lua-api.md](gaps-lua-api.md) |
| Gap 4 — missing constants (`SKILL_*`, `actionIds.destroyableStone`) | not started — [gaps-lua-api.md](gaps-lua-api.md) |
| Gap 5 — implicit load contract | ⚠️ partial (recursive scans + boot assertion done) — [gaps-load.md](gaps-load.md) |
| Gap 5a — lib stage fatal | ⬅ **next** (unblocked by 7c) — [gaps-load.md](gaps-load.md) |
| Gap 6 — 772 parity numbers in scripts | not started — [gaps-lua-api.md](gaps-lua-api.md) |
| Gap 7a — `register_class` | ✅ done 2026-08-10 (userdata classes; 17/17 `lib/core` files load) — [gap7-class-globals.md](gap7-class-globals.md) |
| Gap 7b — userdata `__index` chains | ✅ done 2026-08-10 (8 userdata types) — [gap7-class-globals.md](gap7-class-globals.md) |
| **Gap 7c — revscript ctor globals** | ✅ done 2026-08-13 — [gap7-class-globals.md](gap7-class-globals.md) |
| VM hardening pillar 4 (memory limit) | ✅ done 2026-08-10 — [vm-hardening.md](vm-hardening.md) |

## This folder

| Doc | Contents |
|---|---|
| [re-audit-2026-08-13.md](re-audit-2026-08-13.md) | **Read this first.** Everything verified by probe on 2026-08-13: what held, what was wrong, stale line refs |
| [gaps-load.md](gaps-load.md) | Gaps 1, 2, 5, 5a — the load pipeline and its error policy |
| [gaps-lua-api.md](gaps-lua-api.md) | Gaps 3, 4, 6 — missing Lua API surface, constants, and misplaced parity numbers |
| [gap7-class-globals.md](gap7-class-globals.md) | Gap 7a/7b/7c — class globals, userdata `__index` chains, revscript ctor globals |
| [architecture.md](architecture.md) | TVP load model investigation + the target architecture all gaps converge on |
| [vm-hardening.md](vm-hardening.md) | The five sandboxing pillars: which to adopt, which to reject, and when |
| [decisions.md](decisions.md) | Why helpers stay in Lua, why the TFS contract stays, resolved decisions, open questions |

**Target architecture:** see [architecture.md](architecture.md) for the end state all gaps converge on. No gap should be implemented in a way that moves away from it.

## Primary scripts

`data/scripts/actions/tools/*.lua` — 9 files:

| File | Item ids | Behavior |
|------|----------|----------|
| `crowbar.lua` | 2416 | Quest progression (storage 297), door unlock (1209→1211), bookshelf reveal (2593), falls back to `destroyItem` |
| `fishing_rod.lua` | 2580 | Fishing skill gain + fish creation; `allowFarUse(true)` |
| `helmet_of_the_ancients.lua` | 2342 | Attaches gem (2147) → transforms to 2343 |
| `knife.lua` | 2566 | Wraps `onUseKnife` (cake shaping 2683→2096) |
| `machete.lua` | 2420, 2442 | Wraps `onUseMachete` (jungle grass 1499 + grass table) |
| `pick.lua` | 2553 | Destroyable stone (40% / -50 HP), wraps `onUsePick` (pickHole 105 → 392) |
| `rope.lua` | 2120 | Wraps `onUseRope` (rope spots + holes) |
| `scythe.lua` | 2550 | Wraps `onUseScythe` (wheat 2739→2737 + 2694) |
| `shovel.lua` | 2554 | Wraps `onUseShovel` (holes + sand + scarab) |

## Three-layer framing

| Layer | Source of truth | What we match |
|-------|-----------------|---------------|
| **Outcomes** | 772 decompile + TVP `gameserver` use path | Tool transform ids, fishing formula, pick 40%/-50, rope/shovel tile relocations |
| **Domain shape** | TFS revscripts + `data/` | `Action()` self-register; `functions.lua` global helpers (`onUseRope` etc.); `actionIds` table |
| **Implementation** | Rust idioms | Reuse existing `Action()` pipeline; add missing Lua API methods 1:1 to `luascript.cpp` references |

## Current state (verified 2026-08-09)

Pipeline that already works for these scripts:

- `Action()` constructor with `:id` / `:aid` / `:register` — `crates/tfs-rust-lua/src/runtime.rs:1403-1452`
- Recursive loader `load_action_scripts` — `crates/tfs-rust-lua/src/actions.rs:254`
- Dispatch `fire_on_use_action` (`lua_scope.rs:446-462`) → `dispatch_on_use_action` (`lua_event_dispatcher.rs:475`) → `LuaRuntime::call_action_on_use` (`runtime.rs:560`, called at `lua_event_dispatcher.rs:489`)
- Wired into `run_server.rs:255-272` after `inject_door_tables_from_global` + `load_data_lib` + `assert_required_data_globals`

Load test result (all 9 files run through `LuaRuntime::load_action_script`):

```
OK   crowbar.lua
FAIL fishing_rod.lua:83: attempt to call method 'allowFarUse' (a nil value)
OK   helmet_of_the_ancients.lua
OK   knife.lua
OK   machete.lua
OK   pick.lua
OK   rope.lua
OK   scythe.lua
OK   shovel.lua
tools: 9 files, 1 errors, 8 actions registered
```

8 of 9 scripts register. The 9th (`fishing_rod.lua`) fails at **load** time. All 9 would misbehave at **runtime** because of missing Lua API surface (see [Gap 3](gaps-lua-api.md#gap-3--missing-lua-api-methods-runtime-failures-even-after-gap-2)).

## Suggested implementation order

**Reordered 2026-08-10** after the Gap 7 probe; **re-checked 2026-08-13.** Gap 7 moved to the front as a hard prerequisite. 7a+7b landed 2026-08-10; **7c landed 2026-08-13.** Next is Gap 5a.

1. **Gap 2** ✅ done — load `functions.lua` + `data/lib/core/*.lua` before actions in `run_server.rs`.
2. **Gap 5** ⚠️ partial — recursive scans done; warn-and-continue still hides failing lib files (Gap 5a — 3 in `data/scripts/lib` as of 2026-08-13, down from 9+3).
3. **Gap 7a — `register_class`** ✅ done — introduced `crates/tfs-rust-lua/src/class_registry.rs` (`register_class` + `register_engine_class_tables`), routed the **userdata** class globals through it (not the revscript ctor globals — Gap 7c), deleted the hardcoded 8-name bootstrap list and the two `__call` copies. Clears the 9 core lib **load** failures (verified by `lib_core_files_load_with_zero_errors` test).
3b. **Gap 7b — userdata `__index` chain** ✅ done — shared `class_index_lookup` helper in `class_registry.rs` + 8 userdata `MetaMethod::Index` fallbacks (`CreatureRef`/`TileRef`/`ItemRef`/`ContainerRef`/`ItemTypeRef`/`PositionRef`/`CombatRef`/`VocationRef`). Without it 7a fixes loading but `tile:relocateTo(pos)` still fails at call time, which is exactly what `onUsePick` / `onUseShovel` need. Also fixes the latent `Creature`-table bug (`CreatureRef` now chains `Player → Creature`). Verified against live userdata by `gap7b_lua_class_method_callable_via_userdata` (all 8 types), `gap7b_creature_ref_reaches_creature_table`, `gap7b_native_method_wins_over_lua_override`, plus two `class_index_lookup` unit tests. 75 lib tests pass.
3c. **Gap 7c — revscript ctor globals** ✅ done 2026-08-13 — `Action`/`TalkAction`/`MoveEvent`/`Channel`/`Condition`/`Variant`/`MonsterType` through `register_class`; added `CreatureEvent`/`GlobalEvent` with `__call`; ported `createFunctions` into `data/lib/core/create_functions.lua` (not the full compat layer); table-driven `all_class_globals_are_tables` + `scripts_lib_files_load_with_zero_failures`. `data/scripts/lib` loads clean (5/5).
4. **Gap 5a — Phase 2 fatal** — aggregate lib-stage failures into one error and make them boot-blocking; keep content-stage warn-and-continue. **Unblocked by 7c.** After this, every later gap surfaces at boot instead of at use-time.
5. **`new_for_test()`** — route the **8** hand-assembled test VMs in `userdata/combat.rs` (`Lua::new()` at 1082/1122/1198/1226/1316/1446/1565/1650) through the real init path, so tests stop validating a VM that isn't shipped.
6. ~~**Gap 3 re-audit**~~ ✅ **done 2026-08-13** — see [re-audit-2026-08-13.md](re-audit-2026-08-13.md#gap-3--re-audit-result-supersedes-the-gap-3-correction-table). Nine genuinely-missing entries, not fourteen.
7. **Gap 1** — `Action:allowFarUse` + plumbing. Fixes the one load failure.
8. **Gap 4** — `SKILL_*` constants block in `combat_enums.rs`, plus `actionIds.destroyableStone`. Needed for fishing and pick.
9. **Gap 3** — the remaining genuinely-missing methods. Each maps 1:1 to a `luascript.cpp` reference per the C++-reference rule.
10. **Gap 6** — relocate the pick / fishing parity numbers into the profile once the scripts actually run and can be observed.
11. **`global.lua` via dofile** — optional cleanup once 3-5 land: delete `inject_door_tables_from_global`, the inline `string.trim` chunk, and the `data/lib/core` scan. Pure deletion, no behavior change.
12. **LuaLS type definitions from the class registry** — emit `.d.lua` for every registered class, method, and constant. Enabled by 3 (`register_class` as single owner); gives the data pack autocomplete + static missing-global detection. Highest-value item after the tools scripts run. See [*VM hardening*](vm-hardening.md) pillar 5.
13. **[VM hardening](vm-hardening.md)** — `set_memory_limit` ✅ **DONE (2026-08-10)** — `DEFAULT_LUA_MEMORY_LIMIT_BYTES` (512 MiB) applied in `LuaRuntime::new` (`runtime.rs`), overridable from `config.lua` via `luaMemoryLimit` (MB) in `run_server.rs`; test `memory_limit_default_applied_and_enforced` asserts the default + an over-limit allocation errors instead of OOM-killing the process. Instruction-budget hook and stdlib allowlist still gated on Gaps 1-6 + JIT-cost measurement / `tfs.appendLog` capability. See [*VM hardening*](vm-hardening.md) for gates and caveats.

Dependency summary (updated 2026-08-13): 7a+7b+**7c** ✅ done → **5a (next)** → 3 (re-audit ✅ done, implement pending); 3 depends on 4 for the fishing path; 6 is easiest after 3 makes the paths reachable; 1 is independent; 11 is last and optional — and note 11 must **replace** the `data/lib/core` scan, not coexist with it, since `core.lua` currently double-loads 15 core files.

## Verification

```sh
rtk cargo check
rtk cargo test -p tfs-rust-lua actions::tests
```

Existing guards in `crates/tfs-rust-lua/src/actions.rs::tests` (all passing — 77 lib tests as of Gap 7c):

- `tools_scripts_load_and_register` — the 9 tools files register their item ids. **Currently excludes `fishing_rod.lua`** and asserts 8; once Gap 1 is closed, drop the exclusion and assert 9 with zero load errors.
- `required_data_globals_present_after_lib_load` — the Gap 5 allowlist assertion (10 names). Necessary, not sufficient: it passed while 9 lib files failed to load.
- `lib_core_files_load_with_zero_errors` — the Gap 7a guard; `data/lib/core` must load clean.
- `scripts_lib_files_load_with_zero_failures` — the Gap 7c guard; `data/scripts/lib` must load clean.

Target-architecture tests (add as the corresponding step lands):

| Test | Asserts | Step |
|---|---|---|
| `all_class_globals_are_tables` | every `register_class` name is a `table`, and callable where it has a ctor — table-driven so new classes are covered automatically | 3c ✅ |
| `scripts_lib_files_load_with_zero_failures` | the `data/scripts/lib/**` stage loads clean | 3c ✅ |
| `lua_methods_callable_through_userdata` | a Lua-defined method on each class table resolves through a live userdata instance; native Rust methods still take priority | 3b ✅ (`gap7b_lua_class_method_callable_via_userdata` + `gap7b_native_method_wins_over_lua_override`) |
| `lib_stage_loads_with_zero_failures` | Phase 2 returns `Ok` — replaces the 10-name allowlist as the primary guard | 4 |
| tests use `LuaRuntime::new_for_test()` | tests exercise the shipped init path, not a hand-assembled subset | 5 |
