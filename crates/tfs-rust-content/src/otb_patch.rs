//! Patch `items.otb` `ITEM_ATTR_SPEED` in place from `objects.srv` `Waypoints`.
// C++ reference: `src/fileloader.cpp` (OTB ESCAPE), `src/items.cpp` `ITEM_ATTR_SPEED`.

use crate::otb::OtbLoader;
use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};

const NODE_START: u8 = 0xFE;
const NODE_END: u8 = 0xFF;
const ESCAPE: u8 = 0xFD;
const ITEM_ATTR_SERVERID: u8 = 0x10;
const ITEM_ATTR_SPEED: u8 = 0x14;

/// Build `server_id -> Waypoints` from `objects.srv` resolved against loaded OTB ids.
pub fn build_speed_patches(
    objects_srv: &Path,
    otb_path: &Path,
) -> Result<HashMap<u16, u16>> {
    let items = OtbLoader::load_from_file(otb_path)?;
    let entries = crate::objects_srv::parse_walkable_waypoints(objects_srv)?;
    let mut patches = HashMap::new();
    for entry in entries {
        let Some(server_id) =
            crate::objects_srv::resolve_server_id_for_patch(entry.type_id, &items)
        else {
            continue;
        };
        patches.insert(server_id, entry.waypoints);
    }
    Ok(patches)
}

/// Rewrite `path` with patched `ITEM_ATTR_SPEED` values. Returns count of nodes updated.
pub fn patch_file(path: &Path, patches: &HashMap<u16, u16>) -> Result<u32> {
    let input = std::fs::read(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    crate::otb::validate_items_otb_root_version_for_patch(&input, path)?;

    let mut output = input[..4].to_vec();
    let mut pos = 4usize;
    let mut patched = 0u32;
    while pos < input.len() {
        if is_node_start(&input, pos) {
            let (n, _, _) =
                patch_or_copy_node(&input, &mut pos, &mut output, patches, path)?;
            patched += n;
        } else {
            output.push(input[pos]);
            pos += 1;
        }
    }
    std::fs::write(path, &output).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    Ok(patched)
}

fn is_node_start(input: &[u8], pos: usize) -> bool {
    input.get(pos) == Some(&NODE_START)
}

fn patch_or_copy_node(
    input: &[u8],
    pos: &mut usize,
    output: &mut Vec<u8>,
    patches: &HashMap<u16, u16>,
    path: &Path,
) -> Result<(u32, u32, u32)> {
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

    let needs_patch = server_id.is_some_and(|sid| {
        patches.get(&sid).is_some_and(|new_speed| {
            attrs
                .iter()
                .find(|(t, _)| *t == ITEM_ATTR_SPEED)
                .and_then(|(_, d)| (d.len() >= 2).then(|| u16::from_le_bytes([d[0], d[1]])))
                .map(|old| old != *new_speed)
                .unwrap_or(true)
        })
    });

    let sid_in_map = server_id.is_some_and(|sid| patches.contains_key(&sid));
    let mut patched = 0u32;
    let mut sid_seen = u32::from(sid_in_map);
    let mut need_seen = u32::from(needs_patch);

    output.push(NODE_START);
    output.push(node_type);
    output.extend_from_slice(&flags.to_le_bytes());
    if needs_patch {
        let mut new_attrs = attrs;
        if let Some(sid) = server_id {
            if let Some(&new_speed) = patches.get(&sid) {
                let new_bytes = new_speed.to_le_bytes().to_vec();
                if let Some(i) = new_attrs.iter().position(|(t, _)| *t == ITEM_ATTR_SPEED) {
                    new_attrs[i].1 = new_bytes;
                } else {
                    new_attrs.push((ITEM_ATTR_SPEED, new_bytes));
                }
                patched = 1;
            }
        }
        write_escaped_props(output, &new_attrs, path)?;
    } else {
        output.extend_from_slice(&input[props_begin..props_end]);
    }

    while *pos < input.len() && is_node_start(input, *pos) {
        let (n, s, d) = patch_or_copy_node(input, pos, output, patches, path)?;
        patched += n;
        sid_seen += s;
        need_seen += d;
    }

    if *pos >= input.len() || input[*pos] != NODE_END {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: format!("expected NODE_END at {pos} (server_id {server_id:?})"),
        });
    }
    *pos += 1;
    output.push(NODE_END);
    Ok((patched, sid_seen, need_seen))
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
        let overlap = items
            .keys()
            .filter(|sid| patches.contains_key(sid))
            .count();
        eprintln!("patch keys {} otb items {} overlap {}", patches.len(), items.len(), overlap);
        assert!(overlap > 0, "expected patch server_ids to exist in OTB");
        assert!(patches.contains_key(&434), "stairs 434 expected in patches");
        assert!(items.contains_key(&434), "stairs 434 expected in OTB");
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
