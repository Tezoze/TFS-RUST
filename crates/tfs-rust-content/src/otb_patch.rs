//! Patch `items.otb` in place from `objects.srv` (Waypoints + Unpass→blockSolid).
// C++ reference: `src/fileloader.cpp` (OTB ESCAPE), `src/items.cpp` `ITEM_ATTR_SPEED`,
// `itemloader.h` `FLAG_BLOCK_SOLID` ↔ 772 `UNPASS`.

use crate::objects_srv::ObjectsSrvTypeFlags;
use crate::otb::{ItemType, OtbLoader};
use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};

const NODE_START: u8 = 0xFE;
const NODE_END: u8 = 0xFF;
const ESCAPE: u8 = 0xFD;
const ITEM_ATTR_SERVERID: u8 = 0x10;
const ITEM_ATTR_SPEED: u8 = 0x14;
/// `itemflags_t::FLAG_BLOCK_SOLID` — 772 `Unpass` (`itemloader.h`).
pub const FLAG_BLOCK_SOLID: u32 = 1 << 0;

/// 772 identity of an OTB row = `objects.srv[client_id]`.
///
/// The `.sec`→OTBM conversion remaps 772 `TypeID → server_id` via `client_id`, so a row's real
/// terrain flags/Waypoints come from its **`client_id`** (== the shared 772 `TypeID`), never its
/// renumbered `server_id`. Only fall back to `server_id` for rows with no client id (`client_id == 0`).
fn srv_identity<'a>(
    row: &ItemType,
    srv: &'a HashMap<u16, ObjectsSrvTypeFlags>,
) -> Option<&'a ObjectsSrvTypeFlags> {
    if row.client_id != 0 {
        if let Some(t) = srv.get(&row.client_id) {
            return Some(t);
        }
    }
    srv.get(&row.server_id)
}

/// Build `server_id -> Waypoints` for walkable BANK OTB rows (`Bank && !Unpass && Waypoints > 0`).
///
/// Iterates OTB rows and resolves each row's 772 identity by `client_id` (see [`srv_identity`]).
pub fn build_speed_patches(objects_srv: &Path, otb_path: &Path) -> Result<HashMap<u16, u16>> {
    let items = OtbLoader::load_from_file(otb_path)?;
    let srv = crate::objects_srv::parse_all_types(objects_srv)?;
    let mut patches = HashMap::new();
    for (&server_id, row) in &items {
        if let Some(t) = srv_identity(row, &srv) {
            if t.flags.bank && !t.flags.unpass && t.waypoints > 0 {
                patches.insert(server_id, t.waypoints as u16);
            }
        }
    }
    Ok(patches)
}

/// Build `server_id -> speed` for **all** OTB rows so `ITEM_ATTR_SPEED` matches `objects.srv`.
///
/// - BANK ground: walkable `Waypoints`, or `0` for `Unpass` / invalid (`<= 0`) tiles (mountain/water).
/// - Non-BANK (or unknown 772 type): `0` — 772 `ITEM_ATTR_SPEED` is BANK-only, so stale TVP/TFS
///   ground speeds are cleared.
///
/// Each row's identity is resolved by `client_id`, fixing rows where TVP renumbered `server_id`
/// (e.g. rock soil client 4402 → server 4413, whose old `server_id` join wrongly used the
/// "a mountain" `objects.srv[4413]`). Only emits entries that actually change (`row.speed != new`).
// C++ reference: `cract.cc` `TShortway::FillMap` (BANK && !UNPASS && Waypoints > 0).
pub fn build_all_speed_patches(objects_srv: &Path, otb_path: &Path) -> Result<HashMap<u16, u16>> {
    let items = OtbLoader::load_from_file(otb_path)?;
    let srv = crate::objects_srv::parse_all_types(objects_srv)?;
    let mut patches = HashMap::new();
    for (&server_id, row) in &items {
        let new_speed = match srv_identity(row, &srv) {
            Some(t) if t.flags.bank => {
                if t.flags.unpass || t.waypoints <= 0 {
                    0
                } else {
                    t.waypoints as u16
                }
            }
            _ => 0,
        };
        if row.speed != new_speed {
            patches.insert(server_id, new_speed);
        }
    }
    Ok(patches)
}

