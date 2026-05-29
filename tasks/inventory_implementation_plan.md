# Inventory & Equipment — Implementation Plan

Prioritized work breakdown derived from `tasks/inventory_audit.md`.
Items grouped into phases where each phase builds on the previous.
Within a phase, items with the same letter should be implemented together.

> **Save Pipeline is deferred** — the DB is currently a static export for testing.
> Save will become Phase 8 once live DB loading/updating is wired.

---

## Phase 1: Container Core Operations

The container is the second cylinder type and blocks almost everything else.

### 1A. `ContainerIterator` (recursive BFS)
- **File:** `crates/tfs-rust-core/src/container.rs`
- **C++ ref:** `src/container.cpp:762–795` (`ContainerIterator`), `src/container.h:25–39`
- Implement a recursive iterator over `ContainerRegistry` that yields all
  `ItemId`s in a container and its nested sub-containers (BFS, matching C++
  `ContainerIterator`).
- Needed by: `getItemHoldingCount`, `isHoldingItem`, `getItemTypeCount`,
  `getContentDescription`, `removeItemOfType`, decay, and many Lua bindings.

### 1B. Container `totalWeight` Tracking
- **File:** `crates/tfs-rust-core/src/container.rs`
- **C++ ref:** `src/container.cpp:222–244` (`Container::updateItemWeight`), `src/container.h:161` (`totalWeight` member)
- Add `total_weight: u32` to `Container`.
- Update incrementally on `add_item` / `remove_item` / `update_item` (propagate
  diff to parent container via registry).
- Replaces the current `item_recursive_weight_oz` full-tree walk for weight queries
  (keep the full walk as a debug assertion / fallback).

### 1C. Complete Container Cylinder Operations
- **File:** `crates/tfs-rust-core/src/container.rs` + `game_world_inventory.rs`
- **C++ ref:**
  - `Container::queryAdd` — `src/container.cpp:284–384`
  - `Container::queryRemove` — `src/container.cpp:386–409`
  - `Container::queryMaxCount` — `src/container.cpp:411–467`
  - `Container::queryDestination` — `src/container.cpp:469–528`
  - `Container::addThing` — `src/container.cpp:530–576`
  - `Container::updateThing` — `src/container.cpp:578–601`
  - `Container::replaceThing` — `src/container.cpp:603–633`
  - `Container::removeThing` — `src/container.cpp:635–681`
  - `Container::isHoldingItem` — `src/container.cpp:258–282`
- `Container::queryAdd` — full validation: `isPickupable`, `isStoreItem`/depot rules,
  cycle check (`isHoldingItem`), inbox blocking, `FLAG_NOLIMIT`, `FLAG_CHILDISOWNER`
  propagation to top parent.
- `Container::queryRemove` — moveability + count validation.
- `Container::queryMaxCount` — stackable free-slot calculation.
- `Container::queryDestination` — move-up (254), add-wherever (255), grey-area
  clamp, sub-container redirect, auto-stack search.
- `addThing` (push_front + weight update), `removeThing` (partial stack),
  `updateThing`, `replaceThing`.

### 1D. Fix `internal_move_item` Stubs
- **File:** `crates/tfs-rust-core/src/game_world.rs`
- **C++ ref:** `src/game.cpp:1078–1285` (`Game::internalMoveItem`)
- Tile→Container: actually add to container after tile remove.
- Container→Tile: actually remove from container before tile add.
- Container→Container: remove from source, add to dest.
- Wire `queryDestination` loop — `src/game.cpp:1102–1109`.
- Wire `queryMaxCount` for stackable merge — `src/game.cpp:1167–1178`.
- Wire `NEEDEXCHANGE` handling — `src/game.cpp:1118–1159`.

**Deliverable:** Items can be moved between all cylinder types correctly.

---

## Phase 2: Container UI Protocol

Players need to see and interact with containers.

### 2A. Outgoing Container Packets
- **File:** `crates/tfs-rust-net/src/outgoing_extra.rs` (or new file)
- **C++ ref:**
  - `sendContainer` — `src/protocolgame.cpp` `ProtocolGame::sendContainer`
  - `sendAddContainerItem` — `src/protocolgame.cpp` `ProtocolGame::sendAddContainerItem`
  - `sendUpdateContainerItem` — `src/protocolgame.cpp` `ProtocolGame::sendUpdateContainerItem`
  - `sendRemoveContainerItem` — `src/protocolgame.cpp` `ProtocolGame::sendRemoveContainerItem`
  - `sendCloseContainer` — `src/protocolgame.cpp` `ProtocolGame::sendCloseContainer`

