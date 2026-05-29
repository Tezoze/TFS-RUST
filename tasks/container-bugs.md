# Container Bug Investigation

## Issue 1 — Containers display pagination (page buttons) when they shouldn't

### Symptom
Opening a normal player container (e.g. a 20-slot backpack) renders with pagination buttons (forward/back arrows), similar to a Browse Field or mailbox, instead of showing all items at once.

### Root Cause — Two defects in `build_container_open_packet`
**File:** `crates/tfs-rust-core/src/container_ui.rs` — `build_container_open_packet()`

#### Defect A — Pagination flag (`container_ui.rs:259`)
```rust
let pagination = cont.pagination || total_items > u16::from(CONTAINER_VISIBLE_SLOTS);
```
`CONTAINER_VISIBLE_SLOTS` is hard-coded to **12**. Any container with more than 12 items gets `pagination = true`, even a normal 20-slot backpack that should never paginate.

**C++ reference** (`protocolgame.cpp:1771`):
```cpp
msg.addByte(container->hasPagination() ? 0x01 : 0x00);
```
C++ sends **only** `container->hasPagination()` — the stored field, which is `false` for all normal containers. It only becomes `true` for Browse Field containers. There is no item-count threshold.

**Fix:** Change to `let pagination = cont.pagination;` — mirror the C++ behavior exactly.

#### Defect B — Items-per-page cap (`container_ui.rs:262`)
```rust
let n_show = remain.min(u16::from(CONTAINER_VISIBLE_SLOTS)) as u8;
```
Items shown per page are capped at 12 regardless of container capacity.

**C++ reference** (`protocolgame.cpp:1777`):
```cpp
uint8_t itemsToSend = std::min<uint32_t>(
    std::min<uint32_t>(container->capacity(), containerSize - firstIndex),
    std::numeric_limits<uint8_t>::max());
```
C++ uses `capacity()` (e.g. 20 for a backpack) as the upper bound, not 12.

**Fix:** Change to `let n_show = remain.min(u16::from(capacity)) as u8;` — use the container's actual capacity.

---

## Issue 2 — "You need to exchange items." when moving items into player containers

### Symptom
Dragging an item into an open container fails with the cancel message *"You need to exchange items."* (`ReturnValue::NeedExchange`). This happens when the item is dropped onto any occupied slot in the container window.

### Root Cause — Spurious `NeedExchange` check in `container_query_add`
**File:** `crates/tfs-rust-core/src/container_ops.rs:207-218`

```rust
if index != INDEX_WHEREEVER && index >= 0 {
    let idx = index as usize;
    if let Some(dest_id) = cont.get_item(idx) {
        let stackable = it.map(|t| t.stackable()).unwrap_or(false);
        if !(stackable
            && self.items_stack_mergeable(item_id, dest_id)
            && self.items.get(dest_id).is_some_and(|d| d.count < 100))
        {
            return ReturnValue::NeedExchange;
        }
    }
}
```

This block returns `NeedExchange` whenever a non-stackable item is being added to a specific container slot that already contains an item. **This check does not exist anywhere in the C++ `Container::queryAdd`.**

**C++ reference** (`container.cpp:265-349`):
C++ `Container::queryAdd` checks: `unlocked`, `pickupable`, self-reference, store-item rules, parent-chain cycles, and — **only** when `index == INDEX_WHEREEVER` — capacity (`size() >= capacity()`). It never returns `RETURNVALUE_NEEDEXCHANGE`. That return value is exclusive to `Player::queryAdd` for equipment slot conflicts.

### Call path that triggers the bug
1. Player drags an item onto a visible slot in a container window.
2. Client sends `moveThing` with destination `(0xFFFF, 0x40|cid, slot_index)`.
3. `resolve_container_move_destination` → `container_query_destination` sees a non-container item at `slot_index`, returns `StayHere { index: slot_index, ... }`.
4. `internal_move_item` calls `container_query_add(cid, slot_index, ...)`.
5. The spurious block at line 207 matches (item exists at `slot_index`, not stackable) → returns `NeedExchange`.
6. `internal_move_item` returns `Err(NeedExchange)` and the cancel message is sent.

In C++, the same scenario: `queryDestination` sets `*destItem` but doesn't redirect. `queryAdd` is called with the specific index and **passes** (no `NeedExchange` check). The item is then inserted at position 0 (front) via `addThing(index, item)`, which always does `itemlist.push_front(item)` — the `index` parameter in C++ `addThing` is only bounds-checked against capacity, not used as the insertion point.

**Fix:** Remove the entire `NeedExchange` block (lines 207-218) from `container_query_add`. This check has no C++ equivalent in `Container::queryAdd`.

---

## Verified Feature — Auto-stacking stackable items into containers

### Status: **Implemented** (with gaps in two edge-case paths)

### How it works
When a stackable item (e.g. arrows, gold coins) is moved into a container that already holds a compatible stack with count < 100, the engine merges them instead of creating a separate slot.

#### Resolution path (`container_query_destination` — `container_ops.rs:394-427`)
Mirrors C++ `Container::queryDestination` (`container.cpp:487-502`):
1. If `auto_stack` is enabled, the item is stackable, and the source isn't the same container:
   - First checks the specific dest slot for a merge target.
   - Then scans all items in the container for a matching stack with `count < 100`.
2. Returns `dest_stack_item = Some(merge_id)` to the caller.

#### Merge execution in `internal_move_item` (`game_world.rs`)
| Source → Dest | Full-stack merge | Partial-stack merge |
|---|---|---|
| **Tile → Container** (line 887) | ✅ Merges via `to_merge_item` | ❌ Early-return at line 874 (`NotPossible`) — partial stack from tile to container not supported |
| **Container → Container** (line 995) | ✅ Merges via `to_merge_item` | ❌ Partial path (line 968) creates new item via `container_add_thing` — ignores `to_merge_item` |
| **Inventory → Container** (line 1107) | ❌ No `to_merge_item` handling — adds as separate stack | ❌ Early-return at line 1111 (`NotPossible`) — partial stack rejected |

### Notes
- The two most common player interactions (picking up a full stack from the ground, moving a full stack between bags) **do auto-stack correctly**.
- Partial-stack moves and inventory→container moves silently skip the merge and either error out or create a duplicate slot. The C++ `internalMoveItem` (`game.cpp:1208-1232`) handles all of these uniformly via a single `toItem` merge block after source removal, regardless of cylinder type.

---

## Interaction between the two bugs

Issue 1 makes normal containers show only 12 items with pagination, so every visible slot is occupied. Issue 2 means dropping an item on any occupied slot fails. Together, if a player has more than 12 items in a backpack, the only way to add items is to drop on the narrow empty space below items (which sends `INDEX_WHEREEVER = 255`), making item management nearly unusable.
