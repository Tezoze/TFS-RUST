//! Server-id → client-id rewrite for datapack text and OTBM item ids.
//!
//! OTBM applies the PVP/magic-wall pair table first, then the client id of the
//! persistent id. Lua literals use the temporary id's own client id.
//! `4384` is left untouched so puzzle scripts can test decay state.

use std::collections::HashMap;

use tfs_rust_common::error::{Result, TfsRustError};

use crate::otb::ItemType;

const ESCAPE: u8 = 0xFD;
const NODE_START: u8 = 0xFE;
const NODE_END: u8 = 0xFF;
const OTBM_TILE: u8 = 5;
const OTBM_ITEM: u8 = 6;
const OTBM_HOUSETILE: u8 = 14;
const OTBM_ATTR_TILE_FLAGS: u8 = 3;
const OTBM_ATTR_ITEM: u8 = 9;

/// Temporary field id → persistent id, applied before the client-id lookup.
fn persistent_field_id(id: u16) -> u16 {
    match id {
        1487 => 1492,
        1488 => 1493,
        1489 => 1494,
        1490 => 1496,
        1491 => 1495,
        1497 => 1498,
        1499 => 2721,
        other => other,
    }
}

/// Live OTB server id → client id. `client_id == 0` stays the server id (fluids).
#[derive(Debug, Clone)]
pub struct ClientIdMap {
    to_client: HashMap<u16, u16>,
}

impl ClientIdMap {
    pub fn from_items(items: &HashMap<u16, ItemType>) -> Self {
        let mut to_client = HashMap::with_capacity(items.len());
        for (sid, it) in items {
            let cid = if it.client_id == 0 { *sid } else { it.client_id };
            to_client.insert(*sid, cid);
        }
        Self { to_client }
    }

    pub fn client_of(&self, server_id: u16) -> Option<u16> {
        self.to_client.get(&server_id).copied()
    }

    /// Map file id: persistent pair first, then that row's client id.
    /// Pairs match `Item::CreateItem(PropStream)` (`item.cpp`).
    pub fn otbm_id(&self, id: u16) -> u16 {
        let persistent = persistent_field_id(id);
        self.to_client.get(&persistent).copied().unwrap_or(persistent)
    }
}

#[derive(Debug, Default, Clone)]
pub struct RewriteStats {
    pub replacements: usize,
    pub puzzle_lever_left: usize,
    pub unmapped: Vec<u16>,
}

/// Rewrite item-id literals. Does not touch coordinates inside `{...}` or `4384`.
pub fn rewrite_item_literals(src: &str, map: &ClientIdMap) -> (String, RewriteStats) {
    let mut stats = RewriteStats::default();
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        if let Some(len) = marker_len(bytes, i) {
            out.push_str(&src[i..i + len]);
            i += len;
            i = rewrite_following_number(&src, bytes, i, &mut out, map, &mut stats);
            continue;
        }
        if call_at(bytes, i, b"isItemInPosition") || call_at(bytes, i, b"transformItemInPosition") {
            let name_end = if call_at(bytes, i, b"isItemInPosition") {
                i + "isItemInPosition".len()
            } else {
                i + "transformItemInPosition".len()
            };
            let paren = skip_ws_bytes(bytes, name_end);
            if paren < bytes.len() && bytes[paren] == b'(' {
                let call_end = skip_parens(bytes, paren);
                rewrite_call_outside_braces(&src[i..call_end], map, &mut out, &mut stats);
                i = call_end;
                continue;
            }
        }
        out.push(src[i..].chars().next().unwrap_or('\0'));
        i += src[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
    }
    stats.unmapped.sort_unstable();
    stats.unmapped.dedup();
    (out, stats)
}