### 2B. Open-Container Tracking Rework
- **File:** `crates/tfs-rust-core/src/container.rs`
- **C++ ref:** `src/player.cpp:563–641` (`addContainer`, `closeContainer`, `setContainerIndex`, `getContainerByID`, `getContainerID`, `getContainerIndex`), `src/player.h:75–78` (`OpenContainer` struct), `src/player.h:1238` (`openContainers` map)
- Replace `player_containers: HashMap<u32, Vec<ItemId>>` with
  `HashMap<u32, HashMap<u8, OpenContainer>>` where `OpenContainer` holds
  `container_id: ItemId` + `index: u16` (scroll offset).
- Port `addContainer`, `closeContainer`, `setContainerIndex`, `getContainerByID`,
  `getContainerID`, `getContainerIndex`.

### 2C. Use-Action → Container Open
- **File:** `crates/tfs-rust-core/src/game_world.rs` (use-item path)
- **C++ ref:** `src/actions.cpp` `Actions::useItem` (container branch), `src/game.cpp` `Game::playerUseItem`
- When a player "uses" an item that `is_container()`, assign a client cid,
  register viewer, send `sendContainer`.
- Wire right-click on ground containers + inventory containers.

### 2D. Container Navigation + Close
- **File:** `crates/tfs-rust-core/src/game_world.rs`
- **C++ ref:**
  - `Game::playerCloseContainer` — `src/game.cpp:2355–2378`
  - `Game::playerUpContainer` — (follows `playerCloseContainer` in `game.cpp`)
  - `Game::playerUpdateContainer` / seek — `src/game.cpp` (update/seek handlers)
- `player_close_container` — remove viewer, send `sendCloseContainer`.
- `player_up_container` — resolve parent, send new `sendContainer`.
- `player_update_container` / `player_seek_in_container` — scroll/pagination.

### 2E. Spectator Notifications
- **Files:** `container.rs`, `game_world_inventory.rs`
- **C++ ref:**
  - `Player::sendAddContainerItem` — `src/player.cpp:1015–1044`
  - `Player::sendUpdateContainerItem` — `src/player.cpp:1047–1067`
  - `Player::sendRemoveContainerItem` — `src/player.cpp:1074–1095`
  - `Player::autoCloseContainers` — `src/player.cpp:2357–2378`
  - `Player::onSendContainer` — `src/player.cpp:1447–1457`
  - `Player::onCloseContainer` — `src/player.cpp:1432–1443`
  - `Container::postAddNotification` — `src/container.cpp:683–714`
  - `Container::postRemoveNotification` — `src/container.cpp:716–760`
- `onAddContainerItem` / `onUpdateContainerItem` / `onRemoveContainerItem` —
  notify all viewers (not just the acting player).
- `autoCloseContainers` — when container moves out of range or is removed.
- `onSendContainer` — refresh open container after content change.

### 2F. `autoOpenContainers` on Login
- **File:** `crates/tfs-rust-core/src/login.rs` + `player_inventory_load.rs`
- **C++ ref:** `src/player.cpp:938–960` (`Player::autoOpenContainers`)
- After hydrating inventory, scan equipment for containers with
  `ATTR_OPEN_CONTAINER` set, call `addContainer` + `sendContainer`.

**Deliverable:** Players can open bags, move items in/out, navigate parent, close.

---

## Phase 3: Full `Player::queryAdd` (Equipment Validation)

### 3A. Port `Player::queryAdd` (~220 lines)
- **File:** `crates/tfs-rust-core/src/inventory.rs` or new `player_query_add.rs`
- **C++ ref:** `src/player.cpp:2397–2617` (`Player::queryAdd`)
- Full per-slot switch with:
  - `isPickupable` / `isStoreItem` guards
  - Right hand (5): classic vs non-classic, shield-only in non-classic, two-hand
    conflict, dual-shield/dual-weapon block
  - Left hand (6): mirror rules
  - Two-handed weapon + opposite hand occupied → `BOTHHANDSNEEDTOBEFREE`
  - Ammo slot classic-mode exception
  - `CONST_SLOT_WHEREEVER` / `-1` fallback
  - Distinct error messages (`PUTTHISOBJECTINBOTHHANDS`, `PUTTHISOBJECTINYOURHAND`,
    `CANNOTBEDRESSED`, `DROPTWOHANDEDITEM`, `CANONLYUSEONEWEAPON`, `CANONLYUSEONESHIELD`)
