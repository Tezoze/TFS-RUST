# Tools Action Scripts — Gap Analysis & Plan

**Status:** Gap 2 done; **Gap 7a done** (`register_class` — 9 core lib files now load); Gap 5 **partial** (scans done, warn-and-continue still hides failures — Gap 5a, now unblocked by 7a); Gaps 1, 3, 4, 6, 7b not started
**Target architecture:** see *Target architecture — Lua API + loading system* for the end state all gaps converge on.
**Date:** 2026-08-09 (updated 2026-08-10 — Lua/Rust boundary decision, Gaps 5-6, Gap 5 done, TVP load model investigation)
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

## Design decision — why the tool helpers stay in Lua

**Settled (2026-08-10):** `onUseRope` / `onUsePick` / `onUseShovel` / `onUseScythe` / `onUseMachete` / `onUseKnife` / `destroyItem` stay in `data/scripts/functions.lua`. Do **not** reimplement them as Rust builtins.

Rationale:

- **`functions.lua` is the data-pack contract, not incidental Lua.** Per `TFS-Core`, the domain-shape layer is TFS-shaped precisely so `data/` keeps working. These are globals any later-loaded script can override; as Rust builtins, a shard wanting rope on a custom tile would have to fork the engine instead of dropping a file in `data/scripts/`.
- **Their bodies are content, not mechanics.** `jungleGrass[targetId]`, `ropeSpots`, `holes`, `sandIds`, `1499`, `2739→2737`, `2683→2096` are data-pack item ids. In Rust they'd require a recompile per content change and a fork wherever 772 and 1098 ids diverge.
- **No perf argument.** `onUse` is a rare, player-triggered event, not a per-tick path. Lua call overhead is noise.
- **Gap 2 is the evidence.** It unblocked 7 of 9 scripts with zero new Rust code — the Lua layer is carrying the right amount of weight.

### The boundary: Rust supplies verbs, Lua composes policy

| Rust (Lua API surface) | `functions.lua` (data pack) | `MechanicsProfile` / `data/formulas/772.lua` |
|---|---|---|
| `Tile:relocateTo`, `Tile:getBottomCreature`, `Tile:addItem`, `Item.actionid`, `Player:addSkillTries`, `Player:isPzLocked`, `Player:getEffectiveSkillLevel`, `Game.createItem`, `Game.transformItemInPosition`, `Game.sendMagicEffect`, `doTargetCombatHealth` | `onUse*` helpers, `destroyItem`, the item-id tables (`pickGrounds`, `ropeSpots`, `holeId`, `holes`, `sandIds`, `jungleGrass`) | 772 parity **numbers** — see Gap 6 |

Every Gap 3 entry is an engine capability that cannot be expressed in Lua, so all of them must be Rust. None is generic logic. That split is correct as-is.

**Corollary:** the real weakness of the Lua path is not the language, it's the **load contract** — missing globals surface as `nil` hours after boot, inside a rope click. That is fixed by a load-time assertion (Gap 5), not by moving logic into Rust.

## Strategic decision — keep the TFS Lua-facing contract

**Settled 2026-08-10**, after questioning whether TFS compatibility is worth its cost at all. Answer: **keep it**, for one reason that outweighs the rest.

### What the contract is

| # | Surface | Example |
|---|---|---|
| 1 | Global class tables, monkey-patched from data | `function Tile.relocateTo(self, pos)` — `lib/core/tile.lua:17` |
| 2 | Opaque userdata handles | `CreatureRef(u64)`; native methods first, class table second |
| 3 | Hundreds of global constants | `CONST_ME_POFF`, `COMBAT_PHYSICALDAMAGE`, `RETURNVALUE_*` |
| 4 | Global helper functions | `onUseRope`, `destroyItem`, `doTargetCombatHealth`, `table.contains` |
| 5 | Self-registering revscript objects | `Action():id(2553):register()` → `_pending_actions` drain |
| 6 | Event hook methods | `function Player:onLook(…)` — `data/events/scripts/` |
| 7 | Data files | XML, OTB/OTBM, monster/NPC defs, `config.lua` |
| 8 | Load-order-dependent global assembly | the whole surface is built by executing files in sequence |

Its design flaws are real: one global namespace, no modules/`require`, no encapsulation, no types, and missing names fail as `nil` at use-time.

### Why we keep it anyway

- **The data pack is the 772 parity oracle.** Verification of mechanics outcomes depends on running *the same scripts the reference stack runs* and comparing. A bespoke API turns every parity question from "does the identical script produce the identical result?" into "is my reimplementation faithful?" That trades a solvable implementation problem for an unsolvable verification problem — directly against the `TFS-Core` mandate.
- **The pain to date was not the contract's fault.** Every defect found in this doc — `Tile` clobbered to a function, missing `__index` fallback, absent `Creature` chain, unregistered `Party`/`Teleport`/`Vocation`, warn-and-continue hiding 9 files, four divergent test VMs — is a **half-implementation**, fixed by implementing the contract *more* faithfully. A new API built with the same partial rigor would grow its own equivalent set.
- Secondary: size of the data pack, and existing community content.

