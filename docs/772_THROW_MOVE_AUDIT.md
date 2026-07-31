# 772 Throw / Move Item — Parity Audit + Implementation Plan

**Audited:** 2026-07-30 (supersedes the 2026-07-11 pass)
**Reference (772 mechanics):** `reference/cipsoft-772/tibia-game-master/src/` — `operate.cc`,
`cract.cc`, `info.cc`, `map.cc`, `receiving.cc`, `enums.hh`.
**Reference (772 wire):** `reference/tvp-772/gameserver/src/` — not used here (no packet changes).

**Rust files audited:**

| Rust file | 772 counterpart |
|---|---|
| `game_world_player_throw.rs` | `cract.cc:475` `TCreature::Move`, `receiving.cc:233` `CMoveObject` |
| `game_world_item_move.rs` | `operate.cc:1275` `Move`, `operate.cc:1449` `Merge`, `operate.cc:418` `CheckMoveObject` |
| `game_world_item_cylinder.rs` | `info.cc:366` `GetTopObject`, `info.cc:398` `GetObject`, `operate.cc:451` `IsMapBlocked` |
| `map/los.rs` | `info.cc:1154` `ThrowPossible` |
| `container_ops.rs` | `operate.cc:606` `CheckContainerDestination`, `operate.cc:646` `CheckDepotSpace` |
| `player/inventory/query_add.rs` | `operate.cc:675` `CheckInventoryDestination` |
| `creature_todo.rs` / `idle_stimulus.rs` | `cract.cc:1123` `ToDoMove`, `cract.cc:823` `Execute` `TDMove` |

**Data-vs-mechanics split (unchanged decision):** the data layer is OTB (item types, flags, slot
bitmask); the mechanics layer is the 772 decompile (move/merge/throw outcomes, LOS, validation
order). When a finding says "772 checks flag X", the fix is "check the OTB equivalent at the same
mechanics point", not "add 772 flag X to the data layer". The one exception this audit surfaces is
`UNLAY`, which currently has **no** OTB equivalent at all (see B4).

---

## 0. Verdict summary

Structurally the port is sound. `ThrowPossible`, `CheckTopMoveObject`, `IsMapBlocked`, the
inventory catch-and-swap, the `Split` / `Ignore` / `NoMerge` parameters, HANG-hook walk-to-reach,
and Lua move-event firing are all present and faithful. The remaining defects cluster in three
areas: **failed merges abort the whole move**, the **`CreatureID == 0` / non-player early returns
were not ported**, and **`UNLAY` has no data source**.

| # | Finding | Severity | Outcome differs? |
|---|---------|----------|------------------|
| B1 | Failed tile merge aborts the move instead of falling through to a separate stack | **High** | Yes |
| B2 | `CheckMoveObject` missing the `CreatureID == 0` early return; owner check rejects actor-less moves | **High** | Yes |
| B3 | `CheckTopMoveObject` applied to non-player actors (monster box-kick) | Medium | Yes |
| B4 | `UNLAY` has no data source → `IsMapBlocked` never blocks a wall/tree tile | **High** | Yes |
| B5 | Partial merge into inventory/container silently drops the remainder | Medium | Yes |
| B6 | `INVENTORY_ANY` (`DestY = 0`) treated as a literal slot index | Medium | Yes |
| B7 | `ObjectInRange` z-equality missing on the non-pickupable range gate | Low | Yes |
| B8 | `unlay` is an unregistered `items.xml` key — stringly-typed, unvalidated, warns at load | Medium | Contributes to B4 |
| G1 | `CheckWeight` charged to the actor, not the destination container's owner | Medium | Yes |
| G2 | `CheckMapDestination` only runs in the client path | Medium | Yes |
| G3 | `CheckTopMoveObject` walk shape: no `PRIORITY_LOW` break, dead `top_items` loop | Low | Edge |
| G4 | One `moveable()`-filtered helper serves both `GetTopObject` and `CheckTopMoveObject` | Low | Edge |
| G5 | Post-move `CloseContainer(Obj, false)` missing | Low | Edge |
| G6 | `CheckSpecialCoordinates` only applied to map sources | Low | Edge |

### Observed in-game symptoms → root cause

| Symptom | Cause |
|---|---|
| Items can be placed on top of trees | **B4** (+ **B8**) — a tree tile is `BANK` + `UNPASS` with no `UNLAY`; the `has_bank && !has_unlay → not blocked` leg of `is_map_blocked` accepts the drop. `UNLAY` is only readable as an untyped `items.xml` string present on 6 items |
| Different item types won't stack on the same tile (gold coin + backpack) | **B1** for the stackable direction — dropping a stackable onto a tile whose top moveable object is a different type returns `NoMatch` and moves nothing. The non-stackable direction needs a repro (task P0-0) — no rejection is reachable in the current code path |

