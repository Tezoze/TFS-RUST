# Map / Grid Performance & Parity Audit

**Date:** 2026-07-19
**Scope:** static source audit of map storage, spatial indexing, tile lookup, spectator
fan-out, line-of-sight / throw, pathfinding viewport scans, OTBM ingest, and full-map
iteration paths. This is **not** a sampled CPU profile — findings are ranked from code
shape, known cardinalities, and the 772 reference behavior.
**Companion to:** `docs/MAP_SYSTEM_AUDIT.md` (correctness fixes #1–#9, Phases 1–3 ✅).
This doc focuses on **performance and parity hot paths** that survived the correctness
pass and are now the scaling ceiling.

**Rust side:**
- `crates/tfs-rust-core/src/map/grid.rs` — `SparseGrid`, `Chunk`, `ChunkKey`, `collect_spectators`, `find_item_position`
- `crates/tfs-rust-core/src/map/mod.rs` — `Map`, OTBM ingest, `register_creature_at` / `unregister_creature_at`
- `crates/tfs-rust-core/src/map/los.rs` — `walk_grid_line`, `is_sight_clear`, `throw_possible`, `column_blocks_throw`
- `crates/tfs-rust-core/src/tile.rs` — `TileBody`, `add_creature` / `remove_creature`
- `crates/tfs-rust-core/src/game_world_spectators.rs` — `spectator_conns_via_grid`, `spectator_players_in_box`
- `crates/tfs-rust-core/src/monster_events.rs` — `collect_spatial_spectators`, `monsters_witnessing_move`, `spectator_z_range`
- `crates/tfs-rust-core/src/pathfinding.rs` — `scan_min_terrain_waypoints`, forward A*, `TShortway` reverse path
- `crates/tfs-rust-core/src/game_world_item_cylinder.rs` — `internal_add_item_to_tile`, `internal_remove_item_from_tile`
- `crates/tfs-rust-core/src/game_world_lifecycle.rs` — `remove_creature` summons scan

**Reference:**
- 772 outcomes: `reference/cipsoft-772/tibia-game-master/src/` — `map.hh`, `info.cc`, `crmain.cc`, `cract.cc`, `operate.cc`
- 772 wire / TFS-style domain: `reference/tvp-772/gameserver/src/map.cpp` — `Map::getSpectators`, `QTreeLeafNode`
- 1098 TFS: repo-root `src/map.cpp`, `src/map.h`

Findings are graded **BUG / GAP / SUSPECT** and severity-ranked **HIGH / MED / LOW** by
hot-path frequency × per-call cost × parity impact.

---

## Executive summary

The chunk grid is well-shaped for a resident OTBM world (lazy allocation, packed
`ChunkKey`, `FxHashMap` for integer keys, dual creature list sealed by `pub(super)` and
verified by `debug_assert_creature_lists_agree`). The correctness pass
(`MAP_SYSTEM_AUDIT.md` Phases 1–3) fixed the LOS flag, the 772 wire stack cap, void-tile
registration, and the player fan-out path. What remains is **amplification**:

1. **Per-move fan-out does four sort+dedup passes** for a single creature step. With
   1000 moving players this is ~4000 sorts/sec on top of the actual broadcast cost.
2. **`find_item_position` is a full-world scan** (every chunk × 4096 slots). Currently
   only house auto-close and a few teleport checks use it, but there is no reverse index
   guarding the hot path.
3. **`remove_creature` scans all creatures to find summons** on every death. 772 keeps
   `summons` on the master; Rust re-derives it each time.
4. **`scan_min_terrain_waypoints` runs a 21×21 tile scan on every monster pathfinding
   attempt.** Each tile is a `get_tile` HashMap lookup — ~441 lookups per chase repath,
   multiplied by the monster count.
5. **Chunk granularity is 4× coarser than 772's 16×16 creature blocks.** Already
   documented (audit #5, `MAP_SYSTEM_AUDIT.md`); this doc quantifies the over-collection
   and ties it to the redundant sort/dedup work.
6. **LOS / throw / `walk_grid_line` re-hash the chunk key per tile visited.** A 20-tile
   line does 20 `ChunkKey::from_pos` + 20 `FxHashMap::get` calls where one chunk
   reference could cover most of the line.
7. **OTBM load allocates 3 `Vec`s per tile** (`down_items`, `top_items`, `creatures`)
   even when the tile is empty. 100k tiles → 300k heap allocations during load.
8. **`TShortway` allocates a 529-cell `Vec` per reverse path search.** 772 uses a
   stack-allocated `matrix<TShortwayPoint>`; Rust heap-allocates and initializes every
   cell on every chase repath.

Items 1, 3, and 4 are the highest-leverage fixes (per-tick amplification). Items 2, 6,
and 7 are load/scan hot paths. Item 5 is a parity gap with a performance tail. Items 8
is per-chase-repath allocation churn.

---

## Reference behavior that must remain intact

| Area | 772 behavior | Rust status |
|---|---|---|
| Tile storage | 32×32 `TSector` per floor, swappable | 64×64 lazy `Chunk`, resident OTBM — ✅ correct replacement |
| Creature spatial index | 16×16 `TFindCreatures` blocks, row-major walk, `NextChainCreature` linked list (`crmain.cc:101-144`) | 64×64 `Chunk.creatures` `SmallVec`, sorted by SlotMap key — ⚠️ 4× coarser, different tie-break order |
| Spectator fan-out | `TFindCreatures::getNext` walks blocks, caller filters by `canSee` | `collect_spectators` walks chunks, caller filters by `canSee` — ✅ shape, ⚠️ granularity |
| LOS | `ThrowPossible` (`info.cc:1154`) iterates line tiles via `GetFirstObject` | `throw_possible` (`los.rs`) iterates `walk_grid_line` output — ✅ outcomes, ⚠️ allocation per call |
| Item position | Tracked via `Object` container parent pointers | `find_item_position` full-world scan — ❌ no reverse index |
| Pathfinding viewport | `TShortway::FillMap` (`cract.cc:80-138`) walks viewport once per path | `scan_min_terrain_waypoints` walks viewport once per path — ✅ shape, ⚠️ per-tile hash lookup |
| Summons on death | Master creature keeps `summons` list (TFS `Creature::summons`) | `remove_creature` scans all creatures — ❌ O(N) per death |
| OTBM tile load | Direct pointer assignment into sector array | Per-tile `Box<Tile>` + 3 `Vec::new()` — ⚠️ allocation overhead |
| Player-only fan-out | 772 has no separate `player_list`; 1098 `QTreeLeafNode` keeps one | Single `Chunk.creatures` list, filtered at call site — ⚠️ missed optimization |

---

## Prioritized findings

| ID | Priority | Hot path | Failure mode |
|---|---:|---|---|
| MAP-1 | **P0 / Critical** | Per-move fan-out | 4 sort+dedup passes per creature step; 4000 sorts/sec at 1000 movers |
| MAP-2 | **P0 / Critical** | `find_item_position` | Full-world scan; O(chunks × 4096) per house auto-close / teleport check |
| MAP-3 | **P0 / Critical** | `remove_creature` death | Scans all creatures to find summons; O(N) per death in mass combat |
| MAP-4 | **P1 / High** | `scan_min_terrain_waypoints` | 441 `get_tile` hash lookups per monster pathfinding attempt |
| MAP-5 | **P1 / High** | Chunk granularity | 64×64 vs 772 16×16 → 16× over-collection + tie-break order divergence |
| MAP-6 | **P1 / High** | LOS / throw loops | Per-tile `ChunkKey::from_pos` + `FxHashMap::get`; no chunk reference caching |
| MAP-7 | **P1 / High** | OTBM load | 3 `Vec::new()` per tile; 300k allocations for 100k-tile map |
| MAP-8 | **P1 / High** | `TShortway` reverse path | 529-cell `Vec` allocated + initialized per chase repath |
| MAP-9 | **P2 / Medium** | Tile creature removal | `Vec::position` linear scan; O(n) on stacked depot tiles |
| MAP-10 | **P2 / Medium** | Z-range over-scan | `spectator_z_range` scans 8–10 floors for surface queries; no per-floor creature count guard |
| MAP-11 | **P2 / Medium** | Spectator Vec capacity | `Vec::new()` with no capacity hint; reallocs in crowded areas |
| MAP-12 | **P2 / Medium** | `walk_grid_line` allocation | New `Vec<Position>` per LOS check; ~480 bytes for a 20-tile line |
| MAP-13 | **P3 / Low** | Sparse chunk memory waste | 64×64 chunk = 32 KB pointer array even with 1 populated tile |
| MAP-14 | **P3 / Low** | Missing player-only list | 1098 `QTreeLeafNode::player_list` optimization absent; Rust filters after collection |
| MAP-15 | **P3 / Low** | `SmallVec<[CreatureId; 4]>` capacity | Crowded chunks (depots, spawns) spill to heap |

---

## MAP-1 — Per-move fan-out does four sort+dedup passes — P0 / Critical

**Hot path:** every creature step (`walk/mod.rs:1059-1160`), every monster move stimulus
(`monster_events.rs:105-133`).

**Rust:**
```rust
// walk/mod.rs:1059-1062 — player packet fan-out
let mut spectator_conns: Vec<ConnId> = self.spectator_conns_via_grid(old_pos);  // sort+dedup #1
spectator_conns.extend(self.spectator_conns_via_grid(new_pos));                  // sort+dedup #2
spectator_conns.sort_by_key(|c| c.0);                                            // sort #3
spectator_conns.dedup();                                                         // dedup #3

// monster_events.rs:110-132 — monster move stimulus
let mut ids: Vec<CreatureId> = self
    .collect_spatial_spectators(old_pos, true)    // sort+dedup #1 (inside)
    .into_iter()
    .chain(self.collect_spatial_spectators(new_pos, true))  // sort+dedup #2 (inside)
    .filter(|&id| /* monster filter */)
    .collect();
ids.sort_by_key(|id| id.data().as_ffi());         // sort #3
ids.dedup();                                       // dedup #3
```

**772:** `TFindCreatures::getNext` (`crmain.cc:101-144`) walks 16×16 blocks once per
query; the move stimulus calls it twice (old + new) but each call is a single
row-major traversal with no sort — the linked-list order is the fan-out order.

**Impact:**
- 4 sorts + 4 dedups per creature move
- At 1000 moving players: ~4000 sorts/sec on top of the actual packet emission
- O(k log k) × 4 where k = spectators in viewport (typically 30–100 in towns)

**Required change (idiomatic Rust, parity-preserving):**
Single-pass collection with a `HashSet<CreatureId>` for dedup, one sort at the end:
```rust
fn spectators_union(&self, old_pos: Position, new_pos: Position) -> Vec<CreatureId> {
    let mut seen = FxHashSet::default();
    let mut out = Vec::new();
    for pos in [old_pos, new_pos] {
        for z in Self::spectator_z_range(pos.z, true) {
            self.map.grid.collect_spectators_dedup(pos.x, pos.y, z,
                MAP_MAX_VIEWPORT, MAP_MAX_VIEWPORT, &mut seen, &mut out);
        }
    }
    out.sort_by_key(|id| id.data().as_ffi());  // single sort
    out
}
```
`collect_spectators_dedup` skips creatures already in `seen` during the chunk walk,
eliminating both inner sort+dedup passes and the outer union sort+dedup.

**Verification:**
- `tests/map_storage.rs` — add `spectators_union_single_sort_dedup` benchmark
- `cargo test -p tfs-rust-core map_storage` stays green
- Flamegraph: sort/dedup CPU time drops ~75% under mass movement

---

## MAP-2 — `find_item_position` is a full-world scan — P0 / Critical

**Hot path:** house auto-close, teleport destination validation, depot checks.

**Rust** (`grid.rs:228-240`):
```rust
pub fn find_item_position(&self, item_id: crate::ids::ItemId) -> Option<Position> {
    for (key, chunk) in &self.chunks {
        let (ox, oy, z) = key.chunk_origin();
        for (idx, slot) in chunk.tiles.iter().enumerate() {
            if let Some(tile) = slot {
                if tile.has_item(item_id) {
                    return Some(position_from_chunk_slot(ox, oy, z, idx));
                }
            }
        }
    }
    None
}
```

**772:** `Object` carries a container parent pointer (`map.hh:64` `Container`) — item
position is read directly from the object, no scan. TFS 1.4.2 `Item::getTile` walks
the parent cylinder chain, also O(1).

**Impact:**
- O(chunks × 4096 slots) per call
- 1000 chunks × 4096 = 4.096M tile checks worst case
- Currently called from house auto-close and a few teleport paths — not yet a hot path,
  but **no reverse index guards it**. Any new caller (e.g. a script hook) makes it a
  scaling cliff.

**Required change:**
Add a reverse index `HashMap<ItemId, Position>` on `GameWorld` (or `Map`), maintained
in:
- `internal_add_item_to_tile` (`game_world_item_cylinder.rs:219`) — insert after tile add
- `internal_remove_item_from_tile` (`game_world_item_cylinder.rs:347`) — remove after tile remove
- Item move between tiles / containers — update on parent change
- OTBM load — bulk-insert during `from_map_data`

Keep `find_item_position` as a fallback for items whose parent is not a tile (containers,
inventory). The reverse index only tracks tile-resident items.

**Verification:**
- `tests/map_storage.rs` — `find_item_position_uses_reverse_index` (O(1) lookup)
- Benchmark: 100k tiles, find item < 1µs (was up to 4M iterations)

---

## MAP-3 — `remove_creature` scans all creatures for summons — P0 / Critical

**Hot path:** every creature death (`game_world_lifecycle.rs:47-52`).

**Rust:**
```rust
let mut summons: Vec<CreatureId> = Vec::new();
for (cid, k) in self.creatures.iter() {  // Scans ALL creatures
    if k.base().master == Some(id) {
        summons.push(cid);
    }
}
for s in summons {
    self.remove_creature(s);
}
```

**772 / TFS:** `TCreature` keeps a `summons` list on the master (TFS `Creature::summons`,
`creature.h`). Death walks the master's own list, not the world.

**Impact:**
- O(N) per death where N = all creatures on the server
- Mass combat (UE spell, raid): 10000 creatures × 10000 scans = 100M iterations/sec
  worst case
- Amplifies during raid events and PvP brawls

**Required change:**
Add `summons: SmallVec<[CreatureId; 2]>` to `CreatureBase` (or `Monster`), maintained in:
- `summon_creature` / `create_summon` — push to master's list
- `remove_creature` — drain the master's list (current scan path becomes a fallback)
- Master logout / despawn — clear list

**Verification:**
- `tests/monster_ai.rs` — `remove_creature_uses_master_summons_list`
- Benchmark: 1000 summons, death of master < 1ms (was O(N) scan)

---

## MAP-4 — `scan_min_terrain_waypoints` does 441 hash lookups per path — P1 / High

**Hot path:** every monster pathfinding attempt
(`pathfinding.rs:54-83`, called from `monster_ai.rs:844, 1350, 1413`).

**Rust:**
```rust
for dy in -radius..=radius {          // 21 iterations for radius=10
    for dx in -radius..=radius {      // 21×21 = 441 iterations
        let Some(pos) = offset_position(origin, dx, dy) else { continue; };
        if !map.is_walkable(pos) {    // get_tile → ChunkKey::from_pos → FxHashMap::get
            continue;
        }
        let wp = effective_terrain_waypoints(ground_cost(pos));  // another get_tile
        // ...
    }
}
```

**772:** `TShortway::FillMap` (`cract.cc:80-138`) walks the same viewport once per
path, but the sector array is direct-indexed — no hash lookup per tile.

**Impact:**
- 441 `get_tile` calls per pathfinding attempt, each = 1 `ChunkKey::from_pos` + 1
  `FxHashMap::get` + 1 array index
- 1000 monsters pathfinding/sec → 441,000 hash lookups/sec
- `ground_cost` callback (caller-supplied) typically does another `get_tile`, doubling
  the lookup count

**Required change (incremental, parity-preserving):**
1. **Cache the chunk reference across the inner loop.** When `dx` advances within the
   same chunk, reuse the `&Chunk` reference instead of re-hashing. The chunk boundary
   is at `x % CHUNK_SIZE == 0` — detectable in the inner loop.
2. **Pass the tile reference to `ground_cost`** instead of the position, eliminating
   the callback's redundant `get_tile`.
3. **Long-term:** cache `effective_terrain_waypoints` per tile in the chunk during map
   load (a `Box<[u32; CHUNK_AREA]>` per chunk, populated at OTBM ingest). Runtime
   reads become a single array index. Tile mutations (field creation, ground change)
   invalidate the cached entry.

**Verification:**
- `tests/pathfinding.rs` — `scan_min_terrain_waypoints_uses_cached_chunk`
- Benchmark: 441-tile scan < 50µs (was ~200µs with per-tile hash lookup)

---

## MAP-5 — Chunk granularity 4× coarser than 772 creature blocks — P1 / High

**Already documented:** `MAP_SYSTEM_AUDIT.md` finding #5 (Phase 4). This doc quantifies
the over-collection and ties it to MAP-1.

**Rust:** `CHUNK_SIZE = 64` (`grid.rs:13`). One chunk serves both tiles and the creature
spatial index. A range-11 spectator query (23×23 box) can pull creatures from a
~128×128 span (16 chunks × 16 chunks worst case).

**772:** 32×32 tile sectors **plus** a separate 16×16 creature block index
(`TFindCreatures`, `crmain.cc:101-144`). The creature blocks are walked in row-major
order (blockx inner, blocky outer), following the `NextChainCreature` linked list
within each block.

**Impact:**
- **Over-collection:** 16× more creatures collected per query in the worst case
  (64×64 / 16×16 = 16× area ratio)
- **Sort/dedup amplification:** the over-collected set is what MAP-1 sorts four times
  per move
- **Tie-break order divergence:** creatures in one 64×64 chunk but different 16×16
  blocks arrive in chunk-insertion order, not 772 block order. Observable only in
  tie-break edge cases (multiple monsters equidistant from a move event). Documented
  at `monster_events.rs:120-129` (GL#24).

**Required change (parity fix, larger effort):**
Add a dedicated 16×16 creature bucket index, separate from the 64×64 tile chunk —
exactly as 772 splits them. Tile chunks stay 64×64 (good for tile storage density);
the creature index gets its own `FxHashMap<BlockKey, SmallVec<[CreatureId; 2]>>` with
16×16 blocks.

```rust
const CREATURE_BLOCK_SIZE: u16 = 16;

pub(crate) struct CreatureBlockIndex {
    blocks: FxHashMap<BlockKey, SmallVec<[CreatureId; 2]>>,
}

// Walk 16×16 blocks in row-major order (blockx inner, blocky outer) — matches
// TFindCreatures::getNext (crmain.cc:101-144).
pub fn collect_spectators_16x16(&self, ...) -> Vec<CreatureId> { ... }
```

This eliminates the over-collection, the tie-break divergence, and shrinks the sort
input for MAP-1.

**Verification:**
- `tests/map_storage.rs` — `collect_spectators_16x16_matches_772_block_order`
- `tests/monster_events.rs` — `monsters_witnessing_move_uses_block_order`
- Parity trace: compare fan-out order against 772 `TFindCreatures::getNext` trace

---

## MAP-6 — LOS / throw loops re-hash the chunk key per tile — P1 / High

**Hot path:** `is_sight_clear` (`los.rs:59-67`), `throw_possible` (`los.rs:87-152`),
`can_throw_object_to` (`game_world_player_throw.rs:177-199`).

**Rust:**
```rust
for p in walk_grid_line(from, to) {
    if p == from || p == to { continue; }
    if self.blocks_sight(p) {  // get_tile → ChunkKey::from_pos → FxHashMap::get
        return false;
    }
}
```

Each `blocks_sight(p)` call:
1. `ChunkKey::from_pos(x, y, z)` — packed u32 computation
2. `self.chunks.get(&key)` — FxHashMap lookup
3. `tile_index(x, y)` — array index
4. `as_deref()` — Option chain

For a 20-tile LOS check, that's 20 hash lookups. Most of those tiles are in the same
chunk (64×64 = 4096 tiles per chunk).

**772:** `ThrowPossible` (`info.cc:1154`) iterates `GetFirstObject(x, y, z)` per tile —
sector array direct-indexed, no hash lookup.

**Impact:**
- 20–100 hash lookups per LOS check (multi-floor throws)
- Combat: every wand / bolt / spell LOS check
- Monster AI: `clear_sight` path goals (`pathfinding.rs:904`), kiting / flee decisions
  (`monster_ai.rs:1566, 1864`)

**Required change:**
Cache the chunk reference across the inner loop. When the line stays within one chunk,
hold the `&Chunk` reference and index directly into `tiles[tile_index(x, y)]`. Re-hash
only on chunk boundary crossing.

```rust
fn blocks_sight_cached(&self, p: Position, cached: &mut Option<(ChunkKey, &Chunk)>) -> bool {
    let key = ChunkKey::from_pos(p.x, p.y, p.z);
    if cached.map_or(false, |(k, _)| k == key) {
        // same chunk — use cached reference
    } else {
        *cached = self.chunks.get(&key).map(|c| (key, c.as_ref()));
    }
    // ...
}
```

For `throw_possible`'s multi-floor loop, batch tile lookups by chunk: collect all
positions first, group by chunk, then fetch each chunk once.

**Verification:**
- `tests/map_los.rs` — `is_sight_clear_uses_cached_chunk` (no behavior change)
- Benchmark: 20-tile LOS check < 5µs (was ~20µs with per-tile hash lookup)

---

## MAP-7 — OTBM load allocates 3 `Vec`s per tile — P1 / High

**Hot path:** map load (`map/mod.rs:327-334`).

**Rust:**
```rust
let mut body = TileBody {
    ground: None,
    down_items: Vec::new(),      // Allocates per tile
    top_items: Vec::new(),       // Allocates per tile
    creatures: Vec::new(),       // Allocates per tile
    flags: converted_flags,
    zone,
};
```

**772:** `TSector` is a fixed array; tiles are pointer slots. `StaticTile` (TFS 1.4.2)
defers vector allocation until items are added.

**Impact:**
- 100k tiles → 300k `Vec` allocations during load
- Each `Vec::new()` is a heap allocation (24 bytes for the Vec struct, plus capacity
  growth when items are added)
- Load time for large maps (100k+ tiles) dominated by allocator pressure

**Required change (idiomatic Rust, no behavior change):**
Switch to `SmallVec` for the common case (most tiles have 0–2 items):
```rust
pub struct TileBody {
    pub ground: Option<u16>,
    pub down_items: SmallVec<[ItemId; 2]>,   // inline 0–2, heap for larger
    pub top_items: SmallVec<[ItemId; 2]>,    // inline 0–2, heap for larger
    pub creatures: SmallVec<[CreatureId; 2]>, // inline 0–2, heap for larger
    pub flags: u32,
    pub zone: ZoneType,
}
```

`SmallVec<[T; 2]>` stores 2 elements inline (16 bytes for the buffer + 24 bytes for
the Vec fields = 40 bytes total). For tiles with 0 items, no heap allocation. For
tiles with 1–2 items, no heap allocation. For tiles with 3+ items, one heap allocation
(same as current `Vec`).

**Trade-off:** `TileBody` size grows from ~80 bytes to ~120 bytes. The chunk pointer
array (`Box<[Option<Box<Tile>>; 4096]>`) is unaffected — tiles are still boxed.

**Verification:**
- `tests/map_storage.rs` — `tile_body_smallvec_no_alloc_for_empty_tile`
- Benchmark: 100k-tile map load < 500ms (was ~1s with 300k Vec allocations)
- `cargo test -p tfs-rust-core` stays green (SmallVec is API-compatible with Vec)

---

## MAP-8 — `TShortway` allocates 529 cells per reverse path search — P1 / High

**Hot path:** every monster chase repath (`pathfinding.rs:672-684`).

**Rust:**
```rust
let mut cells = vec![
    TShortwayCell {
        waypoints: -1,
        waylength: TSHORTWAY_UNVISITED_WL,
        heuristic: TSHORTWAY_UNVISITED_H,
        parent: None,
        parent_diagonal: false,
        expand_next: None,
        in_matrix: false,
    };
    TSHORTWAY_MAX_CELLS  // 23×23 = 529 cells
]
.into_boxed_slice();
```

**772:** `TShortway::TShortway` (`cract.cc:51-72`) allocates a `matrix<TShortwayPoint>`
on the heap via `new`, but the matrix is fixed-size and the C++ allocator reuses freed
slots aggressively. The structural cost is the same; the difference is Rust's
per-search allocation vs C++'s arena reuse.

**Impact:**
- 529 cells × ~48 bytes = ~25 KB per reverse path search
- 1000 monsters repathing/sec → 25 MB/sec allocation churn
- Each cell is initialized with 8 fields — memset cost on top of allocation

**Required change:**
1. **Short-term:** use `Box::new([TShortwayCell::default(); TSHORTWAY_MAX_CELLS])`
   with `TShortwayCell: Copy + Default` — same heap allocation but the compiler can
   optimize the initialization to a `memset`.
2. **Medium-term:** pool the buffer. Keep a `Vec<Box<[TShortwayCell; 529]>>` on
   `GameWorld` (or thread-local) and reuse across searches. `clear()` between uses
   instead of `new()`.
3. **Long-term:** stack-allocate via `MaybeUninit<[TShortwayCell; 529]>` and only
   initialize cells as they're visited. Most cells in a 23×23 viewport are never
   touched (walls, obstacles).

**Verification:**
- `tests/pathfinding.rs` — `tshortway_reuses_pooled_buffer`
- Benchmark: 1000 reverse path searches < 50ms (was ~150ms with per-search allocation)

---

## MAP-9 — Tile creature removal is O(n) — P2 / Medium

**Hot path:** every creature move off a tile (`tile.rs:175-182`).

**Rust:**
```rust
pub fn remove_creature(&mut self, id: CreatureId) -> bool {
    let body = self.body_mut();
    if let Some(i) = body.creatures.iter().position(|&c| c == id) {  // O(n)
        body.creatures.swap_remove(i);
        return true;
    }
    false
}
```

**772 / TFS:** tile creature lists are short (typically 1–5 creatures); linear scan is
fine. Depots and spawn tiles can hit 20+.

**Impact:**
- O(n) per removal where n = creatures on tile
- Stacked depot tiles (20+ players): 20 linear scans per move
- Amplified by MAP-1 (4 fan-out passes per move)

**Required change (only if profiling shows it's hot):**
- Keep `Vec<CreatureId>` for stack-rendering order (creatures must render in insertion
  order for the client stackpos).
- Add a side index `HashMap<CreatureId, usize>` on `TileBody` for O(1) lookup, rebuilt
  on `swap_remove`. Only worth it if depots are a measured hotspot.

**Verification:** defer until profiling confirms depot tiles are hot.

---

## MAP-10 — Z-range over-scan for surface queries — P2 / Medium

**Hot path:** every spectator fan-out (`monster_events.rs:41-60`,
`game_world_spectators.rs:150-161`).

**Rust:**
```rust
pub(crate) fn spectator_z_range(center_z: u8, multifloor: bool) -> RangeInclusive<u8> {
    if !multifloor { return center_z..=center_z; }
    if center_z > 7 { return center_z.saturating_sub(2)..=(center_z + 2).min(15); }
    if center_z == 6 { return 0..=8; }   // 9 floors
    if center_z == 7 { return 0..=9; }   // 10 floors
    0..=7                                // 8 floors
}
```

**772:** `TFindCreatures` walks 16×16 blocks on a **single floor** — the caller
explicitly passes the floor. Multi-floor spectator queries are a TFS / 1098 concept
(`Map::getSpectators` with `multifloor = true`).

**Impact:**
- Surface (z ≤ 7) scans 8–10 floors per query
- Most creatures are on a single floor; the other 7–9 floors are empty chunks
- Each empty floor still costs a `FxHashMap::get` (returns `None` fast, but multiplied
  by query rate)

**Required change:**
Track per-floor creature count on `SparseGrid`:
```rust
pub struct SparseGrid {
    chunks: FxHashMap<ChunkKey, Box<Chunk>>,
    floor_creature_count: [usize; 16],  // indexed by z
}
```
Update in `register_creature` / `unregister_creature`. In `collect_spectators`, skip
floors with `floor_creature_count[z] == 0`:
```rust
for z in Self::spectator_z_range(pos.z, multifloor) {
    if self.map.grid.floor_creature_count[z as usize] == 0 { continue; }
    self.map.grid.collect_spectators(pos.x, pos.y, z, ...);
}
```

**Verification:**
- `tests/map_storage.rs` — `spectator_z_range_skips_empty_floors`
- Benchmark: surface query with all creatures on z=7, scan 1 floor (was 8–10)

---

## MAP-11 — Spectator Vec has no capacity hint — P2 / Medium

**Hot path:** `spectator_conns_via_grid` (`game_world_spectators.rs:151`),
`collect_spatial_spectators` (`monster_events.rs:65`).

**Rust:**
```rust
let mut creature_ids: Vec<CreatureId> = Vec::new();  // No capacity
```

**Impact:**
- Reallocs as the Vec grows (1 → 2 → 4 → 8 → 16 → ...)
- Crowded areas (depots, raids): 5–6 reallocs per query
- Memory fragmentation under load

**Required change:**
```rust
let chunk_count = ((ck_x1 - ck_x0 + 1) * (ck_y1 - ck_y0 + 1)) as usize;
let z_count = Self::spectator_z_range(pos.z, multifloor).count();
let mut creature_ids: Vec<CreatureId> = Vec::with_capacity(chunk_count * z_count * 4);
```
Heuristic: 4 creatures per chunk-floor (tunable). Over-allocates slightly in sparse
areas, eliminates reallocs in dense areas.

**Verification:** benchmark in crowded depot scenario.

---

## MAP-12 — `walk_grid_line` allocates a Vec per LOS check — P2 / Medium

**Hot path:** `is_sight_clear` (`los.rs:10-46`).

**Rust:**
```rust
pub fn walk_grid_line(a: Position, b: Position) -> Vec<Position> {
    let mut out = Vec::new();
    // Bresenham loop
    out.push(Position { ... });
    out
}
```

**Impact:**
- 20-tile line → 20 `Position` structs (24 bytes each = 480 bytes) + Vec overhead
- Every LOS check allocates and deallocates
- Combat: every spell / wand / bolt LOS check

**Required change:**
Convert to an iterator pattern returning positions lazily:
```rust
pub fn walk_grid_line_iter(a: Position, b: Position) -> impl Iterator<Item = Position> {
    // Bresenham state in a struct, yield positions one at a time
}
```
Or pass a callback closure:
```rust
pub fn for_each_grid_line_point<F: FnMut(Position)>(a: Position, b: Position, mut f: F) {
    // Bresenham loop, call f(p) for each point
}
```
The closure version is the simplest drop-in: `is_sight_clear` becomes
```rust
for_each_grid_line_point(from, to, |p| {
    if p != from && p != to && self.blocks_sight(p) { return true; }  // early exit
    false
});
```

**Verification:**
- `tests/map_los.rs` — `is_sight_clear_no_allocation` (use `#[track_caller]` + allocator hook)
- Benchmark: 1000 LOS checks < 5ms (was ~15ms with per-check allocation)

---

## MAP-13 — Sparse chunk memory waste — P3 / Low

**Rust:** 64×64 chunk = 4096 slots × 8 bytes (pointer) = 32 KB pointer array per
chunk, even if only 1 tile is populated.

**772:** 32×32 sector = 1024 cells × 8 bytes = 8 KB per sector. 4× less waste per
sparsely-populated region.

**Impact:**
- 10% tile density: 25 chunks × 32 KB = 800 KB pointer arrays for 100k tiles
- 772 equivalent: ~200 KB
- 600 KB waste on a 100k-tile map — not a scaling problem on modern hardware

**Required change:** defer. Only revisit if memory profiling shows chunk pointer arrays
are a significant fraction of resident memory. Options if needed:
- Reduce to 32×32 chunks (4× less waste, matches 772 sector size)
- Hybrid: dense array for high-density chunks, `HashMap<tile_index, Box<Tile>>` for
  low-density chunks

**Verification:** none — defer until memory profiling justifies the change.

---

## MAP-14 — Missing player-only creature list — P3 / Low

**Rust:** single `Chunk.creatures` list; player-only queries (most broadcasts) filter
after collection.

**1098:** `QTreeLeafNode` keeps separate `creature_list` and `player_list`
(`src/map.h:144-149`). Player-only spectator queries walk `player_list` directly.

**772:** no separate player list — `TFindCreatures` filters by `Mask & 0x01` (PLAYER)
during traversal (`crmain.cc:140`).

**Impact:**
- Player-only fan-out (most broadcasts) collects all creatures then filters
- In monster-heavy areas (raids), 90% of collected creatures are filtered out

**Required change:**
Add `players: SmallVec<[CreatureId; 2]>` to `Chunk`, maintained in
`register_creature` / `unregister_creature` (check `CreatureKind::Player`). Player-only
queries walk `players` instead of `creatures`.

**Trade-off:** dual-list maintenance (similar to the tile/chunk creature list split,
audit #7). Add a `debug_assert_players_subset_of_creatures` check.

**Verification:** defer until monster-heavy raid profiling shows player fan-out is hot.

---

## MAP-15 — `SmallVec<[CreatureId; 4]>` capacity may be too small — P3 / Low

**Rust:** `Chunk.creatures: SmallVec<[CreatureId; 4]>` (`grid.rs:55`).

**Impact:**
- Crowded chunks (depots, spawn points): 5+ creatures → heap spill
- Each spilled chunk has its own heap allocation

**Required change:** profile actual chunk creature counts in production. If 90th
percentile > 4, bump to `SmallVec<[CreatureId; 8]>` (16 bytes more per chunk inline,
eliminates most spills).

**Verification:** defer until production profiling data is available.

---

## Phased implementation plan

Ordered by hot-path frequency × per-call cost. Each phase is independently shippable
and testable. Follow `tfs-cpp-references.md`: every changed behavior cites its C++
source in the module header.

### Phase A — Per-tick amplification (P0, small, isolated)

Targets findings **MAP-1, MAP-3**.

1. **Single-pass spectator union (MAP-1).** Add `SparseGrid::collect_spectators_dedup`
   that takes a `&mut FxHashSet<CreatureId>` and `&mut Vec<CreatureId>`, skipping
   already-seen creatures during the chunk walk. Rewrite
   `spectator_conns_via_grid` and `monsters_witnessing_move` to call it once per
   (old_pos, new_pos) pair, with a single sort at the end. Eliminates 3 of 4
   sort+dedup passes per move.
2. **Summons list on master (MAP-3).** Add `summons: SmallVec<[CreatureId; 2]>` to
   `CreatureBase` (or `Monster`). Maintain in `summon_creature` /
   `remove_creature`. `remove_creature` drains the master's list instead of scanning
   all creatures. Keep the full scan as a `debug_assert!`-gated fallback.
3. **Tests:**
   - `tests/map_storage.rs` — `spectators_union_single_sort_dedup`,
     `spectators_union_no_duplicate_when_old_new_overlap`
   - `tests/monster_ai.rs` — `remove_creature_drains_master_summons_list`,
     `remove_creature_falls_back_to_scan_in_debug`
4. **Verify:** `cargo test -p tfs-rust-core`, `cargo test -p tfs-rust-net`,
   `cargo clippy`. Flamegraph: sort/dedup CPU time drops ~75% under mass movement.

**Exit criteria:** per-move fan-out does one sort + one dedup; death of a summoning
master is O(summons) not O(all creatures).

### Phase B — Hot-path scans (P0–P1, medium)

Targets findings **MAP-2, MAP-4, MAP-6**.

1. **Reverse item index (MAP-2).** Add `item_positions: FxHashMap<ItemId, Position>`
   to `Map` (or `GameWorld`). Maintain in `internal_add_item_to_tile`,
   `internal_remove_item_from_tile`, item move between tiles, and OTBM load.
   `find_item_position` checks the reverse index first, falls back to full-world scan
   for items not in the index (containers, inventory).
2. **Chunk reference caching in LOS (MAP-6).** Refactor `blocks_sight` /
   `column_blocks_throw` to accept a `&mut Option<(ChunkKey, &Chunk)>` cache. Rewrite
   `is_sight_clear` and `throw_possible` inner loops to reuse the cached chunk
   reference across tiles in the same chunk. Re-hash only on chunk boundary.
3. **`scan_min_terrain_waypoints` chunk caching (MAP-4).** Cache the `&Chunk`
   reference across the inner `dx` loop. Re-hash only when `x % CHUNK_SIZE == 0`.
   Pass the tile reference to `ground_cost` instead of the position (eliminates
   callback's redundant `get_tile`).
4. **Tests:**
   - `tests/map_storage.rs` — `find_item_position_uses_reverse_index`,
     `reverse_index_invalidates_on_item_remove`
   - `tests/map_los.rs` — `is_sight_clear_cached_chunk_no_behavior_change`,
     `throw_possible_cached_chunk_matches_uncached`
   - `tests/pathfinding.rs` — `scan_min_terrain_waypoints_cached_chunk`
5. **Verify:** `cargo test -p tfs-rust-core`, `cargo clippy`. Benchmark: 20-tile LOS
   check < 5µs, 441-tile waypoint scan < 50µs, 100k-tile find_item < 1µs.

**Exit criteria:** no per-tile hash lookup in LOS / throw / waypoint scan loops;
`find_item_position` is O(1) for tile-resident items.

### Phase C — Load-time and per-repath allocation (P1, medium)

Targets findings **MAP-7, MAP-8**.

1. **`SmallVec` for tile stacks (MAP-7).** Switch `TileBody.down_items`,
   `top_items`, `creatures` from `Vec` to `SmallVec<[T; 2]>`. Update all callers
   (API-compatible). Verify `map_object_chain` still renders in insertion order.
2. **`TShortway` buffer pooling (MAP-8).** Add a `Vec<Box<[TShortwayCell; 529]>>`
   pool to `GameWorld` (or thread-local). `path_matching_tshortway` borrows a buffer,
   `clear()`s it, uses it, returns it. Eliminates per-search allocation.
3. **Tests:**
   - `tests/map_storage.rs` — `tile_body_smallvec_no_alloc_for_empty_tile`,
     `tile_body_smallvec_grows_to_heap_for_3_items`
   - `tests/pathfinding.rs` — `tshortway_reuses_pooled_buffer`,
     `tshortway_pooled_buffer_cleared_between_uses`
4. **Verify:** `cargo test -p tfs-rust-core`, `cargo clippy`. Benchmark: 100k-tile
   map load < 500ms, 1000 reverse path searches < 50ms.

**Exit criteria:** map load allocates 0 `Vec`s for empty tiles; reverse path search
allocates 0 buffers per search (pooled).

### Phase D — Parity: 16×16 creature blocks (P1, larger effort)

Targets finding **MAP-5**.

1. **`CreatureBlockIndex` (MAP-5).** Add a separate 16×16 creature block index,
   `FxHashMap<BlockKey, SmallVec<[CreatureId; 2]>>`, maintained alongside the chunk
   creature list. `BlockKey` packs `(z, block_x, block_y)` into a u32 (same scheme as
   `ChunkKey`).
2. **`collect_spectators_16x16`.** Walk 16×16 blocks in row-major order (blockx
   inner, blocky outer), matching `TFindCreatures::getNext` (`crmain.cc:101-144`).
   Replace `collect_spectators` calls in `spectator_conns_via_grid`,
   `collect_spatial_spectators`, `monsters_witnessing_move`.
3. **Remove GL#24 tie-break divergence.** With 16×16 block order matching 772, the
   SlotMap-key sort fallback in `monsters_witnessing_move` can be replaced with
   block-order traversal (or kept as a stable secondary sort, but block order becomes
   primary).
4. **Tests:**
   - `tests/map_storage.rs` — `collect_spectators_16x16_matches_772_block_order`,
     `creature_block_index_stays_in_sync_with_chunk_list`
   - `tests/monster_events.rs` — `monsters_witnessing_move_uses_block_order`,
     `monster_move_stimulus_tie_break_matches_772_trace`
5. **Verify:** `cargo test -p tfs-rust-core`, `cargo clippy`. Parity trace: compare
   fan-out order against 772 `TFindCreatures::getNext` trace for a fixed scenario.

**Exit criteria:** spectator fan-out walks 16×16 blocks in 772 order; tie-break
divergence (GL#24) resolved; over-collection reduced 16×.

### Phase E — Tuning (P2–P3, defer until profiling)

Targets findings **MAP-9, MAP-10, MAP-11, MAP-12, MAP-13, MAP-14, MAP-15**.

Defer all Phase E items until production profiling data is available. Each item is a
small, isolated change with no parity impact:

- **MAP-9:** tile creature `HashMap` index — only if depot tiles are hot.
- **MAP-10:** per-floor creature count guard — only if surface queries are hot.
- **MAP-11:** spectator Vec capacity hint — only if reallocs show in profiling.
- **MAP-12:** `walk_grid_line` iterator — only if LOS allocation shows in profiling.
- **MAP-13:** 32×32 chunks — only if memory profiling shows chunk pointer arrays are
  significant.
- **MAP-14:** player-only creature list — only if raid player fan-out is hot.
- **MAP-15:** `SmallVec` capacity bump — only if production chunk creature counts
  exceed 4.

**Exit criteria:** each item addressed only when its hot path is confirmed by
profiling; no speculative changes.

---

## Cross-references

- **`docs/MAP_SYSTEM_AUDIT.md`** — correctness fixes #1–#9 (Phases 1–3 ✅). This doc
  is the performance/parity successor.
- **`docs/GAME_LOOP_DECAY_IDLE_TODO_PERFORMANCE_AUDIT.md`** — game loop, decay, idle
  stimulus, ToDo heap. MAP-1 (per-move fan-out) amplifies the same per-tick budget
  that doc's GL-2 (unbounded ingress) and IDLE-3 (target/move search sort+dedup)
  address.
- **`tasks/lessons.md`** §205 — live timing notes that motivated this audit.
- **`tasks/todo.md`** — tracking entries for Phases A–E.

## Citations

### 772 reference (outcomes)
- `reference/cipsoft-772/tibia-game-master/src/map.hh:74-79` — `TSector` (32×32 tile sector)
- `reference/cipsoft-772/tibia-game-master/src/crmain.cc:59-144` — `TFindCreatures` (16×16 creature blocks, row-major walk)
- `reference/cipsoft-772/tibia-game-master/src/crmain.cc:995-1073` — `InsertChainCreature` / `DeleteChainCreature` / `MoveChainCreature`
- `reference/cipsoft-772/tibia-game-master/src/info.cc:1154-1212` — `ThrowPossible` (LOS / throw line walk)
- `reference/cipsoft-772/tibia-game-master/src/cract.cc:51-149` — `TShortway::TShortway` / `FillMap` / `ClearMap`
- `reference/cipsoft-772/tibia-game-master/src/operate.cc:922-962` — `NotifyAllCreatures` (move stimulus fan-out)

### 1098 / TFS reference (domain shape)
- `src/map.cpp:365-474` — `Map::getSpectatorsInternal` / `Map::getSpectators` (quadtree + multifloor)
- `src/map.h:62-150` — `QTreeLeafNode` (separate `creature_list` / `player_list`)
- `src/creature.h` — `Creature::summons` (master-side summons list)

### Rust implementation
- `crates/tfs-rust-core/src/map/grid.rs:1-379` — `SparseGrid`, `Chunk`, `ChunkKey`, `collect_spectators`, `find_item_position`
- `crates/tfs-rust-core/src/map/mod.rs:1-475` — `Map`, OTBM ingest, `register_creature_at` / `unregister_creature_at`
- `crates/tfs-rust-core/src/map/los.rs:1-178` — `walk_grid_line`, `is_sight_clear`, `throw_possible`, `column_blocks_throw`
- `crates/tfs-rust-core/src/tile.rs:66-76, 171-182` — `TileBody`, `add_creature` / `remove_creature`
- `crates/tfs-rust-core/src/game_world_spectators.rs:100-211` — `spectator_conns_via_grid`, `spectator_players_in_box`
- `crates/tfs-rust-core/src/monster_events.rs:40-133` — `spectator_z_range`, `collect_spatial_spectators`, `monsters_witnessing_move`
- `crates/tfs-rust-core/src/pathfinding.rs:40-83, 672-684` — `scan_min_terrain_waypoints`, `TShortway` buffer allocation
- `crates/tfs-rust-core/src/game_world_item_cylinder.rs:216-393` — `internal_add_item_to_tile`, `internal_remove_item_from_tile`
- `crates/tfs-rust-core/src/game_world_lifecycle.rs:47-52` — `remove_creature` summons scan

### Audit documentation
- `docs/MAP_SYSTEM_AUDIT.md` — correctness fixes #1–#9 (Phases 1–3 ✅)
- `docs/GAME_LOOP_DECAY_IDLE_TODO_PERFORMANCE_AUDIT.md` — game loop / decay / idle / ToDo performance
- `tasks/lessons.md` — engine architecture & implementation lessons
- `tasks/todo.md` — project tracking