/// Build `server_id → FLAG_BLOCK_SOLID` OR-masks for `Unpass` OTB rows missing the bit.
///
/// Includes **Bank+Unpass** dirt/earth/stone walls and water — those must block players.
/// OTBM mountain "rock soil" (`Name = "a mountain"`) is solidified here then cleared by
/// [`build_bank_unpass_clear_solid`] so players can still walk those cliff banks.
/// Identity resolved by `client_id`.
// C++ reference: `UNPASS` → `blockSolid` — `docs/772_OTB_OBJECTS_SRV_FLAG_MAPPING.md`.
pub fn build_unpass_block_solid_ors(objects_srv: &Path, otb_path: &Path) -> Result<HashMap<u16, u32>> {
    let items = OtbLoader::load_from_file(otb_path)?;
    let srv = crate::objects_srv::parse_all_types(objects_srv)?;
    let mut ors = HashMap::new();
    for (&server_id, row) in &items {
        let Some(t) = srv_identity(row, &srv) else {
            continue;
        };
        if t.flags.unpass && row.flags & FLAG_BLOCK_SOLID == 0 {
            ors.insert(server_id, FLAG_BLOCK_SOLID);
        }
    }
    Ok(ors)
}

/// Clear `FLAG_BLOCK_SOLID` only on OTBM walkable mountain banks (`objects.srv` name
/// `"a mountain"`, Bank+Unpass, Waypoints ≤ 0).
///
/// Lesson 171 cleared **all** Bank+Unpass+wp0 (dirt walls, earth, water) — players then
/// pathfind through dirt walls to ladders. Dirt/earth/stone walls keep `blockSolid` from
/// [`build_unpass_block_solid_ors`]. Monsters still treat mountains as Unpass via
/// `is_unpassable_for_field` (Bank+wp0). Identity by `client_id`.
pub fn build_bank_unpass_clear_solid(objects_srv: &Path, otb_path: &Path) -> Result<HashMap<u16, u32>> {
    let items = OtbLoader::load_from_file(otb_path)?;
    let srv = crate::objects_srv::parse_all_types(objects_srv)?;
    let mut clears = HashMap::new();
    for (&server_id, row) in &items {
        let Some(t) = srv_identity(row, &srv) else {
            continue;
        };
        if !is_otbm_walkable_mountain_bank(t) {
            continue;
        }
        // Clear after OR-solidify (patch applies OR then clear), or if already solid.
        clears.insert(server_id, FLAG_BLOCK_SOLID);
    }
    Ok(clears)
}

/// OTBM places srv `"a mountain"` as walkable cliff/rock-soil ground (lesson 171).
fn is_otbm_walkable_mountain_bank(t: &ObjectsSrvTypeFlags) -> bool {
    t.flags.bank
        && t.flags.unpass
        && t.waypoints <= 0
        && t.name.eq_ignore_ascii_case("a mountain")
}

/// Passable OTB ground (`group==1`, speed 0, identity not `Unpass`, not `blockSolid`) → default 150.
///
/// OTBM often uses Clip borders as sole ground; CipSoft maps keep Bank underneath. Identity by `client_id`.
pub fn build_passable_zero_speed_defaults(objects_srv: &Path, otb_path: &Path) -> Result<HashMap<u16, u16>> {
    let items = OtbLoader::load_from_file(otb_path)?;
    let srv = crate::objects_srv::parse_all_types(objects_srv)?;
    let mut patches = HashMap::new();
    for (&server_id, row) in &items {
        if row.group != ItemType::GROUP_GROUND || row.speed != 0 {
            continue;
        }
        if srv_identity(row, &srv).is_some_and(|t| t.flags.unpass) {
            continue;
        }
        if row.flags & FLAG_BLOCK_SOLID != 0 {
            continue;
        }
        patches.insert(server_id, 150);
    }
    Ok(patches)
}

/// Rewrite `path` with patched `ITEM_ATTR_SPEED` values. Returns count of nodes updated.
pub fn patch_file(path: &Path, patches: &HashMap<u16, u16>) -> Result<u32> {
    let (speed, _) =
        patch_file_speeds_and_flags(path, patches, &HashMap::new(), &HashMap::new())?;
    Ok(speed)
}

