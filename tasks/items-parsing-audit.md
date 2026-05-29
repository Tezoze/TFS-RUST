# Items Parsing Audit — Full Parity Report

**Date:** 2026-04-22
**Scope:** `crates/tfs-rust-content/src/otb.rs` + `items.rs` vs TFS 1.4.2 C++ (`src/items.cpp`, `src/items.h`, `src/itemloader.h`)
**Method:** Line-by-line comparison of `loadFromOtb` / `parseItemNode` / `ItemType` defaults.

---

## Executive Summary

The OTB parser covers the core fields C++ reads (SERVERID, CLIENTID, SPEED, LIGHT2, TOPORDER, WAREID, CLASSIFICATION) plus several extras C++ skips. The items.xml parser has two **runtime-breaking logic bugs** (`readable`/`writeable`), one **missing function** (`is_known_xml_key` — compilation blocker), several missing structured fields (`blocking`, `allowdistread`, `hitchance`), and dozens of keys that C++ maps to concrete `ItemType` fields that the Rust code only stores in the flat `xml_attributes` HashMap. Abilities (skills, absorb, suppress, stats, regen, element) are unimplemented as typed fields — acceptable for now but should be tracked as Phase 3.

---

## Bug Severity Legend

| Sev | Meaning |
|-----|---------|
| **BLOCKER** | Prevents compilation |
| **CRITICAL** | Wrong runtime behavior, protocol/gameplay breakage |
| **HIGH** | Silent wrong values, gameplay divergence |
| **MEDIUM** | Non-parity deviation that is harmless for typical datapacks but wrong against the spec |
| **LOW** | Cosmetic / warning-only |

---

## Part 1 — OTB Parsing (`otb.rs`)

### OTB-1 MEDIUM — Rust parses OTB attributes C++ explicitly skips

C++ `loadFromOtb` (`src/items.cpp` lines 307–399) handles exactly **7** item attributes in its `switch` and routes everything else to `default: stream.skip(datalen)`. The Rust `apply_attr` handles those 7 **plus** five additional ones that C++ ignores:

| OTB Attribute | C++ | Rust |
|---|---|---|
| `ITEM_ATTR_NAME (0x12)` | skipped | parsed → `item.name` |
| `ITEM_ATTR_DESCR (0x13)` | skipped | parsed → `item.description` |
| `ITEM_ATTR_MAXITEMS (0x16)` | skipped | parsed → `item.max_items` |
| `ITEM_ATTR_WEIGHT (0x17)` | skipped | parsed → `item.weight` |
| `ITEM_ATTR_ROTATETO (0x1E)` | skipped | parsed → `item.rotate_to` |

**Impact:** For items present in OTB but absent from items.xml, C++ yields empty name / 0 weight / default max_items=8 / 0 rotateTo. Rust populates these from OTB. For items present in both, items.xml overwrites so this is mostly harmless. Still a parity deviation.

**Fix:** Remove the five extra cases from `apply_attr` (or keep behind a `#[cfg(feature = "otb_extra_attrs")]` flag) so unknown attrs fall through to a skip, matching C++ exactly.

### OTB-2 LOW — `clientIdToServerIdMap` includes `clientId == 0`

C++ calls `clientIdToServerIdMap.emplace(clientId, serverId)` for every item node unconditionally, including those where `clientId == 0`. The Rust `build_client_to_server_map` skips `client_id == 0`. This is actually **better** behavior (C++ `getItemIdByClientId` guards against `spriteId < 100` anyway), but it is a technical deviation.

**Fix:** Low priority. Document the intentional deviation with a comment.

---

## Part 2 — items.xml Parsing (`items.rs`)

### XML-1 BLOCKER — `is_known_xml_key` is called but never defined

`apply_xml_attribute` line 611 calls `is_known_xml_key(&k)` which does not exist anywhere in the crate.

```
// crates/tfs-rust-content/src/items.rs line 611
if !is_known_xml_key(&k) {
    warn_unknown_xml_key_once(item_id, &k);
}
```

