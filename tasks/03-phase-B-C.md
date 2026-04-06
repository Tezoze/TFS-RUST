# Phase B — Item Runtime & Containers

**Goal:** Items exist as rich objects; containers can be opened, items can be moved.
**C++ ref files:** `item.cpp`, `item.h`, `items.cpp`, `container.cpp`, `container.h`, `cylinder.cpp`
**Estimated effort:** 3–5 days

**Status:** ✅ Complete

---

## Completed

- [x] **B.1** Expand `Item` struct — full attribute system with `ItemAttributes`, `AttrType`, `ItemAttrFlags`, `DecayState`, `CustomAttrValue`. Mirror C++ `item.h` AttrTypes_t and ItemAttributes class.
  - Files: `item.rs`, `item_attributes.rs`
  - 35+ attribute types supported (actionId, uniqueId, text, duration, charges, custom attributes, etc.)

- [x] **B.2** Port `Container` with full hierarchy — child items vec, capacity, parent container, open-by tracking, Depot/Inbox/StoreInbox types.
  - Files: `container.rs` with `Container`, `ContainerRegistry`, `ContainerType` enum
  - Pagination support, viewer tracking (cid list)

- [x] **B.3** Implement `Cylinder` enum dispatch (zero-cost) — replaced trait objects with enum for Tile/Container/Inventory.
  - Files: `cylinder.rs`
  - **DEVIATION FROM C++:** Using enum dispatch instead of virtual class hierarchy. Rationale: exactly 3 known types, zero-cost, more idiomatic Rust.

- [x] **B.4** Port `Game::internalMoveItem` / `internalAddItem` / `internalRemoveItem`.
  - Files: `game_world.rs`, `tile.rs`
  - Resolved architectural ripple: `Vec<u16>` → `Vec<ItemId>` on tiles, items SlotMap plumbed through map loading → GameWorld.
  - `internal_move_item`: full tile↔tile with stackable merge/split, tile↔container stubs.
  - `internal_add_item_to_tile`: stackable merge into existing stacks (100-cap), remainder handling.
  - `internal_remove_item_from_tile`: partial count removal for stackables, full removal with SlotMap cleanup.
  - `internal_get_cylinder`: resolves client position encoding (tile / container / inventory).
  - `internal_get_thing_move`: STACKPOS_MOVE — top moveable down item, fallback to creature.
  - `query_add_item_to_tile`: max items check, ground requirement, block_solid vs creatures.
  - `validate_item_in_cylinder`: confirms item exists on source tile.

- [x] **B.5** Handle `Throw` (item move) — `GamePacket::Throw` → `player_move_thing` → `player_move_item`.
  - Files: `game_loop.rs`, `game_world.rs`
  - Sprite ID verification, moveable check, z-level check, distance check, cylinder resolution.

- [x] **B.6** Implement server→client item packets (0x6A-0x6C, 0x6E-0x72).
  - Files: `game_world.rs` (broadcast functions), `outgoing_extra.rs` (packet builders — pre-existing)
  - `broadcast_tile_item_add` (0x6A): sends `sendAddTileItem` template to all spectators.
  - `broadcast_tile_item_update` (0x6B): sends `sendUpdateTileItem` for stackable count changes.
  - `broadcast_tile_item_remove` (0x6C): sends `sendRemoveTileThing` on full removal.
  - Container packets (0x6E open, 0x6F close, 0x70 add, 0x71 update, 0x72 remove) — builders exist, wiring pending full container runtime.

- [x] **B.7** Handle `CloseContainer`, `UpArrowContainer`, `UpdateContainer`, `SeekInContainer` game packets.
  - Files: `game_loop.rs` (dispatch), `game_world.rs` (handler stubs)
  - All four packets wired from game loop to GameWorld handler methods.
  - Handler bodies are stubs pending full container viewer/registry integration.

- [x] **B.8** Port fluid container / splash encoding for live items.
  - Files: `item_encode.rs` (pre-existing), `game_world.rs` (broadcast integration)
  - `FLUID_MAP` / `server_fluid_to_client` matches C++ `fluidMap` in `src/const.h`.
  - `write_item_template` and `write_item_live` both handle `is_splash_or_fluid` path.
  - Broadcast functions pass `is_splash() || is_fluid_container()` from ItemType.

---

# Phase C — Inventory & Equipment

**Goal:** Players can equip/unequip items, inventory renders in client, capacity works.
**C++ ref files:** `player.cpp` (slots), `player.h`, `game.cpp` (playerEquipItem)
**Estimated effort:** 2–3 days

> **Depends on:** Phase B complete.

---

- [ ] **C.1** Real inventory slot model on `Player` — 10 slots + store inbox, each holds an `ItemId` or container.
- [ ] **C.2** `sendInventoryItem` (0x78) / `sendInventoryItemRemove` (0x79) — real items not stubs.
- [ ] **C.3** Port `Player::onEquipItem` / `onDeEquipItem` — stat modifiers, condition apply (e.g. life ring regen).
- [ ] **C.4** Capacity system — `Player::getFreeCapacity`, weight calculation on move/equip.
- [ ] **C.5** `EquipObject` handler (quick-equip from hotbar).
- [ ] **C.6** Item description / `LookAt` handler — build description string, send `sendTextMessage`.
