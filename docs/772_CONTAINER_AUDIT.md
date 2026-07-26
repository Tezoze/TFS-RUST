# 772 Container System — Rust vs Decompile Parity Audit

**Audited:** 2026-07-26
**Reference (772 mechanics):** `reference/cipsoft-772/tibia-game-master/src/` — `operate.cc`,
`cract.cc`, `moveuse.cc`, `info.cc`, `map.cc`, `crplayer.cc`, `receiving.cc`, `sending.cc`.
**Reference (772 wire, authoritative):** `reference/tvp-772/gameserver/src/` — `protocolgame.cpp`,
`networkmessage.cpp`, `player.cpp`, `game.cpp`.

**Rust files audited:**

| Rust file | C++ counterpart |
|---|---|
| `crates/tfs-rust-core/src/container.rs` | `map.hh` `TObject`, `cr.hh:916` `OpenContainer[16]`, `crplayer.cc:791` `GetOpenContainer` |
| `crates/tfs-rust-core/src/container_ops.rs` | `operate.cc:606` `CheckContainerDestination`, `:621` `CheckContainerPlace`, `:646` `CheckDepotSpace`, `:1275` `Move`, `:1449` `Merge`, `map.cc:2017` `PlaceObject` |
| `crates/tfs-rust-core/src/container_ui.rs` | `operate.cc:128` `AnnounceChangedContainer`, `:1060` `CloseContainer`, `moveuse.cc:1536` `UseContainer`, `receiving.cc:609` `CUpContainer` |
| `crates/tfs-rust-core/src/game_world_item_cylinder.rs` | `info.cc:390` `GetContainer`, `:398` `GetObject`, `:321` `GetBodyContainer` |
| `crates/tfs-rust-core/src/game_world_item_move.rs` | `cract.cc:475` `TCreature::Move` (destination resolution + exchange) |
| `crates/tfs-rust-core/src/player/inventory/notifications.rs` | `operate.cc:1060` `CloseContainer(Obj, false)` |
| `crates/tfs-rust-net/src/codec/v772.rs` | `protocolgame.cpp:1326/1398/1871/1880/1890`, `sending.cc:696–798` |

**Related prior audits:** `docs/772_THROW_MOVE_AUDIT.md` (item relocation / `Move`+`Merge` chain),
`tasks/f8-decompile-parity-audit.md` (ToDo builder layer). This audit covers the **container
subsystem specifically**: storage model, capacity, open-window lifecycle, destination resolution,
and the `0x6E`–`0x72` UI packets.

---

## 0. Executive summary

The Rust container system is a faithful **TFS 1.x** `Container` port (cylinder `queryAdd` /
`queryMaxCount` / `queryDestination` / `addThing` / `removeThing`, `ContainerIterator`, depot chest
+ locker + inbox + store-inbox types, pagination). Per `TFS-Core`, that TFS-shaped domain is
correct and should be preserved. The gaps below are places where the **772 observable outcome**
diverges from that TFS shape and is not currently absorbed by `MechanicsProfile` / era gating.

**Findings: 6 bugs (2 critical), 9 parity gaps, 6 improvements.**

Highest-priority items:

| # | Severity | One-liner |
|---|---|---|
| B1 | **Critical** | `0x70` add-to-container carries no slot on 772; any non-front insert desyncs the client window |
| B2 | **Critical** | Auto-stack scans the whole container (TFS); 772 only merges into the **explicitly targeted slot** |
| B3 | High | Container still in reach after moving is **force-closed** instead of refreshed; dead range check |
| B4 | High | `has_parent` is false for ground/tile containers; 772 shows the up-arrow for them |
| B5 | Medium | Non-top tile items cannot be moved — `stack_pos`/sprite resolution ignores the object list walk |
| B6 | Medium | `INDEX_MOVE_UP` (z=254) with a body-slot parent silently re-adds to the same container |

---

## 1. Data model — MATCH (with one representational note)

**772:** containers are singly-linked lists of `TObject` (`map.hh:62`), with `Container` = parent
handle and `NextObject` = sibling. Index 0 is the list head (`map.cc:2298` `GetContainerObject`).

**Rust:** `Container.items: Vec<ItemId>` + `parent_container: Option<ItemId>`
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/container.rs" lines="128-157" />).

