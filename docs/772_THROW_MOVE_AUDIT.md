# 772 Throw / Move Item — Rust vs Decompile Parity Audit

**Audited:** 2026-07-11
**Reference (772 mechanics):** `reference/cipsoft-772/tibia-game-master/src/` — `cract.cc`,
`operate.cc`, `info.cc`, `receiving.cc`, `map.cc`.
**Rust files audited:**

| Rust file | C++ counterpart |
|---|---|
| `game_world_player_throw.rs` | `cract.cc:475` `TCreature::Move`, `receiving.cc:233` `CMoveObject`, `operate.cc:1275` `Move` |
| `game_world_item_move.rs` | `operate.cc:1275` `Move`, `operate.cc:1449` `Merge` |
| `game_world_item_cylinder.rs` | `info.cc` `GetObject`, `ObjectInRange`, `ObjectAccessible` |
| `map/los.rs` | `info.cc:1154` `ThrowPossible` |
| `creature_todo.rs` | `cract.cc:1123` `ToDoMove` |
| `idle_stimulus.rs` (Move arm) | `cract.cc:823` `Execute` `TDMove` |
| `container_ops.rs` | `operate.cc:606` `CheckContainerDestination`, `operate.cc:646` `CheckDepotSpace` |
| `player/inventory/query_add.rs` | `operate.cc:675` `CheckInventoryDestination` |
| `cylinder.rs` | `cylinder.h` (structural — enum vs virtual) |

**Prior audit:** `tasks/f8-decompile-parity-audit.md` covers the ToDo builder layer
(`enqueue_player_move` / `execute_player_move`). This audit focuses on the **item relocation
semantics** — the `Move` / `Merge` / `CheckMapDestination` / `ThrowPossible` chain that actually
moves items between cylinders.

**Data-vs-mechanics split (architectural decision):**
- **Data layer = OTB** (TFS 1.4.2 item types, flags, `SlotPosition` bitmask, depot/inbox/store-item
  concepts). The Rust server reads OTB, not `objects.srv`. Item type flags (`pickupable`,
  `stackable`, `moveable`, `block_projectile`, `always_on_top`, slot positions) come from OTB.
- **Mechanics layer = 772 decompile** (`cract.cc`, `operate.cc`, `info.cc`, `receiving.cc`).
  Move/throw/merge outcomes, LOS algorithm, priority walk, cylinder validation flow, event firing
  must match decompile behavior.
- `MechanicsProfile` gates **mechanics** (combat formulas, walk speed, condition ticks), not
  **data layer** concepts. Depot/inbox/store-item checks are OTB-era and stay regardless of
  `clientVersion`. Slot validation uses OTB `SlotPosition`, not 772 `BODYPOSITION`.

**772 flag → OTB flag mapping** (used throughout this audit):

| 772 flag (`enums.hh`) | OTB flag (`otb.rs`) | Used in mechanics |
|---|---|---|
| `TAKE` (40) | `pickupable()` | `CheckMoveObject(Take=true)`, `CheckMapDestination` range gate |
| `UNMOVE` (33) | `moveable()` | `CheckMoveObject`, `CheckTopMoveObject` priority walk |
| `CUMULATIVE` (6) | `stackable()` | `Move`/`Merge` count logic, `SplitObject` |
| `HANG` (41) | `is_hangable()` | `IsMapBlocked` hook check, `ObjectAccessible` HANG range |
| `UNTHROW` (34) | `block_projectile()` | `ThrowPossible` LOS, `is_tile_clear_for_throw` |
| `UNPASS` (32) | `block_solid()` | `IsMapBlocked`, `query_add_item_to_tile` |
| `UNLAY` (35) | (no direct OTB equiv — tile flag) | `IsMapBlocked` BANK check |
| `BANK` (0) | (tile ground flag) | `IsMapBlocked`, `GetTopObject` priority walk |
| `CLIP` (1) | (no direct OTB equiv — tile flag) | `GetTopObject` priority walk |
| `BOTTOM` (2) | `always_on_top()` + `always_on_top_order` | `GetTopObject` priority walk |
| `TOP` (3) | `always_on_top()` + `always_on_top_order` | `GetTopObject` priority walk |
| `CONTAINER` (4) | (OTB item group, not flag) | `CheckContainerDestination`, `Move` container dispatch |
| `CHEST` (5) | (OTB `is_container()` + depot chest type) | `CheckContainerDestination` capacity skip |
| `CLOTHES` | `slot_position` bitmask (OTB) | `CheckInventoryDestination` slot validation |
| `BODYPOSITION` | `slot_position` bitmask (OTB) | `CheckInventoryDestination` slot match |

**Rule:** When a finding says "772 checks flag X", the fix is "check OTB `Y()` at the same
mechanics point as 772 does" — not "add 772 flag X to the data layer".

---

## 0. Verdict summary

| # | Finding | Severity | Outcome differs? | Status |
|---|---------|----------|------------------|--------|
| T1 | **Wrong LOS algorithm for throw** — Rust uses Bresenham + `is_tile_clear_for_throw`; 772 uses `ThrowPossible` (major-axis interpolation) | **High** | Yes — different tiles block on diagonals; cross-floor throws entirely broken | **Fixed** |
| T2 | **`can_throw_object_to` rejects all cross-floor throws** (`from.z != to.z → false`); 772 `ThrowPossible` supports multi-floor via `MinZ` stepping | **High** | Yes — z-different throws fail in Rust, succeed in 772 | **Fixed** |
| T3 | **`can_throw_to_tile` checks `BLOCK_PROJECTILE | BLOCK_SOLID`**; 772 destination check is `IsMapBlocked` (BANK/UNPASS/UNLAY/HANG hooks) — completely different predicate | **High** | Yes — items rejected/accepted on wrong tiles | **Fixed** |
| T4 | **Missing `CheckTopMoveObject`** — 772 requires the moved item to be the top moveable object on the source tile; Rust `internal_get_thing_move` returns *any* top moveable down item without verifying it's the `Best` candidate per the priority walk | **Medium** | Yes — can move items buried under other moveables | Open |
| T5 | **Missing `CheckMoveObject`** — no `ObjectAccessible` (HANG/hook range + owner check), no OTB `moveable()` check at the `Move` executor level (Rust checks `moveable()` in `player_move_item` but not in `internal_move_item`), no OTB `pickupable()` check at `Take=true` point | **Medium** | Partially — `moveable` checked early; HANG hooks + `ObjectAccessible` + `pickupable` at `Take=true` missing | Open |
| T6 | **Missing `CheckMapDestination` creature-container branch** — z-delta > 1 reject, height-24 jump check, `MovePossible` gate, protection-zone gate | Low | Creature push not ported (D9) | Open (blocked) |
| T7 | **Missing `INVENTORY_ANY` auto-slot scan** — 772 `Move` scans slots 1–10 then containers when `DestY == INVENTORY_ANY`; Rust has no equivalent | **Medium** | Yes — "drop to inventory" auto-placement fails | Open |
| T8 | **Missing exchange-on-move-failure** — 772 `Move` catches `NOROOM`/`HANDSNOTFREE`/`HANDBLOCKED`/`ONEWEAPONONLY` for inventory dest and swaps `DestObj` out; Rust has `NeedExchange` in `query_add` but not the `Move`-level catch-and-swap | **Medium** | Yes — inventory swap fails differently | Open |
| T9 | **Missing `CheckWeight` / `CheckDepotSpace` in `internal_move_item`** — 772 `Move` checks weight + depot capacity before relocating; Rust has capacity in `player_query_add` and depot limit in `container_query_add` but no `CheckWeight` equivalent for container moves | Low | Partially covered | Open |
| T10 | **`Merge` 100-cap uses `TOOMANYPARTS`**; Rust uses `stack_merge_room` returning `ContainerNotEnoughRoom` — wrong error code | Low | Yes — wrong client message | Open |
| T11 | **Missing `SeparationEvent` / `MovementEvent` / `CollisionEvent` / `NotifyAllCreatures`** — 772 fires Lua move events + collision events on every `Move`/`Merge`; Rust has `MoveEventsRegistry` but doesn't call it from `internal_move_item` | **Medium** | Yes — Lua `onMoveItem`/`onStepIn` fields never fire | Open |
| T12 | **`CMoveObject` handler pre-validation absent** — `isMapContainer` reject, `stackable() && Count==0` reject, `CheckSpecialCoordinates`, `CheckVisibility` | Low | Edge/robustness (D8) | Open |
| T13 | **`RNum` forced to 1 for map tiles** — 772 `CMoveObject` sets `RNum = 1` when `OrigX != 0xFFFF`; Rust `internal_get_thing_move` ignores `stack_pos` for map tiles | Low | Same outcome (stack_pos unused for map) | OK |
| T14 | **HANG hook destination walk-to-reach** — 772 `Move` has a special HANG+hook branch that picks up the item, walks to dest, then re-moves; Rust has no HANG-specific path in `Move` executor | Low | HANG items on hooks | Open |

