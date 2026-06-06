//! A* pathfinding — TFS `Map::getPathMatching` / CipSoft `TShortway` (`cract.cc:7`).
//!
//! - Forward: `map.cpp` `getPathMatching`, Dijkstra-style (g-only open key).
//! - Reverse: CipSoft 7.72 `TShortway::Expand` — dest → origin, leave-tile waypoints,
//!   Manhattan heuristic with `MinWaypoints`, branch-and-bound pruning.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use tfs_rust_common::{enums::Direction, Position};

use crate::formulas::{PathCostModel, PathSearchModel};
use crate::map::Map;

/// TFS `map.h` — `MAP_NORMALWALKCOST`.
pub const MAP_NORMAL_WALK_COST: u32 = 10;
/// TFS `map.h` — `MAP_DIAGONALWALKCOST`.
const MAP_DIAGONAL_WALK_COST: u32 = 25;
/// TFS `AStarNodes::getTileWalkCost` — occupied tile penalty (`map.cpp` ~929–931).
pub const CREATURE_ON_TILE_PATH_COST: u32 = MAP_NORMAL_WALK_COST * 3;
/// TFS closed-node cap when `maxSearchDist == 0` (`map.cpp` ~680).
const MAX_CLOSED_NODES: usize = 100;
/// CipSoft monster path viewport half-extent — `VisibleX`/`VisibleY` (`cract.cc` `TShortway`).
const CIPSOFT_PATH_VIEW_RADIUS: i32 = 10;
/// CipSoft ~21×21 monster viewport tile budget (`cract.cc` `TShortway`).
const CIPSOFT_MAX_CLOSED_NODES: usize = 441;

/// TFS `FindPathParams` (`creature.h`).
#[derive(Clone, Copy, Debug)]
pub struct FindPathParams {
    pub min_target_dist: i32,
    pub max_target_dist: i32,
    pub clear_sight: bool,
    pub allow_diagonal: bool,
    /// C++ `FindPathParams::fullPathSearch` — symmetric vs directional search box.
    pub full_path_search: bool,
    /// `0` = unlimited (still capped by [`MAX_CLOSED_NODES`] like C++).
    pub max_search_dist: u32,
}

