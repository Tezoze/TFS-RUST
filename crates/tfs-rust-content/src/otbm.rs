//! OTBM map loader (tiles, towns, waypoints, external spawn/house file refs).
// C++ reference: src/iomap.cpp IOMap::{loadMap, parseTileArea, parseMapDataAttributes}

use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_common::Position;
use tracing::info;

/// One stack entry on a map tile (ground or top items), in map load order.
#[derive(Debug, Clone)]
pub enum TileThing {
    /// `OTBM_ATTR_ITEM` — only `uint16` item type id is read by `Item::CreateItem(PropStream)` (src/item.cpp).
    EmbeddedItemId(u16),
    /// Full unescaped OTBM props for an `OTBM_ITEM` child node (`Item::CreateItem` + `unserializeItemNode`).
    ItemNodeProps(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct TileData {
    pub position: Position,
    pub house_id: Option<u32>,
    pub tile_flags: u32,
    pub things: Vec<TileThing>,
}

#[derive(Debug, Clone)]
pub struct HouseData {
    pub id: u32,
}

#[derive(Debug, Clone)]
pub struct TownData {
    pub id: u32,
    pub name: String,
    pub temple_position: Position,
}

#[derive(Debug, Clone)]
pub struct MapData {
    pub width: u16,
    pub height: u16,
    /// Filename from `OTBM_ATTR_EXT_SPAWN_FILE` (relative to OTBM directory), if set.
    pub spawn_file: Option<String>,
    /// Filename from `OTBM_ATTR_EXT_HOUSE_FILE`, if set.
    pub house_file: Option<String>,
    /// Filled by `pipeline::load_all` when `*-spawn.xml` exists (see `spawn_file` / default name).
    pub spawn_zones: Vec<crate::spawns::SpawnZone>,
    pub tiles: HashMap<Position, TileData>,
    pub houses: HashMap<u32, HouseData>,
    pub towns: HashMap<u32, TownData>,
    pub waypoints: HashMap<String, Position>,
}

pub struct OtbmLoader;

impl OtbmLoader {
    pub fn load_from_file(path: &Path) -> Result<MapData> {
        info!("Loading OTBM map from {:?}", path);
        let data = std::fs::read(path).map_err(|e| TfsRustError::Content {
            file: path.to_string_lossy().into(),
            message: e.to_string(),
        })?;

        let root = parse_otb_tree(&data, path)?;
        let root_props = unescaped_props(&data, &root, path)?;
        if root_props.len() < 16 {
            return Err(TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: "invalid OTBM root header".to_string(),
            });
        }

        let width = u16::from_le_bytes([root_props[4], root_props[5]]);
        let height = u16::from_le_bytes([root_props[6], root_props[7]]);
        let mut map_data = MapData {
            width,
            height,
            spawn_file: None,
            house_file: None,
            spawn_zones: Vec::new(),
            tiles: HashMap::new(),
            houses: HashMap::new(),
            towns: HashMap::new(),
            waypoints: HashMap::new(),
        };

        let Some(map_node) = root
            .children
            .iter()
            .find(|node| node.node_type == OTBM_MAP_DATA)
        else {
            return Err(TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: "missing OTBM_MAP_DATA node".to_string(),
            });
        };

        let map_props = unescaped_props(&data, map_node, path)?;
        let (spawn, house) = parse_map_data_attributes(&map_props, path)?;
        map_data.spawn_file = spawn;
        map_data.house_file = house;

        for child in &map_node.children {
            match child.node_type {
                OTBM_TILE_AREA => parse_tile_area(&data, child, &mut map_data, path)?,
                OTBM_TOWNS => parse_towns(&data, child, &mut map_data, path)?,
                OTBM_WAYPOINTS => parse_waypoints(&data, child, &mut map_data, path)?,
                _ => {}
            }
        }

        Ok(map_data)
    }
}

const ESCAPE: u8 = 0xFD;
const NODE_START: u8 = 0xFE;
const NODE_END: u8 = 0xFF;

const OTBM_MAP_DATA: u8 = 2;
const OTBM_TILE_AREA: u8 = 4;
const OTBM_TILE: u8 = 5;
const OTBM_ITEM: u8 = 6;
const OTBM_TOWNS: u8 = 12;
const OTBM_TOWN: u8 = 13;
const OTBM_HOUSETILE: u8 = 14;
const OTBM_WAYPOINTS: u8 = 15;
const OTBM_WAYPOINT: u8 = 16;

// src/iomap.h OTBM_AttrTypes_t
const OTBM_ATTR_DESCRIPTION: u8 = 1;
const OTBM_ATTR_TILE_FLAGS: u8 = 3;
const OTBM_ATTR_ITEM: u8 = 9;
const OTBM_ATTR_EXT_SPAWN_FILE: u8 = 11;
const OTBM_ATTR_EXT_HOUSE_FILE: u8 = 13;

#[derive(Debug, Clone)]
struct Node {
    node_type: u8,
    props_begin: usize,
    props_end: usize,
    children: Vec<Node>,
}

