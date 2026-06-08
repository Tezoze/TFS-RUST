//! OTB item loader and item metadata model.
//! C++ reference: `src/items.cpp` (`Items::loadFromOtb`), `src/itemloader.h` (`itemattrib_t`, `itemflags_t`).

use std::collections::HashMap;
use std::path::Path;
use crate::item_abilities::ItemAbilities;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::warn;

#[derive(Debug, Clone)]
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
    /// C++ `ItemType::abilities` — `src/items.h` (`struct Abilities`); populated from items.xml (`src/items.cpp`).
    pub abilities: ItemAbilities,
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
    /// `SlotPositionBits` — `src/items.h`; default `SLOTP_HAND` (`src/items.h` ItemType).
    pub slot_position: u32,
    /// C++ `ItemType::floorChange` — `TileStatesMap` bits (`src/items.cpp`, `src/tile.h`).
    pub floor_change: u8,
    /// C++ `ItemType::charges` (`src/items.h`); default `0`.
    pub charges: u32,
    /// C++ `ItemType::maxItems` (`src/items.h`) from XML `containersize` only (OTB `ITEM_ATTR_MAXITEMS` skipped like C++).
    pub max_items: u16,
    /// `WeaponType_t` — `src/const.h`; default `WEAPON_NONE`.
    pub weapon_type: u8,
    /// `items.xml` combat — `ItemType::attack` (`src/items.h`).
    pub attack: i32,
    pub defense: i32,
    pub extra_defense: i32,
    /// `article="..."` on `<item>` (`src/items.cpp`).
    pub article: String,
    /// `plural="..."` on `<item>`; empty → `ItemType::getPluralName` rules (`src/items.h`).
    pub plural_name: String,
    /// Default `true` — `ItemType::showCount` (`src/items.h`).
    pub show_count: bool,
    /// `Ammo_t` — `src/const.h`; `AMMO_NONE` = 0.
    pub ammo_type: u8,
    /// `items.xml` — `ItemType::armor`.
    pub armor: i32,
    /// Milliseconds — `ItemType::attackSpeed` (`item.cpp` `/ 1000` for display).
    pub attack_speed: u32,
    /// `range` — `ItemType::shootRange`; default `1` (`src/items.h` / `items.cpp` `parseItemNode`).
    pub shoot_range: i32,
    /// `hitchance` — `ItemType::hitChance` (`int8_t`), clamped in C++ to `[-100, 100]` (`items.cpp`).
    pub hit_chance: i8,
    /// `maxhitchance` — `ItemType::maxHitChance` (`int32_t`), default `-1` (`src/items.h`).
    pub max_hit_chance: i32,
    /// XML override parity fields (`Items::parseItemNode` in `src/items.cpp`).
    pub moveable_override: Option<bool>,
    pub block_projectile_override: Option<bool>,
    pub block_solid_override: Option<bool>,
    pub allow_dist_read_override: Option<bool>,
    /// `readable` / `writeable` set `canReadText` in C++; OTB uses `FLAG_READABLE`. When present,
    /// XML wins (`src/items.cpp` `ITEM_PARSE_READABLE`, `ITEM_PARSE_WRITEABLE`).
    pub can_read_text_override: Option<bool>,
    pub allow_pickupable: bool,
    pub force_serialize: bool,
    pub replaceable: bool,
    pub walk_stack: bool,
    pub store_item: bool,
    pub can_write_text: bool,
    pub max_text_len: u16,
    /// C++ `ItemType::showCharges` — `src/items.h`.
    pub show_charges: bool,
    /// C++ `ItemType::showAttributes` — `src/items.h`; default `false`.
    pub show_attributes: bool,
    /// C++ `ItemType::minReqLevel` — `src/items.h`; default `0`.
    pub min_req_level: u32,
    /// C++ `ItemType::minReqMagicLevel` — `src/items.h`; default `0`.
    pub min_req_magic_level: u32,
    /// C++ `ItemType::vocEquipMap` keys — lowercase vocation names from items.xml `vocation`.
    pub voc_equip_names: Vec<String>,
    /// C++ `ItemTypes_t` from items.xml `type="..."` — `src/items.h` (`ITEM_TYPE_DEPOT`, etc.).
    pub type_tag: u8,
}

/// `SLOTP_HAND` — `src/items.h`
const SLOTP_LEFT: u32 = 1 << 5;
const SLOTP_RIGHT: u32 = 1 << 4;
const SLOTP_HAND_DEFAULT: u32 = SLOTP_LEFT | SLOTP_RIGHT;

