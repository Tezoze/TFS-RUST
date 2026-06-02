# Memory usage analysis — Phase B: Sparse spatial hash grid

This file records the **Phase B** implementation spec (sparse chunk grid). Phase A (baseline RSS breakdown) should be captured in server diagnostics before/after merge; see the migration checklist for metrics to log.

**Goal:** Replace `QTreeNode::build` (multi-GB eager tree) + `HashMap<Position, Tile>` (SipHash, duplicate `Position` key) with a unified lazy chunk grid that achieves **the same observable outcomes** as TFS 1.4.2’s lazy quadtree (spectator superset, lazy allocation, low RSS) using idiomatic Rust — **not** the same internal structure as C++ `QTreeNode`.

**Replaces:**
- `crates/tfs-rust-core/src/map/qtree.rs` — entire module
- `map/mod.rs:47–55` (`QTreeNode::build` at load)
- `map/mod.rs:300–307` (`register_creature_index` eager per-floor build)
- `map/mod.rs:29` (`HashMap<Position, Tile>` tile store)

---

## Constants

```rust
// map/grid.rs
pub const CHUNK_SIZE: u16 = 64;
pub const CHUNK_AREA: usize = (CHUNK_SIZE as usize) * (CHUNK_SIZE as usize); // 4096
```

64×64 is the modern sweet spot for Tibia:

- Spectator view radius (~11–15 tiles) touches **1–4 chunks** instead of 4–9 at 32×32.
- Fewer `HashMap` lookups per query = measurably lower overhead on the hottest path.
- `CHUNK_SIZE` is a `const` — flip to 128 after benchmarks if hash pressure warrants it.

**Chunk memory:** Each chunk that exists pays **32 KB** for the tile pointer array (`4096 × 8` bytes), regardless of how many of those slots are populated. At Forgotten scale (~7.8 M OTBM tiles), expect on the order of **~1.9k–80k+ chunks** depending on geographic clustering (not one chunk per tile). ~2k chunks ≈ **~60 MB** of slot arrays alone, plus heap `Tile` bodies — still far below eager full-map quadtrees.

---

## ChunkKey

```rust
/// Packed (floor, chunk_x, chunk_y) — the HashMap key.
/// floor:   4 bits  (0–15)
/// chunk_x: 10 bits (0–1023 for 65535/64)
/// chunk_y: 10 bits
/// Total: 24 bits, fits u32, zero padding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkKey(u32);

impl ChunkKey {
    #[inline]
    pub fn from_pos(x: u16, y: u16, z: u8) -> Self {
        let cx = (x / CHUNK_SIZE) as u32;
        let cy = (y / CHUNK_SIZE) as u32;
        ChunkKey((z as u32) << 20 | cy << 10 | cx)
    }

    #[inline]
    pub fn chunk_origin(self) -> (u16, u16, u8) {
        let cx = (self.0 & 0x3FF) as u16;
        let cy = ((self.0 >> 10) & 0x3FF) as u16;
        let z  = (self.0 >> 20) as u8;
        (cx * CHUNK_SIZE, cy * CHUNK_SIZE, z)
    }
}

/// World position from grid coordinates (inverse of key + index).
#[inline]
pub fn position_at(x: u16, y: u16, z: u8) -> Position {
    Position::new(x, y, z)
}

/// Tile index within a chunk — derived from position, never stored.
#[inline]
fn tile_index(x: u16, y: u16) -> usize {
    let lx = (x % CHUNK_SIZE) as usize;
    let ly = (y % CHUNK_SIZE) as usize;
    ly * CHUNK_SIZE as usize + lx
}
```

`Position` is no longer a `HashMap` key anywhere on the hot path.  
`tile_index` is the only arithmetic needed to go from world coords to array slot.

---

## Chunk structure — hot/cold separation

