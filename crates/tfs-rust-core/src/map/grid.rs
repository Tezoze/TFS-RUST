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

/// Packed `(floor, chunk_x, chunk_y)` — `FxHashMap` key.
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
        let z = (self.0 >> 20) as u8;
        (cx * CHUNK_SIZE, cy * CHUNK_SIZE, z)
    }
}

#[inline]
pub fn position_at(x: u16, y: u16, z: u8) -> Position {
    Position::new(x, y, z)
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
pub struct Chunk {
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
        self.chunks.values().map(|c| usize::from(c.tile_count)).sum()
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
        self.chunks
            .get_mut(&key)?
            .tiles[tile_index(x, y)]
            .as_deref_mut()
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
        chunk.creatures.retain(|c| *c != id);
        if chunk.creatures.is_empty() && chunk.tile_count == 0 {
            self.chunks.remove(&key);
        }
    }

    pub fn move_creature(&mut self, from_x: u16, from_y: u16, from_z: u8, to_x: u16, to_y: u16, to_z: u8, id: CreatureId) {
        self.unregister_creature(from_x, from_y, from_z, id);
        self.register_creature(to_x, to_y, to_z, id);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

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
}