This is a compilation error the moment any code path reaches the `_ =>` arm and the function is not inlined or defined. The code must have a stub elsewhere or this was a planned-but-incomplete function.

**Fix (Phase 1):** Define `fn is_known_xml_key(k: &str) -> bool` listing every key that `apply_xml_attribute` handles in its `match` arms, plus every known-but-xml_attributes-only key that should not emit a warning. Matches C++ behavior where *all* keys in `ItemParseAttributesMap` are recognized and unknown keys print a warning.

---

### XML-2 CRITICAL — `readable` parsing is backwards

C++ (`src/items.cpp` line 709):
```cpp
case ITEM_PARSE_READABLE: {
    it.canReadText = valueAttribute.as_bool();
    break;
}
```

Rust (`items.rs` lines 553–557):
```rust
"readable" => {
    if parse_xml_bool(value).unwrap_or(false) {
        item.can_write_text = false;  // ← WRONG field, wrong operation
    }
}
```

Two errors:
1. Sets `can_write_text` instead of `can_read_text`.
2. Only fires when value is `true`; silently does nothing when `readable=false`, so `canReadText` is never cleared.

**Fix (Phase 1):**
```rust
"readable" => {
    if let Some(v) = parse_xml_bool(value) {
        item.can_read_text = v;
    }
}
```

---

### XML-3 CRITICAL — `writeable` does not propagate to `can_read_text`

C++ (`src/items.cpp` lines 713–716):
```cpp
case ITEM_PARSE_WRITEABLE: {
    it.canWriteText = valueAttribute.as_bool();
    it.canReadText = it.canWriteText;  // ← also sets canReadText
    break;
}
```

Rust (`items.rs` lines 558–562):
```rust
"writeable" => {
    if parse_xml_bool(value).unwrap_or(false) {
        item.can_write_text = true;  // ← missing canReadText propagation
    }
}
```

Two errors:
1. `can_read_text` is never set when `writeable` is processed.
2. `writeable=false` does not clear `can_write_text` (parse only fires on `true`).

**Fix (Phase 1):**
```rust
"writeable" => {
    if let Some(v) = parse_xml_bool(value) {
        item.can_write_text = v;
        item.can_read_text = v;
    }
}
```

---

### XML-4 HIGH — `blocking` key is not handled

C++ (`src/items.cpp` line 1346):
```cpp
case ITEM_PARSE_BLOCKING: {
    it.blockSolid = valueAttribute.as_bool();
    break;
}
```

Rust has no case for `"blocking"`. The key goes to `xml_attributes` only. Items with `<attribute key="blocking" value="1"/>` will not have `FLAG_BLOCK_SOLID` behaviour overridden, breaking collision on any such item.

**Fix (Phase 1):** Add a `block_solid_override: Option<bool>` field to `ItemType` (mirroring the existing `moveable_override` pattern) and apply it via `"blocking"`:
```rust
"blocking" => {
    item.block_solid_override = parse_xml_bool(value);
}
```
Update `block_solid()` accessor to consult the override before the flag.

---

### XML-5 HIGH — `allowdistread` key is not handled

C++ (`src/items.cpp` line 1351):
```cpp
case ITEM_PARSE_ALLOWDISTREAD: {
    it.allowDistRead = booleanString(valueAttribute.as_string());
    break;
}
```

Rust has no case for `"allowdistread"`. Books/signs that rely on distance reading will silently use only the OTB flag, ignoring any XML override.

**Fix (Phase 1):** Add `"allowdistread"` to `apply_xml_attribute`, toggling or setting the `FLAG_ALLOWDISTREAD` bit (or a new override field). Since C++ sets the direct field (not an override), it is cleanest to add `allow_dist_read_override: Option<bool>` and update the accessor.

---

### XML-6 HIGH — `hitchance` (per-shot chance) is entirely missing as a field

C++ has **two separate** fields:
- `int8_t hitChance` — clamped to `[-100, 100]` — set by `ITEM_PARSE_HITCHANCE` (`hitchance` key)
- `int32_t maxHitChance` — clamped to `[0, 100]` — set by `ITEM_PARSE_MAXHITCHANCE` (`maxhitchance` key)