```rust
use smallvec::SmallVec;

/// One 64×64 region on a single floor.
///
/// **Layout rationale:**
/// `creatures` is on the spectator hot path (every tick, every player).
/// `tiles` is on the map-read cold path (movement validation, item lookup).
/// Keeping them separate means spectator queries never touch the tile heap allocation.
pub struct Chunk {
    /// O(1) empty-chunk test for pruning — never scan 4096 slots on unregister.
    pub tile_count: u16,

    /// Chunk-level creature index (spectator / spatial queries).
    /// Must stay in sync with per-tile `Tile::creatures` — see **Dual creature index** below.
    pub creatures: SmallVec<[CreatureId; 4]>,

    /// `Tile::Normal` | `Tile::House` — same enum as today (`tile.rs`).
    /// None for empty slots; `Box` keeps the slot array at 8 bytes per cell.
    pub tiles: Box<[Option<Box<Tile>>; CHUNK_AREA]>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            tile_count: 0,
            creatures: SmallVec::new(),
            // No `unsafe` — one-time init when a chunk is first touched.
            tiles: Box::new(std::array::from_fn(|_| None)),
        }
    }
}
```

### Why `Option<Box<Tile>>` not `Option<Tile>`

`Tile` is `Normal(TileBody)` or `House(HouseTile)`; `TileBody` has three `Vec` fields plus ground, flags, zone — roughly **96+ bytes** inline. `[Option<Tile>; 4096]` ≈ **~400 KB per chunk**, which is not cache-friendly for a sparse world.

`Option<Box<Tile>>` is pointer-sized (8 bytes); the array stays at **32 KB**. Heap allocation happens only for tiles that actually exist (~7.8 M across Forgotten).

### Why `SmallVec<[CreatureId; 4]>`

Most chunks contain 0–3 creatures. `SmallVec<[T; 4]>` stores up to 4 inline (no heap), spills in towns/boss rooms. Spectator scans are stack-local in the common case.

### Dual creature index (invariant)

TFS keeps creatures on the **tile stack** and in the **spatial index**. This design does the same:

| Index | Where | Used for |
|-------|--------|----------|
| Per-tile | `Tile::creatures()` / `TileBody.creatures` | Movement, stack order, cylinder logic |
| Per-chunk | `Chunk.creatures` | `collect_spectators`, move/appear fan-out |

**Rule:** Every path that adds/removes/moves a creature on a tile must update **both** lists (via a single `Map::register_creature_at` / `unregister_creature_at` helper). Do not call `Chunk::creatures` and tile lists from unrelated call sites.

---

## SparseGrid

