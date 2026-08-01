//! Deserialize TFS `Item::unserializeAttr` blobs (DB) and OTBM item node props.
//!
//! C++ reference: `src/item.cpp` `Item::readAttr` / `unserializeAttr` / `serializeAttr`;
//! OTBM Remere extensions: RME `iomap_otbm.h` `OTBM_ATTR_KEYNUMBER`…`CHESTQUESTNUMBER`
//! (bytes 23–28 collide with TFS `ATTR_CONTAINER_ITEMS`…`ATTR_WEIGHT` — map-only).

use tfs_rust_common::{PropStream, PropWriteStream, Result as TfsResult};
use tfs_rust_content::items::ItemDatabase;

use crate::item::Item;
use crate::item_attributes::{
    AttrType, CustomAttrValue, DecayState, ItemAttrFlags, ItemAttributes,
};

/// Where the attr blob came from — Remere map attrs 23–28 vs TFS DB attrs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemBlobSource {
    /// `player_items.attributes` / depot — TFS `AttrTypes_t` (NAME=24, …).
    Database,
    /// OTBM `OTBM_ITEM` props after the `u16` id — Remere key/door attrs at 23–28.
    OtbmMap,
}

/// Remere OTBM item attrs (`iomap_otbm.h`) — u16 payloads; not TFS `AttrTypes_t`.
const OTBM_ATTR_KEYNUMBER: u8 = 23;
const OTBM_ATTR_KEYHOLENUMBER: u8 = 24;
const OTBM_ATTR_DOORQUESTNUMBER: u8 = 25;
const OTBM_ATTR_DOORQUESTVALUE: u8 = 26;
const OTBM_ATTR_DOORLEVEL: u8 = 27;
const OTBM_ATTR_CHESTQUESTNUMBER: u8 = 28;

/// Custom-attr keys for Remere OTBM door/key fields (Lua `ITEM_ATTRIBUTE_*` string aliases).
pub const OTBM_CUSTOM_KEYNUMBER: &str = tfs_rust_common::remere_attr::KEYNUMBER;
pub const OTBM_CUSTOM_KEYHOLENUMBER: &str = tfs_rust_common::remere_attr::KEYHOLENUMBER;
pub const OTBM_CUSTOM_DOORQUESTNUMBER: &str = tfs_rust_common::remere_attr::DOORQUESTNUMBER;
pub const OTBM_CUSTOM_DOORQUESTVALUE: &str = tfs_rust_common::remere_attr::DOORQUESTVALUE;
pub const OTBM_CUSTOM_DOORLEVEL: &str = tfs_rust_common::remere_attr::DOORLEVEL;
pub const OTBM_CUSTOM_CHESTQUESTNUMBER: &str = tfs_rust_common::remere_attr::CHESTQUESTNUMBER;

/// Result of parsing a persisted item attribute blob.
pub struct ParsedItemBlob {
    pub attrs: ItemAttributes,
    /// `ATTR_COUNT` / `ATTR_RUNE_CHARGES` — overrides `Item::count` like C++ `setSubType`.
    pub subtype_override: Option<u8>,
}

/// Parse `attributes` BLOB from `player_items` / depot tables.
/// `is_container`: true when `ItemType` is a container — `ATTR_CONTAINER_ITEMS` is valid.
pub fn parse_item_blob(blob: &[u8], is_container: bool) -> TfsResult<ParsedItemBlob> {
    parse_item_blob_from(blob, is_container, ItemBlobSource::Database)
}

/// Parse OTBM `OTBM_ITEM` attribute bytes (after item id).
/// Remere attrs 23–28 are key/door u16s, not TFS NAME/WEIGHT strings.
pub fn parse_otbm_item_blob(blob: &[u8], is_container: bool) -> TfsResult<ParsedItemBlob> {
    parse_item_blob_from(blob, is_container, ItemBlobSource::OtbmMap)
}