---

## 1. B1 — Failed tile merge aborts the move (High)

### 772 reference (`operate.cc:1304-1320`)

```cpp
if(!NoMerge && ConType.isMapContainer() && ObjType.getFlag(CUMULATIVE) && OldCon != Con){
    Object Top = GetTopObject(ConX, ConY, ConZ, true);
    if(Top != NONE){
        try{
            Merge(CreatureID, Obj, Top, Count, Ignore);
            return;                      // merge succeeded
        }catch(RESULT r){
            if(r == DESTROYED){ throw; } // only DESTROYED propagates
        }
    }
}
// ...falls through to MoveObject(Obj, Con) — item lands as a SEPARATE stack
```

`Merge` throws `NOMATCH` (different type), `NOTCUMULABLE` (not stackable), and `TOOMANYPARTS`
(`Count + DestCount > 100`). All three are swallowed and the move continues.

### Rust

```378:398:crates/tfs-rust-core/src/game_world_item_cylinder.rs
        if is_stackable
            && !flags.contains(CylinderFlags::IGNORE_AUTO_STACK)
            && !flags.contains(CylinderFlags::NO_MERGE)
        {
            if let Some(target_id) = self.get_top_object_for_move(pos, None) {
                self.merge_check(item_id, target_id, item_count)?;
```

The `?` propagates. Same pattern in the three tile-destination arms of `internal_move_item`:
`game_world_item_move.rs:296-301` (Tile→Tile), `:393-398` (Container→Tile), `:850-855`
(Inventory→Tile).

### Impact

- Throwing 50 gold onto a tile holding 80 gold: 772 → `[80, 50]`; Rust → `TooManyParts`, nothing moves.
- Dropping gold onto a tile whose top moveable object is a sword: 772 → separate stack; Rust →
  `NoMatch`, nothing moves. **This is the reported "different items won't stack" symptom.**
- The tile arms additionally pass `item_count` (the full source stack) to `merge_check` rather than
  the move count `m`, so a 100-cap rejection fires even when only part of the stack is moving.

### Fix

Treat a failed tile merge as "no merge target" instead of an error. Introduce a helper that mirrors
the 772 try/catch and keep `DESTROYED` (Rust: item no longer exists) as the only propagating case:

```rust
/// 772 `Move` (`operate.cc:1311-1319`) — a failed `Merge` falls through to a separate
/// stack; only `DESTROYED` propagates. Returns `Some(target)` only when the merge is legal.
fn tile_merge_target(&self, item_id: ItemId, pos: Position, count: u16) -> Option<ItemId> {
    let target = self.get_top_object_for_move(pos, None)?;
    self.merge_check(item_id, target, count).ok().map(|_| target)
}
```

Replace all four `merge_check(...)?` call sites with this, and pass the move count (`m` / `m_move`),
not `item_count`.

---

## 2. B2 — `CheckMoveObject` missing the `CreatureID == 0` early return (High)

### 772 reference (`operate.cc:418-447`)

```cpp
void CheckMoveObject(uint32 CreatureID, Object Obj, bool Take){
    if(CreatureID == 0){ return; }          // <-- engine/system moves skip ALL checks
    if(!ObjectAccessible(CreatureID, Obj, 1)){ throw NOTACCESSIBLE; }
    if(ObjType.getFlag(UNMOVE)){ throw NOTMOVABLE; }
    // ... creature-push gate ...
    if(Take && !ObjType.getFlag(TAKE)){ throw NOTTAKABLE; }
}
```

`ObjectAccessible` (`info.cc:258-264`) returns `OwnerID == CreatureID` for any owned object and
never falls through to the range check for those.

### Rust

```61:65:crates/tfs-rust-core/src/game_world_item_move.rs
        if let Some(owner_id) = owner {
            if !actor.is_some_and(|a| a == owner_id) {
                return Err(ReturnValue::NotPossible);
            }
        }
```

`check_move_object` runs unconditionally from `internal_move_item:131`. With `actor == None` and an
inventory- or container-owned source, `actor.is_some_and(..)` is `false`, so the move is rejected.

### Impact

