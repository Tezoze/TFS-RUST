# `actions/other` — 772 outcomes plan

**Scope:** `data/scripts/actions/other/*.lua` (20 files) load, run, and match **772 decompile outcomes** (TFS `Action()` domain, OTB item ids).
**Date:** 2026-08-15 (reaudit; remaining APIs for map/quests/tools). **Companions:** [tools-actions/README.md](tools-actions/README.md), [doors-actions-plan.md](doors-actions-plan.md).

Do **not** port `moveuse.dat` as an engine. Pattern: `food.lua` (772 numbers + cite in the script). 1098 TFS-only extras go behind `formulas.otherActions`. CipSoft TypeIDs ≠ OTB ids — scripts keep OTB.

`Random(n)` = `random(1,100) <= n` (`moveuse.cc:349-350`). Effects: 3 poff, 4 blockhit, 19 sound green, 22 yellow, 23 purple.

---

## Implementation order

1. ~~**E1** constants + tests + `emit-lua-defs --check`~~ — **done** (`42b9457`). Fluids / music / birdcage / used_lamp / create_bread / change_gold load.
2. ~~**E2** zero-thing target + `isHotkey`~~ — **done**. Stops no-target crashes.
3. ~~**E3** `ItemType:getDestroyId` / `getFluidSource` + **`destroyItem` 1/3 `transform`**.~~ — **done**.
4. ~~**E4** `addHealth`~~ + **E5** `say` + **E8** drunk stack + **rewrite `fluids.lua`**.
5. **Script pass** — food `>`, birdcage, waterpipe, music, create_bread, used_lamp, construction (**E7**), teleport, watch/cuckoo, change_gold gate, `formulas.otherActions`.
6. **E6** `showTextDialog` + learned-spell list + **rewrite `spellbook.lua`** (`GetSpellbook`).
7. **E9** `getFormattedWorldTime` — `watch.lua` (can land earlier).
8. **After this folder** — remaining actions APIs **R1–R5** (map chests + mintwallin `createTile`). E3 already covers tool `destroyItem`. Then G10 `decay(id)`, G12.

---

## Engine

| ID | Work | Where |
|---|---|---|
| **E1** ✅ | `FLUID_NONE…LEMONADE` 0–12 (`const.h:94-106`); `TALKTYPE_SAY` **1** / `MONSTER_SAY` **0x11** (`const.h:62,76`); `CONST_ME_SOUND_YELLOW/PURPLE/BLUE/WHITE` 22–25 (`const.h:32-35`). `CONDITION_PARAM_DRUNKENNESS` **55** (`enums.h:275`) so leftover `setParameter` does not fail load (E8 ignores the value). `ITEM_GOLD/PLATINUM/CRYSTAL_COIN` 2148/2152/2160 (`const.h:451`) — 1098 change-gold + `data/lib`. Sequential 772 fluids, not TFS colour-mapped subtypes. | `constants.rs`, `combat_enums.rs`. Shipped `42b9457`. |
| **E2** ✅ | No-target → TFS zero-thing table (`uid/itemid/actionid/type = 0`); pass `isHotkey` as 6th arg (`Action::executeUse` `callFunction(6)`). Hotkey pos is client `(0xFFFF,0,0)`. | `runtime.rs`, `lua_scope.rs`. |
| **E3** ✅ | `getDestroyId` / `getFluidSource` from `items.xml` `destroyto` / `fluidsource`. | `script_context.rs`, `game_world_script.rs`, `userdata/item_type.rs`. |
| **E4** ✅ | `Player:addHealth(n)` — HP clamp, same shape as `addMana`. | `LuaMutation` + applier. 772 `Heal` in `DrinkPotion`. |
| **E5** | `Player:say(text[, type])` → `broadcast_creature_say_viewport`, **not** `player_say` (that parses spells). | 772 `Talk` (`TALK_SAY=1`); TFS `luaPlayerSay`. |
| **E6** | `Player:showTextDialog(itemId, text)` → existing `send_text_window_item` (`0x96`). `hasLearnedSpell(name)` + list of **instant** defs (incl. rune-conjure instants). **Do not** implement TFS `getInstantSpells` = `canCast` (vocation dump). | 772 `SendEditText` → `GetSpellbook`. |
| **E7** | `Tile:getHouse()` → `nil` or truthy. **Never return `0`**. | `userdata/tile.rs`. 772 `IsHouse(Obj1)`. |
| **E8** | Drunk stack: if level `< 5` then `level+1`; Count=MaxCount=120; `base.drunkenness` = Cycle 1–5. Each ProcessSkills second: Count--; on 0, Cycle toward 0, Count=120; at 0 remove. Stagger already uses `max(7-level,1)`. Spell-drunk (`idle_stimulus`) stays Power-gated; beer/wine uses this stack. | `condition.rs`, `process_skills.rs`. `moveuse.cc:1776-1782`, `crskill.cc:176-193`. |
| **E9** | Inject `getFormattedWorldTime` from `data/global.lua` (`getWorldTime` already bound). **Do not** full-`dofile` `global.lua` this pass — it `dofile`s `lib.lua` and would double-load. | `actions.rs` inject chunk. |

