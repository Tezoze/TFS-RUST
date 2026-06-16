# 772 `objects.srv` ↔ `items.otb` flag & attribute mapping

Reference for porting 772 mechanics (decompile / `objects.srv`) against the TVP/TFS content stack (`items.otb` + `items.xml`). OTB uses **different names and packaging** than 772, but the same item universe for 7.72 shards.

## Authorities

| Layer | Path | Role |
|-------|------|------|
| OTB wire format | [`reference/tvp-772/gameserver/src/itemloader.h`](../reference/tvp-772/gameserver/src/itemloader.h) | `itemflags_t`, `itemgroup_t`, `itemattrib_t` |
| OTB → runtime | [`reference/tvp-772/gameserver/src/items.cpp`](../reference/tvp-772/gameserver/src/items.cpp) | OTB load + `items.xml` overrides |
| 772 type defs | [`reference/cipsoft-772/tibia-game-master/src/enums.hh`](../reference/cipsoft-772/tibia-game-master/src/enums.hh) | `enum FLAG`, `INSTANCEATTRIBUTE` |
| 772 type load | [`reference/cipsoft-772/tibia-game-master/src/objects.cc`](../reference/cipsoft-772/tibia-game-master/src/objects.cc) | `FlagNames[]`, `TypeAttributeNames[]` |
| 772 data | `reference/cipsoft-772/runtime/dat/objects.srv` | Authoritative flag strings per `TypeID` |
| Rust loader | [`crates/tfs-rust-content/src/otb.rs`](../crates/tfs-rust-content/src/otb.rs) | Mirrors TVP `itemloader.h` |

**Do not conflate:**

- **`Waypoints`** — 772 BANK terrain weight (`objects.srv` attribute; `cract.cc` `TShortway::FillMap`, `NotifyGo`). Stored in OTB as **`ITEM_ATTR_SPEED`** (`ItemType::speed`). Not `items.xml` equipment `speed`.
- **`GetSpeed()`** — creature movement stat; unrelated to tile Waypoints.

### Waypoints → `ITEM_ATTR_SPEED` (patched OTB)

`data/items/items.otb` is **patched offline** so `ITEM_ATTR_SPEED` mirrors walkable `objects.srv` `Waypoints`:

```bash
cargo run -p tfs-rust-content --bin patch-otb-waypoints   # writes items.otb; creates items.otb.bak once
cargo test -p tfs-rust-content audit_objects_srv_waypoints -- --nocapture
```

At **runtime**, Rust reads `ItemType::speed` from OTB only — no `objects.srv` parse on the hot path. Optional dev overlay: [`overlay_otb_speeds_from_objects_srv`](../crates/tfs-rust-content/src/objects_srv.rs) when reference dat is present and OTB has not been re-patched.

**`TypeID` → OTB resolve:** `objects.srv` `TypeID` is the OTB **`server_id`** for game items (try direct lookup first, then `client_id` fallback in [`resolve_server_id`](../crates/tfs-rust-content/src/objects_srv.rs)). A prior audit bug matched `client_id` only, producing false “14 mismatches” (e.g. TypeID 397 → server 394).

After `patch-otb-waypoints` with the fixed resolver, audit should report **843 / 843** walkable BANK exact matches.

---

## OTB `itemflags_t` (bit → TVP `ItemType` field)

From TVP [`itemloader.h`](../reference/tvp-772/gameserver/src/itemloader.h) (`items.cpp` ~413–432).

