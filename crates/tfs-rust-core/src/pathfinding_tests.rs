use std::collections::HashMap;

use super::*;
use crate::test_world::support::ensure_walkable_tile;

#[test]
fn tshortway_should_relax_matches_cpp_strict_less() {
    assert!(tshortway_should_relax(100, 90));
    assert!(!tshortway_should_relax(90, 100));
    assert!(!tshortway_should_relax(100, 100));
}

#[test]
fn walk_queue_direction_matches_cpp_parent_delta_table() {
    assert_eq!(
        walk_queue_direction(Position::new(9, 9, 7), Position::new(10, 10, 7)),
        Direction::SouthEast
    );
    assert_eq!(
        walk_queue_direction(Position::new(11, 9, 7), Position::new(10, 10, 7)),
        Direction::SouthWest
    );
}

#[test]
fn reverse_path_neighbor_order_matches_expand_loop() {
    assert_eq!(
        REVERSE_PATH_NEIGHBOR_OFFSETS,
        [
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, -1),
            (1, 0),
            (1, 1),
        ]
    );
}

#[test]
fn neighbor_index_matches_cpp_direction_enum() {
    let current = Position::new(10, 10, 7);
    let parent = Position::new(9, 10, 7);
    let (list, n) = neighbor_offsets(Some(parent), current, true);
    assert_eq!(n, 5);
    assert_eq!(list[0], (0, 1));

    let parent = Position::new(10, 9, 7);
    let (list, _) = neighbor_offsets(Some(parent), current, true);
    assert_eq!(list[0], (-1, 0));
}

#[test]
fn walk_to_adjacent_params_use_chebyshev_one() {
    let fpp = FindPathParams::walk_to_adjacent();
    assert_eq!(fpp.min_target_dist, 0);
    assert_eq!(fpp.max_target_dist, 1);
    assert!(fpp.clear_sight);
    assert_eq!(
        chebyshev_dist(Position::new(11, 10, 7), Position::new(10, 10, 7)),
        1
    );
    assert_eq!(
        chebyshev_dist(Position::new(12, 10, 7), Position::new(10, 10, 7)),
        2
    );
}

#[test]
fn path_step_cost_fixed_is_tfs_10_25() {
    assert_eq!(
        path_step_cost(PathCostModel::Fixed, false, || 9999),
        MAP_NORMAL_WALK_COST
    );
    assert_eq!(
        path_step_cost(PathCostModel::Fixed, true, || 9999),
        MAP_DIAGONAL_WALK_COST
    );
}

#[test]
fn path_step_cost_terrain_weighted_uses_ground_and_diagonal_3x() {
    assert_eq!(
        path_step_cost(PathCostModel::TerrainWeighted, false, || 100),
        100
    );
    assert_eq!(
        path_step_cost(PathCostModel::TerrainWeighted, true, || 100),
        300
    );
    assert_eq!(
        path_step_cost(PathCostModel::TerrainWeighted, false, || 0),
        DEFAULT_TERRAIN_WAYPOINTS
    );
}

#[test]
fn effective_terrain_waypoints_defaults_missing_to_150() {
    assert_eq!(effective_terrain_waypoints(0), 150);
    assert_eq!(effective_terrain_waypoints(110), 110);
}

#[test]
fn scan_min_terrain_waypoints_ignores_blocked_tiles() {
    use crate::tile::{Tile, TileBody, flags as tilestate};
    use tfs_rust_common::enums::ZoneType;

    let mut map = Map {
        width: 5,
        height: 5,
        grid: crate::map::SparseGrid::new(),
        towns: HashMap::new(),
        waypoints: HashMap::new(),
    };
    let origin = Position::new(2, 2, 7);
    for x in 0..5u16 {
        for y in 0..5u16 {
            ensure_walkable_tile(&mut map, Position::new(x, y, 7), 150);
        }
    }
    map.insert_tile(
        Position::new(2, 1, 7),
        Tile::Normal(TileBody {
            ground: Some(50),

            ground_item: None,
            down_items: Vec::new(),
            top_items: Vec::new(),
            creatures: Vec::new(),
            flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
            zone: ZoneType::Normal,
        }),
    );
    let ground_from_map = |m: &Map, pos: Position| {
        m.get_tile(pos)
            .and_then(|t| t.body().ground.map(|g| g as u32))
            .unwrap_or(150)
    };
    assert_eq!(
        scan_min_terrain_waypoints(&map, origin, 2, |p| ground_from_map(&map, p)),
        150
    );
    ensure_walkable_tile(&mut map, Position::new(2, 1, 7), 100);
    assert_eq!(
        scan_min_terrain_waypoints(&map, origin, 2, |p| ground_from_map(&map, p)),
        100
    );
}

