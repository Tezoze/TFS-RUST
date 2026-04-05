use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::warn;

#[derive(Debug, Clone, Default)]
pub struct ItemType {
    pub id: u16,
    pub server_id: u16,
    pub client_id: u16,
    /// OTB node type — `itemgroup_t` in `src/itemloader.h` / `src/items.cpp` (`itemNode.type`).
    pub group: u8,
    pub name: String,
    pub flags: u32,
    /// Ground speed from OTB (`ITEM_ATTR_SPEED`). C++ `ItemType::speed` (`src/items.cpp` `loadFromOtb` ~336–343, ~461).
    /// This is the tile walk speed, NOT equipment speed bonus (which is `abilities.speed` from items.xml).
    pub speed: u16,
    /// `ITEM_ATTR_LIGHT2` — `lightBlock2::lightLevel` (`src/itemloader.h`).
    pub light_level: u8,
    /// `ITEM_ATTR_LIGHT2` — `lightBlock2::lightColor`.
    pub light_color: u8,
    /// `ITEM_ATTR_WAREID` (`src/items.cpp` `loadFromOtb` ~373–381).
    pub ware_id: u16,
    pub weight: u32,
    pub rotate_to: u16,
    pub description: String,
    /// All `<attribute key="..." value="..."/>` pairs from `items.xml` (merged; keys lowercased).
    // C++ reference: src/items.cpp Items::parseItemNode
    pub xml_attributes: HashMap<String, String>,
    /// `ITEM_ATTR_TOPORDER` — used when `always_on_top` is true (`src/items.cpp` `loadFromOtb`).
    pub always_on_top_order: u8,
}

pub struct OtbLoader;

impl OtbLoader {
    pub fn load_from_file(path: &Path) -> Result<HashMap<u16, ItemType>> {
        let data = std::fs::read(path).map_err(|e| TfsRustError::Content {
            file: path.to_string_lossy().into(),
            message: e.to_string(),
        })?;

        validate_items_otb_root_version(&data, path)?;

        let mut db = HashMap::new();
        let mut index = 0usize;
        while index < data.len() {
            if data[index] == NODE_START {
                parse_node(&data, &mut index, &mut db, path)?;
            } else {
                index += 1;
            }
        }
        Ok(db)
    }
}

const ESCAPE: u8 = 0xFD;
const NODE_START: u8 = 0xFE;
const NODE_END: u8 = 0xFF;

const ITEM_ATTR_SERVERID: u8 = 0x10;
const ITEM_ATTR_CLIENTID: u8 = 0x11;
const ITEM_ATTR_NAME: u8 = 0x12;
const ITEM_ATTR_DESCR: u8 = 0x13;
/// `itemattrib_t::ITEM_ATTR_SPEED` (`src/itemloader.h`) — ground tile speed, `uint16_t`.
const ITEM_ATTR_SPEED: u8 = 0x14;
/// `itemattrib_t::ITEM_ATTR_WEIGHT` — **0x17**, not `0x16` (`MAXITEMS`) (`src/itemloader.h`).
const ITEM_ATTR_WEIGHT: u8 = 0x17;
/// `itemattrib_t::ITEM_ATTR_ROTATETO` — **0x1E** (`src/itemloader.h`).
const ITEM_ATTR_ROTATETO: u8 = 0x1E;
/// `itemattrib_t::ITEM_ATTR_LIGHT2` — `lightBlock2` {u16 lightLevel, u16 lightColor} (`src/itemloader.h`).
const ITEM_ATTR_LIGHT2: u8 = 0x2A;
/// `itemattrib_t::ITEM_ATTR_TOPORDER` (`src/itemloader.h`).
const ITEM_ATTR_TOPORDER: u8 = 0x2B;
/// `itemattrib_t::ITEM_ATTR_WAREID` (`src/itemloader.h`).
const ITEM_ATTR_WAREID: u8 = 0x2D;
/// `itemattrib_t::ITEM_ATTR_CLASSIFICATION` (`src/itemloader.h`) — skipped (1 byte), not stored.
const ITEM_ATTR_CLASSIFICATION: u8 = 0x2E;

/// `rootattrib_::ROOT_ATTR_VERSION` (`src/itemloader.h`).
const ROOT_ATTR_VERSION: u8 = 0x01;
/// `sizeof(VERSIONINFO)` in C++ (`src/itemloader.h`) — `uint32_t`×3 + `uint8_t[128]`.
const VERSIONINFO_SIZE: usize = 4 + 4 + 4 + 128;
/// `CLIENT_VERSION_1098` (`src/itemloader.h`).
const CLIENT_VERSION_1098: u32 = 57;