| Bit | OTB name | TVP field | Notes |
|-----|----------|-----------|-------|
| 0 | `FLAG_BLOCK_SOLID` | `blockSolid` | Overridable by `items.xml` `blocking` |
| 1 | `FLAG_BLOCK_PROJECTILE` | `blockProjectile` | XML `blockprojectile` |
| 2 | `FLAG_BLOCK_PATHFIND` | `blockPathFind` | XML `blockpathfind` |
| 3 | `FLAG_HAS_HEIGHT` | `hasHeight` | |
| 4 | `FLAG_USEABLE` | `useable` | |
| 5 | `FLAG_PICKUPABLE` | `pickupable` | XML `pickupable` / `allowpickupable` |
| 6 | `FLAG_MOVEABLE` | `moveable` | XML `moveable` / `movable` |
| 7 | `FLAG_STACKABLE` | `stackable` | |
| 8–12 | `FLAG_FLOORCHANGE*` | — | Unused in OTB loader |
| 13 | `FLAG_ALWAYSONTOP` | `alwaysOnTop` | |
| 14 | `FLAG_READABLE` | `canReadText` | XML `readable` |
| 15 | `FLAG_ROTATABLE` | `rotatable` | XML `rotateto` |
| 16 | `FLAG_HANGABLE` | `isHangable` | |
| 17 | `FLAG_VERTICAL` | `isVertical` | |
| 18 | `FLAG_HORIZONTAL` | `isHorizontal` | |
| 19 | `FLAG_CANNOTDECAY` | — | Unused |
| 20 | `FLAG_ALLOWDISTREAD` | `allowDistRead` | XML `allowdistread` |
| 21 | `FLAG_UNUSED` | — | |
| 22 | `FLAG_CLIENTCHARGES` | — | Deprecated |
| 23 | `FLAG_LOOKTHROUGH` | `lookThrough` | |
| 24 | `FLAG_ANIMATION` | `isAnimation` | |
| 25 | `FLAG_FULLTILE` | — | Unused |
| 26 | `FLAG_FORCEUSE` | `forceUse` | XML `forceuse` |

TVP 772 OTB has **no** `FLAG_AMMO` / `FLAG_REPORTABLE` bits present in repo-root TFS 1.4.2 `itemloader.h`.

---

## OTB `itemgroup_t` → TVP `ItemTypes_t`

| OTB group | Value | TVP `type` set in loader | 772 `objects.srv` analogue |
|-----------|-------|---------------------------|----------------------------|
| `ITEM_GROUP_GROUND` | 1 | (none) | **`Bank`** on floor tiles |
| `ITEM_GROUP_CONTAINER` | 2 | `ITEM_TYPE_CONTAINER` | **`Container`** |
| `ITEM_GROUP_WEAPON` etc. | deprecated | — | **`Weapon`**, **`Armor`**, … flags (type attrs) |
| `ITEM_GROUP_SPLASH` | 11 | — | splash fluids |
| `ITEM_GROUP_FLUID` | 12 | — | **`LiquidContainer`** / **`LiquidSource`** |
| `ITEM_GROUP_DOOR` | deprecated | `ITEM_TYPE_DOOR` | **`KeyDoor`**, **`LevelDoor`**, … |
| `ITEM_GROUP_MAGICFIELD` | deprecated | `ITEM_TYPE_MAGICFIELD` | **`MagicField`** (+ XML `field`) |
| `ITEM_GROUP_TELEPORT` | deprecated | `ITEM_TYPE_TELEPORT` | **`TeleportAbsolute`** / **`TeleportRelative`** |
| `ITEM_GROUP_KEY` | deprecated | `ITEM_TYPE_KEY` | **`Key`** |
| `ITEM_GROUP_WRITEABLE` | deprecated | — | **`Text`**, **`Write`**, **`WriteOnce`** |

XML `type="depot|mailbox|trashholder|bed|rune"` sets `ItemTypes_t` without a dedicated OTB group bit.

---

## OTB `itemattrib_t` ↔ 772 type attributes

| OTB attribute | TVP field | 772 `objects.srv` attribute | 772 requires flag |
|---------------|-----------|----------------------------|-------------------|
| `ITEM_ATTR_SPEED` | `speed` | **`Waypoints`** | **`Bank`** |
| `ITEM_ATTR_WEIGHT` | `weight` | **`Weight`** | **`Take`** |
| `ITEM_ATTR_ROTATETO` | `rotateTo` | **`RotateTarget`** | **`Rotate`** |
| `ITEM_ATTR_WRITEABLE` | `maxTextLen`, write flags | **`MaxLength`** / **`MaxLengthOnce`** | **`Write`** / **`WriteOnce`** |
| `ITEM_ATTR_LIGHT` / `LIGHT2` | `lightLevel`, `lightColor` | **`Brightness`**, **`LightColor`** | **`Light`** |
| `ITEM_ATTR_MAXITEMS` | `maxItems` | **`Capacity`** | **`Container`** |
| `ITEM_ATTR_DECAY` | `decayTime`, `decayTo` | **`TotalExpireTime`**, **`ExpireTarget`** | **`Expire`** |
| `ITEM_ATTR_WEAPON` / `ARMOR` / … | combat fields | `WeaponAttackValue`, `ArmorValue`, … | **`Weapon`**, **`Armor`**, … |

