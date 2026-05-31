//! Item database loaders for OTB + items.xml.
//! C++ reference: `src/items.cpp` (`Items::loadFromXml`, `Items::parseItemNode`), `src/items.h` (`ItemType`).

use crate::item_abilities::apply_ability_attribute;
use crate::items_xml_keys::is_known_xml_key;
use crate::otb::{ItemType, OtbLoader};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::{info, warn};

static UNKNOWN_XML_KEYS_WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

/// C++ `ItemTypes_t` — `src/items.h`.
pub const ITEM_TYPE_NONE: u8 = 0;
pub const ITEM_TYPE_DEPOT: u8 = 1;
pub const ITEM_TYPE_MAILBOX: u8 = 2;
pub const ITEM_TYPE_TRASHHOLDER: u8 = 3;
pub const ITEM_TYPE_CONTAINER: u8 = 4;

#[derive(Clone)]
pub struct ItemDatabase {
    pub items: HashMap<u16, ItemType>,
    /// C++ `clientIdToServerIdMap` — first server id wins for duplicate client ids (`src/items.cpp` `loadFromOtb`).
    pub client_to_server: HashMap<u16, u16>,
}

fn extract_attribute_key_value(elem: &BytesStart<'_>) -> Option<(String, String)> {
    let mut key: Option<String> = None;
    let mut value: Option<String> = None;
    for attr in elem.attributes() {
        let Ok(attr) = attr else {
            return None;
        };
        match attr.key.as_ref() {
            b"key" => key = Some(String::from_utf8_lossy(attr.value.as_ref()).into_owned()),
            b"value" => value = Some(String::from_utf8_lossy(attr.value.as_ref()).into_owned()),
            _ => {}
        }
    }
    match (key, value) {
        (Some(k), Some(v)) => Some((k, v)),
        _ => None,
    }
}

impl ItemDatabase {
    /// OTB / `ItemType::clientId` for map and inventory protocol (`addItem`); 0 if unknown.
    // C++: `Items::getItemType(serverId).clientId` — `src/items.cpp`
    #[inline]
    pub fn client_id_for_server(&self, server_id: u16) -> u16 {
        self.items
            .get(&server_id)
            .map(|t| t.client_id)
            .unwrap_or(0)
    }

    /// Reverse lookup: OT client sprite id → server item id (`Items::getServerId` patterns / `clientIdToServerIdMap`).
    #[inline]
    pub fn server_id_for_client(&self, client_id: u16) -> Option<u16> {
        self.client_to_server.get(&client_id).copied()
    }

    /// C++ `Items::buildInventoryList` — `src/items.cpp` (lines 511–530): `clientId`s for equipment-relevant
    /// item types (weapon, ammo, attack/defense, armor, certain slot bits). Sorted ascending (no dedup: matches C++ push order + `std::sort`).
    pub fn inventory_client_ids(&self) -> Vec<u16> {
        let mut v: Vec<u16> = self
            .items
            .values()
            .filter(|t| {
                t.weapon_type != 0
                    || t.ammo_type != 0
                    || t.attack != 0
                    || t.defense != 0
                    || t.extra_defense != 0
                    || t.armor != 0
                    || (t.slot_position & (SLOTP_NECKLACE
                        | SLOTP_RING
                        | SLOTP_AMMO
                        | SLOTP_FEET
                        | SLOTP_HEAD
                        | SLOTP_ARMOR
                        | SLOTP_LEGS))
                        != 0
            })
            .map(|t| t.client_id)
            .collect();
        v.sort_unstable();
        v
    }

    /// OTB `FLAG_ANIMATION` — extra `0xFE` before duration in `addItem` (`src/networkmessage.cpp`).
    #[inline]
    pub fn is_animation_for_server(&self, server_id: u16) -> bool {
        self.items
            .get(&server_id)
            .is_some_and(|t| t.is_animation())
    }

    /// OTB `FLAG_STACKABLE` — `ItemType::stackable` (`src/items.cpp`).
    #[inline]
    pub fn stackable_for_server(&self, server_id: u16) -> bool {
        self.items
            .get(&server_id)
            .is_some_and(|t| t.stackable())
    }

    /// Splash / fluid container group — `NetworkMessage::addItem` fluid byte (`src/networkmessage.cpp`).
    #[inline]
    pub fn is_splash_or_fluid_for_server(&self, server_id: u16) -> bool {
        self.items
            .get(&server_id)
            .is_some_and(|t| t.is_splash() || t.is_fluid_container())
    }

    /// Whether this item behaves as a container for loot nesting (`loadLootContainer` in TFS).
    /// C++ source of truth: `ItemType::isContainer()` => `group == ITEM_GROUP_CONTAINER` (`src/items.h`).
    pub fn is_container(&self, id: u16) -> bool {
        self.items
            .get(&id)
            .is_some_and(|t| t.group == ItemType::GROUP_CONTAINER)
    }

    /// C++ `ItemType::isDepot()` — `src/items.h` (`type == ITEM_TYPE_DEPOT`).
    #[inline]
    pub fn is_depot(&self, id: u16) -> bool {
        self.items
            .get(&id)
            .is_some_and(|t| t.type_tag == ITEM_TYPE_DEPOT)
    }

    /// Openable as a container window — normal bags or map depot lockers (`actions.cpp`).
    #[inline]
    pub fn is_openable_container(&self, id: u16) -> bool {
        self.is_container(id) || self.is_depot(id)
    }

    /// Default `subType` for loot when omitted: C++ uses `ItemType::charges` (`src/items.h`).
    pub fn charges_default(&self, id: u16) -> i32 {
        self.items.get(&id).map(|t| t.charges as i32).unwrap_or(0)
    }

