# Gaps 3, 4, 6 — Lua API surface, constants, parity numbers

Index: [README.md](README.md) · load pipeline: [gaps-load.md](gaps-load.md) · re-audited inventory: [re-audit-2026-08-13.md](re-audit-2026-08-13.md)

## Gap 3 — Missing Lua API methods (runtime failures even after Gap 2)

✅ **done 2026-08-14.**

Nine engine verbs plus `Tile:getGround` returning Item userdata (TFS `luaTileGetGround`). Bindings in `tfs-rust-lua`; mutations apply immediately via `LuaMutation` → `game_world_lua_tools.rs`.

| Method | Needed by | C++ reference | Status |
|---|---|---|---|
| `Player:addSkillTries(skill, tries)` (native) | fishing_rod; `lib/core/player.lua:110` wrapper | `luaPlayerAddSkillTries` | ✅ no `rateSkill` in Rust (wrapper disables multiplier) |
| `Player:getEffectiveSkillLevel(skill)` | fishing_rod | `luaPlayerGetEffectiveSkillLevel` | ✅ `skill_level_profile` / `SkillNr::from_tfs_skill_id` |
| `Player:isPzLocked()` | `functions.lua:204` `onUseRope` | `luaPlayerIsPzLocked` | ✅ `earliest_protection_zone_round > round_nr` |
| `Tile:getBottomCreature()` | `functions.lua:220` `onUseRope` | `luaTileGetBottomCreature` | ✅ oldest = `creatures.first()` (Rust `push` vs TFS `insert(begin)`) |
| `Tile:addItem(id, count)` | fishing_rod fallback | `luaTileAddItem` | ✅ |
| `Item:getFluidType()` | `Tile.relocateTo` | `luaItemGetFluidType` | ✅ `ScriptItemData.fluid_type` |
| `Game.createItem(id, count, pos)` | `onUseScythe`, `container.lua`, `functions.lua` | `luaGameCreateItem` | ✅ with pos → tile + `FLAG_NOLIMIT`; without → detached |
| `doTargetCombatHealth(...)` | `pick.lua:13` | TFS native `luaDoTargetCombat`; Health is `compat.lua:314` alias | ✅ both globals; caster `0`/`nil` = environment |
| `Item.actionid` field (get) | `functions.lua` | `compat.lua` `__index` | ✅ next to `itemid` |
| `Tile:getGround()` Item userdata | `onUsePick` (`ground.actionid` / `:transform`) | `luaTileGetGround` | ✅ `body.ground_item` (was `ItemTypeRef` from type id) |

`Container:addItemEx` is still missing (loot after createItem-without-pos).

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
`getId`, `getActionId`, `getPosition`, `transform`, `decay`, `remove`, `getParent`, `addItem` (container/player), `isItem`, `isCreature`, `moveTo`, `getType`, `getStorageValue`, `setStorageValue`, `removeItem`, `sendCancelMessage`, `teleportTo`, `getName`, `hasFlag` (tile+player), `getTopDownItem`, `Position:sendMagicEffect`, `Position:moveUpstairs`. `Tile:getGround` is listed in the Gap 3 table (Item userdata).

## Gap 4 — Missing constants / globals

✅ **done 2026-08-14.**

| Symbol | Used by | Status |
|---|---|---|
| `SKILL_FISHING` (=6) + the `SKILL_*` enum family | fishing_rod | ✅ `register_skills` in `crates/tfs-rust-lua/src/combat_enums.rs` — `enums.h` `skills_t` / `luascript.cpp` `registerEnum(SKILL_FIST)`…`SKILL_LEVEL`. Not 772 timer-skill ids. |
| `actionIds` table (`pickHole`, `sandHole`, `destroyableStone`, …) | pick, `functions.lua`, puzzle/sandstone/blocking scripts | ✅ TVP placement: `data/global.lua` 4000–4005 (`destroyableStone=4004`). Injected by `inject_door_tables_from_global` (starts at `actionIds = {`). `actionids.lua` only merges TFS extras (`levelDoor`/`citizenship`) so the core scan does not replace the table. |

Already registered and OK: `CONST_ME_LOSEENERGY`, `CONST_ME_POFF`, `COMBAT_PHYSICALDAMAGE`, `TILESTATE_PROTECTIONZONE`, `RETURNVALUE_PLAYERISPZLOCKED`.

## Gap 6 — 772 parity numbers hardcoded in tool scripts

✅ **done 2026-08-14.**

Era-tuned numbers live in `MechanicsProfile` / `data/formulas/{772,1098}.lua`. Scripts keep the control flow and read `formulas.*`. The game VM execs the same era file via `inject_era_formulas` (after lib load, before `assert_required_data_globals`).

| Literal | Was | Now | Citation |
|---|---|---|---|
| 40% destroy chance, `-50` physical self-damage | `pick.lua` | `formulas.destroyableStone` | **TVP/TFS data pack** — not in 772 `moveuse.dat` `BEGIN "Picking"` (that section is `372→394` pick-hole + two position-locked quest rocks with `Damage(Null,User,1,40\|50)`). Frozen as TVP knobs. `MOVEUSE_CONDITION_RANDOM` is `random(1,100) <= n`. |
| Fishing success | TFS `min(max(10 + (skill-10)*0.597, 10), 50)` | 772: `formulas.fishingSuccess` → `TSkillProbe::Probe(80, 50)`; 1098: TFS linear clamp | **772:** `moveuse.dat` `TestSkill (User,Fishing,80,50)` + `crskill.cc:546` `TSkillProbe::Probe`. The 0.597 coefficient is **TFS-only** (hits 50% at skill 77); it is not in the decompile. |

`Container:addItemEx` is still missing (loot after createItem-without-pos).
