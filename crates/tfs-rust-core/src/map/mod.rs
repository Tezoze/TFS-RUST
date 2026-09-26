//! Game map: sparse chunk grid, LOS helpers.
// C++ reference: `map.h` / `map.cpp`.

mod grid;
mod los;
mod otbm_load;
mod sector_index;

use std::collections::HashMap;

use rustc_hash::FxHashSet;
use slotmap::SlotMap;
use tfs_rust_common::Position;
use tfs_rust_content::items::ItemDatabase;

use crate::ids::{CreatureId, ItemId};
use crate::item::Item;
use crate::tile::{Tile, TileBody, flags};

pub use grid::{CHUNK_AREA, CHUNK_SIZE, SECTOR_SIZE, SparseGrid};
pub use los::walk_grid_line;

/// Runtime map state (sparse chunk grid + metadata).
#[derive(Debug)]
pub struct Map {
    pub width: u16,
    pub height: u16,
    pub grid: SparseGrid,
    pub towns: HashMap<u32, tfs_rust_content::otbm::TownData>,
    pub waypoints: HashMap<String, Position>,
    /// OTBM `HOUSETILE` membership collected at build (TFS `House::addTile` during parse).
    /// Drained by [`crate::house::ownership`] `house_scan_map` — not a full-grid walk.
    pub house_tiles: Vec<(u32, Position, Vec<ItemId>)>,
    /// Non-house `TILEFLAG_REFRESH` positions from OTBM. Cylinder raster keys off this
    /// set so lazy snapshots cannot shrink which ORIGMAP XY columns are paced.
    pub refresh_positions: FxHashSet<Position>,
    /// Item clones for REFRESH restore. Empty at load; filled on first stack mutation.
    pub refresh_snapshots: HashMap<Position, crate::sector_refresh::TileRefreshSnap>,
}

impl Map {
    pub fn insert_tile(&mut self, pos: Position, tile: Tile) {
        self.grid.insert_tile(pos.x, pos.y, pos.z, tile);
    }

    pub fn get_tile(&self, pos: Position) -> Option<&Tile> {
        self.grid.get_tile(pos.x, pos.y, pos.z)
    }

    pub fn get_tile_mut(&mut self, pos: Position) -> Option<&mut Tile> {
        self.grid.get_tile_mut(pos.x, pos.y, pos.z)
    }

    /// Clone the load-time stack on first mutation of a REFRESH tile.
    ///
    /// No-op when a snap already exists, the tile is a house, or `pos` is not in
    /// [`Self::refresh_positions`]. Restore uses this clone, not live ItemIds.
    pub fn snapshot_refresh_if_needed(&mut self, pos: Position, items: &SlotMap<ItemId, Item>) {
        if self.refresh_snapshots.contains_key(&pos) {
            return;
        }
        if !self.refresh_positions.contains(&pos) {
            return;
        }
        let Some(tile) = self.get_tile(pos) else {
            return;
        };
        if matches!(tile, Tile::House(_)) {
            return;
        }
        if tile.body().flags & flags::REFRESH == 0 {
            return;
        }
        let snap = crate::sector_refresh::TileRefreshSnap::from_tile(tile.body(), items);
        self.refresh_snapshots.insert(pos, snap);
    }

    /// Find a tile that holds `item_id` (down or top stack). Used for house / auto-close checks.
    // C++ ref: `Thing::getTile` / map item position queries (`game.cpp`).
    pub fn find_item_position(&self, item_id: ItemId) -> Option<Position> {
        self.grid.find_item_position(item_id)
    }

    pub fn for_each_tile(&self, f: impl FnMut(Position, &Tile)) {
        self.grid.for_each_tile(f);
    }

    /// True if tile blocks movement (no tile = blocked).
    pub fn is_walkable(&self, pos: Position) -> bool {
        match self.get_tile(pos) {
            Some(t) => {
                let body = t.body();
                body.flags & flags::BLOCK_SOLID == 0 && body.ground.is_some()
            }
            None => false,
        }
    }

