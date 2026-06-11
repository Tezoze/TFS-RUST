//! Game map: sparse chunk grid, LOS helpers.
// C++ reference: `map.h` / `map.cpp`.

mod grid;
mod los;

use std::collections::HashMap;

use slotmap::SlotMap;
use tfs_rust_common::Position;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::otbm::{self, MapData, TileData, TileThing};

use crate::ids::{CreatureId, ItemId};
use crate::item::Item;
use crate::tile::HouseTile;
use crate::tile::{flags, Tile, TileBody};

pub use grid::{SparseGrid, CHUNK_AREA, CHUNK_SIZE};
pub use los::walk_grid_line;

/// Runtime map state (sparse chunk grid + metadata).
#[derive(Debug)]
pub struct Map {
    pub width: u16,
    pub height: u16,
    pub grid: SparseGrid,
    pub towns: HashMap<u32, tfs_rust_content::otbm::TownData>,
    pub waypoints: HashMap<String, Position>,
}

impl Map {
    /// Build runtime tiles from parsed OTBM (`IOMap::parseTileArea` + `Tile::internalAddThing` — `src/iomap.cpp`, `src/tile.cpp`).
    ///
    /// DEVIATION FROM C++: Creates actual Item instances in the items SlotMap instead of
    /// just storing raw item types. This is required for the new item system.
    pub fn from_map_data(data: MapData, items_db: &ItemDatabase, items: &mut SlotMap<ItemId, Item>) -> Self {
        let mut grid = SparseGrid::new();
        for (pos, td) in data.tiles {
            let tile = tile_from_data(td, items_db, items);
            grid.insert_tile(pos.x, pos.y, pos.z, tile);
        }
        Self {
            width: data.width,
            height: data.height,
            grid,
            towns: data.towns,
            waypoints: data.waypoints,
        }
    }

    pub fn insert_tile(&mut self, pos: Position, tile: Tile) {
        self.grid.insert_tile(pos.x, pos.y, pos.z, tile);
    }

    pub fn get_tile(&self, pos: Position) -> Option<&Tile> {
        self.grid.get_tile(pos.x, pos.y, pos.z)
    }

    pub fn get_tile_mut(&mut self, pos: Position) -> Option<&mut Tile> {
        self.grid.get_tile_mut(pos.x, pos.y, pos.z)
    }