/// Rewrite `path` with speed and/or flag OR/clear masks.
/// Returns `(speed_nodes, flag_nodes)`.
pub fn patch_file_speeds_and_flags(
    path: &Path,
    speeds: &HashMap<u16, u16>,
    flag_ors: &HashMap<u16, u32>,
    flag_clears: &HashMap<u16, u32>,
) -> Result<(u32, u32)> {
    let input = std::fs::read(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    crate::otb::validate_items_otb_root_version_for_patch(&input, path)?;

    let mut output = input[..4].to_vec();
    let mut pos = 4usize;
    let mut speed_patched = 0u32;
    let mut flag_patched = 0u32;
    while pos < input.len() {
        if is_node_start(&input, pos) {
            let (s, f, _, _) = patch_or_copy_node(
                &input,
                &mut pos,
                &mut output,
                speeds,
                flag_ors,
                flag_clears,
                path,
            )?;
            speed_patched += s;
            flag_patched += f;
        } else {
            output.push(input[pos]);
            pos += 1;
        }
    }
    std::fs::write(path, &output).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    Ok((speed_patched, flag_patched))
}

fn is_node_start(input: &[u8], pos: usize) -> bool {
    input.get(pos) == Some(&NODE_START)
}

fn patch_or_copy_node(
    input: &[u8],
    pos: &mut usize,
    output: &mut Vec<u8>,
    speeds: &HashMap<u16, u16>,
    flag_ors: &HashMap<u16, u32>,
    flag_clears: &HashMap<u16, u32>,
    path: &Path,
) -> Result<(u32, u32, u32, u32)> {
    if read_u8(input, pos, path)? != NODE_START {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "expected NODE_START".to_string(),
        });
    }
    let node_type = read_u8(input, pos, path)?;
    let flags = read_u32(input, pos, path)?;
    let props_begin = *pos;
    let props_end = find_props_end(input, props_begin, path)?;
    *pos = props_end;

    let props = unescape_props(&input[props_begin..props_end]);
    let attrs = parse_attrs(&props, path)?;
    let server_id = attrs
        .iter()
        .find(|(t, _)| *t == ITEM_ATTR_SERVERID)
        .and_then(|(_, d)| (d.len() >= 2).then(|| u16::from_le_bytes([d[0], d[1]])));

    let needs_speed = server_id.is_some_and(|sid| {
        speeds.get(&sid).is_some_and(|new_speed| {
            attrs
                .iter()
                .find(|(t, _)| *t == ITEM_ATTR_SPEED)
                .and_then(|(_, d)| (d.len() >= 2).then(|| u16::from_le_bytes([d[0], d[1]])))
                .map(|old| old != *new_speed)
                .unwrap_or(true)
        })
    });
    let flag_or = server_id.and_then(|sid| flag_ors.get(&sid).copied()).unwrap_or(0);
    let flag_clear = server_id
        .and_then(|sid| flag_clears.get(&sid).copied())
        .unwrap_or(0);
    let new_flags = (flags | flag_or) & !flag_clear;
    let needs_flag = new_flags != flags;

    let sid_in_map = server_id.is_some_and(|sid| {
        speeds.contains_key(&sid) || flag_ors.contains_key(&sid) || flag_clears.contains_key(&sid)
    });
    let mut speed_patched = 0u32;
    let mut flag_patched = u32::from(needs_flag);
    let mut sid_seen = u32::from(sid_in_map);
    let mut need_seen = u32::from(needs_speed || needs_flag);

    output.push(NODE_START);
    output.push(node_type);
    output.extend_from_slice(&new_flags.to_le_bytes());
    if needs_speed {
        let mut new_attrs = attrs;
        if let Some(sid) = server_id {
            if let Some(&new_speed) = speeds.get(&sid) {
                let new_bytes = new_speed.to_le_bytes().to_vec();
                if let Some(i) = new_attrs.iter().position(|(t, _)| *t == ITEM_ATTR_SPEED) {
                    new_attrs[i].1 = new_bytes;
                } else {
                    new_attrs.push((ITEM_ATTR_SPEED, new_bytes));
                }
                speed_patched = 1;
            }
        }
        write_escaped_props(output, &new_attrs, path)?;
    } else {
        output.extend_from_slice(&input[props_begin..props_end]);
    }

    while *pos < input.len() && is_node_start(input, *pos) {
        let (s, f, seen, need) =
            patch_or_copy_node(input, pos, output, speeds, flag_ors, flag_clears, path)?;
        speed_patched += s;
        flag_patched += f;
        sid_seen += seen;
        need_seen += need;
    }

    if *pos >= input.len() || input[*pos] != NODE_END {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: format!("expected NODE_END at {pos} (server_id {server_id:?})"),
        });
    }
    *pos += 1;
    output.push(NODE_END);
    Ok((speed_patched, flag_patched, sid_seen, need_seen))
}

