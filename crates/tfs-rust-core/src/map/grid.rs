//! Lazy 64×64 chunk grid — tiles + per-chunk creature spatial index.
//!
//! Replaces `HashMap<Position, Tile>` and `QTreeNode` (`map.cpp` lazy spatial index outcomes).
// C++ reference: `map.cpp` `Map::getSpectators`, tile storage (sparse world).

use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use tfs_rust_common::Position;

use crate::ids::CreatureId;
use crate::tile::Tile;

pub const CHUNK_SIZE: u16 = 64;
pub const CHUNK_AREA: usize = (CHUNK_SIZE as usize) * (CHUNK_SIZE as usize);
/// 772 `TFindCreatures` block size (`crmain.cc` `blockx` / `blocky`).
pub const SECTOR_SIZE: u16 = 16;

/// Packed `(floor, chunk_x, chunk_y)` — `FxHashMap` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ChunkKey(u32);

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
        let z = (self.0 >> 20) as u8;
        (cx * CHUNK_SIZE, cy * CHUNK_SIZE, z)
    }
}

#[inline]
fn tile_index(x: u16, y: u16) -> usize {
    let lx = (x % CHUNK_SIZE) as usize;
    let ly = (y % CHUNK_SIZE) as usize;
    ly * CHUNK_SIZE as usize + lx
}

#[inline]
fn position_from_chunk_slot(origin_x: u16, origin_y: u16, z: u8, idx: usize) -> Position {
    let lx = (idx % CHUNK_SIZE as usize) as u16;
    let ly = (idx / CHUNK_SIZE as usize) as u16;
    Position::new(origin_x + lx, origin_y + ly, z)
}

/// One 64×64 region on a single floor.
#[derive(Debug)]
pub(crate) struct Chunk {
    pub tile_count: u16,
    pub creatures: SmallVec<[CreatureId; 4]>,
    pub tiles: Box<[Option<Box<Tile>>; CHUNK_AREA]>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            tile_count: 0,
            creatures: SmallVec::new(),
            tiles: Box::new(std::array::from_fn(|_| None)),
        }
    }
}

/// Sparse tile store + chunk-level creature index (replaces quadtree + `HashMap<Position, Tile>`).
#[derive(Debug, Default)]
pub struct SparseGrid {
    chunks: FxHashMap<ChunkKey, Box<Chunk>>,
}

impl SparseGrid {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn populated_tile_count(&self) -> usize {
        self.chunks
            .values()
            .map(|c| usize::from(c.tile_count))
            .sum()
    }

    pub fn tile_stack_item_refs(&self) -> usize {
        self.chunks
            .values()
            .flat_map(|c| c.tiles.iter())
            .filter_map(|slot| slot.as_deref())
            .map(|t| {
                let b = t.body();
                b.down_items.len() + b.top_items.len()
            })
            .sum()
    }

    pub fn get_tile(&self, x: u16, y: u16, z: u8) -> Option<&Tile> {
        let key = ChunkKey::from_pos(x, y, z);
        self.chunks.get(&key)?.tiles[tile_index(x, y)].as_deref()
    }

    pub fn get_tile_mut(&mut self, x: u16, y: u16, z: u8) -> Option<&mut Tile> {
        let key = ChunkKey::from_pos(x, y, z);
        self.chunks.get_mut(&key)?.tiles[tile_index(x, y)].as_deref_mut()
    }

    pub fn insert_tile(&mut self, x: u16, y: u16, z: u8, tile: Tile) {
        let key = ChunkKey::from_pos(x, y, z);
        let chunk = self
            .chunks
            .entry(key)
            .or_insert_with(|| Box::new(Chunk::new()));
        let idx = tile_index(x, y);
        if chunk.tiles[idx].is_none() {
            chunk.tile_count += 1;
        }
        chunk.tiles[idx] = Some(Box::new(tile));
    }

