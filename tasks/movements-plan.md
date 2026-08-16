# Movements — 772 outcomes plan

**Scope:** `data/scripts/movements/**` (133 files) load, fire, and match **772 decompile outcomes** (TFS `MoveEvent()` domain so the data pack works).
**Date:** 2026-08-16. **Companions:** [doors-actions-plan.md](doors-actions-plan.md) Phase 6 (StepIn/Out plumbing), [other-actions-plan.md](other-actions-plan.md) (shared Lua APIs).

Do **not** port `moveuse.dat` Collision as an engine. Pattern: keep TFS `MoveEvent()` + OTB/aid map scripts; put 772 numbers in native field/trap paths (`magic_field.rs`, `trap.lua` rewrite). CipSoft TypeIDs ≠ OTB ids.

**Already working (do not redo):** XML equip abilities (lesson 181); native `onStepInField` / `AddItemField` (lesson 276); door StepOut auto-close + mutation scope + deferred packets (lessons 284–289); load scan of all 133 files; **M1** StepIn/Out aid lookup at fire (lesson 358).

---

## Three layers

| Layer | Source of truth | This folder |
|---|---|---|
| **Outcomes** | 772 `CollisionEvent` / `SeparationEvent` (`moveuse.cc`, `moveuse.dat` Trap Damage) | Field ticks, trap transform-then-damage, peaceful-only fields, door ClearField+Change |
| **Domain** | TFS `MoveEvent()` + `data/scripts/movements/**` | `:id` / `:aid` / `:tileItem` / `onStepIn` / `onAddItem` so OTBM aids keep working |
| **Implementation** | Idiomatic Rust | `MoveEventsRegistry` lookup + `LuaMutation` scope; no `*_772` forks |

Conflict: when TFS Lua damages/transforms differently from 772 Collision, **772 wins** for the 772 profile. Map quest/depot/vocation tiles have no 772 global engine equivalent — leave as Lua.

---

## Current state (2026-08-16 audit)

| Piece | Status |
|---|---|
| Load `scripts/movements/**` (133 files) | **Done** — warn-on-fail; drain after all files |
| `MoveEvent()` `:id` / `:aid` / `:type` / `:slot` / `:level` / `:register` | **Done** — fire uses `get_event` (aid then itemid) |
| `:tileItem` / `:uid` / `:position` | **`:tileItem` stub bound** (load only; ITEMTILE remap is M2). `:uid` / `:position` skipped |
| StepIn/StepOut fire after walk packets, mutation scope, all tile items | **Done** — uid skipped → aid → itemid |
| AddItem/RemoveItem cylinder fire | **Call site exists**; **wrong Lua args**; no tileItem/aid; no mutation scope; requires actor |
| Equip XML native abilities | **Done** |
| Native magic fields (1487/1490/1491/campfires) | **Done** — match 772 Trap Damage via `items.xml` |
| `get_by_aid` | **Done** — used by `get_event` |

Corpus: **133** files, **196** `MoveEvent()` regs. **119** `onStepIn`, **15** `onStepOut`, **59** `onAddItem`, **3** `onRemoveItem`. **126** `:aid` (`map/` + root), **7** `:id` (`other/` only). **0** `:uid` / `:position`. Equip/DeEquip: XML only.

| Combo | Files | Unblocked by |
|---|---|---|
| StepIn only | 59 | **M1 done** |
| StepIn + AddItem | 46 | **M1 walk done** / **M2 drop** |
| AddItem only | 11 | **M2 only** (scarab coins, altars) |
| StepIn + StepOut | 13 | **M1 done** |
| StepOut only | 1 | **M1 done** (`closing_doors.lua`) |
| RemoveItem (± AddItem) | 3 | M2 (`demon_scroll`, crate puzzle, sandstone) |

`data/movements/scripts/*.lua` (12 XML-era files) are **not loaded** — `movements.xml` has no `script=` attrs; Rust XML loader only takes `function=`. No dual-register with revscripts. XML `function="onStepInField"` stays native (lesson 276).

---

## Implementation order

1. **M1 Done** aid lookup at fire (uid→aid→id) for StepIn/Out — unblocks walk-on-aid (~126 files). Does **not** unblock 11 AddItem-only scripts.
2. **M2** AddItem/RemoveItem TFS signature + `:tileItem(true)` ITEMTILE remap + mutation scope + sibling tile items — drops onto aid tiles + 11 AddItem-only.
3. **M3** `Player:setTown` + `Creature:getMaster` (home tiles + bear-trap summons).
4. **M4** 772 trap/field script pass (`trap.lua`, `fields.lua`, peaceful fields).
5. Tests with each slice. Then map scripts run without further engine work.

---

## Engine