    /// TFS `ItemType::speed` for ground — `Creature::getStepDuration` (`creature.cpp` ~1513–1521).
    /// Reads from OTB `ITEM_ATTR_SPEED` (`src/items.cpp` `loadFromOtb`), NOT `items.xml` `"speed"`
    /// (which is equipment speed bonus = `abilities.speed`).
    #[inline]
    pub fn ground_speed_for_item(&self, server_id: u16) -> u32 {
        let raw = self
            .items
            .get(&server_id)
            .map(|t| t.speed)
            .unwrap_or(0);
        if raw == 0 {
            150
        } else {
            raw as u32
        }
    }

    /// Resolve `name="..."` loot references; errors if unknown or ambiguous (see `monsters.cpp` `loadLootItem`).
    pub fn item_id_by_exact_name(&self, name: &str, file: &str) -> Result<u16> {
        let lower = name.to_ascii_lowercase();
        let mut matches: Vec<u16> = self
            .items
            .iter()
            .filter(|(_, it)| !it.name.is_empty() && it.name.to_ascii_lowercase() == lower)
            .map(|(&id, _)| id)
            .collect();
        matches.sort_unstable();
        match matches.len() {
            0 => Err(TfsRustError::Content {
                file: file.to_string(),
                message: format!("unknown loot item name \"{name}\""),
            }),
            1 => Ok(matches[0]),
            _ => Err(TfsRustError::Content {
                file: file.to_string(),
                message: format!("non-unique loot item name \"{name}\""),
            }),
        }
    }

    pub fn load(otb_path: &Path, xml_path: &Path) -> Result<Self> {
        info!("Loading OTB from {:?}", otb_path);
        let mut items = OtbLoader::load_from_file(otb_path)?;

        info!("Merging items.xml from {:?}", xml_path);
        Self::merge_xml(&mut items, xml_path)?;

        let client_to_server = Self::build_client_to_server_map(&items);
        Ok(Self {
            items,
            client_to_server,
        })
    }

    /// C++ `clientIdToServerIdMap` — `src/items.cpp` `loadFromOtb` emplaces every `clientId` including `0`.
    /// We skip `client_id == 0` so the map has no useless entries; see `tasks/items-parsing-audit.md` OTB-2.
    /// C++ `loadFromOtb` emplaces `clientId == 0`; reverse lookup would be useless, so we omit those entries.
    fn build_client_to_server_map(items: &HashMap<u16, ItemType>) -> HashMap<u16, u16> {
        let mut m = HashMap::new();
        for (&sid, it) in items {
            if it.client_id != 0 {
                m.entry(it.client_id).or_insert(sid);
            }
        }
        m
    }