**Skip for this folder:** G3 `item.type` (use `getFluidType()`), G6 `getReturnMessage`, G9 item `queryAdd`, `setEarliestSpellTime`, G10, G12. **G4 `addItemEx`** is not needed by the `fluids.lua` rewrite (spill `2016` in place); it **is** remaining for `map/quests.lua` — see R1–R5.

---

## Scripts

Cite `moveuse.cc` / `moveuse.dat` / `objects.srv` in a one-line header like `food.lua`.

| File | Change |
|---|---|
| `food.lua` | Keep nutrition×12 and `"You are full."` (`FEDUP`). Change `>= 1200` to **`> 1200`** (`Cur + Add > Max`, `moveuse.cc:1842`). Exact cap 1200 is allowed. |
| `birdcage.lua` | Empty iff `random(100)<=1 and random(100)<=10`. Else effect 22. (`moveuse.dat` Fun 2976) |
| `waterpipe.lua` | `random(100)<=90` poff on **item**, else **player**. Id **2093 only**. (`2974`) |
| `music.lua` | Didgeridoo chance **10** (else poff). Cornucopia **3957 and 2369**: 95% keep + 10 grapes; else 9 grapes + `transform(2681)`. Drop bongo `3951` / war drum `3953` unless `extraInstruments`. Piano 50 / plain green stay. |
| `fluids.lua` | Rewrite to `UseLiquidContainer`: fill (`LIQUIDSOURCE`) → pour (empty dest container) → drink **iff dest is self** → else spill `2016`. Drink: beer/wine `addCondition(drunk)`; slime `Damage(200, POISON_PERIODIC)` = cycle 200 / count 3 / max 3; mana **50–150**; life **25–75**; lemonade `"Mmmh."`; milk/default `"Gulp."`; urine `"Urgh!"`; none no-op. No magic-blue, no exhaust. |
| `change_gold.lua` | Early `return` before `Action()` if `not formulas.otherActions.changeGold`. 772: `false`. |
| `create_bread.lua` | Millstone **wheat only** (`2694`), `transform` in place (`Change` 3605→3603). Flour+water → `transform` dough in place. Dough+oven → `Delete` dough, `Create` bread on oven. |
| `used_lamp.lua` | Oil (`FLUID_OIL=7`) → empty vial, `transform` unlit lamp, **no** `decay()`. (`Fun` MultiUse 2916+2874) |
| `construction_kits.lua` | House: transform + effect 3. Else **effect 4, no text**. Drop floor/house messages. (`Furniture Parcels`) |
| `spellbook.lua` | Learned-spell window for **2175 only**. See Spellbook below. |
| `watch.lua` | `"The time is " .. getFormattedWorldTime() .. "."` (`INFORMATION_TIME`). Ids: pendulum 1728–1731, watch 2036, cuckoo **1873–1877 and 1881**. **Drop sundial 3900** (no 772 `Information` type). |
| `decayto.lua` | Remove cuckoo 1873–1876 (expire already in `items.xml`; use shows time). Keep lamps/torches/candelabra. |
| `teleport.lua` | Remove script PZ-lock cancel. Keep `moveUpstairs` / `z+1` (TFS shape of `MoveRel`). |
| `functions.lua` `destroyItem` | Always poff. `random(1,3)==1` → empty container, `target:transform(destroyId)` (`UseWeapon` `Change`). |
| `pumpkinhead.lua` | E2 guard only. Keep transform+decay (`Fun` MultiUse 2977+2917; Expire 3000). |
| `destroy.lua` | Unchanged; helper does the roll. |

