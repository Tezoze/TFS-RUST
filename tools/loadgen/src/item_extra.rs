//! Client-id → extra-byte bitset for 772 `addItem` (stackable / splash / fluid).
//!
//! C++ reference: `itemloader.h` `itemflags_t` / `itemgroup_t`; `networkmessage.cpp` `addItem`.
//! Tiny OTB walk — not `tfs-rust-content` (loadgen must not pull the content crate).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const FLAG_STACKABLE: u32 = 1 << 7;
const GROUP_SPLASH: u8 = 11;
const GROUP_FLUID: u8 = 12;
const ESCAPE: u8 = 0xFD;
const NODE_START: u8 = 0xFE;
const NODE_END: u8 = 0xFF;
const ITEM_ATTR_SERVERID: u8 = 0x10; // `itemloader.h` `ITEM_ATTR_SERVERID`
const ITEM_ATTR_CLIENTID: u8 = 0x11;
const WORD_COUNT: usize = 1024; // 65536 bits

/// Client ids that write a count/liquid byte after the `u16` id on 772 wire.
#[derive(Clone, Debug)]
pub struct ItemExtraBits {
    bits: Box<[u64]>,
    /// Server item id → 772 client look id (`items.otb`).
    server_to_client: HashMap<u16, u16>,
}

impl Default for ItemExtraBits {
    fn default() -> Self {
        Self {
            bits: vec![0u64; WORD_COUNT].into_boxed_slice(),
            server_to_client: HashMap::new(),
        }
    }
}

impl ItemExtraBits {
    pub fn has(&self, client_id: u16) -> bool {
        let bit = u32::from(client_id);
        let word = (bit / 64) as usize;
        let mask = 1u64 << (bit % 64);
        self.bits.get(word).is_some_and(|w| w & mask != 0)
    }

    pub fn client_id(&self, server_id: u16) -> Option<u16> {
        self.server_to_client.get(&server_id).copied()
    }

    fn set(&mut self, client_id: u16) {
        let bit = u32::from(client_id);
        let word = (bit / 64) as usize;
        let mask = 1u64 << (bit % 64);
        if let Some(w) = self.bits.get_mut(word) {
            *w |= mask;
        }
    }

    /// Load `TFS_ITEMS_OTB` or the repo `data/items/items.otb`. Missing file → empty set.
    pub fn load_default() -> Self {
        let path = resolve_otb_path();
        match path {
            Some(p) => Self::from_otb_file(&p).unwrap_or_default(),
            None => Self::default(),
        }
    }

    pub fn from_otb_file(path: &Path) -> Option<Self> {
        let data = std::fs::read(path).ok()?;
        let mut out = Self::default();
        let mut index = 0usize;
        while index < data.len() {
            if data[index] == NODE_START {
                parse_node(&data, &mut index, &mut out)?;
            } else {
                index += 1;
            }
        }
        Some(out)
    }
}

pub fn shared_item_extra() -> Arc<ItemExtraBits> {
    static CELL: std::sync::OnceLock<Arc<ItemExtraBits>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| Arc::new(ItemExtraBits::load_default()))
        .clone()
}

fn resolve_otb_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("TFS_ITEMS_OTB") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    let candidates = [
        PathBuf::from("data/items/items.otb"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/items/items.otb"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

fn parse_node(data: &[u8], index: &mut usize, out: &mut ItemExtraBits) -> Option<()> {
    if *index >= data.len() || data[*index] != NODE_START {
        return None;
    }
    *index += 1;
    let group = read_data_u8(data, index)?;
    let flags = read_data_u32(data, index)?;
    let mut server_id = 0u16;
    let mut client_id = 0u16;

    while *index < data.len() {
        match data[*index] {
            NODE_START => {
                parse_node(data, index, out)?;
            }
            NODE_END => {
                *index += 1;
                break;
            }
            _ => {
                let attr_type = read_data_u8(data, index)?;
                let attr_size = usize::from(read_data_u16(data, index)?);
                let attr_data = read_data_bytes(data, index, attr_size)?;
                if attr_type == ITEM_ATTR_SERVERID && attr_data.len() == 2 {
                    server_id = u16::from_le_bytes([attr_data[0], attr_data[1]]);
                }
                if attr_type == ITEM_ATTR_CLIENTID && attr_data.len() == 2 {
                    client_id = u16::from_le_bytes([attr_data[0], attr_data[1]]);
                }
            }
        }
    }

    let extra = (flags & FLAG_STACKABLE) != 0 || group == GROUP_SPLASH || group == GROUP_FLUID;
    if extra && client_id != 0 {
        out.set(client_id);
    }
    if server_id != 0 && client_id != 0 {
        out.server_to_client.insert(server_id, client_id);
    }
    Some(())
}

fn read_data_u8(data: &[u8], index: &mut usize) -> Option<u8> {
    if *index >= data.len() {
        return None;
    }
    let value = if data[*index] == ESCAPE {
        *index += 1;
        *data.get(*index)?
    } else {
        data[*index]
    };
    *index += 1;
    Some(value)
}

fn read_data_u16(data: &[u8], index: &mut usize) -> Option<u16> {
    let lo = read_data_u8(data, index)?;
    let hi = read_data_u8(data, index)?;
    Some(u16::from_le_bytes([lo, hi]))
}

fn read_data_u32(data: &[u8], index: &mut usize) -> Option<u32> {
    let b0 = read_data_u8(data, index)?;
    let b1 = read_data_u8(data, index)?;
    let b2 = read_data_u8(data, index)?;
    let b3 = read_data_u8(data, index)?;
    Some(u32::from_le_bytes([b0, b1, b2, b3]))
}

fn read_data_bytes(data: &[u8], index: &mut usize, len: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        out.push(read_data_u8(data, index)?);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn default_has_no_extras() {
        let bits = ItemExtraBits::default();
        assert!(!bits.has(100));
        assert!(!bits.has(3031));
    }

    #[test]
    fn set_and_has() {
        let mut bits = ItemExtraBits::default();
        bits.set(3031);
        assert!(bits.has(3031));
        assert!(!bits.has(3032));
    }

    #[test]
    fn repo_otb_marks_gold_extra_if_present() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/items/items.otb");
        let Some(bits) = ItemExtraBits::from_otb_file(&path) else {
            return;
        };
        // 772 gold coin client id is typically 3031; if this OTB uses another id the
        // skipper still works — this only checks the parser found some extra-byte ids.
        assert!(
            (0u16..=u16::MAX).any(|id| bits.has(id)),
            "parsed {} but no stackable/splash/fluid client ids",
            path.display()
        );
        assert!(
            bits.client_id(2268).is_some(),
            "OTB missing server id 2268 (sudden death rune)"
        );
    }
}
