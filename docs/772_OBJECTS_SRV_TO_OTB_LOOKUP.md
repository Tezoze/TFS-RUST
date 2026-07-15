# 772 `objects.srv` → OTB / `items.xml` lookup guide

**Date:** 2026-06-26  
**Companion:** [`772_OTB_OBJECTS_SRV_FLAG_MAPPING.md`](772_OTB_OBJECTS_SRV_FLAG_MAPPING.md) (flag bits, Waypoints, audits)

Practical reference for reading **772 `.sec` sector files** and mapping **TypeIDs** to what the
Rust stack (`items.otb` + `items.xml`) uses at runtime.

---

## Two stacks, one number line

| Consumer | Authoritative source | TypeID meaning |
|----------|---------------------|----------------|
| **772 C++ decompile / `.sec` files** | `reference/cipsoft-772/runtime/dat/objects.srv` | Name, `Bank`/`Unpass`/`Waypoints`, disguise |
| **Rust sim / OTBM** | `data/items/items.otb` + `data/items/items.xml` | OTB flags + XML overrides; `speed` = patched Waypoints |

**Rule:** When auditing `.sec` terrain or explaining in-game layout, use **`objects.srv` names and
flags**. When implementing Rust walkability/pathfinding, use **OTB/XML-derived `ItemType`** (which
should match `objects.srv` after `patch-otb-waypoints` and flag audits).

**Do not trust `items.xml` `name=` for 772 map archaeology** — thousands of ids share the numeric
key but carry TVP/TFS display names that disagree with `objects.srv` (4270+ rough mismatches in a
full scan). The **numeric `id` / `server_id` is the join key**; names are not.

---

## TypeID → OTB resolve (Rust)

From [`crates/tfs-rust-content/src/objects_srv.rs`](../crates/tfs-rust-content/src/objects_srv.rs)
`resolve_server_id`:

1. **By `client_id`:** find the OTB row where `client_id == TypeID` (smallest `server_id` wins for
   duplicate client ids). Use that row's `server_id`.
2. **Fallback:** aligned rows where `server_id == client_id == TypeID` and no distinct client row.

**772 `TypeID` == OTB `client_id`** (the shared 772 sprite/type id). The `.sec`→OTBM conversion
remaps `TypeID → server_id` via `client_id` (verified: ~940/1024 tiles per sector equal
`client_to_server[sec_id]`), so an OTB row's real terrain flags/Waypoints come from
`objects.srv[client_id]`. **Do not resolve `server_id`-first** — for the ~90% of ground rows where
TVP renumbered `server_id != client_id` that reads the *wrong* item (e.g. rock soil client 4402 →
server 4413, whose `server_id` maps to "a mountain" in `objects.srv`), storing walkable rock soil as
`wp0` → monster-impassable. Patch/audit tooling iterates **OTB rows** and joins on `client_id`.

**OTB fields used for 772 mechanics parity:**

| `objects.srv` | OTB / `ItemType` | XML can override |
|---------------|------------------|------------------|
| `Bank` | `group == ITEM_GROUP_GROUND` (`is_ground_tile()`) | rarely |
| `Unpass` | `FLAG_BLOCK_SOLID` → `block_solid()` | `blocking` |
| `Unmove` | `!FLAG_MOVEABLE` → `!moveable()` | `moveable` |
| `Waypoints` | `ITEM_ATTR_SPEED` → `speed` | no (patched offline) |
| `Avoid` | no OTB bit | `field` / magic field type |

Patch Waypoints: `cargo run -p tfs-rust-content --bin patch-otb-waypoints`  
Audit: `cargo test -p tfs-rust-content audit_objects_srv_waypoints -- --nocapture`

---

## Cyclops-field example — same id, different names

Validated against `1014-1002-07.sec` and client screenshot at `(32451, 32065, 7)`.

| TypeID | `objects.srv` (use for `.sec`) | `items.xml` (display only) | Flags (srv) | Visual / role |
|--------|-------------------------------|----------------------------|-------------|---------------|
| **104** | sand | sand | `Bank` wp=160 | Sandy patches in gravel |
| **4515** | grass | rock soil | `Bank` wp=150 | Green grass (SW of field) |
| **4521** | grass | rock soil | `Bank` wp=150 | Grass |
| **4555–4562** | gravel | gravel / rock soil | `Bank` wp=150 | Light gravel clearing (cyclops floor) |
| **4594** | sand | rock soil | `Clip` | Overlay on gravel, not ground |
| **4599** | **shallow water** | sand | `Bank`+`Unpass` wp=0 | Blue pool NW corner — **not sand** |
| **595** | a hole | sand | `Bank` wp=150 | — |
| **602** | a ramp | swamp | `Clip` | Overlay decoration |
| **1099** | **a mountain** | sandstone wall | `Bank`+`Unpass` wp=0, `Disguise→1128` | Dark rock cliff walls — **not void** |
| **1128** | a mountain | oriental wall | `Bank`+`Unpass` wp=0 | Disguise target / rock face |
| **3682** | **a small fir tree** | ramp | `Bottom`+`Unpass` | Trees on grass — **not a ramp** |
| **4458** | a mountain | rocks | `Bottom`+`Unpass` | Rock wall pieces (stack overlays) |
| **1781** | a small stone | big wine cask | `Clip` | Loose stones in gravel |

