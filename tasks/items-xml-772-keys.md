# Items.xml 772 keys — parity backlog

Pack attributes that were only stored in `xml_attributes` (WARNs silenced via `KNOWN_XML_KEYS` in [`crates/tfs-rust-content/src/items_xml_keys.rs`](../crates/tfs-rust-content/src/items_xml_keys.rs)). Values still land in `xml_attributes`; most are not typed on `ItemType` or read by simulation yet.

**Goal:** type each key (or confirm OTB already covers it), then wire the matching Use / tile / combat path so observable 772 outcomes match.

Related: [`docs/772_THROW_MOVE_AUDIT.md`](../docs/772_THROW_MOVE_AUDIT.md) § unknown keys, [`docs/772_OTB_OBJECTS_SRV_FLAG_MAPPING.md`](../docs/772_OTB_OBJECTS_SRV_FLAG_MAPPING.md).

---

## Summary

| XML key | Pack examples | Status today | Priority | Implement later? |
|---------|---------------|--------------|----------|------------------|
| `forceuse` | Ladder `1386` | Known key + OTB `FLAG_FORCEUSE` / `force_use()`; **Use stack walk ignores it** | **High** | Yes — Use path |
| `poisondamagecycles` | Poison arrow `2545` | Known key; **stringly** read in `ranged.rs` fallback; live pack uses Lua | Medium | Yes — type + keep Lua primary |
| `replacemagicfields` | Fire fields `1487`… | Known key only; dead for place logic | Medium | Yes — field place |
| `blockpathfind` | Field variants (merged / OTB) | Known key; **OTB `block_path_find()` already drives tile flags** | Low | Confirm XML override; type if needed |
| `specialfieldblockpath` | Some magic fields | Known key only | Low | Yes — pathfind props if gaps appear |

---

## 1. `forceuse`

### Meaning
Object may be used even when not the visually “top” thing on the tile (e.g. ladder under a corpse).

### Reference
| Layer | Where |
|-------|--------|
| Flag | 772 `FORCEUSE` (`enums.hh` / `objects.cc` `"ForceUse"`); OTB `FLAG_FORCEUSE` |
| Decompile | `CheckTopUseObject`, `CheckTopMultiuseObject` — `operate.cc` ~344–416 |
| TVP | `ItemType::forceUse`; tile use picks `forceUse` items |
| Pack | `items.xml` ladder `1386` (`forceuse=true`) |

### Behavior to port
1. When resolving Use / UseEx **map** target, walk the tile stack like `CheckTopUseObject` / `CheckTopMultiuseObject`.
2. Prefer / stop on `force_use()` (and existing priority / liquid-pool rules).
3. If the client’s aimed object is not the chosen “Best”, fail with not-accessible (772 `NOTACCESSIBLE`).

### Current Rust
- OTB: `ItemType::force_use()` exists ([`otb.rs`](../crates/tfs-rust-content/src/otb.rs)).
- XML: stored in `xml_attributes`; **does not** set a typed override if OTB bit missing.
- Use path: ToDo / `resolve_use_object` does **not** implement the FORCEUSE break.

### TODO
- [ ] Apply XML `forceuse` → typed override (or OR into effective `force_use()`).
- [ ] Port top-use / top-multiuse checks into Use enqueue or execute (map tiles only).
- [ ] Regression: creature/corpse on ladder tile → Use still hits ladder.

---

## 2. `poisondamagecycles`

### Meaning
Poison DoT strength for ammo (TVP name). Classic 772 uses ammo `AMMOEFFECTSTRENGTH` → `DAMAGE_POISON_PERIODIC` → `SetTimer(SKILL_POISON, …)`.

### Reference
| Layer | Where |
|-------|--------|
| Decompile | `crcombat.cc` ranged special → `Damage(..., DAMAGE_POISON_PERIODIC)`; `crmain.cc` poison timer |
| TVP / pack | `poisondamagecycles` on poison arrow `2545` |
| Data pack | Prefer `data/scripts/weapons/poison_arrow.lua` (`CONDITION_PARAM_CYCLE`) |

### Behavior to port
- Unscripted fallback: apply periodic poison with cycles from this attribute (not front-loaded Earth HP).
- Scripted path stays Lua.