Every actor-less mover fails: Lua `item:moveTo` without an acting player
(`game_world_inventory.rs:320` passes `acting`, which may be `None`), and any future
decay/system relocation out of a player's container. The `moveable()` and `pickupable()` legs
also reject items that 772 lets the engine relocate freely.

### Fix

```rust
fn check_move_object(&self, actor: Option<CreatureId>, ...) -> Result<(), ReturnValue> {
    // 772 `CheckMoveObject` (`operate.cc:419-421`): `CreatureID == 0` skips every check.
    let Some(actor) = actor else { return Ok(()) };
    ...
}
```

Take `actor: CreatureId` for the rest of the body so the owner and HANG-hook legs stop needing
`is_some_and` / `else return Err` ladders. While here, add the missing `ObjectInRange(1)` leg for
ownerless (tile) sources so non-client callers get the same reach gate the client path gets from
`enqueue_player_move`.

---

## 3. B3 — `CheckTopMoveObject` applied to non-player actors (Medium)

### 772 reference (`operate.cc:302-310`)

```cpp
if(CreatureID == 0 || !IsCreaturePlayer(CreatureID)){ return; }
Object Con = Obj.getContainer();
if(!Con.getObjectType().isMapContainer()){ return; }
```

Only **players** are subject to the top-object rule; monsters and the engine are not.

### Rust

```125:129:crates/tfs-rust-core/src/game_world_item_move.rs
        if let Cylinder::Tile { pos } = from_cylinder {
            if self.get_top_object_for_move(pos, ignore) != Some(item_id) {
                return Err(ReturnValue::NotPossible);
            }
        }
```

No actor test. `monster_push.rs:340` passes the monster as the actor.

### Impact

Monster box-kick (`KickBoxes`) fails whenever the kicked item isn't the top moveable object on its
tile, and the fallback path then **deletes** the item instead of pushing it.

### Fix

Gate the block on `acting_player.is_some_and(|a| self.is_player(a))`. A small
`fn actor_is_player(&self, actor: Option<CreatureId>) -> bool` helper covers both B2 and B3 call
sites.

---

## 4. B4 — `UNLAY` has no data source (High) — the tree symptom

### 772 reference (`operate.cc:451-472`)

```cpp
static bool IsMapBlocked(int DestX, int DestY, int DestZ, ObjectType Type){
    bool HasBank = CoordinateFlag(DestX, DestY, DestZ, BANK);
    if(HasBank && !CoordinateFlag(DestX, DestY, DestZ, UNPASS)){ return false; }
    if(!Type.getFlag(UNPASS)){
        if(HasBank && !CoordinateFlag(DestX, DestY, DestZ, UNLAY)){ return false; }
        ...
    }
    return true;   // blocked
}
```

The second leg is the one that protects walls, trees, and furniture: a tile with ground **and** a
blocking object is only "layable" when that object lacks `UNLAY`. In 772 `objects.srv`, walls,
trees, and bookcases all carry `UNLAY`.

### Rust

```278:279:crates/tfs-rust-core/src/game_world_item_cylinder.rs
                if t.xml_attributes.get("unlay").map(|v| v == "true").unwrap_or(false) {
                    has_unlay = true;
```

`unlay` exists on exactly **six** bookcase entries in `data/items/items.xml`. For a tree tile
(`ground` present, tree `block_solid`, no `unlay`) the flow is: `has_bank && !has_unpass` → false
(tree is UNPASS, skip) → item is not UNPASS → `has_bank && !has_unlay` → **`return false` (not
blocked)** → the item lands on the tree.

Because `query_add_item_to_tile` now delegates entirely to `is_map_blocked`
(`game_world_item_cylinder.rs:228-231`), there is no `block_solid` fallback either — the old TFS
`TILESTATE_BLOCKSOLID` gate (`src/tile.cpp:608`) was replaced, not supplemented.

### Fix

Derive `UNLAY` from OTB instead of a hand-maintained XML attribute. The behavioural definition —
"a solid object you cannot lay things on top of" — maps to an immovable blocking item:

```rust
/// 772 `UNLAY` (`enums.hh:239`) has no direct OTB flag. Behavioural equivalent: an
/// immovable solid object (wall, tree, furniture) blocks laying; a movable solid
/// (parcel, chest) does not. `data/items/items.xml` `unlay="true"` stays as an override.
fn item_type_is_unlay(t: &ItemType) -> bool {
    t.xml_attributes.get("unlay").map(|v| v == "true").unwrap_or(false)
        || (t.block_solid() && !t.moveable() && !t.pickupable())
}
```

