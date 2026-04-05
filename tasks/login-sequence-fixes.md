# Login Sequence Fixes — Protocol 1098

All errors come from OTClient parsing packets that are either too short or have wrong field layout.
Fix them in order — each one unblocks the next.

---

## Error Summary

| OTClient error | Opcode | Root cause |
|---|---|---|
| `prev opcode 0x17, eof` | LoginSuccess | Missing 3 speed-law doubles + trailing fields |
| `prev opcode 0x43, eof` | OTCFeatures | Packet too short or wrong field count |
| `prev opcode 0x64, invalid thing id 0` | MapDescription | Ground item client ID is 0 (server ID ≠ client ID) |
| `prev opcode 0x82, eof` | WorldLight | Packet too short |
| `prev opcode 0xA7, eof` | FightModes | Missing 4th byte (PVP mode) |

---

## Fix 1 — `0x17` LoginSuccess: add speed-law doubles

**C++ reference:** `src/protocolgame.cpp` `sendAddCreature` (line ~2772)

Protocol 1098 has `GameNewSpeedLaw` enabled. After the beat duration `u16`, you must send three doubles:

```
u8   opcode = 0x17
u32  player ID
u16  beat duration = 0x32 (50)
f64  speedA = 857.36      ← MISSING
f64  speedB = 261.29      ← MISSING
f64  speedC = -4795.01    ← MISSING
u8   can report bugs (0x00 or 0x01)
u8   can change pvp framing = 0x00
u8   expert mode button = 0x00
u16  store URL string length = 0x0000
u16  coin package size = 25
```

Doubles are 8 bytes each, little-endian IEEE 754. Total packet body = 4+2+8+8+8+1+1+1+2+2 = 37 bytes after opcode.

---

## Fix 2 — `0xA7` FightModes: add PVP mode byte

**C++ reference:** `src/protocolgame.cpp` `sendFightModes` (line ~2718)

`GamePVPMode` is on for 1098. The packet needs 4 bytes after the opcode, not 3:

```
u8  opcode = 0xA7
u8  fight mode   (1=offensive, 2=balanced, 3=defensive)
u8  chase mode   (0=stand, 1=follow)
u8  secure mode  (0=off, 1=on)
u8  pvp mode     ← MISSING (0 = dove/no-pvp indicator)
```

In your `send_fight_modes` builder, add the 4th `u8` parameter and write it.

---

## Fix 3 — `0x64` MapDescription: item client ID is 0

**C++ reference:** `src/protocolgame.cpp` `GetTileDescription` (line ~2645)