Rust has one field `hit_chance: i32` which is mapped to `"maxhitchance"`. The `"hitchance"` key has no case and the field semantics are conflated.

**Fix (Phase 1):**
- Rename Rust `hit_chance` → `max_hit_chance` to match C++ `maxHitChance`.
- Add `hit_chance: i8` for C++ `hitChance`, handled by `"hitchance"`, clamped `[-100, 100]`.

---

### XML-7 MEDIUM — `maxhitchance` is not clamped to `[0, 100]`

C++ (`src/items.cpp` line 856):
```cpp
it.maxHitChance = std::min<uint32_t>(100, pugi::cast<uint32_t>(valueAttribute.value()));
```

Rust (`items.rs` line 606):
```rust
"maxhitchance" => {
    if let Ok(v) = value.parse::<i32>() {
        item.hit_chance = v;  // ← no clamp
    }
}
```

**Fix (Phase 1):** `item.max_hit_chance = v.clamp(0, 100) as u32;` after renaming from Phase 1 XML-6.

---

### XML-8 MEDIUM — `slottype` pipe-splitting is non-parity

C++ `ITEM_PARSE_SLOTTYPE` compares the lowercased value string as a single token. It does not support `|`-separated multi-tokens. Rust splits on `'|'`.

**Impact:** If a datapack ever ships `slottype="head|body"`, C++ ignores it (unknown string → warning), Rust silently applies both. Non-breaking in practice for standard datapacks but diverges from spec.

**Fix (Phase 2):** Remove the split loop. Accept only a single token like C++. Emit a warning for unrecognized tokens.

---

### XML-9 MEDIUM — Duplicate item handling differs from C++ for id-range entries

C++ `parseItemNode` returns early if `it.name` is not empty (duplicate check). The early return skips **all** attribute processing for the duplicate id.

Rust removes duplicate IDs from `current_ids` before processing, which achieves the same result for simple `id=` entries but behaves differently when a `fromid/toid` range partially overlaps a previously-defined range — Rust warns and skips per-ID; C++ wouldn't encounter this since it calls `parseItemNode` per-id in sequence.

**Fix (Phase 2):** Confirm behavior is equivalent under realistic datapacks. Add a test fixture for a partial range overlap.

---

### XML-10 LOW — `rotateto` parsed as `u16` in Rust, `int32_t` in C++

C++ stores `rotateTo` as `uint16_t` (in `items.h`) but casts via `pugi::cast<int32_t>`. Rust parses as `u16`. For realistic item IDs this is fine, but the type is technically wider in C++.

**Fix (Phase 3):** Change Rust `rotate_to: u16` → `rotate_to: u32` for strict parity.

---

## Part 3 — Missing Structured Fields (xml_attributes-only today)

These keys are recognized by C++ `ItemParseAttributesMap`, stored to concrete `ItemType` fields, and directly affect runtime behavior. Rust stores them in `xml_attributes` only. Phase 2 and 3 work should migrate them to typed fields as the relevant subsystems are implemented.

### Phase 2 targets (gameplay-relevant, runtime paths exist or imminent)