    /// C++ `Map::isTileClear` (repo-root `src/map.cpp:496-508`) — blocks sight **only** on
    /// `CONST_PROP_BLOCKPROJECTILE` (Rust `UNTHROW`, set from `ItemType::block_projectile`).
    /// A missing tile does **not** block (C++ returns `true` for null tiles).
    pub(crate) fn blocks_sight(&self, pos: Position) -> bool {
        match self.get_tile(pos) {
            Some(t) => {
                let body = t.body();
                body.flags & flags::UNTHROW != 0
            }
            None => false,
        }
    }

    /// Update tile stack + 16×16 sector spatial index (`Map::moveCreature` — `map.cpp`).
    ///
    /// Audit #3: a creature placed on a void (unloaded) tile would be silently dropped from
    /// both the tile stack and the chunk spatial index. Surface the violation instead —
    /// `tracing::error!` in release, `debug_assert!` panic in debug/test. Never panics in
    /// release (per `tfs-packets.md` validation rules).
    pub fn register_creature_at(&mut self, pos: Position, id: CreatureId) {
        self.register_creature_role_at(pos, id, false);
    }

    /// Same as [`Self::register_creature_at`] with player-list membership for sector find.
    pub fn register_creature_role_at(&mut self, pos: Position, id: CreatureId, is_player: bool) {
        let tile_present = self.get_tile(pos).is_some();
        if let Some(t) = self.get_tile_mut(pos) {
            let body = t.body();
            if !body.creatures().contains(&id) {
                t.add_creature(id);
            }
        }
        self.grid
            .register_creature_role(pos.x, pos.y, pos.z, id, is_player);
        if !tile_present {
            tracing::error!(
                x = pos.x, y = pos.y, z = pos.z, creature = ?id,
                "register_creature_at: target tile is void (unloaded); \
                 creature dropped from tile stack + sector spatial index"
            );
            debug_assert!(
                tile_present,
                "register_creature_at: target tile at {:?} must exist (void placement)",
                pos
            );
        }
    }

    /// Audit #3 / #7: unregistering on a void tile is a silent no-op in the old code; log at
    /// `warn` so untracked-state bugs are observable. Routes through the grid's `pub(super)`
    /// seam so the dual lists cannot desync via direct grid calls (audit #7).
    pub fn unregister_creature_at(&mut self, pos: Position, id: CreatureId) {
        let tile_present = self.get_tile(pos).is_some();
        if let Some(t) = self.get_tile_mut(pos) {
            t.remove_creature(id);
        }
        self.grid.unregister_creature(pos.x, pos.y, pos.z, id);
        if !tile_present {
            tracing::warn!(
                x = pos.x, y = pos.y, z = pos.z, creature = ?id,
                "unregister_creature_at: target tile is void (unloaded); \
                 creature was not tracked at this position"
            );
        }
    }

    /// Debug-only dual-list consistency check (audit #7). No-op in release builds.
    pub fn debug_assert_creature_lists_agree(&self) {
        self.grid.debug_assert_creature_lists_agree();
    }
}

/// Props still contributed by other things on a tile (excluding one item).
/// Used by [`reset_item_tile_flags`] — C++ `Tile::hasProperty(exclude, prop)`.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TileRemainingProps {
    pub block_solid: bool,
    pub immovable_block_solid: bool,
    pub block_path: bool,
    pub no_field_block_path: bool,
    pub immovable_block_path: bool,
    pub immovable_no_field_block_path: bool,
    pub supports_hangable: bool,
    pub unthrow: bool,
    pub hook_east: bool,
    pub hook_south: bool,
}