/// Third integer of `SELECT a, b, itemtype, count` in seed SQL.
pub fn rewrite_sql_itemtype_selects(src: &str, map: &ClientIdMap) -> (String, RewriteStats) {
    let mut stats = RewriteStats::default();
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;
    let needle = b"SELECT";
    while i < bytes.len() {
        if bytes[i..].starts_with(needle) && boundary_before(bytes, i) {
            let after = i + needle.len();
            out.push_str(&src[i..after]);
            i = rewrite_select_third(src, bytes, after, &mut out, map, &mut stats);
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    (out, stats)
}

/// Second-pass markers the first pass does not own (`id` / `corpse` stay put).
pub fn rewrite_named_ids(src: &str, map: &ClientIdMap, markers: &[&[u8]]) -> (String, RewriteStats) {
    let mut stats = RewriteStats::default();
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        if let Some(len) = named_marker(bytes, i, markers) {
            out.push_str(&src[i..i + len]);
            i += len;
            i = rewrite_following_number(&src, bytes, i, &mut out, map, &mut stats);
            continue;
        }
        out.push(src[i..].chars().next().unwrap_or('\0'));
        i += src[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
    }
    (out, stats)
}

/// `{ set = { var = "type", value = N } }` — the number after `type` is the item id.
pub fn rewrite_shop_type_values(src: &str, map: &ClientIdMap) -> (String, RewriteStats) {
    let mut stats = RewriteStats::default();
    let bytes = src.as_bytes();
    let needle = b"var = \"type\"";
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(needle) {
            out.push_str(&src[i..i + needle.len()]);
            i += needle.len();
            i = rewrite_next_int_on_line(&src, bytes, i, &mut out, map, &mut stats);
            continue;
        }
        out.push(src[i..].chars().next().unwrap_or('\0'));
        i += src[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
    }
    (out, stats)
}

/// Rewrite every integer token that has a different client id.
/// `skip_line` keeps action ids, timers, and effect constants.
pub fn rewrite_mapped_ints_skip_lines(
    src: &str,
    map: &ClientIdMap,
    skip_line: impl Fn(&str) -> bool,
) -> (String, RewriteStats) {
    let mut stats = RewriteStats::default();
    let mut out = String::with_capacity(src.len());
    for line in src.split_inclusive('\n') {
        if skip_line(line) {
            out.push_str(line);
            continue;
        }
        let (rewritten, line_stats) = rewrite_all_ints(line, map);
        stats.replacements += line_stats.replacements;
        stats.puzzle_lever_left += line_stats.puzzle_lever_left;
        stats.unmapped.extend(line_stats.unmapped);
        out.push_str(&rewritten);
    }
    (out, stats)
}

fn rewrite_next_int_on_line(
    src: &str,
    bytes: &[u8],
    mut i: usize,
    out: &mut String,
    map: &ClientIdMap,
    stats: &mut RewriteStats,
) -> usize {
    let start = i;
    while i < bytes.len() && bytes[i] != b'\n' && !bytes[i].is_ascii_digit() {
        i += 1;
    }
    out.push_str(&src[start..i]);
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return i;
    }
    let num_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    replace_number(&src[num_start..i], out, map, stats);
    i
}

fn rewrite_all_ints(src: &str, map: &ClientIdMap) -> (String, RewriteStats) {
    let mut stats = RewriteStats::default();
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric()) {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            replace_number(&src[start..i], &mut out, map, &mut stats);
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    (out, stats)
}

fn named_marker(bytes: &[u8], i: usize, markers: &[&[u8]]) -> Option<usize> {
    for marker in markers {
        if bytes[i..].starts_with(marker)
            && boundary_before(bytes, i)
            && marker_tail(bytes, i + marker.len())
        {
            return Some(marker.len());
        }
    }
    None
}

pub fn rewrite_otbm_bytes(data: &[u8], map: &ClientIdMap) -> Result<Vec<u8>> {
    if data.len() < 6 {
        return Err(TfsRustError::Content {
            file: "otbm".into(),
            message: "OTBM too small".into(),
        });
    }
    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(&data[..4]);
    let mut i = 4;
    copy_node(data, &mut i, &mut out, map)?;
    if i != data.len() {
        out.extend_from_slice(&data[i..]);
    }
    Ok(out)
}

fn rewrite_following_number(
    src: &str,
    bytes: &[u8],
    mut i: usize,
    out: &mut String,
    map: &ClientIdMap,
    stats: &mut RewriteStats,
) -> usize {
    let start_ws = i;
    while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'=' || bytes[i] == b'(')
    {
        i += 1;
    }
    out.push_str(&src[start_ws..i]);
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return i;
    }
    let num_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    replace_number(&src[num_start..i], out, map, stats);
    i
}