impl FindPathParams {
    /// Walk-to-use / walk-to-move item — `getPathTo(..., 0, 1, true, true)` (`game.cpp` ~973, ~2229).
    pub fn walk_to_adjacent() -> Self {
        Self {
            min_target_dist: 0,
            max_target_dist: 1,
            clear_sight: true,
            allow_diagonal: true,
            full_path_search: true,
            max_search_dist: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PathGoalMatch {
    None,
    /// TFS `bestMatchDist == 0` — stop searching.
    Exact,
    /// TFS partial endpoint — keep searching for an exact match.
    Partial { dist: i32 },
}

#[derive(Eq, PartialEq)]
struct OpenNode {
    /// Priority key — accumulated cost for TFS forward; `g + h` for CipSoft reverse A*.
    f: u32,
    g: u32,
    pos: Position,
}

impl Ord for OpenNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f
            .cmp(&self.f)
            .then_with(|| other.g.cmp(&self.g))
            .then_with(|| other.pos.x.cmp(&self.pos.x))
            .then_with(|| other.pos.y.cmp(&self.pos.y))
    }
}

impl PartialOrd for OpenNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct AStarNode {
    parent: Option<Position>,
    g: u32,
}

/// TFS `Map::getPathMatching` / CipSoft `TShortway` — creature-aware via callbacks.
///
/// `search` selects expansion direction (1098 forward / 772 reverse). Edge costs come from
/// `cost_model` (B2): fixed 10/25 for TFS, terrain waypoints + diagonal ×3 for CipSoft.
#[allow(clippy::too_many_arguments)]
pub fn get_path_matching<C, T, G>(
    map: &Map,
    start: Position,
    target: Position,
    fpp: &FindPathParams,
    cost_model: PathCostModel,
    search: PathSearchModel,
    can_walk_to: C,
    tile_walk_cost: T,
    ground_cost: G,
) -> Option<Vec<Direction>>
where
    C: Fn(Position) -> bool,
    T: Fn(Position) -> u32,
    G: Fn(Position) -> u32,
{
    match search {
        PathSearchModel::Forward => path_matching_forward(
            map,
            start,
            target,
            fpp,
            cost_model,
            can_walk_to,
            tile_walk_cost,
            ground_cost,
        ),
        PathSearchModel::Reverse => {
            let reverse = path_matching_reverse(
                map,
                start,
                target,
                fpp,
                cost_model,
                &can_walk_to,
                &tile_walk_cost,
                &ground_cost,
            );
            if let Some(ref dirs) = reverse {
                if !dirs.is_empty() {
                    return reverse;
                }
                if matches!(
                    evaluate_path_goal(map, start, start, target, fpp, 0),
                    PathGoalMatch::Exact | PathGoalMatch::Partial { .. }
                ) {
                    return reverse;
                }
            }
            // CipSoft `TShortway` is dest→origin; when the origin is unreachable (tree/wall
            // between monster and player), fall back to forward A* so the monster can detour.
            path_matching_forward(
                map,
                start,
                target,
                fpp,
                cost_model,
                can_walk_to,
                tile_walk_cost,
                ground_cost,
            )
        }
    }
}

/// TFS forward A* — origin (`start`) → goal band around `target` (`map.cpp` ~654).
#[allow(clippy::too_many_arguments)]
fn path_matching_forward<C, T, G>(
    map: &Map,
    start: Position,
    target: Position,
    fpp: &FindPathParams,
    cost_model: PathCostModel,
    can_walk_to: C,
    tile_walk_cost: T,
    ground_cost: G,
) -> Option<Vec<Direction>>
where
    C: Fn(Position) -> bool,
    T: Fn(Position) -> u32,
    G: Fn(Position) -> u32,
{
    if start.z != target.z {
        return None;
    }

    if matches!(
        evaluate_path_goal(map, start, start, target, fpp, 0),
        PathGoalMatch::Exact | PathGoalMatch::Partial { .. }
    ) {
        return Some(Vec::new());
    }

    let mut nodes: HashMap<Position, AStarNode> = HashMap::new();
    let mut open: BinaryHeap<OpenNode> = BinaryHeap::new();
    let mut closed: HashSet<Position> = HashSet::new();
    let mut best_match_dist = 0i32;
    let mut found_end: Option<Position> = None;

    nodes.insert(
        start,
        AStarNode {
            parent: None,
            g: 0,
        },
    );
    open.push(OpenNode { f: 0, g: 0, pos: start });

    while fpp.max_search_dist != 0 || closed.len() < MAX_CLOSED_NODES {
        let Some(OpenNode { pos: current, .. }) = open.pop() else {
            break;
        };
        if !closed.insert(current) {
            continue;
        }

        match evaluate_path_goal(map, start, current, target, fpp, best_match_dist) {
            PathGoalMatch::None => {}
            PathGoalMatch::Exact => {
                found_end = Some(current);
                best_match_dist = 0;
            }
            PathGoalMatch::Partial { dist } => {
                found_end = Some(current);
                best_match_dist = dist;
            }
        }

        if found_end.is_some() && best_match_dist == 0 {
            break;
        }

        let base_g = nodes.get(&current).map(|n| n.g).unwrap_or(u32::MAX);
        if base_g == u32::MAX {
            continue;
        }

        let parent = nodes.get(&current).and_then(|n| n.parent);
        let (neighbor_list, dir_count) = neighbor_offsets(parent, current, fpp.allow_diagonal);

        for &(ox, oy) in &neighbor_list[..dir_count] {
            let Some(next) = offset_position(current, ox, oy) else {
                continue;
            };

            if fpp.max_search_dist != 0 {
                let sdx = (start.x as i32 - next.x as i32).unsigned_abs();
                let sdy = (start.y as i32 - next.y as i32).unsigned_abs();
                if sdx > fpp.max_search_dist || sdy > fpp.max_search_dist {
                    continue;
                }
            }

            if closed.contains(&next) {
                continue;
            }

            let is_diagonal = ox != 0 && oy != 0;
            if !nodes.contains_key(&next) && !can_walk_to(next) {
                continue;
            }

            let step_cost = path_step_cost(cost_model, is_diagonal, || ground_cost(current));
            let new_g = base_g
                .saturating_add(step_cost)
                .saturating_add(tile_walk_cost(next));

            let prev_g = nodes.get(&next).map(|n| n.g).unwrap_or(u32::MAX);
            if new_g < prev_g {
                nodes.insert(
                    next,
                    AStarNode {
                        parent: Some(current),
                        g: new_g,
                    },
                );
                open.push(OpenNode {
                    f: new_g,
                    g: new_g,
                    pos: next,
                });
            }
        }
    }

    let end_pos = found_end?;
    Some(reconstruct_forward_dirs(&nodes, end_pos))
}

/// CipSoft reverse A* — destination (`target`) → origin (`start`) (`cract.cc:7` `TShortway`).
///
/// Expands from the follow destination back toward the monster with Manhattan heuristic:
/// `H(n) = Waypoints(n) + MinWaypoints × (Manhattan(n, origin) − 1)` (`cract.cc`).
/// Walk directions run origin → destination (parent chain toward the seed).
#[allow(clippy::too_many_arguments)]
fn path_matching_reverse<C, T, G>(
    map: &Map,
    start: Position,
    target: Position,
    fpp: &FindPathParams,
    cost_model: PathCostModel,
    can_walk_to: C,
    tile_walk_cost: T,
    ground_cost: G,
) -> Option<Vec<Direction>>
where
    C: Fn(Position) -> bool,
    T: Fn(Position) -> u32,
    G: Fn(Position) -> u32,
{
    if start.z != target.z {
        return None;
    }

    if matches!(
        evaluate_path_goal(map, start, start, target, fpp, 0),
        PathGoalMatch::Exact | PathGoalMatch::Partial { .. }
    ) {
        return Some(Vec::new());
    }

    let viewport_radius = if fpp.max_search_dist != 0 {
        fpp.max_search_dist as i32
    } else {
        CIPSOFT_PATH_VIEW_RADIUS
    };
    let closed_cap = if fpp.max_search_dist != 0 {
        usize::MAX
    } else {
        CIPSOFT_MAX_CLOSED_NODES
    };
    let use_cipsoft_astar = matches!(cost_model, PathCostModel::TerrainWeighted);
    let min_wp = if use_cipsoft_astar {
        scan_min_waypoints(start, viewport_radius, &ground_cost)
    } else {
        1
    };

    let mut nodes: HashMap<Position, AStarNode> = HashMap::new();
    let mut open: BinaryHeap<OpenNode> = BinaryHeap::new();
    let mut closed: HashSet<Position> = HashSet::new();

    // Destination may hold the follow target — seed without occupancy check (`TShortway` dest tile).
    nodes.insert(
        target,
        AStarNode {
            parent: None,
            g: 0,
        },
    );
    let seed_h = if use_cipsoft_astar {
        cipsoft_shortway_heuristic(target, start, min_wp, &ground_cost)
    } else {
        0
    };
    open.push(OpenNode {
        f: seed_h,
        g: 0,
        pos: target,
    });

    while fpp.max_search_dist != 0 || closed.len() < closed_cap {
        let Some(OpenNode { pos: current, .. }) = open.pop() else {
            break;
        };
        if !closed.insert(current) {
            continue;
        }

        if current == start {
            let dirs = reconstruct_reverse_dirs(&nodes, start);
            let mut trimmed = trim_path_to_goal_band(dirs, start, target, fpp, map);
            trimmed.reverse();
            return Some(trimmed);
        }

        let base_g = nodes.get(&current).map(|n| n.g).unwrap_or(u32::MAX);
        if base_g == u32::MAX {
            continue;
        }

        let parent = nodes.get(&current).and_then(|n| n.parent);
        let (neighbor_list, dir_count) = neighbor_offsets(parent, current, fpp.allow_diagonal);

        for &(ox, oy) in &neighbor_list[..dir_count] {
            let Some(next) = offset_position(current, ox, oy) else {
                continue;
            };

            if !in_path_viewport(start, next, viewport_radius) {
                continue;
            }

            if closed.contains(&next) {
                continue;
            }

            let is_diagonal = ox != 0 && oy != 0;
            if !nodes.contains_key(&next) && !can_walk_to(next) {
                continue;
            }

            let step_cost = path_step_cost(cost_model, is_diagonal, || ground_cost(current));
            let new_g = base_g
                .saturating_add(step_cost)
                .saturating_add(tile_walk_cost(next));

            // CipSoft branch-and-bound — `TShortway::Expand` (`cract.cc`).
            if let Some(&AStarNode { g: origin_g, .. }) = nodes.get(&start) {
                if new_g >= origin_g {
                    continue;
                }
            }

            let prev_g = nodes.get(&next).map(|n| n.g).unwrap_or(u32::MAX);
            if new_g < prev_g {
                let h = if use_cipsoft_astar {
                    cipsoft_shortway_heuristic(next, start, min_wp, &ground_cost)
                } else {
                    0
                };
                nodes.insert(
                    next,
                    AStarNode {
                        parent: Some(current),
                        g: new_g,
                    },
                );
                open.push(OpenNode {
                    f: new_g.saturating_add(h),
                    g: new_g,
                    pos: next,
                });
            }
        }
    }

    None
}

/// TFS `FrozenPathingConditionCall::operator()` (`creature.cpp` ~1688–1720).
fn evaluate_path_goal(
    map: &Map,
    start: Position,
    test: Position,
    target: Position,
    fpp: &FindPathParams,
    best_match_dist: i32,
) -> PathGoalMatch {
    if !path_in_search_box(start, test, target, fpp) {
        return PathGoalMatch::None;
    }
    if fpp.clear_sight && !map.is_sight_clear(test, target) {
        return PathGoalMatch::None;
    }

    let test_dist = chebyshev_dist(test, target);
    if fpp.max_target_dist == 1 {
        return if (fpp.min_target_dist..=fpp.max_target_dist).contains(&test_dist) {
            PathGoalMatch::Exact
        } else {
            PathGoalMatch::None
        };
    }

    if test_dist > fpp.max_target_dist || test_dist < fpp.min_target_dist {
        return PathGoalMatch::None;
    }

    if test_dist == fpp.max_target_dist {
        PathGoalMatch::Exact
    } else if test_dist > best_match_dist {
        PathGoalMatch::Partial { dist: test_dist }
    } else {
        PathGoalMatch::None
    }
}

/// TFS `FrozenPathingConditionCall::isInRange` (`creature.cpp` ~1641–1685).
fn path_in_search_box(start: Position, test: Position, target: Position, fpp: &FindPathParams) -> bool {
    if fpp.full_path_search {
        let dx = (test.x as i32 - target.x as i32).abs();
        let dy = (test.y as i32 - target.y as i32).abs();
        return dx <= fpp.max_target_dist && dy <= fpp.max_target_dist;
    }

    let offset_x = start.x as i32 - target.x as i32;
    let offset_y = start.y as i32 - target.y as i32;

    let dx_max = if offset_x >= 0 { fpp.max_target_dist } else { 0 };
    if (test.x as i32) > (target.x as i32) + dx_max {
        return false;
    }
    let dx_min = if offset_x <= 0 { fpp.max_target_dist } else { 0 };
    if (test.x as i32) < (target.x as i32) - dx_min {
        return false;
    }

    let dy_max = if offset_y >= 0 { fpp.max_target_dist } else { 0 };
    if (test.y as i32) > (target.y as i32) + dy_max {
        return false;
    }
    let dy_min = if offset_y <= 0 { fpp.max_target_dist } else { 0 };
    if (test.y as i32) < (target.y as i32) - dy_min {
        return false;
    }
    true
}

/// Per-step edge cost for the A* expansion (B2).
///
/// - [`PathCostModel::Fixed`] — TFS 1.4.2 constants 10 / 25 (`map.cpp`), terrain ignored.
/// - [`PathCostModel::TerrainWeighted`] — CipSoft 7.72 (`cract.cc:136–155` `TShortway::Expand`):
///   cost = current tile waypoints; a diagonal step costs `×3` (cardinal `+wp`, diagonal `+wp*3`).
fn path_step_cost(model: PathCostModel, is_diagonal: bool, ground_cost: impl FnOnce() -> u32) -> u32 {
    match model {
        PathCostModel::Fixed => {
            if is_diagonal {
                MAP_DIAGONAL_WALK_COST
            } else {
                MAP_NORMAL_WALK_COST
            }
        }
        PathCostModel::TerrainWeighted => {
            let wp = ground_cost().max(1);
            if is_diagonal {
                wp.saturating_mul(3)
            } else {
                wp
            }
        }
    }
}

fn chebyshev_dist(a: Position, b: Position) -> i32 {
    let dx = (a.x as i32 - b.x as i32).unsigned_abs() as i32;
    let dy = (a.y as i32 - b.y as i32).unsigned_abs() as i32;
    dx.max(dy)
}

fn manhattan_dist(a: Position, b: Position) -> i32 {
    (a.x as i32 - b.x as i32).unsigned_abs() as i32 + (a.y as i32 - b.y as i32).unsigned_abs() as i32
}

/// CipSoft `VisibleX`/`VisibleY` rectangle around the origin (`cract.cc` `TShortway`).
fn in_path_viewport(origin: Position, pos: Position, radius: i32) -> bool {
    let dx = (origin.x as i32 - pos.x as i32).unsigned_abs() as i32;
    let dy = (origin.y as i32 - pos.y as i32).unsigned_abs() as i32;
    dx <= radius && dy <= radius
}

/// Minimum tile waypoints in the origin viewport — CipSoft `MinWaypoints` (`cract.cc` `TShortway`).
fn scan_min_waypoints<G>(origin: Position, radius: i32, ground_cost: G) -> u32
where
    G: Fn(Position) -> u32,
{
    let mut min = u32::MAX;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if let Some(pos) = offset_position(origin, dx, dy) {
                min = min.min(ground_cost(pos).max(1));
            }
        }
    }
    min.max(1)
}

