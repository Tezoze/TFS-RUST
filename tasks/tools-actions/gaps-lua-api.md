# Gaps 3, 4, 6 — Lua API surface, constants, parity numbers

Index: [README.md](README.md) · load pipeline: [gaps-load.md](gaps-load.md) · re-audited inventory: [re-audit-2026-08-13.md](re-audit-2026-08-13.md)

## Gap 3 — Missing Lua API methods (runtime failures even after Gap 2)

**Authoritative list — re-audited 2026-08-13 against a fully-loaded lib** (probe + grep of `crates/tfs-rust-lua/src/userdata/`). Nine items; each maps 1:1 to a `luascript.cpp` reference:

| Missing | Needed by | C++ reference |
|---|---|---|
| `Player:addSkillTries(skill, tries)` (native) | fishing_rod; `lib/core/player.lua:110` wrapper calls it | `luaPlayerAddSkillTries` |
| `Player:getEffectiveSkillLevel(skill)` | fishing_rod | `luaPlayerGetEffectiveSkillLevel` |
| `Player:isPzLocked()` | `functions.lua:204` `onUseRope`, `lib/core/creature.lua:171` | `luaPlayerIsPzLocked` |
| `Tile:getBottomCreature()` | `functions.lua:220` `onUseRope` | `luaTileGetBottomCreature` |
| `Tile:addItem(id, count)` | fishing_rod fallback | `luaTileAddItem` |
| `Item:getFluidType()` | `Tile.relocateTo` (`lib/core/tile.lua:26`) — the last primitive it needs | `luaItemGetFluidType` |
| `Game.createItem(id, count, pos)` | `onUseScythe`, `lib/core/container.lua:30`, `functions.lua:44/109/275` | `luaGameCreateItem` |
| `doTargetCombatHealth(attacker, target, type, min, max)` | `pick.lua:13` | `luaDoTargetCombatHealth` |
| `Item.actionid` field (get) | `functions.lua` (`ground.actionid`, `target.actionid`) | `compat.lua` `__index` mapping; only `itemid` + `getActionId()` exist (`userdata/item.rs:85`) |

Supplied by the data pack — **do not port to Rust**: `Tile:relocateTo` (`lib/core/tile.lua:17`), `Game.sendMagicEffect` (`game.lua:64`), `Game.transformItemInPosition` (`game.lua:69`), `Item.getType` (`item.lua:1`), `ItemType:isMovable`.

<details><summary>History — the original (over-counted) inventory and its first correction</summary>

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
| `Item.actionid` field (get) | `functions.lua` (`ground.actionid`, `target.actionid`) | `compat.lua` `__index` mapping; only `itemid` field is registered today (`userdata/item.rs:85`) |

**Correction (2026-08-10) — this list is overstated.** Several entries are already implemented in the data pack's core lib and were missing only because their lib file failed to load (Gap 7, now fixed by 7a+7b). Writing them in Rust would duplicate content that ships in `data/`:

| Listed above | Actually implemented at | Real Rust work |
|---|---|---|
| `Tile:relocateTo` | `data/lib/core/tile.lua:17` | ~~`Tile:getThingCount`, `Tile:getThing`~~ (both already exist — `tile.rs:207/220`); **only `Item:getFluidType`** |
| `Game.sendMagicEffect` | `data/lib/core/game.lua:64` | none — needs only `Position:sendMagicEffect` (already registered) |
| `Game.transformItemInPosition` | `data/lib/core/game.lua:69` | ~~`Tile:getItemById`~~ — already exists (`tile.rs:244`); **none** |
| `Player:addSkillTries` | `data/lib/core/player.lua:110` is a **wrapper** (`local addSkillTriesFunc = Player.addSkillTries`) | native `Player:addSkillTries` still required — the wrapper calls it |
| `Item.getType` | `data/lib/core/item.lua:1` | none |

**Re-audit done 2026-08-13** — superseded by the authoritative table above; full evidence in [re-audit-2026-08-13.md](re-audit-2026-08-13.md#gap-3--re-audit-result-supersedes-the-gap-3-correction-table).

</details>

Already registered and OK (no work needed):
`getId`, `getActionId`, `getPosition`, `transform`, `decay`, `remove`, `getParent`, `addItem` (container/player), `isItem`, `isCreature`, `moveTo`, `getType`, `getStorageValue`, `setStorageValue`, `removeItem`, `sendCancelMessage`, `teleportTo`, `getName`, `hasFlag` (tile+player), `getGround`, `getTopDownItem`, `Position:sendMagicEffect`, `Position:moveUpstairs`.

## Gap 4 — Missing constants / globals

| Symbol | Used by | Status |
|---|---|---|
| `SKILL_FISHING` (=6) + the `SKILL_*` enum family | fishing_rod | **Not registered** — only `CONDITION_PARAM_SKILL_*` exists in `crates/tfs-rust-lua/src/combat_enums.rs`. Add the `SKILL_*` block (enums.h `skills_t`). |
| `actionIds` table (`pickHole`, `sandHole`, `destroyableStone`, …) | pick, `functions.lua` | Defined in `data/lib/core/actionids.lua` — loads once Gap 2's lib load is in place. |

Already registered and OK: `CONST_ME_LOSEENERGY`, `CONST_ME_POFF`, `COMBAT_PHYSICALDAMAGE`, `TILESTATE_PROTECTIONZONE`, `RETURNVALUE_PLAYERISPZLOCKED`.

**Data-pack caveat:** `pick.lua:8` reads `actionIds.destroyableStone`, but `data/lib/core/actionids.lua` does **not** define `destroyableStone`. The branch is currently dead (never matches). **Decision:** add it to `actionids.lua` rather than leave an inert branch (see [open questions](decisions.md#open-questions)).

## Gap 6 — 772 parity numbers hardcoded in tool scripts

Per the `TFS-Core` conflict rule, era-tuned **numbers** belong in `MechanicsProfile` / `data/formulas/772.lua`, not in data-pack scripts. Two literals are currently in the wrong layer:

| Literal | Location | Move to |
|---|---|---|
| 40% destroy chance, `-50` physical self-damage | `pick.lua:9,13` | profile / `772.lua` (e.g. `destroyableStoneChance`, `destroyableStoneSelfDamage`) |
| Fishing success curve `min(max(10 + (skill - 10) * 0.597, 10), 50)` | `fishing_rod.lua:66` | profile / `772.lua` fishing formula |

Keep the **control flow** in Lua; have the scripts read the numbers from the formulas layer. Verify both against `tibia-game-master` before fixing the values — the 0.597 coefficient and the 10/50 clamp need a decompile citation.
