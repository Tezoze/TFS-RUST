//! Regression tests for inventory / container parity gaps (Phase 1–2 audit).

use slotmap::SlotMap;
use tfs_rust_common::enums::ZoneType;
use tfs_rust_common::Position;
use tfs_rust_core::house::HouseManager;
use tfs_rust_core::ids::ItemId;
use tfs_rust_core::map::Map;

#[test]
fn house_manager_is_invited_owner() {
    let mut h = HouseManager::default();
    h.set_owner(42, 1000);
    assert!(h.is_invited(42, 1000));
    assert!(!h.is_invited(42, 999));
}

#[test]
fn house_manager_guest_and_subowner() {
    let mut h = HouseManager::default();
    h.set_owner(1, 10);
    h.houses.entry(1).or_default().guests.insert(20);
    h.houses.entry(1).or_default().subowners.insert(30);
    assert!(h.is_invited(1, 10));
    assert!(h.is_invited(1, 20));
    assert!(h.is_invited(1, 30));
    assert!(!h.is_invited(1, 40));
}

#[test]
fn unknown_house_id_is_not_restricted() {
    let h = HouseManager::default();
    assert!(h.is_invited(9999, 1));
}

#[test]
fn map_find_item_position_finds_down_item() {
    let mut items: SlotMap<ItemId, u8> = SlotMap::with_key();
    let iid = items.insert(0);
    let mut tiles = std::collections::HashMap::new();
    let pos = Position::new(10, 20, 7);
    let body = tfs_rust_core::tile::TileBody {
        position: pos,
        ground: Some(100),
        down_items: vec![iid],
        top_items: Vec::new(),
        creatures: Vec::new(),
        flags: 0,
        zone: ZoneType::Normal,
    };
    tiles.insert(pos, tfs_rust_core::tile::Tile::Normal(body));
    let m = Map {
        width: 100,
        height: 100,
        tiles,
        qtrees: std::collections::HashMap::new(),
        towns: std::collections::HashMap::new(),
        waypoints: std::collections::HashMap::new(),
    };
    assert_eq!(m.find_item_position(iid), Some(pos));
}
