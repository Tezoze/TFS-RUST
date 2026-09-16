//! OTBM map loader (tiles, towns, waypoints, external spawn/house file refs).
//!
//! Production load streams tiles through [`OtbmFile::visit_tiles`] into the runtime
//! grid — never a `HashMap<Position, TileData>` stage (Phase C RSS).
//! C++ reference: src/iomap.cpp IOMap::{loadMap, parseTileArea, parseMapDataAttributes}

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tfs_rust_common::Position;
use tfs_rust_common::error::{Result, TfsRustError};
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

#[derive(Debug, Clone, Default)]
pub struct MapData {
    pub width: u16,
    pub height: u16,
    /// Filename from `OTBM_ATTR_EXT_SPAWN_FILE` (relative to OTBM directory), if set.
    pub spawn_file: Option<String>,
    /// Filename from `OTBM_ATTR_EXT_HOUSE_FILE`, if set.
    pub house_file: Option<String>,
    /// Filled by `pipeline::load_all` when `*-spawn.xml` exists (see `spawn_file` / default name; map path is configurable).
    pub spawn_zones: Vec<crate::spawns::SpawnZone>,
    pub houses: HashMap<u32, HouseData>,
    pub towns: HashMap<u32, TownData>,
    pub waypoints: HashMap<String, Position>,
}

/// Parsed OTBM kept as a node tree + file bytes. Tiles are streamed via
/// [`OtbmFile::visit_tiles`] — callers convert immediately into the runtime grid.
pub struct OtbmFile {
    path: PathBuf,
    data: Vec<u8>,
    root: Node,
    pub width: u16,
    pub height: u16,
    pub spawn_file: Option<String>,
    pub house_file: Option<String>,
    pub towns: HashMap<u32, TownData>,
    pub waypoints: HashMap<String, Position>,
}

pub struct OtbmLoader;

impl OtbmLoader {
    /// Read the OTBM and parse the node tree. Does not stage tiles.
    pub fn open(path: &Path) -> Result<OtbmFile> {
        info!("Loading OTBM map from {:?}", path);
        let data = std::fs::read(path).map_err(|e| TfsRustError::Content {
            file: path.to_string_lossy().into(),
            message: e.to_string(),
        })?;
        Self::from_bytes(path, data)
    }

    /// Parse an already-read OTBM buffer. `path` is for error messages only.
    pub fn from_bytes(path: &Path, data: Vec<u8>) -> Result<OtbmFile> {
        let t0 = std::time::Instant::now();
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
        let (spawn_file, house_file) = parse_map_data_attributes(&map_props, path)?;

        let mut towns = HashMap::new();
        let mut waypoints = HashMap::new();
        for child in &map_node.children {
            match child.node_type {
                OTBM_TOWNS => parse_towns(&data, child, &mut towns, path)?,
                OTBM_WAYPOINTS => parse_waypoints(&data, child, &mut waypoints, path)?,
                _ => {}
            }
        }

        info!(
            width,
            height,
            elapsed_ms = t0.elapsed().as_millis(),
            "OTBM tree parsed (no tile HashMap)"
        );
        Ok(OtbmFile {
            path: path.to_path_buf(),
            data,
            root,
            width,
            height,
            spawn_file,
            house_file,
            towns,
            waypoints,
        })
    }
}

impl OtbmFile {
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Walk `OTBM_TILE` / `OTBM_HOUSETILE` nodes in file order.
    ///
    /// `on_tile` must convert (or copy) the slice before returning — `things` is
    /// reused for the next tile. House ids are collected from `HOUSETILE` nodes
    /// the same way `IOMap::parseTileArea` registers houses (`src/iomap.cpp`).
    pub fn visit_tiles<F>(&self, mut on_tile: F) -> Result<HashMap<u32, HouseData>>
    where
        F: FnMut(Position, Option<u32>, u32, &[TileThing]) -> Result<()>,
    {
        let Some(map_node) = self
            .root
            .children
            .iter()
            .find(|node| node.node_type == OTBM_MAP_DATA)
        else {
            return Err(TfsRustError::Content {
                file: self.path.to_string_lossy().into_owned(),
                message: "missing OTBM_MAP_DATA node".to_string(),
            });
        };

        let mut houses = HashMap::new();
        let mut things = Vec::new();
        let mut tile_props = Vec::new();
        for child in &map_node.children {
            if child.node_type == OTBM_TILE_AREA {
                parse_tile_area(
                    &self.data,
                    child,
                    &mut houses,
                    &mut things,
                    &mut tile_props,
                    &mut on_tile,
                    &self.path,
                )?;
            }
        }
        Ok(houses)
    }