**Verdict: correct and better.** `Vec` + `SlotMap` keys give O(1) indexed access where the
decompile does an O(n) list walk, with identical observable ordering. This is exactly the
"idiomatic Rust behind a TFS-shaped domain" the repo mandates.

**Insertion order — MATCH.** `map.cc:2119` `MoveObject` → `PlaceObject(Obj, Con, false)` = insert
at **front** for non-map containers. Rust `container_add_thing` does `cont.items.insert(0, item_id)`
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/container_ops.rs" lines="703-707" />).
Correct — and it must stay push-front, see B1.

---

## 2. Bugs

### B1 — **Critical**: 772 `0x70` has no slot field; non-front inserts desync the window

772 `sendAddContainerItem` is `cid + item` with **no slot index** — the client unconditionally
inserts the new item at position 0 of the window (`protocolgame.cpp:1871`, `sending.cc:750`).
The Rust 772 codec correctly drops the slot:

```rust
// crates/tfs-rust-net/src/codec/v772.rs:265
pub fn encode_add_container_item(&self, cid: u8, _slot: u16, args: ItemTemplateArgs) -> NetworkMessage
```

But core emits `ContainerContentChange::Add { slot }` with `slot != 0` from
`container_insert_item_at`
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/container_ops.rs" lines="791-795" />),
and that helper is live in the inventory-exchange path:

```rust
// crates/tfs-rust-core/src/game_world_item_move.rs:401-411
let idx = self.get_thing_index_in_container(from_container, item_id)...;
self.container_detach_item(from_container, item_id)?;
self.unequip_item_from_inventory_slot(cid, slot, dest_id, ...)?;
self.container_insert_item_at(from_container, idx, dest_id)?;   // idx may be > 0
```

On 772 the client puts `dest_id` at slot 0 while the server has it at `idx` → every subsequent
click in that window addresses the wrong item (`z` = slot index in the move/use packet).

Worse, this also diverges from the decompile *behaviorally*: the 772 exchange path is
`::Move(this->ID, DestObj, ObjCon, -1, false, NONE)` (`cract.cc:610`), and `Move` → `MoveObject` →
`PlaceObject(..., Append=false)` = **push front**. There is no "restore to original index" in 772.

**Fix:** in the exchange path, push the swapped-out item to the front (index 0) rather than `idx`,
matching `cract.cc:610`. Either delete `container_insert_item_at` or restrict it to `index == 0`
and assert. If a non-front insert is genuinely needed for 1098, make the 772 codec fall back to a
full `0x6E` refresh when `slot != 0` instead of silently dropping the field.

### B2 — **Critical**: auto-stack scans the whole container; 772 merges only into the targeted slot

`container_query_destination` implements TFS `Container::queryDestination`: after failing the
index-targeted merge, it **scans every slot** for a mergeable partial stack:

```rust
// crates/tfs-rust-core/src/container_ops.rs:489-501
if auto_stack && stackable && source_parent_container != Some(container_item_id) {
    for (n, &list_item) in cont.items.iter().enumerate() {
        if list_item != item_id && self.items_stack_mergeable(item_id, list_item) && ...count < 100 {
            return Ok(ContainerDestResolution::StayHere { index: n as i32, dest_stack_item: Some(list_item) });
        }
    }
}
```

772 does **not** do this. `TCreature::Move` (`cract.cc:571-598`) merges only when the client
targeted a concrete slot (`DestZ < 254`) *and* the object at that slot is `CUMULATIVE` of the same
type:

```cpp
}else if(DestObjType.getFlag(CUMULATIVE) && DestObjType == ObjType){
    int DestAmount = (int)DestObj.getAttribute(AMOUNT);
    int MergeCount = MoveCount;
    if((DestAmount + MergeCount) > 100){ MergeCount = 100 - DestAmount; }
    if(MergeCount > 0){
        try{ ::Merge(this->ID, Obj, DestObj, MergeCount, NONE);
             MoveCount -= MergeCount;
             if(MoveCount <= 0){ return; }
        }catch(RESULT r){ if(r == TOOHEAVY){ throw; } }
    }
    DestObj = NONE;
}
```