### What to do instead of replacing it (ranked by value/cost)

1. **Finish `registerClass` uniformly** — Gap 7a/7b. Less code than what exists today.
2. **Fail-fast load phases** — Gap 5a. Converts the contract's worst property (silent `nil`) into a boot error.
3. **Generate LuaLS type definitions from the class registry** — detail and timing in *VM hardening* pillar 5. Once `register_class` is the single owner of class globals, enumerate every class, method, and constant and emit `.d.lua` annotations. Gives data-pack authors autocomplete and **static** detection of typo'd or missing globals — most of what a "better designed" API would provide, at zero runtime-contract cost. A direct payoff of doing step 1 properly.
4. **Optional: a second, clean surface for new content** — expose a namespaced Rust module (`local tile = require('tfs.tile')`) with the TFS globals as a thin shim over it. New first-party scripts use the good API; the existing pack is untouched. TFS set the precedent with `compat.lua`. Incremental and reversible. **Do not start before 1-3 land**, and drop it if nothing adopts it — a second unused surface is pure maintenance cost.

### When this decision should be revisited

If the project ever drops the 772 parity target *and* the data-pack dependency — e.g. a greenfield server with bespoke content — then a modern sandboxed module API is the better design and this decision should be reversed. That is not the current mandate.

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

**Correction (2026-08-10) — this list is overstated.** Several entries are already implemented in the data pack's core lib and are missing only because their lib file fails to load (Gap 7). Writing them in Rust would duplicate content that ships in `data/`:

| Listed above | Actually implemented at | Real Rust work |
|---|---|---|
| `Tile:relocateTo` | `data/lib/core/tile.lua:17` | only primitives: `Tile:getThingCount`, `Tile:getThing`, `Item:getFluidType` |
| `Game.sendMagicEffect` | `data/lib/core/game.lua:64` | none — needs only `Position:sendMagicEffect` (already registered) |
| `Game.transformItemInPosition` | `data/lib/core/game.lua:69` | only `Tile:getItemById` |
| `Player:addSkillTries` | `data/lib/core/player.lua:110` is a **wrapper** (`local addSkillTriesFunc = Player.addSkillTries`) | native `Player:addSkillTries` still required — the wrapper calls it |
| `Item.getType` | `data/lib/core/item.lua:1` | none |

**Re-audit this whole table after Gap 7 lands.** With 9 core lib files currently not loading, any "missing method" inventory taken today over-counts. The remaining genuinely-Rust entries are: `Player:getEffectiveSkillLevel`, `Player:isPzLocked`, `Tile:getBottomCreature`, `Tile:addItem`, `Game.createItem`, `doTargetCombatHealth`, `Item.actionid`, plus the primitives in the table above.

Already registered and OK (no work needed):
`getId`, `getActionId`, `getPosition`, `transform`, `decay`, `remove`, `getParent`, `addItem` (container/player), `isItem`, `isCreature`, `moveTo`, `getType`, `getStorageValue`, `setStorageValue`, `removeItem`, `sendCancelMessage`, `teleportTo`, `getName`, `hasFlag` (tile+player), `getGround`, `getTopDownItem`, `Position:sendMagicEffect`, `Position:moveUpstairs`.

### Gap 4 — Missing constants / globals

| Symbol | Used by | Status |
|---|---|---|
| `SKILL_FISHING` (=6) + the `SKILL_*` enum family | fishing_rod | **Not registered** — only `CONDITION_PARAM_SKILL_*` exists in `crates/tfs-rust-lua/src/combat_enums.rs`. Add the `SKILL_*` block (enums.h `skills_t`). |
| `actionIds` table (`pickHole`, `sandHole`, `destroyableStone`, …) | pick, `functions.lua` | Defined in `data/lib/core/actionids.lua` — loads once Gap 2's lib load is in place. |

Already registered and OK: `CONST_ME_LOSEENERGY`, `CONST_ME_POFF`, `COMBAT_PHYSICALDAMAGE`, `TILESTATE_PROTECTIONZONE`, `RETURNVALUE_PLAYERISPZLOCKED`.

**Data-pack caveat:** `pick.lua:8` reads `actionIds.destroyableStone`, but `data/lib/core/actionids.lua` does **not** define `destroyableStone`. The branch is currently dead (never matches). **Decision:** add it to `actionids.lua` rather than leave an inert branch (see Open questions).

### Gap 5 — Load contract is implicit and fails silently ⚠️ PARTIAL (2026-08-10)

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
- Skip `core.lua` and `lib.lua` in the `data/lib/core` scan — they are `dofile` dispatchers, redundant under a recursive scan, and their CWD-relative `dofile` fails outside the repo root. (Or resolve Gap 7 and load them for real; see below.)
- Keep `assert_required_data_globals` as a cheap extra guard on the specific tools contract, but it is no longer the primary defense.
- Blocked on **Gap 7** — most of the 9 failures are the class-table issue. Fix Gap 7 first, then flip to fatal, or the server will refuse to boot.

**Order note:** Gap 7 is therefore a prerequisite for closing Gap 5a, and both precede Gap 3 (see below).

