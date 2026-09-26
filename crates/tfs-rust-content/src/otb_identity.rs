//! Drop duplicate client-id nodes from an identity `items.otb`.
//!
//! The identity file stores `server_id == client_id` and still has both nodes
//! for each shared client id. The kept node is the one whose group and flags
//! match the chosen live server row.

use std::collections::HashMap;

use tfs_rust_common::error::{Result, TfsRustError};

use crate::otb::ItemType;

const ESCAPE: u8 = 0xFD;
const NODE_START: u8 = 0xFE;
const NODE_END: u8 = 0xFF;
const ITEM_ATTR_SERVERID: u8 = 0x10;
const ITEM_ATTR_CLIENTID: u8 = 0x11;

struct ItemNode {
    start: usize,
    end: usize,
    group: u8,
    flags: u32,
    client_id: u16,
}

/// `survivors` is `(client_id, live server id to keep)`.
pub fn dedup_identity_otb(
    data: &[u8],
    live: &HashMap<u16, ItemType>,
    survivors: &[(u16, u16)],
) -> Result<Vec<u8>> {
    let path = std::path::Path::new("items.otb");
    if data.len() < 5 || (&data[..4] != b"OTBI" && data[..4] != [0, 0, 0, 0]) {
        return Err(TfsRustError::Content {
            file: "items.otb".into(),
            message: "identity OTB must start with OTBI or a wildcard identifier".into(),
        });
    }
    let nodes = item_nodes(data).map_err(|message| TfsRustError::Content {
        file: path.display().to_string(),
        message,
    })?;
    let mut drop_at = vec![false; nodes.len()];
    let mut by_client: HashMap<u16, Vec<usize>> = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        by_client.entry(node.client_id).or_default().push(i);
    }
    let want: HashMap<u16, u16> = survivors.iter().copied().collect();
    for (client_id, idxs) in &by_client {
        if idxs.len() < 2 {
            continue;
        }
        let Some(server_id) = want.get(client_id) else {
            return Err(TfsRustError::Content {
                file: "items.otb".into(),
                message: format!("duplicate client id {client_id} has no survivor"),
            });
        };
        let Some(live_item) = live.get(server_id) else {
            return Err(TfsRustError::Content {
                file: "items.otb".into(),
                message: format!("survivor server id {server_id} missing from live OTB"),
            });
        };
        let mut matched = idxs
            .iter()
            .copied()
            .filter(|&i| nodes[i].group == live_item.group && nodes[i].flags == live_item.flags)
            .collect::<Vec<_>>();
        if matched.is_empty() {
            return Err(TfsRustError::Content {
                file: "items.otb".into(),
                message: format!(
                    "no identity node for client {client_id} matches server {server_id} group {} flags {:#x}",
                    live_item.group, live_item.flags
                ),
            });
        }
        matched.sort_unstable();
        let keep = matched[0];
        for i in idxs {
            if *i != keep {
                drop_at[*i] = true;
            }
        }
    }
    let mut out = Vec::with_capacity(data.len());
    let mut cursor = 0;
    for (i, node) in nodes.iter().enumerate() {
        if drop_at[i] {
            out.extend_from_slice(&data[cursor..node.start]);
            cursor = node.end;
        }
    }
    out.extend_from_slice(&data[cursor..]);
    Ok(out)
}

fn item_nodes(data: &[u8]) -> std::result::Result<Vec<ItemNode>, String> {
    if data.get(4) != Some(&NODE_START) {
        return Err("missing root node".into());
    }
    let mut nodes = Vec::new();
    let mut i = 5; // inside root, past NODE_START
    while i < data.len() {
        match data[i] {
            NODE_START => {
                let end = node_end(data, i).ok_or("unterminated item node")?;
                let (group, flags, client_id) = parse_item_identity(&data[i..end])?;
                nodes.push(ItemNode {
                    start: i,
                    end,
                    group,
                    flags,
                    client_id,
                });
                i = end;
            }
            NODE_END => break,
            ESCAPE => i += 2,
            _ => i += 1,
        }
    }
    Ok(nodes)
}

fn node_end(data: &[u8], start: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = start;
    while i < data.len() {
        match data[i] {
            NODE_START => {
                depth += 1;
                i += 1;
            }
            NODE_END => {
                depth = depth.saturating_sub(1);
                i += 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            ESCAPE => i += 2,
            _ => i += 1,
        }
    }
    None
}

fn parse_item_identity(node: &[u8]) -> std::result::Result<(u8, u32, u16), String> {
    if node.first() != Some(&NODE_START) {
        return Err("item node missing start".into());
    }
    let mut i = 1;
    let group = read_u8(node, &mut i)?;
    let flags = read_u32(node, &mut i)?;
    let mut client_id = 0u16;
    while i < node.len() {
        match node[i] {
            NODE_START | NODE_END => break,
            _ => {
                let attr = read_u8(node, &mut i)?;
                let size = read_u16(node, &mut i)? as usize;
                let payload = read_bytes(node, &mut i, size)?;
                if attr == ITEM_ATTR_CLIENTID && payload.len() == 2 {
                    client_id = u16::from_le_bytes([payload[0], payload[1]]);
                } else if attr == ITEM_ATTR_SERVERID && client_id == 0 && payload.len() == 2 {
                    client_id = u16::from_le_bytes([payload[0], payload[1]]);
                }
            }
        }
    }
    if client_id == 0 {
        return Err("item node missing client id".into());
    }
    Ok((group, flags, client_id))
}

fn read_u8(data: &[u8], i: &mut usize) -> std::result::Result<u8, String> {
    if *i >= data.len() {
        return Err("truncated otb".into());
    }
    let value = if data[*i] == ESCAPE {
        *i += 1;
        if *i >= data.len() {
            return Err("dangling otb escape".into());
        }
        data[*i]
    } else {
        data[*i]
    };
    *i += 1;
    Ok(value)
}

fn read_u16(data: &[u8], i: &mut usize) -> std::result::Result<u16, String> {
    let lo = read_u8(data, i)?;
    let hi = read_u8(data, i)?;
    Ok(u16::from_le_bytes([lo, hi]))
}

fn read_u32(data: &[u8], i: &mut usize) -> std::result::Result<u32, String> {
    let b0 = read_u8(data, i)?;
    let b1 = read_u8(data, i)?;
    let b2 = read_u8(data, i)?;
    let b3 = read_u8(data, i)?;
    Ok(u32::from_le_bytes([b0, b1, b2, b3]))
}

fn read_bytes(data: &[u8], i: &mut usize, len: usize) -> std::result::Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        out.push(read_u8(data, i)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_id_census::LOCKED_SURVIVORS;
    use crate::otb::OtbLoader;
    use std::path::Path;

    #[test]
    fn identity_otb_keeps_one_node_and_blocking_sandstone() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/items");
        let live = OtbLoader::load_from_file(&root.join("items.otb")).expect("live");
        let identity = std::fs::read(root.join("clientid_output/items.otb")).expect("identity");
        let out = dedup_identity_otb(&identity, &live, LOCKED_SURVIVORS).expect("dedup");
        let path = std::env::temp_dir().join("tfs-identity-dedup.otb");
        std::fs::write(&path, &out).expect("write");
        let db = OtbLoader::load_from_file(&path).expect("reload");
        let sand = db.get(&425).expect("425");
        assert!(sand.block_solid(), "425 keeps the blocking disguise");
        assert!(!sand.is_animation());
        assert_eq!(sand.speed, 70);
        let again = dedup_identity_otb(&out, &live, LOCKED_SURVIVORS).expect("idempotent");
        assert_eq!(again.len(), out.len());
    }
}