/// Scan tile for property contributors excluding `exclude` — C++ `Tile::hasProperty(exclude, …)`.
pub(crate) fn tile_remaining_props(
    body: &TileBody,
    items: &SlotMap<ItemId, Item>,
    items_db: &ItemDatabase,
    exclude: ItemId,
) -> TileRemainingProps {
    let mut out = TileRemainingProps::default();
    let mut consider = |it: &tfs_rust_content::otb::ItemType| {
        if it.block_solid() {
            out.block_solid = true;
            if !it.moveable() {
                out.immovable_block_solid = true;
            }
        }
        if it.block_path_find() {
            out.block_path = true;
            if !it.is_magic_field() {
                out.no_field_block_path = true;
                if !it.moveable() {
                    out.immovable_no_field_block_path = true;
                }
            }
            if !it.moveable() {
                out.immovable_block_path = true;
            }
        }
        if it.block_projectile() {
            out.unthrow = true;
        }
        // 772 `HookEast`/`HookSouth` live on wall pieces (`objects.srv` TypeID 1270/1271),
        // not on `HANG` lamps. OTB: `FLAG_VERTICAL` → east hook, `FLAG_HORIZONTAL` → south.
        if it.is_vertical() {
            out.hook_east = true;
        }
        if it.is_horizontal() {
            out.hook_south = true;
        }
        if it.is_vertical() || it.is_horizontal() {
            out.supports_hangable = true;
        }
    };

    // The departing ground is `exclude`. Counting `body.ground` here left
    // BLOCKSOLID set after water → drawbridge (`change_item_type`).
    if body.ground_item != Some(exclude)
        && let Some(ground_type) = body.ground
        && let Some(it) = items_db.items.get(&ground_type)
    {
        consider(it);
    }
    for &iid in body.down_items().iter().chain(body.top_items().iter()) {
        if iid == exclude {
            continue;
        }
        if let Some(item) = items.get(iid)
            && let Some(it) = items_db.items.get(&item.item_type)
        {
            consider(it);
        }
    }
    out
}

/// Clear tile flags that the departing item contributed and no remaining thing still needs.
/// C++ ref: `Tile::resetTileFlags` — `src/tile.cpp:1537-1596`.
pub(crate) fn reset_item_tile_flags(
    body: &mut TileBody,
    departing: &tfs_rust_content::otb::ItemType,
    remaining: &TileRemainingProps,
    items_db: &ItemDatabase,
) {
    if departing.floor_change != 0
        || departing
            .xml_attributes
            .get("floorchange")
            .is_some_and(|s| !s.is_empty())
    {
        body.flags &= !flags::FLOORCHANGE;
    }

    if departing.block_solid() && !remaining.block_solid {
        body.flags &= !flags::BLOCKSOLID;
    }
    if departing.block_solid() && !departing.moveable() && !remaining.immovable_block_solid {
        body.flags &= !flags::IMMOVABLEBLOCKSOLID;
    }
    if departing.block_path_find() && !remaining.block_path {
        body.flags &= !flags::BLOCKPATH;
    }
    if departing.block_path_find() && !departing.is_magic_field() && !remaining.no_field_block_path
    {
        body.flags &= !flags::NOFIELDBLOCKPATH;
    }
    if departing.block_path_find() && !departing.moveable() && !remaining.immovable_block_path {
        body.flags &= !flags::IMMOVABLEBLOCKPATH;
    }
    if departing.block_path_find()
        && !departing.is_magic_field()
        && !departing.moveable()
        && !remaining.immovable_no_field_block_path
    {
        body.flags &= !flags::IMMOVABLENOFIELDBLOCKPATH;
    }
    if departing.block_projectile() && !remaining.unthrow {
        body.flags &= !flags::UNTHROW;
    }
    if departing.is_vertical() && !remaining.hook_east {
        body.flags &= !flags::HOOKEAST;
    }
    if departing.is_horizontal() && !remaining.hook_south {
        body.flags &= !flags::HOOKSOUTH;
    }
    if (departing.is_vertical() || departing.is_horizontal()) && !remaining.supports_hangable {
        body.flags &= !flags::SUPPORTS_HANGABLE;
    }
    // TFS resets these unconditionally when the departing item is of that kind.
    if departing.is_teleport() {
        body.flags &= !flags::TELEPORT;
    }
    if departing.is_magic_field() {
        body.flags &= !flags::MAGICFIELD;
    }
    if departing.is_mailbox() {
        body.flags &= !flags::MAILBOX;
    }
    if departing.is_trashholder() {
        body.flags &= !flags::TRASHHOLDER;
    }
    if departing.is_bed() {
        body.flags &= !flags::BED;
    }
    if items_db.is_depot(departing.server_id) {
        body.flags &= !flags::DEPOT;
    }
}