/// CipSoft `TShortway` A* heuristic — `cract.cc`.
fn cipsoft_shortway_heuristic<G>(pos: Position, origin: Position, min_wp: u32, ground_cost: G) -> u32
where
    G: Fn(Position) -> u32,
{
    let wp = ground_cost(pos).max(1);
    let md = manhattan_dist(pos, origin).saturating_sub(1).max(0) as u32;
    wp.saturating_add(min_wp.saturating_mul(md))
}

fn offset_position(from: Position, ox: i32, oy: i32) -> Option<Position> {
    let nx = from.x as i32 + ox;
    let ny = from.y as i32 + oy;
    if nx < 0 || ny < 0 {
        return None;
    }
    Some(Position {
        x: nx as u16,
        y: ny as u16,
        z: from.z,
    })
}

/// C++ `dirNeighbors` / `allNeighbors` (`map.cpp` ~663–675).
fn neighbor_offsets(
    parent: Option<Position>,
    current: Position,
    allow_diagonal: bool,
) -> (&'static [(i32, i32)], usize) {
    const ALL_NEIGHBORS: [(i32, i32); 8] = [
        (-1, 0),
        (0, 1),
        (1, 0),
        (0, -1),
        (-1, -1),
        (1, -1),
        (1, 1),
        (-1, 1),
    ];
    const DIR_NEIGHBORS: [[(i32, i32); 5]; 8] = [
        [(-1, 0), (0, 1), (1, 0), (1, 1), (-1, 1)],
        [(-1, 0), (0, 1), (0, -1), (-1, -1), (-1, 1)],
        [(-1, 0), (1, 0), (0, -1), (-1, -1), (1, -1)],
        [(0, 1), (1, 0), (0, -1), (1, -1), (1, 1)],
        [(1, 0), (0, -1), (-1, -1), (1, -1), (1, 1)],
        [(-1, 0), (0, -1), (-1, -1), (1, -1), (-1, 1)],
        [(0, 1), (1, 0), (1, -1), (1, 1), (-1, 1)],
        [(-1, 0), (0, 1), (-1, -1), (1, 1), (-1, 1)],
    ];

    let Some(prev) = parent else {
        return (&ALL_NEIGHBORS, ALL_NEIGHBORS.len());
    };

    let dx = prev.x as i32 - current.x as i32;
    let dy = prev.y as i32 - current.y as i32;
    let idx = if dy == 0 {
        if dx == -1 { 3 } else { 1 }
    } else if !allow_diagonal || dx == 0 {
        if dy == -1 { 0 } else { 2 }
    } else if dy == -1 {
        if dx == -1 { 6 } else { 7 }
    } else if dx == -1 {
        4
    } else {
        5
    };
    let dir_count = if allow_diagonal { 5 } else { 3 };
    (&DIR_NEIGHBORS[idx], dir_count)
}