    fn merge_xml(items: &mut HashMap<u16, ItemType>, xml_path: &Path) -> Result<()> {
        let xml_str = std::fs::read_to_string(xml_path).map_err(|e| TfsRustError::Content {
            file: xml_path.to_string_lossy().into(),
            message: e.to_string(),
        })?;

        let mut reader = Reader::from_str(&xml_str);
        reader.trim_text(true);
        let mut buf = Vec::new();
        let mut current_ids: Vec<u16> = Vec::new();
        let mut xml_defined_ids: HashSet<u16> = HashSet::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"item" => {
                    current_ids.clear();
                    let mut id: Option<u16> = None;
                    let mut from_id: Option<u16> = None;
                    let mut to_id: Option<u16> = None;
                    let mut name: Option<String> = None;
                    let mut article: Option<String> = None;
                    let mut plural: Option<String> = None;

                    for attr in e.attributes() {
                        let attr = attr.map_err(|err| TfsRustError::Content {
                            file: xml_path.to_string_lossy().into_owned(),
                            message: err.to_string(),
                        })?;
                        let key = attr.key.as_ref();
                        let value = String::from_utf8_lossy(attr.value.as_ref()).into_owned();
                        match key {
                            b"id" => {
                                id = Some(parse_u16_attr(xml_path, "id", &value)?);
                            }
                            b"fromid" => {
                                from_id = Some(parse_u16_attr(xml_path, "fromid", &value)?);
                            }
                            b"toid" => {
                                to_id = Some(parse_u16_attr(xml_path, "toid", &value)?);
                            }
                            b"name" => name = Some(value),
                            b"article" => article = Some(value),
                            b"plural" => plural = Some(value),
                            _ => {}
                        }
                    }

                    if let Some(single) = id {
                        current_ids.push(single);
                    } else if let (Some(start), Some(end)) = (from_id, to_id) {
                        if start > end {
                            return Err(TfsRustError::Content {
                                file: xml_path.to_string_lossy().into_owned(),
                                message: format!("invalid item range: fromid {start} > toid {end}"),
                            });
                        }
                        current_ids.extend(start..=end);
                    } else {
                        return Err(TfsRustError::Content {
                            file: xml_path.to_string_lossy().into_owned(),
                            message: "item entry missing required id/fromid+toid".to_string(),
                        });
                    }

                    current_ids.retain(|item_id| {
                        let duplicate = !xml_defined_ids.insert(*item_id);
                        if duplicate {
                            warn!(
                                target: "tfs_rust_content::items",
                                item_id,
                                "duplicate item definition in items.xml (C++ warns and keeps first definition)"
                            );
                        }
                        !duplicate
                    });

                    if let Some(name) = name {
                        for id in &current_ids {
                            let entry = items.entry(*id).or_insert_with(|| ItemType {
                                id: *id,
                                server_id: *id,
                                ..ItemType::default()
                            });
                            entry.name = name.clone();
                        }
                    }
                    if let Some(a) = article {
                        for id in &current_ids {
                            let entry = items.entry(*id).or_insert_with(|| ItemType {
                                id: *id,
                                server_id: *id,
                                ..ItemType::default()
                            });
                            entry.article = a.clone();
                        }
                    }
                    if let Some(p) = plural {
                        for id in &current_ids {
                            let entry = items.entry(*id).or_insert_with(|| ItemType {
                                id: *id,
                                server_id: *id,
                                ..ItemType::default()
                            });
                            entry.plural_name = p.clone();
                        }
                    }
                }
                Ok(Event::Empty(e)) if e.name().as_ref() == b"attribute" => {
                    Self::apply_attribute_event(items, &current_ids, &e, xml_path);
                }
                Ok(Event::Start(e)) if e.name().as_ref() == b"attribute" => {
                    let parent = Self::apply_attribute_event(items, &current_ids, &e, xml_path);
                    let parent_key = parent.as_ref().map(|(k, _)| k.clone());
                    let mut depth = 1usize;
                    while depth > 0 {
                        match reader.read_event_into(&mut buf) {
                            Ok(Event::Start(inner)) if inner.name().as_ref() == b"attribute" => {
                                depth += 1;
                                let child = extract_attribute_key_value(&inner);
                                if depth == 2 {
                                    if let (Some(parent_key), Some((child_key, child_value))) =
                                        (&parent_key, child)
                                    {
                                        for id in &current_ids {
                                            let entry = items.entry(*id).or_insert_with(|| ItemType {
                                                id: *id,
                                                server_id: *id,
                                                ..ItemType::default()
                                            });
                                            apply_nested_xml_attribute(
                                                entry,
                                                parent_key,
                                                &child_key,
                                                &child_value,
                                            );
                                        }
                                    }
                                }
                            }
                            Ok(Event::Empty(inner)) if inner.name().as_ref() == b"attribute" => {
                                let child = extract_attribute_key_value(&inner);
                                if depth == 1 {
                                    if let (Some(parent_key), Some((child_key, child_value))) =
                                        (&parent_key, child)
                                    {
                                        for id in &current_ids {
                                            let entry = items.entry(*id).or_insert_with(|| ItemType {
                                                id: *id,
                                                server_id: *id,
                                                ..ItemType::default()
                                            });
                                            apply_nested_xml_attribute(
                                                entry,
                                                parent_key,
                                                &child_key,
                                                &child_value,
                                            );
                                        }
                                    }
                                }
                            }
                            Ok(Event::End(inner)) if inner.name().as_ref() == b"attribute" => {
                                depth = depth.saturating_sub(1);
                            }
                            Ok(Event::Eof) => break,
                            Err(err) => {
                                return Err(TfsRustError::Content {
                                    file: xml_path.to_string_lossy().into_owned(),
                                    message: err.to_string(),
                                });
                            }
                            _ => {}
                        }
                        buf.clear();
                    }
                }
                Ok(Event::End(e)) if e.name().as_ref() == b"item" => {
                    for &id in &current_ids {
                        if let Some(it) = items.get(&id) {
                            warn_bed_type_mismatch(id, it);
                        }
                    }
                    current_ids.clear();
                }
                Ok(Event::Eof) => break,
                Err(err) => {
                    return Err(TfsRustError::Content {
                        file: xml_path.to_string_lossy().into_owned(),
                        message: err.to_string(),
                    });
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(())
    }

    fn apply_attribute_event(
        items: &mut HashMap<u16, ItemType>,
        current_ids: &[u16],
        elem: &BytesStart<'_>,
        xml_path: &Path,
    ) -> Option<(String, String)> {
        if current_ids.is_empty() {
            return None;
        }

        let Some((key, value)) = extract_attribute_key_value(elem) else {
            warn!(
                target: "tfs_rust_content::items",
                file = %xml_path.display(),
                "invalid xml attribute encoding in items.xml"
            );
            return None;
        };
        for id in current_ids {
            let entry = items.entry(*id).or_insert_with(|| ItemType {
                id: *id,
                server_id: *id,
                ..ItemType::default()
            });
            apply_xml_attribute(entry, &key, &value, *id);
        }

        Some((key, value))
    }
}

fn parse_u16_attr(path: &Path, name: &str, value: &str) -> Result<u16> {
    value.parse::<u16>().map_err(|err| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: format!("invalid {name} '{value}': {err}"),
    })
}

/// `SlotPositionBits` — `src/items.h`
const SLOTP_HEAD: u32 = 1 << 0;
const SLOTP_NECKLACE: u32 = 1 << 1;
const SLOTP_BACKPACK: u32 = 1 << 2;
const SLOTP_ARMOR: u32 = 1 << 3;
const SLOTP_RIGHT: u32 = 1 << 4;
const SLOTP_LEFT: u32 = 1 << 5;
const SLOTP_LEGS: u32 = 1 << 6;
const SLOTP_FEET: u32 = 1 << 7;
const SLOTP_RING: u32 = 1 << 8;
const SLOTP_AMMO: u32 = 1 << 9;
const SLOTP_TWO_HAND: u32 = 1 << 11;
const SLOTP_HAND: u32 = SLOTP_LEFT | SLOTP_RIGHT;

/// C++ `TileStatesMap` / `TILESTATE_FLOORCHANGE_*` — `src/items.cpp` ~154–162, `src/tile.h`.
fn floorchange_token_bit(tok: &str) -> Option<u8> {
    match tok {
        "down" => Some(1 << 0),
        "north" => Some(1 << 1),
        "south" => Some(1 << 2),
        "east" => Some(1 << 3),
        "west" => Some(1 << 4),
        "southalt" => Some(1 << 5),
        "eastalt" => Some(1 << 6),
        _ => None,
    }
}