/// Set runtime tile-state flags from an item's OTB properties, matching C++ `Tile::setTileFlags`.
/// C++ ref: src/tile.cpp:1478-1535
pub(crate) fn apply_item_tile_flags(
    body: &mut TileBody,
    item_type: &tfs_rust_content::otb::ItemType,
    items_db: &ItemDatabase,
) {
    if body.flags & flags::FLOORCHANGE == 0 {
        let typed = u32::from(item_type.floor_change);
        if typed != 0 {
            body.flags |= typed;
        } else if let Some(fc) = item_type.xml_attributes.get("floorchange") {
            let fc_flag = match fc.as_str() {
                "down" => flags::FLOORCHANGE_DOWN,
                "north" => flags::FLOORCHANGE_NORTH,
                "south" => flags::FLOORCHANGE_SOUTH,
                "east" => flags::FLOORCHANGE_EAST,
                "west" => flags::FLOORCHANGE_WEST,
                "southalt" => flags::FLOORCHANGE_SOUTH_ALT,
                "eastalt" => flags::FLOORCHANGE_EAST_ALT,
                _ => 0,
            };
            body.flags |= fc_flag;
        }
    }

    if item_type.block_solid() {
        body.flags |= flags::BLOCKSOLID;
    }

    if item_type.block_solid() && !item_type.moveable() {
        body.flags |= flags::IMMOVABLEBLOCKSOLID;
    }

    if item_type.block_path_find() {
        body.flags |= flags::BLOCKPATH;
    }

    // C++ `CONST_PROP_NOFIELDBLOCKPATH` / `IMMOVABLENOFIELDBLOCKPATH` — `!isMagicField() && blockPathFind` (`src/item.cpp`).
    if item_type.block_path_find() && !item_type.is_magic_field() {
        body.flags |= flags::NOFIELDBLOCKPATH;
        if !item_type.moveable() {
            body.flags |= flags::IMMOVABLENOFIELDBLOCKPATH;
        }
    }

    if item_type.block_path_find() && !item_type.moveable() {
        body.flags |= flags::IMMOVABLEBLOCKPATH;
    }

    // 772 `UNTHROW` — projectile-block, distinct from BLOCKSOLID/BLOCKPATH (`info.cc` `ThrowPossible`).
    if item_type.block_projectile() {
        body.flags |= flags::UNTHROW;
    }

    // 772 `CoordinateFlag(HOOKSOUTH/HOOKEAST)` (`map.cc:2415-2425`, `objects.cc:142-144`).
    // Walls carry the hook (`objects.srv` brick 1270 `HookEast` / 1271 `HookSouth`); hangables
    // carry `Hang` only. OTB `FLAG_VERTICAL` ≡ `HookEast`; `FLAG_HORIZONTAL` ≡ `HookSouth`
    // (TFS `isVertical`/`isHorizontal`; `clientid_output` 1270 vertical / 1271 horizontal).
    if item_type.is_vertical() {
        body.flags |= flags::HOOKEAST;
    }
    if item_type.is_horizontal() {
        body.flags |= flags::HOOKSOUTH;
    }

    if items_db.is_depot(item_type.server_id) {
        body.flags |= flags::DEPOT;
    }

    if item_type.is_teleport() {
        body.flags |= flags::TELEPORT;
    }

    if item_type.is_magic_field() {
        body.flags |= flags::MAGICFIELD;
    }

    if item_type.is_mailbox() {
        body.flags |= flags::MAILBOX;
    }

    if item_type.is_trashholder() {
        body.flags |= flags::TRASHHOLDER;
    }

    if item_type.is_bed() {
        body.flags |= flags::BED;
    }

    // C++ `CONST_PROP_SUPPORTHANGABLE` — `it.isHorizontal || it.isVertical` (`src/item.cpp`).
    if item_type.is_vertical() || item_type.is_horizontal() {
        body.flags |= flags::SUPPORTS_HANGABLE;
    }
}