/// Forward walk-queue: last element is the first step (`creature.cpp` `listWalkDir`).
fn reconstruct_forward_dirs(nodes: &HashMap<Position, AStarNode>, end: Position) -> Vec<Direction> {
    let mut dir_list = Vec::new();
    let mut prev = end;
    let mut cur = nodes.get(&end).and_then(|n| n.parent);
    while let Some(pos) = cur {
        dir_list.push(walk_queue_direction(pos, prev));
        prev = pos;
        cur = nodes.get(&pos).and_then(|n| n.parent);
    }
    dir_list
}

/// Drop trailing steps that overshoot the frozen-path goal band (`creature.cpp` ~1688).
fn trim_path_to_goal_band(
    mut dirs: Vec<Direction>,
    start: Position,
    target: Position,
    fpp: &FindPathParams,
    map: &Map,
) -> Vec<Direction> {
    while !dirs.is_empty() {
        let mut pos = start;
        for &d in &dirs {
            pos = pos.offset(d);
        }
        if matches!(
            evaluate_path_goal(map, start, pos, target, fpp, 0),
            PathGoalMatch::Exact | PathGoalMatch::Partial { .. }
        ) {
            return dirs;
        }
        dirs.pop();
    }
    Vec::new()
}

/// Reverse walk-queue: origin → destination along parent chain toward the seed.
fn reconstruct_reverse_dirs(nodes: &HashMap<Position, AStarNode>, origin: Position) -> Vec<Direction> {
    let mut dir_list = Vec::new();
    let mut cur = origin;
    while let Some(next) = nodes.get(&cur).and_then(|n| n.parent) {
        dir_list.push(walk_queue_direction(cur, next));
        cur = next;
    }
    dir_list
}

