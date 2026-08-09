# Tools Action Scripts — Gap Analysis & Plan

**Status:** Gap 2 done (2026-08-09); Gaps 1, 3, 4 not started
**Date:** 2026-08-09
**Scope:** Make every script in `data/scripts/actions/tools/` load and run end-to-end on the existing `Action()` pipeline.
**Companion doc:** `tasks/doors-actions-plan.md` (the `Action()` pipeline itself)

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

- `Action()` constructor with `:id` / `:aid` / `:register` — `crates/tfs-rust-lua/src/runtime.rs:1359-1414`
- Recursive loader `load_action_scripts` — `crates/tfs-rust-lua/src/actions.rs:80`
- Dispatch `dispatch_on_use_action` → `call_action_on_use` — `crates/tfs-rust-core/src/lua_event_dispatcher.rs:474` ← `crates/tfs-rust-core/src/lua_scope.rs:462`
- Wired into `run_server.rs:230-245` after `inject_door_tables_from_global`

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

8 of 9 scripts register. The 9th (`fishing_rod.lua`) fails at **load** time. All 9 would misbehave at **runtime** because of missing Lua API surface (see §3).

## Gaps

### Gap 1 — Load-time: `Action:allowFarUse` not registered

`fishing_rod.lua:83` calls `action:allowFarUse(true)`. The `Action` table constructor only registers `id` / `aid` / `register` (`runtime.rs:1365-1413`).

**Fix:**
- Add `allowFarUse(bool)` setter on the `Action` constructor that stores `_allow_far_use` on the table.
- Plumb through `PendingAction` → `ActionDef` → `ActionRegistry` so far-use range checks honor it.
- C++ reference: `actions.h` `Action::allowFarUse` / `actions.cpp` `Actions::canUseFar`.

### Gap 2 — `functions.lua` not loaded before action scripts ✅ DONE (2026-08-09)

`rope.lua`, `shovel.lua`, `pick.lua`, `knife.lua`, `machete.lua`, `scythe.lua`, `crowbar.lua` all call global helpers defined in `data/scripts/functions.lua`:

- `onUseRope`, `onUseShovel`, `onUsePick`, `onUseKnife`, `onUseMachete`, `onUseScythe`
- `destroyItem` (crowbar fallback)
- `table.contains` (used by `functions.lua` itself)
- `pickGrounds`, `ropeSpots`, `holeId`, `holes`, `sandIds`, `jungleGrass` tables

These scripts **load fine** (the calls are inside `onUse`, deferred to use-time) but **fail at runtime** — the globals are `nil`.

`functions.lua` is currently only loaded inside the **spell** path (`crates/tfs-rust-lua/src/combat_scripts.rs:210-228`). `run_server.rs:230-244` loads action scripts with only `inject_door_tables_from_global` — no `functions.lua`, no `data/lib/core/*.lua`, no `compat.lua`.

**Fix:**
- Load `data/scripts/functions.lua` (and its `data/lib/core/*.lua` deps, notably `actionids.lua`) before `load_action_scripts` in `run_server.rs`, mirroring `combat_scripts.rs`.
- Extract a shared `load_data_lib(runtime, data_dir)` helper to avoid the third copy of this pattern (already flagged by `TFS-code-hygiene` rule).
- C++ reference: `luascript.cpp` `loadScripts` recursive lib load.

Unblocks 7 of 9 scripts at runtime with zero new Rust API code.

### Gap 3 — Missing Lua API methods (runtime failures even after Gap 2)

Verified by grepping registered methods in `crates/tfs-rust-lua/src/userdata/`. The tools scripts (and `functions.lua`) call these, which are **not registered**:

| Surface | Used by | C++ reference |
|---|---|---|
| `Player:addSkillTries(skill, tries)` | fishing_rod | `luascript.cpp` `luaPlayerAddSkillTries` |
| `Player:getEffectiveSkillLevel(skill)` | fishing_rod | `luaPlayerGetEffectiveSkillLevel` |
| `Player:isPzLocked()` | `functions.lua` `onUseRope` | `luaPlayerIsPzLocked` |
| `Tile:relocateTo(pos)` | `functions.lua` `onUsePick` / `onUseShovel` | `luaTileRelocateTo` |
| `Tile:getBottomCreature()` | `functions.lua` `onUseRope` | `luaTileGetBottomCreature` |
| `Tile:addItem(id, count)` | fishing_rod fallback | `luaTileAddItem` |
| `Game.transformItemInPosition(pos, fromId, toId)` | crowbar | `luaGameTransformItemInPosition` |
| `Game.sendMagicEffect(pos, effect)` | crowbar | `luaGameSendMagicEffect` |
| `Game.createItem(id, count, pos)` | `functions.lua` `onUseScythe` | `luaGameCreateItem` |
| `doTargetCombatHealth(attacker, target, type, min, max)` | pick | `luascript.cpp` `luaDoTargetCombatHealth` |
| `Item.actionid` field (get) | `functions.lua` (`ground.actionid`, `target.actionid`) | `compat.lua` `__index` mapping; only `itemid` field is registered today (`userdata/item.rs:82-97`) |

Already registered and OK (no work needed):
`getId`, `getActionId`, `getPosition`, `transform`, `decay`, `remove`, `getParent`, `addItem` (container/player), `isItem`, `isCreature`, `moveTo`, `getType`, `getStorageValue`, `setStorageValue`, `removeItem`, `sendCancelMessage`, `teleportTo`, `getName`, `hasFlag` (tile+player), `getGround`, `getTopDownItem`, `Position:sendMagicEffect`, `Position:moveUpstairs`.

### Gap 4 — Missing constants / globals

| Symbol | Used by | Status |
|---|---|---|
| `SKILL_FISHING` (=6) + the `SKILL_*` enum family | fishing_rod | **Not registered** — only `CONDITION_PARAM_SKILL_*` exists in `crates/tfs-rust-lua/src/combat_enums.rs`. Add the `SKILL_*` block (enums.h `skills_t`). |
| `actionIds` table (`pickHole`, `sandHole`, `destroyableStone`, …) | pick, `functions.lua` | Defined in `data/lib/core/actionids.lua` — loads once Gap 2's lib load is in place. |

Already registered and OK: `CONST_ME_LOSEENERGY`, `CONST_ME_POFF`, `COMBAT_PHYSICALDAMAGE`, `TILESTATE_PROTECTIONZONE`, `RETURNVALUE_PLAYERISPZLOCKED`.

**Data-pack caveat:** `pick.lua:8` reads `actionIds.destroyableStone`, but `data/lib/core/actionids.lua` does **not** define `destroyableStone`. The branch is currently dead (never matches). Confirm with user whether to add it to `actionids.lua` or leave the branch inert.

## Suggested implementation order

1. **Gap 2 first** — load `functions.lua` + `data/lib/core/*.lua` before actions in `run_server.rs`. Smallest change; immediately makes 7 scripts functional.
2. **Gap 1** — `Action:allowFarUse` + plumbing. Fixes the one load failure.
3. **Gap 4** — `SKILL_*` constants block in `combat_enums.rs`. Needed for fishing.
4. **Gap 3** — the 11 missing methods/globals. The bulk of the work; each maps 1:1 to a `luascript.cpp` reference per the C++-reference rule.

Each gap is independently testable; none block another's design (only Gap 3 depends on Gap 4 for the fishing path).

## Verification

```sh
rtk cargo check
rtk cargo test -p tfs-rust-lua actions::tests
```

Plus a permanent load+register test in `crates/tfs-rust-lua/src/actions.rs::tests` that asserts all 9 tools files register their item ids (template: scratch test run during this analysis). Once Gap 1 is closed, the test should pass with zero load errors and 9 registered actions.

## Open questions

1. **`actionIds.destroyableStone`** — add to `data/lib/core/actionids.lua` (and pick a value), or leave the `pick.lua` branch inert? It's a data-pack decision, not a Rust one.
2. **`compat.lua` scope** — Gap 2 mentions loading `data/lib/compat/compat.lua` for `Item.actionid` etc., but `compat.lua` is large (~1500 lines) and pulls in many legacy `doX` functions. Confirm whether we want the full compat layer loaded before actions, or just the minimal `__index` mapping for `itemid` / `actionid` fields (smaller blast radius).