`data/formulas/772.lua`: `formulas.otherActions = { changeGold = false, extraInstruments = false, spellbookMagicLevel = false }`
`1098.lua`: `changeGold = true`, `extraInstruments = true` (bongo / war drum / waterpipe 2099), `spellbookMagicLevel = true`.

**Spellbook:** `UseAnnouncer` case 4 → `SendEditText` (`moveuse.cc:1947-1948`). `INFORMATIONTYPE==4` calls `GetSpellbook` (`sending.cc:1102-1112`, `magic.cc:3830-3901`) — **not** item `TEXTSTRING`. Filter `SpellKnown` / `player_spells` (not TFS `canCast`). Group by **level** 1..max only. Include rune-**conjure** instants (`ad,ura gran` etc. are `SPELL_INSTANT` in the data pack). Skip level-0 GM/house spells. GM `ALL_SPELLS` does **not** fill an empty book. Words: strip commas, glue first two syllables (`GetSpellString`: `exura`, `exura gran`). Line: `"  " .. words .. " - " .. name .. ": " .. mana` (`": %d"`). Specials: Summon/Convince `"var"`, Berserk `"4*Level"` (not `80%`). Blank line after each level group. **OTB 2217** (`objects.srv` 3101) is `Text` only — fontsize-1 stored text, **not** `GetSpellbook`. Do not register 2217 on this action. 1098 ML groups: `spellbookMagicLevel`.

**Leave:** `blueberry_bush.lua` (3 berries + decayto 2785), `snowheap.lua`, `trap.lua` (PZ: poff only; else toggle), `doors.lua`, `transforms.lua` (`CHANGEUSE` stand-in).

---

## Status

Load probe 2026-08-15 (post-E1): **20/20 load**. 772 column is decompile match, not “script runs.” `change_gold` still **registers** coins until step 5.

| File | Loads | 772 | Ship |
|---|---|---|---|
| `blueberry_bush.lua` | ✅ | ✅ | leave |
| `snowheap.lua` | ✅ | ✅ | leave |
| `trap.lua` | ✅ | ✅ | leave |
| `doors.lua` | ✅ | ⚠️ | leave (doors plan) |
| `transforms.lua` | ✅ | ⚠️ | leave (`CHANGEUSE` stand-in) |
| `food.lua` | ✅ | ❌ `>=` vs `>` | step 5 |
| `teleport.lua` | ✅ | ⚠️ | drop PZ cancel |
| `pumpkinhead.lua` | ✅ | ⚠️ | E2 only |
| `decayto.lua` | ✅ | ⚠️ | drop cuckoo |
| `watch.lua` | ✅ | ⚠️ ids | E9 + cuckoo; drop 3900 |
| `waterpipe.lua` | ✅ | ❌ 33/67 vs 90/10 | step 5 |
| `birdcage.lua` | ✅ | ❌ 1% vs 0.1% | step 5 |
| `music.lua` | ✅ | ❌ didgeridoo/cornucopia/extras | step 5 |
| `create_bread.lua` | ✅ | ⚠️ | E2 + step 5 |
| `used_lamp.lua` | ✅ | ⚠️ | E2 + step 5 |
| `construction_kits.lua` | ✅ | ⚠️ | E7 + step 5 |
| `destroy.lua` | ✅ | ⚠️ helper 1/3 | E3 shipped; script pass unchanged |
| `spellbook.lua` | ✅ | ❌ filter/format | E6 |
| `fluids.lua` | ✅ | ❌ | E3–E5, E8 + rewrite |
| `change_gold.lua` | ✅ | ❌ still registers | step 5: do not register on 772 |