- Add `CLASSIC_EQUIPMENT_SLOTS` config key.
- Capacity check at end.

### 3B. Port `Player::queryRemove`
- **C++ ref:** `src/player.cpp:2695–2716` (`Player::queryRemove`)
- Moveability check, stackable count validation.

### 3C. Port `Player::queryMaxCount`
- **C++ ref:** `src/player.cpp:2619–2693` (`Player::queryMaxCount`)
- Deep search through all slots + sub-containers for stackable merge capacity.

### 3D. Port `Player::queryDestination`
- **C++ ref:** `src/player.cpp:2718–2841` (`Player::queryDestination`)
- Auto-stack into existing slot items, then search containers BFS for first fit.
- Trade item exclusion.

### 3E. Wire Into `internal_move_item`
- **C++ ref:** `src/game.cpp:1078–1285` (the `queryAdd`/`queryMaxCount`/`queryRemove` call sequence)
- Replace current simplified checks with full `queryAdd`→`queryMaxCount`→`queryRemove`
  chain before every move.

**Deliverable:** Correct equipment rules — no double shields, two-hand works, error messages match C++.

---

## Phase 4: Notification Chain + Weight/Light/Stats

### 4A. `postAddNotification` / `postRemoveNotification`
- **Files:** `game_world.rs`, `game_world_inventory.rs`
- **C++ ref:**
  - `Player::postAddNotification` — `src/player.cpp:3076–3129`
  - `Player::postRemoveNotification` — `src/player.cpp:3131–3191`
  - `Container::postAddNotification` — `src/container.cpp:683–714`
  - `Container::postRemoveNotification` — `src/container.cpp:716–760`
  - `Tile::postAddNotification` / `postRemoveNotification` — `src/tile.cpp`
- On Player: call move-event equip/deequip, inventory update event,
  `updateInventoryWeight`, `updateItemsLight`, `sendStats`.
- On Container: propagate to top parent (player or tile).
- Handle `LINK_OWNER` / `LINK_TOPPARENT` / `LINK_PARENT` / `LINK_NEAR`.

### 4B. `inventoryAbilities` Tracking
- **File:** `creature/player.rs`
- **C++ ref:** `src/player.h:1359` (`inventoryAbilities[CONST_SLOT_LAST + 1]`), `src/player.h:485–490` (`isItemAbilityEnabled`/`setItemAbility`)
- `inventory_abilities: [bool; 11]` — set on equip, clear on deequip.
- Used by stat bonus system (attack, defense, armor, speed, skills, etc.).

### 4C. `updateItemsLight`
- **C++ ref:** `src/player.cpp` `Player::updateItemsLight` (search for `updateItemsLight`)
- Compute player light from equipped items, update creature light level.

### 4D. `onUpdateInventoryItem` / `onRemoveInventoryItem`
- **C++ ref:** `src/player.cpp:1398–1430` (`onUpdateInventoryItem`, `onRemoveInventoryItem`), `src/player.h:899–900`
- Event hooks called from `updateThing` / `removeThing` on Player.

**Deliverable:** Stats, weight, light update correctly on every inventory change.

---

## Phase 5: Player Inventory Utility Methods

### 5A. `getItemTypeCount` + `getAllItemTypeCount`
- **C++ ref:** `src/player.cpp:2974–2996` (`getItemTypeCount`), `src/player.cpp:3049–3066` (`getAllItemTypeCount`)
- Recursive count across all slots + containers using `ContainerIterator`.
- Needed by Lua `player:getItemCount`, spells, quest checks.

### 5B. `removeItemOfType`
- **C++ ref:** `src/player.cpp:2998–3047` (`Player::removeItemOfType`)
- Scan equipped items (optionally) + all containers, batch remove.
- Needed by Lua `player:removeItem` (full version), spells consuming reagents.

### 5C. `getWeapon` / `getWeaponType` / `getWeaponSkill`
- **C++ ref:** `src/player.cpp:195–230` (`getWeapon(slot)`, `getWeapon()`), `src/player.h:631` (`getWeaponType`), `src/player.h:632` (`getWeaponSkill`)
- Needed by combat system.

### 5D. `PlayerFlag` Checks
- **C++ ref:** `src/player.h:454–471` (`getCapacity`/`getFreeCapacity` with flag checks), `src/enums.h` (`PlayerFlags` enum)
- `HasInfiniteCapacity`, `CannotPickupItem` in capacity methods.
- `PlayerFlag` enum or bitfield on Player.