### Gap 6 — 772 parity numbers hardcoded in tool scripts

Per the `TFS-Core` conflict rule, era-tuned **numbers** belong in `MechanicsProfile` / `data/formulas/772.lua`, not in data-pack scripts. Two literals are currently in the wrong layer:

| Literal | Location | Move to |
|---|---|---|
| 40% destroy chance, `-50` physical self-damage | `pick.lua:9,13` | profile / `772.lua` (e.g. `destroyableStoneChance`, `destroyableStoneSelfDamage`) |
| Fishing success curve `min(max(10 + (skill - 10) * 0.597, 10), 50)` | `fishing_rod.lua:66` | profile / `772.lua` fishing formula |

Keep the **control flow** in Lua; have the scripts read the numbers from the formulas layer. Verify both against `tibia-game-master` before fixing the values — the 0.597 coefficient and the 10/50 clamp need a decompile citation.

## TVP revscript loading model (investigation 2026-08-10)

Investigated how TVP (`reference/tvp-772/gameserver/src/`) loads Lua scripts to verify our `load_data_lib` matches the engine's design.

### TVP load sequence

Source: `otserv.cpp:239-257` + `scriptmanager.cpp:45-110` + `script.cpp:24-83`.

```
1. C++:  loadFile("data/global.lua")                    ← only hardcoded C++ entry point
   └─ Lua: dofile('data/lib/lib.lua')                   ← inside global.lua:2
        └─ Lua: dofile('data/lib/core/core.lua')        ← inside lib.lua:2
             └─ Lua: dofile('data/lib/core/storages.lua')  ← inside core.lua
             └─ Lua: dofile('data/lib/core/achievements.lua')
             └─ ... (14 more dofiles — explicit list in core.lua)
        └─ Lua: dofile('data/lib/compat/compat.lua')    ← inside lib.lua:5
        └─ Lua: dofile('data/lib/debugging/dump.lua')   ← inside lib.lua:8-9
        └─ Lua: dofile('data/lib/debugging/lua_version.lua')

2. C++:  loadScripts("scripts/lib", true, false)        ← recursive scan, sorted
   └─ data/scripts/lib/**/*.lua (create_functions, event_callbacks, etc.)

3. C++:  XML systems load (weapons, spells, actions, etc.) — XML only, no Lua

4. C++:  loadScripts("scripts", false, false)           ← recursive scan, sorted
   └─ data/scripts/**/*.lua (skips lib/ and events/ subdirs)
       └─ functions.lua, scarab_tiles.lua, actions/**, spells/**, movements/**, etc.

5. C++:  loadScripts("monster", false, false)           ← recursive scan, sorted
   └─ data/monster/**/*.lua
```

**Key findings:**
- **Only `data/global.lua` is hardcoded in C++** (`scriptmanager.cpp:47`). Everything else is recursive directory scans (`script.cpp:24-83`) or Lua-side `dofile` chains.
- **The lib chain (`data/lib/**`) is entirely Lua-driven** via `dofile` from `global.lua`. The scripts stage (`data/scripts/**`, `data/monster/**`) is C++-driven recursive scan.
- **`Scripts::loadScripts`** (`script.cpp:24-83`): recursive `boost::filesystem` iterator, skips `lib/` subdir (when `isLib=false`), skips `events/` subdir always, skips files starting with `#`, sorts `PathBuf`, loads each via `scriptInterface.loadFile`. No per-file logic — pure filesystem walk.
- **TVP does NOT have separate per-subsystem Lua loaders.** One single recursive scan of `data/scripts/**` picks up `functions.lua`, `scarab_tiles.lua`, AND all revscripts in `actions/`, `spells/`, `movements/`, `weapons/`, `talkactions/`, `creaturescripts/`, `chatchannels/`, `eventcallbacks/`. The `PendingAction` / `PendingSpell` / etc. drains happen after that one pass.

### Our current architecture (deviation from TVP)

We have separate per-subsystem loaders: `load_action_scripts` (scans `actions/`), `load_weapon_scripts` (scans `weapons/`), `load_spell_scripts` (scans `spells/`), `load_move_event_scripts` (scans `movements/`), etc. Each is a separate recursive scan of its own subdir, with per-subsystem pending drains after each.

`load_data_lib` handles the lib stage + top-level `data/scripts/*.lua` (the files no per-subsystem loader covers). It uses recursive scans matching TVP's `loadScripts` — no hardcoded file lists.

### `dofile` and `os.time` availability

**Both work in our mlua LuaJIT VM by default.** The comment at `runtime.rs:1294` ("`dofile`/`os.time` dependencies not yet wired") is **stale** — verified by probe test:

- `os.time()` returns epoch seconds (mlua LuaJIT exposes `os` stdlib by default).
- `dofile(path)` resolves relative to process CWD (same as TVP). When CWD is the repo root, `dofile('data/global.lua')` → `dofile('data/lib/lib.lua')` → `dofile('data/lib/core/core.lua')` → all core files resolve correctly.

### Blocker for loading `global.lua` via dofile chain

The dofile chain works (CWD resolution is fine) but fails at `data/lib/core/combat.lua:1`:

```
runtime error: data/lib/core/combat.lua:1: attempt to index global 'Combat' (a function value)
```

`data/lib/core/combat.lua` does `function Combat:getPositions(...)` — adding a method to the `Combat` global. In TVP, `Combat` is a **class table** (created by `registerClass("Combat")` in `luascript.cpp`) with a `__call` metamethod, so it's both callable (`Combat()` creates a `CombatRef` userdata) AND extensible (`function Combat:method(...)` adds a method).

Our `Combat` global is a **bare function** (`userdata/combat.rs:345` — `lua.globals().set("Combat", combat_new)`), not a table. Lua can't index a function value, so `function Combat:getPositions(...)` fails.

### Gap 7 — engine class globals registered as bare functions / not at all

**7a done 2026-08-10** (`register_class` + `register_engine_class_tables` in `crates/tfs-rust-lua/src/class_registry.rs`; all 9 core lib files load — `lib_core_files_load_with_zero_errors` test). **7b not started.** Scope corrected 2026-08-10 — the earlier guess ("likely affects `Spell`, `Weapon`, `Condition`") was **wrong in both directions**. Verified by grepping `data/lib/**` and `data/scripts/**` for `function <Class>[:.]` and probing global kinds after bootstrap.

**Nothing in the data pack extends `Spell`, `Weapon`, or `Condition`** — converting those is speculative work with no consumer. Do not do it. The classes that actually break:

| Global | Registered as | Extended by | Result |
|---|---|---|---|
| `Tile` | function (`userdata/tile.rs:411`) | `lib/core/tile.lua` (6 methods incl. `relocateTo`) | **fails** |
| `Position` | function (`userdata/position.rs:239`) | `lib/core/position.lua` (3 methods incl. `moveUpstairs`) | **fails** |
| `Combat` | function (`userdata/combat.rs:345`) | `lib/core/combat.lua` (2 methods) | **fails** |
| `ItemType` | function (`userdata/item_type.rs:146`) | `lib/core/itemtype.lua`, `item.lua:111` | **fails** |
| `Party` | **not registered** | `lib/core/party.lua`, `events/scripts/party.lua` | **fails** |
| `Teleport` | **not registered** | `lib/core/teleport.lua` | **fails** |
| `Vocation` | **not registered** | `lib/core/vocation.lua` | **fails** |
| `Player`, `Creature`, `Item`, `Container`, `Game` | table (`runtime.rs:1215-1227`) | many | OK |

**Problem:** a bare constructor function can be called but not indexed, so `function Tile.relocateTo(self, …)` raises `attempt to index global 'Tile' (a function value)`. TVP's `registerClass` (`luascript.cpp`) creates a **table** with a `__call` metamethod — callable *and* extensible.

**Root cause is architectural, not a list of 7 omissions.** Three mechanisms independently write class globals, and their outcome is decided by line ordering in `LuaRuntime::new` (`runtime.rs:120-145`):

1. `userdata/*.rs` → `globals.set("Tile", tile_new)` — a bare **function** (`tile.rs:411`, `position.rs:239`, `combat.rs:345`, `item_type.rs:146`, `spell.rs:144`, `weapon.rs:88`)
2. `register_event_script_bootstrap` → `globals.set(name, table)` for a **hardcoded list of 8** (`runtime.rs:1215-1227`)
3. Hand-written `__call` merge blocks — only for `Player` (`runtime.rs:1519-1534`) and `Creature` (`1541-1558`), two near-identical ~18-line copies

(1) clobbers (2), acknowledged in-tree at `runtime.rs:139`:

```rust
register_event_script_bootstrap(&lua)?;
// Overwrite empty `Tile` / `Game` stubs from bootstrap with real constructors.
register_tile_constructor(&lua)?;
```

So the clobbering that breaks `tile.lua` is **deliberate and documented** — the design, not an oversight. Whether a class ends up callable, extensible, both, or `nil` depends on where its registration sits in a 40-line init function. `Position` is a function because it registers *before* the bootstrap and isn't on the list; `Party` is `nil` because nobody added it. Converting 7 classes by hand reproduces this with a longer hardcoded list and **9 copies** of the `__call` block — already at 2, and `TFS-code-hygiene` says extract before the third.

**Correction (2026-08-10, second probe): a class table alone is NOT sufficient.** The first version of this fix would have cleared the 9 load errors and still left `tile:relocateTo(pos)` broken at call time. Verified:

```
Tile made a class table, `function Tile.luaSideMethod` defined,
  TileRef userdata :luaSideMethod()   → Err "attempt to call method (a nil value)"
CreatureRef :creatureOnlyMethod()     → Err "attempt to call method (a nil value)"   (Creature table)
CreatureRef :playerOnlyMethod()       → Ok("player")                                  (Player table)
```

Two separate mechanisms are required, and only the second one makes the methods *callable*:

| | Fixes | Mechanism |
|---|---|---|
| **7a** class table | *load* time — `function Tile.relocateTo(…)` stops erroring | `Tile` global is a table with `__call` |
| **7b** userdata `__index` fallback | *call* time — `tile:relocateTo(pos)` actually resolves | the userdata metatable chains `__index` → the class table |

