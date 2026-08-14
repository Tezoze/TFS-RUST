#[test]
fn firefield_xml_nested_attrs() {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/items"));
    let db = tfs_rust_content::items::ItemDatabase::load(
        &root.join("items.otb"),
        &root.join("items.xml"),
    )
    .expect("load");
    let it = db.items.get(&1487).expect("1487");
    assert!(it.is_magic_field());
    assert!(!it.is_cip_priority_bottom());
    assert_eq!(
        it.xml_attributes.get("field").map(String::as_str),
        Some("fire")
    );
    assert_eq!(
        it.xml_attributes
            .get("field.initdamage")
            .map(String::as_str),
        Some("20")
    );
    assert_eq!(
        it.xml_attributes.get("field.cycles").map(String::as_str),
        Some("70")
    );
}