#[cfg(test)]
mod tile_flag_tests {
    use std::collections::HashMap;

    use slotmap::SlotMap;
    use tfs_rust_common::Position;
    use tfs_rust_content::items::{ITEM_TYPE_TELEPORT, ItemDatabase};
    use tfs_rust_content::otb::ItemType;
    use tfs_rust_content::otbm::{MapData, TileData, TileThing};

    use crate::ids::ItemId;
    use crate::tile::flags;

    fn ground_item_type(id: u16) -> ItemType {
        ItemType {
            id,
            server_id: id,
            group: ItemType::GROUP_GROUND,
            ..ItemType::default()
        }
    }

    fn item_db(entries: Vec<(u16, ItemType)>) -> ItemDatabase {
        ItemDatabase {
            items: entries.into_iter().collect(),
            client_to_server: HashMap::new(),
        }
    }

    fn empty_meta() -> MapData {
        MapData {
            width: 256,
            height: 256,
            spawn_file: None,
            house_file: None,
            spawn_zones: Vec::new(),
            houses: HashMap::new(),
            towns: HashMap::new(),
            waypoints: HashMap::new(),
        }
    }

    fn map_from_single_tile(
        pos: Position,
        things: Vec<TileThing>,
        db: &ItemDatabase,
    ) -> super::Map {
        let mut items: SlotMap<ItemId, crate::item::Item> = SlotMap::with_key();
        super::Map::from_map_data(
            empty_meta(),
            [TileData {
                position: pos,
                house_id: None,
                tile_flags: 0,
                things,
            }],
            db,
            &mut items,
        )
    }

    fn map_and_items_from_single_tile(
        pos: Position,
        things: Vec<TileThing>,
        db: &ItemDatabase,
    ) -> (super::Map, SlotMap<ItemId, crate::item::Item>) {
        let mut items: SlotMap<ItemId, crate::item::Item> = SlotMap::with_key();
        let map = super::Map::from_map_data(
            empty_meta(),
            [TileData {
                position: pos,
                house_id: None,
                tile_flags: 0,
                things,
            }],
            db,
            &mut items,
        );
        (map, items)
    }

    #[test]
    fn otbm_item_node_props_load_attr_text() {
        // OTBM_ITEM props: u16 id + ATTR_TEXT(6) + u16 len + bytes — `item.cpp` ATTR_TEXT.
        const SIGN: u16 = 1429;
        let text = b"Depot";
        let mut raw = Vec::new();
        raw.extend_from_slice(&SIGN.to_le_bytes());
        raw.push(6); // ATTR_TEXT
        raw.extend_from_slice(&(text.len() as u16).to_le_bytes());
        raw.extend_from_slice(text);

        let db = item_db(vec![(
            SIGN,
            ItemType {
                id: SIGN,
                server_id: SIGN,
                name: "sign".into(),
                allow_dist_read_override: Some(true),
                ..ItemType::default()
            },
        )]);
        let pos = Position::new(100, 100, 7);
        let (map, items) =
            map_and_items_from_single_tile(pos, vec![TileThing::ItemNodeProps(raw)], &db);
        let tile = map.get_tile(pos).expect("tile");
        let item_id = tile
            .body()
            .top_items()
            .first()
            .or_else(|| tile.body().down_items().first())
            .copied()
            .expect("sign item");
        let item = items.get(item_id).expect("item");
        assert_eq!(item.item_type, SIGN);
        assert_eq!(item.text(), "Depot");
    }