---

## Decisions

| Topic | Do |
|---|---|
| Spellbook | Learned-spell text window (`GetSpellbook` / `SpellKnown`). 2175 only. Not empty item text. Not TFS `canCast`. |
| Food cap | `>` 1200, not `>=`. Message already `"You are full."` |
| Cuckoo | `InformationType=2` + `Expire` (`objects.srv` 2660–2668). Use = time. Decay is automatic, not `onUse` toggle. |
| Sundial 3900 | Not a 772 information clock. Drop from `watch.lua`. |
| `Player:say` | Viewport broadcast, no parser. |
| Coin use | Do not register on 772. |
| Potion exhaust | Delete `setEarliestSpellTime` + magic-blue. |
| Fluids extras | Rewrite to `UseLiquidContainer`; drop TFS `queryAdd` / `addItemEx` / stairs-redirect. |
| Drunk | Engine stack 1–5 × 120s. Lua only `addCondition`. |
| Destroy | 1/3 + `transform`. |
| E9 | Inject `getFormattedWorldTime` only; no full `dofile` of `global.lua`. |

---

## Tests

- `other_scripts_load_and_register` — 20 files load; 772 does **not** register gold coins; ids 2095 / 2175 / 486 registered; 2217 **not** on spellbook; 3900 **not** on watch; 1877/1881 **are**.
- E1 ✅: `FLUID_WATER==1`, `TALKTYPE_SAY==1`, `CONST_ME_SOUND_YELLOW==22`, `ITEM_GOLD_COIN==2148` (`e1_other_action_constants_unblock_fluids_and_change_gold_load` + `lua_defs` snapshot).
- Zero-thing: `target.uid==0 and target.itemid==0`; `isHotkey` boolean. ✅ `e2_no_target_is_zero_thing_table_and_is_hotkey_boolean`
- `getDestroyId` / `getFluidSource` against known `items.xml` rows. ✅ `destroyto_and_fluidsource_match_items_xml_rows` + `e3_get_destroy_id_and_fluid_source_from_known_xml_rows`
- `destroyItem` 1/3 uses `transform`. ✅ `e3_destroy_item_uses_one_in_three_transform`
- `addHealth` clamp. ✅ `e4_add_health_clamps_to_effective_max_and_zero` + `e4_add_health_clamps_to_equipment_bonus_max`
- Drunk: beer → level 1; second → 2 and count 120; cap 5; after 120 rounds level −1.
- Mana 50..=150, life 25..=75. Lemonade `"Mmmh."`, milk `"Gulp."`.
- Food: remaining 1188 + blueberry (1×12) allowed (sum == 1200); remaining 1189 + blueberry rejected.
- Spellbook: learned Light Healing under `Spells for Level 9` as `exura - Light Healing: 25`; unlearned Berserk absent; learned Berserk shows `4*Level`; no `Spells for Magic Level`.
- `emit-lua-defs --check` green.

```sh
rtk cargo check
rtk cargo test -p tfs-rust-lua actions::tests
rtk cargo test -p tfs-rust-lua --lib lua_defs
rtk cargo test -p tfs-rust-core --lib process_skills
rtk cargo run -p tfs-rust-lua --bin emit-lua-defs -- --check
rtk cargo clippy --all-targets
```

---

## Remaining actions APIs (map / quests / tools)

Not this folder’s script pass. Inventory 2026-08-15 against `data/scripts/actions/{map,quests,tools}/` + `puzzle_switches.lua` + `onUseQuest` / `destroyItem` in `functions.lua`.

E2 (nil `target`) and E3 (`getDestroyId`) already unblock quest kits and crowbar/machete/pick `destroyItem`. Do not redo them here.

