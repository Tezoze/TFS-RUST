//! Game map: tiles, quadtree per floor, LOS helpers.
// C++ reference: `map.h` / `map.cpp`.

mod los;
pub mod qtree;

use std::collections::HashMap;

use tfs_rust_common::Position;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::otbm::{self, MapData, TileData, TileThing};

use crate::ids::CreatureId;
use crate::tile::HouseTile;
use crate::tile::{flags, Tile, TileBody};

use self::qtree::QTreeNode;

pub use los::walk_grid_line;

/// Runtime map state (tiles + spatial indices per floor).
#[derive(Debug)]
pub struct Map {
    pub width: u16,
    pub height: u16,
    /// Tiles that exist in the OTBM (sparse).
    pub tiles: HashMap<Position, Tile>,
    /// Spectator quadtree per floor `z`.
    pub qtrees: HashMap<u8, QTreeNode>,
    pub towns: HashMap<u32, tfs_rust_content::otbm::TownData>,
    pub waypoints: HashMap<String, Position>,
}

impl Map {
    /// Build runtime tiles from parsed OTBM (`IOMap::parseTileArea` + `Tile::internalAddThing` — `src/iomap.cpp`, `src/tile.cpp`).
    pub fn from_map_data(data: MapData, items_db: &ItemDatabase) -> Self {
        let mut tiles: HashMap<Position, Tile> = HashMap::new();
        for (pos, td) in data.tiles {
            tiles.insert(pos, tile_from_data(td, items_db));
        }
        let mut qtrees = HashMap::new();
        qtrees.insert(
            0,
            QTreeNode::build(
                0,
                0,
                data.width.saturating_sub(1),
                data.height.saturating_sub(1),
            ),
        );
        // Additional floors: build lazily in later phases when underground layers are tracked.
        Self {
            width: data.width,
            height: data.height,
            tiles,
            qtrees,
            towns: data.towns,
            waypoints: data.waypoints,
        }
    }

    pub fn get_tile(&self, pos: Position) -> Option<&Tile> {
        self.tiles.get(&pos)
    }

    pub fn get_tile_mut(&mut self, pos: Position) -> Option<&mut Tile> {
        self.tiles.get_mut(&pos)
    }

    pub fn qtree_mut(&mut self, z: u8) -> &mut QTreeNode {
        let w = self.width.saturating_sub(1);
        let h = self.height.saturating_sub(1);
        self.qtrees
            .entry(z)
            .or_insert_with(|| QTreeNode::build(0, 0, w, h))
    }

    pub fn qtree(&self, z: u8) -> Option<&QTreeNode> {
        self.qtrees.get(&z)
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
}

/// C++ `Tile::internalAddThing` for item ids (`src/tile.cpp`).
fn internal_add_item_id(id: u16, items_db: &ItemDatabase, body: &mut TileBody) {
    let id = otbm::remap_create_item_stream_id(id);
    let it = items_db.items.get(&id);
    let is_ground = it.map(|t| t.is_ground_tile()).unwrap_or(false);
    if is_ground && body.ground.is_none() {
        body.ground = Some(id);
        return;
    }
    let always_on_top = it.map(|t| t.always_on_top()).unwrap_or(false);
    if always_on_top {
        let order = it.map(|t| t.always_on_top_order).unwrap_or(0);
        let pos = body.top_items.iter().position(|&tid| {
            items_db
                .items
                .get(&tid)
                .map(|t| t.always_on_top_order > order)
                .unwrap_or(true)
        });
        match pos {
            Some(i) => body.top_items.insert(i, id),
            None => body.top_items.push(id),
        }
    } else {
        body.down_items.insert(0, id);
    }
}

fn tile_from_data(td: TileData, items_db: &ItemDatabase) -> Tile {
    let mut body = TileBody {
        position: td.position,
        ground: None,
        down_items: Vec::new(),
        top_items: Vec::new(),
        creatures: Vec::new(),
        flags: td.tile_flags,
        zone: tfs_rust_common::ZoneType::Normal,
    };
    for thing in td.things {
        match thing {
            TileThing::EmbeddedItemId(id) => {
                internal_add_item_id(id, items_db, &mut body);
            }
            TileThing::ItemNodeProps(raw) => {
                if let Some(id) = otbm::item_id_from_otbm_item_props(&raw) {
                    internal_add_item_id(id, items_db, &mut body);
                }
            }
        }
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

impl Map {
    pub fn register_creature_index(&mut self, pos: Position, id: CreatureId) {
        if let Some(q) = self.qtrees.get_mut(&pos.z) {
            q.insert_creature(pos, id);
        }
    }

    pub fn move_creature_index(&mut self, from: Position, to: Position, id: CreatureId) {
        if let Some(q) = self.qtrees.get_mut(&from.z) {
            q.remove_creature(from, id);
        }
        if from.z == to.z {
            if let Some(q) = self.qtrees.get_mut(&to.z) {
                q.insert_creature(to, id);
            }
        }
    }

    pub fn unregister_creature_index(&mut self, pos: Position, id: CreatureId) {
        if let Some(q) = self.qtrees.get_mut(&pos.z) {
            q.remove_creature(pos, id);
        }
    }
}