fn parse_item_blob_from(
    blob: &[u8],
    is_container: bool,
    source: ItemBlobSource,
) -> TfsResult<ParsedItemBlob> {
    let mut attrs = ItemAttributes::new();
    let mut subtype_override: Option<u8> = None;
    let mut stream = PropStream::new(blob);
    while let Ok(attr_type) = stream.read_u8() {
        if attr_type == 0 {
            break;
        }
        let cont = read_one_attr(
            attr_type,
            &mut stream,
            &mut attrs,
            &mut subtype_override,
            is_container,
            source,
        )?;
        if !cont {
            break;
        }
    }
    Ok(ParsedItemBlob {
        attrs,
        subtype_override,
    })
}

fn read_otbm_reme_u16(
    attr_type: u8,
    stream: &mut PropStream<'_>,
    attrs: &mut ItemAttributes,
) -> TfsResult<bool> {
    let key = match attr_type {
        OTBM_ATTR_KEYNUMBER => OTBM_CUSTOM_KEYNUMBER,
        OTBM_ATTR_KEYHOLENUMBER => OTBM_CUSTOM_KEYHOLENUMBER,
        OTBM_ATTR_DOORQUESTNUMBER => OTBM_CUSTOM_DOORQUESTNUMBER,
        OTBM_ATTR_DOORQUESTVALUE => OTBM_CUSTOM_DOORQUESTVALUE,
        OTBM_ATTR_DOORLEVEL => OTBM_CUSTOM_DOORLEVEL,
        OTBM_ATTR_CHESTQUESTNUMBER => OTBM_CUSTOM_CHESTQUESTNUMBER,
        _ => return Ok(false),
    };
    let v = stream.read_u16()? as i64;
    attrs.set_custom_attribute(key, CustomAttrValue::Integer(v));
    Ok(true)
}