Full 772 attribute ↔ flag pairing is in `objects.cc` `TypeAttributeFlags[62]` and `TypeAttributeNames[62]`.

---

## 772 `objects.srv` flags (`objects.cc` `FlagNames[66]`)

| Idx | `objects.srv` string | `enum FLAG` | Typical OTB / TVP equivalent |
|-----|---------------------|-------------|------------------------------|
| 0 | `Bank` | `BANK` | **`group == ITEM_GROUP_GROUND`** (`isGroundTile()`), not an OTB bit |
| 1 | `Clip` | `CLIP` | *No OTB bit* — client clip / stack layering |
| 2 | `Bottom` | `BOTTOM` | *No OTB bit* — stack order |
| 3 | `Top` | `TOP` | Partial: `alwaysOnTop` / `alwaysOnTopOrder` |
| 4 | `Container` | `CONTAINER` | `group == ITEM_GROUP_CONTAINER` |
| 5 | `Chest` | `CHEST` | Instance flag; quest chest behavior |
| 6 | `Cumulative` | `CUMULATIVE` | `stackable` + subtype / count |
| 7 | `UseEvent` | `USEEVENT` | Script hook; no OTB bit |
| 8 | `ChangeUse` | `CHANGEUSE` | Transform target attrs |
| 9 | `ForceUse` | `FORCEUSE` | **`FLAG_FORCEUSE`** |
| 10 | `MultiUse` | `MULTIUSE` | Use pipeline; no single OTB bit |
| 11 | `DistUse` | `DISTUSE` | Ranged use; no OTB bit |
| 12 | `MovementEvent` | `MOVEMENTEVENT` | Step script; no OTB bit |
| 13 | `CollisionEvent` | `COLLISIONEVENT` | Collision script; no OTB bit |
| 14 | `SeparationEvent` | `SEPARATIONEVENT` | Separation script; no OTB bit |
| 15 | `Key` | `KEY` | `type == ITEM_TYPE_KEY` |
| 16 | `KeyDoor` | `KEYDOOR` | `type == ITEM_TYPE_DOOR` + door attrs |
| 17 | `NameDoor` | `NAMEDOOR` | door attrs |
| 18 | `LevelDoor` | `LEVELDOOR` | `levelDoor` |
| 19 | `QuestDoor` | `QUESTDOOR` | door attrs |
| 20 | `Bed` | `BED` | `type == ITEM_TYPE_BED` |
| 21 | `Food` | `FOOD` | `nutrition` attr; no OTB bit |
| 22 | `Rune` | `RUNE` | `type == ITEM_TYPE_RUNE` |
| 23 | `Information` | `INFORMATION` | sign / info type |
| 24 | `Text` | `TEXT` | `canReadText` |
| 25 | `Write` | `WRITE` | `canWriteText` |
| 26 | `WriteOnce` | `WRITEONCE` | `writeOnceItemId` |
| 27 | `LiquidContainer` | `LIQUIDCONTAINER` | `group == ITEM_GROUP_FLUID` |
| 28 | `LiquidSource` | `LIQUIDSOURCE` | fluid source attrs |
| 29 | `LiquidPool` | `LIQUIDPOOL` | pool liquid instance attr |
| 30 | `TeleportAbsolute` | `TELEPORTABSOLUTE` | `type == ITEM_TYPE_TELEPORT` |
| 31 | `TeleportRelative` | `TELEPORTRELATIVE` | teleport attrs |
| 32 | **`Unpass`** | **`UNPASS`** | **`FLAG_BLOCK_SOLID`** → `blockSolid` |
| 33 | **`Unmove`** | **`UNMOVE`** | **`!FLAG_MOVEABLE`** → `!moveable` |
| 34 | `Unthrow` | `UNTHROW` | *No OTB bit* — throw rules in 772 engine |
| 35 | **`Unlay`** | **`UNLAY`** | TVP XML **`unlay`** → `blockPickupable` (see note below) |
| 36 | **`Avoid`** | **`AVOID`** | *No OTB bit* — hazard tiles (`MovePossible`); often magic fields |
| 37 | `MagicField` | `MAGICFIELD` | `type == ITEM_TYPE_MAGICFIELD` / XML `field` |
| 38 | `RestrictLevel` | `RESTRICTLEVEL` | `minReqLevel` |
| 39 | `RestrictProfession` | `RESTRICTPROFESSION` | `vocationString` |
| 40 | `Take` | `TAKE` | `pickupable` + `weight` |
| 41 | `Hang` | `HANG` | `isHangable` |
| 42 | `HookSouth` | `HOOKSOUTH` | hang + orientation |
| 43 | `HookEast` | `HOOKEAST` | hang + orientation |
| 44 | `Rotate` | `ROTATE` | `rotatable` + `rotateTo` |
| 45 | `Destroy` | `DESTROY` | `destroyTo` |
| 46 | `Clothes` | `CLOTHES` | `slotPosition` |
| 47 | `SkillBoost` | `SKILLBOOST` | `abilities.skills` |
| 48 | `Protection` | `PROTECTION` | `abilities.absorbPercent` |
| 49 | `Light` | `LIGHT` | `lightLevel` / `lightColor` |
| 50 | `RopeSpot` | `ROPESPOT` | *No OTB bit* |
| 51 | `Corpse` | `CORPSE` | `corpseType` |
| 52 | `Expire` | `EXPIRE` | `decayTime` / `decayTo` |
| 53 | `ExpireStop` | `EXPIRESTOP` | decay stop instance state |
| 54 | `WearOut` | `WEAROUT` | charges / uses |
| 55 | `Weapon` | `WEAPON` | `weaponType`, attack/defense |
| 56 | `Shield` | `SHIELD` | defense attrs |
| 57 | `Bow` | `BOW` | distance weapon attrs |
| 58 | `Throw` | `THROW` | throwable attrs |
| 59 | `Wand` | `WAND` | wand attrs |
| 60 | `Ammo` | `AMMO` | ammo attrs |
| 61 | `Armor` | `ARMOR` | `armor` |
| 62 | `Height` | `HEIGHT` | **`FLAG_HAS_HEIGHT`** |
| 63 | `Disguise` | `DISGUISE` | disguise target attr |
| 64 | `ShowDetail` | `SHOWDETAIL` | look detail; no OTB bit |
| 65 | `Special` | `SPECIALOBJECT` | `SPECIALOBJECT` in enum |

