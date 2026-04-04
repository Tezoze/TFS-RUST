use tfs_rust_core::WildcardTree;

#[test]
fn prefix_round_trip() {
    let mut t = WildcardTree::default();
    t.insert("gm alice");
    t.insert("gm bob");
    let v = t.get_by_prefix("gm ");
    assert!(v.iter().any(|s| s == "gm alice"));
    assert!(v.iter().any(|s| s == "gm bob"));
    assert!(t.remove("gm alice"));
    let v2 = t.get_by_prefix("gm ");
    assert!(!v2.iter().any(|s| s == "gm alice"));
}
