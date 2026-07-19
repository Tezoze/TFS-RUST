# Doors + Action System — Implementation Plan

**Status:** Not started  
**Date:** 2026-07-19  
**Goal:** Run `data/scripts/actions/other/doors.lua` (and sibling actions such as `food.lua`) end-to-end on a **TFS-style domain**, with **772 house/door outcomes** where they diverge, implemented as **idiomatic Rust**.

**Primary scripts**
- `data/scripts/actions/other/doors.lua` — open/close, keys, quest/level, house door IDs
- `data/scripts/movements/other/closing_doors.lua` — step-out auto-close
- `data/scripts/movements/other/level_doors.lua` — step-in level gate (when enabled)
- `data/global.lua` — door ID tables (`keys`, `openDoors`, `closedDoors`, …)

**Shared unblocker:** the same `Action()` pipeline also enables every other `data/scripts/actions/**` script (food, fluids, levers, etc.).

---

## 1. Three-layer framing

| Layer | Source of truth | What we match |
|-------|-----------------|---------------|
| **Outcomes** | 772 decompile + TVP `gameserver` use path | Who may open a house door; key/keyhole match; quest/level gate messages; transform ±1 / ±2 |
| **Domain shape** | TFS revscripts + `data/` | `Action()` / `MoveEvent()` self-register; door ID tables; `onUse` / `onStepOut` contracts |
| **Implementation** | Rust idioms | Typed `ItemId` registries, `LuaMutation` apply, no `*_772` forks; house access as native gate before Lua |

**Conflict rule:** data-pack Lua API shape stays TFS. House **access checks** stay native (TFS `Door::canUse` / 772 equivalent), not reimplemented inside `doors.lua`.

---

## 2. Current state (2026-07-19)

| Piece | Status |
|-------|--------|
| Door ID tables in `data/global.lua` | Present |
| `doors.lua` / `closing_doors.lua` content | Present |
| `Action()` userdata + `:id` / `:register` | **Missing** |
| Load `data/scripts/actions/**` | **Missing** |
| `onUse` dispatch from player use / use-with | **Missing** (`player_use_item*` = containers / floor teleports only) |
| `item:getId()` → server type id | **Wrong** (returns SlotMap key) |
| `item:getAttribute` / door+key attr constants | **Missing** |
| `player:getStorageValue` | **Missing** |
| `Tile:getCreatures` / `getTopVisibleThing` / `getCreatureCount` | **Missing** |
| Native house `Door::canUse` | **Missing** (`HouseManager::is_invited` only) |
| `MoveEvent` StepIn/StepOut + load `movements/**` | **Missing** (equip/deequip only today) |
| `openLevelDoors` / `openQuestDoors` in `global.lua` | **Commented out** but referenced by closing/level scripts |

**C++ references**
- Domain: `src/actions.cpp` (`Actions::registerLuaEvent`, `internalUseItem` → `executeUse`), `src/house.cpp` `Door::canUse`
- Wire/use packet path: TVP `gameserver` use handlers; Rust `container_ui.rs` / player use entry points
- 772 outcomes: house/door access + field-on-tile close behavior as needed

---

## 3. Phases

### Phase 0 — Data-pack hygiene (cheap, do early)

**Goal:** Scripts that load later do not nil-crash on missing tables.

| Task | Detail |
|------|--------|
| 0.1 | Uncomment or redefine `openLevelDoors` / `openQuestDoors` in `data/global.lua` to match closed counterparts (IDs +1 from closed quest/level sets, as TFS expects) |
| 0.2 | Confirm door tables cover OTB IDs used on the active map (spot-check common door types) |

**Done when:** `closing_doors.lua` / `level_doors.lua` can parse without `nil` table errors once MoveEvents load.

---

### Phase 1 — Action pipeline (shared unblocker)

**Goal:** `local a = Action(); a:id(n); function a.onUse(...); a:register()` works; use packet reaches Lua.