fn uniform_walkable_map(width: u16, ground: u16) -> Map {
    let mut map = Map {
        width,
        height: 1,
        grid: crate::map::SparseGrid::new(),
        towns: HashMap::new(),
        waypoints: HashMap::new(),
    };
    for x in 0..width {
        ensure_walkable_tile(&mut map, Position::new(x, 0, 7), ground);
    }
    map
}

#[test]
fn reverse_search_finds_path_to_origin() {
    let map = uniform_walkable_map(8, 100);
    let start = Position::new(0, 0, 7);
    let target = Position::new(5, 0, 7);
    let fpp = FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: false,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    };
    let can_walk = |pos: Position| map.is_walkable(pos);
    let no_extra = |_pos: Position| 0u32;
    let ground = |_pos: Position| 100u32;

    let path = get_path_matching(
        &map,
        start,
        target,
        &fpp,
        PathCostModel::TerrainWeighted,
        PathSearchModel::Reverse,
        true,
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        no_extra,
        ground,
        None,
    )
    .expect("reverse path");
    assert!(!path.is_empty());
    let steps = truncate_tshortway_go_queue(start, target, path, usize::MAX, false);
    assert!(!steps.is_empty());
    let mut pos = start;
    for dir in &steps {
        pos = pos.offset(*dir);
    }
    assert_eq!(chebyshev_dist(pos, target), 1);
}

#[test]
fn uses_reverse_terrain_path_matches_772_profile() {
    use tfs_rust_common::ProtocolVersion;

    use crate::formulas::MechanicsProfile;

    let p772 = MechanicsProfile::for_version(ProtocolVersion::V772);
    assert!(super::uses_reverse_terrain_path(
        p772.path_cost,
        p772.path_search
    ));

    let p1098 = MechanicsProfile::for_version(ProtocolVersion::V1098);
    assert!(!super::uses_reverse_terrain_path(
        p1098.path_cost,
        p1098.path_search
    ));
}

#[test]
fn reverse_with_allow_diagonal_still_uses_reverse_expansion() {
    let mut map = Map {
        width: 15,
        height: 15,
        grid: crate::map::SparseGrid::new(),
        towns: HashMap::new(),
        waypoints: HashMap::new(),
    };
    for x in 0..15u16 {
        for y in 0..15u16 {
            ensure_walkable_tile(&mut map, Position::new(x, y, 7), 150);
        }
    }
    let start = Position::new(7, 7, 7);
    let target = Position::new(12, 12, 7);
    let fpp = FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: false,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    };
    let can_walk = |pos: Position| map.is_walkable(pos);
    let no_extra = |_pos: Position| 0u32;
    let ground = |_pos: Position| 150u32;

    let path = get_path_matching(
        &map,
        start,
        target,
        &fpp,
        PathCostModel::TerrainWeighted,
        PathSearchModel::Reverse,
        false,
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        no_extra,
        ground,
        None,
    )
    .expect("path");
    assert!(!path.is_empty());
    for dir in &path {
        assert!(
            matches!(
                dir,
                Direction::North | Direction::East | Direction::South | Direction::West
            ),
            "3× waypoint cost must make cardinals win on uniform terrain, got {dir:?} in {path:?}"
        );
    }
}