Apply to the ground item and to every item in the top/down groups, replacing both
`xml_attributes` reads in `is_map_blocked`. Validate against a handful of real map tiles (tree,
stone wall, parcel, chest, table) before/after, since this flag decides both "throw onto" and
`CheckMapPlace`.

---

## 4b. B8 — `unlay` is an unregistered `items.xml` key (Medium)

The server log confirms the data path at startup:

```
WARN tfs_rust_content::items: unknown items.xml key (first occurrence; key stored in
xml_attributes) item_id=1718 key="unlay"
```

`unlay` is absent from `KNOWN_XML_KEYS` (`items_xml_keys.rs:11-138`), so it falls through to the
catch-all at `items.rs:758-765` and is stored as a raw string in `xml_attributes`. Consequences:

- **No validation.** A typo (`unlai`, `unLay` on an unnormalised path) silently becomes a no-op
  with no error — the tile just stops blocking. A mechanics flag should not be spelled at runtime.
- **Load-time noise.** The warning fires on every boot, training everyone to ignore it.
- **Reinforces B4.** The only reason `is_map_blocked` compiles today is that it reads the untyped
  map. Fixing B4 by adding `unlay="true"` to every wall and tree in `items.xml` is not viable
  (thousands of entries) — which is why P0-2 derives it from OTB instead.

### Sibling keys in the same warning block

The same block flags five other hand-added 772 attributes that are stored but never read. Only the
first is in this audit's scope; the rest are logged here so they aren't lost:

| Key | Item | 772 meaning | Status |
|---|---|---|---|
| `unlay` | 1718 bookcase | `UNLAY` (`enums.hh:239`) — cannot lay objects on it | **B4 / B8, this audit** |
| `forceuse` | 1386 | `FORCEUSE` — breaks the `CheckTopUseObject` / `CheckTopMultiuseObject` priority walk (`operate.cc:368`, `:404`) | Unread anywhere in `tfs-rust-core`; **Use-path gap, separate audit** |
| `replacemagicfields` | 1487 | magic-wall / field replacement (`moveuse.cc:2184`) | Out of scope |
| `specialfieldblockpath` | 1506 | field pathfinding cost | Out of scope |
| `poisondamagecycles` | 2545 | poison decay cycles | Out of scope (conditions) |
| `blockpathfind` | 4351 | pathfinding block | Out of scope (walk) |

`forceuse` is worth calling out: it is the direct analogue of this audit's top-object findings for
the **Use** path, and it is currently dead data — `CheckTopUseObject`'s `FORCEUSE` break cannot be
implemented until the key is typed.

### Fix (bundled into P0-2)

1. Add `"unlay"` to `KNOWN_XML_KEYS` (keep the list alphabetical — there is a `known_keys_sorted`
   test guarding this).
2. Promote it to a typed `ItemType` field (`unlay: bool`) parsed in the items.xml merge, mirrored
   into `xml_attributes` the way the other Phase-2 keys are.
3. Have `ItemType::is_unlay()` (§4) return `self.unlay || (block_solid && !moveable && !pickupable)`
   so the XML value is an explicit override of the OTB derivation.
4. Update the `items_xml_keys.rs` module doc Phase-2 list.

---

## 5. B5 — Partial merge drops the remainder (Medium)

### 772 reference (`cract.cc:578-599`)

```cpp
int MergeCount = MoveCount;
if((DestAmount + MergeCount) > 100){ MergeCount = 100 - DestAmount; }
if(MergeCount > 0){
    try{
        ::Merge(this->ID, Obj, DestObj, MergeCount, NONE);
        MoveCount -= MergeCount;
        if(MoveCount <= 0){ return; }     // fully absorbed
    }catch(RESULT r){ if(r == TOOHEAVY){ throw; } }
    DestObj = NONE;
}
// ...remainder continues into ::Move(this->ID, Obj, DestCon, MoveCount, false, DestObj)
```

### Rust

`m_move` is clamped to the merge room (`game_world_item_move.rs:261-263`) and each merge arm
returns `Ok(merge_id)` — the leftover `m - m_move` never moves.

### Fix

In `player_move_item`, mirror the 772 two-step: merge what fits, then re-enter `internal_move_item`
with the remainder and `to_merge_item` suppressed (`CylinderFlags::NO_MERGE`), letting the existing
catch-and-swap handle an occupied slot. Keep this at the `TCreature::Move` layer — 772 does the
pre-merge there, not inside `Move`.

---

## 6. B6 — `INVENTORY_ANY` treated as a slot index (Medium)