### Second pass (deeper analysis)

| # | Finding | Severity | Outcome differs? | Status |
|---|---------|----------|------------------|--------|
| T15 | **Throw range limit (15) doesn't exist in 772** — Rust applies `item_throw_range = 15` for OTB `pickupable()` items; 772 `CheckMapDestination` has NO range limit for pickupable items (only `ThrowPossible` LOS). Non-pickupable uses `ObjectInRange(2)` — Rust matches this but also wrongly limits pickupable | **High** | Yes — pickupable items can't be thrown as far in Rust | **Fixed** |
| T16 | **`SplitObject` drops all item attributes** — Rust `Item::new(item_type, m)` creates item with `attributes: None`; 772 `SplitObject` → `CopyObject` copies ALL attributes (action_id, unique_id, text, decay, etc.) | **Medium** | Yes — split items lose action IDs, text, decay timers | Open |
| T17 | **Tile auto-merge: partial vs full** — Rust `internal_add_item_to_tile` partial-merges (100+5 from 95+10); 772 `Move` → `Merge` rejects >100 → keeps separate stacks (95+10). 772 only partial-merges for inventory dest, not tile dest | **Medium** | Yes — different stack counts on tiles | Open |
| T18 | **Self-move (creature moving itself) not wired** — 772 `TCreature::Move` checks `Obj == this->CrObject` → `this->Go(DestX, DestY, DestZ)` (walk). Rust `Thing::Creature` branch is a `debug!` stub | Low | Yes — creature self-move is a no-op | Open |
| T19 | **`Count == -1` sentinel missing** — 772 internal `Move` calls use `Count = -1` for "move all" (e.g. catch-and-swap). Rust `count` is `u16` — no "move all" sentinel | Low | Affects swap path when ported (T8) | Open |
| T21 | **`internal_get_thing_move` doesn't match by sprite** — 772 `GetObject` walks the tile's object chain matching by `getDisguise() == Type` (sprite ID); Rust returns the top moveable down item, then validates sprite separately. Can't move items buried under other items | **High** | Yes — non-top items can't be moved | **Fixed** |
| T22 | **Merge target selection differs** — 772 `Move()` auto-merge tries only `GetTopObject(true)` (top non-BANK/CLIP/BOTTOM/TOP item); Rust `internal_add_item_to_tile` scans all `down_items` for same-type match. Rust merges when 772 wouldn't | **Medium** | Yes — different merge target | Open |
| T23 | **772 `Move()` auto-merge only for map dest + different source** — 772 `Move()` only auto-merges when `ConType.isMapContainer() && ObjType.getFlag(CUMULATIVE) && OldCon != Con`. OTB `stackable()` = 772 `CUMULATIVE`. Rust `internal_add_item_to_tile` always tries merge regardless of source | Low | Same-floor same-tile move merges in Rust, not in 772 | Open |

---

## 1. T1 — Wrong LOS algorithm (High)

### C++ reference (`info.cc:1154-1216` `ThrowPossible`)

`CheckMapDestination` (`operate.cc:576`) calls:
```cpp
if(!ThrowPossible(OrigX, OrigY, OrigZ, DestX, DestY, DestZ, 1)){
    throw CANNOTTHROW;
}
```

`ThrowPossible` uses **major-axis linear interpolation**:
```cpp
int MaxT = std::max(std::abs(DestX - OrigX), std::abs(DestY - OrigY));
for(int T = StartT; T <= MaxT; T += 1){
    int CurX = (OrigX * (MaxT - T) + DestX * T) / MaxT;
    int CurY = (OrigY * (MaxT - T) + DestY * T) / MaxT;
    if(CoordinateFlag(CurX, CurY, CurZ, UNTHROW)) break;
}
```

This produces a **different tile set** than Bresenham on diagonals. For a (0,0)→(3,3) throw:
- **ThrowPossible** (MaxT=3): checks (1,1), (2,2), (3,3) — the diagonal cells only.
- **Bresenham**: checks (1,1), (2,2), (3,3) — same on perfect diagonals, but diverges on
  non-45° lines like (0,0)→(5,2): ThrowPossible checks (1,0),(2,1),(3,1),(4,2),(5,2) while
  Bresenham checks (1,0),(2,1),(3,1),(4,2),(5,2) — actually close but the interpolation
  rounding differs at half-steps.

The critical difference: **`ThrowPossible` only checks the tile `UNTHROW` coordinate flag**
(set from OTB `block_projectile()` on items during tile flag aggregation), while Rust
`is_tile_clear_for_throw` checks the tile `UNTHROW` flag **plus** re-checks `block_projectile()`
on every individual item on the tile. The 772 mechanic relies on the pre-aggregated tile flag;
Rust re-checks item flags, which is a TFS 1.4.2 mechanic. The OTB `block_projectile()` flag is
correct data — the **mechanic** that differs is whether to check the tile-level flag (772) or
re-scan all items (TFS 1.4.2).

### Rust (`game_world_player_throw.rs:200` `can_throw_object_to`)

```rust
fn can_throw_object_to(&self, from: Position, to: Position, throw_range: u32) -> bool {
    if from.z != to.z { return false; }          // ← T2: rejects all cross-floor
    let dx = ...; let dy = ...;
    if dx > throw_range || dy > throw_range { return false; }
    if dx < 2 && dy < 2 { return true; }          // adjacent skip
    for p in crate::map::walk_grid_line(from, to) {  // ← Bresenham, not ThrowPossible
        if p == from || p == to { continue; }
        if !self.is_tile_clear_for_throw(p, false) { return false; }
    }
    true
}
```

`walk_grid_line` is integer Bresenham (`map/los.rs:10`). `is_tile_clear_for_throw` checks
`UNTHROW` flag + `block_projectile()` on ground + all top/down items.

### Impact

- **Diagonal throws** may pass/fail differently than 772 — a wall corner that 772's
  interpolation skips may block in Bresenham, or vice versa.
- **`block_projectile()` item check** is not in 772's `ThrowPossible` — only `UNTHROW` flag.
  Rust over-rejects throws over tiles that have projectile-blocking items but no `UNTHROW` flag.

### Fix

Replace `can_throw_object_to`'s line check with `Map::throw_possible(from, to, 1)` (already
implemented in `map/los.rs:87` and verified against `info.cc:1154`). The `throw_possible`
function is currently only used by combat/monster callers, not the item throw path.

---

## 2. T2 — Cross-floor throws rejected (High)

### C++ reference

`ThrowPossible` (`info.cc:1154`) explicitly supports multi-floor throws via `MinZ` stepping:
```cpp
int MinZ = std::max<int>(OrigZ - Power, 0);
// walk up looking for bank ceiling
while(MinZ <= DestZ){
    // interpolate line at MinZ, check if dest column is open down to DestZ
    MinZ += 1;
}
```

With `Power = 1` (the value `CheckMapDestination` passes), `MinZ = OrigZ - 1`, allowing throws
to tiles one floor up. The destination column must be open from `MinZ` down to `DestZ`.

### Rust

```rust
fn can_throw_object_to(&self, from: Position, to: Position, throw_range: u32) -> bool {
    if from.z != to.z { return false; }  // ← hard reject
    ...
}
```

### Impact

All cross-floor item throws fail with `CannotThrow` in Rust. In 772, throwing an item up/down
one floor (with `Power=1`) is valid if the path is clear. This is a significant behavioral
regression for multi-floor gameplay.

### Fix

Use `throw_possible(from, to, 1)` which already implements the `MinZ` stepping. Remove the
`from.z != to.z` early return.

---

## 3. T3 — Wrong destination tile check (High)

### C++ reference (`operate.cc:474-578` `CheckMapDestination`)