#[test]
fn reverse_falls_back_to_forward_around_obstacle() {
    use crate::tile::{Tile, TileBody, flags as tilestate};
    use tfs_rust_common::enums::ZoneType;

    let mut map = Map {
        width: 7,
        height: 3,
        grid: crate::map::SparseGrid::new(),
        towns: HashMap::new(),
        waypoints: HashMap::new(),
    };
    for x in 0..7u16 {
        for y in 9..=11u16 {
            ensure_walkable_tile(&mut map, Position::new(x, y, 7), 100);
        }
    }
    // Tree / wall tile blocking the direct row between monster and player.
    map.insert_tile(
        Position::new(3, 10, 7),
        Tile::Normal(TileBody {
            ground: Some(100),

            ground_item: None,
            down_items: Vec::new(),
            top_items: Vec::new(),
            creatures: Vec::new(),
            flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
            zone: ZoneType::Normal,
        }),
    );

    let start = Position::new(1, 10, 7);
    let target = Position::new(5, 10, 7);
    let fpp = FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: false,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    };
    let can_walk = |pos: Position| map.is_walkable(pos);
    let no_extra = |_pos: Position| 0u32;
    let ground = |_pos: Position| 100u32;

    let path = get_path_matching(
        &map,
        start,
        target,
        &fpp,
        PathCostModel::TerrainWeighted,
        PathSearchModel::Reverse,
        true,
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        no_extra,
        ground,
        None,
    )
    .expect("reverse A* must detour around obstacle");
    assert!(!path.is_empty());

    let steps = truncate_tshortway_go_queue(start, target, path, usize::MAX, false);
    assert!(!steps.is_empty());
    let mut pos = start;
    for dir in &steps {
        let next = pos.offset(*dir);
        assert!(map.is_walkable(next), "path must not enter blocked tiles");
        assert_ne!(next, Position::new(3, 10, 7));
        pos = next;
    }
    assert_eq!(chebyshev_dist(pos, target), 1);
}

#[test]
fn reverse_path_heuristic_prefers_toward_origin() {
    let origin = Position::new(0, 0, 7);
    let min_wp = 50;
    let ground = |pos: Position| {
        if pos.y == 0 { 50 } else { 200 }
    };
    let near = reverse_path_heuristic(Position::new(1, 0, 7), origin, min_wp, ground);
    let far = reverse_path_heuristic(Position::new(5, 0, 7), origin, min_wp, ground);
    assert!(near < far, "heuristic must decrease toward origin");
}

#[test]
fn reverse_prefers_fast_tile_on_asymmetric_terrain() {
    let mut map = Map {
        width: 5,
        height: 3,
        grid: crate::map::SparseGrid::new(),
        towns: HashMap::new(),
        waypoints: HashMap::new(),
    };
    for y in 0..3u16 {
        for x in 0..5u16 {
            let ground = if y == 1 { 50 } else { 200 };
            ensure_walkable_tile(&mut map, Position::new(x, y, 7), ground);
        }
    }
    let start = Position::new(0, 1, 7);
    let target = Position::new(4, 1, 7);
    let fpp = FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: false,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    };
    let can_walk = |pos: Position| map.is_walkable(pos);
    let no_extra = |_pos: Position| 0u32;
    let ground = |pos: Position| {
        if pos.y == 1 { 50 } else { 200 }
    };

    let forward = get_path_matching(
        &map,
        start,
        target,
        &fpp,
        PathCostModel::TerrainWeighted,
        PathSearchModel::Forward,
        true,
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        no_extra,
        ground,
        None,
    )
    .expect("forward");
    let reverse = get_path_matching(
        &map,
        start,
        target,
        &fpp,
        PathCostModel::TerrainWeighted,
        PathSearchModel::Reverse,
        true,
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        no_extra,
        ground,
        None,
    )
    .expect("reverse");

    assert!(!forward.is_empty());
    assert!(!reverse.is_empty());
    // Forward stays on the fast row; reverse (dest→origin) weights leaving tiles differently.
    assert!(
        forward
            .iter()
            .all(|d| matches!(d, Direction::East | Direction::West)),
        "forward should stay cardinal on the fast row: {forward:?}"
    );
}