`INVENTORY_ANY = 0` (`enums.hh:308`). 772 `TCreature::Move` (`cract.cc:501-547`) scans
`INVENTORY_FIRST..=INVENTORY_LAST` with `CheckInventoryDestination`, **preferring** the first slot
that is not right-hand / left-hand / ammo, then falls back to scanning inventory containers with
`CheckContainerDestination`, and throws `NOROOM` if nothing fits.

Rust maps `pos.y == 0` to `Inventory { slot: 0 }`:

```42:45:crates/tfs-rust-core/src/game_world_item_cylinder.rs
        Some(Cylinder::Inventory {
            player_id: cid,
            slot: pos.y as u8,
        })
```

No slot index accepts `0`, so the move fails with `NotPossible`. Nothing in the workspace
references this constant — it is entirely unported.

### Fix

Add `fn resolve_inventory_any(&mut self, cid, item_id, count) -> Option<Cylinder>` implementing the
two-pass scan, and call it from `player_move_item` before `internal_get_cylinder` when
`to_pos.x == 0xFFFF && to_pos.y == 0`. Reuse `player_query_add` for pass 1 and
`container_query_add` for pass 2 so the existing `CheckInventoryDestination` /
`CheckContainerDestination` semantics apply unchanged.

---

## 7. B7 — `ObjectInRange` z-equality missing (Low)

`ObjectInRange` (`info.cc:247-249`) is `posz == ObjZ && |dx| <= Range && |dy| <= Range`. The
non-pickupable gate omits the z term:

```372:378:crates/tfs-rust-core/src/game_world_player_throw.rs
        if !item_is_pickupable {
            let to_dx = (player_pos.x as i32 - map_to_pos.x as i32).unsigned_abs();
            let to_dy = (player_pos.y as i32 - map_to_pos.y as i32).unsigned_abs();
            if to_dx > 2 || to_dy > 2 {
                return Err(ReturnValue::DestinationOutOfReach);
            }
        }
```

A non-pickupable item can therefore be pushed to a different floor within 2 tiles whenever
`throw_possible(.., power = 1)` allows it. The enqueue-time `ObjectInRange(1)` check
(`creature_todo.rs:567-573`) has the same omission, mitigated there by
`validate_action_object_z_floor`.

### Fix

Add a shared `fn object_in_range(&self, actor: CreatureId, pos: Position, range: u32) -> bool`
with the z-equality term and use it at both sites.

---

## 8. Gaps (not implemented)

**G1 — `CheckWeight` charged to the wrong creature (Medium).** 772 `Move:1367-1369` calls
`CheckWeight(ConOwnerID, Obj, Count)` — the *destination container's* owner — and rejects
`!TAKE` items with `TOOHEAVY` (`operate.cc:806`). `container_query_add` only runs the capacity
check when the **actor** holds the container tree (`container_ops.rs:324-338`), so moving into
another creature's container skips their capacity entirely. Fix: resolve the destination root
container's owner and check against that creature, not the actor.

**G2 — `CheckMapDestination` only in the client path (Medium).** `internal_move_item` performs
`is_map_blocked` but not `ObjectInRange(2)`, `ThrowPossible`, or the HANG-hook destination range
check; those live in `player_move_item`. Monster and Lua movers with an actor can place items
through walls or out of range. The inverse also holds: `is_map_blocked` runs even for actor-less
moves, where 772 skips `CheckMapDestination` outright. Fix: move the three checks into a
`check_map_destination(actor, item, pos)` called from `internal_move_item` for tile destinations,
gated on `actor.is_some()`, and delete the duplicate from `player_move_item`.

**G3 — `CheckTopMoveObject` walk shape (Low).** 772 (`operate.cc:319-337`) walks the whole object
chain including creatures (`BestIsCreature`) and **breaks** at `PRIORITY_LOW`.
`get_top_object_for_move` skips priority-bottom items and continues, and its `top_items` loop is
dead code — `!t.always_on_top()` can never hold for an item in `top_items`.

**G4 — One helper serves two 772 functions (Low).** 772 uses `GetTopObject(true)` (no `moveable()`
filter) as the auto-merge target and `CheckTopMoveObject`'s `Best` (moveable-filtered) as the
movable candidate. Rust uses the moveable-filtered helper for both, so a tile whose topmost
stackable is immovable picks a different merge target than 772. Split into
`get_top_object(pos)` and `get_top_move_candidate(pos, ignore)`.

