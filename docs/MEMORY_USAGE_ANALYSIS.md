# Memory usage analysis (Rust vs TFS C++)

This document explains why the TFS Rust server can hold **~15 GB RSS** after map load on a full Forgotten-style world, while the C++ implementations (both TFS 1.4.2 `src/` and TVP 7.72 `gameserver/src/`) typically show a **high peak during OTBM load** and a **lower steady state**.

The proposed fixes are driven by the project's `.agents/rules` (specifically `tfs-rust-idioms`, `tfs-entity-storage`, and `tfs-threading`), focusing on achieving the C++ *behavior* (lazy allocation, zero-cost abstractions) using optimal, safe Rust patterns rather than literal translation.

**Related:** `tasks/lessons.md` §32 (login delay from eager quadtree build).

**Implementation status (memory phases):**

| Phase | Goal | Status |
|-------|------|--------|
| **A** | Uniform lightweight `Item` (`Option<Box<ItemAttributes>>`, no `Item.id`) | **Done** — `crates/tfs-rust-core/src/item.rs` |
| **B** | Unified sparse grid (lazy leaves + tile storage) | Planned |
| **C** | Lua VM merge, spawn `zones`, creature bitflags, decay buckets, etc. | Planned |

> **Drift prevention:** every counter and claim cites `file:function:line` in the Rust source.
> If you refactor any cited function, **update this doc in the same change**. See
> [§ Source anchors](#source-anchors) at the bottom.

---

## Observed steady-state counts

After `startup_spawns()`, `run_server` logs entity counts via `tracing::info!` at
`crates/tfs-rust-core/src/run_server.rs:237–244`:

```rust
// run_server.rs:237–244 — THE authoritative log line for this section
info!(
    map_tiles = world.map.tiles.len(),
    items_slotmap = world.items.len(),
    tile_stack_item_refs = tile_stack_refs,
    creatures_slotmap = world.creatures.len(),
    spawn_slots = world.spawns.slots.len(),
    "GameWorld ready — steady-state entity counts …"
);
```

| Counter | Example value | Code expression | Meaning |
|--------|---------------|-----------------|---------|
| `map_tiles` | 7,846,041 | `world.map.tiles.len()` | `HashMap<Position, Tile>` entries — one per OTBM tile |
| `items_slotmap` | 750,427 | `world.items.len()` | Runtime `Item` in `GameWorld::items` per non-ground stack entry (~16-byte body + SlotMap overhead after Phase A) |
| `tile_stack_item_refs` | 750,427 | Σ `(b.down_items.len() + b.top_items.len())` | `ItemId` references on tile stacks (matches slotmap count for static map items) |
| `creatures_slotmap` | 23,656 | `world.creatures.len()` | Creatures placed at startup (`Spawns::startup` parity) |
| `spawn_slots` | 23,705 | `world.spawns.slots.len()` | Spawn slot metadata kept for respawn scheduling |

**Not yet logged** (recommended — see [§ Recommended fixes](#recommended-fixes-priority) item 7):
`world.map.width`, `world.map.height`, `world.map.qtrees.len()`.

---

## Why C++ drops after map load but Rust stays high

| Phase | TFS C++ (`src/map.cpp`, `src/iomap.cpp`) | TFS Rust (current) |
|-------|------------------------------------------|---------------------:|
| OTBM parse | Streams tiles; temporary buffers freed when done | Full file `read` + `MapData { tiles: HashMap<…> }`, then conversion to runtime `Map` |
| Spatial index | **Lazy** `QTreeNode::createLeaf` only where tiles exist | **Eager** `QTreeNode::build(0, 0, width−1, height−1)` over OTBM header bounds |
| Map items | `Item*` on tile; type metadata in `ItemType` table | **Every** non-ground stack entry → `SlotMap<Item>` slot; **16-byte** `Item` body, `attributes: None` for static map stacks (Phase A) |
| After startup | Tiles + sparse quadtree + creatures | Tiles + **multi-GB quadtrees** + ~750k lightweight items + ~24k creatures |

C++ releases parse temporaries **and** never allocates a spatial index sized for the full 65535×65535 OTBM bounding box. Rust releases `MapData` after `Map::from_map_data`, but keeps structures that C++ does not build at the same scale.

---

## Root causes (by impact)

### 1. Eager full-map quadtree (largest gap — multi-GB)

**C++** (`src/map.h`, `map.cpp`): the spectator quadtree grows only when `setTile` calls `createLeaf` for a 32×32 leaf region. Empty map area costs almost nothing.

**Rust** (`crates/tfs-rust-core/src/map/qtree.rs`, `map/mod.rs`):

- `QTreeNode::build(x0, y0, x1, y1)` (`qtree.rs:35`) recursively subdivides until leaves are ≤32×32, materializing a **complete** tree for the given rectangle.
- On map load, **floor 0 always** gets a full tree from OTBM `width` / `height`:

```rust
// map/mod.rs:47–55 — Map::from_map_data
qtrees.insert(
    0,
    QTreeNode::build(0, 0, data.width.saturating_sub(1), data.height.saturating_sub(1)),
);
```

- When a creature registers on floor `z`, `register_creature_index` (`map/mod.rs:300–307`) builds another full tree for that floor if missing:

```rust
// map/mod.rs:303–306
self.qtrees
    .entry(pos.z)
    .or_insert_with(|| QTreeNode::build(0, 0, w, h));
```

Forgotten OTBMs often declare **~65535×65535** in the header while only ~7.8M tiles are populated. For that box:

- Leaf count ≈ `(65535/32)²` ≈ **4.2M leaves per floor**
- Each leaf: `QTreeNode::Leaf { x0, y0, x1, y1, creatures: Vec, cached_spectators: Option<Vec> }` — at minimum ~72 bytes
- Plus branch nodes and `Box` indirection (~48 bytes each)

**Order-of-magnitude:** hundreds of MB to **multiple GB per floor**. Startup spawns creatures on many `z` levels (0–15), so **many full trees** can exist at once.

**Partial mitigation already in place:** `collect_spatial_spectators` no longer calls `qtree_mut` (which would build trees on first spectator query). See `tasks/lessons.md` §32. **Map load and `register_creature_index` still build full trees.**

**Additional waste:** `cached_spectators: Option<Vec<CreatureId>>` clones the `creatures` vec on first query (`qtree.rs:244`), so every cached leaf holds **two copies** of its creature list. C++ `SpectatorCache` is a **map at `Map` level** cleared on tile/creature changes (`map.cpp` `clearSpectatorCache`), not a per-leaf copy.

---

### 2. Map stack items in `SlotMap<Item>` — **Phase A done** (~42 MB steady; was ~187 MB)

**C++:** tile items are pointers; static map items are mostly type id + count on the stack.

**Rust (Phase A, current)** — `map/mod.rs:136–138`, `internal_add_item_id`:

```rust
let item = Item::new_single(id);
let item_id = items.insert(item);
```

Map load creates items with `attributes: None`. Accessors on `Item` (`item.rs:52–129`) default to empty/zero when `None`, matching C++ getters on an item with no attribute blob.

**`Item`** (`item.rs:25–33`) — inline body only:

| Field | Size (bytes) | Notes |
|-------|-------------|-------|
| `item_type: u16` | 2 | Client-visible type id |
| `count: u16` | 2 | Stack count |
| `attributes: Option<Box<ItemAttributes>>` | 8 | `None` for ~99%+ of map items; heap only on mutation or DB blob |
| Padding | 4 | Align to 16 bytes on 64-bit |
| **Total per `Item` (no attrs)** | **16** | `std::mem::size_of::<Item>()` on 64-bit |

**`ItemAttributes`** (`item_attributes.rs:130–168`) — allocated **only when needed** (~192–208 bytes on heap):

| Field cluster | Size (bytes) |
|---|---|
| `attribute_bits` (u32 bitflags) | 4 |
| 16 scalar numerics (`action_id` through `depot_id`) | ~82 |
| 6 `Option<String>` (24 bytes each on 64-bit) | 144 |
| `Option<Box<CustomAttributeMap>>` | 8 |
| Alignment padding | ~10–14 |
| **Total (when boxed)** | **~192–208** |

With **~750k** map items (mostly `attributes: None`):
- Item bodies: ~750k × 16 bytes ≈ **~12 MB**
- SlotMap slot overhead (~40 bytes each): **~30 MB** (unchanged)
- Attribute boxes: negligible at map load (player/depot items pay per-instance later)
- **Total: ~42 MB** (down from ~187 MB pre–Phase A)

**Remaining gap vs C++:** we still allocate one `SlotMap` slot + 16-byte node per stack entry; C++ uses a pointer without a redundant key. Phase B does not change the item model.

---

### 3. ~7.8M tiles in `HashMap<Position, Tile>` (~0.9–1.5 GB)

This is **expected** for a dense Forgotten map: Rust keeps one `Tile` per OTBM tile in `Map::tiles` (`map/mod.rs:29`).

**`TileBody`** (`tile.rs:53–63`) layout:

| Field | Size (bytes) | Notes |
|-------|-------------|-------|
| `position: Position` | 8 (5 + padding) | **Redundant** — also stored as `HashMap` key |
| `ground: Option<u16>` | 4 | |
| `down_items: Vec<ItemId>` | 24 | Heap pointer + len + cap |
| `top_items: Vec<ItemId>` | 24 | |
| `creatures: Vec<CreatureId>` | 24 | |
| `flags: u32` | 4 | |
| `zone: ZoneType` | 1 + padding | |
| **Total TileBody** | **~92** | Before heap allocations |

`HashMap` overhead per entry: ~64 bytes (key + value + hash + bucket metadata).

**Ballpark:** 7.8M × ~156 bytes (TileBody + HashMap overhead) → **~1.2 GB**. This is similar in order to C++.

**Waste within this expected cost:**
- **Duplicate `Position`**: stored as HashMap key *and* inside `TileBody.position` — ~7.8M × 8 bytes = **~62 MB** pure redundancy.
- **Three `Vec` headers for mostly-empty vecs**: most tiles have 0 creatures and 0–3 items. Each `Vec` is 24 bytes even when empty. **~7.8M × 72 bytes = ~562 MB** in Vec headers alone, many pointing to empty heap allocations.
- **SipHash overhead**: `Position` derives `Hash`, using the default SipHash hasher. For 7.8M entries, a cheaper hash (FxHash / AHash) would reduce bucket collisions and memory.

---

## Architectural Recommendations (The "Better Ways")

To answer how we do this without breaking the outcome and minimizing code churn, here is the recommended path forward:

1. **Don't use a two-tier item system.** It fractures the `ItemId` API and makes every container/inventory interaction complex. **Phase A** applied **uniform lightweight items**: no `Item.id`, `Option<Box<ItemAttributes>>`, 16-byte bodies — same `SlotMap<ItemId, Item>` API, ~95% of the per-item memory win without splitting static vs live items.
2. **Don't globally adopt `FxHashMap`.** Instead, eliminate the tile `HashMap` entirely by merging tiles into the sparse quadtree leaves (Priority 1). This perfectly matches the C++ `QTreeLeafNode` architecture (which stores a `Tile*[1024]` array for its 32x32 grid). This provides DoS-resistant O(1) array lookups for tiles without a global hash table.
3. **Execution order:** 
   - **Phase A — done:** **Uniform lightweight items** — `item.rs` (+ call sites); removed `Item.id` and boxed attributes. ~145 MB saved on Forgotten-scale maps.
   - **Phase B — next:** **Unified sparse grid** — replaces eager `QTreeNode` and `HashMap<Position, Tile>`; fixes multi-GB load and tile-hash waste.
   - **Phase C:** Smaller debt (Lua VM merge, spawn `zones`, creature bitflags, decay buckets, load-path fixes).

### Priority 1 — The Unified Sparse Grid (Multi-GB savings)

**Current:** A `HashMap<Position, Tile>` (7.8M entries) AND a separate eager `QTreeNode` recursive tree.
**The "Better Way" (Outcome Parity with C++):** C++ doesn't use a hash map for tiles; it stores a `Tile*[1024]` array inside each 32x32 quadtree leaf. We can do the same in Rust to eliminate the 7.8M hash entries, SipHash overhead, and the eager multi-GB tree all at once.

```rust
/// Key is (x / 32, y / 32, z)
pub struct MapGrid {
    leaves: FxHashMap<(u16, u16, u8), Box<Leaf>>,
}

pub struct Leaf {
    /// Flat array for O(1) index: (x % 32) + (y % 32) * 32
    pub tiles: [Option<TileBody>; 1024],
    /// Spectators in this 32x32 area
    pub creatures: SmallVec<[CreatureId; 4]>,
}
```

- **Why it's better:** It merges Priority 1a (Quadtree) and Priority 2a/2b (HashMap tiles) into a single, elegant data structure. It provides O(1) tile access (array indexing) without 7.8M heap allocations for hash nodes. It exactly mirrors the C++ spatial layout memory profile while remaining 100% safe Rust.

### Priority 2 — Uniform lightweight items — **implemented (Phase A)**

**Was:** `Item` ~216 bytes inline (`Item.id` + embedded `ItemAttributes`).

**Now** (`item.rs:25–33`):

```rust
pub struct Item {
    pub item_type: u16,
    pub count: u16,
    pub attributes: Option<Box<ItemAttributes>>,
}
```

- **Outcome:** 16-byte `Item` bodies at map load; `ItemAttributes` heap-allocated on first mutation or DB deserialize (`from_player_item_record`, `item_blob::parse_item_blob`).
- **API:** `ItemId` comes only from `SlotMap` keys; convenience getters/setters on `Item` hide the `Option`.
- **Not changed:** `ItemAttributes` layout in `item_attributes.rs` (still ~192–208 bytes when boxed).

---

## Load-time spike (both engines; Rust drops less)

During `tfs_rust_content::pipeline::load_all` and `Map::from_map_data`:

1. Entire OTBM file in memory (`std::fs::read` in `crates/tfs-rust-content/src/otbm.rs`).
2. `MapData.tiles: HashMap<Position, TileData>` (duplicate representation).
3. Conversion to runtime tiles + `SlotMap` items.
4. Full z=0 quadtree build.

After conversion, `MapData` is dropped, but quadtree + item slotmap remain. **glibc** (default allocator) also often **does not return** freed peak memory to the OS; both C++ and Rust can look "stuck" after a spike, but Rust **steady** retention is higher here.

**Additional load-time cost:** `tile_from_data` (`map/mod.rs:266,282`) calls `otbm::item_id_from_otbm_item_props(&raw)` **twice** per `ItemNodeProps` thing — once in the match guard and again in the arm body. The parse is redundant.

**Another load-time cost:** `Tile::add_item` (`tile.rs:126`) does `down_items.insert(0, item_id)` which is O(n) memmove per item during OTBM load. With ~750k items across tiles, this causes ~750k unnecessary memmoves.

---

## Smaller contributors (tens of MB each)

| Source | Typical scale | Code location |
|--------|---------------|---------------|
| **Two Lua VMs at boot** | ~10–80 MB (doubles VM overhead) | `run_server.rs:163` (`LuaRuntime::new`) + `formulas.rs:464` (`Lua::new`) |
| **Duplicate spawn XML** | ~few–20 MB | `spawn.rs:48` (`zones: Vec<SpawnZone>`) — never read after `from_zones` |
| **`ItemDatabase` clone at load** | Transient tens of MB | `pipeline.rs:63` (`items.clone()`) |
| **Creature name per instance** | ~1–5 MB at ~24k spawns | `CreatureBase::name: String` cloned from `MonsterType` |
| **`Monster::registered_events: HashSet<String>`** | ~1.1 MB (48-byte empty HashSet × 24k) | `monster.rs:72` — only ever checks `"onThink"` |
| **`CreatureBase::damage_map: HashMap`** | ~1.1 MB (48-byte empty HashMap × 24k) | `base.rs:115` — empty at spawn |
| **Fat `ItemType` rows** | ~20–80 MB | `Arc<ItemDatabase>` with `HashMap` per type |
| **Fat `MonsterType` rows** | ~10–50 MB | Spells as `Vec<MonsterSpellNode>` with `HashMap<String, String>` per node |
| **`CreatureKind` enum variant padding** | ~5–10 MB | `kind.rs:15` — `Player` variant inflates all slots |
| `Arc<ItemDatabase>` / `Arc<MonsterDatabase>` | Tens–low hundreds of MB | XML strings, spell nodes, loot tables |
| ~24k startup creatures | Tens of MB | `CreatureBase` + `Monster` per slot |
| Lua (`mlua`) + loaded scripts | Tens of MB | Unless very large script sets |
| Tokio / DB pool | Small | Default pool sizes |

---

## Detail: two Lua VMs

```text
run_server.rs
  ├─ LuaRuntime (creaturescripts, movements, player events)  → LuaEventDispatcher
  └─ formulas.rs:464 load_mechanics → FormulaHooks { _lua: Some(Lua), … }  → second VM
```

`FormulaHooks` **must** keep `_lua` alive so cached `mlua::Function` handles stay valid (`formulas.rs`). Merging into one VM (formulas + events) would match C++ "one script world" and remove duplicate allocator/GC overhead.

---

## Detail: dead `SpawnManager::zones`

`SpawnManager::from_zones` (`spawn.rs:75`) moves the parsed `Vec<SpawnZone>` into `zones` and **also** copies every entry into `slots`. Grep shows **`zones` is never read** after construction — only `slots` drives respawn. C++ does not retain the entire parsed spawn document alongside per-slot runtime state.

**Fix:** drop `zones` after building slots, or store only `radius` on each slot (already done) and delete the field.

---

## Detail: quadtree leaf spectator cache doubles creature lists

Even with lazy allocation, `QTreeNode::Leaf` has `cached_spectators: Option<Vec<CreatureId>>` (`qtree.rs:30`). On cache fill (`qtree.rs:244`), `creatures.clone()` creates a second copy. C++ `SpectatorCache` is a **map at `Map` level** cleared on tile/creature changes (`map.cpp` `clearSpectatorCache`), not a per-leaf copy on a materialized full tree.

---

## Detail: duplicate `Position` on every tile

`HashMap<Position, Tile>` uses `Position` as the key while `TileBody` **also** stores `position` (`tile.rs:54`). ~7.8M redundant 6-byte keys + alignment → on the order of **~50–100 MB**. C++ addresses tiles via quadtree leaf + floor grid, not a hash key + stored coord.

---

## Detail: ground vs stack item model

Ground uses `Option<u16>` (efficient). Non-ground uses `SlotMap<Item>` with 16-byte nodes and lazy `Option<Box<ItemAttributes>>` (Phase A). C++ uses `Item*` for both but without a separate attribute blob per static piece. SlotMap slot overhead (~30 MB at 750k entries) remains. See §2.

---

## Detail: `Monster::registered_events` is a per-instance `HashSet<String>`

`monster.rs:72` allocates a `HashSet<String>` for each monster instance. The only key ever checked is `"onThink"` (`monster.rs:127`). A `HashSet` has minimum ~48 bytes overhead even when empty. With 24k monsters at startup, this is **~1.1 MB** of wasted hash table overhead. Should be a `u8` bitflags enum.

---

## Detail: `CreatureBase::damage_map` per idle creature

`base.rs:115` stores `damage_map: HashMap<CreatureId, u64>` on every creature. At spawn, it's empty but still allocates HashMap metadata (~48 bytes). With 24k creatures: **~1.1 MB**. Should be `Option<Box<HashMap<…>>>`, initialized `None` until first hit.

---

## Detail: `Item.id` redundant inside SlotMap — **fixed (Phase A)**

Removed from `Item`; `SlotMap<ItemId, Item>` is the sole owner of runtime ids. `from_player_item_record` still accepts an `ItemId` parameter for call-site compatibility but does not store it on the struct.

---

## Detail: `CreatureKind` enum variant size disparity

`kind.rs:15–19` defines `CreatureKind` as `Player(Player) | Monster(Monster) | Npc(Npc)`. `Player` is the largest variant (~400+ bytes), `Monster` is ~200 bytes, `Npc` is ~90 bytes. The enum is `#[allow(clippy::large_enum_variant)]`. Every `SlotMap` slot pays `sizeof(Player)` regardless of variant, wasting **~5–10 MB** for 24k mostly-Monster slots.

---

## Runtime patterns (not 15 GB at idle)

| Issue | Location | Notes |
|-------|----------|-------|
| **Broadcast clones packets** | `game_world.rs:208–213` `broadcast_to_spectators` | `packet.clone()` per spectator conn — correct for fan-out, can spike with many viewers |
| **`spectator_conns` linear scan** | `game_world.rs:199–204` | O(all_connections) + `Vec<ConnId>` alloc per broadcast event |
| **`find_item_position` full scan** | `map/mod.rs:77–84` | O(tiles) linear scan over **7.8M** tiles per call — CPU, not steady RAM, but catastrophic if hot |
| **`walk_grid_line` allocates Vec** | `map/los.rs:9` | `Vec<Position>` per LOS check — transient heap alloc on hot path |
| **Pathfinding `HashMap<Position, …>`** | `pathfinding.rs:116` | Per search, capped at `MAX_CLOSED_NODES = 100` — bounded; OK |
| **Monster AI list clones** | `monster_ai.rs` | `opponent_ids.clone()` / `friend_ids.clone()` on some paths — small per monster |
| **`known_creatures_by_conn`** | `game_world.rs:92` | Per-player `HashSet<u32>` — grows with play, expected |
| **`DecayManager` full-map scan** | `decay.rs:35–46` | `retain()` over entire HashMap every tick — should be time-bucketed |

---

## What is *not* a problem (confirmed)

- **23k startup creatures** — expected for Forgotten `Spawns::startup`; not a leak.
- **Spawn slot metadata** — required for respawn; size is small vs map.
- **Single-threaded `SlotMap`** — overhead is linear in entity count, not exponential.
- **DB pool** (`mysqlConnectionMaxPoolSize = 5`) — negligible vs map.
- **Creature / player `HashSet` known lists** — empty at boot.
- **`WildcardTree`** — only online player names; tiny at boot.
- **`HouseManager`** — empty until houses wired; tiny.
- **`StabilityManager`** — `DashMap` counters; negligible.
- **`pending_outgoing`** — empty when no players; drained each tick.
- **Outfits / mounts** from `Content` — dropped after `load_all`; not kept on `GameWorld`.
- **Pathfinding vs C++** — Rust uses `HashMap` for A* nodes; C++ uses fixed `AStarNodes nodes[512]` on stack — Rust is heavier **per path request**, not a 15 GB baseline.

---

## Memory model diagram

```text
GameWorld (steady state after load)
├── map.tiles: HashMap<Position, Tile>     ~7.8M entries  (~0.9–1.5 GB)
│   ├── Position key + TileBody.position   → ~62 MB redundant
│   └── 3× Vec<> headers per tile          → ~562 MB (most empty/tiny)
├── items: SlotMap<Item>                   ~750k entries  (~42 MB after Phase A)
│   ├── Item body: 16 bytes (type + count + Option<Box<attrs>>)
│   └── SlotMap slot overhead              → ~30 MB (unchanged)
│   └── ItemAttributes boxes               → rare at map load; player/depot pay later
├── map.qtrees: HashMap<z, QTreeNode>      per-floor FULL trees  (many GB)  ← main gap
│   └── cached_spectators clones           → doubles creature lists
├── creatures: SlotMap<CreatureKind>        ~24k at startup
│   ├── registered_events: HashSet         → ~1.1 MB empty sets
│   ├── damage_map: HashMap                → ~1.1 MB empty maps
│   └── CreatureKind enum padding          → ~5–10 MB
├── spawns.slots                           ~24k metadata
│   └── spawns.zones (dead field)          → few MB wasted
├── items_db / monsters_db (Arc)           content tables
├── Lua runtime × 2                        → doubled VM overhead
└── Lua runtime                            scripts + VM
```

---

## Recommended fixes (priority)

### Steady-state (GB-scale)

1. **Lazy sparse quadtree (C++ parity)** — Replace recursive `Box<QTreeNode>` tree with a flat `HashMap<(u16, u16), LeafData>` where keys are `(x / 32, y / 32)`. Leaves created on first creature insert, not `build()`. Remove `QTreeNode::build(0, 0, w, h)` from `from_map_data` (`map/mod.rs:47–55`) and `register_creature_index` (`map/mod.rs:303–306`). Eliminates multi-GB eager tree. Drop `cached_spectators` (unnecessary with flat hash map query).

2. ~~**Uniform lightweight items**~~ — **Done (Phase A).** `Option<Box<ItemAttributes>>`, no `Item.id`. ~145 MB saved at Forgotten scale.

### Hundreds of MB

6. **`FxHashMap` + packed Position key** — Superseded by Phase B unified grid for tiles; only relevant if Phase B is deferred. — Pack `(x, y, z)` into `u64` for the tile map key. Switch `HashMap<Position, Tile>` to `FxHashMap<u64, Tile>`. Faster lookups + fewer collisions + smaller bucket arrays. Saves ~50–100 MB.

### Boot / tens of MB

7. **Extend RSS diagnostic** in `run_server.rs:237–244` with `map_width`, `map_height`, `qtree_floors`, and `size_of::<Item>()` (expect **16** post–Phase A) / `size_of::<ItemAttributes>()` / `size_of::<TileBody>()` for compile-time layout audits in logs.

8. ~~**Remove `Item.id` field**~~ — **Done (Phase A).**

9. **Merge Lua VMs** — Load `data/formulas/<version>.lua` into the same `LuaRuntime` as creaturescripts; store formula `mlua::Function`s in `FormulaHooks` via the shared VM's `RegistryKey`.

10. **Drop `SpawnManager::zones`** after slot construction (`spawn.rs:75`), or remove the field.

11. **`Monster::registered_events`** → `u8` bitflags enum. Only ever checks `"onThink"`.

12. **Lazy `damage_map` / `spell_cooldown_end`** — `Option<Box<HashMap<…>>>`, initialized `None` until first use.

13. **Avoid `ItemDatabase::clone`** in `pipeline.rs:63` — pass `Arc<ItemDatabase>` or `&ItemDatabase` into monster load on the blocking task.

14. **Creature display names** — store `MonsterType` key or `Arc<str>` shared name instead of `String` clone per spawn.

### Load spike / hot-path

15. **Fix `tile_from_data` double parse** (`map/mod.rs:266,282`) — cache the result of `item_id_from_otbm_item_props` in the first match arm.

16. **Fix `add_item` insert(0)** (`tile.rs:126`) — during OTBM load, build items in reverse or push to end and reverse once. Eliminates O(n) memmove per item.

17. **Iterator-based LOS** — Replace `walk_grid_line` → `Vec<Position>` (`los.rs:9`) with a zero-alloc Bresenham iterator.

18. **`DecayManager` time-bucketed** — Replace `HashMap<ItemId, DecayEntry>` with `BTreeMap<(u64, ItemId), …>` for O(log n) expiration instead of full-map `retain()`.

19. **Optional:** global allocator (`jemalloc` / `mimalloc`) so transient OTBM parse memory returns to the OS faster.

20. **Item position index** — if `find_item_position` stays, maintain `ItemId → Position` for map items instead of scanning 7.8M tiles.

---

## How to verify on your machine

While the server is running after the log line `GameWorld ready — steady-state entity counts`:

```bash
ps -o pid,rss,vsz,cmd -p "$(pgrep -f 'tfs-rust' | head -1)"
ls -lh data/world/forgotten.otbm   # OTBM file size (load spike input)
```

If `width`/`height` are ~65535 and `qtrees.len()` is ~10–16 after startup (once fix #7 lands), the eager quadtree hypothesis is confirmed.

---

## Source anchors

Every section above references specific code locations. If any of these are refactored, **update this document in the same PR** (per `tfs-code-hygiene` rule: "minimal diff, fix the task").

| Anchor | File | Line(s) | What |
|--------|------|---------|------|
| RSS log | `crates/tfs-rust-core/src/run_server.rs` | 237–244 | `info!(map_tiles, items_slotmap, …)` |
| Eager qtree z=0 | `crates/tfs-rust-core/src/map/mod.rs` | 47–55 | `QTreeNode::build(0, 0, w, h)` |
| Eager qtree per-floor | `crates/tfs-rust-core/src/map/mod.rs` | 300–307 | `register_creature_index` → `or_insert_with(build)` |
| `QTreeNode::build` | `crates/tfs-rust-core/src/map/qtree.rs` | 35 | Recursive full subdivision |
| Spectator cache clone | `crates/tfs-rust-core/src/map/qtree.rs` | 244 | `creatures.clone()` |
| `internal_add_item_id` | `crates/tfs-rust-core/src/map/mod.rs` | 136–138 | `Item::new_single(id)` + `items.insert` |
| `Item` struct | `crates/tfs-rust-core/src/item.rs` | 25–33 | `item_type`, `count`, `Option<Box<ItemAttributes>>` |
| `Item` accessors | `crates/tfs-rust-core/src/item.rs` | 52–129 | Lazy `get_or_insert_with` on setters |
| `ItemAttributes` struct | `crates/tfs-rust-core/src/item_attributes.rs` | 130–168 | Heap-only when boxed; 28 scalars + 6 Option<String> |
| `write_item_blob` | `crates/tfs-rust-core/src/item_blob.rs` | 224–239 | Early return when `attributes` is `None` |
| `TileBody` struct | `crates/tfs-rust-core/src/tile.rs` | 53–63 | `position`, `ground`, 3× Vec, `flags`, `zone` |
| `TileBody.position` | `crates/tfs-rust-core/src/tile.rs` | 54 | Duplicate of HashMap key |
| `Tile::add_item` insert(0) | `crates/tfs-rust-core/src/tile.rs` | 126 | `down_items.insert(0, item_id)` |
| `SpawnManager::zones` | `crates/tfs-rust-core/src/spawn.rs` | 48 | Dead field after `from_zones` |
| `SpawnManager::from_zones` | `crates/tfs-rust-core/src/spawn.rs` | 75 | Builds `zones` + `slots` |
| `Monster::registered_events` | `crates/tfs-rust-core/src/creature/monster.rs` | 72 | `HashSet<String>` per instance |
| `Monster::wants_lua_think` | `crates/tfs-rust-core/src/creature/monster.rs` | 127 | Only user of `registered_events` |
| `CreatureBase::damage_map` | `crates/tfs-rust-core/src/creature/base.rs` | 115 | `HashMap<CreatureId, u64>` per creature |
| `CreatureKind` enum | `crates/tfs-rust-core/src/creature/kind.rs` | 15–19 | `allow(large_enum_variant)` |
| `walk_grid_line` | `crates/tfs-rust-core/src/map/los.rs` | 9 | `Vec<Position>` per LOS |
| `find_item_position` | `crates/tfs-rust-core/src/map/mod.rs` | 77–84 | O(tiles) linear scan |
| `spectator_conns` | `crates/tfs-rust-core/src/game_world.rs` | 199–204 | O(all_connections) per broadcast |
| `tile_from_data` double parse | `crates/tfs-rust-core/src/map/mod.rs` | 266, 282 | `item_id_from_otbm_item_props` called twice |
| `DecayManager::tick` | `crates/tfs-rust-core/src/decay.rs` | 35–46 | `retain()` full scan per tick |
| `ItemDatabase::clone` | `crates/tfs-rust-content/src/pipeline.rs` | 63 | `items.clone()` at load |
| Lua VM #1 | `crates/tfs-rust-core/src/run_server.rs` | 163 | `LuaRuntime::new()` |
| Lua VM #2 | `crates/tfs-rust-core/src/formulas.rs` | 464 | `Lua::new()` for formulas |
| `Position` struct | `crates/tfs-rust-common/src/position.rs` | 5 | `u16, u16, u8` — derives Hash (SipHash) |
| Map tiles HashMap | `crates/tfs-rust-core/src/map/mod.rs` | 29 | `HashMap<Position, Tile>` |

---

## Code references

| Topic | Rust | C++ (TFS 1.4.2 / TVP 7.72) |
|-------|------|-----|
| Map tiles | `crates/tfs-rust-core/src/map/mod.rs` | `src/map.cpp`, `src/iomap.cpp` |
| Quadtree | `crates/tfs-rust-core/src/map/qtree.rs` | `src/map.h`, `src/map.cpp` (both use lazy `createLeaf`, external `SpectatorCache`) |
| Map item creation | `map/mod.rs` `internal_add_item_id` | `src/tile.cpp` `internalAddThing` |
| Boot / RSS log | `crates/tfs-rust-core/src/run_server.rs` | `src/otserv.cpp` / `Game::loadMap` |
| Startup spawns | `crates/tfs-rust-core/src/spawn_lifecycle.rs` | `src/spawn.cpp` `Spawns::startup` |
| Content load | `crates/tfs-rust-content/src/pipeline.rs` | `src/game.cpp` loaders |
| Item attributes | `crates/tfs-rust-core/src/item_attributes.rs` | `src/item.h` `ItemAttributes` |
| Tile structure | `crates/tfs-rust-core/src/tile.rs` | `src/tile.h`, `src/tile.cpp` |
| Creature base | `crates/tfs-rust-core/src/creature/base.rs` | `src/creature.h` |
| Monster | `crates/tfs-rust-core/src/creature/monster.rs` | `src/monster.h` |

---

## Bottom line

~15 GB RSS on Forgotten is explained by **legitimate world size** (~7.8M tiles, ~750k stack items, ~24k startup creatures) plus **implementation choices that C++ does not make**:

- **Full-map quadtrees** per active floor (often **many GB**) — primary issue (Phase B).
- ~~**Heavyweight per-item `SlotMap` entries**~~ — **mitigated (Phase A):** 16-byte `Item` + lazy attributes (~42 MB item area vs ~187 MB).
- **Oversized tile storage**: duplicate Position, empty Vec headers, SipHash — tertiary ~200+ MB issue (Phase B).
- **Per-creature waste**: empty HashSet/HashMap per instance, enum variant padding — ~8 MB.
- **Dual Lua VMs, dead `zones`, cloned databases** — tens of MB each.
- **Hot-path allocations** (LOS Vec, spectator_conns Vec, broadcast clones) — runtime, not steady-state.

Phase A removed ~145 MB from the item slotmap area; **RSS will still be dominated by quadtrees and tiles** until Phase B. Fixing the quadtree to match C++ lazy allocation is the highest-leverage remaining change and matches the behavior you see when C++ memory falls after map load.