| ID | Work | Where | Refs |
|---|---|---|---|
| **M1 Done** | Fire StepIn/Out with TFS `getEvent(Item*)` order: uniqueid → actionid → itemid. Snapshot `action_id` in `tile_move_event_items`. `get_event` uses `get_by_aid`. First registered event per key. No-op `:tileItem` so dual StepIn+AddItem files load. | `walk/mod.rs` `tile_move_event_items`; `lua_event_dispatcher.rs` `dispatch_move_step`; `lua_scope.rs` `fire_creature_step_events`; `move_events.rs` `get_event` | TFS `movement.cpp:366-397`. 772 map tiles are Collision-by-coord; OT pack uses **aid** — domain stays TFS aid. |
| **M2** | `executeAddRemItem(moveitem, tileitem, pos)`. `:tileItem(true)` remaps to ITEMTILE (sibling tile item, not the moved item). Iterate other items on the tile. Actor optional. Wrap in mutation + ScriptContext like step events. | `runtime.rs` ctor + `call_move_item`; `move_events.rs` kind or flag; `game_world_item_move.rs`; `lua_event_dispatcher.rs` | TFS `onItemMove` `movement.cpp:477-515`, `registerLuaEvent` `243-255`, `executeAddRemItem` `1017-1036` |
| **M3** | `player:setTown(Town)` mutation; `creature:getMaster()` → summoner Creature/Player or nil. | `userdata/player.rs`, `userdata/creature.rs`, `LuaMutation` | TFS `luaPlayerSetTown` / `luaCreatureGetMaster`. 772 temple Change is content; OT home tiles call `setTown`. |
| **M4a** | Peaceful / Meaning-harmless fields (OT 1500–1504): skip players and summons (`!IsPeaceful` in 772). | `magic_field.rs` | `moveuse.dat` 2131–2135; `crmain.cc:900`; `crnonpl.cc:2295` |
| **M4b** | Searing 1506/1507: 772 is `Damage(64,10)+Damage(4,300)`. Prefer `items.xml` field attrs + native; **remove or no-op `fields.lua`** so we do not double-hit. | `items.xml` / `fields.lua` | `moveuse.dat:1516-1520` |
| **Skip this pass** | `:uid` / `:position` maps — unused in this pack. `Position.__eq` — polish (`doRelocate` identity compare). | — | `movement.cpp:292-311`, `417-427` |

**Lookup contract (M1 Done):** an item with actionid 3052 must fire the aid-3052 script even if its type also has an `:id()` trap. C++ skips itemid when that kind’s aid/uid list is non-empty; aid *set* with no event of this kind still falls through to itemid (lesson 358).

**Return values:** Step/Add/Remove Lua `false` does **not** undo the move (TFS discards it; walk already committed). Scripts teleport/relocate themselves. Equip `false` still blocks dress.

---

## 772 vs TFS Lua (script pass)

Cite `moveuse.cc` / `moveuse.dat` in script headers like `food.lua`.

| File | 772 | Change |
|---|---|---|
| `other/trap.lua` slits 1510 | 2145 `Change(2146)` only; damage is on **blades/spikes** `Damage(1,60)` | First step on holes = transform only; 60 physical on 1511/1513. **No PZ skip** on Collision. |
| `other/trap.lua` bear 2579 | 3482 `!IsPeaceful` → Change+30 physical; else Change+poff. PZ only on **arm** (action) | Keep `dontDamagePlayers`. Drop step-in PZ check. Needs **M3 `getMaster`**. |
| `other/trap.lua` maw 4208 | 3944 Change+`Damage(2,30)` instant poison | Leave numbers; type stays earth/poison instant (not periodic). |
| `other/fields.lua` | 2137 `Damage(64,10)+Damage(4,300)` | Native field attrs; disable Lua to avoid double 300. |
| `other/closing_doors.lua` | `SeparationEvent` ClearField then Change (`moveuse.cc:2327-2339`) | **Leave** (already 772-shaped). |
| `other/level_doors.lua` | Content gate | **Leave**. |
| `other/tiles.lua` depot | 772 depot is **per-coord** Collision LoadDepot | **Leave** OT switch-tile Lua. Depot APIs already bound. |
| `other/transform.lua` holes | Collision `MoveTopRel(0,0,1)+Change` | **Leave Lua**; watch double relocate vs OTB floorchange. |
| `other/remove.lua` 1497–1499 | Step-in delete | Leave `:id()` StepIn (tileItem on StepIn is a TFS no-op). |
| Magic fields in `movements.xml` | Trap Damage 2118/2121/2122 = 70+20 fire, poison 100, energy 25+30 | **Native only.** Do not also run Lua. |
| Peaceful fields 1500–1504 | `!IsPeaceful` only | **M4a** native gate. |
| Lava / swamp dunk | Liquid Deletions delete **items**; no creature lava DoT | Keep trashholder; do **not** port legacy `data/movements/scripts/trap.lua` lava-500. |
| Teleports / stairs | Engine `TELEPORT*` / `MoveTopRel` | Keep native `tile_specials`; quest teles stay Lua. |
| All `map/**` + `blocking_tile.lua` + sandstone | No 772 global equivalent (OTBM aids) | **Leave Lua** once M1–M3 fire. |