fn read_one_attr(
    attr_type: u8,
    stream: &mut PropStream<'_>,
    attrs: &mut ItemAttributes,
    subtype_override: &mut Option<u8>,
    is_container: bool,
    source: ItemBlobSource,
) -> TfsResult<bool> {
    if source == ItemBlobSource::OtbmMap
        && read_otbm_reme_u16(attr_type, stream, attrs)?
    {
        return Ok(true);
    }

    match attr_type {
        x if x == AttrType::Count as u8 || x == AttrType::RuneCharges as u8 => {
            *subtype_override = Some(stream.read_u8()?);
        }
        x if x == AttrType::ActionId as u8 => {
            attrs.set_action_id(stream.read_u16()?);
        }
        x if x == AttrType::UniqueId as u8 => {
            attrs.set_unique_id(stream.read_u16()?);
        }
        x if x == AttrType::Text as u8 => {
            attrs.set_text(stream.read_string()?);
        }
        x if x == AttrType::Description as u8 => {
            attrs.set_description(stream.read_string()?);
        }
        x if x == AttrType::WrittenDate as u8 => {
            attrs.set_date(stream.read_u32()? as i64);
        }
        x if x == AttrType::WrittenBy as u8 => {
            attrs.set_writer(stream.read_string()?);
        }
        x if x == AttrType::Charges as u8 => {
            attrs.set_charges(stream.read_u16()?);
        }
        x if x == AttrType::Duration as u8 => {
            attrs.set_duration(stream.read_i32()?);
        }
        x if x == AttrType::DecayingState as u8 => {
            let st = stream.read_u8()?;
            if st != 0 {
                attrs.set_decaying(DecayState::Pending);
            }
        }
        x if x == AttrType::Name as u8 => {
            attrs.set_name(stream.read_string()?);
        }
        x if x == AttrType::Article as u8 => {
            attrs.set_article(stream.read_string()?);
        }
        x if x == AttrType::PluralName as u8 => {
            attrs.set_plural_name(stream.read_string()?);
        }
        x if x == AttrType::Weight as u8 => {
            attrs.set_weight_attr(stream.read_u32()?);
        }
        x if x == AttrType::Attack as u8 => {
            attrs.set_attack(stream.read_i32()?);
        }
        x if x == AttrType::AttackSpeed as u8 => {
            attrs.set_attack_speed(stream.read_u32()?);
        }
        x if x == AttrType::ContainerSize as u8 => {
            attrs.set_container_size(stream.read_u32()? as u8);
        }
        x if x == AttrType::Defense as u8 => {
            attrs.set_defense(stream.read_i32()?);
        }
        x if x == AttrType::ExtraDefense as u8 => {
            attrs.set_extra_defense(stream.read_i32()?);
        }
        x if x == AttrType::Armor as u8 => {
            attrs.set_armor(stream.read_i32()?);
        }
        x if x == AttrType::HitChance as u8 => {
            attrs.set_hit_chance(stream.read_i8()? as i32);
        }
        x if x == AttrType::ShootRange as u8 => {
            attrs.set_shoot_range(stream.read_u8()? as i32);
        }
        x if x == AttrType::DecayTo as u8 => {
            attrs.set_decay_to(stream.read_i32()? as u32);
        }
        x if x == AttrType::WrapId as u8 => {
            attrs.set_wrap_id(stream.read_u16()? as u32);
        }
        x if x == AttrType::StoreItem as u8 => {
            attrs.set_store_item(stream.read_u8()? as u32);
        }
        x if x == AttrType::OpenContainer as u8 => {
            attrs.set_auto_open(stream.read_u8()?);
        }
        x if x == AttrType::Tier as u8 => {
            let _ = stream.read_u8()?;
        }
        x if x == AttrType::PodiumOutfit as u8 => {
            for _ in 0..15 {
                stream.read_u8()?;
            }
        }
        x if x == AttrType::DepotId as u8 => {
            attrs.set_depot_id(stream.read_u16()?);
        }
        x if x == AttrType::HouseDoorId as u8 => {
            // C++ `Door::readAttr` `ATTR_HOUSEDOORID` — `house.cpp`.
            attrs.set_door_id(stream.read_u8()?);
        }
        x if x == AttrType::SleeperGuid as u8 => {
            let _ = stream.read_u32()?;
        }
        x if x == AttrType::SleepStart as u8 => {
            let _ = stream.read_u32()?;
        }
        x if x == AttrType::TeleDest as u8 => {
            for _ in 0..5 {
                stream.read_u8()?;
            }
        }
        x if x == AttrType::ContainerItems as u8 => {
            // DB only — OTBM Remere KEYNUMBER (23) handled above.
            let _n = stream.read_u32()?;
            if !is_container {
                return Err(tfs_rust_common::error::TfsRustError::PropStream(
                    "ATTR_CONTAINER_ITEMS on non-container".into(),
                ));
            }
            return Ok(false);
        }
        x if x == AttrType::CustomAttributes as u8 => {
            let n = stream.read_u64()?;
            for _ in 0..n {
                let key = stream.read_string()?;
                let pos = stream.read_u8()?;
                let val = match pos {
                    1 => CustomAttrValue::String(stream.read_string()?),
                    2 => CustomAttrValue::Integer(stream.read_i64()?),
                    3 => CustomAttrValue::Float(stream.read_f64()?),
                    4 => CustomAttrValue::Boolean(stream.read_bool_byte()?),
                    _ => {
                        return Err(tfs_rust_common::error::TfsRustError::PropStream(
                            "unknown custom attribute variant".into(),
                        ));
                    }
                };
                attrs.set_custom_attribute(key, val);
            }
        }
        x if x == AttrType::TileFlags as u8 => {
            let _ = stream.read_u32()?;
        }
        x if x == AttrType::Item as u8 => {
            let _ = stream.read_u16()?;
        }
        _ => {
            return Err(tfs_rust_common::error::TfsRustError::PropStream(format!(
                "unknown item attr type {attr_type}"
            )));
        }
    }
    Ok(true)
}

fn write_custom_value(w: &mut PropWriteStream, v: &CustomAttrValue) {
    match v {
        CustomAttrValue::None => {}
        CustomAttrValue::String(s) => {
            w.write_u8(1);
            w.write_string(s);
        }
        CustomAttrValue::Integer(n) => {
            w.write_u8(2);
            w.write_i64(*n);
        }
        CustomAttrValue::Float(x) => {
            w.write_u8(3);
            w.write_f64(*x);
        }
        CustomAttrValue::Boolean(b) => {
            w.write_u8(4);
            w.write_u8(u8::from(*b));
        }
    }
}