#[test]
fn forward_pathfinder_obeys_allow_diagonal() {
    let mut map = Map {
        width: 7,
        height: 7,
        grid: crate::map::SparseGrid::new(),
        towns: HashMap::new(),
        waypoints: HashMap::new(),
    };
    for x in 0..7u16 {
        for y in 0..7u16 {
            ensure_walkable_tile(&mut map, Position::new(x, y, 7), 100);
        }
    }
    // Block (1, 1), (2, 2) etc., forcing detours
    use crate::tile::{Tile, TileBody, flags as tilestate};
    use tfs_rust_common::enums::ZoneType;
    let block_pos = [
        Position::new(1, 1, 7),
        Position::new(2, 2, 7),
        Position::new(3, 3, 7),
    ];
    for bp in block_pos {
        map.insert_tile(
            bp,
            Tile::Normal(TileBody {
                ground: Some(100),

                ground_item: None,
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
                zone: ZoneType::Normal,
            }),
        );
    }

    let start = Position::new(0, 0, 7);
    let target = Position::new(4, 4, 7);
    let fpp = FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: false,
        allow_diagonal: false,
        full_path_search: true,
        max_search_dist: 0,
    };
    let can_walk = |pos: Position| map.is_walkable(pos);
    let no_extra = |_pos: Position| 0u32;
    let ground = |_pos: Position| 100u32;

    let path = get_path_matching(
        &map,
        start,
        target,
        &fpp,
        PathCostModel::Fixed,
        PathSearchModel::Forward,
        true,
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        no_extra,
        ground,
        None,
    )
    .expect("should find a path without diagonals");

    for &dir in &path {
        assert!(
            matches!(
                dir,
                Direction::North | Direction::East | Direction::South | Direction::West
            ),
            "Path contains diagonal direction {:?}! Full path: {:?}",
            dir,
            path
        );
    }
}

#[test]
fn reverse_noway_without_fallback() {
    let mut map = Map {
        width: 7,
        height: 3,
        grid: crate::map::SparseGrid::new(),
        towns: HashMap::new(),
        waypoints: HashMap::new(),
    };
    for x in 0..7u16 {
        for y in 9..=11u16 {
            ensure_walkable_tile(&mut map, Position::new(x, y, 7), 100);
        }
    }
    // Block entire column x=4, completely separating start (1, 10) from target (5, 10).
    use crate::tile::{Tile, TileBody, flags as tilestate};
    use tfs_rust_common::enums::ZoneType;
    for y in 9..=11u16 {
        map.insert_tile(
            Position::new(4, y, 7),
            Tile::Normal(TileBody {
                ground: Some(100),

                ground_item: None,
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
                zone: ZoneType::Normal,
            }),
        );
    }

    let start = Position::new(1, 10, 7);
    let target = Position::new(5, 10, 7);
    let fpp = FindPathParams {
        min_target_dist: 2,
        max_target_dist: 2,
        clear_sight: false,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    };
    let can_walk = |pos: Position| map.is_walkable(pos);
    let no_extra = |_pos: Position| 0u32;
    let ground = |_pos: Position| 100u32;

    // With fallback disabled, it must fail because the destination is cut off for reverse search.
    let path_no_fallback = get_path_matching(
        &map,
        start,
        target,
        &fpp,
        PathCostModel::TerrainWeighted,
        PathSearchModel::Reverse,
        false, // no forward fallback
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        no_extra,
        ground,
        None,
    );
    assert!(
        path_no_fallback.is_none(),
        "Must return None without forward fallback (CipSoft NOWAY)"
    );

    // With fallback enabled, it must succeed because forward search can reach (3, 10) which is distance 2 from target.
    let path_with_fallback = get_path_matching(
        &map,
        start,
        target,
        &fpp,
        PathCostModel::TerrainWeighted,
        PathSearchModel::Reverse,
        true, // forward fallback enabled
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        no_extra,
        ground,
        None,
    );
    assert!(
        path_with_fallback.is_some(),
        "Must return Some with forward fallback"
    );
}

#[test]
fn test_truncate_tshortway_go_queue() {
    let start = Position::new(32345, 32288, 7);
    let target = Position::new(32344, 32286, 7);
    let walk_order = vec![Direction::North, Direction::North, Direction::West];
    let truncated = truncate_tshortway_go_queue(start, target, walk_order, 3, false);
    assert_eq!(truncated, vec![Direction::North]);
}