For non-creature items, the destination tile check is `IsMapBlocked` (`operate.cc:451`):
```cpp
static bool IsMapBlocked(int DestX, int DestY, int DestZ, ObjectType Type){
    bool HasBank = CoordinateFlag(DestX, DestY, DestZ, BANK);
    if(HasBank && !CoordinateFlag(DestX, DestY, DestZ, UNPASS)) return false;
    if(!Type.getFlag(UNPASS)){
        if(HasBank && !CoordinateFlag(DestX, DestY, DestZ, UNLAY)) return false;
        if(Type.getFlag(HANG)){
            bool HasHook = CoordinateFlag(HOOKSOUTH) || CoordinateFlag(HOOKEAST);
            if(HasHook && !CoordinateFlag(HANG)) return false;
        }
    }
    return true;  // blocked
}
```

This checks **map coordinate flags** (BANK, UNPASS, UNLAY, HANG, HOOKSOUTH, HOOKEAST) — not
item properties. Then for non-takeable items, `ObjectInRange(CreatureID, MapCon, 2)` (range 2,
not throw range). Then `ThrowPossible`.

### Rust (`game_world_player_throw.rs:278` `can_throw_to_tile`)

```rust
fn can_throw_to_tile(&self, pos: Position, _item_id: ItemId) -> bool {
    let body = tile.body();
    if body.flags & (BLOCK_PROJECTILE | BLOCK_SOLID) != 0 { return false; }
    // check ground item block_projectile / block_solid
    // check all top/down items block_projectile / block_solid
    true
}
```

### Impact

- Rust checks **item flags** (`block_projectile`, `block_solid`); 772 checks **tile coordinate
  flags** (BANK, UNPASS, UNLAY, HANG, hooks).
- A tile with `BANK` flag and no `UNPASS` is always valid in 772 (ground exists). Rust may
  reject it if items on it have `block_solid`.
- A HANG item thrown to a hook tile is valid in 772 (`HasHook && !HANG flag → false = not
  blocked`). Rust has no hook logic in `can_throw_to_tile`.
- The `query_add_item_to_tile` function (`game_world_item_cylinder.rs:147`) does check ground
  existence + blocking + creatures, which is closer to `IsMapBlocked`, but `can_throw_to_tile`
  is called **before** `internal_move_item` and uses a different predicate.

### Fix

Replace `can_throw_to_tile` with an `is_map_blocked(pos, item_type)` check mirroring
`IsMapBlocked` (`operate.cc:451`), or remove `can_throw_to_tile` entirely and rely on
`query_add_item_to_tile` (which is called inside `internal_move_item` anyway). The 772 flow
is: `CheckMapDestination` (IsMapBlocked + range + ThrowPossible) → then `Move` →
`MoveObject` (which does the actual placement). Rust duplicates the destination check in
`can_throw_to_tile` with the wrong predicate.

---

## 4. T4 — Missing `CheckTopMoveObject` (Medium)

### C++ reference (`operate.cc:296-342`)

```cpp
void CheckTopMoveObject(uint32 CreatureID, Object Obj, Object Ignore){
    // For map-tile sources, verify Obj is the top moveable object
    // Walk the tile's item list, find Best (first non-UNMOVE (= OTB !moveable()), or first creature)
    // If Obj != Best → throw NOTACCESSIBLE
}
```

Called at `operate.cc:1349` in `Move()`. This prevents moving items that are buried under
other moveable items on the same tile.

### Rust

`internal_get_thing_move` (`game_world_item_cylinder.rs:50`) returns `get_top_down_item()`
— the top down item if it's moveable (OTB `moveable()`). But it does **not** walk the full item
stack to find the "best" moveable candidate per the 772 priority rules (first non-`UNMOVE`
= OTB `!moveable()`, skipping creatures, `PRIORITY_LOW` break). If the top down item is
moveable, it's returned regardless of whether 772 would consider it the `Best` candidate.

### Impact

Items under other items on a tile can be moved when 772 would reject with `NOTACCESSIBLE`.
The `get_top_down_item` returns the top of the down-items vector, which may not match the 772
priority walk (which iterates `GetFirstContainerObject` → `getNextObject` with priority breaks).

### Fix

Port `CheckTopMoveObject` as a validation step in `internal_move_item` for tile sources, or
fix `internal_get_thing_move` to match the 772 priority walk exactly.

---

## 5. T5 — Missing `CheckMoveObject` / `ObjectAccessible` (Medium)

### C++ reference (`operate.cc:418-447`)

```cpp
void CheckMoveObject(uint32 CreatureID, Object Obj, bool Take){
    if(!ObjectAccessible(CreatureID, Obj, 1)) throw NOTACCESSIBLE;
    if(ObjType.getFlag(UNMOVE)) throw NOTMOVABLE;
    // creature pushable check
    if(Take && !ObjType.getFlag(TAKE)) throw NOTTAKABLE;
}
```

`ObjectAccessible` (`info.cc:252-300`) includes the **HANG hook range** check — for hangable
items on hook tiles, the player must be within range 1 of the hook (with the asymmetric
HOOKEAST/HOOKSOUTH bounds). This is separate from `ObjectInRange`.

### Rust

- `moveable()` (OTB equivalent of 772 `UNMOVE`) is checked in `player_move_item`
  (`game_world_player_throw.rs:95`) — correct for the client throw path, but **not** in
  `internal_move_item` (which is also called by Lua/monster paths).
- `ObjectAccessible` (HANG hook range) is not checked anywhere in the move path.
  `player_rotate_item` has an `ObjectAccessible` reference but only for rotate, not move.
- `pickupable()` (OTB equivalent of 772 `TAKE`) is not checked at the `Take=true` mechanics
  point — 772 rejects non-takable items when `Take=true` (container/inventory destinations).
  Rust checks `pickupable()` in `container_query_add` (T40) but not at the 772 mechanics point.

### Fix

Add a `check_move_object` function mirroring `operate.cc:418` and call it in
`internal_move_item` for the appropriate `Take` value (false for map dest, true for
container/inventory dest). Use OTB `moveable()` for `UNMOVE`, OTB `pickupable()` for `TAKE`.
Include the `ObjectAccessible` HANG hook range check (using OTB `is_hangable()` + tile
`HOOKSOUTH`/`HOOKEAST` flags).

---

## 6. T7 — Missing `INVENTORY_ANY` auto-slot scan (Medium)

### C++ reference (`cract.cc:500-547`)

When `DestX == 0xFFFF && DestY == INVENTORY_ANY`, 772 scans all inventory slots 1–10 trying
`CheckInventoryDestination`, prioritizing non-hand/ammo slots, then falls back to scanning
containers in inventory slots. If nothing fits → `NOROOM`.

### Rust

`player_move_item` resolves `to_cylinder` via `internal_get_cylinder` which for `pos.y` as
slot returns `Cylinder::Inventory { slot: pos.y }`. There is no `INVENTORY_ANY` handling —
the client must send a specific slot. If the client sends `INVENTORY_ANY` (value not found),
it would be treated as a raw slot index.

### Impact

"Drop item to inventory" (auto-place) doesn't work — the client's auto-place request is
treated as a specific slot request. 772 clients use this when the player double-clicks an
item to pick it up.

### Fix

Port the `INVENTORY_ANY` scan loop in `player_move_item` / `execute_player_move` when
`to_pos.x == 0xFFFF && to_pos.y == INVENTORY_ANY`.

---

## 7. T8 — Missing exchange-on-move-failure (Medium)

### C++ reference (`cract.cc:607-623`)

```cpp
try{
    ::Move(this->ID, Obj, DestCon, MoveCount, false, DestObj);
}catch(RESULT r){
    if(DestY >= INVENTORY_FIRST && DestY <= INVENTORY_LAST
            && DestObj != NONE
            && (r == NOROOM || r == HANDSNOTFREE || r == HANDBLOCKED || r == ONEWEAPONONLY)){
        Object ObjCon = Obj.getContainer();
        ::Move(this->ID, DestObj, ObjCon, -1, false, NONE);  // swap out dest item
        ::Move(this->ID, Obj, DestCon, MoveCount, false, DestObj);  // retry
    }else{
        throw;
    }
}
```

### Rust

`internal_move_item` has `NeedExchange` handling in the `player_query_add` path
(`game_world_item_move.rs:52`) via `try_resolve_inventory_need_exchange`. This covers the
`queryAdd` `NEEDEXCHANGE` return. But the 772 `Move`-level catch-and-swap for `NOROOM` /
`HANDSNOTFREE` / `HANDBLOCKED` / `ONEWEAPONONLY` is a **different** mechanism — it catches
errors from the full `Move` call (including `CheckInventoryDestination` throws) and swaps the
dest item to the source container before retrying.