/// C++ `Item::serializeAttr` — binary compatible with TFS 1.4.2 `PropWriteStream` output.
pub fn write_item_blob(item: &Item, items_db: &ItemDatabase) -> Vec<u8> {
    write_item_blob_with_duration(item, items_db, None)
}

/// Like [`write_item_blob`], but `duration_override_ms` replaces raw Duration when set
/// (player save: live remaining from `DecayManager`, not schedule-time snapshot).
pub fn write_item_blob_with_duration(
    item: &Item,
    items_db: &ItemDatabase,
    duration_override_ms: Option<i32>,
) -> Vec<u8> {
    let mut w = PropWriteStream::new();
    let it = items_db.items.get(&item.item_type);

    let stackable_or_fluid =
        it.is_some_and(|t| t.stackable() || t.is_fluid_container() || t.is_splash());
    if stackable_or_fluid {
        w.write_u8(AttrType::Count as u8);
        w.write_u8(item.client_count());
    }

    let attrs = match item.attributes.as_deref() {
        Some(a) => a,
        None => return w.finish(),
    };

    let charges = attrs.get_charges();
    if charges != 0 {
        w.write_u8(AttrType::Charges as u8);
        w.write_u16(charges);
    }

    let moveable = it.is_none_or(|t| t.moveable());
    if moveable {
        let action_id = attrs.get_action_id();
        if action_id != 0 {
            w.write_u8(AttrType::ActionId as u8);
            w.write_u16(action_id);
        }
    }
    if attrs.has_unique_id() {
        let uid = attrs.get_unique_id();
        if uid != 0 {
            w.write_u8(AttrType::UniqueId as u8);
            w.write_u16(uid);
        }
    }

    if attrs.has_depot_id() {
        w.write_u8(AttrType::DepotId as u8);
        w.write_u16(attrs.get_depot_id());
    }

    // Persist door id for house access lists (`listid` = door id). TFS Door::serializeAttr
    // is empty, but map/DB hydrate needs ATTR_HOUSEDOORID (`house.cpp` Door::readAttr).
    if ItemAttrFlags::from_bits_truncate(attrs.attribute_bits()).contains(ItemAttrFlags::DOOR_ID) {
        w.write_u8(AttrType::HouseDoorId as u8);
        w.write_u8(attrs.get_door_id());
    }

    let text = attrs.get_text();
    if !text.is_empty() {
        w.write_u8(AttrType::Text as u8);
        w.write_string(text);
    }

    let date = attrs.get_date();
    if date != 0 {
        w.write_u8(AttrType::WrittenDate as u8);
        w.write_u32(date as u32);
    }

    let writer = attrs.get_writer();
    if !writer.is_empty() {
        w.write_u8(AttrType::WrittenBy as u8);
        w.write_string(writer);
    }

    let desc = attrs.get_description();
    if !desc.is_empty() {
        w.write_u8(AttrType::Description as u8);
        w.write_string(desc);
    }

    let duration_to_write = duration_override_ms.or_else(|| {
        if attrs.has_duration() {
            Some(attrs.get_duration_raw())
        } else {
            None
        }
    });
    if let Some(dur) = duration_to_write {
        w.write_u8(AttrType::Duration as u8);
        w.write_i32(dur);
    }

    let decay = attrs.get_decaying();
    if decay == DecayState::True || decay == DecayState::Pending {
        w.write_u8(AttrType::DecayingState as u8);
        w.write_u8(decay as u8);
    }

    if let Some(s) = attrs.get_name_str() {
        w.write_u8(AttrType::Name as u8);
        w.write_string(s);
    }
    if let Some(s) = attrs.get_article_str() {
        w.write_u8(AttrType::Article as u8);
        w.write_string(s);
    }
    if let Some(s) = attrs.get_plural_name_str() {
        w.write_u8(AttrType::PluralName as u8);
        w.write_string(s);
    }
    if let Some(weight) = attrs.weight_serial() {
        w.write_u8(AttrType::Weight as u8);
        w.write_u32(weight);
    }
    if let Some(atk) = attrs.get_attack() {
        w.write_u8(AttrType::Attack as u8);
        w.write_i32(atk);
    }
    let bits = ItemAttrFlags::from_bits_truncate(attrs.attribute_bits());
    if bits.contains(ItemAttrFlags::ATTACK_SPEED) {
        w.write_u8(AttrType::AttackSpeed as u8);
        w.write_u32(attrs.get_attack_speed());
    }
    if bits.contains(ItemAttrFlags::CONTAINER_SIZE) {
        w.write_u8(AttrType::ContainerSize as u8);
        w.write_u32(u32::from(attrs.container_size_serial()));
    }
    if let Some(def) = attrs.get_defense() {
        w.write_u8(AttrType::Defense as u8);
        w.write_i32(def);
    }
    if let Some(ed) = attrs.get_extra_defense() {
        w.write_u8(AttrType::ExtraDefense as u8);
        w.write_i32(ed);
    }
    if let Some(ar) = attrs.get_armor() {
        w.write_u8(AttrType::Armor as u8);
        w.write_i32(ar);
    }
    if let Some(hc) = attrs.get_hit_chance_attr() {
        w.write_u8(AttrType::HitChance as u8);
        w.write_i8(hc as i8);
    }
    if let Some(sr) = attrs.get_shoot_range_attr() {
        w.write_u8(AttrType::ShootRange as u8);
        w.write_u8(sr as u8);
    }
    if bits.contains(ItemAttrFlags::DECAY_TO) {
        w.write_u8(AttrType::DecayTo as u8);
        w.write_i32(attrs.get_decay_to() as i32);
    }
    if bits.contains(ItemAttrFlags::WRAP_ID) {
        w.write_u8(AttrType::WrapId as u8);
        w.write_u16(attrs.get_wrap_id() as u16);
    }
    if attrs.is_store_item() {
        w.write_u8(AttrType::StoreItem as u8);
        w.write_u8(attrs.store_item_serial_byte());
    }
    if attrs.has_auto_open() {
        w.write_u8(AttrType::OpenContainer as u8);
        w.write_i8(attrs.get_auto_open() as i8);
    }

    if let Some(map) = attrs.custom_attributes() {
        w.write_u8(AttrType::CustomAttributes as u8);
        w.write_u64(map.len() as u64);
        for (k, v) in map {
            w.write_string(k);
            write_custom_value(&mut w, v);
        }
    }

    w.finish()
}