    /// Chunk spatial list only — does not allocate a chunk (tile must exist first).
    ///
    /// `pub(super)`: all creature placement must funnel through `Map::register_creature_at`
    /// so the dual `TileBody.creatures` / `Chunk.creatures` lists stay in sync (audit #7).
    // C++ reference: `map.cpp` `Map::moveCreature` creature-list bookkeeping.
    pub(super) fn register_creature(&mut self, x: u16, y: u16, z: u8, id: CreatureId) {
        let key = ChunkKey::from_pos(x, y, z);
        let Some(chunk) = self.chunks.get_mut(&key) else {
            return;
        };
        if !chunk.creatures.contains(&id) {
            chunk.creatures.push(id);
        }
    }

    /// `pub(super)`: see [`SparseGrid::register_creature`] — route through
    /// `Map::unregister_creature_at` to keep the dual lists in sync (audit #7).
    pub(super) fn unregister_creature(&mut self, x: u16, y: u16, z: u8, id: CreatureId) {
        let key = ChunkKey::from_pos(x, y, z);
        let Some(chunk) = self.chunks.get_mut(&key) else {
            return;
        };
        chunk.creatures.retain(|c| *c != id);
        if chunk.creatures.is_empty() && chunk.tile_count == 0 {
            self.chunks.remove(&key);
        }
    }

    /// Debug-only dual-list consistency check (audit #7).
    ///
    /// Verifies every `Chunk.creatures` entry is on some tile's `TileBody.creatures` list
    /// within that chunk, and vice versa. All assertions are `debug_assert!` so the entire
    /// body compiles out in release builds — safe to call from test harnesses.
    pub fn debug_assert_creature_lists_agree(&self) {
        for (key, chunk) in &self.chunks {
            // Every chunk-list creature must be on some tile in this chunk.
            for &cid in &chunk.creatures {
                let on_tile = chunk.tiles.iter().any(|slot| {
                    slot.as_deref()
                        .map(|t| t.body().creatures.contains(&cid))
                        .unwrap_or(false)
                });
                debug_assert!(
                    on_tile,
                    "creature {:?} in chunk {:?} spatial list but not on any tile",
                    cid, key
                );
            }
            // Every tile-list creature must be in the chunk spatial list.
            let (ox, oy, z) = key.chunk_origin();
            for (idx, slot) in chunk.tiles.iter().enumerate() {
                let Some(tile) = slot else { continue };
                let body = tile.body();
                if body.creatures.is_empty() {
                    continue;
                }
                let lx = (idx % CHUNK_SIZE as usize) as u16;
                let ly = (idx / CHUNK_SIZE as usize) as u16;
                let pos = Position::new(ox + lx, oy + ly, z);
                for &cid in &body.creatures {
                    debug_assert!(
                        chunk.creatures.contains(&cid),
                        "creature {:?} on tile {:?} missing from chunk spatial list",
                        cid,
                        pos
                    );
                }
            }
        }
    }