fn rewrite_call_outside_braces(
    call: &str,
    map: &ClientIdMap,
    out: &mut String,
    stats: &mut RewriteStats,
) {
    let bytes = call.as_bytes();
    let mut i = 0;
    let mut depth = 0i32;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                depth += 1;
                out.push('{');
                i += 1;
            }
            b'}' => {
                depth -= 1;
                out.push('}');
                i += 1;
            }
            b'0'..=b'9' if depth == 0 => {
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                replace_number(&call[start..i], out, map, stats);
            }
            _ => {
                out.push(bytes[i] as char);
                i += 1;
            }
        }
    }
}

fn rewrite_select_third(
    src: &str,
    bytes: &[u8],
    mut i: usize,
    out: &mut String,
    map: &ClientIdMap,
    stats: &mut RewriteStats,
) -> usize {
    // SELECT <int>, <int>, <itemtype>, <int>
    let mut nums = 0;
    while i < bytes.len() && nums < 3 {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            nums += 1;
            if nums == 3 {
                replace_number(&src[start..i], out, map, stats);
            } else {
                out.push_str(&src[start..i]);
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
            if bytes[i - 1] == b'\n' {
                break;
            }
        }
    }
    i
}

fn replace_number(text: &str, out: &mut String, map: &ClientIdMap, stats: &mut RewriteStats) {
    let Ok(id) = text.parse::<u16>() else {
        out.push_str(text);
        return;
    };
    if id == 4384 {
        stats.puzzle_lever_left += 1;
        out.push_str(text);
        return;
    }
    match map.client_of(id) {
        Some(cid) if cid != id => {
            stats.replacements += 1;
            out.push_str(&cid.to_string());
        }
        Some(_) => out.push_str(text),
        None => {
            stats.unmapped.push(id);
            out.push_str(text);
        }
    }
}

fn marker_len(bytes: &[u8], i: usize) -> Option<usize> {
    const MARKERS: &[&[u8]] = &[
        b"corpse",
        b"createItem",
        b"weapon:id",
        b"item:transform",
        b"item:getId()",
        b"id",
    ];
    for marker in MARKERS {
        if bytes[i..].starts_with(marker) && boundary_before(bytes, i) && marker_tail(bytes, i + marker.len())
        {
            return Some(marker.len());
        }
    }
    None
}

fn marker_tail(bytes: &[u8], i: usize) -> bool {
    let i = skip_ws_bytes(bytes, i);
    if i >= bytes.len() {
        return false;
    }
    // `getId() ==`, `weapon:id(`, `createItem(`, `transform(`, `id =`, `corpse =`
    bytes[i] == b'=' || bytes[i] == b'('
}

fn call_at(bytes: &[u8], i: usize, name: &[u8]) -> bool {
    bytes[i..].starts_with(name) && boundary_before(bytes, i)
}

fn boundary_before(bytes: &[u8], i: usize) -> bool {
    i == 0
        || !bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_'
}

fn skip_ws_bytes(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn skip_parens(bytes: &[u8], start: usize) -> usize {
    let mut depth = 0i32;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return i;
                }
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    bytes.len()
}

fn copy_node(data: &[u8], i: &mut usize, out: &mut Vec<u8>, map: &ClientIdMap) -> Result<()> {
    if *i >= data.len() || data[*i] != NODE_START {
        return Err(TfsRustError::Content {
            file: "otbm".into(),
            message: "expected OTBM node".into(),
        });
    }
    out.push(NODE_START);
    *i += 1;
    if *i >= data.len() {
        return Err(TfsRustError::Content {
            file: "otbm".into(),
            message: "OTBM node missing type".into(),
        });
    }
    let ty = data[*i];
    out.push(ty);
    *i += 1;
    if ty == OTBM_ITEM {
        let id = read_data_u16(data, i)?;
        write_data_u16(out, map.otbm_id(id));
    } else if ty == OTBM_TILE || ty == OTBM_HOUSETILE {
        let props = read_raw_props(data, i)?;
        let logical = unescape(&props);
        let rewritten = rewrite_tile_props(&logical, ty == OTBM_HOUSETILE, map);
        write_escaped(out, &rewritten);
    }
    while *i < data.len() {
        match data[*i] {
            NODE_START => copy_node(data, i, out, map)?,
            NODE_END => {
                out.push(NODE_END);
                *i += 1;
                return Ok(());
            }
            ESCAPE => {
                out.push(ESCAPE);
                *i += 1;
                if *i < data.len() {
                    out.push(data[*i]);
                    *i += 1;
                }
            }
            b => {
                out.push(b);
                *i += 1;
            }
        }
    }
    Err(TfsRustError::Content {
        file: "otbm".into(),
        message: "unterminated OTBM node".into(),
    })
}