/// C++ `ITEM_PARSE_SLOTTYPE` — single token only (`src/items.cpp` ~740–769). Returns `true` if recognized.
fn apply_slot_type_token(item: &mut ItemType, tok: &str) -> bool {
    match tok {
        "head" => {
            item.slot_position |= SLOTP_HEAD;
            true
        }
        "body" => {
            item.slot_position |= SLOTP_ARMOR;
            true
        }
        "legs" => {
            item.slot_position |= SLOTP_LEGS;
            true
        }
        "feet" => {
            item.slot_position |= SLOTP_FEET;
            true
        }
        "backpack" => {
            item.slot_position |= SLOTP_BACKPACK;
            true
        }
        "two-handed" => {
            item.slot_position |= SLOTP_TWO_HAND;
            true
        }
        "right-hand" => {
            item.slot_position &= !SLOTP_LEFT;
            true
        }
        "left-hand" => {
            item.slot_position &= !SLOTP_RIGHT;
            true
        }
        "necklace" => {
            item.slot_position |= SLOTP_NECKLACE;
            true
        }
        "ring" => {
            item.slot_position |= SLOTP_RING;
            true
        }
        "ammo" => {
            item.slot_position |= SLOTP_AMMO;
            true
        }
        "hand" => {
            item.slot_position |= SLOTP_HAND;
            true
        }
        _ => false,
    }
}