fn parse_map_data_attributes(
    props: &[u8],
    path: &Path,
) -> Result<(Option<String>, Option<String>)> {
    let mut cursor = 0usize;
    let mut spawn = None;
    let mut house = None;
    while cursor < props.len() {
        let attr = read_u8_at(props, &mut cursor, path)?;
        match attr {
            OTBM_ATTR_DESCRIPTION => {
                let _desc = read_prop_string(props, &mut cursor, path)?;
            }
            OTBM_ATTR_EXT_SPAWN_FILE => {
                spawn = Some(read_prop_string(props, &mut cursor, path)?);
            }
            OTBM_ATTR_EXT_HOUSE_FILE => {
                house = Some(read_prop_string(props, &mut cursor, path)?);
            }
            _ => {
                return Err(TfsRustError::Content {
                    file: path.to_string_lossy().into_owned(),
                    message: format!("unknown OTBM map data attribute {attr} (see src/iomap.cpp)"),
                });
            }
        }
    }
    Ok((spawn, house))
}

fn parse_otb_tree(data: &[u8], path: &Path) -> Result<Node> {
    if data.len() < 6 {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "OTBM file too small".to_string(),
        });
    }

    let id = &data[0..4];
    if id != b"OTBM" && id != [0, 0, 0, 0] {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "invalid OTBM identifier".to_string(),
        });
    }

    if data[4] != NODE_START {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "invalid OTBM root start marker".to_string(),
        });
    }
    parse_node_recursive(data, 4, path)
}

fn parse_node_recursive(data: &[u8], start_idx: usize, path: &Path) -> Result<Node> {
    let mut idx = start_idx;
    if idx >= data.len() || data[idx] != NODE_START {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "expected node start".to_string(),
        });
    }
    idx += 1;
    if idx >= data.len() {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "missing node type".to_string(),
        });
    }
    let node_type = data[idx];
    idx += 1;

    let props_begin = idx;
    let mut props_end = idx;
    let mut children = Vec::new();

    while idx < data.len() {
        match data[idx] {
            NODE_START => {
                if children.is_empty() {
                    props_end = idx;
                }
                let child = parse_node_recursive(data, idx, path)?;
                idx = child_end_offset(data, idx, path)?;
                children.push(child);
            }
            NODE_END => {
                if children.is_empty() {
                    props_end = idx;
                }
                break;
            }
            ESCAPE => idx += 2,
            _ => idx += 1,
        }
    }

    Ok(Node {
        node_type,
        props_begin,
        props_end,
        children,
    })
}

fn child_end_offset(data: &[u8], start_idx: usize, path: &Path) -> Result<usize> {
    let mut depth = 0usize;
    let mut idx = start_idx;
    while idx < data.len() {
        match data[idx] {
            NODE_START => {
                depth += 1;
                idx += 1;
            }
            NODE_END => {
                depth = depth.saturating_sub(1);
                idx += 1;
                if depth == 0 {
                    return Ok(idx);
                }
            }
            ESCAPE => idx += 2,
            _ => idx += 1,
        }
    }
    Err(TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: "unterminated child node".to_string(),
    })
}

fn unescaped_props(data: &[u8], node: &Node, path: &Path) -> Result<Vec<u8>> {
    if node.props_begin > node.props_end || node.props_end > data.len() {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "invalid OTBM property range".to_string(),
        });
    }
    let mut out = Vec::with_capacity(node.props_end.saturating_sub(node.props_begin));
    let mut idx = node.props_begin;
    while idx < node.props_end {
        let b = data[idx];
        if b == ESCAPE {
            idx += 1;
            if idx >= node.props_end {
                return Err(TfsRustError::Content {
                    file: path.to_string_lossy().into_owned(),
                    message: "dangling OTBM escape in props".to_string(),
                });
            }
            out.push(data[idx]);
        } else {
            out.push(b);
        }
        idx += 1;
    }
    Ok(out)
}