**G5 — Post-move `CloseContainer(Obj, false)` (Low).** `operate.cc:1440-1442` closes/refreshes
container UIs after every move of a container item. Rust only handles the `CreatureID == 0`
pre-move case (`game_world_item_move.rs:138-144`).

**G6 — `CheckSpecialCoordinates` only on map sources (Low).** `player_move_thing` validates z and
visibility only when `from_pos.x != 0xFFFF` (`game_world_player_throw.rs:52-59`); 772 validates
both endpoints (`receiving.cc:262-270`).

---

## 9. What is already correct

- **`ThrowPossible`** (`map/los.rs:87`) — faithful port of `info.cc:1154`: `MinZ` ceiling stepping,
  major-axis interpolation, `HOOKEAST`/`HOOKSOUTH` `StartT = 0` origin case, `UNTHROW`-only tile
  test. It is now actually called from the item throw path.
- **Range model** — no distance limit for `pickupable()` items, `ObjectInRange(2)` otherwise,
  matching `operate.cc:489` (the old TFS `throwRange = 15` is gone).
- **`clone_for_split`** (`item.rs:45`) — preserves attributes, matching `map.cc` `CopyObject`.
- **`Split` propagation** — reaches `evaluate_player_inventory_slot_query_with_split`
  (`query_add.rs:296-299`) for the `ONEWEAPONONLY` relaxation (`operate.cc:724`).
- **`merge_check` error codes** — returns `NoMatch` / `NotCumulable` / `TooManyParts` exactly as
  `operate.cc:1470-1486` throws them (the bug is that callers propagate them).
- **Catch-and-swap** (`game_world_player_throw.rs:433-463`) — result list matches `cract.cc:610`,
  with `Ignore` threaded into the retry.
- **HANG hooks** — `is_hang_hook_accessible` reproduces the asymmetric `ObjectAccessible` bounds
  (`info.cc:279-295`); `hang_hook_walk_to_reach` reproduces `cract.cc:630-646`.
- **Move events** — `on_remove_item` / `on_step_out` / `on_add_item` fire behind the
  `OldCon != Con` gate, matching `operate.cc:1379-1381` + `:1444-1446`.
- **Creature push** — `MovePossible`, height-24 jump gate, protection-zone gate, `NotifyTurn` /
  `AnnounceMovingCreature` / `NotifyGo` ordering, and `DelayAttack(2000)` all present.
- **Tile ordering** — `add_item` inserts at the front of `down_items`, so index 0 is the topmost
  object and the `GetTopObject` walk direction is correct.

---

## 10. Implementation plan

Each phase is independently shippable and ends green on `cargo check` + `cargo test`.

### P0 — Correctness blockers (fixes both reported symptoms)

| Task | Finding | Files | Effort | Status |
|---|---|---|---|---|
| **P0-0** Reproduce the non-stackable stacking failure | symptom 2 | — | S | Not run |
| **P0-1** Failed tile merge falls through to a separate stack | B1 | `game_world_item_cylinder.rs`, `game_world_item_move.rs` | S | Done |
| **P0-2** Derive `UNLAY` from OTB + register/type the `unlay` key | B4, B8 | `otb.rs`, `items.rs`, `items_xml_keys.rs`, `game_world_item_cylinder.rs` | M | Done |
| **P0-3** `CheckMoveObject` / `CheckTopMoveObject` actor gates | B2, B3 | `game_world_item_move.rs` | S | Done |

**P0 status:** Implemented in commit `20417db`. `cargo check --workspace`, `cargo clippy`, and `cargo test -p tfs-rust-content` pass; `cargo test -p tfs-rust-core` has 3 unrelated pre-existing failures.

**P0-0** must run first because it decides whether P0-1 fully closes the reported stacking symptom.
Static analysis shows no reachable rejection for a **non**-stackable item landing on an occupied
tile, so if a backpack genuinely refuses to land on a gold-coin tile there is a second cause.
Repro steps: place a gold coin, drag a backpack onto that tile, and capture (a) the client's cancel
message, (b) `RUST_LOG=tfs_rust_core=trace` around `execute_move` / `execute_move_done`, and
(c) the `ReturnValue` from `player_move_item`. If the cancel is `NotPossible` with no
`merge_check` involvement, bisect `internal_get_thing_move` → `check_move_object` →
`query_add_item_to_tile`.

**P0-1** — add `tile_merge_target()` (§1) and replace the four `merge_check(...)?` sites; pass the
move count rather than `item_count`.