    #[test]
    fn otbm_ground_item_node_keeps_action_and_unique_id() {
        // OTBM_ITEM props: u16 id + ATTR_ACTION_ID(4) + u16 + ATTR_UNIQUE_ID(5) + u16.
        const GROUND: u16 = 100;
        let mut raw = Vec::new();
        raw.extend_from_slice(&GROUND.to_le_bytes());
        raw.push(4); // ATTR_ACTION_ID
        raw.extend_from_slice(&1001u16.to_le_bytes());
        raw.push(5); // ATTR_UNIQUE_ID
        raw.extend_from_slice(&2001u16.to_le_bytes());

        let db = item_db(vec![(GROUND, ground_item_type(GROUND))]);
        let pos = Position::new(100, 100, 7);
        let (map, items) =
            map_and_items_from_single_tile(pos, vec![TileThing::ItemNodeProps(raw)], &db);
        let tile = map.get_tile(pos).expect("tile");
        let gid = tile.body().ground_item.expect("ground_item");
        let item = items.get(gid).expect("item");
        assert_eq!(item.item_type, GROUND);
        assert_eq!(item.action_id(), 1001);
        assert_eq!(item.unique_id(), 2001);
    }

    #[test]
    fn otbm_teleport_dest_only_is_kept() {
        // Dest-only OTBM_ITEM: u16 id + ATTR_TELE_DEST(8) + x/y/z. No action/unique bits.
        const GROUND: u16 = 100;
        const TELEPORT: u16 = 1387;
        let dest = Position::new(110, 120, 7);
        let mut raw = Vec::new();
        raw.extend_from_slice(&TELEPORT.to_le_bytes());
        raw.push(8); // ATTR_TELE_DEST
        raw.extend_from_slice(&dest.x.to_le_bytes());
        raw.extend_from_slice(&dest.y.to_le_bytes());
        raw.push(dest.z);

        let db = item_db(vec![
            (GROUND, ground_item_type(GROUND)),
            (
                TELEPORT,
                ItemType {
                    id: TELEPORT,
                    server_id: TELEPORT,
                    type_tag: ITEM_TYPE_TELEPORT,
                    ..ItemType::default()
                },
            ),
        ]);
        let pos = Position::new(100, 100, 7);
        let (map, items) = map_and_items_from_single_tile(
            pos,
            vec![
                TileThing::EmbeddedItemId(GROUND),
                TileThing::ItemNodeProps(raw),
            ],
            &db,
        );
        let tile = map.get_tile(pos).expect("tile");
        assert_ne!(tile.body().flags & flags::TELEPORT, 0);
        let tele_id = tile
            .body()
            .down_items()
            .first()
            .copied()
            .or_else(|| tile.body().top_items().first().copied())
            .expect("teleport item");
        let item = items.get(tele_id).expect("item");
        assert_eq!(item.item_type, TELEPORT);
        assert_eq!(item.tele_dest(), Some(dest));
    }

    #[test]
    fn teleport_item_sets_tile_teleport_flag() {
        const GROUND: u16 = 100;
        const TELEPORT: u16 = 1387;
        let pos = Position::new(100, 100, 7);
        let db = item_db(vec![
            (GROUND, ground_item_type(GROUND)),
            (
                TELEPORT,
                ItemType {
                    id: TELEPORT,
                    server_id: TELEPORT,
                    type_tag: ITEM_TYPE_TELEPORT,
                    ..ItemType::default()
                },
            ),
        ]);
        let map = map_from_single_tile(
            pos,
            vec![
                TileThing::EmbeddedItemId(GROUND),
                TileThing::EmbeddedItemId(TELEPORT),
            ],
            &db,
        );
        let tile = map.get_tile(pos).expect("tile");
        assert_ne!(tile.body().flags & flags::TELEPORT, 0);
    }

