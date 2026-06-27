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

fn flat_map(w: u16, h: u16) -> Map {
    let mut map = Map {
        width: w,
        height: h,
        grid: SparseGrid::new(),
        towns: std::collections::HashMap::new(),
        waypoints: std::collections::HashMap::new(),
    };
    for x in 0..w {
        for y in 0..h {
            map.insert_tile(Position::new(x, y, 7), body_at(x, y, 0));
        }
    }
    map
}

/// 772 `ThrowPossible` is clear across open ground (same floor, `power = 0`).
#[test]
fn throw_possible_clear_on_open_ground() {
    let map = flat_map(8, 8);
    assert!(map.throw_possible(Position::new(0, 0, 7), Position::new(5, 0, 7), 0));
    assert!(map.throw_possible(Position::new(0, 0, 7), Position::new(4, 4, 7), 0));
}

/// `UNTHROW` (projectile-block) on the interpolated line blocks 772 throw.
#[test]
fn throw_possible_blocked_by_unthrow() {
    let mut map = flat_map(8, 8);
    map.insert_tile(Position::new(2, 0, 7), body_at(2, 0, flags::UNTHROW));
    assert!(!map.throw_possible(Position::new(0, 0, 7), Position::new(5, 0, 7), 0));
}

/// Finding 16b — a solid-but-throwable tile (`BLOCKSOLID`/`BLOCKPATH`, no `UNTHROW`) does **not**
/// block 772 throw, even though it blocks the 1098 Bresenham `is_sight_clear`.
#[test]
fn throw_possible_ignores_solid_without_unthrow() {
    let mut map = flat_map(8, 8);
    map.insert_tile(
        Position::new(2, 0, 7),
        body_at(2, 0, flags::BLOCKSOLID | flags::BLOCKPATH),
    );
    assert!(
        !map.is_sight_clear(Position::new(0, 0, 7), Position::new(5, 0, 7)),
        "1098 Bresenham is blocked by the solid tile"
    );
    assert!(
        map.throw_possible(Position::new(0, 0, 7), Position::new(5, 0, 7), 0),
        "772 throw passes a solid-but-throwable tile (UNTHROW not set)"
    );
}

/// Adjacent / same tile is always reachable (`MaxT <= 1`).
#[test]
fn throw_possible_adjacent_is_clear() {
    let map = flat_map(4, 4);
    assert!(map.throw_possible(Position::new(1, 1, 7), Position::new(2, 1, 7), 0));
    assert!(map.throw_possible(Position::new(1, 1, 7), Position::new(1, 1, 7), 0));
}