fn parse_xml_bool(value: &str) -> Option<bool> {
    let v = value.to_ascii_lowercase();
    match v.as_str() {
        "1" | "true" | "yes" => Some(true),
        "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

fn apply_nested_xml_attribute(item: &mut ItemType, parent_key: &str, key: &str, value: &str) {
    let composite = format!("{}.{}", parent_key.to_ascii_lowercase(), key.to_ascii_lowercase());
    item.xml_attributes.insert(composite, value.to_string());
}

/// C++: end of `Items::parseItemNode` — `src/items.cpp` (lines 1381–1383). Uses `xml_attributes` until
/// `ItemType` has `type` + transform fields like C++.
fn warn_bed_type_mismatch(item_id: u16, item: &ItemType) {
    let is_bed = item
        .xml_attributes
        .get("type")
        .is_some_and(|s| s.eq_ignore_ascii_case("bed"));
    if is_bed {
        return;
    }
    // C++: `transformToFree` or either `transformToOnUse` sex; `malesleeper`/`femalesleeper` alias the same cases.
    let tfree = u16_from_xml(&item.xml_attributes, "transformto");
    let m = u16_from_xml(&item.xml_attributes, "maletransformto")
        .max(u16_from_xml(&item.xml_attributes, "malesleeper"));
    let f = u16_from_xml(&item.xml_attributes, "femaletransformto")
        .max(u16_from_xml(&item.xml_attributes, "femalesleeper"));
    if tfree == 0 && m == 0 && f == 0 {
        return;
    }
    warn!(
        target: "tfs_rust_content::items",
        item_id,
        "item is not set as a bed-type (C++: Items::parseItemNode bed check)"
    );
}

fn u16_from_xml(map: &HashMap<String, String>, key: &str) -> u16 {
    map.get(key)
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn warn_unknown_xml_key_once(item_id: u16, key: &str) {
    let warned = UNKNOWN_XML_KEYS_WARNED.get_or_init(|| Mutex::new(HashSet::new()));
    let mut guard = match warned.lock() {
        Ok(g) => g,
        Err(_) => {
            warn!(
                target: "tfs_rust_content::items",
                item_id,
                key,
                "unknown items.xml key (warning cache poisoned); key stored in xml_attributes"
            );
            return;
        }
    };

    if guard.insert(key.to_string()) {
        warn!(
            target: "tfs_rust_content::items",
            item_id,
            key,
            "unknown items.xml key (first occurrence; key stored in xml_attributes)"
        );
    }
}

fn apply_xml_attribute(item: &mut ItemType, key: &str, value: &str, item_id: u16) {
    let k = key.to_ascii_lowercase();
    item.xml_attributes.insert(k.clone(), value.to_string());
    if apply_ability_attribute(&mut item.abilities, k.as_str(), value) {
        return;
    }
    match k.as_str() {
        "type" => {
            let type_value = value.to_ascii_lowercase();
            match type_value.as_str() {
                "container" => {
                    item.group = ItemType::GROUP_CONTAINER;
                    item.type_tag = ITEM_TYPE_CONTAINER;
                }
                "depot" => item.type_tag = ITEM_TYPE_DEPOT,
                "mailbox" => item.type_tag = ITEM_TYPE_MAILBOX,
                "trashholder" => item.type_tag = ITEM_TYPE_TRASHHOLDER,
                "magicfield" | "key" | "teleport" | "door" | "bed" | "rune" => {}
                _ => {
                    warn!(
                        target: "tfs_rust_content::items",
                        item_id,
                        key = "type",
                        value,
                        "unknown item type token in items.xml (C++ warns)"
                    );
                }
            }
        }
        "description" => item.description = value.to_string(),
        "weight" => {
            if let Ok(v) = value.parse::<u32>() {
                item.weight = v;
            }
        }
        "rotateto" => {
            if let Ok(v) = value.parse::<u16>() {
                item.rotate_to = v;
            }
        }
        // C++ `ITEM_PARSE_FLOORCHANGE` — `it.floorChange |=` bitmask (`src/items.cpp` ~670–678).
        "floorchange" => {
            let tok = value.trim().to_ascii_lowercase();
            if tok.is_empty() {
                warn!(
                    target: "tfs_rust_content::items",
                    item_id,
                    key = "floorchange",
                    "unknown floorChange in items.xml (empty token; C++ warns)"
                );
            } else if let Some(bit) = floorchange_token_bit(tok.as_str()) {
                item.floor_change |= bit;
            } else {
                warn!(
                    target: "tfs_rust_content::items",
                    item_id,
                    key = "floorchange",
                    value = %value,
                    "unknown floorChange in items.xml (C++ warns)"
                );
            }
        }
        // C++ `ITEM_PARSE_CHARGES` — `src/items.cpp` (`it.charges`).
        "charges" => {
            if let Ok(v) = value.parse::<u32>() {
                item.charges = v;
            }
        }
        // C++ `ITEM_PARSE_SLOTTYPE` — one string only; `head|body` is unknown (`src/items.cpp` ~740–769).
        "slottype" => {
            let tok = value.trim().to_ascii_lowercase();
            if tok.is_empty() || !apply_slot_type_token(item, tok.as_str()) {
                warn!(
                    target: "tfs_rust_content::items",
                    item_id,
                    key = "slottype",
                    value = %value,
                    "unknown slotType in items.xml (C++ warns)"
                );
            }
        }
        "weapontype" => {
            let w = value.to_ascii_lowercase();
            match w.as_str() {
                "sword" => item.weapon_type = 1,
                "club" => item.weapon_type = 2,
                "axe" => item.weapon_type = 3,
                "shield" => item.weapon_type = 4,
                "distance" => item.weapon_type = 5,
                "wand" => item.weapon_type = 6,
                "ammunition" => item.weapon_type = 7,
                _ => {
                    // C++: `src/items.cpp` (Unknown weaponType) — leaves `weaponType` unchanged on unknown.
                    warn!(
                        target: "tfs_rust_content::items",
                        item_id,
                        key = "weapontype",
                        value,
                        "unknown weaponType in items.xml (C++ warns)"
                    );
                }
            }
        }
        "attack" => {
            if let Ok(v) = value.parse::<i32>() {
                item.attack = v;
            }
        }
        "defense" => {
            if let Ok(v) = value.parse::<i32>() {
                item.defense = v;
            }
        }
        "extradef" | "extradefense" => {
            if let Ok(v) = value.parse::<i32>() {
                item.extra_defense = v;
            }
        }
        "moveable" | "movable" => {
            item.moveable_override = parse_xml_bool(value);
        }
        // C++ `ITEM_PARSE_BLOCKING` — `src/items.cpp` ~1346 (`it.blockSolid = value`).
        "blocking" => {
            item.block_solid_override = parse_xml_bool(value);
        }
        "blockprojectile" => {
            item.block_projectile_override = parse_xml_bool(value);
        }
        "allowpickupable" | "pickupable" => {
            item.allow_pickupable = parse_xml_bool(value).unwrap_or(false);
        }
        "forceserialize" | "forcesave" => {
            item.force_serialize = parse_xml_bool(value).unwrap_or(false);
        }
        "replaceable" => {
            item.replaceable = parse_xml_bool(value).unwrap_or(item.replaceable);
        }
        "walkstack" => {
            item.walk_stack = parse_xml_bool(value).unwrap_or(item.walk_stack);
        }
        "storeitem" => {
            item.store_item = parse_xml_bool(value).unwrap_or(false);
        }
        // C++ `ITEM_PARSE_READABLE` — `src/items.cpp` ~709 (`it.canReadText = value`).
        "readable" => {
            if let Some(v) = parse_xml_bool(value) {
                item.can_read_text_override = Some(v);
            }
        }
        // C++ `ITEM_PARSE_WRITEABLE` — `src/items.cpp` ~713–716 (`canWriteText` + `canReadText = canWriteText`).
        "writeable" => {
            if let Some(v) = parse_xml_bool(value) {
                item.can_write_text = v;
                item.can_read_text_override = Some(v);
            }
        }
        "maxtextlen" => {
            if let Ok(v) = value.parse::<u16>() {
                item.max_text_len = v;
            }
        }
        "containersize" => {
            if let Ok(v) = value.parse::<u16>() {
                item.max_items = v;
            }
        }
        "showcount" => {
            let v = value.to_ascii_lowercase();
            item.show_count = !(v == "0" || v == "false");
        }
        "ammotype" => {
            let v = value.to_ascii_lowercase();
            let parsed = match v.as_str() {
                "arrow" => Some(1u8),
                "bolt" => Some(2),
                "spear" | "huntingspear" | "hunting" => Some(3),
                "throwingstar" | "throwing star" => Some(4),
                "throwingknife" | "throwing knife" => Some(5),
                "stone" => Some(6),
                "snowball" => Some(7),
                _ => None,
            };
            if let Some(n) = parsed {
                item.ammo_type = n;
            } else {
                // C++: `getAmmoType` + warn when `AMMO_NONE` — `src/items.cpp` (Unknown ammoType).
                item.ammo_type = 0;
                warn!(
                    target: "tfs_rust_content::items",
                    item_id,
                    key = "ammotype",
                    value,
                    "unknown ammoType in items.xml (C++ warns)"
                );
            }
        }
        "armor" => {
            if let Ok(v) = value.parse::<i32>() {
                item.armor = v;
            }
        }
        "attackspeed" => {
            if let Ok(v) = value.parse::<u32>() {
                item.attack_speed = if v > 0 && v < 100 { 100 } else { v };
            }
        }
        "range" => {
            if let Ok(v) = value.parse::<i32>() {
                item.shoot_range = v;
            }
        }
        // C++ `ITEM_PARSE_HITCHANCE` — `src/items.cpp` ~851 (`hitChance` clamped to `[-100, 100]`).
        "hitchance" => {
            if let Ok(v) = value.parse::<i16>() {
                item.hit_chance = v.clamp(-100, 100) as i8;
            }
        }
        // C++ `ITEM_PARSE_MAXHITCHANCE` — `src/items.cpp` ~856 (`maxHitChance` capped at 100).
        "maxhitchance" => {
            if let Ok(v) = value.parse::<i32>() {
                item.max_hit_chance = v.clamp(0, 100);
            }
        }
        // C++ `ITEM_PARSE_SHOWCHARGES` / `ITEM_PARSE_SHOWATTRIBUTES` / `ITEM_PARSE_LEVELREQUIRED` /
        // `ITEM_PARSE_MAGICLEVELREQUIRED` / `ITEM_PARSE_VOCATION` — `src/items.cpp`.
        "showcharges" => {
            item.show_charges = parse_xml_bool(value).unwrap_or(false);
        }
        "showattributes" => {
            item.show_attributes = parse_xml_bool(value).unwrap_or(false);
        }
        "levelrequired" => {
            if let Ok(v) = value.parse::<u32>() {
                item.min_req_level = v;
            }
        }
        "magiclevelrequired" => {
            if let Ok(v) = value.parse::<u32>() {
                item.min_req_magic_level = v;
            }
        }
        "vocation" => {
            item
                .voc_equip_names
                .push(value.trim().to_ascii_lowercase());
        }
        // C++ `ITEM_PARSE_ALLOWDISTREAD` — `src/items.cpp` ~1351 (`it.allowDistRead`).
        "allowdistread" => {
            item.allow_dist_read_override = parse_xml_bool(value);
        }
        _ => {
            if !is_known_xml_key(&k) {
                warn_unknown_xml_key_once(item_id, &k);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_abilities::{combat_absorb_index, CONDITION_DRUNK, STAT_MAGICPOINTS};
    use std::fs;
    use tfs_rust_common::enums::{CombatType, Skill};

    #[test]
    fn applies_runtime_critical_xml_overrides() {
        let mut item = ItemType::default();
        apply_xml_attribute(&mut item, "moveable", "0", 100);
        apply_xml_attribute(&mut item, "blockprojectile", "1", 100);
        apply_xml_attribute(&mut item, "allowpickupable", "true", 100);
        apply_xml_attribute(&mut item, "storeitem", "1", 100);
        apply_xml_attribute(&mut item, "walkstack", "0", 100);
        apply_xml_attribute(&mut item, "containersize", "12", 100);

        assert_eq!(item.moveable_override, Some(false));
        assert_eq!(item.block_projectile_override, Some(true));
        assert!(item.allow_pickupable);
        assert!(item.store_item);
        assert!(!item.walk_stack);
        assert_eq!(item.max_items, 12);
    }

    #[test]
    fn itemtype_defaults_match_cpp_shoot_range_and_max_hit_chance() {
        let it = ItemType::default();
        assert_eq!(it.shoot_range, 1);
        assert_eq!(it.max_hit_chance, -1);
        assert_eq!(it.hit_chance, 0);
        assert_eq!(it.floor_change, 0);
        assert_eq!(it.charges, 0);
    }

    #[test]
    fn showcharges_showattributes_level_vocation_xml_to_itemtype() {
        let mut item = ItemType::default();
        apply_xml_attribute(&mut item, "showcharges", "1", 1);
        apply_xml_attribute(&mut item, "showattributes", "true", 1);
        apply_xml_attribute(&mut item, "levelrequired", "120", 1);
        apply_xml_attribute(&mut item, "magiclevelrequired", "15", 1);
        apply_xml_attribute(&mut item, "vocation", "Sorcerer", 1);
        apply_xml_attribute(&mut item, "vocation", "Druid", 1);

        assert!(item.show_charges);
        assert!(item.show_attributes);
        assert_eq!(item.min_req_level, 120);
        assert_eq!(item.min_req_magic_level, 15);
        assert_eq!(
            item.voc_equip_names,
            vec!["sorcerer".to_string(), "druid".to_string()]
        );
    }

    #[test]
    fn readable_writeable_follow_items_cpp() {
        let mut item = ItemType::default();
        apply_xml_attribute(&mut item, "readable", "true", 1);
        assert_eq!(item.can_read_text_override, Some(true));

        let mut item = ItemType::default();
        apply_xml_attribute(&mut item, "readable", "false", 1);
        assert_eq!(item.can_read_text_override, Some(false));

        let mut item = ItemType::default();
        apply_xml_attribute(&mut item, "writeable", "true", 1);
        assert!(item.can_write_text);
        assert_eq!(item.can_read_text_override, Some(true));

        let mut item = ItemType::default();
        apply_xml_attribute(&mut item, "writeable", "false", 1);
        assert!(!item.can_write_text);
        assert_eq!(item.can_read_text_override, Some(false));
    }

    #[test]
    fn can_read_text_xml_overrides_otb_readable_flag() {
        const FLAG_READABLE: u32 = 1 << 14;
        let mut item = ItemType {
            flags: FLAG_READABLE,
            ..ItemType::default()
        };
        assert!(item.can_read_text());
        apply_xml_attribute(&mut item, "readable", "0", 1);
        assert!(!item.can_read_text());
    }

    #[test]
    fn blocking_and_allowdistread_overrides() {
        const FLAG_BLOCK_SOLID: u32 = 1 << 0;
        const FLAG_ALLOWDISTREAD: u32 = 1 << 20;
        let mut solid = ItemType {
            flags: 0,
            ..ItemType::default()
        };
        assert!(!solid.block_solid());
        apply_xml_attribute(&mut solid, "blocking", "1", 1);
        assert!(solid.block_solid());

        let mut not_solid = ItemType {
            flags: FLAG_BLOCK_SOLID,
            ..ItemType::default()
        };
        assert!(not_solid.block_solid());
        apply_xml_attribute(&mut not_solid, "blocking", "false", 1);
        assert!(!not_solid.block_solid());

        let mut dist = ItemType {
            flags: 0,
            ..ItemType::default()
        };
        assert!(!dist.allow_dist_read());
        apply_xml_attribute(&mut dist, "allowdistread", "yes", 1);
        assert!(dist.allow_dist_read());

        let mut no_dist = ItemType {
            flags: FLAG_ALLOWDISTREAD,
            ..ItemType::default()
        };
        assert!(no_dist.allow_dist_read());
        apply_xml_attribute(&mut no_dist, "allowdistread", "0", 1);
        assert!(!no_dist.allow_dist_read());
    }

    #[test]
    fn hitchance_and_maxhitchance_clamped() {
        let mut item = ItemType::default();
        apply_xml_attribute(&mut item, "hitchance", "200", 1);
        assert_eq!(item.hit_chance, 100);
        apply_xml_attribute(&mut item, "hitchance", "-999", 1);
        assert_eq!(item.hit_chance, -100);

        let mut item = ItemType::default();
        apply_xml_attribute(&mut item, "maxhitchance", "500", 1);
        assert_eq!(item.max_hit_chance, 100);
        apply_xml_attribute(&mut item, "maxhitchance", "-3", 1);
        assert_eq!(item.max_hit_chance, 0);
    }

    #[test]
    fn unknown_xml_key_triggers_warning_path_only_for_unlisted_keys() {
        assert!(is_known_xml_key("speed"));
        assert!(!is_known_xml_key("totally_unknown_key_xyz"));
    }

    /// C++: `Items::parseItemNode` `abilities` — `src/items.cpp` ~860–1158.
    #[test]
    fn item_abilities_typed_from_xml_attributes() {
        let mut item = ItemType::default();
        apply_xml_attribute(&mut item, "speed", "20", 1);
        assert_eq!(item.abilities.speed, 20);
        assert_eq!(item.speed, 0);

        apply_xml_attribute(&mut item, "skillsword", "5", 1);
        assert_eq!(item.abilities.skills[Skill::Sword as usize], 5);

        apply_xml_attribute(&mut item, "absorbpercentenergy", "3", 1);
        apply_xml_attribute(&mut item, "absorbpercentenergy", "2", 1);
        let e = combat_absorb_index(CombatType::Energy);
        assert_eq!(item.abilities.absorb_percent[e], 5);

        apply_xml_attribute(&mut item, "absorbpercentall", "1", 1);
        assert!(item.abilities.absorb_percent.iter().all(|&v| v >= 1));

        apply_xml_attribute(&mut item, "healthgain", "7", 1);
        assert!(item.abilities.regeneration);
        assert_eq!(item.abilities.health_gain, 7);

        apply_xml_attribute(&mut item, "elementfire", "12", 1);
        assert_eq!(item.abilities.element_damage, 12);
        assert_eq!(item.abilities.element_type, Some(CombatType::Fire));

        apply_xml_attribute(&mut item, "suppressdrunk", "1", 1);
        assert!(item.abilities.condition_suppressions & CONDITION_DRUNK != 0);

        apply_xml_attribute(&mut item, "magicpoints", "2", 1);
        assert_eq!(item.abilities.stats[STAT_MAGICPOINTS], 2);
        apply_xml_attribute(&mut item, "magiclevelpoints", "3", 1);
        assert_eq!(item.abilities.stats[STAT_MAGICPOINTS], 3);
    }

    /// C++ duplicate keys: `absorbpercentearth` / `fieldabsorbpercentearth` → same as poison/earth index (`src/items.cpp`).
    #[test]
    fn absorb_earth_keys_alias_earth_combat_type_like_cpp() {
        let i = combat_absorb_index(CombatType::Earth);
        let mut a = ItemType::default();
        apply_xml_attribute(&mut a, "absorbpercentpoison", "4", 1);
        let mut b = ItemType::default();
        apply_xml_attribute(&mut b, "absorbpercentearth", "4", 1);
        assert_eq!(a.abilities.absorb_percent[i], b.abilities.absorb_percent[i]);

        let mut c = ItemType::default();
        apply_xml_attribute(&mut c, "fieldabsorbpercentpoison", "2", 1);
        let mut d = ItemType::default();
        apply_xml_attribute(&mut d, "fieldabsorbpercentearth", "2", 1);
        assert_eq!(
            c.abilities.field_absorb_percent[i],
            d.abilities.field_absorb_percent[i]
        );
    }

    #[test]
    fn slottype_single_token_sets_slot_like_cpp() {
        let mut item = ItemType::default();
        apply_xml_attribute(&mut item, "slottype", "head", 1);
        assert_ne!(item.slot_position & super::SLOTP_HEAD, 0);
    }

    #[test]
    fn slottype_pipe_not_split_unknown_like_cpp() {
        let mut item = ItemType::default();
        let before = item.slot_position;
        apply_xml_attribute(&mut item, "slottype", "head|body", 1);
        assert_eq!(item.slot_position, before);
    }

    #[test]
    fn floorchange_or_bitmask_like_cpp() {
        let mut item = ItemType::default();
        apply_xml_attribute(&mut item, "floorchange", "down", 1);
        apply_xml_attribute(&mut item, "floorchange", "north", 1);
        assert_eq!(item.floor_change, (1 << 0) | (1 << 1));
    }

    #[test]
    fn charges_typed_field_and_charges_default() {
        let mut items = HashMap::new();
        items.insert(
            10,
            ItemType {
                id: 10,
                server_id: 10,
                ..ItemType::default()
            },
        );
        apply_xml_attribute(items.get_mut(&10).unwrap(), "charges", "5", 10);
        let db = ItemDatabase {
            items,
            client_to_server: HashMap::new(),
        };
        assert_eq!(db.items.get(&10).unwrap().charges, 5);
        assert_eq!(db.charges_default(10), 5);
    }

    #[test]
    fn merge_xml_skips_duplicate_id_attributes_keeps_first_block() {
        let mut items = HashMap::new();
        let xml = r#"<items>
  <item fromid="100" toid="102" name="first range">
    <attribute key="weight" value="100" />
  </item>
  <item id="101" name="duplicate override">
    <attribute key="weight" value="999" />
  </item>
</items>"#;
        let path = std::env::temp_dir().join(format!(
            "items-xml-dup-range-{}-{}.xml",
            std::process::id(),
            101u16
        ));
        fs::write(&path, xml).expect("write temp xml");

        ItemDatabase::merge_xml(&mut items, &path).expect("merge xml");
        let _ = fs::remove_file(&path);

        let it = items.get(&101).expect("item 101");
        assert_eq!(it.name, "first range");
        assert_eq!(it.weight, 100);
    }

    #[test]
    fn container_truth_uses_group() {
        let mut items = HashMap::new();
        items.insert(
            100,
            ItemType {
                id: 100,
                server_id: 100,
                group: ItemType::GROUP_CONTAINER,
                ..ItemType::default()
            },
        );
        items.insert(
            101,
            ItemType {
                id: 101,
                server_id: 101,
                ..ItemType::default()
            },
        );

        let db = ItemDatabase {
            items,
            client_to_server: HashMap::new(),
        };
        assert!(db.is_container(100));
        assert!(!db.is_container(101));
    }

    #[test]
    fn nested_attribute_is_stored_with_parent_prefix() {
        let mut item = ItemType::default();
        apply_nested_xml_attribute(&mut item, "field", "ticks", "2000");
        assert_eq!(item.xml_attributes.get("field.ticks"), Some(&"2000".to_string()));
    }

    #[test]
    fn merge_xml_parses_non_empty_nested_attribute_block() {
        let mut items = HashMap::new();
        items.insert(
            200,
            ItemType {
                id: 200,
                server_id: 200,
                ..ItemType::default()
            },
        );

        let xml = r#"<items>
  <item id="200" name="test field">
    <attribute key="moveable" value="0" />
    <attribute key="field" value="fire">
      <attribute key="ticks" value="4000" />
      <attribute key="count" value="3" />
    </attribute>
  </item>
</items>"#;
        let path = std::env::temp_dir().join(format!(
            "items-xml-nested-{}-{}.xml",
            std::process::id(),
            200u16
        ));
        fs::write(&path, xml).expect("write temp xml");

        ItemDatabase::merge_xml(&mut items, &path).expect("merge xml");
        let _ = fs::remove_file(&path);

        let it = items.get(&200).expect("item exists");
        assert_eq!(it.moveable_override, Some(false));
        assert_eq!(it.xml_attributes.get("field"), Some(&"fire".to_string()));
        assert_eq!(it.xml_attributes.get("field.ticks"), Some(&"4000".to_string()));
        assert_eq!(it.xml_attributes.get("field.count"), Some(&"3".to_string()));
    }

    #[test]
    fn merge_xml_applies_attributes_even_if_otb_name_exists() {
        let mut items = HashMap::new();
        items.insert(
            300,
            ItemType {
                id: 300,
                server_id: 300,
                name: "from otb".to_string(),
                ..ItemType::default()
            },
        );

        let xml = r#"<items>
  <item id="300" name="xml name">
    <attribute key="description" value="xml description" />
    <attribute key="weight" value="123" />
  </item>
</items>"#;
        let path = std::env::temp_dir().join(format!(
            "items-xml-otb-name-{}-{}.xml",
            std::process::id(),
            300u16
        ));
        fs::write(&path, xml).expect("write temp xml");

        ItemDatabase::merge_xml(&mut items, &path).expect("merge xml");
        let _ = fs::remove_file(&path);

        let it = items.get(&300).expect("item exists");
        assert_eq!(it.name, "xml name");
        assert_eq!(it.description, "xml description");
        assert_eq!(it.weight, 123);
    }

    /// XML-9 / audit: overlapping `fromid`/`toid` ranges — first block wins for ids already defined.
    #[test]
    fn merge_xml_overlapping_ranges_first_wins_for_shared_ids() {
        let mut items = HashMap::new();
        let xml = r#"<items>
  <item fromid="100" toid="105" name="first">
    <attribute key="weight" value="100" />
  </item>
  <item fromid="102" toid="110" name="second">
    <attribute key="weight" value="999" />
  </item>
</items>"#;
        let path = std::env::temp_dir().join(format!(
            "items-xml-overlap-{}-{}.xml",
            std::process::id(),
            1u8
        ));
        fs::write(&path, xml).expect("write temp xml");

        ItemDatabase::merge_xml(&mut items, &path).expect("merge xml");
        let _ = fs::remove_file(&path);

        for id in 100..=101 {
            assert_eq!(items.get(&id).map(|i| i.weight), Some(100), "id {id}");
        }
        for id in 102..=105 {
            assert_eq!(items.get(&id).map(|i| i.weight), Some(100), "id {id} first range");
        }
        for id in 106..=110 {
            assert_eq!(items.get(&id).map(|i| i.weight), Some(999), "id {id} second only");
        }
    }

    /// OTB-2: `client_id == 0` must not appear in `clientToServer` reverse map.
    #[test]
    fn client_id_zero_omitted_from_client_to_server_map() {
        let mut items = HashMap::new();
        items.insert(
            1,
            ItemType {
                id: 1,
                server_id: 1,
                client_id: 0,
                ..ItemType::default()
            },
        );
        items.insert(
            2,
            ItemType {
                id: 2,
                server_id: 2,
                client_id: 4242,
                ..ItemType::default()
            },
        );
        let m = ItemDatabase::build_client_to_server_map(&items);
        assert!(!m.contains_key(&0));
        assert_eq!(m.get(&4242), Some(&2));
    }

    /// C++: `Items::buildInventoryList` — `src/items.cpp` (lines 511–530).
    #[test]
    fn inventory_client_ids_matches_cpp_predicate() {
        let i = ItemType {
            id: 1,
            server_id: 1,
            client_id: 5000,
            attack: 10,
            ..Default::default()
        };
        let j = ItemType {
            id: 2,
            server_id: 2,
            client_id: 6000,
            ..Default::default()
        };
        let k = ItemType {
            id: 3,
            server_id: 3,
            client_id: 7000,
            slot_position: super::SLOTP_HEAD,
            ..Default::default()
        };
        let mut items = HashMap::new();
        items.insert(1, i);
        items.insert(2, j);
        items.insert(3, k);
        let db = ItemDatabase {
            items,
            client_to_server: HashMap::new(),
        };
        let v = db.inventory_client_ids();
        assert_eq!(v, vec![5000, 7000]);
    }
}