The only other merge site is `operate.cc:1304`, gated on `ConType.isMapContainer()` — i.e. stacks
auto-merge **on the ground only**. Dropping 20 gold into a backpack that already holds a 50-gold
stack yields two stacks in 772, one stack in the Rust server.

Note also the **partial-merge remainder**: 772 merges `100 - DestAmount`, decrements `MoveCount`,
and then continues to `::Move` the remainder into the container as a separate object. Rust's
`resolve_move_destination` returns a single `to_merge_item` and has no remainder loop.

**Fix:** gate the container-wide scan behind `MechanicsProfile` (off for 772, on for 1098) — the
targeted-slot merge at lines 475-487 already matches 772 and should stay. Add the remainder
continuation so an over-100 merge splits correctly.

### B3 — High: reachable container is force-closed on move instead of refreshed; dead range check

772 `CloseContainer(Con, Force)` (`operate.cc:1060-1100`) **refreshes** the window when the
container is still accessible, and only closes when `Force` or `!ObjectAccessible(..., 1)`:

```cpp
if(Force || !ObjectAccessible(Player->ID, Con, 1)){
    Player->SetOpenContainer(ContainerNr, NONE);
    SendCloseContainer(Player->Connection, ContainerNr);
}else{
    SendContainer(Player->Connection, ContainerNr);   // refresh, keep open
}
```

Rust's post-remove notification closes in **both** branches — the range test is dead code:

```rust
// crates/tfs-rust-core/src/player/inventory/notifications.rs:247-253
if let Some(cpos) = self.container_item_position(item_id) {
    if !Self::positions_in_range_1(player_pos, cpos) {
        self.auto_close_containers_for_container_item(cid, item_id);
        return;
    }
}
self.auto_close_containers_for_container_item(cid, item_id);
```

Effect: moving a bag from your backpack to an adjacent tile closes its window, where 772 keeps it
open and re-sends `0x6E`. Same for `decay_apply.rs:314` / `:405`.

This also violates `TFS-code-hygiene` ("remove dead code in the same PR").

**Fix:** replace the second call with `refresh_container_ui_for_all_viewers(item_id)` and collapse
the branch, so the shape mirrors `CloseContainer(Con, Force=false)`.

### B4 — High: `has_parent` is false for tile/ground containers

772 (`sending.cc:714`):

```cpp
bool HasUpContainer = (Con.getContainer() != NONE
        && !Con.getContainer().getObjectType().isBodyContainer());
```

Only a **body** container suppresses the up-arrow. A bag lying on the ground has a *map* container
as parent → `HasUpContainer = 1`, and pressing up runs `CUpContainer` (`receiving.cc:609`) which
sees a map-container parent and **closes** the window.

Rust: `has_parent: cont.parent_container.is_some()`
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/container_ui.rs" lines="324-327" />)
— `parent_container` is `None` for a tile container, so the arrow never appears.
`player_up_container` likewise early-returns when there is no parent
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/container_ui.rs" lines="766-772" />)
instead of closing the window.

**Fix:** compute `has_parent` as `parent_container.is_some() || container is on a map tile`, and
make `player_up_container` send `0x6F` when the resolved parent is a tile/inventory root.

### B5 — Medium: non-top tile items cannot be moved

`internal_get_thing_move` for map positions returns only the **top** down item and ignores
`stack_pos`
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/game_world_item_cylinder.rs" lines="56-72" />);
`player_move_item` then rejects the request when the sprite does not match.

772 `CMoveObject` forces `RNum = 1` for map coordinates (`receiving.cc:275`) and `GetObject`
(`info.cc:398-432`) **walks the tile object list matching the client `TypeID`**:

```cpp
}else if(RNum != -1){
    Obj = GetFirstObject(x, y, z);
    while(Obj != NONE){
        if(Obj.getObjectType().getDisguise() == Type){ break; }
        Obj = Obj.getNextObject();
    }
}
```

So in 772 you can move a buried item by sprite; in Rust it fails with `NotPossible`.

**Fix:** on sprite mismatch, walk the tile stack for the first item whose client id equals
`sprite_id` (the helper `find_tile_item_by_client_sprite` already exists in `container_ui.rs:504`
but is not wired into the move path).

### B6 — Medium: `INDEX_MOVE_UP` with a body-slot parent re-adds to the same container