**Deliverable:** Lua scripts and combat can query/modify inventory correctly.

---

## Phase 6: Depot + Inbox

### 6A. Depot Data Structures
- **C++ ref:** `src/depotchest.h`, `src/depotlocker.h`, `src/player.h:1239–1240` (`depotLockerMap`, `depotChests`), `src/player.h:1336` (`lastDepotId`)
- `DepotChest`, `DepotLocker` types (or enum variants).
- `depot_chests: HashMap<u32, ItemId>` on Player.
- `depot_locker_map: HashMap<u32, ItemId>` on Player.
- `last_depot_id: i16`.

### 6B. Hydrate Depot from DB
- **C++ ref:** `src/iologindata.cpp:449–487` (depot item loading loop)
- `login.rs` → pass `loaded.items.depot` to a new `load_depot_table`.
- Build depot chest containers in `ContainerRegistry`.

### 6C. Hydrate Inbox from DB
- **C++ ref:** `src/iologindata.cpp:489–533` (inbox + store inbox loading)
- Same for `loaded.items.inbox`.
- `inbox` and `store_inbox` item references on Player.

### 6D. Depot Interaction
- **C++ ref:** `src/player.cpp` `Player::isNearDepotBox`, `src/player.h:506–510` (`getDepotChest`, `getDepotLocker`), `src/player.h:700` (`getMaxDepotItems`)
- `isNearDepotBox()` — scan nearby tiles for depot locker items.
- Open depot locker → open depot chest container UI.
- `getMaxDepotItems()` (premium vs free).
- `DepotIsFull` return value enforcement.

**Deliverable:** Players can store/retrieve items from depots.

---

## Phase 7: Lua Bindings Expansion

### 7A. Container Userdata
- **File:** new `crates/tfs-rust-lua/src/userdata/container.rs`
- **C++ ref:** `src/luascript.cpp:2343–2360` (registration), `src/luascript.cpp:7105–7390` (implementations)
- `Container(uid)` constructor
- Methods: `getSize`, `getCapacity`, `setCapacity`, `getEmptySlots`,
  `getContentDescription`, `getItems`, `getItemHoldingCount`, `getItemCountById`,
  `getItem`, `hasItem`, `addItem`, `addItemEx`, `getCorpseOwner`

### 7B. Player Inventory Lua Methods
- **C++ ref:** `src/luascript.cpp:2514–2515` (`getItemCount`/`getItemById` registration), `src/luascript.cpp:2551–2553` (`addItem`/`addItemEx`/`removeItem`), `src/luascript.cpp:2471–2472` (`getDepotChest`/`getInbox`), `src/luascript.cpp:2615–2617` (`getContainerId`/`getContainerById`/`getContainerIndex`)
- **C++ impl:** `src/luascript.cpp:9055–9077` (`luaPlayerGetItemCount`), `src/luascript.cpp:9080–9120` (`luaPlayerGetItemById`), `src/luascript.cpp:9476–9559` (`luaPlayerAddItem`), `src/luascript.cpp:8577–8600` (`luaPlayerGetDepotChest`)
- `player:getItemCount(itemId, subType)` — needs `getItemTypeCount`
- `player:getItemById(itemId, deepSearch, subType)` — recursive search
- `player:addItem` full version (canDropOnMap, subType, slot)
- `player:addItemEx`
- `player:getDepotChest(depotId, autoCreate)`
- `player:getInbox()`
- `player:getContainerId(container)`
- `player:getContainerById(cid)`
- `player:getContainerIndex(cid)`

### 7C. Item Lua Methods (Missing)
- **C++ ref:** `src/luascript.cpp:2278–2341` (Item method registration), implementations throughout `src/luascript.cpp:6800–7100`
- `item:getPosition`, `item:getParent`, `item:getTopParent`
- `item:moveTo(pos/container/player)` — calls `internalMoveItem`
- `item:remove(count)` — calls `internalRemoveItem`
- `item:transform(newId, count)` — calls `transformItem`
- `item:isContainer`, `item:getContainer`
- `item:getActionId`, `item:setActionId`, `item:getUniqueId`, `item:setUniqueId`
- `item:getAttribute`, `item:setAttribute`, `item:removeAttribute`
- `item:isStoreItem`, `item:setStoreItem`

**Deliverable:** Lua scripts have full inventory/container API parity with TFS.

---

## Phase 8: Save Pipeline (Deferred — Wire When Live DB Is Ready)

