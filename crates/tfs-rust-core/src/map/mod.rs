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

pub use grid::{SparseGrid, CHUNK_AREA, CHUNK_SIZE, SECTOR_SIZE};
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
    pub fn from_map_data(
        data: MapData,
        items_db: &ItemDatabase,
        items: &mut SlotMap<ItemId, Item>,
    ) -> Self {
        let mut grid = SparseGrid::new();
        for (pos, td) in data.tiles {
            let tile = tile_from_data(pos, td, items_db, items);
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

    /// Update tile stack + chunk spatial index (`Map::moveCreature` creature lists — `map.cpp`).
    ///
    /// Audit #3: a creature placed on a void (unloaded) tile would be silently dropped from
    /// both the tile stack and the chunk spatial index. Surface the violation instead —
    /// `tracing::error!` in release, `debug_assert!` panic in debug/test. Never panics in
    /// release (per `tfs-packets.md` validation rules).
    pub fn register_creature_at(&mut self, pos: Position, id: CreatureId) {
        let tile_present = self.get_tile(pos).is_some();
        if let Some(t) = self.get_tile_mut(pos) {
            let body = t.body();
            if !body.creatures.contains(&id) {
                t.add_creature(id);
            }
        }
        self.grid.register_creature(pos.x, pos.y, pos.z, id);
        if !tile_present {
            tracing::error!(
                x = pos.x, y = pos.y, z = pos.z, creature = ?id,
                "register_creature_at: target tile is void (unloaded); \
                 creature dropped from tile stack + chunk spatial index"
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

/// C++ `Tile::internalAddThing` for item ids (`src/tile.cpp`).
/// Creates an Item instance and returns its ItemId.
///
/// `otbm_attr_blob`: bytes after the `u16` item id in an `OTBM_ITEM` node
/// (`Item::unserializeItemNode` / `unserializeAttr` — `item.cpp`). Used for
/// sign/blackboard `ATTR_TEXT`, action ids, teleports, etc. `None` for bare
/// `OTBM_ATTR_ITEM` embeds (id only).
fn internal_add_item_id(
    pos: Position,
    id: u16,
    items_db: &ItemDatabase,
    body: &mut TileBody,
    items: &mut SlotMap<ItemId, Item>,
    otbm_attr_blob: Option<&[u8]>,
) {
    let id = otbm::remap_create_item_stream_id(id);
    let it = items_db.items.get(&id);
    let is_ground = it.map(|t| t.is_ground_tile()).unwrap_or(false);
    if is_ground && body.ground.is_none() {
        body.ground = Some(id);
        return;
    }

    let mut item = Item::new_single(id);
    if let Some(blob) = otbm_attr_blob.filter(|b| !b.is_empty()) {
        let is_container = it.map(|t| t.group == tfs_rust_content::otb::ItemType::GROUP_CONTAINER)
            .unwrap_or(false);
        // Remere OTBM attrs 23–28 (key/door) — not DB `AttrTypes_t` NAME/WEIGHT.
        match crate::item_blob::parse_otbm_item_blob(blob, is_container) {
            Ok(parsed) => {
                if parsed.attrs.attribute_bits() != 0 {
                    item.attributes = Some(Box::new(parsed.attrs));
                }
                if let Some(st) = parsed.subtype_override {
                    let is_fluid = it.is_some_and(|t| t.is_fluid_container() || t.is_splash());
                    if is_fluid {
                        // Fluid subtype 0 = empty; do not force count≥1 (would look like water).
                        item.count = u16::from(st);
                        item.set_fluid_type(u16::from(st));
                    } else {
                        item.count = u16::from(st).max(1);
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    item_id = id,
                    ?pos,
                    error = %e,
                    "OTBM item attr unserialize failed (item placed without attrs)"
                );
            }
        }
    }
    item.parent = Some(crate::cylinder::Cylinder::Tile { pos });
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
        if it.is_hangable() {
            if it.is_horizontal() {
                out.hook_east = true;
            }
            if it.is_vertical() {
                out.hook_south = true;
            }
        }
        if it.is_vertical() || it.is_horizontal() {
            out.supports_hangable = true;
        }
    };

    if let Some(ground_type) = body.ground {
        if let Some(it) = items_db.items.get(&ground_type) {
            consider(it);
        }
    }
    for &iid in body.down_items.iter().chain(body.top_items.iter()) {
        if iid == exclude {
            continue;
        }
        if let Some(item) = items.get(iid) {
            if let Some(it) = items_db.items.get(&item.item_type) {
                consider(it);
            }
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
    if departing.is_hangable() && departing.is_horizontal() && !remaining.hook_east {
        body.flags &= !flags::HOOKEAST;
    }
    if departing.is_hangable() && departing.is_vertical() && !remaining.hook_south {
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

    // 772 wall hooks — `HOOKEAST` (horizontal) / `HOOKSOUTH` (vertical) hangable spots.
    // NOTE(parity): CipSoft `HOOKEAST`/`HOOKSOUTH` ≈ OTB hangable + horizontal/vertical; affects only
    // the `StartT=0` origin-tile special case of `ThrowPossible` when throwing west/north.
    if item_type.is_hangable() {
        if item_type.is_horizontal() {
            body.flags |= flags::HOOKEAST;
        }
        if item_type.is_vertical() {
            body.flags |= flags::HOOKSOUTH;
        }
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

/// Raw OTBM item stream id before `remap_create_item_stream_id` (`src/item.cpp` `CreateItem(PropStream&)`).
#[cfg(test)]
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
    pos: Position,
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
        match &thing {
            TileThing::EmbeddedItemId(stream_id) => {
                let id = otbm::remap_create_item_stream_id(*stream_id);
                if let Some(item_type) = items_db.items.get(&id) {
                    apply_item_tile_flags(&mut body, item_type, items_db);
                }
                internal_add_item_id(pos, *stream_id, items_db, &mut body, items, None);
            }
            TileThing::ItemNodeProps(raw) => {
                if raw.len() < 2 {
                    continue;
                }
                let stream_id = u16::from_le_bytes([raw[0], raw[1]]);
                let id = otbm::remap_create_item_stream_id(stream_id);
                if let Some(item_type) = items_db.items.get(&id) {
                    apply_item_tile_flags(&mut body, item_type, items_db);
                }
                // Bytes after the item id — C++ `unserializeItemNode` (`item.cpp:754`).
                let attr_blob = &raw[2..];
                internal_add_item_id(
                    pos,
                    stream_id,
                    items_db,
                    &mut body,
                    items,
                    Some(attr_blob),
                );
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

#[cfg(test)]
mod tile_flag_tests {
    use std::collections::HashMap;

    use slotmap::SlotMap;
    use tfs_rust_common::Position;
    use tfs_rust_content::items::{ItemDatabase, ITEM_TYPE_TELEPORT};
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

    fn map_from_single_tile(
        pos: Position,
        things: Vec<TileThing>,
        db: &ItemDatabase,
    ) -> super::Map {
        let mut items: SlotMap<ItemId, crate::item::Item> = SlotMap::with_key();
        let mut tiles = HashMap::new();
        tiles.insert(
            pos,
            TileData {
                position: pos,
                house_id: None,
                tile_flags: 0,
                things,
            },
        );
        let data = MapData {
            width: 256,
            height: 256,
            spawn_file: None,
            house_file: None,
            spawn_zones: Vec::new(),
            tiles,
            houses: HashMap::new(),
            towns: HashMap::new(),
            waypoints: HashMap::new(),
        };
        super::Map::from_map_data(data, db, &mut items)
    }

    fn map_and_items_from_single_tile(
        pos: Position,
        things: Vec<TileThing>,
        db: &ItemDatabase,
    ) -> (super::Map, SlotMap<ItemId, crate::item::Item>) {
        let mut items: SlotMap<ItemId, crate::item::Item> = SlotMap::with_key();
        let mut tiles = HashMap::new();
        tiles.insert(
            pos,
            TileData {
                position: pos,
                house_id: None,
                tile_flags: 0,
                things,
            },
        );
        let data = MapData {
            width: 256,
            height: 256,
            spawn_file: None,
            house_file: None,
            spawn_zones: Vec::new(),
            tiles,
            houses: HashMap::new(),
            towns: HashMap::new(),
            waypoints: HashMap::new(),
        };
        let map = super::Map::from_map_data(data, db, &mut items);
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
        let (map, items) = map_and_items_from_single_tile(
            pos,
            vec![TileThing::ItemNodeProps(raw)],
            &db,
        );
        let tile = map.get_tile(pos).expect("tile");
        let item_id = tile
            .body()
            .top_items
            .first()
            .or_else(|| tile.body().down_items.first())
            .copied()
            .expect("sign item");
        let item = items.get(item_id).expect("item");
        assert_eq!(item.item_type, SIGN);
        assert_eq!(item.text(), "Depot");
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
}