impl Default for ItemType {
    fn default() -> Self {
        Self {
            id: 0,
            server_id: 0,
            client_id: 0,
            group: 0,
            name: String::new(),
            flags: 0,
            speed: 0,
            abilities: ItemAbilities::default(),
            light_level: 0,
            light_color: 0,
            ware_id: 0,
            weight: 0,
            rotate_to: 0,
            description: String::new(),
            xml_attributes: HashMap::new(),
            always_on_top_order: 0,
            slot_position: SLOTP_HAND_DEFAULT,
            floor_change: 0,
            charges: 0,
            max_items: 8,
            weapon_type: 0,
            attack: 0,
            defense: 0,
            extra_defense: 0,
            article: String::new(),
            plural_name: String::new(),
            show_count: true,
            ammo_type: 0,
            armor: 0,
            attack_speed: 0,
            shoot_range: 1,
            hit_chance: 0,
            max_hit_chance: -1,
            moveable_override: None,
            block_projectile_override: None,
            block_solid_override: None,
            allow_dist_read_override: None,
            can_read_text_override: None,
            allow_pickupable: false,
            force_serialize: false,
            replaceable: true,
            walk_stack: true,
            store_item: false,
            can_write_text: false,
            max_text_len: 0,
            show_charges: false,
            show_attributes: false,
            min_req_level: 0,
            min_req_magic_level: 0,
            voc_equip_names: Vec::new(),
            type_tag: 0,
        }
    }
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
/// C++ skips in `loadFromOtb` — not handled in `apply_attr`. IDs kept for tests / reference (`src/items.cpp`).
#[allow(dead_code)]
const ITEM_ATTR_NAME: u8 = 0x12;
#[allow(dead_code)]
const ITEM_ATTR_DESCR: u8 = 0x13;
/// `itemattrib_t::ITEM_ATTR_SPEED` (`src/itemloader.h`) — ground tile speed, `uint16_t`.
const ITEM_ATTR_SPEED: u8 = 0x14;
/// `itemattrib_t::ITEM_ATTR_MAXITEMS` (`src/itemloader.h`) — C++ skips; see `otb_does_not_apply_maxitems_attribute`.
#[allow(dead_code)]
const ITEM_ATTR_MAXITEMS: u8 = 0x16;
/// `itemattrib_t::ITEM_ATTR_WEIGHT` — **0x17**, not `0x16` (`MAXITEMS`) (`src/itemloader.h`). C++ skips in OTB load.
#[allow(dead_code)]
const ITEM_ATTR_WEIGHT: u8 = 0x17;
/// `itemattrib_t::ITEM_ATTR_ROTATETO` — **0x1E** (`src/itemloader.h`). C++ skips in OTB load.
#[allow(dead_code)]
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
/// OTB `majorVersion` (format) + `minorVersion` (client) per era. The OTB file is self-describing and
/// the Rust binary serves both eras by `clientVersion`, so accept either era's pair (each C++ tree
/// accepts only its own: 1098 `src/items.cpp` major 3 / `CLIENT_VERSION_1098` 57; 772
/// `gameserver/src/items.cpp` major 2 / `CLIENT_VERSION_800` 7). Item-node attribute IDs are identical
/// across both (`itemattrib_t` is the same auto-incremented enum in both `itemloader.h`).
const OTB_MAJOR_1098: u32 = 3;
const CLIENT_VERSION_1098: u32 = 57;
const OTB_MAJOR_772: u32 = 2;
const CLIENT_VERSION_800: u32 = 7;

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
                apply_attr(&mut item, attr_type, &attr_data, path)?;
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
pub(crate) fn validate_items_otb_root_version_for_patch(data: &[u8], path: &Path) -> Result<()> {
    validate_items_otb_root_version(data, path)
}

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
    // Accept either supported era's self-describing version pair (1098 major 3 / minor 57;
    // 772 major 2 / minor 7). C++ ref: `src/items.cpp` (1098) and `gameserver/src/items.cpp` (772)
    // `Items::loadFromOtb` version gates.
    let ok = (major == OTB_MAJOR_1098 && minor >= CLIENT_VERSION_1098)
        || (major == OTB_MAJOR_772 && minor == CLIENT_VERSION_800);
    if !ok {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: format!(
                "items.otb: unsupported version (major {major}, minor {minor}); \
                 expected major {OTB_MAJOR_1098}/minor>={CLIENT_VERSION_1098} (10.98) \
                 or major {OTB_MAJOR_772}/minor {CLIENT_VERSION_800} (7.72) (Items::loadFromOtb)"
            ),
        });
    }
    Ok(())
}