#[cfg(test)]
mod write_roundtrip_tests {
    use super::{
        parse_item_blob, parse_otbm_item_blob, write_item_blob, OTBM_CUSTOM_DOORLEVEL,
        OTBM_CUSTOM_DOORQUESTNUMBER, OTBM_CUSTOM_DOORQUESTVALUE, OTBM_CUSTOM_KEYHOLENUMBER,
        OTBM_CUSTOM_KEYNUMBER,
    };
    use crate::item::Item;
    use crate::item_attributes::{AttrType, CustomAttrValue};
    use std::collections::HashMap;
    use tfs_rust_content::items::ItemDatabase;
    use tfs_rust_content::otb::ItemType;

    #[test]
    fn minimal_item_roundtrips_through_parse() {
        let db = ItemDatabase {
            items: HashMap::new(),
            client_to_server: HashMap::new(),
        };
        let item = Item::new_single(99);
        let blob = write_item_blob(&item, &db);
        let parsed = parse_item_blob(&blob, false).expect("parse");
        assert_eq!(
            parsed.attrs.attribute_bits(),
            item.attributes
                .as_deref()
                .map(|a| a.attribute_bits())
                .unwrap_or(0)
        );
    }

    #[test]
    fn stackable_writes_count_attr_and_parse_sets_subtype() {
        let mut it = ItemType {
            server_id: 1000,
            ..Default::default()
        };
        it.flags |= 1 << 7; // OTB `FLAG_STACKABLE` (`otb.rs`)
        let db = ItemDatabase {
            items: HashMap::from([(1000, it)]),
            client_to_server: HashMap::new(),
        };
        let item = Item::new(1000, 42);
        let blob = write_item_blob(&item, &db);
        let parsed = parse_item_blob(&blob, false).expect("parse");
        assert_eq!(parsed.subtype_override, Some(42));
    }