| ID | Missing | Blocks | Domain |
|---|---|---|---|
| **R1** | `ItemType:getArticle` / `getPluralName` / `getWeight([count])` / `isContainer`; **real** `getName` (today `"item_{id}"`) | `onUseQuest` found-text + capacity (`map/quests.lua`) | TFS `luaItemTypeGetArticle` / `GetPluralName` / `GetWeight` (`weight * max(1,count)`) / `IsContainer` / `GetName` |
| **R2** | `Game.createItem` must return **Container** userdata when the type is a container (TFS `setItemMetatable`) | `reward:addItem(...)` filling bag/backpack `content`. Rust always pushes `ItemRef`; `Item` has no `addItem` | `luaGameCreateItem` → `pushUserdata<Item>` + `setItemMetatable` |
| **R3** | `Player:addItemEx(item[, canDropOnMap])` → `RETURNVALUE_*` (tools **G4**). Same cylinder move for `Container:addItemEx` / `Tile:addItemEx` (lib loot, 1098 `change_gold`) | `onUseQuest` `player:addItemEx(reward)`; `data/lib/core/container.lua` loot | `luaPlayerAddItemEx` / `luaContainerAddItemEx` / `luaTileAddItemEx`. Detached item only (`VirtualCylinder` parent) |
| **R4** | Inject `getPlayerFlagValue(cid, flag)` = `Player(cid):hasFlag(flag)` (compat one-liner; **do not** load `compat.lua`) | `onUseQuest` infinite-capacity skip (`PlayerFlag_HasInfiniteCapacity` already bound) | `compat.lua:460` |
| **R5** | `Game.createTile(position[, isDynamic])` (also `x,y,z` form) | `map/thais/mintwallin_bridge_lever.lua` only | `luaGameCreateTile` — get-or-create tile, return `Tile` |

**Related, not a new named method:** `Item:getParent()` tile arm returns a `{x,y,z}` table, not `Tile` userdata — `parent:isContainer()` / `parent:addItem` on floor items (`quests/ice_pick.lua`, `tools/fishing_rod.lua`) error. Push `Tile` like TFS.

**Already bound — do not treat as remaining:** `doRelocate` (injected), `doTargetCombatHealth`, `Game.createItem`/`createMonster`, lib `Game.isItemInPosition` / `removeItemInPosition` / `transformItemInPosition` / `sendMagicEffect`, `item:transform`/`decay()`/`remove`/`moveTo`, `player:teleportTo`/`sendTextMessage`/`getItemCount`/`removeItem`/`addSkillTries`/`getEffectiveSkillLevel`/`isPzLocked`/`getFreeCapacity`/`hasFlag`, `Tile:getGround`/`getItemById`/`getBottomCreature`/`relocateTo`, `Position:moveUpstairs`, `Container:addItem`.

**`other/`-only remaining (E5–E9, not map/quests/tools):** `say`, `getHouse`, `showTextDialog`, `getFormattedWorldTime`, `Item:decay(id)`. **E1 shipped:** `FLUID_*`, `CONST_ME_SOUND_YELLOW…WHITE`, `TALKTYPE_SAY`/`MONSTER_SAY`, `ITEM_*_COIN`, `CONDITION_PARAM_DRUNKENNESS`. **E3 shipped:** `getDestroyId`, `getFluidSource`. **E4 shipped:** `addHealth`.

---

## Deferred

- **R1–R5** — remaining actions APIs above (after E1–E9).
- **G10** `Item:decay(id)` — map uses no-arg `decay()`; only `other/music.lua` passes an id.
- **G12** — delete `data/items/#items.lua` and `data/actions/**`.
- **1098 spellbook** — vocation dump + magic-level groups behind `formulas.otherActions.spellbookMagicLevel`.
- **Doors `GetInfo` strings** — already shipped; not this pass.
- Full `dofile('data/global.lua')` replacing `inject_door_tables_from_global`.

772 use dispatch (`operate.cc:2531-2569`): container → chest → liquid → food → write → information → rune → doors → weapon+destroy → CHANGEUSE → USEEVENT (`HandleEvent`, first `moveuse.dat` match) → fontsize-1 text.