> Not needed while testing with a static DB export. Implement when live
> DB loading/updating is wired.

### 8A. Item Blob Serialization (`write_item_blob`)
- **File:** `crates/tfs-rust-core/src/item_blob.rs`
- Mirror `parse_item_blob` with a `write_item_blob(item: &Item) -> Vec<u8>` that
  serializes all `ItemAttributes` back into the TFS binary blob format.
- C++ ref: `src/item.cpp` `Item::serializeAttr` / `PropWriteStream`

### 8B. Flatten Runtime Inventory → `Vec<ItemRecord>`
- **File:** new `crates/tfs-rust-core/src/player_inventory_save.rs`
- Walk `equipment_slots[0..11]` and `ContainerRegistry` children recursively,
  assigning `pid`/`sid` exactly like C++ `IOLoginData::saveItems`.
- Must handle nested containers (BFS queue with parent sid tracking).
- Include `openContainers` cid mapping so `ATTR_OPEN_CONTAINER` is preserved.
- C++ ref: `src/iologindata.cpp:561–610`

### 8C. Wire Save on Logout + Graceful Shutdown
- **Files:** `crates/tfs-rust-core/src/game_loop.rs`, `login.rs` / logout path
- Build `PlayerSaveData` from live `Player` + flattened items.
- Call `PlayerStore::save_player` on disconnect and in `graceful_shutdown`.
- Save all 4 tables: inventory, depot (if depot was accessed), inbox, store inbox.

### 8D. Depot Save
- Wire `skip_depot_save` / `last_depot_id` logic.
- Flatten depot chests to `ItemRecord` rows for `player_depotitems`.

**Deliverable:** Players keep their items across server restarts.

---

## Phase 9: Edge Cases & Polish

### 9A. `Game::internalAddItem` with Remainder
- **C++ ref:** `src/game.cpp:1287–1374` (`internalAddItem`), `src/game.cpp:1422–1439` (`internalPlayerAddItem`)
- General-purpose add that returns remainder count for overflow.
- `internalPlayerAddItem` with `dropOnMap` fallback.

### 9B. Money Operations
- **C++ ref:** `src/game.cpp:1535–1610` (`Game::addMoney`, `Game::removeMoney`)
- `addMoney` / `removeMoney` on cylinder.

### 9C. `transformItem` (type change / charges depletion)
- **C++ ref:** `src/game.cpp:1618–1700` (`Game::transformItem`)

### 9D. BrowseField Containers
- **C++ ref:** `src/game.cpp:2518–2570` (`Game::playerBrowseField`), `src/container.cpp:42–60` (`Container(Tile*)` constructor)
- `playerBrowseField` — create temporary container from tile items.

### 9E. Decay Recursion in Containers
- **C++ ref:** `src/container.cpp:175–205` (`Container::startDecaying`), `src/container.cpp:207–220` (`Container::stopDecaying`)
- `Container::startDecaying` / `stopDecaying` propagating to children.

### 9F. Trade System Integration
- **C++ ref:** `src/game.cpp` `Game::playerSetUpTrade` / `playerAcceptTrade`, `src/player.cpp:1191` (`checkTradeState`), `src/player.cpp:3193–3215` (`updateSaleShopList`)
- `tradeItem` on Player, `checkTradeState`, blocking in `internalMoveItem`.
- `updateSaleShopList` on inventory change.

### 9G. Market / Shop Integration
- **C++ ref:** `src/player.h:1039–1042` (`sendMarketEnter`), `src/protocolgame.cpp` `ProtocolGame::sendMarketEnter`
- `sendMarketEnter(depotId)` uses depot item counts.
- Shop list refresh on inventory changes.

---

## Dependency Graph (Simplified)

```
Phase 1 (Container Core) ──── Phase 2 (Container UI) ───┐
    │                              │                     │
Phase 3 (queryAdd)                 │                     │
    │                              │                     │
Phase 4 (Notifications) ──────────┘                      │
    │                                                    │
Phase 5 (Utility Methods) ── Phase 7 (Lua Bindings) ────┘
    │                                                    │
Phase 6 (Depot + Inbox) ────────────────────────────────┘
    │
Phase 8 (Save) ← deferred until live DB is wired
    │
Phase 9 (Polish)
```

**Phase 1 (Container Core) is the top priority** — it unblocks Phases 2–5.
Phase 3 can be done in parallel with Phase 2.
Phase 8 (Save) is independent and deferred until live DB loading is set up.