fn parse_node(
    data: &[u8],
    index: &mut usize,
    db: &mut HashMap<u16, ItemType>,
    path: &Path,
) -> Result<()> {
    expect_raw(data, index, NODE_START, path)?;

    let node_type = read_data_u8(data, index, path)?;
    let flags = read_data_u32(data, index, path)?;
    let mut item = ItemType {
        flags,
        group: node_type,
        ..ItemType::default()
    };

    while *index < data.len() {
        match data[*index] {
            NODE_START => {
                parse_node(data, index, db, path)?;
            }
            NODE_END => {
                *index += 1;
                break;
            }
            _ => {
                let attr_type = read_data_u8(data, index, path)?;
                let attr_size = read_data_u16(data, index, path)? as usize;
                let attr_data = read_data_bytes(data, index, attr_size, path)?;
                apply_attr(&mut item, attr_type, &attr_data);
            }
        }
    }

    if item.server_id != 0 {
        item.id = item.server_id;
        db.insert(item.server_id, item);
    }
    Ok(())
}

/// C++ `Items::loadFromOtb` root/version check (`src/items.cpp`) — OTBI header + `VERSIONINFO` in root props.
fn validate_items_otb_root_version(data: &[u8], path: &Path) -> Result<()> {
    const OTBI: &[u8] = b"OTBI";
    /// C++ `OTB::Loader` accepts four zero bytes as wildcard (`src/fileloader.cpp`).
    const WILDCARD_ID: [u8; 4] = [0, 0, 0, 0];

    // Identifier (4) + START (1) + root type (1) + flags (4) + attr (1) + datalen (2) + VERSIONINFO.
    const MIN_ROOT: usize = 4 + 1 + 1 + 4 + 1 + 2 + VERSIONINFO_SIZE;
    if data.len() < MIN_ROOT {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: format!("items.otb too small for OTBI root + VERSIONINFO (need >= {MIN_ROOT} bytes)"),
        });
    }
    if data[..4] != *OTBI && data[..4] != WILDCARD_ID {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "items.otb must start with OTBI (or wildcard \\0\\0\\0\\0)".to_string(),
        });
    }

    let mut idx = 4usize;
    if read_data_u8(data, &mut idx, path)? != NODE_START {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "items.otb: expected root NODE_START (0xFE) after identifier".to_string(),
        });
    }
    let _root_type = read_data_u8(data, &mut idx, path)?;
    let _flags = read_data_u32(data, &mut idx, path)?;
    let attr = read_data_u8(data, &mut idx, path)?;
    if attr != ROOT_ATTR_VERSION {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: format!(
                "items.otb root: expected ROOT_ATTR_VERSION (0x01), got {attr:#x} (see Items::loadFromOtb)"
            ),
        });
    }
    let datalen = read_data_u16(data, &mut idx, path)? as usize;
    if datalen != VERSIONINFO_SIZE {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: format!(
                "items.otb VERSIONINFO: expected datalen {VERSIONINFO_SIZE}, got {datalen}"
            ),
        });
    }
    let vi = read_data_bytes(data, &mut idx, datalen, path)?;
    let major = u32::from_le_bytes([vi[0], vi[1], vi[2], vi[3]]);
    let minor = u32::from_le_bytes([vi[4], vi[5], vi[6], vi[7]]);

    if major == 0xFFFF_FFFF {
        warn!(
            target: "tfs_rust_content::otb",
            path = %path.display(),
            "items.otb uses generic client version (C++ warns and continues)"
        );
        return Ok(());
    }
    if major != 3 {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: format!(
                "items.otb: majorVersion must be 3, got {major} (Items::loadFromOtb)"
            ),
        });
    }
    if minor < CLIENT_VERSION_1098 {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: format!(
                "items.otb: minorVersion must be >= {CLIENT_VERSION_1098} (10.98), got {minor}"
            ),
        });
    }
    Ok(())
}

