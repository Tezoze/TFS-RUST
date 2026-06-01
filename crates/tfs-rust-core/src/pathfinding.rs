//! A* pathfinding — TFS `Map::getPathMatching` / `AStarNodes` (`map.cpp`).
// C++ reference: `map.cpp` `getPathMatching`, `canWalkTo`; `creature.cpp` `getPathTo`, `FrozenPathingConditionCall`.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use tfs_rust_common::{enums::Direction, Position};

use crate::formulas::PathCostModel;
use crate::map::Map;

/// TFS `map.h` — `MAP_NORMALWALKCOST`.
pub const MAP_NORMAL_WALK_COST: u32 = 10;
/// TFS `map.h` — `MAP_DIAGONALWALKCOST`.
const MAP_DIAGONAL_WALK_COST: u32 = 25;
/// TFS `AStarNodes::getTileWalkCost` — occupied tile penalty (`map.cpp` ~929–931).
pub const CREATURE_ON_TILE_PATH_COST: u32 = MAP_NORMAL_WALK_COST * 3;
/// TFS closed-node cap when `maxSearchDist == 0` (`map.cpp` ~680).
const MAX_CLOSED_NODES: usize = 100;

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
    f: u32,
    pos: Position,
}

impl Ord for OpenNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.cmp(&self.f)
    }
}

impl PartialOrd for OpenNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct AStarNode {
    parent: Option<Position>,
    f: u32,
}

/// TFS `Map::getPathMatching` — creature-aware via `can_walk_to` / `tile_walk_cost` callbacks.
///
/// `cost_model` selects the edge-cost function (B2):
/// - [`PathCostModel::Fixed`] — TFS 1.4.2: 10 normal / 25 diagonal (`map.cpp`). 1098 default, unchanged.
/// - [`PathCostModel::TerrainWeighted`] — CipSoft 7.72 `TShortway` (`cract.cc:136–155`): step cost is
///   the **current** tile's terrain waypoints (from `ground_cost`), diagonal = `×3` that tile.
///
/// `ground_cost(pos)` returns the tile's terrain waypoint weight (only consulted for
/// `TerrainWeighted`; `Fixed` ignores it). The algorithm/search box are identical for both models.
#[allow(clippy::too_many_arguments)]
pub fn get_path_matching<C, T, G>(
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
            f: 0,
        },
    );
    open.push(OpenNode { f: 0, pos: start });

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

        let base_f = nodes.get(&current).map(|n| n.f).unwrap_or(u32::MAX);
        if base_f == u32::MAX {
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
            let new_f = base_f
                .saturating_add(step_cost)
                .saturating_add(tile_walk_cost(next));

            let prev_f = nodes.get(&next).map(|n| n.f).unwrap_or(u32::MAX);
            if new_f < prev_f {
                nodes.insert(
                    next,
                    AStarNode {
                        parent: Some(current),
                        f: new_f,
                    },
                );
                // C++ `AStarNodes` stores **g** in `node->f` and picks lowest g — no heuristic (`map.cpp`).
                open.push(OpenNode { f: new_f, pos: next });
            }
        }
    }

    let end_pos = found_end?;
    Some(reconstruct_walk_queue_dirs(&nodes, end_pos))
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
/// `DIR_NEIGHBORS` rows are indexed by `Direction` enum (`position.h`: N=0, E=1, S=2, W=3, …).
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
        [(-1, 0), (0, 1), (1, 0), (1, 1), (-1, 1)],       // DIRECTION_NORTH = 0
        [(-1, 0), (0, 1), (0, -1), (-1, -1), (-1, 1)],   // DIRECTION_EAST = 1
        [(-1, 0), (1, 0), (0, -1), (-1, -1), (1, -1)],   // DIRECTION_SOUTH = 2
        [(0, 1), (1, 0), (0, -1), (1, -1), (1, 1)],      // DIRECTION_WEST = 3
        [(1, 0), (0, -1), (-1, -1), (1, -1), (1, 1)],    // DIRECTION_SOUTHWEST = 4
        [(-1, 0), (0, -1), (-1, -1), (1, -1), (-1, 1)],  // DIRECTION_SOUTHEAST = 5
        [(0, 1), (1, 0), (1, -1), (1, 1), (-1, 1)],      // DIRECTION_NORTHWEST = 6
        [(-1, 0), (0, 1), (-1, -1), (1, 1), (-1, 1)],    // DIRECTION_NORTHEAST = 7
    ];

    let Some(prev) = parent else {
        return (&ALL_NEIGHBORS, ALL_NEIGHBORS.len());
    };

    // C++ `map.cpp` ~703–728: `offset = parent - current` = vector pointing BACK toward the parent
    // = the direction we "came from". DIRECTION_WEST (=3) when parent is to the west, even
    // though we travelled east. This matches C++ exactly (same delta formula, same direction enum).
    let dx = prev.x as i32 - current.x as i32;
    let dy = prev.y as i32 - current.y as i32;
    let idx = if dy == 0 {
        if dx == -1 { 3 } else { 1 } // WEST / EAST
    } else if !allow_diagonal || dx == 0 {
        if dy == -1 { 0 } else { 2 } // NORTH / SOUTH
    } else if dy == -1 {
        if dx == -1 { 6 } else { 7 } // NW / NE
    } else if dx == -1 {
        4 // SW
    } else {
        5 // SE
    };
    let dir_count = if allow_diagonal { 5 } else { 3 };
    (&DIR_NEIGHBORS[idx], dir_count)
}