### Impact

Equipping an item into an occupied slot may fail in Rust where 772 would auto-swap the
existing item to the source container. The `NeedExchange` path handles some cases but not
the `HANDSNOTFREE` / `HANDBLOCKED` / `ONEWEAPONONLY` weapon-specific cases.

### Fix

Add a catch-and-swap wrapper around the inventory-destination `Move` call, mirroring
`cract.cc:610-622`. This is in the `TCreature::Move` executor, not `internal_move_item` —
it's a player-action-level retry.

---

## 8. T11 — Missing move events (Medium)

### C++ reference (`operate.cc:1444-1446`)

```cpp
MovementEvent(Obj, OldCon, Con);
CollisionEvent(Obj, Con);
NotifyAllCreatures(Obj, OBJECT_MOVED, OldCon);
```

`Move` fires `MovementEvent` (Lua `onMoveItem` / `onStepOut` / `onStepIn`), `CollisionEvent`
(items with collision like fire fields), and `NotifyAllCreatures` (broadcast). `Merge` fires
`CollisionEvent` + `NotifyAllCreatures` (`operate.cc:1530-1531`).

### Rust

`internal_move_item` does not call any Lua move events. `MoveEventsRegistry` exists
(`lua_event_dispatcher.rs:23`) and is loaded at startup (`run_server.rs:199`), but it's only
wired for equip/dequip, not for tile-to-tile move events.

### Impact