| Task | Detail | Refs |
|------|--------|------|
| 1.1 | `Action` userdata + registry (mirror `TalkAction` / Channel self-register pattern) | `talkaction.cpp`, `chat-system-plan.md` CH-4 pattern |
| 1.2 | Methods: `:id`, `:aid` (if needed), `:register`, store `onUse` function | `actions.cpp` `registerLuaEvent` |
| 1.3 | Startup: scan/load `data/scripts/actions/**/*.lua` (after `global.lua`) | TVP/TFS script loader order |
| 1.4 | Hook player **use** and **use-with** → look up action by item type (then actionid) → call `onUse` | `Actions::useItem` / `internalUseItem` |
| 1.5 | Return semantics: `true` = handled (skip further native); `false` = fall through | TFS `executeUse` |
| 1.6 | Fix `Item:getId()` → **server type id**; expose `item.itemid` field for TFS script style | `luascript.cpp` `pushThing` |
| 1.7 | Smoke: register a tiny test action **or** `food.lua` eat path | `data/scripts/actions/other/food.lua` |

**Done when:** Using a registered item ID invokes Lua `onUse` and can mutate/transform/remove via existing `LuaMutation` paths.

**Out of scope for Phase 1:** door-specific attrs, house gate, MoveEvents.

---

### Phase 2 — Basic door open / close / locked

**Goal:** Click closed → open (+1); click open → close (−1); locked → message only.

| Task | Detail |
|------|--------|
| 2.1 | Ensure `item:transform` + map spectators work for door type changes (block/unblock walk) |
| 2.2 | `Tile:getCreatures()` — push occupants before close |
| 2.3 | `item:getPosition()` returns `Position` userdata (not `(x,y,z)` tuple) so `+ offset` works |
| 2.4 | `Tile:getItemByType(ITEM_TYPE_MAGICFIELD)` already present — verify field removed on close |
| 2.5 | Locked branch: `sendTextMessage(MESSAGE_INFO_DESCR, "It is locked.")` (APIs exist) |
| 2.6 | Live smoke: normal + extra doors from `openDoors` / `closedDoors` / `openExtraDoors` / `closedExtraDoors` |

**Done when:** Non-house, non-quest doors open/close; locked doors refuse with text; creature on door tile is shoved aside when closing.

---

### Phase 3 — Keys (use-with)

**Goal:** Key on door toggles locked ↔ closed ↔ open per `doors.lua` transform rules.

| Task | Detail |
|------|--------|
| 3.1 | Use-with dispatch reaches Action with `target` = door item |
| 3.2 | `target:isItem()`, `target.itemid` |
| 3.3 | `Tile(toPosition):getTopVisibleThing()` |
| 3.4 | `getAttribute` / `hasAttribute` for key number ↔ keyhole |
| 3.5 | Register constants (or string custom-attr aliases): `ITEM_ATTRIBUTE_KEYNUMBER`, `ITEM_ATTRIBUTE_KEYHOLENUMBER` |
| 3.6 | Map/OTBM must populate keyhole attrs where keys are used (content/map issue if missing) |

**Done when:** Matching key locks/unlocks; mismatch shows "The key does not match."

---

### Phase 4 — Quest + level doors

**Goal:** `closedQuestDoors` / `closedLevelDoors` branches work; optional step-in scripts later.

| Task | Detail |
|------|--------|
| 4.1 | Decide authority: revscript custom attrs (`DOORQUESTNUMBER` / `DOORQUESTVALUE` / `DOORLEVEL`) **vs** legacy `actionid` scripts under `data/movements/scripts/` |
| 4.2 | Prefer revscript attrs for `doors.lua`; keep legacy scripts available if map still uses actionids |
| 4.3 | `player:getStorageValue` (+ set if other scripts need it) |
| 4.4 | `player:getGroup():getAccess()` already exists — GM bypass |
| 4.5 | On success: `transform(+1)` + `teleportTo(toPosition, true)` |
| 4.6 | Fail messages: sealed / "Only the worthy may pass." |

**Done when:** Configured quest/level doors open for eligible players only.

---

### Phase 5 — House doors (native gate + Lua transform)

**Goal:** House door IDs in `openHouseDoors` / `closedHouseDoors` respect ownership/guest lists.

