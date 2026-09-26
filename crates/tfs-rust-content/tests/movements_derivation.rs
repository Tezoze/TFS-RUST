//! Phase 6.1 / 8.8b: frozen `movements.xml` bindings vs item-data derivation.
//!
//! The XML file is deleted. These sets are the last dump of its `function=`
//! entries. Native field DoT uses `ItemType::is_magic_field`; equip abilities
//! use `ItemAbilities` / `transformequipto` — not a Lua id list.

use std::collections::BTreeSet;
use std::path::Path;

use tfs_rust_content::item_abilities::ItemAbilities;
use tfs_rust_content::items::ItemDatabase;

/// Frozen StepIn/AddItem ids from the last `data/movements/movements.xml`.
const XML_FIELD_IDS: &[u16] = &[
    1998, 1999, 2000, 2118, 2119, 2120, 2121, 2122, 2123, 2124, 2125, 2126, 2127, 2131, 2132, 2133,
    2134, 2135, 2136,
];

/// Magic-field ids in item data that XML omitted (lossy copy). Native coverage kept.
const EXTRA_NATIVE_FIELD_IDS: &[u16] = &[2137, 2138, 2140];

/// Frozen Equip ids from the last `movements.xml`.
/// Plain armor with no ability or transform is omitted: dwarven helmet 2502,
/// dwarven armor 2503, dwarven legs 2504, wood cape 2664.
const XML_EQUIP_IDS: &[u16] = &[
    3045, 3048, 3049, 3050, 3051, 3052, 3053, 3054, 3056, 3057, 3079, 3081, 3082, 3083, 3084, 3085,
    3086, 3087, 3088, 3089, 3090, 3091, 3092, 3093, 3094, 3095, 3096, 3097, 3098, 3099, 3100, 3549,
];

/// Amulet of loss: XML Equip/DeEquip stub, no abilities / no transform. Native no-op.
const XML_ONLY_EQUIP_AOL: u16 = 3057;

fn load_pack() -> ItemDatabase {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/items");
    ItemDatabase::load(&root.join("items.otb"), &root.join("items.xml")).expect("load items")
}

fn derived_field_ids(db: &ItemDatabase) -> BTreeSet<u16> {
    db.items
        .iter()
        .filter(|(_, it)| it.is_magic_field())
        .map(|(id, _)| *id)
        .collect()
}

fn derived_equip_ids(db: &ItemDatabase) -> BTreeSet<u16> {
    db.items
        .iter()
        .filter(|(_, it)| {
            it.abilities != ItemAbilities::default()
                || it.xml_attributes.contains_key("transformequipto")
                || it.xml_attributes.contains_key("transformdeequipto")
        })
        .map(|(id, _)| *id)
        .collect()
}

#[test]
fn native_magic_fields_match_xml_plus_documented_extras() {
    let db = load_pack();
    let derived = derived_field_ids(&db);
    let xml: BTreeSet<u16> = XML_FIELD_IDS.iter().copied().collect();
    let extra: BTreeSet<u16> = EXTRA_NATIVE_FIELD_IDS.iter().copied().collect();

    let xml_not_native: Vec<u16> = xml.difference(&derived).copied().collect();
    assert!(
        xml_not_native.is_empty(),
        "XML field ids missing is_magic_field(): {xml_not_native:?}"
    );

    let native_not_xml: BTreeSet<u16> = derived.difference(&xml).copied().collect();
    assert_eq!(
        native_not_xml, extra,
        "native field extras must be exactly 1506–1508 searing/ashes; got {native_not_xml:?}"
    );
}

#[test]
fn native_equip_candidates_match_xml_minus_aol() {
    let db = load_pack();
    let derived = derived_equip_ids(&db);
    let mut xml: BTreeSet<u16> = XML_EQUIP_IDS.iter().copied().collect();
    xml.remove(&XML_ONLY_EQUIP_AOL);

    let xml_not_derived: Vec<u16> = xml.difference(&derived).copied().collect();
    assert!(
        xml_not_derived.is_empty(),
        "XML equip ids without abilities/transform: {xml_not_derived:?}"
    );

    let derived_not_xml: Vec<u16> = derived.difference(&xml).copied().collect();
    assert!(
        derived_not_xml.is_empty(),
        "item-data equip candidates missing from frozen XML: {derived_not_xml:?}"
    );

    let aol = db.items.get(&XML_ONLY_EQUIP_AOL).expect("2173");
    assert_eq!(aol.abilities, ItemAbilities::default());
    assert!(!aol.xml_attributes.contains_key("transformequipto"));
    assert!(!aol.xml_attributes.contains_key("transformdeequipto"));
}