Lua `onMoveItem`, `onStepIn`, `onStepOut` move events never fire. Items with collision effects
(fire fields, magic walls) don't trigger on move. This affects any Lua script relying on move
events (e.g. quest items that can't leave a tile, traps triggered by moving items).

### Fix

Wire `MoveEventsRegistry` into `internal_move_item` — fire `onMoveItem` before the move
(can cancel), `onStepOut` from old tile, `onStepIn` to new tile after the move. This is a
significant feature gap, not a quick fix.

---

## 11. T15 — Throw range limit doesn't exist in 772 for pickupable items (High)

### C++ reference (`operate.cc:474-578` `CheckMapDestination`)

```cpp
void CheckMapDestination(uint32 CreatureID, Object Obj, Object MapCon){
    ...
    if(!ObjType.isCreatureContainer()){
        if(IsMapBlocked(DestX, DestY, DestZ, ObjType)) throw NOROOM;
        // Range check ONLY for non-takeable items:
        if(!ObjType.getFlag(TAKE) && !ObjectInRange(CreatureID, MapCon, 2))
            throw OUTOFRANGE;
    }
    // HANG hook check...
    if(!ThrowPossible(OrigX, OrigY, OrigZ, DestX, DestY, DestZ, 1))
        throw CANNOTTHROW;
}
```

772 `TAKE` flag = OTB `pickupable()`. For **pickupable** items: NO range check at all — only
`ThrowPossible` LOS (which has no horizontal distance limit, only `MinZ` floor stepping). For
**non-pickupable** items: `ObjectInRange(2)` from **player** to **dest** (range 2).

### Rust (`game_world_player_throw.rs:166-180`)

```rust
// C++ ref: src/game.cpp:1046-1060 Game::playerMoveItem
if !item_is_pickupable && player_pos.z != map_to_pos.z {
    return Err(ReturnValue::DestinationOutOfReach);
}
let to_dx = (player_pos.x as i32 - map_to_pos.x as i32).unsigned_abs();
let to_dy = (player_pos.y as i32 - map_to_pos.y as i32).unsigned_abs();
if to_dx > item_throw_range || to_dy > item_throw_range {
    return Err(ReturnValue::DestinationOutOfReach);
}
if !self.can_throw_object_to(map_from_pos, map_to_pos, item_throw_range) {
    return Err(ReturnValue::CannotThrow);
}
```

`item_throw_range` = 15 for pickupable, 2 for non-pickupable. **Two range checks** that don't
exist in 772 for pickupable items:
1. `player → dest` range against `item_throw_range` (15 for pickupable)
2. `can_throw_object_to`: `source → dest` range against `item_throw_range`

### Impact

Pickupable items (gold, runes, food, etc.) can only be thrown 15 tiles in Rust. In 772, they
can be thrown any distance as long as the `ThrowPossible` line is clear. The `item_throw_range = 15`
is a TFS 1.4.2 mechanic (`Item::getThrowRange() = 15` for pickupable, `item.h:828`) that doesn't
exist in the 772 decompile. The OTB flag `pickupable()` is correct for determining takeable vs
non-takeable — the **mechanic** that differs is the range limit.

### Fix

Remove the `item_throw_range` limit for pickupable items (OTB `pickupable() == true`). Keep
range 2 for non-pickupable (matches 772 `ObjectInRange(2)` gated on `!TAKE` = `!pickupable()`).
Replace `can_throw_object_to` with `throw_possible(from, to, 1)` which has no horizontal range
limit (T1/T2 fix).

---

## 12. T16 — `SplitObject` drops all item attributes (Medium)

### C++ reference (`map.cc:2210-2235` `SplitObject` → `map.cc:2164-2208` `CopyObject`)

```cpp
Object SplitObject(Object Obj, int Count){
    ...
    Res = CopyObject(Obj.getContainer(), Obj);
    Res.setAttribute(AMOUNT, (uint32)Count);
    Obj.setAttribute(AMOUNT, Amount - (uint32)Count);
    return Res;
}

Object CopyObject(Object Con, Object Source){
    Object NewObj = SetObject(Con, SourceType, 0);
    for(int i = 0; i < NARRAY(TObject::Attributes); i += 1)
        AccessObject(NewObj)->Attributes[i] = AccessObject(Source)->Attributes[i];
    // duplicate text strings, clear container content...
    return NewObj;
}
```

`CopyObject` copies **ALL** attributes (action_id, unique_id, text, decay timer, charges,
etc.) before setting the new amount.

### Rust (`game_world_item_move.rs:157-158`, `game_world_item_move.rs:265-266`, etc.)

```rust
let new_item = Item::new(item_type, m);  // attributes: None
let new_id = self.items.insert(new_item);
```

`Item::new` creates an item with `attributes: None` — no action_id, unique_id, text, decay.

### Impact

Splitting a stack of items with attributes (e.g. a signed book, a quest key with action_id, a
rune with decay timer) loses all attributes on the split portion. The 772 `CopyObject` preserves
them. This affects:
- Stackable quest items with action_id/unique_id
- Written runes (text attribute)
- Decay timers on split items

### Fix

Replace `Item::new(item_type, m)` in split paths with a `clone_with_count` method that deep-copies
`attributes` (minus container content). See `Item::clone_for_split` pattern.

---

## 13. T17 — Tile auto-merge: partial vs full (Medium)

### C++ reference

772 has **two different merge behaviors** depending on destination type:

**Map tile dest** (`operate.cc:1303-1320` `Move()` → `Merge()`):
```cpp
// Move() tries Merge with GetTopObject:
Object Top = GetTopObject(ConX, ConY, ConZ, true);
if(Top != NONE){
    try{ Merge(CreatureID, Obj, Top, Count, Ignore); return; }
    catch(RESULT r){ if(r == DESTROYED) throw; }
    // non-DESTROYED → fall through to full Move
}
```

`Merge()` (`operate.cc:1484`): `if((Count + DestCount) > 100) throw TOOMANYPARTS;` — **no
partial merge**. If the total exceeds 100, the merge fails and the item is placed as a
separate stack via `MoveObject`.

**Inventory/container dest** (`cract.cc:579-597` `TCreature::Move` pre-merge):
```cpp
int MergeCount = MoveCount;
if((DestAmount + MergeCount) > 100) MergeCount = 100 - DestAmount;  // ← partial merge
if(MergeCount > 0){
    ::Merge(this->ID, Obj, DestObj, MergeCount, NONE);
    MoveCount -= MergeCount;
    if(MoveCount <= 0) return;
}
```

Inventory dest **does** partial-merge: merge as many as fit, move the remainder separately.

### Rust (`game_world_item_cylinder.rs:241-279` `internal_add_item_to_tile`)

```rust
let can_add = (100u16).saturating_sub(target_count).min(item_count);
if can_add > 0 {
    // merge can_add, keep remainder as separate item
}
```

Rust **always partial-merges** for tile destinations. 772 only partial-merges for
inventory/container dest; tile dest uses full-merge-only (reject >100, keep separate stacks).

### Impact

Throwing 50 gold to a tile with 80 gold:
- **772**: `Merge(80+50=130 > 100)` → `TOOMANYPARTS` → fall through → `MoveObject` → two stacks: [80, 50]
- **Rust**: `can_add = 20` → merge 20 → [100, 30] — different stack counts

### Fix

For tile destinations, only merge if the full count fits (`count + dest_count <= 100`).
If it doesn't fit, place as a separate stack. Keep partial-merge for container/inventory dest.

---

## 14. T21 — `internal_get_thing_move` doesn't match by sprite (High)

### C++ reference (`info.cc:398-432` `GetObject`)

```cpp
Object GetObject(uint32 CreatureID, int x, int y, int z, int RNum, ObjectType Type){
    if(x != 0xFFFF){  // map tile
        Obj = GetFirstObject(x, y, z);
        while(Obj != NONE){
            if(Obj.getObjectType().getDisguise() == Type) break;  // ← match by sprite
            Obj = Obj.getNextObject();
        }
    }
    // verify type matches (unless wildcard)
    if(Obj != NONE && !Type.isMapContainer()
            && Obj.getObjectType().getDisguise() != Type) Obj = NONE;
    return Obj;
}
```

772 walks the **entire object chain** on the tile and returns the first object whose
`getDisguise()` matches the client-sent `Type` (sprite ID). This is a **type-based search**.

### Rust (`game_world_item_cylinder.rs:50-91` `internal_get_thing_move`)

```rust
pub fn internal_get_thing_move(&self, cid: CreatureId, pos: Position, _stack_pos: u8) -> Option<Thing> {
    if pos.x != 0xFFFF {
        let tile = self.map.get_tile(pos)?;
        if let Some(top_item_id) = tile.get_top_down_item() {
            if let Some(item) = self.items.get(top_item_id) {
                if it.map(|t| t.moveable()).unwrap_or(false) {
                    return Some(Thing::Item(top_item_id));  // ← top item, no sprite match
                }
            }
        }
        // Fall through to creature
        if let Some(&creature_id) = body.creatures.last() {
            return Some(Thing::Creature(creature_id));
        }
        None
    }
    ...
}
```

Rust returns the **top moveable down item** — no sprite matching. The sprite is validated
afterward in `validate_move_object_ref` / `player_move_item`, but if the top item doesn't
match the sprite, it returns `NotPossible` instead of searching further.

### Impact

If a tile has [sword, shield] (sword on top) and the client wants to move the shield:
- **772**: `GetObject` walks the chain, skips sword (type mismatch), finds shield → success
- **Rust**: `internal_get_thing_move` returns sword (top down item), sprite validation fails → `NotPossible`

**Items that aren't on top of the tile stack cannot be moved.** This is a significant
behavioral regression — players routinely move items that have other items on top of them.

Note: `validate_action_object_ref` (for Use/Turn) **does** have a `find_tile_item_by_client_sprite`
fallback (`container_ui.rs:464`) that scans `down_items` + `top_items` by sprite. But
`validate_move_object_ref` (for Move) does **not** — it only uses `internal_get_thing_move`.

### Fix

Add `find_tile_item_by_client_sprite` fallback to `validate_move_object_ref` and
`player_move_thing` (same pattern as `validate_action_object_ref`):
```rust
let item_id = if let Some(Thing::Item(id)) = self.internal_get_thing_move(cid, obj.pos, obj.stack_pos) {
    Some(id)
} else if obj.pos.x != 0xFFFF {
    self.find_tile_item_by_client_sprite(obj.pos, obj.sprite_id)
} else {
    None
};
```

Or better: make `internal_get_thing_move` match by sprite for map tiles, mirroring `GetObject`.

---

## 15. T22 — Merge target selection differs (Medium)

### C++ reference (`operate.cc:1309` `Move()` auto-merge)

```cpp
Object Top = GetTopObject(ConX, ConY, ConZ, true);
```

`GetTopObject(x, y, z, true)` (`info.cc:366-388`) walks the object chain and returns the
**top** non-BANK/CLIP/BOTTOM/TOP/creature item. `Move()` only tries to merge with this one
item. If it's a different type, `Merge` throws `NOMATCH` → fall through → separate stack.

### Rust (`game_world_item_cylinder.rs:244-252` `internal_add_item_to_tile`)

```rust
for &did in &tile.body().down_items {
    if let Some(existing) = self.items.get(did) {
        if existing.item_type == item_type && existing.count < 100 {
            merge_target = Some(did);
            break;  // ← first match in down_items
        }
    }
}
```

Rust scans **all** `down_items` for a same-type match. If the top item is a different type
but a lower item is the same type, Rust merges with the lower item while 772 wouldn't.

### Impact

Tile has [gold(50), sword] (sword on top). Player throws gold(30):
- **772**: `GetTopObject` returns sword. `Merge` fails (`NOMATCH`). Fall through → [gold(50), sword, gold(30)]
- **Rust**: scans `down_items`, finds gold(50). Merges → [gold(80), sword] — different!

### Fix

Only try to merge with the top item on the destination tile (matching `GetTopObject(true)`),
not scan all items. If the top item is a different type, place as a separate stack.

---

### Third pass (deep `Merge` / `CheckMoveObject` / `CheckInventoryDestination` / broadcast)

| # | Finding | Severity | Outcome differs? | Status |
|---|---------|----------|------------------|--------|
| T24 | **`CheckMoveObject` creature-push gate missing** — 772 rejects pushing `RaceUnpushable` creatures (unless Non-PVP + peaceful); Rust has no creature push at all (T18 stub) but the **gate** is also absent from `internal_move_item` for when creature push lands | Low | Blocked on T18 | Open |
| T25 | **`CheckInventoryDestination` `WRONGPOSITION`/`WRONGCLOTHES`/`WRONGPOSITION2` not mapped** — 772 checks `CLOTHES` flag + `BODYPOSITION` attribute for non-hand/ammo slots; Rust `evaluate_player_inventory_slot_query` does this but uses TFS 1.4.2 slot semantics, not 772 `BODYPOSITION` attribute. `HANDSNOTFREE`/`HANDBLOCKED`/`ONEWEAPONONLY` for hand slots are in `evaluate_player_inventory_slot_query` but the **`Split` parameter** (772 `CheckInventoryDestination(Obj, Con, Split)`) is not propagated — 772 uses `Split` to relax the `Other != Obj` check in the `ONEWEAPONONLY` test | **Medium** | Yes — splitting a weapon into an occupied hand slot may wrongly reject | Open |
| T26 | **`CheckDepotSpace` directionality missing** — 772 only rejects when moving **into** the depot (`!IsHeldByContainer(Source, Depot) && IsHeldByContainer(Destination, Depot)`); Rust `container_query_add` checks depot limit on every add to a depot-type container regardless of source — same outcome for cross-cylinder moves but may over-reject for moves **within** the depot | Low | Edge case — within-depot moves | Open |
| T27 | **`CheckWeight` self-ownership skip missing** — 772 `CheckWeight` returns early if `GetObjectCreatureID(Obj) == CreatureID` (moving your own item within your own cylinders doesn't check weight); Rust `player_has_capacity` has `player_carries_item(cid, item_id)` which is similar but checks if the player **currently carries** the item, not if the item's owner == actor. For items owned by the player but on the ground (dropped), Rust checks weight, 772 doesn't | Low | Edge case — dropped own items | Open |
| T28 | **`Merge` `NOMATCH`/`NOTCUMULABLE` error codes missing** — 772 `Merge` throws `NOMATCH` (different types) and `NOTCUMULABLE` (non-stackable); Rust `stack_merge_room` returns `ContainerNotEnoughRoom` for all merge failures | Low | Wrong client message | Open |
| T29 | **`Merge` `Ignore` parameter not used** — 772 `Merge` passes `Ignore` to `CheckTopMoveObject` (skips the moved item when checking if source is top); Rust has no `Ignore` parameter in `internal_move_item` | Low | Affects catch-and-swap (T8) | Open |
| T30 | **`Merge` `SeparationEvent` only fires when `ObjCon != DestCon`** — 772 gates `SeparationEvent` on `ObjCon != DestCon`; Rust has no `SeparationEvent` at all (T11) but the gate should be preserved when ported | Low | Blocked on T11 | Open |
| T31 | **`Move` `NoMerge` parameter not used** — 772 `Move(CreatureID, Obj, Con, Count, NoMerge, Ignore)` has a `NoMerge` flag that skips the auto-merge-with-top-item step; Rust `internal_move_item` has no `NoMerge` parameter — auto-merge in `internal_add_item_to_tile` always runs | Low | Internal callers can't skip merge | Open |
| T32 | **`Move` `Ignore` parameter not used** — 772 `Move` passes `Ignore` to `CheckTopMoveObject` + `Merge`; Rust has no `Ignore` parameter. Used by `TCreature::Move` catch-and-swap (T8) to skip the dest item when re-checking source top | Low | Blocked on T8 | Open |
| T33 | **`Move` `OldCon != Con` gate on `SeparationEvent` missing** — 772 only fires `SeparationEvent` when `OldCon != Con` (moving within the same container doesn't fire); Rust has no `SeparationEvent` (T11) | Low | Blocked on T11 | Open |
| T34 | **`Move` creature-container `NotifyTurn`/`NotifyGo`/`AnnounceMovingCreature` missing** — 772 fires `NotifyTurn(Con)` + `AnnounceMovingCreature` before `MoveObject`, then `NotifyGo` after; Rust `Thing::Creature` branch is a stub (T18) | Low | Blocked on T18 | Open |
| T35 | **`Move` `CloseContainer(Obj, true)` for `CreatureID == 0` + container items** — 772 closes container UIs for system-initiated moves of container items; Rust has no equivalent | Low | System moves only | Open |
| T36 | **`Move` `NotifyCreature(ObjOwnerID, ...)` + `NotifyCreature(ConOwnerID, ...)` missing** — 772 notifies both old and new owners (for container UI refresh); Rust has `notify_player_container_tree_changed` but it's only called in some cylinder arms, not all | **Medium** | Container UI may desync for cross-player moves | Open |
| T37 | **`Move` `AnnounceChangedObject(OBJECT_DELETED)` before `MoveObject` + `AnnounceChangedObject(OBJECT_CREATED)` after** — 772 broadcasts delete-then-create for the source/dest tiles; Rust does `broadcast_tile_item_remove` + `broadcast_tile_item_add` in tile arms but **not** for container/inventory arms — container-to-container moves don't broadcast tile changes | Low | Container moves don't need tile broadcast | OK |
| T38 | **`get_top_down_item` returns `down_items.first()` — doesn't match `GetTopObject(true)`** — 772 `GetTopObject(x,y,z,true)` walks the **entire object chain** (ground → top items → creatures → down items) and returns the first non-BANK/CLIP/BOTTOM/TOP/creature item; Rust `get_top_down_item` returns only `down_items.first()`, skipping top items and creatures entirely | **High** | Yes — top items (splashes, ladders) are never returned as the "top moveable" | **Fixed** |
| T39 | **`internal_get_thing_move` creature fallback uses `creatures.last()`** — 772 `GetObject` with `RNum != -1` walks by type match; `GetTopObject(true)` skips creatures when `Move=true`; Rust returns `creatures.last()` as fallback after down items, which may return a creature that 772 would not | **Medium** | Yes — creature returned when 772 returns NONE | Open |
| T40 | **`container_query_add` checks `pickupable()` at wrong mechanics point** — 772 `CheckContainerDestination` only checks `IsHeldByContainer` (cycle) + capacity; the `TAKE` (= OTB `pickupable()`) check is in `CheckMoveObject(Take=true)` (T5). Rust `container_query_add` rejects non-pickupable items with `CannotPickup` — correct flag (OTB `pickupable()`), wrong mechanics point. 772 would reject at `CheckMoveObject` layer with `NOTTAKABLE` | Low | Same outcome (rejected) but different error code | Open |
| T41 | **`container_query_add` depot/inbox/store-item checks are OTB concepts, not 1098-era** — withdrawn: OTB is the data source; depot/inbox/store-item types are OTB item-type concepts we keep regardless of `clientVersion` | — | No outcome difference | **Withdrawn** |
| T42 | **Slot validation uses OTB `SlotPosition` bitmask, not 772 `BODYPOSITION`** — withdrawn: OTB is the data source for item types/flags/slots; 772 `BODYPOSITION` is an `objects.srv` concept we don't use. Mechanics outcomes still must match | — | No outcome difference | **Withdrawn** |

---

## 17. T38 — `get_top_down_item` doesn't match `GetTopObject(true)` (High)

### C++ reference (`info.cc:366-388` `GetTopObject`)

```cpp
Object GetTopObject(int x, int y, int z, bool Move){
    Object Obj = GetFirstObject(x, y, z);  // ground
    if(Obj != NONE){
        while(true){
            Object Next = Obj.getNextObject();
            if(Next == NONE) break;
            ObjectType ObjType = Obj.getObjectType();
            if(!ObjType.getFlag(BANK)
                    && !ObjType.getFlag(CLIP)
                    && !ObjType.getFlag(BOTTOM)
                    && !ObjType.getFlag(TOP)
                    && (!Move || !ObjType.isCreatureContainer())){
                break;  // ← this is the "top" object
            }
            Obj = Next;
        }
    }
    return Obj;
}
```

`GetTopObject(true)` walks the **entire tile object chain** (ground → top items → creatures →
down items) and returns the first object that is NOT BANK/CLIP/BOTTOM/TOP/creature-container.
This is the "top moveable" object — the one that `Move()` tries to auto-merge with.

### Rust (`tile.rs:247-249` `get_top_down_item`)

```rust
pub fn get_top_down_item(&self) -> Option<ItemId> {
    self.body().down_items.first().copied()
}
```

Returns only `down_items[0]` — **skips top items and creatures entirely**.

### Impact

- **Top items** (splashes, ladders, signs, borders — OTB `always_on_top() == true`) are never
  returned as the "top moveable" object. If a moveable top item (e.g. a splash) is on the tile,
  772 returns it as `GetTopObject(true)`, but Rust returns the first down item instead.
- **Auto-merge target** (`internal_add_item_to_tile` T22): Rust scans `down_items` for a merge
  target, but 772 uses `GetTopObject(true)` which may return a top item. Different merge target.
- **`internal_get_thing_move`** (T21/T39): Rust returns `down_items.first()` if moveable, else
  falls through to `creatures.last()`. 772 `GetTopObject(true)` may return a top item before
  reaching down items.

### Fix

Port `GetTopObject(Move=true)` as a tile method that walks ground → top_items → creatures →
down_items and returns the first non-BANK/CLIP/BOTTOM/TOP/creature-container object. Use this
in both `internal_get_thing_move` and `internal_add_item_to_tile` (for merge target).

---

## 18. T25 — `Split` parameter not propagated to inventory slot query (Medium)

### C++ reference (`operate.cc:675-731` `CheckInventoryDestination`)

```cpp
void CheckInventoryDestination(Object Obj, Object Con, bool Split){
    ...
    if(HandContainer){
        for(int Position = INVENTORY_HAND_FIRST; Position <= INVENTORY_HAND_LAST; Position += 1){
            Object Other = GetBodyObject(CreatureID, Position);
            if(Other != NONE){
                ...
                if(Split || Other != Obj){  // ← Split relaxes the self-check
                    if(OtherType.isWeapon() && ObjType.isWeapon()){
                        throw ONEWEAPONONLY;
                    }
                }
            }
        }
    }
}
```

The `Split` parameter relaxes the `Other != Obj` check in the `ONEWEAPONONLY` test. When
splitting a stack (e.g. taking 5 of 100 gold from your right hand to your left hand), `Split=true`
means the check runs even when `Other == Obj` (same item in both hand slots — impossible for
non-stackable, but relevant for stackable weapons like throwing knives).

### Rust (`player/inventory/query_add.rs:269-275`)

```rust
let ret = evaluate_player_inventory_slot_query(
    index, classic, it, item_id, item_count, left, right,
);
```

`evaluate_player_inventory_slot_query` has no `Split` parameter — the `ONEWEAPONONLY` check
always uses `Other != Obj` semantics. When splitting a stackable weapon between hand slots,
Rust may wrongly reject with `ONEWEAPONONLY` where 772 would allow it.

### Fix

Add a `split: bool` parameter to `evaluate_player_inventory_slot_query` and propagate it from
`internal_move_item` (which knows whether `m_move < item_count`).

---

## 19. T36 — `NotifyCreature` for old/new owners missing in some arms (Medium)

### C++ reference (`operate.cc:1437-1438` `Move`)

```cpp
NotifyCreature(ObjOwnerID, Obj, OldCon.getObjectType().isBodyContainer());
NotifyCreature(ConOwnerID, Obj, Con.getObjectType().isBodyContainer());
```

772 notifies both the old owner (where the item came from) and the new owner (where it went)
for container UI refresh. This is critical for cross-player moves (e.g. giving an item to
another player's container).

### Rust

`notify_player_container_tree_changed` is called in some cylinder arms (e.g.
`Inventory → Container` at line 500, `Container → Inventory` via `equip_item_to_inventory_slot`)
but **not all**:
- `Tile → Tile`: no owner notification (correct — tiles have no owner)
- `Tile → Container`: `notify_container_stack_merge` but no owner notification
- `Container → Container`: `notify_container_stack_merge` but no owner notification for
  cross-player moves
- `Container → Tile`: no owner notification (correct — tiles have no owner)
- `Inventory → Tile`: no owner notification (correct — dest is tile)
- `Tile → Inventory`: `player_post_add_notification` for dest player (correct)

The cross-player container→container case is the gap: if player A moves an item from their
backpack to player B's backpack, player B's container UI may not refresh.

### Fix

Add `NotifyCreature` equivalent (owner container UI refresh) for container→container and
tile→container arms when the container owner differs from the acting player.

---

## 20. T39 — Creature fallback in `internal_get_thing_move` (Medium)

### C++ reference

772 `GetTopObject(true)` skips creatures entirely (`!Move || !ObjType.isCreatureContainer()` —
when `Move=true`, creature-containers break the loop and are returned, but only if they're
the top object). `GetObject` with `RNum != -1` walks by type match — creatures are only
returned if their `getDisguise()` matches the requested type.

### Rust (`game_world_item_cylinder.rs:68-72`)

```rust
// Fall through to creature
let body = tile.body();
if let Some(&creature_id) = body.creatures.last() {
    return Some(Thing::Creature(creature_id));
}
return None;
```

Rust returns `creatures.last()` as a fallback **regardless of the requested sprite type**.
772 would return `NONE` if no object on the tile matches the client-sent `Type`.

### Impact

If a client sends a move request for sprite ID 100 (a sword) on a tile that has only a
creature (no items), Rust returns the creature as the `Thing`, then `player_move_thing`
matches `Thing::Creature` → creature move stub. 772 would return `NONE` → `NOTACCESSIBLE`.

### Fix

Only return a creature from `internal_get_thing_move` if the client-sent sprite matches a
creature type (or remove the creature fallback entirely for the move path, since 772
`GetTopObject(true)` only returns creatures when they're creature-containers at the top).

---

## 21. T42 — Slot validation: OTB `SlotPosition` is correct, not 772 `BODYPOSITION` (Resolved — not a finding)

### Decision

**Data layer is OTB (TFS 1.4.2), mechanics layer is 772 decompile.** Slot validation uses
OTB `Item::getSlotPosition()` bitmask (`SLOTP_HEAD`, `SLOTP_NECK`, etc.) — this is correct
and intentional. The 772 `BODYPOSITION` attribute is an `objects.srv` concept that we do not
use; OTB is the data source for item types, flags, and slot positions.

The 772 `CheckInventoryDestination` **mechanics outcome** (reject wrong-slot items, reject
two-hand into occupied hand, reject dual weapons) must still be matched — but the **data source**
for "which slot does this item belong in" is OTB, not `objects.srv`.

### Status

**Not a finding.** T42 is withdrawn. The current `evaluate_player_inventory_slot_query` using
OTB `SlotPosition` is correct. The remaining gap is T25 (`Split` parameter not propagated),
which is a mechanics issue, not a data issue.

### T41 — Depot/inbox/store-item checks: also correct (Resolved)

Similarly, T41 is withdrawn. Depot chests, store inbox, and store-item restrictions are OTB
item-type concepts that we keep. They are not "1098-era only" — they're part of the OTB data
layer that the Rust server uses regardless of `clientVersion`. The `MechanicsProfile` gates
**mechanics** (combat formulas, walk speed, condition ticks), not **data layer** concepts.

### Fourth pass (`CheckTopMoveObject` priority walk, `ObjectAccessible` owner check, `Combat.DelayAttack`, `CheckMapDestination` HANG hooks, `receiving.cc` handler)

| # | Finding | Severity | Outcome differs? | Status |
|---|---------|----------|------------------|--------|
| T43 | **`ObjectAccessible` owner-check missing** — 772 `ObjectAccessible` returns `true` immediately if the item has an owner (`OwnerID != 0`) and `OwnerID == CreatureID`; only ownerless items fall through to range/HANG check. The "owner" is determined by the cylinder hierarchy (which player's inventory/container the item is in), not by an OTB flag — this is a 772 **mechanics** concept. Rust has no owner check in the move path — any player within range can move items from containers they have open, even if the container belongs to another player | **Medium** | Yes — other players' container items can be moved | Open |
| T44 | **`Combat.DelayAttack(2000)` for creature-container moves missing** — 772 `TCreature::Move` (`cract.cc:489`) calls `this->Combat.DelayAttack(2000)` when `ObjType.isCreatureContainer()`. Rust `player_move_thing` / `internal_move_item` has no equivalent — moving a creature doesn't delay the actor's next attack | Low | Blocked on T18 (creature push) | Open |
| T45 | **`CheckMapDestination` HANG hook destination range check missing** — 772 has a HANG+hook-specific range check at the destination (`operate.cc:538-573`): asymmetric HOOKSOUTH/HOOKEAST bounds from **player to dest** + z-floor gate. Rust `can_throw_to_tile` has no hook logic; `can_throw_object_to` has no asymmetric range | **Medium** | Yes — HANG items thrown to hooks use wrong range | Open |
| T46 | **`CheckMapDestination` `TAKE` flag uses OTB `pickupable`** — 772 checks `!ObjType.getFlag(TAKE)` for the `ObjectInRange(2)` non-takeable range check. Rust uses `item_is_pickupable` (OTB `pickupable()`). This is correct per the OTB-data decision — `pickupable` is the OTB equivalent of 772 `TAKE` | — | No outcome difference | **OK** |
| T47 | **`CheckTopMoveObject` `GetObjectPriority` / `PRIORITY_LOW` break missing** — 772 walks the tile's container object chain and breaks at `PRIORITY_LOW` (stops searching after low-priority items). Rust `get_top_down_item` returns `down_items.first()` with no priority walk. This is the root cause of T4/T38 — the priority break means 772 may return a different "best" than Rust's "first down item" | Low | Subsumed by T4/T38 | **Duplicate** |
| T48 | **`CheckTopMoveObject` `Ignore` parameter skips the moved object** — 772 skips `Help != Ignore` in the priority walk. Used by `Move()` catch-and-swap (T8) to re-check source top after swapping dest item to source. Rust has no `Ignore` param | Low | Subsumed by T29/T32 | **Duplicate** |
| T49 | **`receiving.cc` `CMoveObject` `Count == 0` for `stackable()` silently drops** — 772 `receiving.cc:258` rejects `CUMULATIVE && Count == 0` silently (no `SendResult`). OTB `stackable()` = 772 `CUMULATIVE`. Rust `player_move_thing` accepts `count: u8` and passes it through — `count == 0` would be treated as "move 0" which `internal_move_item` may handle differently | Low | Edge case — 0-count move | Open |
| T50 | **`receiving.cc` `isMapContainer` type reject missing** — 772 `receiving.cc:258` rejects `Type.isMapContainer()` (type ID 0 = ground/wildcard) silently. Rust doesn't validate the sprite ID against map-container types | Low | Edge case — invalid type ID | Open |
| T51 | **`query_add_item_to_tile` doesn't check `IsMapBlocked` BANK/UNPASS/UNLAY/HANG** — 772 `IsMapBlocked` checks tile coordinate flags (BANK, UNPASS, UNLAY, HANG hooks) for destination validity. Rust `query_add_item_to_tile` checks ground existence + blocking creatures + max items — different predicate. A tile with BANK flag and no UNPASS is always valid in 772 (ground exists); Rust may reject it if no ground item is set. HANG items on hook tiles are valid in 772; Rust requires ground for non-hangable items only | **Medium** | Yes — different accept/reject on BANK/hook tiles | **Fixed** |
| T52 | **`ObjectInRange` is player→dest, not source→dest** — 772 `CheckMapDestination` uses `ObjectInRange(CreatureID, MapCon, 2)` = player to dest range 2. Rust `player_move_item` checks `player_pos → map_to_pos` against `item_throw_range` — same direction (player→dest). But Rust uses `item_throw_range` (15 for pickupable) while 772 uses range 2 for non-takeable only, no range for takeable. This is T15, confirmed correct direction | — | Subsumed by T15 | **Duplicate** |
| T53 | **`can_throw_object_to` range is source→dest, not player→dest** — Rust `can_throw_object_to(map_from_pos, map_to_pos, ...)` checks source-to-dest range. 772 `ThrowPossible` has NO horizontal range limit (only `MinZ` floor stepping). The source→dest range check is a TFS 1.4.2 concept. This is T15, confirmed | — | Subsumed by T15 | **Duplicate** |
| T54 | **`CheckMapDestination` creature-container `MovePossible` gate missing** — 772 calls `MovingCreature->MovePossible(DestX, DestY, DestZ, true, OrigZ != DestZ)` to check if the creature can stand on the dest tile. Rust has no `MovePossible` equivalent (creature push is stubbed, T18) | Low | Blocked on T18 | Open |
| T55 | **`CheckMapDestination` `AVOID` flag + protection-zone gate missing** — 772 rejects pushing creatures to `AVOID` flag tiles and from protection zone to non-protection zone. Rust has no equivalent | Low | Blocked on T18 | Open |
| T56 | **`CheckMapDestination` height-24 jump check missing** — 772 checks `GetHeight(x,y,z) < 24` for up/down creature pushes. Rust has no equivalent | Low | Blocked on T18 | Open |
| T57 | **`Execute` catch `SnapbackNecessary` uses `ToDoClear() \|\| this->Stop`** — 772 sets `SnapbackNecessary = (this->ToDoClear() || this->Stop)` before the `EXHAUSTED` check. Rust `apply_todo_result_catch` uses `had_pending_go` from `player_todo_clear` — this is `ToDoClear()` result, but doesn't include `this->Stop`. If the player is stopped but the queue was already empty, 772 sends snapback, Rust doesn't | Low | Edge case — stopped with empty queue | Open |

---

## 22. Updated remediation priority

| Priority | Finding | Effort |
|----------|---------|--------|
| **P0** | T1+T2+T15: Replace `can_throw_object_to` with `throw_possible(from, to, 1)`; remove throw range limit for takeable items | Small — function exists ✅ |
| **P0** | T3+T51: Replace `can_throw_to_tile` / `query_add_item_to_tile` with `IsMapBlocked` equivalent (BANK/UNPASS/UNLAY/HANG hooks) | Small ✅ |
| **P0** | T21: Add `find_tile_item_by_client_sprite` fallback to Move resolution | Small — function exists ✅ |
| **P0** | T38: Fix `get_top_down_item` to walk full tile priority (top items + creatures + down items) per `GetTopObject(true)` | Medium ✅ |
| **P1** | T4: Port `CheckTopMoveObject` priority walk with `PRIORITY_LOW` break | Medium ✅ |
| **P1** | T5+T43: Add `check_move_object` (ObjectAccessible + owner check + OTB `moveable()` + OTB `pickupable()`) to `internal_move_item` | Medium ✅ |
| **P1** | T7: Port `INVENTORY_ANY` auto-slot scan | Medium ✅ |
| **P1** | T16: Fix `SplitObject` to copy item attributes | Small — add `clone_for_split` ✅ |
| **P1** | T17: Fix tile auto-merge to reject >100 (not partial-merge) | Small ✅ |
| **P1** | T22: Fix merge target to use top item only | Small ✅ |
| **P1** | T25: Propagate `Split` parameter to inventory slot query for `ONEWEAPONONLY` relaxation | Small ✅ |
| **P1** | T36: Add `NotifyCreature` for both old/new owners in all cylinder arms | Medium ✅ |
| **P1** | T39: Fix creature fallback in `internal_get_thing_move` to match `GetTopObject(true)` skip-creature rule | Small ✅ |
| **P1** | T45: Port `CheckMapDestination` HANG hook destination range check (asymmetric HOOKSOUTH/HOOKEAST) | Medium ✅ |
| **P2** | T8+T19+T29+T32: Add `Move`-level catch-and-swap with `Count=-1`, `Ignore` param | Medium ✅ |
| **P2** | T11+T30+T33: Wire `MoveEventsRegistry` into `internal_move_item` with `OldCon != Con` gate | Large (feature) ✅ |
| **P3** | T10+T28: Fix `Merge` error codes (`TooManyParts`/`NoMatch`/`NotCumulable`) — variants added, not yet wired into `Merge` | Trivial |
| **P3** | T12+T20: Add `CMoveObject` handler pre-validation (`CheckSpecialCoordinates`/`CheckVisibility`) | Low |
| **P3** | T49+T50: Add `CMoveObject` handler pre-validation (`Count==0`/`isMapContainer`) | Low ✅ |
| **P3** | T14: HANG hook destination walk-to-reach | Low |
| **P3** | T18+T24+T34+T44+T54+T55+T56: Wire self-move + creature push (`MovePossible`/`AVOID`/protection-zone/height-24/`DelayAttack(2000)`/`NotifyTurn`/`NotifyGo`) | Low |
| **P3** | T23: Guard tile auto-merge with `OldCon != Con` check | Trivial ✅ |
| **P3** | T26: Gate `CheckDepotSpace` directionality (only reject moves **into** depot) | Small ✅ |
| **P3** | T27: Fix `CheckWeight` self-ownership skip (owner == actor, not "currently carries") | Small ✅ |
| **P3** | T31: Add `NoMerge` parameter to `internal_move_item` | Small |
| **P3** | T35: Close container UIs for system-initiated container moves | Low |
| **P3** | T40: Move `pickupable` check from `container_query_add` to `check_move_object` | Small ✅ |
| **P3** | T57: Include `this->Stop` in `SnapbackNecessary` calculation | Trivial ✅ |
| **Deferred** | T6: Creature-container `CheckMapDestination` (blocked on creature push, D9) | — |

---

## 23. What is already correct

- **ToDoMove builder** (`enqueue_player_move`): `Wait{100}` prefix, z-floor gate,
  object validation — all match `cract.cc:1123-1172`. (D1/D2/D6 fixed in prior audit.)
- **Move execute arm** (`idle_stimulus.rs:2665`): `Go`-prepend walk-to-reach mirrors
  `cract.cc:1138-1139` `ToDoGo`. Re-validates object at execute time (`Obj.exists()`).
- **`ThrowPossible` implementation** (`map/los.rs:87`): correctly implements major-axis
  interpolation, `MinZ` stepping, `HOOKEAST`/`HOOKSOUTH` `StartT=0` — verified against
  `info.cc:1154-1216`. **But it's not called from the throw path** (T1/T2).
- **Cylinder enum dispatch** (`cylinder.rs`): enum over Tile/Container/Inventory is
  zero-cost and exhaustive — correct Rust idiom replacing C++ virtual hierarchy.
- **`queryDestination` chain** (`resolve_move_destination`): correctly loops through
  `player_query_destination` → `container_query_destination` with 16-iteration cap.
- **Stack merge/split**: `merge_partial_stack_counts` / `merge_detached_stack_counts` /
  `stack_merge_room` correctly handle the 100-cap and partial-vs-full merge cases.
- **`NeedExchange`** in `player_query_add`: correctly returns `NeedExchange` when dest slot
  occupied, and `try_resolve_inventory_need_exchange` handles the swap.
- **Container capacity** (`container_query_add`): depot limit, inbox reject, store-item
  restrictions, parent chain cycle detection — all ported. These are OTB data-layer concepts
  (depot chest, store inbox, store-item flag) that we keep regardless of `clientVersion`.
- **Sprite re-validation** (`validate_move_object_ref`): correctly mirrors `Obj.exists()`
  type check at enqueue and execute time.
- **OTB flag usage**: `pickupable()`, `moveable()`, `stackable()`, `is_hangable()`,
  `block_projectile()`, `block_solid()`, `always_on_top()` — all read from OTB, not `objects.srv`.
  The **flags** are correct; the **mechanics points** where they're checked are the findings (T5, T15, T40).
- **`SlotPosition` bitmask** (OTB): used for inventory slot validation in
  `evaluate_player_inventory_slot_query` — correct per OTB-data decision. 772 `BODYPOSITION`
  is an `objects.srv` concept we don't use.

---