```rust
use rustc_hash::FxHashMap; // ChunkKey is internal u32 — not attacker-controlled input

/// Replaces both the QTree (spectator/creature index) and `HashMap<Position, Tile>`.
pub struct SparseGrid {
    chunks: FxHashMap<ChunkKey, Box<Chunk>>,
}

impl SparseGrid {
    pub fn new() -> Self {
        Self { chunks: FxHashMap::default() }
    }

    // ── Tile access ─────────────────────────────────────────────────────────

    pub fn get_tile(&self, x: u16, y: u16, z: u8) -> Option<&Tile> {
        let key = ChunkKey::from_pos(x, y, z);
        self.chunks.get(&key)?.tiles[tile_index(x, y)].as_deref()
    }

    pub fn get_tile_mut(&mut self, x: u16, y: u16, z: u8) -> Option<&mut Tile> {
        let key = ChunkKey::from_pos(x, y, z);
        self.chunks.get_mut(&key)?
            .tiles[tile_index(x, y)]
            .as_deref_mut()
    }

    /// Lazily create the chunk and tile slot (map load, dynamic tiles).
    pub fn get_or_create_tile(&mut self, x: u16, y: u16, z: u8) -> &mut Tile {
        let key = ChunkKey::from_pos(x, y, z);
        let chunk = self.chunks.entry(key).or_insert_with(|| Box::new(Chunk::new()));
        let idx = tile_index(x, y);
        if chunk.tiles[idx].is_none() {
            chunk.tiles[idx] = Some(Box::new(Tile::empty_normal()));
            chunk.tile_count += 1;
        }
        chunk.tiles[idx].as_deref_mut().expect("just inserted")
    }

    // ── Creature index ───────────────────────────────────────────────────────

    /// Register in the chunk spatial list only. Does **not** allocate a new chunk.
    /// Creatures always stand on an existing tile; chunks are created by OTBM load or
    /// `get_or_create_tile` first. Avoids 32 KB creature-only chunks in empty wilderness
    /// (the failure mode of `register_creature_index` + eager `QTreeNode::build`).
    pub fn register_creature(&mut self, x: u16, y: u16, z: u8, id: CreatureId) {
        let key = ChunkKey::from_pos(x, y, z);
        let Some(chunk) = self.chunks.get_mut(&key) else {
            return;
        };
        if !chunk.creatures.contains(&id) {
            chunk.creatures.push(id);
        }
    }

    pub fn unregister_creature(&mut self, x: u16, y: u16, z: u8, id: CreatureId) {
        let key = ChunkKey::from_pos(x, y, z);
        let Some(chunk) = self.chunks.get_mut(&key) else {
            return;
        };
        chunk.creatures.retain(|&c| c != id);
        if chunk.creatures.is_empty() && chunk.tile_count == 0 {
            self.chunks.remove(&key);
        }
    }

    // ── Spectator query ──────────────────────────────────────────────────────

    /// Spatial **superset** of creatures near `center` on floor `z`.
    ///
    /// Returns every `CreatureId` in chunks overlapping the axis-aligned view rectangle
    /// `[center_x ± range_x, center_y ± range_y]`. Does **not** filter by per-creature
    /// position (creatures near chunk edges may lie outside the true viewport). This matches
    /// current `QTreeNode::get_spectators` / C++ `Map::getSpectators` (leaf/chunk overlap only).
    ///
    /// Callers that need viewport-accurate sets must filter afterward — e.g.
    /// `collect_creature_spectators` → `creature_can_see` in `monster_ai.rs`.
    pub fn collect_spectators(
        &self,
        center_x: u16,
        center_y: u16,
        z: u8,
        range_x: u16,
        range_y: u16,
        out: &mut Vec<CreatureId>,
    ) {
        let x0 = center_x.saturating_sub(range_x);
        let y0 = center_y.saturating_sub(range_y);
        let x1 = center_x.saturating_add(range_x);
        let y1 = center_y.saturating_add(range_y);

        let ck_x0 = x0 / CHUNK_SIZE;
        let ck_y0 = y0 / CHUNK_SIZE;
        let ck_x1 = x1 / CHUNK_SIZE;
        let ck_y1 = y1 / CHUNK_SIZE;

        for chunk_y in ck_y0..=ck_y1 {
            for chunk_x in ck_x0..=ck_x1 {
                let key = ChunkKey::from_pos(
                    chunk_x * CHUNK_SIZE,
                    chunk_y * CHUNK_SIZE,
                    z,
                );
                if let Some(chunk) = self.chunks.get(&key) {
                    out.extend_from_slice(&chunk.creatures);
                }
            }
        }
    }
}
```

No `cached_spectators` clone. No per-leaf `Vec` duplication.  
The output `Vec` is caller-owned and reused across calls — zero allocations on the hot path after warmup.

### Chunk pruning policy

| Approach | Decision |
|----------|----------|
| Scan 4096 slots on unregister | **Rejected** — O(4096) on hot path |
| `tile_count` on `Chunk` | **Yes** — increment on first tile insert, decrement on tile remove |
| Prune when | `creatures.is_empty() && tile_count == 0` → `chunks.remove(key)` |
| Never prune | Acceptable only if creature-only chunks are never created (see `register_creature`) |
| Timer / lazy sweep | **Deferred** — unnecessary for Phase B |

When removing the last tile in a slot, decrement `tile_count`; if zero and no creatures, remove the chunk.

---

## Drop-in for `TileBody`

Remove the `position` field — coordinates are implicit from `ChunkKey` + `tile_index` (use `position_at(x, y, z)` at call sites).

Saves ~8 bytes per tile (~61 MB at Forgotten scale).

```rust
// tile.rs — remove TileBody.position (was tile.rs:54, duplicate of map key)
pub struct TileBody {
    pub ground:     Option<ItemId>,
    pub down_items: Vec<ItemId>,
    pub top_items:  Vec<ItemId>,
    pub creatures:  Vec<CreatureId>, // tile stack; kept in sync with Chunk.creatures
    pub flags:      u32,
    pub zone:       u16,
}

impl TileBody {
    pub fn new() -> Self {
        Self {
            ground:     None,
            down_items: Vec::new(),
            top_items:  Vec::new(),
            creatures:  Vec::new(),
            flags:      0,
            zone:       0,
        }
    }
}

impl Tile {
    pub fn empty_normal() -> Self {
        Tile::Normal(TileBody::new())
    }
}
```

