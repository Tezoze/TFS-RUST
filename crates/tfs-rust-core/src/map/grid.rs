//! Lazy 64×64 chunk grid — tiles only. Spatial creature find is 16×16 sectors.
//!
//! Dense `Vec<Tile>` + `[u16; 4096]` slot index (`u16::MAX` = empty). Layout only —
//! `get_tile` stays O(1). Spectator / idle find walks [`SectorIndex`] (`crmain.cc:101–144`).
//!
//! Replaces `HashMap<Position, Tile>` and `QTreeNode` (`map.cpp` lazy spatial index outcomes).
// C++ reference: `map.cpp` `Map::getSpectators`, tile storage (sparse world).
// Corpus find: `crmain.cc` `TFindCreatures` 16×16 `blockx`/`blocky`.

use rustc_hash::FxHashMap;
use tfs_rust_common::Position;

use crate::ids::CreatureId;
use crate::tile::Tile;

use super::sector_index::SectorIndex;

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

const EMPTY_SLOT: u16 = u16::MAX;

/// One 64×64 region on a single floor (tiles only — creatures live on 16×16 sectors).
#[derive(Debug)]
pub(crate) struct Chunk {
    pub tile_count: u16,
    /// Slot → dense `tiles` index; [`EMPTY_SLOT`] means unoccupied.
    slot_index: Box<[u16; CHUNK_AREA]>,
    tiles: Vec<Tile>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            tile_count: 0,
            slot_index: Box::new([EMPTY_SLOT; CHUNK_AREA]),
            tiles: Vec::new(),
        }
    }

    fn tile_at(&self, slot: usize) -> Option<&Tile> {
        let dense = self.slot_index[slot];
        if dense == EMPTY_SLOT {
            None
        } else {
            self.tiles.get(usize::from(dense))
        }
    }

    fn tile_at_mut(&mut self, slot: usize) -> Option<&mut Tile> {
        let dense = self.slot_index[slot];
        if dense == EMPTY_SLOT {
            None
        } else {
            self.tiles.get_mut(usize::from(dense))
        }
    }

    fn insert_at(&mut self, slot: usize, tile: Tile) {
        let dense = self.slot_index[slot];
        if dense == EMPTY_SLOT {
            let di = self.tiles.len() as u16;
            self.slot_index[slot] = di;
            self.tiles.push(tile);
            self.tile_count += 1;
        } else {
            self.tiles[usize::from(dense)] = tile;
        }
    }
}