fn parse_tile_area(data: &[u8], area: &Node, map: &mut MapData, path: &Path) -> Result<()> {
    let props = unescaped_props(data, area, path)?;
    if props.len() < 5 {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "invalid tile area props".to_string(),
        });
    }
    let base_x = u16::from_le_bytes([props[0], props[1]]);
    let base_y = u16::from_le_bytes([props[2], props[3]]);
    let z = props[4];

    for tile in &area.children {
        if tile.node_type != OTBM_TILE && tile.node_type != OTBM_HOUSETILE {
            continue;
        }
        let tile_props = unescaped_props(data, tile, path)?;
        if tile_props.len() < 2 {
            continue;
        }
        let x = base_x.saturating_add(tile_props[0] as u16);
        let y = base_y.saturating_add(tile_props[1] as u16);
        let mut house_id = None;
        let mut cursor = 2usize;
        if tile.node_type == OTBM_HOUSETILE {
            if tile_props.len() < 6 {
                return Err(TfsRustError::Content {
                    file: path.to_string_lossy().into_owned(),
                    message: "housetile missing house id".to_string(),
                });
            }
            let id =
                u32::from_le_bytes([tile_props[2], tile_props[3], tile_props[4], tile_props[5]]);
            house_id = Some(id);
            map.houses.entry(id).or_insert(HouseData { id });
            cursor = 6;
        }

        let mut tile_flags: u32 = 0;
        let mut things = Vec::new();

        while cursor < tile_props.len() {
            let attr = read_u8_at(&tile_props, &mut cursor, path)?;
            match attr {
                OTBM_ATTR_TILE_FLAGS => {
                    let flags = read_u32_at(&tile_props, &mut cursor, path)?;
                    tile_flags = flags;
                }
                OTBM_ATTR_ITEM => {
                    let id = read_u16_at(&tile_props, &mut cursor, path)?;
                    things.push(TileThing::EmbeddedItemId(id));
                }
                other => {
                    return Err(TfsRustError::Content {
                        file: path.to_string_lossy().into_owned(),
                        message: format!(
                            "unknown tile attribute {other} at ({x},{y},{z}) (src/iomap.cpp)"
                        ),
                    });
                }
            }
        }

        for item_node in &tile.children {
            if item_node.node_type != OTBM_ITEM {
                return Err(TfsRustError::Content {
                    file: path.to_string_lossy().into_owned(),
                    message: format!("expected OTBM_ITEM child at ({x},{y},{z})"),
                });
            }
            let raw = unescaped_props(data, item_node, path)?;
            things.push(TileThing::ItemNodeProps(raw));
        }

        let pos = Position::new(x, y, z);
        map.tiles.insert(
            pos,
            TileData {
                position: pos,
                house_id,
                tile_flags,
                things,
            },
        );
    }
    Ok(())
}

fn parse_towns(data: &[u8], towns: &Node, map: &mut MapData, path: &Path) -> Result<()> {
    for town in &towns.children {
        if town.node_type != OTBM_TOWN {
            continue;
        }
        let props = unescaped_props(data, town, path)?;
        let mut cursor = 0usize;
        if props.len() < 4 {
            continue;
        }
        let id = read_u32_at_slice(&props, &mut cursor, path)?;
        let name = read_prop_string(&props, &mut cursor, path)?;
        let x = read_u16_at_slice(&props, &mut cursor, path)?;
        let y = read_u16_at_slice(&props, &mut cursor, path)?;
        let z = read_u8_at_slice(&props, &mut cursor, path)?;
        map.towns.insert(
            id,
            TownData {
                id,
                name,
                temple_position: Position::new(x, y, z),
            },
        );
    }
    Ok(())
}

fn parse_waypoints(data: &[u8], waypoints: &Node, map: &mut MapData, path: &Path) -> Result<()> {
    for waypoint in &waypoints.children {
        if waypoint.node_type != OTBM_WAYPOINT {
            continue;
        }
        let props = unescaped_props(data, waypoint, path)?;
        let mut cursor = 0usize;
        let name = read_prop_string(&props, &mut cursor, path)?;
        let x = read_u16_at_slice(&props, &mut cursor, path)?;
        let y = read_u16_at_slice(&props, &mut cursor, path)?;
        let z = read_u8_at_slice(&props, &mut cursor, path)?;
        map.waypoints.insert(name, Position::new(x, y, z));
    }
    Ok(())
}

fn read_u8_at(data: &[u8], cursor: &mut usize, path: &Path) -> Result<u8> {
    if *cursor >= data.len() {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "unexpected EOF in OTBM props".to_string(),
        });
    }
    let v = data[*cursor];
    *cursor += 1;
    Ok(v)
}

fn read_u16_at(data: &[u8], cursor: &mut usize, path: &Path) -> Result<u16> {
    let lo = read_u8_at(data, cursor, path)?;
    let hi = read_u8_at(data, cursor, path)?;
    Ok(u16::from_le_bytes([lo, hi]))
}

fn read_u32_at(data: &[u8], cursor: &mut usize, path: &Path) -> Result<u32> {
    let b0 = read_u8_at(data, cursor, path)?;
    let b1 = read_u8_at(data, cursor, path)?;
    let b2 = read_u8_at(data, cursor, path)?;
    let b3 = read_u8_at(data, cursor, path)?;
    Ok(u32::from_le_bytes([b0, b1, b2, b3]))
}

fn read_u32_at_slice(data: &[u8], cursor: &mut usize, path: &Path) -> Result<u32> {
    read_u32_at(data, cursor, path)
}

fn read_u16_at_slice(data: &[u8], cursor: &mut usize, path: &Path) -> Result<u16> {
    read_u16_at(data, cursor, path)
}

fn read_u8_at_slice(data: &[u8], cursor: &mut usize, path: &Path) -> Result<u8> {
    read_u8_at(data, cursor, path)
}

fn read_prop_string(data: &[u8], cursor: &mut usize, path: &Path) -> Result<String> {
    let len = read_u16_at(data, cursor, path)? as usize;
    if *cursor + len > data.len() {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "unexpected EOF reading string".to_string(),
        });
    }
    let value = String::from_utf8_lossy(&data[*cursor..*cursor + len]).to_string();
    *cursor += len;
    Ok(value)
}
