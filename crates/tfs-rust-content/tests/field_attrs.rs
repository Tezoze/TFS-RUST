#[test]
fn firefield_xml_nested_attrs() {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/items"));
    let db =
        tfs_rust_content::items::ItemDatabase::load_ron(&root.join("772/items.ron")).expect("load");
    let it = db.items.get(&2118).expect("2118");
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

    let peaceful = db.items.get(&2131).expect("2131");
    assert_eq!(
        peaceful
            .xml_attributes
            .get("field.skippeaceful")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        peaceful.xml_attributes.get("field").map(String::as_str),
        Some("fire")
    );
    assert_eq!(
        peaceful
            .xml_attributes
            .get("field.initdamage")
            .map(String::as_str),
        Some("20")
    );

    for id in [2137_u16, 2138] {
        let searing = db.items.get(&id).unwrap_or_else(|| panic!("{id}"));
        assert_eq!(
            searing.xml_attributes.get("field").map(String::as_str),
            Some("fire")
        );
        assert_eq!(
            searing
                .xml_attributes
                .get("field.initdamage")
                .map(String::as_str),
            Some("300")
        );
        assert_eq!(
            searing
                .xml_attributes
                .get("field.cycles")
                .map(String::as_str),
            Some("10")
        );
    }
}