OTClient rejects item ID 0 as invalid. Your `tile_from_data` in `crates/tfs-rust-core/src/map/mod.rs` stores the raw server item ID from the OTBM file as the ground item. But the protocol requires the **client ID** (from the OTB file's `ITEM_ATTR_CLIENTID` field).

The fix: when building the map description packet, look up each item's `client_id` from `ItemDatabase` and send that instead of the server ID. If `client_id` is 0 for an item, skip it or log a warning.

In `map_description.rs`, the `ItemStack.client_id` field is already named correctly — the issue is upstream where tiles are populated. The `TileData` from the OTBM loader stores server IDs. You need to resolve them to client IDs when constructing `TileContent` for the map description.

---

## Fix 4 — `0x82` WorldLight: verify packet length

**C++ reference:** `src/protocolgame.cpp` `AddWorldLight` (line ~3322)

The packet is exactly 3 bytes:

```
u8  opcode = 0x82
u8  light level (0xFF if GM/access player, else world light level)
u8  light color (default = 215 = 0xD7)
```

Check your `send_world_light` builder — if it's sending more than 2 bytes after the opcode, trim it. The default world light at startup is level=0, color=215.

---

## Fix 5 — `0x43` OTCFeatures: verify field count

**C++ reference:** `src/protocolgame.cpp` `sendOTCFeatures` (line ~1564 area)

The C++ sends feature 68 (GameUnjustifiedPoints) enabled:

```
u8   opcode = 0x43
u16  feature count = 1
u8   feature ID = 68
u8   enabled = 1
```

Total = 5 bytes after opcode. Check your `send_otcv8_features` builder produces exactly this. The error `eof` after `0x43` means OTClient tried to read more bytes than you sent.

---

## Fix 6 — `0xA0` Stats: verify all fields present

**C++ reference:** `src/protocolgame.cpp` `AddPlayerStats` (line ~3246)

For protocol 1098 the stats packet is:

```
u8   opcode = 0xA0
u16  health
u16  max health
u32  free capacity
u32  total capacity
u64  experience
u16  level
u8   level percent
u16  base xp gain rate = 100
u16  xp voucher = 0
u16  low level bonus = 0
u16  xp boost = 0
u16  stamina multiplier = 100
u16  mana
u16  max mana
u8   magic level
u8   base magic level
u8   magic level percent
u8   soul
u16  stamina minutes
u16  base speed / 2
u16  regeneration ticks / 1000 (or 0)
u16  offline training time / 60 / 1000
u16  xp boost time = 0
u8   store xp boost enabled = 0
```

---

## Fix 7 — `0xA1` Skills: verify special skills tail

**C++ reference:** `src/protocolgame.cpp` `AddPlayerSkills` (line ~3289)

After the 7 regular skills (each `u16` level + `u16` base + `u8` percent), send the special skills:

```
u8   opcode = 0xA1
// For each of 7 skills (fist, club, sword, axe, dist, shield, fishing):
u16  skill level
u16  base skill level
u8   skill percent
// Then for each special skill (SPECIALSKILL_FIRST to SPECIALSKILL_LAST):
u16  value (capped at 100)
u16  base = 0
```

Check how many special skills your enum defines — TFS has 12 (`SPECIALSKILL_LAST = 11`).

---

## Fix 8 — `0x9F` BasicData: spell list length

**C++ reference:** `src/protocolgame.cpp` `sendBasicData` (line ~1564)

```
u8   opcode = 0x9F
u8   premium = 0 or 1
u32  premium ends at (unix timestamp, 0 if not premium)
u8   vocation client ID
u16  known spell count = 255 (0xFF)
// Then 255 bytes: spell IDs 0x00 through 0xFE
```

This is a large packet (261 bytes). Make sure your builder writes all 255 spell ID bytes.

---

## Correct Login Burst Order

The complete sequence `sendAddCreature` sends for the player, in order:

```
1.  0x32  ExtendedOpcode (OTCv8 init, if otclientV8 detected)
2.  0x43  OTCFeatures (sendOTCFeatures)
3.  0x17  LoginSuccess (player ID, beat, speed doubles, flags)
4.  0x0A  PendingStateEntered
5.  0x43  OTCFeatures again (sendOTCFeatures called twice in C++)
6.  0x0F  EnterWorld
7.  0x64  MapDescription
8.  0x83  MagicEffect (teleport effect at login position)
9.  0x79  InventoryItem × 9 (slots 1-9, empty = just opcode + slot byte)
10. 0x79  StoreInbox slot
11. 0xA0  Stats (sendStats → AddPlayerStats)
12. 0xB7  UnjustifiedStats (sendUnjustifiedStats, called inside sendStats)
13. 0x9F  BasicData (sendBasicData)
14. 0xA1  Skills (sendSkills)
15. 0x82  WorldLight
16. 0x8D  CreatureLight
17. 0xD2  VIPEntries (empty list = opcode + u16(0))
18. 0x9F  BasicData again (C++ calls sendBasicData twice)
19. 0xA2  Icons (sendIcons → u16 icon bitmask)
```

Note: `sendOTCFeatures` is called **before** `0x17` in the C++ `login()` function (line ~172), then again inside `sendAddCreature` (line ~2800). Make sure your sequence matches this order.