| XML Key | C++ Field | Rust Status |
|---|---|---|
| `floorchange` | `it.floorChange` (u8 bitmask) | xml_attributes only |
| `corpsetype` | `it.corpseType` (RaceType_t) | xml_attributes only |
| `fluidsource` | `it.fluidSource` (FluidTypes_t) | xml_attributes only |
| `shoottype` | `it.shootType` (ShootType_t) | xml_attributes only |
| `effect` | `it.magicEffect` (MagicEffectClasses) | xml_attributes only |
| `stopduration` | `it.stopTime` (bool) | xml_attributes only |
| `decayto` | `it.decayTo` (i32, default -1) | xml_attributes only |
| `duration` | `it.decayTime` (u32) | xml_attributes only |
| `showduration` | `it.showDuration` (bool) | xml_attributes only |
| `charges` | `it.charges` (u32) | xml_attributes only (but `charges_default()` reads it) |
| `showcharges` | `it.showCharges` (bool) | xml_attributes only |
| `showattributes` | `it.showAttributes` (bool) | xml_attributes only |
| `transformequipto` | `it.transformEquipTo` (u16) | xml_attributes only |
| `transformdeequipto` | `it.transformDeEquipTo` (u16) | xml_attributes only |
| `transformto` | `it.transformToFree` (u16) | xml_attributes only |
| `destroyto` | `it.destroyTo` (u16) | xml_attributes only |
| `maletransformto` / `malesleeper` | `it.transformToOnUse[MALE]` | xml_attributes only |
| `femaletransformto` / `femalesleeper` | `it.transformToOnUse[FEMALE]` | xml_attributes only |
| `leveldoor` | `it.levelDoor` (u32) | xml_attributes only |
| `partnerdirection` | `it.bedPartnerDir` (Direction) | xml_attributes only |
| `writeonceitemid` | `it.writeOnceItemId` (u16) | xml_attributes only |
| `runespellname` | `it.runeSpellName` (String) | xml_attributes only |
| `supply` | `it.supply` (bool) | xml_attributes only |
| `field` | `it.group=MAGICFIELD`, `it.combatType`, `it.conditionDamage` | xml_attributes partial (no sub-attr parse → no damage config) |

### Phase 3 targets (Abilities subsystem — deferred until combat/equip layer)

All keys mapping to `ItemType::abilities` in C++:

- `speed` → `abilities.speed` (equipment speed bonus)
- `invisible` → `abilities.invisible`
- `healthgain` / `healthticks` / `managain` / `manaticks` → regen fields + `abilities.regeneration = true`
- `manashield` → `abilities.manaShield`
- `skillsword` / `skillaxe` / `skillclub` / `skilldist` / `skillfish` / `skillshield` / `skillfist` → `abilities.skills[SKILL_*]`
- `maxhitpoints` / `maxhitpointspercent` / `maxmanapoints` / `maxmanapointspercent` / `magicpoints` / `magicpointspercent` → `abilities.stats / statsPercent`
- `criticalhitchance` / `criticalhitamount` / `lifeleechchance` / `lifeleechamount` / `manaleechchance` / `manaleechamount` → `abilities.specialSkills`
- `fieldabsorbpercentenergy` / `fieldabsorbpercentfire` / `fieldabsorbpercentpoison` → `abilities.fieldAbsorbPercent`
- `absorbpercentall` / `absorbpercentelements` / `absorbpercentmagic` / `absorbpercent{energy,fire,poison,ice,holy,death,lifedrain,manadrain,drown,physical,healing,undefined}` → `abilities.absorbPercent`
- `suppressdrunk` / `suppressenergy` / `suppressfire` / `suppresspoison` / `suppressdrown` / `suppressphysical` / `suppressfreeze` / `suppressdazzle` / `suppresscurse` → `abilities.conditionSuppressions`
- `elementice` / `elementearth` / `elementfire` / `elementenergy` / `elementdeath` / `elementholy` → `abilities.elementDamage` + `abilities.elementType`

---

## Part 4 — `ItemType` Defaults Parity Check

| Field | C++ default | Rust default | Match? |
|---|---|---|---|
| `group` | `ITEM_GROUP_NONE (0)` | `0` | ✓ |
| `maxItems` | `8` | `8` | ✓ |
| `slotPosition` | `SLOTP_HAND (LEFT\|RIGHT = 0x30)` | `SLOTP_HAND_DEFAULT (0x30)` | ✓ |
| `showCount` | `true` | `true` | ✓ |
| `replaceable` | `true` | `true` | ✓ |
| `walkStack` | `true` | `true` | ✓ |
| `shootRange` | `1` | `0` | ✗ **WRONG** |
| `maxHitChance` | `-1` | `0` | ✗ **WRONG** |
| `decayTo` | `-1` | N/A (no field) | missing field |
| `canReadText` | `false` | derived from OTB flag | ✓ (via accessor) |

**Fix (Phase 1):** `shoot_range` default must be `1` not `0`. `max_hit_chance` default must be `-1` (signed, as C++ `int32_t maxHitChance = -1`).