### `Unlay` naming trap

772 **`Unlay`** blocks placing items on a tile (`moveuse.cc`, `operate.cc`). TVP maps XML **`unlay`** to **`blockPickupable`** (`items.cpp` `ITEM_PARSE_UNLAY`), used in `tile.cpp` for pickup blocking — related placement semantics, **not** a direct OTB flag bit. Check `items.xml` per id when parity depends on `Unlay`.

---

## Pathfinding-critical subset

Used by `TShortway::FillMap` and `TMonster::MovePossible` (`cract.cc`, `crnonpl.cc`).

| 772 check | `objects.srv` | Rust / OTB should use |
|-----------|---------------|------------------------|
| Floor present | `getFlag(BANK)` on stack top | First stack object with `is_ground_tile()` **or** walk stack like `GetFirstObject` |
| Blocks walking | `getFlag(UNPASS)` | `block_solid()` on item type |
| Terrain cost | `getAttribute(WAYPOINTS)` | `ItemType::speed` (`ITEM_ATTR_SPEED` in patched `items.otb`) |
| Can't kick | `getFlag(UNMOVE)` | `!moveable()` |
| Hazard | `getFlag(AVOID)` | No OTB bit — `is_magic_field()` + field type / conditions |
| No floor | no `Bank` on stack | `waypoints = -1` |

**FillMap** (`cract.cc:89-103`) only inspects **`GetFirstObject`** (top of map object chain), not ground field alone:

```cpp
if (ObjType.getFlag(BANK) && !ObjType.getFlag(UNPASS)) {
    Waypoints = ObjType.getAttribute(WAYPOINTS);
    if (!Creature->MovePossible(..., false, false)) Waypoints = -1;
}
```

**MovePossible** walks the full stack: creature containers, then `UNPASS` / `AVOID` on items (`crnonpl.cc:2186+`).

---

## `items.xml` overrides (after OTB load)

TVP [`items.cpp`](../reference/tvp-772/gameserver/src/items.cpp) can change OTB-derived flags:

| XML key | Effect |
|---------|--------|
| `blocking` | Sets `blockSolid` (overrides OTB `FLAG_BLOCK_SOLID`) |
| `moveable` / `movable` | Sets `moveable` |
| `blockpathfind` | Sets `blockPathFind` |
| `unlay` | Sets `blockPickupable` |
| `forceuse` | Sets `forceUse` |
| `field` | Forces magic field group/type |
| `type` | Sets `ItemTypes_t` (door, depot, teleport, …) |

Rust [`ItemDatabase`](../crates/tfs-rust-content/src/items.rs) applies the same XML overrides when loading.

---

## Empirical notes (this repo's `items.otb` + `objects.srv`)

Run: `cargo test -p tfs-rust-content audit_objects_srv_waypoints -- --nocapture`

| Check | Result (`data/items/items.otb`, patched) |
|-------|---------------------------------------------|
| Walkable BANK (`Bank`, `!Unpass`, `Waypoints > 0`) | **843** |
| Exact `ITEM_ATTR_SPEED == Waypoints` | **843 / 843** after `patch-otb-waypoints` with fixed resolver |
| Blocked BANK (`Unpass` or `Waypoints == 0`) | **364** |
| OTB ground types with `speed > 0` | **899** |

Flag correlation: `cargo test -p tfs-rust-content audit_objects_srv_flag_correlation -- --nocapture`

| Flag mapping | Match |
|--------------|-------|
| `Bank` → `isGroundTile()` | 1179 / 27 mismatch |
| `Unpass` → `blockSolid` | 1947 / 35 mismatch |
| `Unmove` → `!moveable` | 3572 / 68 mismatch |

Re-patch after `objects.srv` changes: `scripts/patch_otb_waypoints.sh` or `patch-otb-waypoints` binary.

---

## Rust helpers (target API)

Prefer OTB/XML-derived checks, not runtime `objects.srv` parsing, in hot paths:

| 772 flag | Suggested `ItemType` helper |
|----------|----------------------------|
| `BANK` | `is_ground_tile()` (document stack-top rule separately) |
| `UNPASS` | `block_solid()` |
| `UNMOVE` | `!moveable()` |
| `WAYPOINTS` | `speed` (`ITEM_ATTR_SPEED` in patched OTB; FillMap treats `0` as invalid per C++) |
| `AVOID` | `is_magic_field()` + combat/condition data |
| `MAGICFIELD` | `is_magic_field()` |

See also: [`crates/tfs-rust-content/src/otb_patch.rs`](../crates/tfs-rust-content/src/otb_patch.rs) (offline OTB patch), [`objects_srv.rs`](../crates/tfs-rust-content/src/objects_srv.rs) (optional runtime overlay).

---

## Related docs & tests

- [`scripts/audit_objects_srv_waypoints_vs_otb.py`](../scripts/audit_objects_srv_waypoints_vs_otb.py) — Waypoints audit wrapper
- [`scripts/audit_otb_objects_srv_flags.py`](../scripts/audit_otb_objects_srv_flags.py) — flag correlation wrapper
- [`crates/tfs-rust-content/tests/audit_objects_srv_waypoints.rs`](../crates/tfs-rust-content/tests/audit_objects_srv_waypoints.rs)
- [`crates/tfs-rust-content/tests/audit_objects_srv_flag_correlation.rs`](../crates/tfs-rust-content/tests/audit_objects_srv_flag_correlation.rs)
- [`docs/PROTOCOL_VERSIONING.md`](PROTOCOL_VERSIONING.md) — wire vs mechanics axes
