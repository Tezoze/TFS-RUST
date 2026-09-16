//! Stream OTBM nodes into [`SparseGrid`] — no `HashMap<Position, TileData>` stage.
//!
//! Pack: `IOMap::parseTileArea` + `Tile::internalAddThing` (`src/iomap.cpp`, `src/tile.cpp`).
//! Layout only; stack order, flags, zone, house membership, refresh set unchanged.

use std::collections::HashMap;

use rustc_hash::FxHashSet;
use slotmap::SlotMap;
use tfs_rust_common::Position;
use tfs_rust_common::error::Result;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::otbm::{self, MapData, OtbmFile, TileData, TileThing};

use super::{SparseGrid, apply_item_tile_flags};
use crate::ids::ItemId;
use crate::item::Item;
use crate::tile::HouseTile;
use crate::tile::{Tile, TileBody, flags};

use super::Map;

impl Map {
    /// Convert one OTBM file into the runtime grid. Each tile is ingested as the
    /// node walk yields it (`OtbmFile::visit_tiles`).
    pub fn from_otbm(
        mut otbm: OtbmFile,
        items_db: &ItemDatabase,
        items: &mut SlotMap<ItemId, Item>,
    ) -> Result<Self> {
        let width = otbm.width;
        let height = otbm.height;
        let towns = std::mem::take(&mut otbm.towns);
        let waypoints = std::mem::take(&mut otbm.waypoints);

        let mut grid = SparseGrid::new();
        let mut house_tiles = Vec::new();
        let mut refresh_positions = FxHashSet::default();
        otbm.visit_tiles(|pos, house_id, tile_flags, things| {
            ingest_tile(
                &mut grid,
                &mut house_tiles,
                &mut refresh_positions,
                pos,
                house_id,
                tile_flags,
                things,
                items_db,
                items,
            );
            Ok(())
        })?;
        grid.shrink_to_fit();
        Ok(Self {
            width,
            height,
            grid,
            towns,
            waypoints,
            house_tiles,
            refresh_positions,
            refresh_snapshots: HashMap::new(),
        })
    }

    /// Synthetic / unit-test maps. Production boot uses [`Self::from_otbm`].
    pub fn from_map_data(
        data: MapData,
        tiles: impl IntoIterator<Item = TileData>,
        items_db: &ItemDatabase,
        items: &mut SlotMap<ItemId, Item>,
    ) -> Self {
        let mut grid = SparseGrid::new();
        let mut house_tiles = Vec::new();
        let mut refresh_positions = FxHashSet::default();
        for td in tiles {
            ingest_tile(
                &mut grid,
                &mut house_tiles,
                &mut refresh_positions,
                td.position,
                td.house_id,
                td.tile_flags,
                &td.things,
                items_db,
                items,
            );
        }
        grid.shrink_to_fit();
        Self {
            width: data.width,
            height: data.height,
            grid,
            towns: data.towns,
            waypoints: data.waypoints,
            house_tiles,
            refresh_positions,
            refresh_snapshots: HashMap::new(),
        }
    }
}

fn ingest_tile(
    grid: &mut SparseGrid,
    house_tiles: &mut Vec<(u32, Position, Vec<ItemId>)>,
    refresh_positions: &mut FxHashSet<Position>,
    pos: Position,
    house_id: Option<u32>,
    tile_flags: u32,
    things: &[TileThing],
    items_db: &ItemDatabase,
    items: &mut SlotMap<ItemId, Item>,
) {
    let tile = tile_from_data(pos, house_id, tile_flags, things, items_db, items);
    if let Tile::House(h) = &tile {
        let body = tile.body();
        let item_ids: Vec<ItemId> = body
            .down_items()
            .iter()
            .copied()
            .chain(body.top_items().iter().copied())
            .collect();
        house_tiles.push((h.house_id, pos, item_ids));
    } else if tile.body().flags & flags::REFRESH != 0 {
        refresh_positions.insert(pos);
    }
    grid.insert_tile(pos.x, pos.y, pos.z, tile);
}