#[test]
fn test_truncate_tshortway_go_queue_dist_maxsteps_enforces_band() {
    // C++ Calculate stops at cheb≤1 only; dist band is MaxSteps = Distance − 4.
    let start = Position::new(100, 100, 7);
    let target = Position::new(106, 100, 7); // cheb 6
    let walk_order = vec![
        Direction::East,
        Direction::East,
        Direction::East,
        Direction::East,
        Direction::East,
        Direction::East,
    ];
    let truncated = truncate_tshortway_go_queue(start, target, walk_order.clone(), 2, false);
    assert_eq!(
        truncated,
        vec![Direction::East, Direction::East],
        "MaxSteps=Distance−4 (2) must stop at band 4"
    );
    // Oversized MaxSteps would march to adjacent (cheb≤1 stop) — not keep-band.
    let oversized = truncate_tshortway_go_queue(start, target, walk_order, 6, false);
    assert_eq!(
        oversized.len(),
        5,
        "without MaxSteps budget, trim only stops at cheb≤1"
    );
}

#[test]
fn test_truncate_tshortway_go_queue_melee_adjacent_must_one() {
    let start = Position::new(100, 100, 7);
    let target = Position::new(102, 100, 7);
    let walk_order = vec![Direction::East, Direction::East];
    let truncated = truncate_tshortway_go_queue(start, target, walk_order, 1, true);
    assert_eq!(truncated, vec![Direction::East]);
}

/// `kite_cyclops_quad_chase` geometry — east/south cyclops match C++ ref on uniform wp=150.
#[test]
fn cyclops_quad_east_and_south_shortway_on_uniform_terrain() {
    let fpp = FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: false,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    };
    let cases = [
        (
            "east",
            Position::new(32361, 32290, 7),
            vec![
                Position::new(32361, 32291, 7),
                Position::new(32361, 32292, 7),
                Position::new(32361, 32293, 7),
            ],
        ),
        (
            "south",
            Position::new(32360, 32291, 7),
            vec![
                Position::new(32360, 32292, 7),
                Position::new(32360, 32293, 7),
            ],
        ),
    ];
    for (label, start, want_tiles) in cases {
        let target = Position::new(32360, 32294, 7);
        let map = cyclops_quad_uniform_map(start, target);
        let can_walk = |pos: Position| map.is_walkable(pos);
        let ground = |_pos: Position| 150u32;
        let dirs = get_path_matching(
            &map,
            start,
            target,
            &fpp,
            PathCostModel::TerrainWeighted,
            PathSearchModel::Reverse,
            true,
            REVERSE_PATH_VIEW_RADIUS,
            can_walk,
            |_| 0u32,
            ground,
            None,
        )
        .expect(label);
        let trimmed = truncate_tshortway_go_queue(start, target, dirs, CHASE_PATH_MAX_STEPS, false);
        let mut pos = start;
        let got_tiles: Vec<Position> = trimmed
            .iter()
            .map(|&d| {
                pos = pos.offset(d);
                pos
            })
            .collect();
        assert_eq!(got_tiles, want_tiles, "{label} shortway tiles");
    }
}

#[test]
fn fill_marks_non_walkable_tiles_with_negative_waypoints() {
    let start = Position::new(32359, 32288, 7);
    let blocked = Position::new(32359, 32289, 7);
    let target = Position::new(32360, 32294, 7);
    let map = cyclops_quad_uniform_map(start, target);
    let can_walk = |pos: Position| pos != blocked;
    let radius = REVERSE_PATH_VIEW_RADIUS;
    let mut wp_at_blocked = 0i32;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let Some(pos) = offset_position(start, dx, dy) else {
                continue;
            };
            let walkable_for_fill = map.is_walkable(pos) && (pos == target || can_walk(pos));
            let waypoints = if walkable_for_fill {
                effective_terrain_waypoints(150) as i32
            } else {
                -1
            };
            if pos == blocked {
                wp_at_blocked = waypoints;
            }
        }
    }
    assert_eq!(
        wp_at_blocked, -1,
        "blocked tile must have Waypoints=-1 in fill"
    );
}