    #[test]
    fn otbm_reme_keyhole_is_u16_not_attr_name_string() {
        // forgotten.otbm door 1212 @ 32619,32241,z8: attr 0x18 + u16 0x0ce5
        let blob = [0x18, 0xe5, 0x0c];
        let parsed = parse_otbm_item_blob(&blob, false).expect("otbm parse");
        assert_eq!(
            parsed.attrs.get_custom_attribute(OTBM_CUSTOM_KEYHOLENUMBER),
            Some(&CustomAttrValue::Integer(0x0ce5))
        );
        // DB path would treat 0x18 as ATTR_NAME and EOF on string length 3301
        assert!(parse_item_blob(&blob, false).is_err());
    }

    #[test]
    fn otbm_reme_key_number_is_u16_not_container_items() {
        // key 2088: attr 0x17 + u16 0x0bbe
        let blob = [0x17, 0xbe, 0x0b];
        let parsed = parse_otbm_item_blob(&blob, false).expect("otbm parse");
        assert_eq!(
            parsed.attrs.get_custom_attribute(OTBM_CUSTOM_KEYNUMBER),
            Some(&CustomAttrValue::Integer(0x0bbe))
        );
    }

    #[test]
    fn otbm_reme_quest_and_level_doors() {
        // 1223: DOORQUESTNUMBER=320, DOORQUESTVALUE=5
        let blob = [0x19, 0x40, 0x01, 0x1a, 0x05, 0x00];
        let parsed = parse_otbm_item_blob(&blob, false).expect("otbm parse");
        assert_eq!(
            parsed.attrs.get_custom_attribute(OTBM_CUSTOM_DOORQUESTNUMBER),
            Some(&CustomAttrValue::Integer(320))
        );
        assert_eq!(
            parsed.attrs.get_custom_attribute(OTBM_CUSTOM_DOORQUESTVALUE),
            Some(&CustomAttrValue::Integer(5))
        );

        // 1229: DOORLEVEL=30
        let level = parse_otbm_item_blob(&[0x1b, 0x1e, 0x00], false).expect("level");
        assert_eq!(
            level.attrs.get_custom_attribute(OTBM_CUSTOM_DOORLEVEL),
            Some(&CustomAttrValue::Integer(30))
        );
    }

    #[test]
    fn house_door_id_attr_sets_door_id() {
        // ATTR_HOUSEDOORID (14) + door id 3
        let blob = [AttrType::HouseDoorId as u8, 3];
        let parsed = parse_item_blob(&blob, false).expect("door id");
        assert_eq!(parsed.attrs.get_door_id(), 3);
    }

    #[test]
    fn house_door_id_roundtrips_write() {
        let db = ItemDatabase {
            items: HashMap::new(),
            client_to_server: HashMap::new(),
        };
        let mut item = Item::new_single(1219);
        let mut attrs = crate::item_attributes::ItemAttributes::new();
        attrs.set_door_id(7);
        item.attributes = Some(Box::new(attrs));
        let blob = write_item_blob(&item, &db);
        let parsed = parse_item_blob(&blob, false).expect("parse");
        assert_eq!(parsed.attrs.get_door_id(), 7);
    }

    #[test]
    fn latin1_text_attr_accepted() {
        // ATTR_TEXT + "à" as Latin-1 0xE0 (invalid as UTF-8)
        let blob = [6u8, 1, 0, 0xe0];
        let parsed = parse_item_blob(&blob, false).expect("latin1");
        assert_eq!(parsed.attrs.get_text(), "à");
    }
}