fn apply_attr(item: &mut ItemType, attr_type: u8, attr_data: &[u8], path: &Path) -> Result<()> {
    let invalid_attr_len = |expected: &str| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: format!(
            "items.otb: invalid length {} for attr {attr_type:#04x}, got {}",
            expected,
            attr_data.len()
        ),
    };

    // C++ `Items::loadFromOtb` only reads a subset of `itemattrib_t`; `ITEM_ATTR_NAME` (0x12),
    // `ITEM_ATTR_DESCR` (0x13), `ITEM_ATTR_MAXITEMS` (0x16), `ITEM_ATTR_WEIGHT` (0x17),
    // `ITEM_ATTR_ROTATETO` (0x1E) fall through to `default: stream.skip(datalen)` — not stored (`src/items.cpp`).
    match attr_type {
        ITEM_ATTR_SERVERID => {
            if attr_data.len() != 2 {
                return Err(invalid_attr_len("(expected 2 bytes)"));
            }
            item.server_id = u16::from_le_bytes([attr_data[0], attr_data[1]]);
        }
        ITEM_ATTR_CLIENTID => {
            if attr_data.len() != 2 {
                return Err(invalid_attr_len("(expected 2 bytes)"));
            }
            item.client_id = u16::from_le_bytes([attr_data[0], attr_data[1]]);
        }
        ITEM_ATTR_SPEED => {
            if attr_data.len() != 2 {
                return Err(invalid_attr_len("(expected 2 bytes)"));
            }
            item.speed = u16::from_le_bytes([attr_data[0], attr_data[1]]);
        }
        ITEM_ATTR_LIGHT2 => {
            if attr_data.len() != 4 {
                return Err(invalid_attr_len("(expected 4 bytes)"));
            }
            // C++ `lightBlock2` is packed {u16 lightLevel, u16 lightColor} (`src/itemloader.h`).
            item.light_level = u16::from_le_bytes([attr_data[0], attr_data[1]]) as u8;
            item.light_color = u16::from_le_bytes([attr_data[2], attr_data[3]]) as u8;
        }
        ITEM_ATTR_WAREID => {
            if attr_data.len() != 2 {
                return Err(invalid_attr_len("(expected 2 bytes)"));
            }
            item.ware_id = u16::from_le_bytes([attr_data[0], attr_data[1]]);
        }
        ITEM_ATTR_CLASSIFICATION => {
            // C++ skips 1 byte (`stream.skip(1)`) — not stored.
        }
        ITEM_ATTR_TOPORDER => {
            if attr_data.len() != 1 {
                return Err(invalid_attr_len("(expected 1 byte)"));
            }
            item.always_on_top_order = attr_data[0];
        }
        _ => {}
    }
    Ok(())
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

    /// C++ `ItemType::blockSolid` — OTB flag, overridden by XML `blocking` (`items.cpp` `ITEM_PARSE_BLOCKING`).
    #[inline]
    pub fn block_solid(&self) -> bool {
        self.block_solid_override
            .unwrap_or(self.flags & Self::FLAG_BLOCK_SOLID != 0)
    }

    #[inline]
    pub fn block_projectile(&self) -> bool {
        self.block_projectile_override
            .unwrap_or(self.flags & Self::FLAG_BLOCK_PROJECTILE != 0)
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
        self.flags & Self::FLAG_PICKUPABLE != 0 || self.allow_pickupable
    }

    #[inline]
    pub fn moveable(&self) -> bool {
        self.moveable_override
            .unwrap_or(self.flags & Self::FLAG_MOVEABLE != 0)
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
        self.can_read_text_override
            .unwrap_or(self.flags & Self::FLAG_READABLE != 0)
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

    /// C++ `ItemType::allowDistRead` — XML `allowdistread` (`items.cpp` `ITEM_PARSE_ALLOWDISTREAD`).
    #[inline]
    pub fn allow_dist_read(&self) -> bool {
        self.allow_dist_read_override
            .unwrap_or(self.flags & Self::FLAG_ALLOWDISTREAD != 0)
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

fn read_data_bytes(data: &[u8], index: &mut usize, len: usize, path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(len);
    for _ in 0..len {
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

    /// C++ skips `ITEM_ATTR_MAXITEMS` in `loadFromOtb`; `maxItems` comes from items.xml `containersize`.
    #[test]
    fn otb_does_not_apply_maxitems_attribute() {
        let mut item = super::ItemType::default();
        let path = Path::new("items.otb");
        let data = 12u16.to_le_bytes();
        let res = super::apply_attr(&mut item, super::ITEM_ATTR_MAXITEMS, &data, path);
        assert!(res.is_ok());
        assert_eq!(item.max_items, 8);
    }

    #[test]
    fn fails_on_invalid_known_attr_length() {
        let mut item = super::ItemType::default();
        let path = Path::new("items.otb");
        let err = super::apply_attr(&mut item, super::ITEM_ATTR_CLIENTID, &[0x01], path)
            .expect_err("must error");
        match err {
            tfs_rust_common::error::TfsRustError::Content { message, .. } => {
                assert!(message.contains("invalid length"));
                assert!(message.contains("0x11"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

}
