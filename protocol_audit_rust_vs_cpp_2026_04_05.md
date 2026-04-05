# Rust vs C++ protocol & content audit (2026-04-05)

High-level comparison of this repo’s **Rust** game stack (`crates/tfs-rust-*`) against the bundled **TFS 1.4.2 C++** sources under `src/`. Focus: symptoms **login works**, **most of the screen is black**, **only some tiles show** — consistent with **sparse/wrong map data**, **item ID resolution**, or **historical packet misalignment** (not only a single opcode bug).

**C++ reference files used:** `src/protocolgame.cpp` (`GetTileDescription`, `GetMapDescription`, `sendMapDescription`), `src/networkmessage.cpp` (`addItem`), `src/iomap.cpp` (OTBM tile load), `src/tile.cpp` (`internalAddThing`), `src/map.h` (viewport constants).

---

## 1. Verdict: where the biggest gaps are

| Area | Parity vs C++ | Risk for “black / partial map” |
|------|----------------|----------------------------------|
| **OTBM → runtime tile** | **Incomplete** | **High** — child `OTBM_ITEM` stacks ignored; ground/stack rules differ |
| **Item server ID → client sprite** | Depends on `items.otb` / DB | **High** — `client_id == 0` skips drawing that item |
| **Tile item order (top / creatures / bottom)** | **Mismatch** | **Medium** — wrong layering; can look broken or “empty” on busy tiles |
| **`GetMapDescription` / skip / z-loop** | **Largely aligned** | Low if tile callback returns correct bytes |
| **`NetworkMessage::addItem` template path** | **Aligned** when `with_description` is false for map tiles | Was a critical desync if map sent OTCv8 description slots; see §4 |

---

## 2. OTBM / map loading (highest impact)

### 2.1 `OTBM_ITEM` child nodes — not applied to the world

Rust parses child item nodes into `TileThing::ItemNodeProps(Vec<u8>)` (`crates/tfs-rust-content/src/otbm.rs`) but `Map::from_map_data` → `tile_from_data` (`crates/tfs-rust-core/src/map/mod.rs`) **only** handles `TileThing::EmbeddedItemId(u16)`.

C++ (`src/iomap.cpp`) creates real `Item*` for each child node (`Item::CreateItem`, `unserializeItemNode`) and calls `tile->internalAddThing(item)`.

**Effect:** Any tile whose visible stack depends on **child `OTBM_ITEM` nodes** (very common) will be **missing most or all non-attribute items** in Rust. Tiles that only have children and no `OTBM_ATTR_ITEM` in the attributes stream can end up **with no ground** in Rust → `map_tile_content` returns `None` → **skipped tile in the map stream** → **holes / black**.

### 2.2 Ground vs non-ground — simplified first-ID rule

Rust treats the **first** `EmbeddedItemId` as ground and the rest as a flat `Vec` (`tile_from_data`).

C++ uses `ItemType::isGroundTile()` and `internalAddThing` logic (`src/tile.cpp`): ground is the first **ground-type** item; other items are classified as **always-on-top** vs **down** items using OTB flags (`alwaysOnTop`, ordering, `addDownItemCount`).

**Effect:** Wrong classification on edge cases; less common than §2.1 but still a parity gap.

### 2.3 Top vs down items for the protocol

C++ `GetTileDescription` (`src/protocolgame.cpp`) sends, in order:

1. Ground  
2. **Top** items (`getBeginTopItem` → `getEndTopItem`)  
3. Creatures (reverse iteration)  
4. **Down** items (`getBeginDownItem` → `getEndDownItem`) while count &lt; 10  

Rust `map_tile_content` (`crates/tfs-rust-core/src/login_out.rs`) puts **all** non-ground `body.items` into `TileContent::top_items` and **never fills `bottom_items`**.

**Effect:** Protocol order differs from TFS whenever a tile mixes **down** items with creatures — client draw order can be wrong; some setups may look “empty” or glitchy. This is a **structural** mismatch with C++, not just optimization.

### 2.4 Spectator quadtree

`Map::from_map_data` only inserts a quadtree for **z = 0** (`crates/tfs-rust-core/src/map/mod.rs`). C++ maintains spatial structures per floor as needed.

**Effect:** Mainly creature/spectator logic on other floors; secondary to the black-map symptom unless features depend on it.

---

## 3. Items / OTB (`client_id` and flags)

### 3.1 Unknown server IDs → invisible items