772 (`cract.cc:563-566`): `DestZ == 254` sets `DestCon = DestCon.getContainer()`, then throws
`NOTACCESSIBLE` if that is `NONE`. For a bag worn in a body slot, the parent is the **body
container** — the item gets equipped.

Rust returns `StayHere { index: INDEX_WHEREEVER }` when there is no `parent_container`
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/container_ops.rs" lines="429-441" />),
i.e. the item is silently dropped back into the container the player was trying to move it *out* of.

**Fix:** when `parent_container` is `None`, redirect to the owning inventory slot cylinder (or the
tile cylinder for a ground container) instead of `StayHere`.

---

## 3. Parity gaps

### G1 — No `CHEST` flag (unlimited capacity)

`operate.cc:612` / `:625` skip the capacity check entirely when the destination has the `CHEST`
flag. Rust has no equivalent: `Container::is_full()` always compares against `capacity`
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/container.rs" lines="228-230" />).
Depot lockers, quest chests and other `CHEST`-flagged objects will reject items at capacity where
772 accepts them (depot volume is bounded separately by `DepotSpace`, see G2).

### G2 — Depot accounting model differs

| Aspect | 772 | Rust |
|---|---|---|
| Limit source | `TDepotInfo.Size` **per town** (`map.cc:2514`) | global `config.depot_premium_limit()` / `max_depot_items` (`player/depot.rs:19-30`) |
| Premium | `Size *= 2` (`map.cc:2545`) | separate premium/non-premium config values |
| Tracking | `Player->DepotSpace` decremented live (`moveuse.cc:620`) | recursive `total_item_count` recomputed per query |
| Check scope | only when source is outside and destination inside the depot (`operate.cc:646` `CheckDepotSpace`) | `container_query_add` `Depot` branch, any add |
| Lifetime | loaded from DB on use, objects **deleted from memory** on save (`moveuse.cc:658`) | resident containers |

The Rust model is TFS-shaped (correct per the mandate), but the **per-town size** and **×2 premium**
numbers are 772 outcomes and belong in `MechanicsProfile` / `772.lua`, not a global config key.
Also verify that moving items *within* the depot does not consume space (772 explicitly allows it).

### G3 — Container-window allocation: client-chosen vs server-allocated

772 `UseContainer(CreatureID, Con, NextContainerNr)` (`moveuse.cc:1536`) uses the **client-supplied**
`NextContainerNr` verbatim — there is no free-slot search, and reusing an occupied number silently
replaces that window. Rust's `add_container` falls back to `alloc_free_cid` and returns `None` when
all 16 are taken
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/container.rs" lines="494-502" />),
sending a cancel message 772 would never send. The toggle-on-reopen behavior *does* match
(`try_open_container_for_item` closes an already-open container).

Confirm the `UseItem` `index` byte is threaded into `preferred_cid` for the 772 path; if it is,
prefer honouring it unconditionally (clamped to 0–15) over the free-slot search.

### G4 — Notification spectator radius

`AnnounceChangedContainer` (`operate.cc:128`) searches `TFindCreatures Search(1, 1, Obj, FIND_PLAYERS)`
— only players **within 1 tile of the container object** *and* holding it open receive
`0x70`/`0x71`/`0x72`. Rust notifies every entry in `Container.open_by` regardless of distance
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/container_ui.rs" lines="414-436" />).
Mostly masked by auto-close, but observable for a shared ground container immediately after one
viewer steps away and before the auto-close sweep runs.

### G5 — `MAX_OBJECTS_PER_CONTAINER = 36`

The decompile hard-caps the open-container packet at 36 items (`sending.cc:12`, `:717`) and
`SendChangeInContainer` / `SendDeleteInContainer` **suppress the packet entirely** when
`ObjIndex >= 36` (`sending.cc:772`, `:792`). tvp-772 uses `min(capacity, size, 255)`
(`protocolgame.cpp:1337`); the Rust codec matches tvp
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-net/src/codec/v772.rs" lines="403-407" />).
Harmless for ≤20-slot bags, but a depot chest or browse-field with >36 slots will send updates the
772 client cannot address. Consider a `MAX_OBJECTS_PER_CONTAINER` clamp in the 772 codec so
`0x71`/`0x72` are dropped for `slot >= 36` rather than truncated by `slot as u8`.