/// TFS parent-chain direction encoding (`map.cpp` ~806–821).
fn walk_queue_direction(from: Position, to: Position) -> Direction {
    let dx = from.x as i32 - to.x as i32;
    let dy = from.y as i32 - to.y as i32;
    match (dx, dy) {
        (1, 1) => Direction::NorthWest,
        (-1, 1) => Direction::NorthEast,
        (1, -1) => Direction::SouthWest,
        (-1, -1) => Direction::SouthEast,
        (1, 0) => Direction::West,
        (-1, 0) => Direction::East,
        (0, 1) => Direction::North,
        (0, -1) => Direction::South,
        _ => Direction::North,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::test_world::support::ensure_walkable_tile;

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
        assert_eq!(chebyshev_dist(Position::new(11, 10, 7), Position::new(10, 10, 7)), 1);
        assert_eq!(chebyshev_dist(Position::new(12, 10, 7), Position::new(10, 10, 7)), 2);
    }

    #[test]
    fn path_step_cost_fixed_is_tfs_10_25() {
        assert_eq!(path_step_cost(PathCostModel::Fixed, false, || 9999), MAP_NORMAL_WALK_COST);
        assert_eq!(path_step_cost(PathCostModel::Fixed, true, || 9999), MAP_DIAGONAL_WALK_COST);
    }

    #[test]
    fn path_step_cost_terrain_weighted_uses_ground_and_diagonal_3x() {
        assert_eq!(path_step_cost(PathCostModel::TerrainWeighted, false, || 100), 100);
        assert_eq!(path_step_cost(PathCostModel::TerrainWeighted, true, || 100), 300);
        assert_eq!(path_step_cost(PathCostModel::TerrainWeighted, false, || 0), 1);
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
            can_walk,
            no_extra,
            ground,
        )
        .expect("reverse path");
        assert!(!path.is_empty());
        let mut pos = start;
        for dir in path.iter().rev() {
            pos = pos.offset(*dir);
        }
        assert_eq!(chebyshev_dist(pos, target), 1);
    }

    #[test]
    fn reverse_falls_back_to_forward_around_obstacle() {
        use crate::tile::{flags as tilestate, Tile, TileBody};
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
            can_walk,
            no_extra,
            ground,
        )
        .expect("reverse A* must detour around obstacle");
        assert!(!path.is_empty());

        let mut pos = start;
        for dir in path.iter().rev() {
            let next = pos.offset(*dir);
            assert!(map.is_walkable(next), "path must not enter blocked tiles");
            assert_ne!(next, Position::new(3, 10, 7));
            pos = next;
        }
        assert_eq!(chebyshev_dist(pos, target), 1);
    }

    #[test]
    fn cipsoft_heuristic_prefers_toward_origin() {
        let origin = Position::new(0, 0, 7);
        let min_wp = 50;
        let ground = |pos: Position| {
            if pos.y == 0 { 50 } else { 200 }
        };
        let near = cipsoft_shortway_heuristic(Position::new(1, 0, 7), origin, min_wp, ground);
        let far = cipsoft_shortway_heuristic(Position::new(5, 0, 7), origin, min_wp, ground);
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
            can_walk,
            no_extra,
            ground,
        )
        .expect("forward");
        let reverse = get_path_matching(
            &map,
            start,
            target,
            &fpp,
            PathCostModel::TerrainWeighted,
            PathSearchModel::Reverse,
            can_walk,
            no_extra,
            ground,
        )
        .expect("reverse");

        assert!(!forward.is_empty());
        assert!(!reverse.is_empty());
        // Forward stays on the fast row; reverse (dest→origin) weights leaving tiles differently.
        assert!(
            forward.iter().all(|d| matches!(d, Direction::East | Direction::West)),
            "forward should stay cardinal on the fast row: {forward:?}"
        );
    }
}