Only `CreatureRef` has 7b today, hardcoded to the `Player` table (`player.rs:800-803`). Nothing else does.

**Latent bug this exposes:** `CreatureRef`'s fallback reaches `Player` but **not `Creature`**, so all 15 methods in `data/lib/core/creature.lua` (`getPlayer`, `isPlayer`, `setMonsterOutfit`, `addSummon`, `addDamageCondition`, `canAccessPz`, …) plus `functions.lua:530` `Creature:addAttributeCondition` are unreachable from Lua today — independent of the tools work. TFS's `registerClass` takes a **base class** and chains the hierarchy; our single-level hardcoded fallback does not.

**Fix — one primitive pair, not seven conversions:**

Introduce `register_class` as the *only* way a class global is created:

```rust
/// Get-or-create the class table for `name`, optionally attaching a `__call`
/// constructor. Idempotent and order-independent — never replaces an existing
/// table, so registration sequence stops mattering.
/// C++ reference: `luascript.cpp` `LuaScriptInterface::registerClass`.
fn register_class(lua: &Lua, name: &str, ctor: Option<Function>) -> Result<Table, mlua::Error>
```

Then:
- Every `userdata/*.rs` swaps `globals.set(Name, ctor_fn)` → `register_class(lua, Name, Some(ctor_fn))?`.
- `Party` / `Teleport` / `Vocation` → `register_class(lua, Name, None)?` (table-only; no constructor needed for the lib files to attach methods).
- The hardcoded 8-name list in `register_event_script_bootstrap` **is deleted** — a class exists because something registered it, not because it appears on a list.
- The two `__call` blocks collapse into `register_class`; `Player` / `Creature` become ordinary call sites.
- Delete the `runtime.rs:139` overwrite comment and the ordering constraint it documents.

**7b — the matching userdata-side helper.** Generalise `player.rs:800-803` into one shared `MetaMethod::Index` fallback that walks a declared class chain instead of a hardcoded `"Player"`:

| Userdata | Chain (first hit wins, after native Rust methods) |
|---|---|
| `CreatureRef` | `Player` → `Creature` |
| `TileRef` | `Tile` |
| `ItemRef` | `Item` |
| `ItemTypeRef` | `ItemType` |
| `PositionRef` | `Position` |
| `CombatRef` | `Combat` |

Native Rust methods keep priority — mlua only invokes `__index` when the registered-method lookup misses (`player.rs:789-790`), so a Lua override cannot silently shadow an engine method. That ordering must be asserted, not assumed.

Gap 7 then becomes two small helpers plus ~10 mechanical call-site edits, and the bug class cannot recur: there is no longer an API that replaces a class table with a function, and no userdata that silently lacks a fallback.

**Tests:**
- every class global is a `table`, and still callable where it has a constructor (table-driven over registered names)
- for each userdata type, a Lua-defined method on its class table is **callable through a live userdata instance** — this is the check that would have caught the 7a-only plan
- a native Rust method still wins over a same-named Lua method on the class table

**Scope:** **prerequisite for Gap 5a and Gap 3**, not independent. Nine core lib files fail to load because of this; until they load, Gap 5a cannot make lib errors fatal without bricking boot, and Gap 3's inventory can't be trusted (see Gap 3 correction).

## Target architecture — Lua API + loading system

The gaps above are symptoms. This is the end state they should converge on; each gap is a step toward it, and no gap should be implemented in a way that moves away from it.

### Principle: one owner per concern

| Concern | Single owner | Today |
|---|---|---|
| Class globals (`Tile`, `Combat`, …) | `register_class` (Gap 7a) | 3 competing mechanisms, order-dependent |
| Userdata → class-table method lookup | shared `__index` chain helper (Gap 7b) | one hardcoded `"Player"` fallback; every other userdata has none |
| Lib-stage load + error policy | `load_data_lib`, **fatal** | warn-and-continue, hides 9 failures |
| Content-stage load + error policy | per-subsystem loaders, **warn** | same as lib — no distinction |
| Test VM construction | `LuaRuntime::new_for_test()` | 4 hand-assembled copies |

### Three phases with distinct error policies

The current `run_server.rs` sequence is a flat list of calls whose ordering constraints are implicit. Make the phases explicit, because **their error handling genuinely differs**:

```
Phase 1 — Bootstrap (Rust)      register_class for every engine class; constants; enums
                                 → any failure is fatal (programming error)
Phase 2 — Lib (data pack)        data/lib/**, data/scripts/lib/**, data/scripts/*.lua
                                 → FATAL, aggregated: the data pack ships with this repo,
                                   a lib file that does not load is a boot-blocking defect
Phase 3 — Content (revscripts)   data/scripts/{actions,spells,movements,…}/**
                                 → WARN and continue: a broken shard script must not
                                   brick the server; report the file and keep going
```

Phase 2 being fatal is the real fix for Gap 5a — not a longer allowlist. Phase 3 staying lenient is deliberate: shard operators edit content, not lib.

### Prefer `data/global.lua` over hand-rolled substitutes