---

## Map integration

```rust
// map/mod.rs — replace HashMap<Position, Tile> + qtrees HashMap<u8, QTreeNode>
pub struct Map {
    pub grid: SparseGrid,
    pub width:  u16,
    pub height: u16,
    // towns, waypoints unchanged
}

impl Map {
    pub fn from_map_data(data: MapData, items: &mut SlotMap<ItemId, Item>) -> Self {
        let mut map = Map {
            grid:   SparseGrid::new(),
            width:  data.width,
            height: data.height,
        };
        // No QTreeNode::build — chunks created per tile in load_tile / get_or_create_tile.
        for (pos, td) in data.tiles {
            map.load_tile(pos, td, items);
        }
        map
    }

    /// Single entry point: update chunk index + tile stack (walk, spawn, remove).
    pub fn register_creature_at(&mut self, pos: Position, id: CreatureId) { /* both indices */ }
    pub fn unregister_creature_at(&mut self, pos: Position, id: CreatureId) { /* both indices */ }
}
```

`register_creature_index` (`map/mod.rs:300–307`) is replaced by `register_creature_at` — no full-floor `QTreeNode::build`.

`collect_spatial_spectators` (`monster_ai.rs`) calls `grid.collect_spectators(center.x, center.y, z, MAP_MAX_VIEWPORT, MAP_MAX_VIEWPORT, &mut buf)`; `collect_creature_spectators` keeps `creature_can_see` filtering unchanged.

---

## Cargo dependencies

```toml
[dependencies]
# FxHashMap — fast for small integer keys (u32 ChunkKey); keys are not user-supplied strings
rustc-hash = "1"

# ahash is an alternative if the workspace standardizes on it
# ahash = "0.8"

smallvec = { version = "1", features = ["union"] }
```

---

## Memory estimate (Forgotten scale)

| Structure | Old | New |
|---|---|---|
| Quadtrees (floor 0 + per active floor) | **multi-GB** | **0** — eliminated |
| Tile storage | `HashMap<Position, Tile>` + SipHash + duplicate pos | `FxHashMap<ChunkKey, Box<Chunk>>` — ~32 KB/chunk slot array, lazy |
| Chunk creature lists | qtree leaf + `cached_spectators` clone | `SmallVec<[CreatureId; 4]>` inline |
| `TileBody.position` | ~8 B × 7.8 M ≈ **61 MB** | **0** — `position_at` |
| `cached_spectators` | clone per leaf per query | **0** |

Primary RSS win: quadtree elimination. Tile dedup and chunk layout are secondary but real.

---

## Migration checklist

- [x] Create `map/grid.rs` with `ChunkKey`, `tile_index`, `position_at`, `Chunk`, `SparseGrid`
- [x] `Chunk`: `tile_count`, `SmallVec` creatures, `Option<Box<Tile>>` slots, safe `Chunk::new`
- [x] Remove `map/qtree.rs`
- [x] Remove `qtrees: HashMap<u8, QTreeNode>` from `Map`
- [x] Replace `HashMap<Position, Tile>` with `grid: SparseGrid`
- [x] Remove `QTreeNode::build` in `from_map_data`
- [x] Add `register_creature_at` / `unregister_creature_at` (dual index + prune policy)
- [x] Remove `TileBody.position`; `TileBody::new()` without coordinates
- [x] Wire `collect_spatial_spectators` → `grid.collect_spectators` (`center_x`/`center_y`, document superset)
- [x] RSS diagnostic: `map_chunks`, `map_tiles`, `map_chunk_slots_per_chunk` in `run_server.rs`
- [x] Phase B shipped (June 2026)

---

## What this does NOT do (defer to Phase C)

- Lua VM merge (`run_server.rs:163` + `formulas.rs:464`)
- `Monster::registered_events` → bitflags
- `damage_map` / `spell_cooldown_end` lazy `Option<Box<HashMap>>`
- `DecayManager` time-bucketed `BTreeMap`
- Dead `SpawnManager::zones` field (`spawn.rs:48`)