    /// Spatial **superset** for spectator fan-out — chunk overlap only; callers filter with `canSee`.
    // C++ reference: `Map::getSpectators` — `map.cpp` ~386–474.
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
                let key = ChunkKey::from_pos(chunk_x * CHUNK_SIZE, chunk_y * CHUNK_SIZE, z);
                if let Some(chunk) = self.chunks.get(&key) {
                    out.extend_from_slice(&chunk.creatures);
                }
            }
        }
    }

    /// Viewport creatures in 772 `TFindCreatures::getNext` 16×16 sector order (`crmain.cc:101–144`).
    ///
    /// Walks `blocky` outer / `blockx` inner over sectors covering the XY box, then tiles
    /// within each sector (y outer, x inner) appending each tile's creature list.
    /// Exact LIFO `NextChainCreature` within a sector would need per-sector chains; tile-list
    /// order is the deterministic stand-in (IDLE-3).
    pub fn collect_spectators_sector_order(
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

        let bx0 = x0 / SECTOR_SIZE;
        let by0 = y0 / SECTOR_SIZE;
        let bx1 = x1 / SECTOR_SIZE;
        let by1 = y1 / SECTOR_SIZE;

        for by in by0..=by1 {
            for bx in bx0..=bx1 {
                let sx0 = bx * SECTOR_SIZE;
                let sy0 = by * SECTOR_SIZE;
                let sx1 = sx0 + SECTOR_SIZE - 1;
                let sy1 = sy0 + SECTOR_SIZE - 1;
                let tx0 = sx0.max(x0);
                let ty0 = sy0.max(y0);
                let tx1 = sx1.min(x1);
                let ty1 = sy1.min(y1);
                for y in ty0..=ty1 {
                    for x in tx0..=tx1 {
                        if let Some(tile) = self.get_tile(x, y, z) {
                            let creatures = &tile.body().creatures;
                            if !creatures.is_empty() {
                                out.extend_from_slice(creatures);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn find_item_position(&self, item_id: crate::ids::ItemId) -> Option<Position> {
        for (key, chunk) in &self.chunks {
            let (ox, oy, z) = key.chunk_origin();
            for (idx, slot) in chunk.tiles.iter().enumerate() {
                if let Some(tile) = slot
                    && tile.has_item(item_id)
                {
                    return Some(position_from_chunk_slot(ox, oy, z, idx));
                }
            }
        }
        None
    }

    pub fn for_each_tile(&self, mut f: impl FnMut(Position, &Tile)) {
        for (key, chunk) in &self.chunks {
            let (ox, oy, z) = key.chunk_origin();
            for (idx, slot) in chunk.tiles.iter().enumerate() {
                if let Some(tile) = slot.as_deref() {
                    f(position_from_chunk_slot(ox, oy, z, idx), tile);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::{Key, SlotMap};

    use crate::ids::ItemId;
    use crate::tile::TileBody;

    #[test]
    fn chunk_key_roundtrip_origin() {
        let key = ChunkKey::from_pos(100, 200, 7);
        let (ox, oy, z) = key.chunk_origin();
        assert_eq!(ox, 64);
        assert_eq!(oy, 192);
        assert_eq!(z, 7);
    }

    #[test]
    fn collect_spectators_only_hits_overlapping_chunks() {
        let mut grid = SparseGrid::new();
        let mut items: SlotMap<ItemId, _> = SlotMap::with_key();
        let item = items.insert(crate::item::Item::new_single(100));
        let tile = crate::tile::Tile::Normal(TileBody {
            ground: Some(100),

            ground_item: None,
            down_items: vec![item],
            top_items: vec![],
            creatures: vec![],
            flags: 0,
            zone: tfs_rust_common::ZoneType::Normal,
        });
        grid.insert_tile(70, 70, 7, tile);

        let mut c1 = SlotMap::<CreatureId, ()>::with_key();
        let id1 = c1.insert(());
        grid.register_creature(70, 70, 7, id1);

        let mut out = Vec::new();
        grid.collect_spectators(70, 70, 7, 11, 11, &mut out);
        assert!(out.contains(&id1));

        out.clear();
        grid.collect_spectators(0, 0, 7, 5, 5, &mut out);
        assert!(!out.contains(&id1));
    }

    /// IDLE-3: adjacent 16×16 sectors inside one 64×64 chunk emit in sector order, not
    /// SlotMap-key (creation) order.
    #[test]
    fn collect_spectators_sector_order_not_slotmap_key_order() {
        let mut grid = SparseGrid::new();
        let mut sm: SlotMap<CreatureId, ()> = SlotMap::with_key();

        // Insert A first (lower SlotMap key), place it in the *later* sector (x=16).
        // Insert B second (higher key), place it in the *earlier* sector (x=0).
        // Same 64×64 chunk (origin 0,0); center (8,8) range 16 covers both.
        let id_a = sm.insert(());
        let id_b = sm.insert(());
        assert!(
            id_a.data().as_ffi() < id_b.data().as_ffi(),
            "precondition: A must have lower SlotMap key than B"
        );

        for (x, y, id) in [(16u16, 8u16, id_a), (0u16, 8u16, id_b)] {
            let tile = crate::tile::Tile::Normal(TileBody {
                ground: Some(100),

                ground_item: None,
                down_items: vec![],
                top_items: vec![],
                creatures: vec![id],
                flags: 0,
                zone: tfs_rust_common::ZoneType::Normal,
            });
            grid.insert_tile(x, y, 7, tile);
            grid.register_creature(x, y, 7, id);
        }

        let mut out = Vec::new();
        grid.collect_spectators_sector_order(8, 8, 7, 16, 16, &mut out);
        assert_eq!(
            out,
            vec![id_b, id_a],
            "sector (0,0) must emit before sector (1,0); got {out:?}"
        );

        // SlotMap-key sort would be [A, B] — prove that differs.
        let mut by_key = out.clone();
        by_key.sort_by_key(|id| id.data().as_ffi());
        assert_eq!(by_key, vec![id_a, id_b]);
        assert_ne!(out, by_key);
    }

    /// Audit #7 — a creature in `Chunk.creatures` but not on any tile's `TileBody.creatures`
    /// must trip `debug_assert_creature_lists_agree`. Debug-only (`debug_assert!` compiles
    /// out in release).
    #[cfg(debug_assertions)]
    #[test]
    fn debug_assert_catches_chunk_list_creature_not_on_tile() {
        let mut grid = SparseGrid::new();
        let tile = crate::tile::Tile::Normal(TileBody {
            ground: Some(100),

            ground_item: None,
            down_items: vec![],
            top_items: vec![],
            creatures: vec![],
            flags: 0,
            zone: tfs_rust_common::ZoneType::Normal,
        });
        grid.insert_tile(70, 70, 7, tile);

        let mut sm: SlotMap<CreatureId, ()> = SlotMap::with_key();
        let orphan = sm.insert(());

        // Corrupt: push into the chunk spatial list without touching any tile list.
        let key = ChunkKey::from_pos(70, 70, 7);
        grid.chunks.get_mut(&key).unwrap().creatures.push(orphan);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            grid.debug_assert_creature_lists_agree();
        }));
        assert!(
            result.is_err(),
            "debug_assert_creature_lists_agree must catch a chunk-list creature not on any tile"
        );
    }

    /// Audit #7 — a creature on a tile's `TileBody.creatures` list but missing from the
    /// chunk spatial list must trip `debug_assert_creature_lists_agree`. Debug-only.
    #[cfg(debug_assertions)]
    #[test]
    fn debug_assert_catches_tile_list_creature_not_in_chunk() {
        let mut grid = SparseGrid::new();
        let mut sm: SlotMap<CreatureId, ()> = SlotMap::with_key();
        let orphan = sm.insert(());

        let tile = crate::tile::Tile::Normal(TileBody {
            ground: Some(100),

            ground_item: None,
            down_items: vec![],
            top_items: vec![],
            creatures: vec![orphan],
            flags: 0,
            zone: tfs_rust_common::ZoneType::Normal,
        });
        grid.insert_tile(70, 70, 7, tile);
        // Corrupt: remove from the chunk spatial list (insert_tile did not add it, and we
        // deliberately skip register_creature).
        let key = ChunkKey::from_pos(70, 70, 7);
        assert!(
            !grid.chunks.get(&key).unwrap().creatures.contains(&orphan),
            "precondition: orphan must not be in chunk list"
        );

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            grid.debug_assert_creature_lists_agree();
        }));
        assert!(
            result.is_err(),
            "debug_assert_creature_lists_agree must catch a tile-list creature missing from the chunk"
        );
    }

    /// Audit #7 — a clean grid must NOT trip the consistency check. The tile list and chunk
    /// list must both hold the creature (the `*_at` seam keeps them in sync; here we mirror
    /// that by inserting a tile whose `creatures` list already contains the id, then syncing
    /// the chunk list via `register_creature`).
    #[test]
    fn debug_assert_passes_on_clean_grid() {
        let mut grid = SparseGrid::new();
        let mut sm: SlotMap<CreatureId, ()> = SlotMap::with_key();
        let id = sm.insert(());

        let tile = crate::tile::Tile::Normal(TileBody {
            ground: Some(100),

            ground_item: None,
            down_items: vec![],
            top_items: vec![],
            creatures: vec![id],
            flags: 0,
            zone: tfs_rust_common::ZoneType::Normal,
        });
        grid.insert_tile(70, 70, 7, tile);
        grid.register_creature(70, 70, 7, id);
        // No panic expected in either build.
        grid.debug_assert_creature_lists_agree();
    }
}