/// Apply `OTBM_ITEM` props after the u16 id — `Item::unserializeItemNode` (`item.cpp`).
/// Shared by ground and stacked items so ActionID/UniqueID (and text, fluids, …) land.
fn apply_otbm_item_node_attrs(
    item: &mut Item,
    it: Option<&tfs_rust_content::otb::ItemType>,
    otbm_attr_blob: Option<&[u8]>,
    pos: Position,
    id: u16,
) {
    let Some(blob) = otbm_attr_blob.filter(|b| !b.is_empty()) else {
        return;
    };
    let is_container = it
        .map(|t| t.group == tfs_rust_content::otb::ItemType::GROUP_CONTAINER)
        .unwrap_or(false);
    // Remere OTBM attrs 23–28 (key/door) — not DB `AttrTypes_t` NAME/WEIGHT.
    match crate::item_blob::parse_otbm_item_blob(blob, is_container) {
        Ok(parsed) => {
            // `ATTR_TELE_DEST` lives on TFS `Teleport::destPos`, not `itemAttrTypes`
            // (`teleport.cpp` / `enums.h`). `set_tele_dest` therefore does not set bits —
            // dest-only OTBM pads (most magic forcefields) must still keep attributes.
            if parsed.attrs.attribute_bits() != 0 || parsed.attrs.tele_dest().is_some() {
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

/// C++ `Tile::internalAddThing` for item ids (`src/tile.cpp`).
/// Creates an Item instance and stores it on the tile (ground, top, or down).
///
/// `otbm_attr_blob`: bytes after the `u16` item id in an `OTBM_ITEM` node
/// (`Item::unserializeItemNode` / `unserializeAttr` — `item.cpp`). Used for
/// sign/blackboard `ATTR_TEXT`, action ids, unique ids, teleports, etc. `None`
/// for bare `OTBM_ATTR_ITEM` embeds (id only).
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

    let mut item = Item::new_single(id);
    apply_otbm_item_node_attrs(&mut item, it, otbm_attr_blob, pos, id);
    item.parent = Some(crate::cylinder::Cylinder::Tile { pos });

    if is_ground && body.ground.is_none() {
        // TFS `Tile::setGround` stores a full `Item*` — create an Item instance
        // so StepIn/transform/decay can mutate the ground (e.g. pitfall 293↔294).
        let gid = items.insert(item);
        body.ground = Some(id);
        body.ground_item = Some(gid);
        return;
    }

    let item_id = items.insert(item);

    let always_on_top = it.map(|t| t.always_on_top()).unwrap_or(false);
    if always_on_top {
        body.top_items_mut().push(item_id);
    } else {
        body.down_items_mut().insert(0, item_id);
    }
}

/// Convert raw OTBM tile flags to TILESTATE flags.
/// C++ ref: src/iomap.cpp:270-280 — OTBM zone flags use a different bit layout than runtime TILESTATE.
fn convert_otbm_flags(otbm_flags: u32) -> (u32, tfs_rust_common::ZoneType) {
    const OTBM_TILEFLAG_PROTECTIONZONE: u32 = 1 << 0;
    const OTBM_TILEFLAG_NOPVPZONE: u32 = 1 << 2;
    const OTBM_TILEFLAG_NOLOGOUT: u32 = 1 << 3;
    const OTBM_TILEFLAG_PVPZONE: u32 = 1 << 4;
    const OTBM_TILEFLAG_REFRESH: u32 = 1 << 5;

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
    if otbm_flags & OTBM_TILEFLAG_REFRESH != 0 {
        tileflags |= flags::REFRESH;
    }

    (tileflags, zone)
}

fn tile_from_data(
    pos: Position,
    house_id: Option<u32>,
    tile_flags: u32,
    things: &[TileThing],
    items_db: &ItemDatabase,
    items: &mut SlotMap<ItemId, Item>,
) -> Tile {
    let (converted_flags, zone) = convert_otbm_flags(tile_flags);

    let mut body = TileBody {
        ground: None,

        ground_item: None,
        stacks: None,
        flags: converted_flags,
        zone,
    };

    for thing in things {
        match thing {
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
                internal_add_item_id(pos, stream_id, items_db, &mut body, items, Some(attr_blob));
            }
        }
    }

    if let Some(hid) = house_id {
        Tile::House(HouseTile {
            inner: body,
            house_id: hid,
        })
    } else {
        Tile::Normal(body)
    }
}

#[cfg(test)]
mod from_otbm_tests {
    use std::collections::HashMap;
    use std::path::Path;

    use slotmap::SlotMap;
    use tfs_rust_common::Position;
    use tfs_rust_content::items::ItemDatabase;
    use tfs_rust_content::otb::ItemType;
    use tfs_rust_content::otbm::OtbmLoader;

    use crate::ids::ItemId;
    use crate::tile::Tile;

    fn tiny_otbm() -> Vec<u8> {
        const START: u8 = 0xFE;
        const END: u8 = 0xFF;
        fn emit(out: &mut Vec<u8>, ty: u8, props: &[u8], kids: impl FnOnce(&mut Vec<u8>)) {
            out.push(START);
            out.push(ty);
            out.extend_from_slice(props);
            kids(out);
            out.push(END);
        }
        let mut body = Vec::new();
        let mut root_props = Vec::new();
        root_props.extend_from_slice(&2u32.to_le_bytes());
        root_props.extend_from_slice(&256u16.to_le_bytes());
        root_props.extend_from_slice(&256u16.to_le_bytes());
        root_props.extend_from_slice(&3u32.to_le_bytes());
        root_props.extend_from_slice(&57u32.to_le_bytes());
        emit(&mut body, 0, &root_props, |root| {
            emit(root, 2, &[], |map| {
                let mut area = Vec::new();
                area.extend_from_slice(&100u16.to_le_bytes());
                area.extend_from_slice(&100u16.to_le_bytes());
                area.push(7);
                emit(map, 4, &area, |tiles| {
                    let mut house = vec![0u8, 0];
                    house.extend_from_slice(&7u32.to_le_bytes());
                    house.push(9); // OTBM_ATTR_ITEM
                    house.extend_from_slice(&100u16.to_le_bytes());
                    emit(tiles, 14, &house, |_| {});
                });
            });
        });
        let mut file = b"OTBM".to_vec();
        file.extend_from_slice(&body);
        file
    }

    #[test]
    fn from_otbm_streams_housetile_into_grid() {
        let otbm = OtbmLoader::from_bytes(Path::new("tiny.otbm"), tiny_otbm()).expect("otbm");
        let db = ItemDatabase {
            items: HashMap::from([(
                100,
                ItemType {
                    id: 100,
                    server_id: 100,
                    group: ItemType::GROUP_GROUND,
                    ..ItemType::default()
                },
            )]),
            client_to_server: HashMap::new(),
        };
        let mut items: SlotMap<ItemId, crate::item::Item> = SlotMap::with_key();
        let map = super::Map::from_otbm(otbm, &db, &mut items).expect("from_otbm");
        assert_eq!(map.width, 256);
        assert_eq!(map.grid.populated_tile_count(), 1);
        assert_eq!(map.house_tiles.len(), 1);
        assert_eq!(map.house_tiles[0].0, 7);
        assert_eq!(map.house_tiles[0].1, Position::new(100, 100, 7));
        let tile = map.get_tile(Position::new(100, 100, 7)).expect("tile");
        assert!(matches!(tile, Tile::House(_)));
        assert_eq!(tile.body().ground, Some(100));
    }
}