---

## Phased Fix Plan

### Phase 1 — Correctness Blockers (fix immediately, all high/critical/blocker)

1. **XML-1 BLOCKER:** Define `fn is_known_xml_key(k: &str) -> bool` listing all match arms + known xml_attributes-only keys.
2. **XML-2 CRITICAL:** Fix `readable` to set `can_read_text`, not `can_write_text`.
3. **XML-3 CRITICAL:** Fix `writeable` to also set `can_read_text = v`.
4. **XML-4 HIGH:** Add `block_solid_override: Option<bool>` field + `"blocking"` match arm + update `block_solid()` accessor.
5. **XML-5 HIGH:** Add `allow_dist_read_override: Option<bool>` field + `"allowdistread"` match arm + update `allow_dist_read()` accessor.
6. **XML-6 HIGH:** Split `hit_chance` into separate `hit_chance: i8` (for `hitchance`) and `max_hit_chance: i32` (for `maxhitchance`, default `-1`).
7. **XML-7 MEDIUM:** Clamp `max_hit_chance` to `[0, 100]` on parse.
8. **Default parity:** Set `shoot_range` default to `1`. Set `max_hit_chance` default to `-1`.

### Phase 2 — Structured Fields for Active Subsystems

Migrate xml_attributes-only keys to typed fields as each subsystem is built. Priority order (most runtime-critical first):

1. `floorchange` → `floor_change: u8` bitmask (tile system)
2. `fluidsource` → `fluid_source: u8` (splash/fluid container protocol)
3. `charges` → `charges: u32` (currently read back from xml_attributes via `charges_default()` — promote to typed field)
4. `decayto` / `duration` / `stopduration` → decay subsystem fields (default `decayTo = -1`)
5. `transformto` / `destroyto` / `transformequipto` / `transformdeequipto` / `maletransformto` / `femaletransformto` → transform fields
6. `shoottype` / `effect` → typed enums for projectile/magic effect
7. `corpsetype` → `RaceType` for loot/corpse system
8. `field` + sub-attribute parsing → full magic field `CombatType` + `ConditionDamage` config
9. `supply` / `showcharges` / `showattributes` / `showduration` / `writeonceitemid` / `runespellname` / `leveldoor` / `partnerdirection` → misc ItemType fields
10. **XML-8:** Remove `slottype` pipe-splitting, accept single token only (C++ parity).
11. **OTB-1:** Remove the 5 extra OTB attrs that C++ skips (NAME, DESCR, MAXITEMS, WEIGHT, ROTATETO) from `apply_attr`.

### Phase 3 — Abilities Subsystem

Implement `ItemAbilities` struct and populate from all XML keys listed in Part 3 Phase 3 table. This unblocks equipment stat bonuses, elemental damage weapons, absorption gear, regeneration items, and condition suppression items.

### Phase 4 — Diagnostics Parity

1. **XML-9:** Add test for partial range overlap duplicate detection.
2. **OTB-2:** Document / test the `clientId == 0` skip is intentional.
3. Mirror C++ warning output for unknown type tokens, unknown ammo types, unknown weapontype strings.
4. Mirror C++ bed-item sanity check: warn if `transformToFree != 0 || transformToOnUse[*] != 0` but `type != ITEM_TYPE_BED`.
5. `buildInventoryList()` equivalent for use by protocol/UI layers.

---

## C++ Reference Map

| Finding | C++ File | Function | Lines |
|---|---|---|---|
| OTB attribute enum | `src/itemloader.h` | `itemattrib_t` | 102–140 |
| OTB load loop | `src/items.cpp` | `Items::loadFromOtb` | 307–466 |
| XML parse dispatch | `src/items.cpp` | `Items::parseItemNode` | 533–1385 |
| ItemType fields + defaults | `src/items.h` | `class ItemType` | 196–381 |
| Abilities struct | `src/items.h` | `struct Abilities` | 160–192 |
| SlotPositionBits | `src/items.h` | `enum SlotPositionBits` | 12–27 |
| ItemParseAttributesMap | `src/items.cpp` | top-level map | 16–139 |