fn rewrite_tile_props(props: &[u8], house: bool, map: &ClientIdMap) -> Vec<u8> {
    let mut out = Vec::with_capacity(props.len());
    let header = if house { 6 } else { 2 };
    if props.len() < header {
        out.extend_from_slice(props);
        return out;
    }
    out.extend_from_slice(&props[..header]);
    let mut cursor = header;
    while cursor < props.len() {
        let attr = props[cursor];
        cursor += 1;
        out.push(attr);
        match attr {
            OTBM_ATTR_TILE_FLAGS => {
                let end = (cursor + 4).min(props.len());
                out.extend_from_slice(&props[cursor..end]);
                cursor = end;
            }
            OTBM_ATTR_ITEM => {
                if cursor + 2 > props.len() {
                    break;
                }
                let id = u16::from_le_bytes([props[cursor], props[cursor + 1]]);
                cursor += 2;
                out.extend_from_slice(&map.otbm_id(id).to_le_bytes());
            }
            _ => {
                out.extend_from_slice(&props[cursor..]);
                break;
            }
        }
    }
    out
}

fn read_raw_props(data: &[u8], i: &mut usize) -> Result<Vec<u8>> {
    let start = *i;
    while *i < data.len() {
        match data[*i] {
            NODE_START | NODE_END => break,
            ESCAPE => *i += 2,
            _ => *i += 1,
        }
    }
    Ok(data[start..*i].to_vec())
}

fn unescape(buf: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(buf.len());
    let mut i = 0;
    while i < buf.len() {
        if buf[i] == ESCAPE {
            i += 1;
            if i < buf.len() {
                out.push(buf[i]);
            }
            i += 1;
        } else {
            out.push(buf[i]);
            i += 1;
        }
    }
    out
}

fn write_escaped(out: &mut Vec<u8>, logical: &[u8]) {
    for &b in logical {
        if b == ESCAPE || b == NODE_START || b == NODE_END {
            out.push(ESCAPE);
        }
        out.push(b);
    }
}

fn read_data_u16(data: &[u8], i: &mut usize) -> Result<u16> {
    let lo = read_data_u8(data, i)?;
    let hi = read_data_u8(data, i)?;
    Ok(u16::from_le_bytes([lo, hi]))
}

fn read_data_u8(data: &[u8], i: &mut usize) -> Result<u8> {
    if *i >= data.len() {
        return Err(TfsRustError::Content {
            file: "otbm".into(),
            message: "truncated OTBM item id".into(),
        });
    }
    let value = if data[*i] == ESCAPE {
        *i += 1;
        if *i >= data.len() {
            return Err(TfsRustError::Content {
                file: "otbm".into(),
                message: "dangling OTBM escape".into(),
            });
        }
        data[*i]
    } else {
        data[*i]
    };
    *i += 1;
    Ok(value)
}

