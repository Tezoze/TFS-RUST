use tfs_rust_common::Position;
use tfs_rust_common::ZoneType;
use tfs_rust_core::map::{walk_grid_line, Map, SparseGrid};
use tfs_rust_core::tile::{flags, Tile, TileBody};

fn body_at(x: u16, y: u16, flags: u32) -> Tile {
    let _ = (x, y);
    Tile::Normal(TileBody {
        ground: Some(100),
        down_items: vec![],
        top_items: vec![],
        creatures: vec![],
        flags,
        zone: ZoneType::Normal,
    })
}

fn map_with_wall() -> Map {
    let mut map = Map {
        width: 8,
        height: 8,
        grid: SparseGrid::new(),
        towns: std::collections::HashMap::new(),
        waypoints: std::collections::HashMap::new(),
    };
    map.insert_tile(Position::new(0, 0, 7), body_at(0, 0, 0));
    map.insert_tile(Position::new(1, 0, 7), body_at(1, 0, 0));
    map.insert_tile(
        Position::new(2, 0, 7),
        body_at(2, 0, flags::BLOCK_SOLID | flags::BLOCK_PROJECTILE),
    );
    map.insert_tile(Position::new(3, 0, 7), body_at(3, 0, 0));
    map
}

#[test]
fn sight_blocked_by_tile() {
    let m = map_with_wall();
    let a = Position::new(0, 0, 7);
    let b = Position::new(3, 0, 7);
    assert!(!m.is_sight_clear(a, b));
}

#[test]
fn los_symmetric_when_clear() {
    let mut map = Map {
        width: 4,
        height: 4,
        grid: SparseGrid::new(),
        towns: std::collections::HashMap::new(),
        waypoints: std::collections::HashMap::new(),
    };
    for x in 0..4u16 {
        for y in 0..4u16 {
            let pos = Position::new(x, y, 7);
            map.insert_tile(
                pos,
                Tile::Normal(TileBody {
                    ground: Some(1),
                    down_items: vec![],
                    top_items: vec![],
                    creatures: vec![],
                    flags: 0,
                    zone: ZoneType::Normal,
                }),
            );
        }
    }
    let a = Position::new(0, 0, 7);
    let b = Position::new(3, 3, 7);
    assert_eq!(map.is_sight_clear(a, b), map.is_sight_clear(b, a));
}

#[test]
fn grid_line_includes_endpoints() {
    let a = Position::new(0, 0, 7);
    let b = Position::new(2, 0, 7);
    let w = walk_grid_line(a, b);
    assert!(w.contains(&a));
    assert!(w.contains(&b));
}