| Task | Detail |
|------|--------|
| 5.1 | Door identity on items (`door_id` / house linkage from map load) |
| 5.2 | Native **before Lua**: `canUse(player)` — owner/subowner **or** per-door access list | `house.cpp` `Door::canUse`, `actions.cpp` pre-check |
| 5.3 | Wire lists from DB (`IOMapSerialize` house door lists) into runtime |
| 5.4 | On deny: sealed/locked-style message (match TFS/772 text) |
| 5.5 | On allow: existing Lua ±1 transform |
| 5.6 | Optional follow-up: `edit_door.lua` / House Lua (`getDoors`, access-list UI) — **not** required for click open/close |

**Done when:** Invited players open house doors; strangers cannot; transform still driven by `doors.lua`.

---

### Phase 6 — Auto-close MoveEvents

**Goal:** Leaving an open quest/level door tile closes it (`closing_doors.lua`).

| Task | Detail |
|------|--------|
| 6.1 | Extend `MoveEvent` beyond equip/deequip: `onStepIn` / `onStepOut`, `:id`, `:register` |
| 6.2 | Load `data/scripts/movements/**` |
| 6.3 | Tile APIs: `getCreatureCount`, `queryAdd`, `getThing` / `getThingCount`, `getItemByGroup` |
| 6.4 | Global `doRelocate(from, to)` |
| 6.5 | Depends on Phase 0 door table fix |
| 6.6 | `level_doors.lua` step-in kick if desired in same pass |

**Done when:** Walking out of an open quest/level door auto-closes and relocates leftovers per script.

---

## 4. Phase dependency graph

```
Phase 0 (global.lua tables)
    │
    ▼
Phase 1 (Action + onUse) ──────────────► enables food.lua / levers / etc.
    │
    ▼
Phase 2 (basic open/close/locked)
    │
    ├──────────► Phase 3 (keys)
    ├──────────► Phase 4 (quest/level)
    └──────────► Phase 5 (house native gate)
                      │
Phase 6 (MoveEvent close) ◄── needs Phase 0 + 2 (+ 4 for quest/level IDs)
```

Phases 3–5 can proceed in parallel after Phase 2. Phase 6 needs MoveEvent work and is independent of keys/house except for shared tile APIs.

---

## 5. Verification checklist (per phase)

| Phase | Suggested checks |
|-------|------------------|
| 1 | Unit: Action register + dispatch mock; live: use food / log `onUse` |
| 2 | Live: open/close wooden doors; stand on tile and close; locked door text |
| 3 | Live: key use-with match/mismatch |
| 4 | Live: storage/level gate + GM access bypass |
| 5 | Live: house owner vs stranger; guest list |
| 6 | Live: step out of open quest door → closes |

Also: `rtk cargo test -p tfs-rust-lua` / `tfs-rust-core` for new registries; no silent change to non-door use paths.

---

## 6. Explicit non-goals (this plan)

- Rewriting `doors.lua` into Rust (keep data-pack script)
- Full House editor UI (`edit_door.lua`) before Phase 5 core gate
- Porting every `data/scripts/actions/**` script body — only the **Action plumbing**; scripts come free once Phase 1 works
- Changing door ID tables to decompile item IDs if OTB already maps correctly

---

## 7. Suggested first PR slice

**Phase 1 only:** Action userdata + load `actions/**` + use → `onUse` + fix `Item:getId()`.  
Prove with `food.lua` or a one-line debug action, then Phase 2 doors in the next PR.

---

## 8. Cross-links

- `tasks/chat-system-plan.md` — self-registering Lua object pattern (`Channel` / `TalkAction`)
- `tasks/lua-api-plan.md` — shared Player/Condition primitives (storage may land there or here)
- `tasks/f8-player-actions-todo-subplan.md` — ToDo `Use` routing (orthogonal; Action Lua still needs a call site when Use executes)
- `data/scripts/actions/other/doors.lua`
- `data/scripts/movements/other/closing_doors.lua`
- `data/global.lua` (door tables)