`data/formulas/772.lua`: **no** movement damage knobs this pass (amounts live in `moveuse.dat` / `items.xml`). Optional later: peaceful-field policy flag for 1098.

---

## Lua APIs

**Bound — do not treat as remaining:** `isPlayer`, `getPlayer` (lib), `teleportTo`, `getVocation():getId()`, `isPremium`, `getStorageValue`/`setStorageValue`, `getLevel`, `say`, `sendTextMessage`, `addCondition`, `getGroup():getAccess()`/`getMaxDepotItems()`, `isInGhostMode`, `getDepotLocker`, `getItemHoldingCount`, `getDirection`/`getNextPosition`, `transform`/`decay`/`remove`, `getFluidType`, `getActionId`/`getUniqueId`, `Tile`/`getItemById`/`getItemByType`/`hasFlag`, `Game.sendMagicEffect`/`createItem`/`createMonster`/`clearField`/`isItemInPosition`/`removeItemInPosition`/`transformItemInPosition`/`getStorageValue` (lib table), `doRelocate`, `doTargetCombat`/`doTargetCombatHealth`, `Town()`, `Player(creature)`, `table.contains` (injected), `ItemType:isMovable`.

| ID | Missing | Blocks |
|---|---|---|
| **M3a** | `Player:setTown(town)` | 9 `*_home.lua` temple tiles |
| **M3b** | `Creature:getMaster()` | `trap.lua` bear-trap summons |
| **M2a** | `MoveEvent:tileItem(bool)` ITEMTILE remap (method stubbed in M1 for load) | 61 AddItem/RemoveItem scripts |

---

## Tests

- **M1 Done:** tile item with `action_id=3052` fires aid callback, **not** a same-type `:id()` trap; item with aid 0 still hits `:id()`.
- **M1 Done:** load 133 files, non-zero `by_aid`, live `get_event` for rookgaard 3051/3052 vs trap 1510.
- M2: drop item on aid-3052 tile with `tileItem(true)` → `onAddItem(moveitem, tileitem, pos)` and `doRelocate` under mutation scope; no actor still fires.
- M2: `onAddItem` Lua sees `tileitem.itemid` of the trap/tile, not the dropped item.
- M3: `setTown(Town("Thais"))` updates town id; `getMaster()` nil for wild monster, player for summon.
- M4: 1510 step transforms without damage; 1513 deals 60 physical in PZ; bear trap does not damage player/summon.
- Native field: 1487 still init+DoT once (no Lua double).
- Peaceful field 1500: player walks through without condition; wild monster takes hit.
- `emit-lua-defs --check` if `setTown` / `getMaster` / `tileItem` are recorded. (`tileItem` stub in `engine.d.lua` from M1.)

```sh
rtk cargo check
rtk cargo test -p tfs-rust-lua --lib move_events
rtk cargo test -p tfs-rust-core --lib magic_field
rtk cargo run -p tfs-rust-lua --bin emit-lua-defs -- --check
rtk cargo clippy --all-targets
```

---

## Decisions

| Topic | Do |
|---|---|
| Map scripts | Keep TFS Lua + OTBM aids. 772 Collision-by-coord is not an OT map. |
| Aid vs itemid | C++ order uid→aid→id. Aid **hit** for that kind ⇒ skip itemid. Aid set with no event of this kind ⇒ fall through (lesson 358). |
| `return false` on StepIn | Does not cancel walk (TFS). Scripts `teleportTo(fromPosition)` / `doRelocate`. |
| Fields | Native `magic_field.rs` is the 772 path. Lua only for IDs without field attrs. |
| Traps | Rewrite `trap.lua` to 772 transform/damage order; stay Lua. |
| Depot tiles | Leave `tiles.lua`. |
| `moveuse.dat` engine | Do not port. |
| uid/position maps | Skip until a script needs them. |

---

## Deferred

- `:uid` / `:position` registry maps.
- `Position.__eq` for `doRelocate(fromPos == toPos)`.
- 1098-only extras behind `formulas` if a later audit finds any.
- Full `data/movements/scripts/` XML-era Lua (revscripts replace them).
- G12 delete `data/movements/**` XML after revscripts fully own equip+step.