Once `register_class` lands, the dofile chain works (CWD resolution already verified). At that point these hand-rolled stand-ins should be **deleted**, not maintained:

- `inject_door_tables_from_global` — substring-extraction hack over `global.lua`
- the inline `string.trim` / `string.splitTrimmed` chunk (`runtime.rs:1296`) and its now-stale "dofile not yet wired" comment (`runtime.rs:1294`)
- the `data/lib/core/**` scan in `load_data_lib` (the Lua `dofile` chain replaces it)

Phase 2 then reduces to: `exec_chunk("global.lua")` → `data/scripts/lib/**` scan → top-level `data/scripts/*.lua`. One source of truth, matching TVP's `scriptmanager.cpp:47`.

### Keep the per-subsystem loaders

Resolves open question 3: **do not** merge `load_action_scripts` / `load_spell_scripts` / … into one recursive scan. TVP uses a single scan because C++ made typed pending-drains awkward; the per-subsystem split is cleaner in Rust and produces an identical set of loaded scripts. Loader structure is *implementation* layer, where idiomatic Rust wins over C++ fidelity (`TFS-Core`). Only the phase boundary (lib vs content) needs to match.

### Test VM parity

`userdata/combat.rs` hand-assembles a VM in four places (`1303-1309`, `1433-1437`, `1552-1556`, `1637-1638`), each registering a different subset. Tests therefore validate a VM that is not the one shipped — a contributing reason the Gap 5 assertion passed against a half-loaded lib. Add `LuaRuntime::new_for_test()` that runs the real Phase 1 + Phase 2 and route all tests through it.

## VM hardening — the parts of a "modern sandboxed API" we can adopt

**Measured 2026-08-10.** A modern sandboxed scripting design has five pillars. Three are **orthogonal to the TFS contract** and should be adopted; two are the contract itself and are rejected (see *Strategic decision*).

| Pillar | Breaks the data pack? | Verdict |
|---|---|---|
| 4. Resource limits (instruction + memory) | No | **Adopt** |
| 5. Typed contracts (LuaLS) | No | **Adopt** |
| 1. Stdlib allowlist | Barely — 2 runtime call sites | **Adopt** |
| 2. Per-script `_ENV` | Yes — contract *is* shared globals | Reject |
| 3. Modules / returned descriptors | Yes — replaces `Action():register()` | Reject |

### Pillar 4 — resource limits (reliability)

```rust
lua.set_hook(HookTriggers::new().every_nth_instruction(N), |_, _| {
    Err(mlua::Error::runtime("script exceeded instruction budget"))
})?;
lua.set_memory_limit(BYTES)?;
```

Both exist in mlua 0.12.0; `set_hook` is available on non-Luau builds (our LuaJIT) and is documented for exactly this use.

**Why it matters here:** game simulation is single-threaded (`TFS-threading`). One `while true do end` anywhere in `data/scripts/**` hangs the whole server — no ticks, no packets, no saves, recoverable only by `kill -9`, every player losing state since last save. A runaway allocation OOMs the process. No attacker required; an accidental loop in a quest script is the normal case. The guard turns a total outage into one failed script call.

**Two caveats to decide, not skip:**
- **No rollback.** Mutations apply immediately so scripts can read them back mid-callback (`TFS-lua-boundaries`, Mutation Path). Aborting mid-script leaves *partially applied* effects — failure isolation, not atomicity. Document the semantic per callback.
- **LuaJIT + active hooks probably forces interpreter fallback** (LuaJIT does not call count hooks from compiled traces), which cuts against choosing LuaJIT for speed. **Measure before enabling globally.** Fallbacks: generous budget; hooks on content-stage scripts only; or ship `set_memory_limit` alone first (no JIT impact).

Choose the budget by measuring the heaviest legitimate callback (large loot loops, map-wide iteration), then ~10×.

### Pillar 5 — typed contracts (LuaLS)

Emit `.d.lua` definitions for every registered class, method, and constant; `lua-language-server` gives editor autocomplete and a headless CI check over `data/`.

**Why it matters here:** every Gap 3 / Gap 4 defect is a "name doesn't exist" error that is **statically detectable** — `SKILL_FISHING` undefined, `actionIds.destroyableStone` nil, `Tile:relocateTo` missing. Note that the Gap 3 table in this document is a hand-maintained inventory of which methods exist, and it was **wrong** (over-counted by five). Generated definitions are correct by construction; hand-maintained inventories rot.

Generate the **union** of two sources: methods registered from Rust (enabled by `register_class` being the single owner — Gap 7a), plus methods the data pack defines in Lua (`Tile.relocateTo` lives in `lib/core/tile.lua`), which LuaLS infers from the workspace.

### Pillar 1 — stdlib allowlist (isolation)

Replace `Lua::new()` (mlua `ALL_SAFE`, which includes `io`, `os`, `package`) with an explicit `Lua::new_with(StdLib::STRING | TABLE | MATH | BIT | COROUTINE, …)`. Probed current VM — all of these are live today:

```
io.open = function   os.execute = function   os.remove = function
package.loadlib = function   loadstring = function   debug = nil
```

Any data-pack file can shell out, delete files, or load a native `.so`.