    /// Find a tile that holds `item_id` (down or top stack). Used for house / auto-close checks.
    // C++ ref: `Thing::getTile` / map item position queries (`game.cpp`).
    pub fn find_item_position(&self, item_id: ItemId) -> Option<Position> {
        self.grid.find_item_position(item_id)
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

    pub(crate) fn blocks_sight(&self, pos: Position) -> bool {
        match self.get_tile(pos) {
            Some(t) => {
                let body = t.body();
                body.flags & (flags::BLOCK_SOLID | flags::BLOCK_PROJECTILE) != 0
            }
            None => true,
        }
    }

    /// Update tile stack + chunk spatial index (`Map::moveCreature` creature lists — `map.cpp`).
    pub fn register_creature_at(&mut self, pos: Position, id: CreatureId) {
        if let Some(t) = self.get_tile_mut(pos) {
            let body = t.body();
            if !body.creatures.contains(&id) {
                t.add_creature(id);
            }
        }
        self.grid.register_creature(pos.x, pos.y, pos.z, id);
    }

    pub fn unregister_creature_at(&mut self, pos: Position, id: CreatureId) {
        if let Some(t) = self.get_tile_mut(pos) {
            t.remove_creature(id);
        }
        self.grid.unregister_creature(pos.x, pos.y, pos.z, id);
    }
}

/// C++ `Tile::internalAddThing` for item ids (`src/tile.cpp`).
/// Creates an Item instance and returns its ItemId.
fn internal_add_item_id(
    id: u16,
    items_db: &ItemDatabase,
    body: &mut TileBody,
    items: &mut SlotMap<ItemId, Item>,
) {
    let id = otbm::remap_create_item_stream_id(id);
    let it = items_db.items.get(&id);
    let is_ground = it.map(|t| t.is_ground_tile()).unwrap_or(false);
    if is_ground && body.ground.is_none() {
        body.ground = Some(id);
        return;
    }

    let item = Item::new_single(id);
    let item_id = items.insert(item);

    let always_on_top = it.map(|t| t.always_on_top()).unwrap_or(false);
    if always_on_top {
        body.top_items.push(item_id);
    } else {
        body.down_items.insert(0, item_id);
    }
}

/// Convert raw OTBM tile flags to TILESTATE flags.
/// C++ ref: src/iomap.cpp:270-280 — OTBM zone flags use a different bit layout than runtime TILESTATE.
fn convert_otbm_flags(otbm_flags: u32) -> (u32, tfs_rust_common::ZoneType) {
    const OTBM_TILEFLAG_PROTECTIONZONE: u32 = 1 << 0;
    const OTBM_TILEFLAG_NOPVPZONE: u32 = 1 << 2;
    const OTBM_TILEFLAG_NOLOGOUT: u32 = 1 << 3;
    const OTBM_TILEFLAG_PVPZONE: u32 = 1 << 4;

    let mut tileflags = 0u32;
    let mut zone = tfs_rust_common::ZoneType::Normal;

    if otbm_flags & OTBM_TILEFLAG_PROTECTIONZONE != 0 {
        tileflags |= flags::PROTECTIONZONE;
        zone = tfs_rust_common::ZoneType::Protection;
    } else if otbm_flags & OTBM_TILEFLAG_NOPVPZONE != 0 {
        tileflags |= flags::NOPVPZONE;
        zone = tfs_rust_common::ZoneType::NoPvp;
    } else if otbm_flags & OTBM_TILEFLAG_PVPZONE != 0 {
        tileflags |= flags::PVPZONE;
        zone = tfs_rust_common::ZoneType::Pvp;
    }

    if otbm_flags & OTBM_TILEFLAG_NOLOGOUT != 0 {
        tileflags |= flags::NOLOGOUT;
    }

    (tileflags, zone)
}

/// Set runtime tile-state flags from an item's OTB properties, matching C++ `Tile::setTileFlags`.
/// C++ ref: src/tile.cpp:1478-1535
fn apply_item_tile_flags(
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

    if items_db.is_depot(item_type.server_id) {
        body.flags |= flags::DEPOT;
    }
}

/// Raw OTBM item stream id before `remap_create_item_stream_id` (`src/item.cpp` `CreateItem(PropStream&)`).
fn otbm_item_stream_id(thing: &TileThing) -> Option<u16> {
    match thing {
        TileThing::EmbeddedItemId(id) => Some(*id),
        TileThing::ItemNodeProps(raw) => {
            if raw.len() < 2 {
                return None;
            }
            Some(u16::from_le_bytes([raw[0], raw[1]]))
        }
    }
}

fn tile_from_data(
    td: TileData,
    items_db: &ItemDatabase,
    items: &mut SlotMap<ItemId, Item>,
) -> Tile {
    let (converted_flags, zone) = convert_otbm_flags(td.tile_flags);

    let mut body = TileBody {
        ground: None,
        down_items: Vec::new(),
        top_items: Vec::new(),
        creatures: Vec::new(),
        flags: converted_flags,
        zone,
    };

    for thing in td.things {
        let Some(stream_id) = otbm_item_stream_id(&thing) else {
            continue;
        };
        let id = otbm::remap_create_item_stream_id(stream_id);

        if let Some(item_type) = items_db.items.get(&id) {
            apply_item_tile_flags(&mut body, item_type, items_db);
        }

        internal_add_item_id(stream_id, items_db, &mut body, items);
    }

    if let Some(hid) = td.house_id {
        Tile::House(HouseTile {
            inner: body,
            house_id: hid,
        })
    } else {
        Tile::Normal(body)
    }
}