### G6 — Pagination / seek is dead weight on 772

`pagination`, `total_size`, `first_index`, `player_seek_in_container` and
`Container::get_page` exist for the 1098 `0x6E` layout. The 772 packet has no such fields, and the
772 client never sends a seek. Currently benign only because `first_index` is always 0 on the 772
path — but `build_container_open_packet` will happily skip `first` items with no way to tell the
client (<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/container_ui.rs" lines="329-339" />).
Guard `player_seek_in_container` on the active codec, or force `first_index = 0` for 772.

### G7 — Truncated slot index in `0x71`/`0x72`

`encode_update_container_item` / `encode_remove_container_item` do `slot as u8`
(`v772.rs:288`, `:299`). This matches tvp-772's `static_cast<uint8_t>(slot)`, so it is *wire*-correct,
but a `slot > 255` silently wraps. Since 772 containers never exceed 36 usable slots (G5), clamp or
drop instead of wrapping.

### G8 — Exchange is proactive, not error-triggered

772 attempts an inventory exchange **only** after `::Move` throws `NOROOM`, `HANDSNOTFREE`,
`HANDBLOCKED` or `ONEWEAPONONLY` (`cract.cc:606-620`). Rust exchanges whenever the target slot is
occupied (`game_world_item_move.rs:397`), which will swap in cases where 772 would return an error
message to the client.

### G9 — No nesting-depth guard, no `CROSSREFERENCE` distinction

772 has no depth limit; cycles are prevented purely by `IsHeldByContainer` → `CROSSREFERENCE`
(`operate.cc:606`, `info.cc:499`). Rust's cycle prevention is equivalent (`container_query_add`
walks `parent_container` and returns `ThisIsImpossible`,
<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/container_ops.rs" lines="247-277" />)
— **this is a match**. But `resolve_move_destination` additionally caps redirection at 16 hops
(`floor_n >= 16`) and then silently `break`s with whatever cylinder it last held, rather than
returning an error. Prefer an explicit `ReturnValue::NotPossible` on exhaustion.

---

## 4. Confirmed matches (no action)

| Area | Evidence |
|---|---|
| Insert-at-front ordering | `map.cc:2131` `PlaceObject(..., false)` ↔ `container_ops.rs:706` `items.insert(0, ..)` |
| Max stack = 100 | `operate.cc:1470` ↔ `container_ops.rs:371,391,479,493` |
| Cycle prevention | `info.cc:499` `IsHeldByContainer` ↔ `container_ops.rs:249-262` parent walk |
| Recursive weight | `info.cc:136` `GetCompleteWeight` ↔ `refresh_container_chain` / `item_recursive_weight_oz` (cached — an *improvement*, see I1) |
| Skip weight check for already-carried items | `operate.cc:797` `GetObjectCreatureID(Obj) == CreatureID` ↔ `player_has_capacity` `player_carries_item` early-return (`query_add.rs:325`) |
| 16 container windows | `cr.hh:916` ↔ `MAX_CONTAINER_WINDOWS = 16` |
| Position encoding `x=0xFFFF`, `y=0x40\|cid`, `z=slot` | `game.cpp:312`, `enums.hh:307` ↔ `internal_get_cylinder` / `resolve_item_at_position` |
| Toggle-close on re-use | `moveuse.cc:1563` ↔ `try_open_container_for_item` (`container_ui.rs:721-727`) |
| `hasParent` false for body-slot containers | `sending.cc:714` ↔ `parent_container.is_some()` |
| Opcodes `0x6E`–`0x72` + field order | `protocolgame.cpp:1326/1398/1871/1880/1890` ↔ `v772.rs:265-409` |
| Item body serialization (id, then count for stackable / colour for fluid) | `networkmessage.cpp:95`, `sending.cc:171` ↔ `write_item_template_args` |
| `CloseAllContainers` on logout | `cr.hh:844` ↔ `close_all_for_player` |
| Container detach on item destruction closes windows | `operate.cc:1060` Force path ↔ `ContainerRegistry::remove` returning closed `(player, cid)` pairs |

---

## 5. Improvements (non-parity)

### I1 — Cached derived state is good; the invalidation is O(depth × subtree)

`refresh_container_derived` recomputes `total_weight` **and** runs a full `ContainerIterator` count
per level, and `refresh_container_chain` calls it for every ancestor
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/container_ops.rs" lines="82-110" />).
For a 4-deep nest that is a quadratic re-walk on every single add/remove. The delta helper
`apply_container_remove_delta_chain` already exists and is used only by the expiry path — extend it
to the add/remove paths so the common case is O(depth).

### I2 — `Container::add_item` / `remove_item` / `insert_item` are structural-only footguns

They mutate `items` without touching `total_weight` / `total_item_count` / `parent_container` /
`Item.parent`, and they *are* reachable from tests and `player/depot.rs`. Either make them private
to the module or rename to `*_raw` with a doc comment naming the paired `GameWorld` helper — per
`TFS-code-hygiene`, "name helpers for their contract".

### I3 — `refresh_container_ui_for_all_viewers` used as an error fallback

`enqueue_container_add_or_update_slot` falls back to a **full refresh for all viewers** whenever the
slot / item / client id lookup fails
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/container_ui.rs" lines="150-167" />).
Three separate `return` paths do this inside a per-viewer loop, so N viewers → N full refreshes each
containing N packets. Hoist the lookup above the viewer loop and refresh once.

### I4 — `ContainerType::BrowseField` / `Inbox` / `StoreInbox` are 1098-only concepts on a 772 target

`new_browse_field`, `pagination = true`, and the store-item checks in `container_query_add`
(lines 222-245) have no 772 analogue. They are correct for the 1098 profile; add a `//!` note so a
future reader does not mistake them for 772 behavior, and confirm they are unreachable when
`clientVersion = 772`.

### I5 — `open_by: Vec<CreatureId>` is O(n) on every viewer op

`add_viewer` / `remove_viewer` / `is_viewer` are linear scans, and `notify_container_content_changed`
clones the whole vec per mutation. Viewer counts are small (≤ a handful), so this is fine today —
but the clone-per-mutation is avoidable with `SmallVec` or by iterating indices.

### I6 — Missing C++ reference headers

`container.rs:2` cites only `src/container.h` / `container.cpp` (TFS 1.4.2). Per `TFS-cpp-references`,
files with 772-relevant outcomes must cite both layers, e.g.:

```rust
//! Containers — TFS-style cylinder domain, idiomatic Rust.
//! Domain: `container.h` / `container.cpp` (TFS 1.4.2) — data-pack contract.
//! 772 outcomes: `map.cc` `PlaceObject`, `operate.cc` `CheckContainerDestination` /
//!               `AnnounceChangedContainer`, `cract.cc` `TCreature::Move` destination resolution.
```

Same for `container_ops.rs` and `container_ui.rs`.

---

## 6. Suggested verification

```bash
rtk cargo check -p tfs-rust-core -p tfs-rust-net
rtk cargo clippy -p tfs-rust-core -p tfs-rust-net -- -D warnings
rtk cargo test -p tfs-rust-core container
rtk cargo test -p tfs-rust-net container_open_772_layout
```

## 7. Suggested tests to add

| Test | Asserts |
|---|---|
| `add_to_container_always_reports_slot_zero_on_772` | B1 — every `Add` change emitted by a move into a container has `slot == 0` |
| `inventory_exchange_pushes_swapped_item_to_front` | B1 — `cract.cc:610` push-front semantics |
| `stack_does_not_autostack_across_container_slots_on_772` | B2 — dropping 20 gold into a bag holding 50 gold yields two stacks under the 772 profile |
| `targeted_slot_merge_splits_remainder_over_100` | B2 — 80 + 40 → 100 in the slot, 20 as a new front entry |
| `moving_open_container_within_reach_refreshes_not_closes` | B3 — expects `0x6E`, not `0x6F` |
| `ground_container_reports_has_parent` | B4 — `0x6E` `hasParent == 1` for a tile bag |
| `up_container_on_ground_bag_closes_window` | B4 — `CUpContainer` map-parent branch |
| `move_non_top_tile_item_by_sprite` | B5 |
| `move_up_from_body_slot_container_equips` | B6 |
| `chest_flag_container_ignores_capacity` | G1 |