**Lesson:** Never infer terrain from `items.xml` names when reading `.sec` files. Type **1099** is
the canonical trap: srv = disguised impassable mountain (renders as cliff), xml = “sandstone wall”.

---

## Reading `.sec` sector files

### World ↔ sector

```
sector_x = x / 32,  sector_y = y / 32,  sector_z = z
local_x  = x % 32,   local_y  = y % 32
file     = {sector_x:04d}-{sector_y:04d}-{sector_z:02d}.sec
```

Example: `(32451, 32065, 7)` → `1014-1002-07.sec`, tile `03-01`.

C++ path: `reference/cipsoft-772/runtime/map/` (`MAPPATH`, cwd `runtime/`).  
Rust OTBM: `data/world/forgotten.otbm` (converted from same sectors).

### `Content={…}` stack order

C++ `map.cc` serializes the tile object linked list; `GetFirstObject` returns the **first** TypeID
in `Content={}` (bottom / first in chain). `SaveObjects` walks `getNextObject()` for subsequent ids.

```
03-01: Content={4562, 4594, 602}
              ^^^^  ^^^^  ^^^
              ground overlays (sand clip, ramp clip)
```

**FillMap** (`cract.cc` `TShortway::FillMap`) checks **only the first object**:

- Walkable iff `Bank` && !`Unpass` && `Waypoints > 0`
- `1099` → blocked (`Unpass`, `Waypoints=0`)

**MovePossible** walks the **full** stack (`Unpass` / `Avoid` on any item).

**Visual map ASCII** (matching client screenshot): classify by **first `Bank` in stack** (or first id
if no Bank), then note tree/water overlays:

| Char | Meaning |
|------|---------|
| `g` | gravel / earth (`Bank`, walkable) |
| `G` | grass |
| `s` | sand bank |
| `~` | shallow water (`4599`, etc.) |
| `T` | grass + tree overlay (`3682`…) |
| `M` | impassable mountain bank (`1099` disguised cliff) |
| `#` | impassable `Bottom` without bank (`4458` wall piece) |

### `Content` with instance attrs

Some tiles embed counts:

```
06-14: Content={4555, 1781 Amount=1, 1781 Amount=1, …}
```

For walkability/visual ground, only bare numeric prefixes matter; ground = **4555**.

---

## Cyclops mountain field layout (reference)

Area around `(32451, 32065, 7)` — sectors `1013–1015` × `1001–1003`, z=7.

```
    [shallow water ~]     [rock M]
[grass / trees G,T]  [==== gravel bowl + cyclops ====]  [rock M]
    [grass / trees]              [rock M]
```

- **Center:** walkable gravel (`4555`–`4562`) — cyclops spawn `(32456, 32071)` on same floor.
- **N/E:** `1099` disguised mountain — impassable cliff ring (matches in-game rock walls).
- **SW:** `4515` grass + `3682` fir trees.
- **NW:** `4599` shallow water pool.

Walkable corridor on target row `y=32065`: about **x=32448–32457** (10 tiles) before rock east.

---

## Workflow cheat sheet

| Task | Use |
|------|-----|
| Parse `.sec` tile at `(x,y,z)` | `objects.srv` flags + names |
| Rust walkability / FillMap | OTB `block_solid`, `is_ground_tile`, patched `speed` |
| Display label in tools / docs | `objects.srv` `Name` |
| RME / items.xml label | Informational only for 772 — verify against srv |
| Resolve TypeID → Rust `ItemType` | `resolve_server_id` (direct server_id first) |
| Flag / Waypoints audit | `772_OTB_OBJECTS_SRV_FLAG_MAPPING.md` + content crate tests |

### Quick srv lookup (shell)

```bash
rtk rg -n "^TypeID\s*=\s*1099\b" reference/cipsoft-772/runtime/dat/objects.srv -A 4
```

### Quick xml lookup (informational)

```bash
rtk rg 'id="1099"' data/items/items.xml
```

---

## Related

- [`772_OTB_OBJECTS_SRV_FLAG_MAPPING.md`](772_OTB_OBJECTS_SRV_FLAG_MAPPING.md) — full flag matrix, audits
- [`TFS-RUST_772_RealMap_Scenario_Proposal.md`](TFS-RUST_772_RealMap_Scenario_Proposal.md) — real-map battery
- [`crates/tfs-rust-content/src/objects_srv.rs`](../crates/tfs-rust-content/src/objects_srv.rs) — parser + resolver
- C++: `map.cc` `LoadSector` / `GetFirstObject`; `cract.cc` `TShortway::FillMap`