fn apply_attr(item: &mut ItemType, attr_type: u8, attr_data: &[u8]) {
    match attr_type {
        ITEM_ATTR_SERVERID if attr_data.len() >= 2 => {
            item.server_id = u16::from_le_bytes([attr_data[0], attr_data[1]]);
        }
        ITEM_ATTR_CLIENTID if attr_data.len() >= 2 => {
            item.client_id = u16::from_le_bytes([attr_data[0], attr_data[1]]);
        }
        ITEM_ATTR_NAME => {
            item.name = String::from_utf8_lossy(attr_data).to_string();
        }
        ITEM_ATTR_DESCR => {
            item.description = String::from_utf8_lossy(attr_data).to_string();
        }
        ITEM_ATTR_SPEED if attr_data.len() >= 2 => {
            item.speed = u16::from_le_bytes([attr_data[0], attr_data[1]]);
        }
        ITEM_ATTR_LIGHT2 if attr_data.len() >= 4 => {
            // C++ `lightBlock2` is packed {u16 lightLevel, u16 lightColor} (`src/itemloader.h`).
            item.light_level = u16::from_le_bytes([attr_data[0], attr_data[1]]) as u8;
            item.light_color = u16::from_le_bytes([attr_data[2], attr_data[3]]) as u8;
        }
        ITEM_ATTR_WAREID if attr_data.len() >= 2 => {
            item.ware_id = u16::from_le_bytes([attr_data[0], attr_data[1]]);
        }
        ITEM_ATTR_CLASSIFICATION => {
            // C++ skips 1 byte (`stream.skip(1)`) — not stored.
        }
        ITEM_ATTR_WEIGHT => {
            item.weight = match attr_data.len() {
                0 => 0,
                1 => attr_data[0] as u32,
                2 => u16::from_le_bytes([attr_data[0], attr_data[1]]) as u32,
                _ => u32::from_le_bytes([attr_data[0], attr_data[1], attr_data[2], attr_data[3]]),
            };
        }
        ITEM_ATTR_ROTATETO if attr_data.len() >= 2 => {
            item.rotate_to = u16::from_le_bytes([attr_data[0], attr_data[1]]);
        }
        ITEM_ATTR_TOPORDER if !attr_data.is_empty() => {
            item.always_on_top_order = attr_data[0];
        }
        _ => {}
    }
}

/// OTB flags / group — `src/items.cpp` `Items::loadFromOtb` ~438–457, `src/itemloader.h` `itemflags_t`.
impl ItemType {
    // ── Flag constants (`itemflags_t`) ──
    const FLAG_BLOCK_SOLID: u32 = 1 << 0;
    const FLAG_BLOCK_PROJECTILE: u32 = 1 << 1;
    const FLAG_BLOCK_PATHFIND: u32 = 1 << 2;
    const FLAG_HAS_HEIGHT: u32 = 1 << 3;
    const FLAG_USEABLE: u32 = 1 << 4;
    const FLAG_PICKUPABLE: u32 = 1 << 5;
    const FLAG_MOVEABLE: u32 = 1 << 6;
    const FLAG_STACKABLE: u32 = 1 << 7;
    const FLAG_ALWAYSONTOP: u32 = 1 << 13;
    const FLAG_READABLE: u32 = 1 << 14;
    const FLAG_ROTATABLE: u32 = 1 << 15;
    const FLAG_HANGABLE: u32 = 1 << 16;
    const FLAG_VERTICAL: u32 = 1 << 17;
    const FLAG_HORIZONTAL: u32 = 1 << 18;
    const FLAG_ALLOWDISTREAD: u32 = 1 << 20;
    const FLAG_LOOKTHROUGH: u32 = 1 << 23;
    const FLAG_ANIMATION: u32 = 1 << 24;
    const FLAG_FORCEUSE: u32 = 1 << 26;

    // ── Group constants (`itemgroup_t`) ──
    /// `itemgroup_t::ITEM_GROUP_GROUND` — numeric value `1` (`src/itemloader.h`).
    pub const GROUP_GROUND: u8 = 1;
    /// `itemgroup_t::ITEM_GROUP_CONTAINER`.
    pub const GROUP_CONTAINER: u8 = 2;
    /// `itemgroup_t::ITEM_GROUP_SPLASH` (`src/itemloader.h`).
    const GROUP_SPLASH: u8 = 11;
    /// `itemgroup_t::ITEM_GROUP_FLUID`.
    const GROUP_FLUID: u8 = 12;

    // ── Flag accessors ──

    #[inline]
    pub fn block_solid(&self) -> bool {
        self.flags & Self::FLAG_BLOCK_SOLID != 0
    }

    #[inline]
    pub fn block_projectile(&self) -> bool {
        self.flags & Self::FLAG_BLOCK_PROJECTILE != 0
    }

    #[inline]
    pub fn block_path_find(&self) -> bool {
        self.flags & Self::FLAG_BLOCK_PATHFIND != 0
    }

    #[inline]
    pub fn has_height(&self) -> bool {
        self.flags & Self::FLAG_HAS_HEIGHT != 0
    }

    #[inline]
    pub fn useable(&self) -> bool {
        self.flags & Self::FLAG_USEABLE != 0
    }

    #[inline]
    pub fn pickupable(&self) -> bool {
        self.flags & Self::FLAG_PICKUPABLE != 0
    }

    #[inline]
    pub fn moveable(&self) -> bool {
        self.flags & Self::FLAG_MOVEABLE != 0
    }

    #[inline]
    pub fn stackable(&self) -> bool {
        self.flags & Self::FLAG_STACKABLE != 0
    }