    /// Metadata only (no tile stage). Spawn XML / house XML attach later.
    pub fn map_data(&self) -> MapData {
        MapData {
            width: self.width,
            height: self.height,
            spawn_file: self.spawn_file.clone(),
            house_file: self.house_file.clone(),
            spawn_zones: Vec::new(),
            houses: HashMap::new(),
            towns: self.towns.clone(),
            waypoints: self.waypoints.clone(),
        }
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

fn unescape_props_into(data: &[u8], node: &Node, out: &mut Vec<u8>, path: &Path) -> Result<()> {
    if node.props_begin > node.props_end || node.props_end > data.len() {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "invalid OTBM property range".to_string(),
        });
    }
    out.clear();
    let need = node.props_end.saturating_sub(node.props_begin);
    if out.capacity() < need {
        out.reserve(need);
    }
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
    Ok(())
}

fn unescaped_props(data: &[u8], node: &Node, path: &Path) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    unescape_props_into(data, node, &mut out, path)?;
    Ok(out)
}

fn parse_tile_area<F>(
    data: &[u8],
    area: &Node,
    houses: &mut HashMap<u32, HouseData>,
    things: &mut Vec<TileThing>,
    tile_props: &mut Vec<u8>,
    on_tile: &mut F,
    path: &Path,
) -> Result<()>
where
    F: FnMut(Position, Option<u32>, u32, &[TileThing]) -> Result<()>,
{
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
        unescape_props_into(data, tile, tile_props, path)?;
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
            houses.entry(id).or_insert(HouseData { id });
            cursor = 6;
        }

        let mut tile_flags: u32 = 0;
        things.clear();

        while cursor < tile_props.len() {
            let attr = read_u8_at(tile_props, &mut cursor, path)?;
            match attr {
                OTBM_ATTR_TILE_FLAGS => {
                    let flags = read_u32_at(tile_props, &mut cursor, path)?;
                    tile_flags = flags;
                }
                OTBM_ATTR_ITEM => {
                    let id = read_u16_at(tile_props, &mut cursor, path)?;
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

        on_tile(Position::new(x, y, z), house_id, tile_flags, things)?;
    }
    Ok(())
}

fn parse_towns(
    data: &[u8],
    towns: &Node,
    out: &mut HashMap<u32, TownData>,
    path: &Path,
) -> Result<()> {
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
        out.insert(
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

fn parse_waypoints(
    data: &[u8],
    waypoints: &Node,
    out: &mut HashMap<String, Position>,
    path: &Path,
) -> Result<()> {
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
        out.insert(name, Position::new(x, y, z));
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

// --- OTBM item stream ids (C++ `Item::CreateItem(PropStream&)` — `src/item.cpp`) ---

/// PVP field / magic wall ids in map files map to persistent ids (same as C++ switch in `Item::CreateItem(PropStream&)`).
pub fn remap_create_item_stream_id(id: u16) -> u16 {
    match id {
        1487 => 1492, // ITEM_FIREFIELD_PVP_FULL -> PERSISTENT_FULL
        1488 => 1493,
        1489 => 1494,
        1490 => 1496, // ITEM_POISONFIELD_PVP -> PERSISTENT
        1491 => 1495, // ITEM_ENERGYFIELD_PVP -> PERSISTENT
        1497 => 1498, // ITEM_MAGICWALL -> PERSISTENT
        1499 => 2721, // ITEM_WILDGROWTH -> ITEM_WILDGROWTH_PERSISTENT
        _ => id,
    }
}

/// First `u16` of an `OTBM_ITEM` props buffer is the item type id (`Item::CreateItem(PropStream)`).
pub fn item_id_from_otbm_item_props(raw: &[u8]) -> Option<u16> {
    if raw.len() < 2 {
        return None;
    }
    Some(remap_create_item_stream_id(u16::from_le_bytes([
        raw[0], raw[1],
    ])))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_escaped(out: &mut Vec<u8>, b: u8) {
        if b == ESCAPE || b == NODE_START || b == NODE_END {
            out.push(ESCAPE);
        }
        out.push(b);
    }

    fn emit_node(out: &mut Vec<u8>, ty: u8, props: &[u8], children: impl FnOnce(&mut Vec<u8>)) {
        out.push(NODE_START);
        out.push(ty);
        for &b in props {
            push_escaped(out, b);
        }
        children(out);
        out.push(NODE_END);
    }

    fn fixture_bytes() -> Vec<u8> {
        let mut body = Vec::new();
        let mut root_props = Vec::new();
        root_props.extend_from_slice(&2u32.to_le_bytes());
        root_props.extend_from_slice(&256u16.to_le_bytes());
        root_props.extend_from_slice(&256u16.to_le_bytes());
        root_props.extend_from_slice(&3u32.to_le_bytes());
        root_props.extend_from_slice(&57u32.to_le_bytes());

        emit_node(&mut body, 0, &root_props, |root| {
            emit_node(root, OTBM_MAP_DATA, &[], |map| {
                let mut area_props = Vec::new();
                area_props.extend_from_slice(&100u16.to_le_bytes());
                area_props.extend_from_slice(&100u16.to_le_bytes());
                area_props.push(7);
                emit_node(map, OTBM_TILE_AREA, &area_props, |area| {
                    let mut tile_props = vec![0u8, 0];
                    tile_props.push(OTBM_ATTR_ITEM);
                    tile_props.extend_from_slice(&100u16.to_le_bytes());
                    emit_node(area, OTBM_TILE, &tile_props, |_| {});

                    let mut house_props = vec![1u8, 0];
                    house_props.extend_from_slice(&7u32.to_le_bytes());
                    emit_node(area, OTBM_HOUSETILE, &house_props, |tile| {
                        emit_node(tile, OTBM_ITEM, &100u16.to_le_bytes(), |_| {});
                    });
                });

                emit_node(map, OTBM_TOWNS, &[], |towns| {
                    let mut props = Vec::new();
                    props.extend_from_slice(&1u32.to_le_bytes());
                    let name = b"Thais";
                    props.extend_from_slice(&(name.len() as u16).to_le_bytes());
                    props.extend_from_slice(name);
                    props.extend_from_slice(&100u16.to_le_bytes());
                    props.extend_from_slice(&100u16.to_le_bytes());
                    props.push(7);
                    emit_node(towns, OTBM_TOWN, &props, |_| {});
                });

                emit_node(map, OTBM_WAYPOINTS, &[], |wps| {
                    let mut props = Vec::new();
                    let name = b"depot";
                    props.extend_from_slice(&(name.len() as u16).to_le_bytes());
                    props.extend_from_slice(name);
                    props.extend_from_slice(&110u16.to_le_bytes());
                    props.extend_from_slice(&120u16.to_le_bytes());
                    props.push(7);
                    emit_node(wps, OTBM_WAYPOINT, &props, |_| {});
                });
            });
        });

        let mut file = b"OTBM".to_vec();
        file.extend_from_slice(&body);
        file
    }

    fn fixture() -> OtbmFile {
        OtbmLoader::from_bytes(Path::new("tiny.otbm"), fixture_bytes()).expect("parse fixture")
    }

    #[test]
    fn open_parses_towns_and_waypoints_without_staging_tiles() {
        let otbm = fixture();
        assert_eq!(otbm.width, 256);
        assert_eq!(otbm.height, 256);
        assert_eq!(otbm.towns.len(), 1);
        assert_eq!(otbm.towns[&1].name, "Thais");
        assert_eq!(
            otbm.waypoints.get("depot"),
            Some(&Position::new(110, 120, 7))
        );
        let meta = otbm.map_data();
        assert!(meta.houses.is_empty());
        assert!(meta.spawn_zones.is_empty());
    }

    #[test]
    fn visit_tiles_streams_file_order_and_house_ids() {
        let otbm = fixture();
        let mut seen = Vec::new();
        let houses = otbm
            .visit_tiles(|pos, house_id, flags, things| {
                seen.push((pos, house_id, flags, things.to_vec()));
                Ok(())
            })
            .expect("visit");
        assert_eq!(houses.len(), 1);
        assert!(houses.contains_key(&7));
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].0, Position::new(100, 100, 7));
        assert_eq!(seen[0].1, None);
        assert!(matches!(
            seen[0].3.as_slice(),
            [TileThing::EmbeddedItemId(100)]
        ));
        assert_eq!(seen[1].0, Position::new(101, 100, 7));
        assert_eq!(seen[1].1, Some(7));
        match seen[1].3.as_slice() {
            [TileThing::ItemNodeProps(raw)] => {
                assert_eq!(raw.as_slice(), [100u8, 0]);
            }
            other => panic!("expected item-node props, got {other:?}"),
        }
        let mut n = 0usize;
        otbm.visit_tiles(|_, _, _, _| {
            n += 1;
            Ok(())
        })
        .expect("second visit");
        assert_eq!(n, 2);
    }
}