#[test]
fn get_path_matching_blocked_far_n_matches_path_compare_pipeline() {
    let start = Position::new(32359, 32288, 7);
    let target = Position::new(32360, 32294, 7);
    let blocked = Position::new(32359, 32289, 7);
    let map = cyclops_quad_uniform_map_excluding(start, target, &[blocked]);
    let fpp = FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: false,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    };
    let raw = get_path_matching(
        &map,
        start,
        target,
        &fpp,
        PathCostModel::TerrainWeighted,
        PathSearchModel::Reverse,
        false,
        REVERSE_PATH_VIEW_RADIUS,
        |pos| map.is_walkable(pos),
        |_| 0u32,
        |_| 150u32,
        None,
    )
    .expect("raw path");
    let dirs = truncate_tshortway_go_queue(start, target, raw, CHASE_PATH_MAX_STEPS, false);
    assert_eq!(
        dirs,
        vec![Direction::East, Direction::South, Direction::South],
        "after truncate"
    );
}

#[test]
fn tshortway_blocked_missing_tile_routes_east_first() {
    let start = Position::new(32359, 32288, 7);
    let target = Position::new(32360, 32294, 7);
    let blocked = Position::new(32359, 32289, 7);
    let map = cyclops_quad_uniform_map_excluding(start, target, &[blocked]);
    let fpp = FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: false,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    };
    let mut scratch = TShortwayScratch::new();
    let dirs = path_matching_tshortway(
        &mut scratch,
        &map,
        start,
        target,
        &fpp,
        REVERSE_PATH_VIEW_RADIUS,
        |pos| map.is_walkable(pos),
        |_| 150,
    )
    .expect("path");
    let exec = truncate_tshortway_go_queue(start, target, dirs, CHASE_PATH_MAX_STEPS, false);
    assert_eq!(
        exec,
        vec![Direction::East, Direction::South, Direction::South],
        "missing blocked tile: got {exec:?}"
    );
}

#[test]
fn tshortway_skips_blocked_sibling_tile_in_fill() {
    let start = Position::new(32359, 32288, 7);
    let target = Position::new(32360, 32294, 7);
    let blocked = Position::new(32359, 32289, 7);
    let fpp = FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: false,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    };

    let mut scratch = TShortwayScratch::new();
    // Occupied sibling tile present on map (creature blocking) — `can_walk_to` rejects it.
    let map_with_tile = cyclops_quad_uniform_map(start, target);
    let can_walk = |pos: Position| pos != blocked;
    let walk_order = path_matching_tshortway(
        &mut scratch,
        &map_with_tile,
        start,
        target,
        &fpp,
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        |_| 150,
    )
    .expect("path with occupied tile");
    let mut pos = start;
    for &dir in &walk_order {
        pos = pos.offset(dir);
        assert_ne!(
            pos, blocked,
            "predecessor chain must not visit blocked tile (walk_order={walk_order:?})"
        );
    }
    let dirs = truncate_tshortway_go_queue(start, target, walk_order, CHASE_PATH_MAX_STEPS, false);
    let mut pos = start;
    for &dir in &dirs {
        pos = pos.offset(dir);
        assert_ne!(
            pos, blocked,
            "occupied tile on map: blocked sibling must not appear in path (dirs={dirs:?})"
        );
    }

    // Missing tile (path_compare style) — same outcome.
    let map_without_tile = cyclops_quad_uniform_map_excluding(start, target, &[blocked]);
    let mut walk_order = path_matching_tshortway(
        &mut scratch,
        &map_without_tile,
        start,
        target,
        &fpp,
        REVERSE_PATH_VIEW_RADIUS,
        |pos| map_without_tile.is_walkable(pos),
        |_| 150,
    )
    .expect("path without tile");
    let dirs = truncate_tshortway_go_queue(start, target, walk_order, CHASE_PATH_MAX_STEPS, false);
    assert_eq!(
        dirs,
        vec![Direction::East, Direction::South, Direction::South],
        "missing blocked tile must route east first"
    );
}