/// Walk-queue order: **last** element is the **first** step (`Creature::getNextStep` pops back).
fn reconstruct_walk_queue_dirs(nodes: &HashMap<Position, AStarNode>, end: Position) -> Vec<Direction> {
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
    use super::*;

    #[test]
    fn walk_queue_direction_matches_cpp_parent_delta_table() {
        // C++ encodes direction from parent→child delta (`map.cpp` ~806–821), not intuitive map north.
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
        // Approaching from WEST (parent west of current): `map.cpp` uses `DIRECTION_WEST` (=3).
        let current = Position::new(10, 10, 7);
        let parent = Position::new(9, 10, 7);
        let (list, n) = neighbor_offsets(Some(parent), current, true);
        assert_eq!(n, 5);
        assert_eq!(list[0], (0, 1)); // first neighbor for WEST row in C++

        // Approaching from NORTH (parent north = smaller y).
        let parent = Position::new(10, 9, 7);
        let (list, _) = neighbor_offsets(Some(parent), current, true);
        assert_eq!(list[0], (-1, 0)); // NORTH row
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

    /// B2 — fixed (1098) edge cost is the unchanged TFS 10/25; terrain (772) weights by tile speed.
    #[test]
    fn path_step_cost_fixed_is_tfs_10_25() {
        assert_eq!(path_step_cost(PathCostModel::Fixed, false, || 9999), MAP_NORMAL_WALK_COST);
        assert_eq!(path_step_cost(PathCostModel::Fixed, true, || 9999), MAP_DIAGONAL_WALK_COST);
        assert_eq!(MAP_NORMAL_WALK_COST, 10);
        assert_eq!(MAP_DIAGONAL_WALK_COST, 25);
    }

    /// B2 — CipSoft terrain weight: cardinal = tile waypoints, diagonal = ×3 (`cract.cc` `TShortway`).
    #[test]
    fn path_step_cost_terrain_weighted_uses_ground_and_diagonal_3x() {
        // Fast tile (low ground speed value) costs less than a slow tile.
        assert_eq!(path_step_cost(PathCostModel::TerrainWeighted, false, || 100), 100);
        assert_eq!(path_step_cost(PathCostModel::TerrainWeighted, true, || 100), 300);
        assert_eq!(path_step_cost(PathCostModel::TerrainWeighted, false, || 250), 250);
        // Zero ground never yields a free edge.
        assert_eq!(path_step_cost(PathCostModel::TerrainWeighted, false, || 0), 1);
    }
}