**P0-2** — two halves. Data: register `"unlay"` in `KNOWN_XML_KEYS`, add the typed
`ItemType::unlay` field, clear the load warning (B8). Mechanics: add `ItemType::is_unlay()` to
`otb.rs` with the immovable-solid derivation plus the typed override, and use it for both the ground
and item-group legs of `is_map_blocked`. Verify against real map tiles (tree, stone wall, parcel,
chest, table) that `is_map_blocked` flips to `true` for trees/walls and stays `false` for ordinary
floor. Do the data half first so the mechanics half has a typed flag to read.

**P0-3** — early-return `Ok(())` from `check_move_object` when `actor.is_none()`; gate the
`CheckTopMoveObject` block on the actor being a player. Add the `ObjectInRange(1)` leg for tile
sources while the function is being reshaped.

### P1 — Missing mechanics

| Task | Finding | Files | Effort | Status |
|---|---|---|---|---|
| **P1-1** `INVENTORY_ANY` two-pass auto-slot scan | B6 | `game_world_player_throw.rs`, `game_world_item_cylinder.rs` | M | Done |
| **P1-2** Partial-merge remainder continues into `Move` | B5 | `game_world_player_throw.rs` | M | Done |
| **P1-3** `CheckWeight` against the destination container's owner | G1 | `container_ops.rs` | M | Done |
| **P1-4** Move `CheckMapDestination` into `internal_move_item` | G2 | `game_world_item_move.rs`, `game_world_player_throw.rs` | M | Done |
| **P1-5** Shared `object_in_range` with z-equality | B7 | `game_world_player_throw.rs`, `creature_todo.rs` | S | Done |

**P1 status:** Implemented in commit `3e1fc29`. `cargo check --workspace`, `cargo clippy --workspace --all-targets`, and `cargo test -p tfs-rust-core` pass with the same 3 unrelated pre-existing failures as P0.

### P2 — Fidelity polish

| Task | Finding | Files | Effort |
|---|---|---|---|
| **P2-1** Split `get_top_object` from `get_top_move_candidate`; add the `PRIORITY_LOW` break; drop the dead `top_items` loop | G3, G4 | `game_world_item_cylinder.rs` | M |
| **P2-2** Post-move `CloseContainer(Obj, false)` | G5 | `game_world_item_move.rs` | S |
| **P2-3** `CheckSpecialCoordinates` on both endpoints | G6 | `game_world_player_throw.rs` | S |

### Verification

```bash
rtk cargo check --workspace
rtk cargo clippy --workspace --all-targets
rtk cargo test -p tfs-rust-core
rtk cargo test --workspace
```

### Tests to add

| Test | Covers |
|---|---|
| `tile_merge_overflow_creates_second_stack` — 80 gold on a tile + throw 50 → `[80, 50]`, no error | B1 |
| `tile_merge_type_mismatch_places_separate_stack` — sword on top + throw gold → both present | B1 / symptom 2 |
| `is_map_blocked_rejects_tree_and_wall_tiles` — tree, stone wall blocked; floor, parcel not | B4 / symptom 1 |
| `unlay_key_is_known_and_typed` — `items.xml` `unlay="true"` sets `ItemType::unlay` and emits no warning | B8 |
| `unlay_derived_for_immovable_solids` — tree/wall derive `is_unlay()`; movable solids do not | B4 / B8 |
| `system_move_out_of_container_succeeds_without_actor` — `internal_move_item(None, ..)` from a container | B2 |
| `monster_kick_moves_buried_box` — box under another moveable item is pushed, not deleted | B3 |
| `inventory_any_places_into_first_free_non_hand_slot` — `DestY = 0` → head/armor before hands | B6 |
| `partial_merge_moves_remainder` — 50 gold onto an 80-gold hand stack → hand 100 + 30 relocated | B5 |
| `non_pickupable_rejects_cross_floor_push` — `dz != 0` within 2 tiles → `DestinationOutOfReach` | B7 |

---

## Appendix A — `items.xml` key registry sweep (2026-07-30)

Full diff of `data/items/items.xml` attribute keys against `KNOWN_XML_KEYS`
(`items_xml_keys.rs:11-138`), cross-referenced with every stringly-typed
`xml_attributes.get(..)` / `contains_key(..)` read site in `crates/`.

**Totals:** 67 distinct keys used in `items.xml`; 126 registry entries; **6 unregistered
top-level keys** (all six warn at load) plus 2 nested keys that are stored as `field.*`
composites and therefore never warn.

### A.1 Unregistered top-level keys — all warn on every boot