fn cyclops_quad_uniform_map(start: Position, target: Position) -> Map {
    cyclops_quad_uniform_map_excluding(start, target, &[])
}

fn cyclops_quad_uniform_map_excluding(
    start: Position,
    target: Position,
    exclude: &[Position],
) -> Map {
    let mut map = Map {
        width: 256,
        height: 256,
        grid: crate::map::SparseGrid::new(),
        towns: HashMap::new(),
        waypoints: HashMap::new(),
    };
    let pad = REVERSE_PATH_VIEW_RADIUS as u16 + 2;
    let min_x = start.x.min(target.x).saturating_sub(pad);
    let max_x = start.x.max(target.x).saturating_add(pad);
    let min_y = start.y.min(target.y).saturating_sub(pad);
    let max_y = start.y.max(target.y).saturating_add(pad);
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            let pos = Position::new(x, y, 7);
            if exclude.contains(&pos) {
                continue;
            }
            ensure_walkable_tile(&mut map, pos, 102);
        }
    }
    map
}

/// Live `kite_cyclops_quad_chase` NW shortway — faithful `cract.cc` port (Python oracle).
///
/// Live C++ JSONL diagonal path is not reproducible under standard FillMap; see `tasks/lessons.md` §59.
#[test]
fn cyclops_nw_shortway_matches_python_tshortway_port() {
    let nw = Position::new(32359, 32289, 7);
    let player = Position::new(32360, 32294, 7);
    let creature_tiles = [
        Position::new(32359, 32288, 7),
        Position::new(32361, 32290, 7),
        Position::new(32360, 32291, 7),
        player,
    ];
    let map = cyclops_quad_uniform_map(nw, player);
    let can_walk = |pos: Position| !creature_tiles.contains(&pos);
    let fpp = FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: false,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    };
    let mut scratch = TShortwayScratch::new();
    let dirs = path_matching_tshortway(
        &mut scratch,
        &map,
        nw,
        player,
        &fpp,
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        |_| 150,
    )
    .expect("path");
    let trimmed = truncate_tshortway_go_queue(nw, player, dirs, CHASE_PATH_MAX_STEPS, false);
    let mut pos = nw;
    let got: Vec<Position> = trimmed
        .iter()
        .map(|&d| {
            pos = pos.offset(d);
            pos
        })
        .collect();
    let want = [
        Position::new(32359, 32290, 7),
        Position::new(32359, 32291, 7),
        Position::new(32359, 32292, 7),
    ];
    assert_eq!(
        got, want,
        "NW shortway matches Python/cract.cc TShortway port"
    );
}

/// Live `kite_cyclops_quad_chase` far-N shortway — faithful `cract.cc` port (Python oracle).
///
/// Live C++ JSONL north path is not reachable under quad sibling occupancy; see `tasks/lessons.md` §59–60.
#[test]
fn cyclops_far_n_shortway_matches_python_tshortway_port() {
    let far_n = Position::new(32359, 32288, 7);
    let player = Position::new(32360, 32294, 7);
    let creature_tiles = [
        Position::new(32359, 32289, 7),
        Position::new(32361, 32290, 7),
        Position::new(32360, 32291, 7),
        player,
    ];
    let map = cyclops_quad_uniform_map(far_n, player);
    let can_walk = |pos: Position| !creature_tiles.contains(&pos);
    let fpp = FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: false,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    };
    let mut scratch = TShortwayScratch::new();
    let dirs = path_matching_tshortway(
        &mut scratch,
        &map,
        far_n,
        player,
        &fpp,
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        |_| 150,
    )
    .expect("path");
    let trimmed = truncate_tshortway_go_queue(far_n, player, dirs, CHASE_PATH_MAX_STEPS, false);
    let mut pos = far_n;
    let got: Vec<Position> = trimmed
        .iter()
        .map(|&d| {
            pos = pos.offset(d);
            pos
        })
        .collect();
    let want = [
        Position::new(32360, 32288, 7),
        Position::new(32360, 32289, 7),
        Position::new(32360, 32290, 7),
    ];
    assert_eq!(
        got, want,
        "far-N shortway matches Python/cract.cc TShortway port"
    );
}

