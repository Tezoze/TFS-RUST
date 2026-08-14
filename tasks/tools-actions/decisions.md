# Decisions — Lua/Rust boundary and the TFS contract

Index: [README.md](README.md)

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
| `Tile:relocateTo`, `Tile:getBottomCreature`, `Tile:addItem`, `Item.actionid`, `Player:addSkillTries`, `Player:isPzLocked`, `Player:getEffectiveSkillLevel`, `Game.createItem`, `Game.transformItemInPosition`, `Game.sendMagicEffect`, `doTargetCombatHealth` | `onUse*` helpers, `destroyItem`, the item-id tables (`pickGrounds`, `ropeSpots`, `holeId`, `holes`, `sandIds`, `jungleGrass`) | 772 parity **numbers** — see [Gap 6](gaps-lua-api.md#gap-6--772-parity-numbers-hardcoded-in-tool-scripts) |

Every [Gap 3](gaps-lua-api.md#gap-3--missing-lua-api-methods-runtime-failures-even-after-gap-2) entry is an engine capability that cannot be expressed in Lua, so all of them must be Rust. None is generic logic. That split is correct as-is.

**Corollary:** the real weakness of the Lua path is not the language, it's the **load contract** — missing globals surface as `nil` hours after boot, inside a rope click. That is fixed by a load-time assertion ([Gap 5](gaps-load.md#gap-5--load-contract-is-implicit-and-fails-silently--partial-2026-08-10)), not by moving logic into Rust.

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

1. **Finish `registerClass` uniformly** — Gap 7a/7b/7c ✅ done (2026-08-13). Less code than what existed before.
2. **Fail-fast load phases** — Gap 5a ✅ done (2026-08-13). Converts the contract's worst property (silent `nil`) into a boot error.
3. **Generate LuaLS type definitions from the class registry** — detail and timing in [*VM hardening*](vm-hardening.md) pillar 5. Once `register_class` is the single owner of class globals, enumerate every class, method, and constant and emit `.d.lua` annotations. Gives data-pack authors autocomplete and **static** detection of typo'd or missing globals — most of what a "better designed" API would provide, at zero runtime-contract cost. A direct payoff of doing step 1 properly. **Now unblocked** by 7a+7b.
4. **Optional: a second, clean surface for new content** — expose a namespaced Rust module (`local tile = require('tfs.tile')`) with the TFS globals as a thin shim over it. New first-party scripts use the good API; the existing pack is untouched. TFS set the precedent with `compat.lua`. Incremental and reversible. **Do not start before 1-3 land**, and drop it if nothing adopts it — a second unused surface is pure maintenance cost.

### When this decision should be revisited

If the project ever drops the 772 parity target *and* the data-pack dependency — e.g. a greenfield server with bespoke content — then a modern sandboxed module API is the better design and this decision should be reversed. That is not the current mandate.

## Resolved decisions

1. **Helpers stay in Lua** — see [*Design decision*](#design-decision--why-the-tool-helpers-stay-in-lua). Rust adds the missing verbs (Gap 3); `functions.lua` keeps the policy. Resolved 2026-08-10.
2. **`actionIds.destroyableStone`** — TVP defines the table in `data/global.lua` (`destroyableStone=4004`, 4000–4005 block). Do **not** re-define it in `actionids.lua` (that scan would replace the table). `inject_door_tables_from_global` starts at `actionIds = {`. `actionids.lua` only merges TFS extras (`levelDoor=1000`, `citizenship=30020..30050`) that TVP does not have (TVP level doors use `ITEM_ATTRIBUTE_DOORLEVEL`).
3. **`compat.lua` scope** — do **not** load the full ~1500-line compat layer for actions. Register `Item.actionid` natively in `crates/tfs-rust-lua/src/userdata/item.rs` next to the existing `itemid` field (`item.rs:85`) and skip the legacy `doX` surface entirely. Smallest blast radius, and `actionid` is an engine field that belongs in Rust anyway.
4. **No hardcoded script file lists** — `load_data_lib` uses recursive directory scans matching TVP's `Scripts::loadScripts`, not a hardcoded `CORE_FILES` array. The Gap 5 assertion catches missing globals regardless of which file defines them. Resolved 2026-08-10.
5. **`dofile`/`os.time` are available** — the comment at `runtime.rs:1332` claiming they're "not yet wired" is stale. Both work in our mlua LuaJIT VM by default (verified by probe). The real blocker for loading `global.lua` via dofile chain was Gap 7 (`Combat` registered as function, not table) — now resolved (7a+7b landed 2026-08-10). Resolved 2026-08-10.
6. **`register_class` is the single owner of class globals** — not a hardcoded bootstrap list, not per-module `globals.set`. Idempotent and order-independent, so init sequencing stops being load-bearing. Mirrors `luascript.cpp` `registerClass`. Resolved 2026-08-10; **true in the tree as of Gap 7c** (2026-08-13) for every engine class the data-pack lib indexes. `NpcType`/`NpcDialogue` remain bare functions (no lib consumer).
7. **Keep the per-subsystem loaders** — do *not* collapse `load_action_scripts` / `load_spell_scripts` / … into one TVP-style recursive scan. Identical loaded-script set; the split gives typed pending drains and is the better Rust shape. Loader structure is implementation layer, where idiomatic Rust wins over C++ fidelity. Only the lib/content **phase boundary** must match TVP. Resolved 2026-08-10.
8. **Error policy differs by phase** — bootstrap and lib stages are fatal; content stage (revscripts under `data/scripts/<subsystem>/**`) warns and continues. The data pack ships with this repo so a broken lib file is a build defect; shard-authored content must not brick the server. Resolved 2026-08-10; **true in the tree as of Gap 5a** (2026-08-13).
9. **Class support is two mechanisms, not one** — a class table (load-time extensibility) *and* a userdata `__index` fallback to it (call-time resolution). Verified by probe: with only the former, `tile:relocateTo()` still fails. Any "class X is supported" claim must be tested through a live userdata instance. Resolved 2026-08-10.
10. **Keep the TFS Lua-facing contract** — do not replace it with a bespoke API. The data pack is the 772 parity oracle; losing it trades an implementation problem for a verification problem. Improve it via `register_class`, fail-fast phases, and generated LuaLS types instead. See [*Strategic decision*](#strategic-decision--keep-the-tfs-lua-facing-contract). Resolved 2026-08-10.
11. **`createFunctions` is ported into `data/lib/core`** — do not load `compat.lua` (decision #3). `data/lib/core/create_functions.lua` is the TFS helper from `compat.lua:1408`; `data/scripts/lib/create_functions.lua` keeps calling it. Resolved 2026-08-13 (Gap 7c).

## Open questions

1. **Gap 6 source values** — the fishing coefficient (`0.597`) and clamp (`10`/`50`), and pick's 40% / `-50`, are inherited from the TFS data pack. Need `tibia-game-master` citations to confirm they're the 772 numbers before freezing them into `772.lua`.
2. **Verification gap** — `required_data_globals_present_after_lib_load` passed while 9 of 17 core lib files failed to load. Any future "X is done" claim in this doc should be backed by a probe that enumerates load results, not just an allowlist assertion. `lib_stage_loads_with_zero_failures` is the primary Phase 2 guard (Gap 5a); `lib_stage_failures_are_fatal_and_aggregated` locks the fatal/aggregate policy; `lib_core_files_load_with_zero_errors` / `scripts_lib_files_load_with_zero_failures` remain per-directory 7a/7c guards.
3. **`data/actions/**` legacy tree** — delete it or wire it? Currently unreferenced dead content duplicating the tool scripts.
*(Former open question 3 — merge the per-subsystem loaders into one TVP-style scan? — is resolved as decision 7 above: keep them.)*