### Current Rust
- [`ranged.rs`](../crates/tfs-rust-core/src/player/combat/ranged.rs) reads `xml_attributes["poisondamagecycles"]` when no `onUseWeapon`.
- Not a typed `ItemType` field.

### TODO
- [ ] Parse into typed field (e.g. `poison_damage_cycles: u16`) on `ItemType`.
- [ ] Point ranged fallback at the typed field; keep Lua as primary for the live pack.
- [ ] Unit test: typed value drives fallback DoT; scripted ammo unchanged.

---

## 3. `replacemagicfields`

### Meaning
When placing this magic field, replace an existing replaceable field on the tile (fire overwrites fire, etc.).

### Reference
| Layer | Where |
|-------|--------|
| TVP | `ItemType::replaceMagicFields`; `Tile` place / `queryAdd` ~917–930 |
| Decompile | Field / `MAGICFIELD` place rules in `moveuse.cc` (no identical XML name) |
| Pack | Fire (and related) fields `1487`… |

### Behavior to port
On add of a magic field (or item with this flag): if tile already has a replaceable field, remove it before place (or refuse non-replaceable old field per TVP).

### Current Rust
- Known key → `xml_attributes` only.
- Some field-replace coverage via monster `create_field` tests; player / rune / spell place may diverge.

### TODO
- [ ] Typed `replace_magic_fields: bool` from XML (and/or OTB if present).
- [ ] Shared place helper used by combat CREATEITEM, field runes, monster fields.
- [ ] Tests: fire on fire replaces; non-replaceable old field blocks when required.

---

## 4. `blockpathfind`

### Meaning
Item contributes to “blocks pathfinding” on its tile (`FLAG_BLOCK_PATHFIND` / TVP `blockPathFind`).

### Reference
| Layer | Where |
|-------|--------|
| OTB | `FLAG_BLOCK_PATHFIND` → `ItemType::block_path_find()` |
| TVP | `CONST_PROP_NOFIELDBLOCKPATH` / `IMMOVABLENOFIELDBLOCKPATH` → tile flags |
| Rust | [`map/mod.rs`](../crates/tfs-rust-core/src/map/mod.rs) `apply_item_tile_flags` already uses `block_path_find()` |

### Behavior
Pathfinding / `queryAdd` with pathfinding flag should avoid those tiles. Magic fields often **excluded** from `NOFIELDBLOCKPATH` (field itself is the hazard; separate from solid block).

### Current Rust
- OTB path is live.
- XML key known but unused as override.

### TODO
- [ ] Audit pack: any item with XML `blockpathfind` but **without** OTB bit?
- [ ] If yes: typed XML override into effective `block_path_find()`.
- [ ] If no: document “OTB sufficient; XML is documentary” and leave known-key only.

---

## 5. `specialfieldblockpath`

### Meaning
TVP-only-style property: field participates in a **special** pathfinding block (`CONST_PROP_SPECIALFIELDBLOCKPATH` → tile state used when pathfinding).

### Reference
| Layer | Where |
|-------|--------|
| TVP | `ItemType::specialFieldBlockPath`; `Item::hasProperty`; tile set/reset flags |
| Decompile | No identical name; path blocking via other object flags |

### Current Rust
- Known key → `xml_attributes` only; unread.

### TODO
- [ ] Confirm whether 772 gameplay needs this vs plain `blockpathfind` / field damage.
- [ ] If needed: typed bool + tile flag apply/clear on add/remove; pathfind `queryAdd` honors it.
- [ ] If not needed for 772 profile: note “TVP/1098 only” and skip.

---

## Suggested implementation order

1. **`forceuse`** — player-visible (ladders under bodies).
2. **`poisondamagecycles`** — type the existing fallback (small, safe).
3. **`replacemagicfields`** — field place consistency.
4. **`blockpathfind` XML override** — only if audit finds OTB gaps.
5. **`specialfieldblockpath`** — only if pathfind bugs show up around fields.

## Non-goals (already done)

- [x] Silence `unknown items.xml key` WARNs for these five (`KNOWN_XML_KEYS`).
- [x] Keep values in `xml_attributes` so nothing is dropped at load.

## Lessons hook

When implementing, add a line to [`tasks/lessons.md`](lessons.md) if decompile outcome ≠ TFS/TVP flag shape (especially `forceuse` stack walk vs `specialfieldblockpath`).