fn write_data_u16(out: &mut Vec<u8>, value: u16) {
    let bytes = value.to_le_bytes();
    write_escaped(out, &bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_with(pairs: &[(u16, u16)]) -> ClientIdMap {
        let mut to_client = HashMap::new();
        for (s, c) in pairs {
            to_client.insert(*s, *c);
        }
        ClientIdMap { to_client }
    }

    #[test]
    #[test]
    fn pass2_rewrites_shop_type_and_range_ids() {
        let map = map_with(&[(1987, 2853), (1945, 2772), (1946, 2773)]);
        let shop = r#"{ set = { var = "type", value = 1987 } }"#;
        let (out, stats) = rewrite_shop_type_values(shop, &map);
        assert_eq!(out, r#"{ set = { var = "type", value = 2853 } }"#);
        assert_eq!(stats.replacements, 1);
        let range = "{fromId = 1945, toId = 1946, decayTo = 1945}";
        let (out, _) = rewrite_named_ids(range, &map, &[b"fromId", b"toId", b"decayTo"]);
        assert_eq!(out, "{fromId = 2772, toId = 2773, decayTo = 2772}");
        let npc = "{ expr = { count = 1945 } }, { delete = { item = 1946 } }";
        let (out, _) = rewrite_named_ids(npc, &map, &[b"count", b"item"]);
        assert_eq!(
            out,
            "{ expr = { count = 2772 } }, { delete = { item = 2773 } }"
        );
    }

    fn loot_and_position_calls_remap_without_touching_coordinates() {
        let map = map_with(&[(2148, 3031), (2229, 3111), (4384, 2773)]);
        let src = r#"
            corpse = 2148,
            loot = { { id = 2148, chance = 1 } },
            Game.isItemInPosition({x = 2148, y = 1, z = 7},2229)
            Game.isItemInPosition({x = 1, y = 2, z = 3},4384)
            houseid = 2148
        "#;
        let (out, stats) = rewrite_item_literals(src, &map);
        assert!(out.contains("corpse = 3031"), "{out}");
        assert!(out.contains("id = 3031"), "{out}");
        assert!(out.contains("x = 2148"), "{out}");
        assert!(out.contains(",3111)") || out.contains(", 3111)"), "{out}");
        assert!(out.contains("4384"), "{out}");
        assert!(out.contains("houseid = 2148"), "{out}");
        assert_eq!(stats.puzzle_lever_left, 1);
        assert!(stats.replacements >= 2);
    }

    #[test]
    fn sql_select_rewrites_the_itemtype_column_only() {
        let map = map_with(&[(2471, 3365)]);
        let src = "SELECT 1 AS pid, 101 AS sid, 2471 AS itemtype, 1 AS `count`";
        let (out, stats) = rewrite_sql_itemtype_selects(src, &map);
        assert!(out.contains("3365"), "{out}");
        assert!(out.contains("101"), "{out}");
        assert_eq!(stats.replacements, 1);
    }

    #[test]
    fn otbm_bytes_remap_embedded_and_item_nodes() {
        let map = map_with(&[(1487, 2118), (1492, 2119)]);
        let raw = mini_otbm(1487);
        let out = rewrite_otbm_bytes(&raw, &map).expect("rewrite");
        let parsed = crate::otbm::OtbmLoader::from_bytes(std::path::Path::new("mini.otbm"), out)
            .expect("parse");
        let mut ids = Vec::new();
        parsed
            .visit_tiles(|_, _, _, things| {
                for thing in things {
                    match thing {
                        crate::otbm::TileThing::EmbeddedItemId(id) => ids.push(*id),
                        crate::otbm::TileThing::ItemNodeProps(props) => {
                            ids.push(u16::from_le_bytes([props[0], props[1]]));
                        }
                    }
                }
                Ok(())
            })
            .expect("visit");
        assert_eq!(ids, vec![2119, 2119]);
    }

    fn mini_otbm(item_id: u16) -> Vec<u8> {
        fn push_escaped(out: &mut Vec<u8>, b: u8) {
            if b == ESCAPE || b == NODE_START || b == NODE_END {
                out.push(ESCAPE);
            }
            out.push(b);
        }
        fn emit(out: &mut Vec<u8>, ty: u8, props: &[u8], children: impl FnOnce(&mut Vec<u8>)) {
            out.push(NODE_START);
            out.push(ty);
            for &b in props {
                push_escaped(out, b);
            }
            children(out);
            out.push(NODE_END);
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
                emit(map, 4, &area, |area| {
                    let mut tile = vec![0, 0, OTBM_ATTR_ITEM];
                    tile.extend_from_slice(&item_id.to_le_bytes());
                    emit(area, OTBM_TILE, &tile, |_| {});
                    let mut house = vec![1, 0];
                    house.extend_from_slice(&7u32.to_le_bytes());
                    emit(area, OTBM_HOUSETILE, &house, |tile| {
                        emit(tile, OTBM_ITEM, &item_id.to_le_bytes(), |_| {});
                    });
                });
            });
        });
        let mut file = b"OTBM".to_vec();
        file.extend(body);
        file
    }

    #[test]
    fn otbm_pair_then_client_id() {
        let map = map_with(&[(1487, 2118), (1492, 2119)]);
        assert_eq!(map.otbm_id(1487), 2119);
        assert_eq!(map.client_of(1487), Some(2118));
    }
}
