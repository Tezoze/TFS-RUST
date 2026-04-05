# Map & Item Packet Fixes

## Current state
Skills, stats, and movement work. Screen is black. OTClient logs `invalid thing id (0)` on every map update packet (`0x64`).

---

## Fix 1 — XTEA inner length value (CRITICAL — blocks everything)

**File:** `crates/tfs-rust-net/src/protocol_game.rs` — `encrypt_xtea_game_frame`

**Bug:** The inner length `v` is set to `plain_len - 2` (the padded block size minus the header). OTClient's `xteaDecrypt` reads `v` then checks `v + 8 > total_decrypted_size`. For a 2-byte payload padded to 8 bytes: `v = 6`, check is `6 + 8 > 8` → true → decrypt fails.

**Fix:** `v` must equal `payload.len()` — the number of payload bytes after the inner length field.

```rust
// Wrong:
let v = plain_len - 2;

// Correct:
let v = payload.len();
```

---

## Fix 2 — `write_item_template` missing animation byte

**File:** `crates/tfs-rust-net/src/item_encode.rs`

**Bug:** C++ `NetworkMessage::addItem` sends an extra `0xFE` byte before the duration byte when `it.isAnimation == true`. Ground tiles (grass, water, etc.) are commonly animated. Your Rust never sends this byte, so OTClient misreads the duration byte as the animation byte and then tries to parse the next item's client ID from garbage — causing the cascade of `invalid thing id (0)` errors.

**C++ reference:** `src/networkmessage.cpp` `NetworkMessage::addItem` lines ~91–115:
```cpp
if (it.isAnimation) {
    addByte(0xFE); // random phase
}
addByte(0x00); // duration
```

**Fix:** Add `is_animation: bool` to `ItemStack` in `map_description.rs`. Pass it to `write_item_template`. Write `0xFE` before the duration byte when true.

```rust
// ItemStack — add field:
pub is_animation: bool,

// write_item_template — add parameter and logic:
pub fn write_item_template(msg: &mut NetworkMessage, client_id: u16, count: u8, stackable: bool, is_animation: bool) {
    msg.write_u16(client_id);
    msg.write_u8(0xFF); // MARK_UNMARKED
    if stackable {
        msg.write_u8(count);
    }
    if is_animation {
        msg.write_u8(0xFE); // random phase
    }
    msg.write_u8(0x00); // duration
}
```

In `login_out.rs` `build_initial_map_packet`, look up `is_animation` from `world.items_db` when building each `ItemStack`.

---

## Fix 3 — Item client ID lookup must use ItemDatabase

**File:** `crates/tfs-rust-core/src/login_out.rs` — `build_initial_map_packet`

**Bug:** OTBM tiles store server IDs. The protocol requires client IDs (from OTB `ITEM_ATTR_CLIENTID`). If `client_id_for_server` returns 0 for any item, OTClient rejects the tile with `invalid thing id (0)`.

**Fix:** Already partially done — `world.items_db.client_id_for_server(gid)` is called and items with `cid == 0` are skipped. Verify `items_db` is populated from the OTB loader and that `client_id_for_server` returns the correct value for all ground tile IDs in the map. If the OTB loader isn't wired into `GameWorld` yet, that's the root cause.

Check: after map load, log how many items have `client_id == 0` in the database. If it's most of them, the OTB loader's `ITEM_ATTR_CLIENTID` parsing is broken.

---

## Fix 4 — `send_map_description_packet` uses wrong origin offset

**File:** `crates/tfs-rust-net/src/map_description.rs`

**Current code:**
```rust
let origin_x = center.x as i32 - MAX_CLIENT_VIEWPORT_X;
let origin_y = center.y as i32 - MAX_CLIENT_VIEWPORT_Y;
```

**C++ reference:** `src/protocolgame.cpp` `sendMapDescription`:
```cpp
GetMapDescription(pos.x - Map::maxClientViewportX, pos.y - Map::maxClientViewportY, pos.z, ...)
```

This matches. ✓ But verify `MAX_CLIENT_VIEWPORT_X` and `MAX_CLIENT_VIEWPORT_Y` are set to the correct values for protocol 1098. TFS uses `maxClientViewportX = 8`, `maxClientViewportY = 6` (18×14 viewport). Check `tfs_rust_common::protocol_constants`.

---

## Fix 5 — `GetFloorDescription` loop bounds

**C++ reference:**
```cpp
for (int32_t nz = startz; nz != endz + zstep; nz += zstep)
```

**Your Rust:**
```rust
loop {
    get_floor_description(..., nz, ...);
    if nz == endz { break; }
    nz += zstep;
}
```

This is equivalent. ✓

---

## Fix 6 — `GetTileDescription` environmental effects u16

**C++ reference:** `GetTileDescription` writes `msg.add<uint16_t>(0x00)` at the start of every tile.

**Your Rust:** `msg.write_u16(0)` — matches. ✓

For protocol 1098, `GameEnvironmentEffect` is on and `GameTibia12Protocol` is off, so this `u16` is always present. Correct.

---

## Fix 7 — `write_add_creature` missing `GameNewWalking` check

**Protocol reference doc:** `GameNewWalking` is **off** by default for stock 1098. Do NOT send ground-speed `u16` + blocking `u8` before the creature stack. Your current `write_add_creature` does not send these — correct. ✓

---

## Priority order

1. **Fix 1** (XTEA inner length) — without this, every packet after the first is misread
2. **Fix 2** (animation byte) — without this, every tile with an animated ground item corrupts the stream
3. **Fix 3** (client ID lookup) — without this, tiles with unknown items are skipped or corrupt
4. **Fix 4** (verify viewport constants) — low risk but worth confirming

Fix 1 alone may make the map appear if the item database is populated correctly. Fix 2 is required for any map with animated ground tiles (which is most of them).