/// First FE/FF in props, respecting `0xFD` escape (`src/fileloader.cpp`).
fn find_props_end(input: &[u8], begin: usize, path: &Path) -> Result<usize> {
    let mut i = begin;
    let mut escaped = false;
    while i < input.len() {
        let b = input[i];
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }
        if b == ESCAPE {
            escaped = true;
            i += 1;
            continue;
        }
        if b == NODE_START || b == NODE_END {
            return Ok(i);
        }
        i += 1;
    }
    Err(TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: "props region unterminated".to_string(),
    })
}

fn unescape_props(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0usize;
    while i < raw.len() {
        if raw[i] == ESCAPE {
            i += 1;
            if i < raw.len() {
                out.push(raw[i]);
            }
            i += 1;
        } else {
            out.push(raw[i]);
            i += 1;
        }
    }
    out
}

fn escape_props(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() + 8);
    for &b in raw {
        if b == ESCAPE || b == NODE_START || b == NODE_END {
            out.push(ESCAPE);
        }
        out.push(b);
    }
    out
}

fn parse_attrs(bytes: &[u8], path: &Path) -> Result<Vec<(u8, Vec<u8>)>> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < bytes.len() {
        let attr_type = read_u8(bytes, &mut pos, path)?;
        let size = read_u16(bytes, &mut pos, path)? as usize;
        let data = read_bytes(bytes, &mut pos, size, path)?;
        out.push((attr_type, data));
    }
    Ok(out)
}

fn write_escaped_props(output: &mut Vec<u8>, attrs: &[(u8, Vec<u8>)], path: &Path) -> Result<()> {
    let mut raw = Vec::new();
    for (attr_type, data) in attrs {
        raw.push(*attr_type);
        let len = u16::try_from(data.len()).map_err(|_| TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "attribute data too large".to_string(),
        })?;
        raw.extend_from_slice(&len.to_le_bytes());
        raw.extend_from_slice(data);
    }
    output.extend_from_slice(&escape_props(&raw));
    Ok(())
}

fn read_u8(input: &[u8], pos: &mut usize, path: &Path) -> Result<u8> {
    if *pos >= input.len() {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "unexpected EOF".to_string(),
        });
    }
    let v = input[*pos];
    *pos += 1;
    Ok(v)
}

fn read_u16(input: &[u8], pos: &mut usize, path: &Path) -> Result<u16> {
    if *pos + 2 > input.len() {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "unexpected EOF reading u16".to_string(),
        });
    }
    let v = u16::from_le_bytes([input[*pos], input[*pos + 1]]);
    *pos += 2;
    Ok(v)
}

fn read_u32(input: &[u8], pos: &mut usize, path: &Path) -> Result<u32> {
    if *pos + 4 > input.len() {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "unexpected EOF reading u32".to_string(),
        });
    }
    let v = u32::from_le_bytes([
        input[*pos],
        input[*pos + 1],
        input[*pos + 2],
        input[*pos + 3],
    ]);
    *pos += 4;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn patch_map_overlaps_otb_server_ids() {
        let otb = repo().join("data/items/items.otb");
        let objects = crate::objects_srv::resolve_objects_srv_path()
            .unwrap_or_else(|| repo().join("reference/classic-772/runtime/dat/objects.srv"));
        if !otb.is_file() || !objects.is_file() {
            return;
        }
        let items = OtbLoader::load_from_file(&otb).expect("otb");
        let patches = build_speed_patches(&objects, &otb).expect("patches");
        let overlap = items.keys().filter(|sid| patches.contains_key(sid)).count();
        eprintln!(
            "patch keys {} otb items {} overlap {}",
            patches.len(),
            items.len(),
            overlap
        );
        assert!(overlap > 0, "expected patch server_ids to exist in OTB");
        // Stairs TypeID 434 (walkable BANK, Waypoints=100) resolves via client_id to its OTB row.
        let stairs = crate::objects_srv::resolve_server_id_for_patch(434, &items)
            .expect("stairs 434 resolvable");
        assert!(items.contains_key(&stairs), "stairs server row expected in OTB");
        assert_eq!(
            patches.get(&stairs).copied(),
            Some(100),
            "stairs server {stairs} expected Waypoints 100 in patches"
        );
    }
}

fn read_bytes(input: &[u8], pos: &mut usize, len: usize, path: &Path) -> Result<Vec<u8>> {
    if *pos + len > input.len() {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: format!("unexpected EOF reading {len} bytes"),
        });
    }
    let v = input[*pos..*pos + len].to_vec();
    *pos += len;
    Ok(v)
}