**Measured cost across the whole data pack:**

| Symbol | Uses | Where |
|---|---|---|
| `io.*` | 14 | `functions.lua:287-294` (command log), `default_onReportBug.lua`, `migrations/11.lua`, `migrations/14.lua` |
| `os.time` / `os.date` | 52 | pure time reads |
| `require` | 0 | (2 hits are the English word in NPC dialogue) |
| `loadstring`, `package.*` | 0 | — |

So: **two runtime call sites** become a `tfs.appendLog(kind, text)` capability constrained to `data/logs/`; migrations are one-off tooling and can run in a separate unrestricted VM; keep a minimal `os` shim with `time`/`date`/`clock`. Nothing uses `require`/`loadstring`/`package`, so those drop free.

**Value depends on threat model.** If we are the only script authors, `os.execute` is not a vulnerability — we already have a shell. It becomes real with community scripts, outside content contributions, or hosting shards for others. Cheap enough to do on principle, but lowest urgency of the three.

### When to implement

| Pillar | When | Gate |
|---|---|---|
| 4 — `set_memory_limit` | ✅ done (2026-08-10) — independent of everything else | none; no JIT impact |
| 5 — LuaLS generation | Immediately after **Gap 7a** | needs `register_class` as single owner |
| 4 — instruction hook | After Gaps 1-6 (tools running end-to-end); **before any production or third-party exposure** | needs a JIT-cost measurement + a chosen budget |
| 1 — stdlib allowlist | After Gaps 1-6, alongside or after the instruction hook | needs the `tfs.appendLog` capability first |

Rationale for the ordering: the memory limit is free and prevents a whole-process kill. LuaLS is gated purely on Gap 7a and pays for itself immediately by replacing hand-maintained inventories. The instruction hook needs a real measurement first, so it should not block the tools work. The allowlist is cheap but addresses a threat we may not have yet.

## Suggested implementation order

**Reordered 2026-08-10** after the Gap 7 probe. Gap 7 moved to the front: it is a hard prerequisite, not an optional cleanup.

1. **Gap 2** ✅ done — load `functions.lua` + `data/lib/core/*.lua` before actions in `run_server.rs`.
2. **Gap 5** ⚠️ partial — recursive scans done; warn-and-continue still hides 9 failing lib files (Gap 5a).
3. **Gap 7a — `register_class`** ✅ done — introduced `crates/tfs-rust-lua/src/class_registry.rs` (`register_class` + `register_engine_class_tables`), routed every class global through it, deleted the hardcoded 8-name bootstrap list and the two `__call` copies. Clears the 9 core lib **load** failures (verified by `lib_core_files_load_with_zero_errors` test).
3b. **Gap 7b — userdata `__index` chain** — shared fallback helper with a declared class chain per userdata type. **Not optional:** without it 7a fixes loading but `tile:relocateTo(pos)` still fails at call time, which is exactly what `onUsePick` / `onUseShovel` need. Also fixes the latent `Creature`-table bug. Verify 7a+7b together against a live userdata, not just a load test.
4. **Gap 5a — Phase 2 fatal** — aggregate lib-stage failures into one error and make them boot-blocking; keep content-stage warn-and-continue. Safe only once Gap 7 lands. After this, every later gap surfaces at boot instead of at use-time.
5. **`new_for_test()`** — route the 4 hand-assembled test VMs in `userdata/combat.rs` through the real init path, so tests stop validating a VM that isn't shipped.
6. **Gap 3 re-audit** — re-run the missing-method inventory against a fully-loaded lib. The current list over-counts (see Gap 3 correction).
7. **Gap 1** — `Action:allowFarUse` + plumbing. Fixes the one load failure.
8. **Gap 4** — `SKILL_*` constants block in `combat_enums.rs`, plus `actionIds.destroyableStone`. Needed for fishing and pick.
9. **Gap 3** — the remaining genuinely-missing methods. Each maps 1:1 to a `luascript.cpp` reference per the C++-reference rule.
10. **Gap 6** — relocate the pick / fishing parity numbers into the profile once the scripts actually run and can be observed.
11. **`global.lua` via dofile** — optional cleanup once 3-5 land: delete `inject_door_tables_from_global`, the inline `string.trim` chunk, and the `data/lib/core` scan. Pure deletion, no behavior change.
12. **LuaLS type definitions from the class registry** — emit `.d.lua` for every registered class, method, and constant. Enabled by 3 (`register_class` as single owner); gives the data pack autocomplete + static missing-global detection. Highest-value item after the tools scripts run. See *VM hardening* pillar 5.
13. **VM hardening** — `set_memory_limit` ✅ **DONE (2026-08-10)** — `DEFAULT_LUA_MEMORY_LIMIT_BYTES` (512 MiB) applied in `LuaRuntime::new` (`runtime.rs`), overridable from `config.lua` via `luaMemoryLimit` (MB) in `run_server.rs`; test `memory_limit_default_applied_and_enforced` asserts the default + an over-limit allocation errors instead of OOM-killing the process. Instruction-budget hook and stdlib allowlist still gated on Gaps 1-6 + JIT-cost measurement / `tfs.appendLog` capability. See *VM hardening* for gates and caveats.