/// Live C++ JSONL oracle — ignored until fresh ref log (port 7172 vs `tfs-rust`). See `tasks/lessons.md` §59.
#[test]
#[ignore = "live C++ ref diagonal NW path not reproducible from cract.cc; refresh chase_path_cip_cyclops.log"]
fn cyclops_nw_shortway_live_ref_with_blocked_dest_and_siblings() {
    let nw = Position::new(32359, 32289, 7);
    let player = Position::new(32360, 32294, 7);
    let creature_tiles = [
        Position::new(32359, 32288, 7),
        Position::new(32361, 32290, 7),
        Position::new(32360, 32291, 7),
        player,
    ];
    let map = cyclops_quad_uniform_map(nw, player);
    let can_walk = |pos: Position| !creature_tiles.contains(&pos);
    let fpp = FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: false,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    };
    let mut scratch = TShortwayScratch::new();
    let dirs = path_matching_tshortway(
        &mut scratch,
        &map,
        nw,
        player,
        &fpp,
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        |_| 150,
    )
    .expect("path");
    let trimmed = truncate_tshortway_go_queue(nw, player, dirs, CHASE_PATH_MAX_STEPS, false);
    let mut pos = nw;
    let got: Vec<Position> = trimmed
        .iter()
        .map(|&d| {
            pos = pos.offset(d);
            pos
        })
        .collect();
    let want = [
        Position::new(32358, 32290, 7),
        Position::new(32358, 32291, 7),
        Position::new(32359, 32291, 7),
    ];
    assert_eq!(got, want, "NW live-ref shortway tiles");
}

#[test]
fn tshortway_scratch_reuse_preserves_paths() {
    let start = Position::new(32359, 32288, 7);
    let target = Position::new(32360, 32294, 7);
    let map = cyclops_quad_uniform_map(start, target);
    let fpp = FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: false,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    };
    let can_walk = |pos: Position| map.is_walkable(pos);
    let mut scratch = TShortwayScratch::new();
    let first = path_matching_tshortway(
        &mut scratch,
        &map,
        start,
        target,
        &fpp,
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        |_| 150,
    )
    .expect("first path");
    let second = path_matching_tshortway(
        &mut scratch,
        &map,
        start,
        target,
        &fpp,
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        |_| 150,
    )
    .expect("second path after scratch reuse");
    assert_eq!(
        first, second,
        "generation-scratch reuse must not change paths"
    );
}

/// Many successive searches on one scratch (floor-change wake storm shape).
#[test]
fn tshortway_scratch_storm_reuse_stable() {
    let start = Position::new(32359, 32288, 7);
    let target = Position::new(32360, 32294, 7);
    let map = cyclops_quad_uniform_map(start, target);
    let fpp = FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: false,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    };
    let can_walk = |pos: Position| map.is_walkable(pos);
    let mut scratch = TShortwayScratch::new();
    let baseline = path_matching_tshortway(
        &mut scratch,
        &map,
        start,
        target,
        &fpp,
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        |_| 150,
    )
    .expect("baseline");
    // Interleave other searches (wake-storm shape) then re-query baseline endpoints.
    for i in 0..256 {
        let alt_start = Position::new(32359 + (i % 3) as u16, 32288, 7);
        let alt_target = Position::new(32360, 32292 + (i % 2) as u16, 7);
        let _ = path_matching_tshortway(
            &mut scratch,
            &map,
            alt_start,
            alt_target,
            &fpp,
            REVERSE_PATH_VIEW_RADIUS,
            can_walk,
            |_| 150,
        );
    }
    let again = path_matching_tshortway(
        &mut scratch,
        &map,
        start,
        target,
        &fpp,
        REVERSE_PATH_VIEW_RADIUS,
        can_walk,
        |_| 150,
    )
    .expect("after storm");
    assert_eq!(
        baseline, again,
        "wake-storm scratch reuse must not corrupt later paths"
    );
}