    #[test]
    fn floorchange_item_sets_tile_floorchange_flag() {
        const GROUND: u16 = 100;
        const STAIR: u16 = 459;
        let pos = Position::new(100, 100, 7);
        let db = item_db(vec![
            (GROUND, ground_item_type(GROUND)),
            (
                STAIR,
                ItemType {
                    id: STAIR,
                    server_id: STAIR,
                    floor_change: 1 << 0,
                    ..ItemType::default()
                },
            ),
        ]);
        let map = map_from_single_tile(
            pos,
            vec![
                TileThing::EmbeddedItemId(GROUND),
                TileThing::EmbeddedItemId(STAIR),
            ],
            &db,
        );
        let tile = map.get_tile(pos).expect("tile");
        assert_ne!(tile.body().flags & flags::FLOORCHANGE_DOWN, 0);
    }

    /// OTB `FLAG_VERTICAL` (1<<17) on a wall → 772 `HOOKEAST` (`objects.srv` 1270).
    #[test]
    fn vertical_wall_sets_hookeast() {
        let mut body = crate::tile::TileBody::default();
        let wall = ItemType {
            flags: (1 << 17) | (1 << 0), // FLAG_VERTICAL | FLAG_BLOCK_SOLID
            ..ItemType::default()
        };
        let db = item_db(vec![]);
        super::apply_item_tile_flags(&mut body, &wall, &db);
        assert_ne!(body.flags & flags::HOOKEAST, 0);
        assert_eq!(body.flags & flags::HOOKSOUTH, 0);
        assert_ne!(body.flags & flags::SUPPORTS_HANGABLE, 0);
    }

    /// OTB `FLAG_HORIZONTAL` (1<<18) on a wall → 772 `HOOKSOUTH` (`objects.srv` 1271).
    #[test]
    fn horizontal_wall_sets_hooksouth() {
        let mut body = crate::tile::TileBody::default();
        let wall = ItemType {
            flags: (1 << 18) | (1 << 0), // FLAG_HORIZONTAL | FLAG_BLOCK_SOLID
            ..ItemType::default()
        };
        let db = item_db(vec![]);
        super::apply_item_tile_flags(&mut body, &wall, &db);
        assert_ne!(body.flags & flags::HOOKSOUTH, 0);
        assert_eq!(body.flags & flags::HOOKEAST, 0);
    }

    /// Hangables carry `Hang` only (`objects.srv` wall lamp 2907) — they must not stamp hooks.
    #[test]
    fn hangable_without_orientation_does_not_set_hooks() {
        let mut body = crate::tile::TileBody::default();
        let lamp = ItemType {
            flags: 1 << 16, // FLAG_HANGABLE
            ..ItemType::default()
        };
        let db = item_db(vec![]);
        super::apply_item_tile_flags(&mut body, &lamp, &db);
        assert_eq!(body.flags & flags::HOOKEAST, 0);
        assert_eq!(body.flags & flags::HOOKSOUTH, 0);
    }

    #[test]
    fn from_map_data_indexes_otbm_house_tiles() {
        const GROUND: u16 = 100;
        let pos = Position::new(100, 100, 7);
        let db = item_db(vec![(GROUND, ground_item_type(GROUND))]);
        let mut items: SlotMap<ItemId, crate::item::Item> = SlotMap::with_key();
        let map = super::Map::from_map_data(
            empty_meta(),
            [
                TileData {
                    position: pos,
                    house_id: Some(7),
                    tile_flags: 0,
                    things: vec![TileThing::EmbeddedItemId(GROUND)],
                },
                TileData {
                    position: Position::new(101, 100, 7),
                    house_id: None,
                    tile_flags: 0,
                    things: vec![TileThing::EmbeddedItemId(GROUND)],
                },
            ],
            &db,
            &mut items,
        );
        assert_eq!(map.house_tiles.len(), 1);
        assert_eq!(map.house_tiles[0].0, 7);
        assert_eq!(map.house_tiles[0].1, pos);
    }
}