Dependency summary: 7 → 5a → 3 (re-audit then implement); 3 depends on 4 for the fishing path; 6 is easiest after 3 makes the paths reachable; 1 is independent; 11 is last and optional.

## Verification

```sh
rtk cargo check
rtk cargo test -p tfs-rust-lua actions::tests
```

Plus a permanent load+register test in `crates/tfs-rust-lua/src/actions.rs::tests` that asserts all 9 tools files register their item ids (template: scratch test run during this analysis). Once Gap 1 is closed, the test should pass with zero load errors and 9 registered actions.

Add alongside it a **required-globals test** (Gap 5): run the lib load stage and assert every name in the declared list resolves to a callable / table. This is the regression guard against the load order silently regressing again.

Target-architecture tests (add as the corresponding step lands):

| Test | Asserts | Step |
|---|---|---|
| `all_class_globals_are_tables` | every `register_class` name is a `table`, and callable where it has a ctor — table-driven so new classes are covered automatically | 3 |
| `lua_methods_callable_through_userdata` | a Lua-defined method on each class table resolves through a live userdata instance; native Rust methods still take priority | 3b |
| `lib_stage_loads_with_zero_failures` | Phase 2 returns `Ok` — replaces the 10-name allowlist as the primary guard | 4 |
| tests use `LuaRuntime::new_for_test()` | tests exercise the shipped init path, not a hand-assembled subset | 5 |

## Resolved decisions

1. **Helpers stay in Lua** — see *Design decision*. Rust adds the missing verbs (Gap 3); `functions.lua` keeps the policy. Resolved 2026-08-10.
2. **`actionIds.destroyableStone`** — **add** it to `data/lib/core/actionids.lua` rather than leave `pick.lua:8` permanently dead. Value still needs picking; cross-check the 772 map's action id usage before choosing one so it doesn't collide.
3. **`compat.lua` scope** — do **not** load the full ~1500-line compat layer for actions. Register `Item.actionid` natively in `crates/tfs-rust-lua/src/userdata/item.rs` next to the existing `itemid` field (`item.rs:82-97`) and skip the legacy `doX` surface entirely. Smallest blast radius, and `actionid` is an engine field that belongs in Rust anyway.
4. **No hardcoded script file lists** — `load_data_lib` uses recursive directory scans matching TVP's `Scripts::loadScripts`, not a hardcoded `CORE_FILES` array. The Gap 5 assertion catches missing globals regardless of which file defines them. Resolved 2026-08-10.
5. **`dofile`/`os.time` are available** — the comment at `runtime.rs:1294` claiming they're "not yet wired" is stale. Both work in our mlua LuaJIT VM by default (verified by probe). The real blocker for loading `global.lua` via dofile chain is Gap 7 (`Combat` registered as function, not table). Resolved 2026-08-10.
10. **Keep the TFS Lua-facing contract** — do not replace it with a bespoke API. The data pack is the 772 parity oracle; losing it trades an implementation problem for a verification problem. Improve it via `register_class`, fail-fast phases, and generated LuaLS types instead. See *Strategic decision*. Resolved 2026-08-10.
9. **Class support is two mechanisms, not one** — a class table (load-time extensibility) *and* a userdata `__index` fallback to it (call-time resolution). Verified by probe: with only the former, `tile:relocateTo()` still fails. Any "class X is supported" claim must be tested through a live userdata instance. Resolved 2026-08-10.
6. **`register_class` is the single owner of class globals** — not a hardcoded bootstrap list, not per-module `globals.set`. Idempotent and order-independent, so init sequencing stops being load-bearing. Mirrors `luascript.cpp` `registerClass`. Resolved 2026-08-10.
7. **Keep the per-subsystem loaders** — do *not* collapse `load_action_scripts` / `load_spell_scripts` / … into one TVP-style recursive scan. Identical loaded-script set; the split gives typed pending drains and is the better Rust shape. Loader structure is implementation layer, where idiomatic Rust wins over C++ fidelity. Only the lib/content **phase boundary** must match TVP. Resolved 2026-08-10.
8. **Error policy differs by phase** — bootstrap and lib stages are fatal; content stage (revscripts under `data/scripts/<subsystem>/**`) warns and continues. The data pack ships with this repo so a broken lib file is a build defect; shard-authored content must not brick the server. Resolved 2026-08-10.

## Open questions

1. **Gap 6 source values** — the fishing coefficient (`0.597`) and clamp (`10`/`50`), and pick's 40% / `-50`, are inherited from the TFS data pack. Need `tibia-game-master` citations to confirm they're the 772 numbers before freezing them into `772.lua`.
2. **Verification gap** — `required_data_globals_present_after_lib_load` passed while 9 of 17 core lib files failed to load. Any future "X is done" claim in this doc should be backed by a probe that enumerates load results, not just an allowlist assertion. Consider a permanent test asserting **zero** lib-stage load failures (feasible once Gap 7 lands).
*(Former open question 3 — merge the per-subsystem loaders into one TVP-style scan? — is resolved as decision 7 below: keep them.)*