`ItemDatabase::client_id_for_server` (`crates/tfs-rust-content/src/items.rs`) returns **0** if the server id is missing from the merged OTB/XML map.

`map_tile_content` skips items with `cid == 0` and skips ground with no valid `client_id`.

**Effect:** Any mismatch between **map item ids** and **loaded `items.otb` / `items.xml`** produces **missing sprites** — often **missing ground** → black tile.

### 3.2 Flags used on the wire

Rust pulls `stackable`, splash/fluid, and animation from the same merged item DB as C++’s `ItemType` fields used in `NetworkMessage::addItem` (`src/networkmessage.cpp`). If OTB flags are wrong or items are missing, **counts / fluid bytes / `0xFE` animation** can be wrong and desync the client.

---

## 4. Protocol encoding (map vs inventory)

### 4.1 Map tiles: `addItem` template must not use OTCv8 description for map

C++ `GetTileDescription` uses `msg.addItem(ground)` / `msg.addItem(*it)` — default `withDescription = false` (`src/networkmessage.cpp`).

Rust `get_tile_description` (`crates/tfs-rust-net/src/map_description.rs`) calls `write_item_template(..., false)` for map items (the `with_description` parameter is effectively unused for item templates there).

**Note:** An earlier internal audit (`protocol_audit_2026_04_05.md`) recorded that passing OTCv8 `with_description` into map tile templates **desynced OTClient** (extra empty string before duration). That must stay **false** for map templates.

### 4.2 `with_description` still threaded through map APIs

`build_initial_map_packet` / `send_move_creature_player` still pass `player.item_with_description()` into `write_map_description_body`, but tile item encoding **does not** use it for item bytes (only the unused parameter on `get_tile_description`). Harmless but confusing; optional cleanup.

### 4.3 Viewport and `GetMapDescription` math

Rust uses `MAX_CLIENT_VIEWPORT_X = 8`, `MAX_CLIENT_VIEWPORT_Y = 6` and `client_viewport_width/height` (`crates/tfs-rust-common/src/protocol_constants.rs`), matching `Map::maxClientViewportX/Y` (`src/map.h`). The z-loop and skip logic in `write_map_description_body` match `ProtocolGame::GetMapDescription` / `GetFloorDescription` (`src/protocolgame.cpp`).

---

## 5. Other protocol notes (secondary to black map)

- **`look_type_ex`:** In C++, `addItemId` maps server id → client id; Rust `creature_encode::write_outfit` writes `look_type_ex` raw; `login_out::outfit_to_wire` hardcodes `0` — fine until illusion / item-outfits are used (`protocol_audit_2026_04_05.md` §5).
- **`AddCreature`:** Rust `write_add_creature` follows the same field order as `ProtocolGame::AddCreature` (`src/protocolgame.cpp` ~3174+).

---

## 6. Recommended investigation order (for “black screen + sparse tiles”)

1. **Confirm OTBM coverage:** Log Rust tile counts vs C++ for the same `world.otbm` (or compare `EmbeddedItemId` vs `ItemNodeProps` counts per tile). If many `ItemNodeProps` exist, implementing **child item creation** (minimal: parse id from props + stack) is mandatory for parity.  
2. **Confirm item resolution:** Log any `server_id` where `client_id_for_server` returns 0 during `map_tile_content`.  
3. **Split top / bottom items:** Mirror C++ `alwaysOnTop` / down-item rules when building `TileContent` from `TileBody`.  
4. **Keep map `write_item_template(..., false)`** for ground/top/bottom map items; reserve `item_with_description()` for **live** items / inventory where C++ passes `otclientV8`.

---

## 7. File index (Rust)

| Concern | Location |
|---------|----------|
| Map packet body | `crates/tfs-rust-net/src/map_description.rs` |
| Item bytes | `crates/tfs-rust-net/src/item_encode.rs` |
| Tile → wire content | `crates/tfs-rust-core/src/login_out.rs` (`map_tile_content`) |
| OTBM parse | `crates/tfs-rust-content/src/otbm.rs` |
| OTBM → `Tile` | `crates/tfs-rust-core/src/map/mod.rs` (`tile_from_data`) |
| Item DB / client id | `crates/tfs-rust-content/src/items.rs`, `otb.rs` |

---

*Generated as a high-level audit for Rust ↔ C++ parity; refine with concrete logs from the same client version (10.98 / OTCv8) and the same `data/` pack.*