    #[inline]
    pub fn always_on_top(&self) -> bool {
        self.flags & Self::FLAG_ALWAYSONTOP != 0
    }

    #[inline]
    pub fn can_read_text(&self) -> bool {
        self.flags & Self::FLAG_READABLE != 0
    }

    #[inline]
    pub fn rotatable(&self) -> bool {
        self.flags & Self::FLAG_ROTATABLE != 0
    }

    #[inline]
    pub fn is_hangable(&self) -> bool {
        self.flags & Self::FLAG_HANGABLE != 0
    }

    #[inline]
    pub fn is_vertical(&self) -> bool {
        self.flags & Self::FLAG_VERTICAL != 0
    }

    #[inline]
    pub fn is_horizontal(&self) -> bool {
        self.flags & Self::FLAG_HORIZONTAL != 0
    }

    #[inline]
    pub fn allow_dist_read(&self) -> bool {
        self.flags & Self::FLAG_ALLOWDISTREAD != 0
    }

    #[inline]
    pub fn look_through(&self) -> bool {
        self.flags & Self::FLAG_LOOKTHROUGH != 0
    }

    #[inline]
    pub fn is_animation(&self) -> bool {
        self.flags & Self::FLAG_ANIMATION != 0
    }

    #[inline]
    pub fn force_use(&self) -> bool {
        self.flags & Self::FLAG_FORCEUSE != 0
    }

    // ── Group accessors ──

    /// `ItemType::isGroundTile()` (`src/items.h`).
    #[inline]
    pub fn is_ground_tile(&self) -> bool {
        self.group == Self::GROUP_GROUND
    }

    #[inline]
    pub fn is_splash(&self) -> bool {
        self.group == Self::GROUP_SPLASH
    }

    #[inline]
    pub fn is_fluid_container(&self) -> bool {
        self.group == Self::GROUP_FLUID
    }
}

fn expect_raw(data: &[u8], index: &mut usize, expected: u8, path: &Path) -> Result<()> {
    if *index >= data.len() {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "unexpected end of OTB stream".to_string(),
        });
    }
    if data[*index] != expected {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: format!(
                "invalid OTB token at offset {}: expected {expected:#04x}, got {:#04x}",
                *index, data[*index]
            ),
        });
    }
    *index += 1;
    Ok(())
}

fn read_data_u8(data: &[u8], index: &mut usize, path: &Path) -> Result<u8> {
    if *index >= data.len() {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "unexpected end of OTB stream".to_string(),
        });
    }
    let value = if data[*index] == ESCAPE {
        *index += 1;
        if *index >= data.len() {
            return Err(TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: "dangling OTB escape byte".to_string(),
            });
        }
        data[*index]
    } else {
        data[*index]
    };
    *index += 1;
    Ok(value)
}

fn read_data_u16(data: &[u8], index: &mut usize, path: &Path) -> Result<u16> {
    let lo = read_data_u8(data, index, path)?;
    let hi = read_data_u8(data, index, path)?;
    Ok(u16::from_le_bytes([lo, hi]))
}

fn read_data_u32(data: &[u8], index: &mut usize, path: &Path) -> Result<u32> {
    let b0 = read_data_u8(data, index, path)?;
    let b1 = read_data_u8(data, index, path)?;
    let b2 = read_data_u8(data, index, path)?;
    let b3 = read_data_u8(data, index, path)?;
    Ok(u32::from_le_bytes([b0, b1, b2, b3]))
}

fn read_data_bytes(data: &[u8], index: &mut usize, count: usize, path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(count);
    for _ in 0..count {
        bytes.push(read_data_u8(data, index, path)?);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::OtbLoader;
    use std::path::Path;

    #[test]
    fn repo_items_otb_passes_root_validation_and_loads() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/items/items.otb");
        let db = OtbLoader::load_from_file(&path).expect("items.otb should load");
        assert!(
            db.contains_key(&100) || db.len() > 100,
            "expected non-trivial item db"
        );
    }

    /// Verify `ITEM_ATTR_SPEED` is parsed from OTB for ground tiles (`group == 1`).
    #[test]
    fn ground_items_have_otb_speed() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/items/items.otb");
        let db = OtbLoader::load_from_file(&path).expect("items.otb should load");
        let ground_with_speed: Vec<_> = db
            .values()
            .filter(|it| it.group == super::ItemType::GROUP_GROUND && it.speed > 0)
            .collect();
        assert!(
            !ground_with_speed.is_empty(),
            "expected at least one ground item with OTB speed > 0"
        );
        // Spot-check: print a few for manual verification.
        for it in ground_with_speed.iter().take(3) {
            eprintln!(
                "  ground id={} client_id={} speed={}",
                it.server_id, it.client_id, it.speed
            );
        }
    }
}