| Key | Items | Read at runtime? | Read site | Verdict |
|---|---|---|---|---|
| `unlay` | 6 | **Yes — stringly** | `game_world_item_cylinder.rs:278`, `:293` | **B4 / B8.** Mechanics decision driven by an unvalidated string on 6 of thousands of blocking items |
| `poisondamagecycles` | 1 | **Yes — stringly** | `player/combat/ranged.rs:385` | **Same defect class as B8.** Register + type it; conditions area |
| `forceuse` | 2 (ladder 1386, …) | No | — | Dead data. 772 `FORCEUSE` breaks the `CheckTopUseObject` / `CheckTopMultiuseObject` priority walk (`operate.cc:368`, `:404`) — unimplementable until typed. **Use-path audit** |
| `replacemagicfields` | 7 | No | — | Dead data. `moveuse.cc:2184` field-replacement rule. Out of scope |
| `specialfieldblockpath` | 3 | No | — | Dead data. Field pathfinding cost. Out of scope (walk) |
| `blockpathfind` | 3 | No | — | Dead data. Pathfinding block. Out of scope (walk) |

Two distinct defects fall out of this table:

1. **Stringly-typed mechanics** (`unlay`, `poisondamagecycles`) — a live game rule reads an
   unregistered, unvalidated string. A typo degrades silently. Both need registry entries and
   typed `ItemType` fields.
2. **Dead 772 data** (`forceuse`, `replacemagicfields`, `specialfieldblockpath`, `blockpathfind`) —
   someone hand-added the attribute to `items.xml` in anticipation of a port that never landed.
   The data is inert; the warning is the only trace. Each needs either a port or removal.

### A.2 Nested `field` keys — stored as composites, never warn, never read

`initdamage` (12 items) and `cycles` (15 items) appear as children of
`<attribute key="field" value="…">` (e.g. campfire 1423: `initdamage=20`, `cycles=70`) and are
stored as `field.initdamage` / `field.cycles` by the composite path at `items.rs:703`. Together
with the registered `field.ticks` / `field.count`, they are read **only in `items.rs` unit tests**
— no runtime consumer.

**This is correct by design, not a finding.** Per `TFS-mechanics-profile.mdc`, 772 field damage and
tick counts come from `MechanicsProfile` / `data/formulas/772.lua`
(`ConditionTicks { fire: 10/8, energy: 25/10, poison_start: 50 }`), not from `items.xml`. The
parent `field` value *is* read at runtime (`items.rs:282` `avoid_damage_type`) to resolve the
damage **type**, which is the correct split: type from data, magnitude from profile. Worth a
one-line comment at the composite parse site so the next reader doesn't "fix" it.

### A.3 Registry entries unused by `items.xml` (67)

`absorbpercent*`, `element*`, `suppress*`, `skilldist`, `maxhitpoints`, `transformto`,
`vocation`, … — upstream TFS `ItemParseAttributesMap` keys the current data pack simply doesn't
use. **No action:** the registry is intentionally a superset so a future data pack doesn't
regress into warnings.

### A.4 Reproducing the sweep

```bash
python3 - <<'PY'
import re, collections, pathlib
xml = pathlib.Path('data/items/items.xml').read_text(errors='replace')
keys = collections.Counter(k.lower() for k in re.findall(r'<attribute\s+key="([^"]+)"', xml))
reg = pathlib.Path('crates/tfs-rust-content/src/items_xml_keys.rs').read_text()
block = reg.split('KNOWN_XML_KEYS: &[&str] = &[', 1)[1].split('];', 1)[0]
known = set(re.findall(r'"([^"]+)"', block))
for k, v in sorted(((k, v) for k, v in keys.items() if k not in known), key=lambda kv: -kv[1]):
    print(f"{k:28} {v:5} item(s)")
PY
```

Note the regex is flat, so nested `field` children (A.2) show up as top-level; cross-check any hit
against the surrounding XML before treating it as unregistered.

### A.5 Follow-up tasks

| Task | Finding | Scope |
|---|---|---|
| **A-1** Register + type `unlay` | B4, B8 | Folded into **P0-2** |
| **A-2** Register + type `poisondamagecycles` | A.1 | Conditions area — same fix shape as A-1 |
| **A-3** Port `FORCEUSE` into the `CheckTopUseObject` walk, or drop the attribute | A.1 | Use-path audit |
| **A-4** Triage `replacemagicfields` / `specialfieldblockpath` / `blockpathfind`: port or remove | A.1 | moveuse / walk audits |
| **A-5** Comment the `field.*` composite parse to record that magnitudes come from the profile | A.2 | Trivial |