/// Sparse tile store + 16×16 sector creature index (replaces quadtree + `HashMap<Position, Tile>`).
#[derive(Debug, Default)]
pub struct SparseGrid {
    chunks: FxHashMap<ChunkKey, Box<Chunk>>,
    sectors: SectorIndex,
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
            .map(|t| {
                let b = t.body();
                b.down_items().len() + b.top_items().len()
            })
            .sum()
    }

    pub fn get_tile(&self, x: u16, y: u16, z: u8) -> Option<&Tile> {
        let key = ChunkKey::from_pos(x, y, z);
        self.chunks.get(&key)?.tile_at(tile_index(x, y))
    }

    pub fn get_tile_mut(&mut self, x: u16, y: u16, z: u8) -> Option<&mut Tile> {
        let key = ChunkKey::from_pos(x, y, z);
        self.chunks.get_mut(&key)?.tile_at_mut(tile_index(x, y))
    }

    pub fn insert_tile(&mut self, x: u16, y: u16, z: u8, tile: Tile) {
        let key = ChunkKey::from_pos(x, y, z);
        let chunk = self
            .chunks
            .entry(key)
            .or_insert_with(|| Box::new(Chunk::new()));
        chunk.insert_at(tile_index(x, y), tile);
    }

    /// Drop spare `Vec<Tile>` capacity after OTBM load.
    pub fn shrink_to_fit(&mut self) {
        for chunk in self.chunks.values_mut() {
            chunk.tiles.shrink_to_fit();
        }
    }

    /// Chunk spatial list only — does not allocate a chunk (tile must exist first).
    ///
    /// `pub(super)`: all creature placement must funnel through `Map::register_creature_at`
    /// so the dual `TileBody.creatures` / sector lists stay in sync (audit #7).
    // C++ reference: `map.cpp` `Map::moveCreature` creature-list bookkeeping.
    // Corpus: `TFindCreatures` 16×16 sector chains — `crmain.cc:101–144`.
    pub(super) fn register_creature_role(
        &mut self,
        x: u16,
        y: u16,
        z: u8,
        id: CreatureId,
        is_player: bool,
    ) {
        let key = ChunkKey::from_pos(x, y, z);
        if !self.chunks.contains_key(&key) {
            return;
        }
        self.sectors.insert(x, y, id, is_player);
    }

    /// Conn-mapping hook — player list membership independent of tile register order.
    pub(crate) fn note_sector_player(&mut self, x: u16, y: u16, id: CreatureId) {
        self.sectors.note_player(x, y, id);
    }

    pub(crate) fn forget_sector_player(&mut self, id: CreatureId) {
        self.sectors.forget_player(id);
    }

    /// `pub(super)`: see [`SparseGrid::register_creature`] — route through
    /// `Map::unregister_creature_at` to keep the dual lists in sync (audit #7).
    pub(super) fn unregister_creature(&mut self, x: u16, y: u16, z: u8, id: CreatureId) {
        self.sectors.remove(x, y, id);
        let key = ChunkKey::from_pos(x, y, z);
        let Some(chunk) = self.chunks.get(&key) else {
            return;
        };
        if chunk.tile_count == 0 {
            self.chunks.remove(&key);
        }
    }

    /// Debug-only dual-list consistency check (audit #7).
    ///
    /// Verifies every sector-list creature is on some tile's `TileBody.creatures` list
    /// in that XY sector (any floor), and vice versa. `debug_assert!` compiles out in
    /// release.
    pub fn debug_assert_creature_lists_agree(&self) {
        #[cfg(debug_assertions)]
        {
            for (key, chunk) in &self.chunks {
                let (ox, oy, z) = key.chunk_origin();
                for (slot, &dense) in chunk.slot_index.iter().enumerate() {
                    if dense == EMPTY_SLOT {
                        continue;
                    }
                    let tile = &chunk.tiles[usize::from(dense)];
                    let body = tile.body();
                    if body.creatures().is_empty() {
                        continue;
                    }
                    let pos = position_from_chunk_slot(ox, oy, z, slot);
                    for &cid in body.creatures() {
                        debug_assert!(
                            self.sectors.contains_creature(pos.x, pos.y, cid),
                            "creature {:?} on tile {:?} missing from sector spatial list",
                            cid,
                            pos
                        );
                    }
                }
            }
            self.sectors.for_each_creature(|_, cid| {
                let on_tile = self.chunks.values().any(|chunk| {
                    chunk
                        .tiles
                        .iter()
                        .any(|t| t.body().creatures().contains(&cid))
                });
                debug_assert!(
                    on_tile,
                    "creature {cid:?} in sector spatial list but not on any tile"
                );
            });
        }
    }

    /// Spatial **superset** for spectator fan-out — 16×16 sector overlap, all floors.
    /// `z` is unused (lists span floors); kept so call sites stay the same. Callers filter
    /// with `canSee` / same-floor as needed.
    // C++ reference: `TFindCreatures` — `crmain.cc:101–144`; TFS `Map::getSpectators` pack surface.
    pub fn collect_spectators(
        &self,
        center_x: u16,
        center_y: u16,
        z: u8,
        range_x: u16,
        range_y: u16,
        out: &mut Vec<CreatureId>,
    ) {
        let _ = z;
        self.sectors
            .collect(center_x, center_y, range_x, range_y, false, out);
    }

    /// Viewport creatures in 772 `TFindCreatures::getNext` 16×16 sector order (`crmain.cc:101–144`).
    ///
    /// Walks `blocky` outer / `blockx` inner over XY sectors covering the box and dumps each
    /// sector's `Vec` (enter order within a sector). Lists span floors; callers filter Z.
    /// Exact LIFO `NextChainCreature` is not reproduced — `Vec` is the IDLE-3 stand-in.
    pub fn collect_spectators_sector_order(
        &self,
        center_x: u16,
        center_y: u16,
        z: u8,
        range_x: u16,
        range_y: u16,
        out: &mut Vec<CreatureId>,
    ) {
        let _ = z;
        self.sectors
            .collect(center_x, center_y, range_x, range_y, false, out);
    }

    /// Player-only sector walk — same XY overlap as [`Self::collect_spectators`].
    pub fn collect_spectator_players(
        &self,
        center_x: u16,
        center_y: u16,
        range_x: u16,
        range_y: u16,
        out: &mut Vec<CreatureId>,
    ) {
        self.sectors
            .collect(center_x, center_y, range_x, range_y, true, out);
    }

    pub fn find_item_position(&self, item_id: crate::ids::ItemId) -> Option<Position> {
        for (key, chunk) in &self.chunks {
            let (ox, oy, z) = key.chunk_origin();
            for (slot, &dense) in chunk.slot_index.iter().enumerate() {
                if dense == EMPTY_SLOT {
                    continue;
                }
                if chunk.tiles[usize::from(dense)].has_item(item_id) {
                    return Some(position_from_chunk_slot(ox, oy, z, slot));
                }
            }
        }
        None
    }

    pub fn for_each_tile(&self, mut f: impl FnMut(Position, &Tile)) {
        for (key, chunk) in &self.chunks {
            let (ox, oy, z) = key.chunk_origin();
            for (slot, &dense) in chunk.slot_index.iter().enumerate() {
                if dense == EMPTY_SLOT {
                    continue;
                }
                f(
                    position_from_chunk_slot(ox, oy, z, slot),
                    &chunk.tiles[usize::from(dense)],
                );
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
            stacks: TileBody::stacks_from(vec![item], vec![], vec![]),
            flags: 0,
            zone: tfs_rust_common::ZoneType::Normal,
        });
        grid.insert_tile(70, 70, 7, tile);

        let mut c1 = SlotMap::<CreatureId, ()>::with_key();
        let id1 = c1.insert(());
        grid.register_creature_role(70, 70, 7, id1, false);

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
                stacks: TileBody::stacks_from(vec![], vec![], vec![id]),
                flags: 0,
                zone: tfs_rust_common::ZoneType::Normal,
            });
            grid.insert_tile(x, y, 7, tile);
            grid.register_creature_role(x, y, 7, id, false);
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

    /// Same 64×64 chunk, creature in an adjacent 16×16 sector outside `range` is not collected.
    #[test]
    fn collect_spectators_does_not_dump_whole_64_chunk() {
        let mut grid = SparseGrid::new();
        let mut sm: SlotMap<CreatureId, ()> = SlotMap::with_key();
        let near = sm.insert(());
        let far = sm.insert(());
        for (x, y, id) in [(8u16, 8u16, near), (48u16, 8u16, far)] {
            let tile = crate::tile::Tile::Normal(TileBody {
                ground: Some(100),
                ground_item: None,
                stacks: TileBody::stacks_from(vec![], vec![], vec![id]),
                flags: 0,
                zone: tfs_rust_common::ZoneType::Normal,
            });
            grid.insert_tile(x, y, 7, tile);
            grid.register_creature_role(x, y, 7, id, false);
        }
        let mut out = Vec::new();
        grid.collect_spectators(8, 8, 7, 11, 11, &mut out);
        assert!(out.contains(&near));
        assert!(
            !out.contains(&far),
            "48,8 is the same 64×64 chunk but a different 16×16 sector; must not be dumped"
        );
    }

    /// Sector lists span floors — find on z=6 when collecting with z=7.
    #[test]
    fn collect_spectators_spans_floors_in_xy_sector() {
        let mut grid = SparseGrid::new();
        let mut sm: SlotMap<CreatureId, ()> = SlotMap::with_key();
        let id = sm.insert(());
        let tile = crate::tile::Tile::Normal(TileBody {
            ground: Some(100),
            ground_item: None,
            stacks: TileBody::stacks_from(vec![], vec![], vec![id]),
            flags: 0,
            zone: tfs_rust_common::ZoneType::Normal,
        });
        grid.insert_tile(70, 70, 6, tile);
        grid.register_creature_role(70, 70, 6, id, false);
        let mut out = Vec::new();
        grid.collect_spectators(70, 70, 7, 1, 1, &mut out);
        assert!(out.contains(&id));
    }

    #[test]
    fn collect_spectator_players_skips_non_players() {
        let mut grid = SparseGrid::new();
        let mut sm: SlotMap<CreatureId, ()> = SlotMap::with_key();
        let player = sm.insert(());
        let monster = sm.insert(());
        for (x, id, is_player) in [(70u16, player, true), (71u16, monster, false)] {
            let tile = crate::tile::Tile::Normal(TileBody {
                ground: Some(100),
                ground_item: None,
                stacks: TileBody::stacks_from(vec![], vec![], vec![id]),
                flags: 0,
                zone: tfs_rust_common::ZoneType::Normal,
            });
            grid.insert_tile(x, 70, 7, tile);
            grid.register_creature_role(x, 70, 7, id, is_player);
        }
        let mut out = Vec::new();
        grid.collect_spectator_players(70, 70, 5, 5, &mut out);
        assert_eq!(out, vec![player]);
        out.clear();
        grid.collect_spectators(70, 70, 7, 5, 5, &mut out);
        assert!(out.contains(&player) && out.contains(&monster));
    }

    /// Audit #7 — a creature in the sector list but not on any tile's `TileBody.creatures`
    /// must trip `debug_assert_creature_lists_agree`. Debug-only (`debug_assert!` compiles
    /// out in release).
    #[cfg(debug_assertions)]
    #[test]
    fn debug_assert_catches_chunk_list_creature_not_on_tile() {
        let mut grid = SparseGrid::new();
        let tile = crate::tile::Tile::Normal(TileBody {
            ground: Some(100),

            ground_item: None,
            stacks: None,
            flags: 0,
            zone: tfs_rust_common::ZoneType::Normal,
        });
        grid.insert_tile(70, 70, 7, tile);

        let mut sm: SlotMap<CreatureId, ()> = SlotMap::with_key();
        let orphan = sm.insert(());

        // Corrupt: push into the sector spatial list without touching any tile list.
        grid.sectors.insert(70, 70, orphan, false);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            grid.debug_assert_creature_lists_agree();
        }));
        assert!(
            result.is_err(),
            "debug_assert_creature_lists_agree must catch a sector-list creature not on any tile"
        );
    }

    /// Audit #7 — a creature on a tile's `TileBody.creatures` list but missing from the
    /// sector spatial list must trip `debug_assert_creature_lists_agree`. Debug-only.
    #[cfg(debug_assertions)]
    #[test]
    fn debug_assert_catches_tile_list_creature_not_in_chunk() {
        let mut grid = SparseGrid::new();
        let mut sm: SlotMap<CreatureId, ()> = SlotMap::with_key();
        let orphan = sm.insert(());

        let tile = crate::tile::Tile::Normal(TileBody {
            ground: Some(100),

            ground_item: None,
            stacks: TileBody::stacks_from(vec![], vec![], vec![orphan]),
            flags: 0,
            zone: tfs_rust_common::ZoneType::Normal,
        });
        grid.insert_tile(70, 70, 7, tile);
        // Corrupt: skip register_creature so the sector list is empty.
        assert!(
            !grid.sectors.contains_creature(70, 70, orphan),
            "precondition: orphan must not be in sector list"
        );

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            grid.debug_assert_creature_lists_agree();
        }));
        assert!(
            result.is_err(),
            "debug_assert_creature_lists_agree must catch a tile-list creature missing from the sector"
        );
    }

    /// Audit #7 — a clean grid must NOT trip the consistency check. The tile list and sector
    /// list must both hold the creature (the `*_at` seam keeps them in sync; here we mirror
    /// that by inserting a tile whose `creatures` list already contains the id, then syncing
    /// the sector list via `register_creature`).
    #[test]
    fn debug_assert_passes_on_clean_grid() {
        let mut grid = SparseGrid::new();
        let mut sm: SlotMap<CreatureId, ()> = SlotMap::with_key();
        let id = sm.insert(());

        let tile = crate::tile::Tile::Normal(TileBody {
            ground: Some(100),

            ground_item: None,
            stacks: TileBody::stacks_from(vec![], vec![], vec![id]),
            flags: 0,
            zone: tfs_rust_common::ZoneType::Normal,
        });
        grid.insert_tile(70, 70, 7, tile);
        grid.register_creature_role(70, 70, 7, id, false);
        // No panic expected in either build.
        grid.debug_assert_creature_lists_agree();
    }
}
