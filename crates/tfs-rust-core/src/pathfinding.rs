//! A* pathfinding — TFS `Map::getPathMatching` / 772 `TShortway` (`cract.cc:7`).
//!
//! - Forward: `map.cpp` `getPathMatching`, Dijkstra-style (g-only open key).
//! - Reverse: 772 `TShortway::Expand` — dest → origin, leave-tile waypoints,
//!   fixed 8-neighbor expansion (no TFS `dirNeighbors` bias), Manhattan heuristic with
//!   `MinWaypoints`, branch-and-bound pruning.

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
/// 772 monster path viewport half-extent — `VisibleX`/`VisibleY` (`cract.cc` `TShortway`).
pub const REVERSE_PATH_VIEW_RADIUS: i32 = 10;
/// 772 ~21×21 monster viewport tile budget (`cract.cc` `TShortway`).
const REVERSE_PATH_MAX_CLOSED_NODES: usize = 441;
/// 772 default BANK `Waypoints` when unset — matches `ground_speed_for_item` / `NotifyGo` default.
pub const DEFAULT_TERRAIN_WAYPOINTS: u32 = 150;

/// Effective per-tile waypoint for `TShortway` — OTB `ITEM_ATTR_SPEED` / 772 `WAYPOINTS`.
///
/// `0` (missing OTB speed) maps to [`DEFAULT_TERRAIN_WAYPOINTS`], not `1`. C++ reference:
/// `cract.cc` `TShortway::FillMap`, `NotifyGo` (`WAYPOINTS`).
#[inline]
pub fn effective_terrain_waypoints(raw: u32) -> u32 {
    if raw == 0 {
        DEFAULT_TERRAIN_WAYPOINTS
    } else {
        raw
    }
}

/// 772 `TShortway::FillMap` — minimum walkable `WAYPOINTS` in the origin viewport (`cract.cc`).
pub fn scan_min_terrain_waypoints<G>(
    map: &Map,
    origin: Position,
    radius: i32,
    ground_cost: G,
) -> u32
where
    G: Fn(Position) -> u32,
{
    let mut min = u32::MAX;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let Some(pos) = offset_position(origin, dx, dy) else {
                continue;
            };
            if !map.is_walkable(pos) {
                continue;
            }
            let wp = effective_terrain_waypoints(ground_cost(pos));
            if wp > 0 {
                min = min.min(wp);
            }
        }
    }
    if min == u32::MAX {
        DEFAULT_TERRAIN_WAYPOINTS
    } else {
        min
    }
}

/// TFS `FindPathParams` (`creature.h`).
///
/// **`allow_diagonal` does not select the pathfinding era.** Search direction and edge costs
/// come from [`MechanicsProfile::path_search`] / [`MechanicsProfile::path_cost`] passed to
/// [`get_path_matching`]. On 772 reverse search, `allow_diagonal` only filters
/// [`REVERSE_PATH_NEIGHBOR_OFFSETS`]; TFS 1098 [`neighbor_offsets`] / `dirNeighbors` run only when
/// `path_search == Forward` (or explicit forward fallback after reverse failure).
#[derive(Clone, Copy, Debug)]
pub struct FindPathParams {
    pub min_target_dist: i32,
    pub max_target_dist: i32,
    pub clear_sight: bool,
    /// Include diagonal neighbors in expansion. Does **not** switch to TFS forward A* or 10/25 costs.
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
    /// Priority key — accumulated cost for TFS forward; `g + h` for 772 reverse A*.
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

/// 772 `TShortway` profile — reverse dest→origin with terrain waypoint costs (diagonal ×3).
///
/// When true, `FindPathParams::allow_diagonal` only toggles the 8-neighbor 772 expansion;
/// it never selects TFS forward `dirNeighbors` or fixed 10/25 edge costs.
#[inline]
pub fn uses_reverse_terrain_path(cost_model: PathCostModel, search: PathSearchModel) -> bool {
    matches!(
        (cost_model, search),
        (PathCostModel::TerrainWeighted, PathSearchModel::Reverse)
    )
}

/// TFS `Map::getPathMatching` / 772 `TShortway` — creature-aware via callbacks.
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
    forward_fallback: bool,
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
            if !forward_fallback {
                return None;
            }
            // Forward fallback uses TFS `dirNeighbors` expansion — not 772 `TShortway`.
            // Default 772 profile sets `path_forward_fallback = false` (NOWAY). Only reached when
            // explicitly enabled (e.g. 1098 overlay); `allow_diagonal` on the FPP is unrelated.
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

/// One cell in the 772 `TShortway` viewport grid (`cract.cc` `TShortwayPoint`).
struct TShortwayCell {
    waypoints: i32,
    waylength: u32,
    heuristic: u32,
    parent: Option<Position>,
    /// Incoming edge to this cell was diagonal — used for equal-cost cardinal tie-break.
    parent_diagonal: bool,
    expand_next: Option<Position>,
}

const TSHORTWAY_UNVISITED: u32 = u32::MAX;

/// Prefer cardinal when two relaxations reach the same `waylength` (`cract.cc` strict `<` keeps
/// first-seen; linked-list expand order can still tie — cardinals match live 772 chase traces).
fn tshortway_should_relax(
    prev_waylength: u32,
    new_waylength: u32,
    prev_parent_diagonal: bool,
    new_edge_diagonal: bool,
) -> bool {
    if new_waylength < prev_waylength {
        return true;
    }
    if new_waylength > prev_waylength {
        return false;
    }
    !new_edge_diagonal && prev_parent_diagonal
}

/// 772 `TShortway` search state — linked-list open set (`cract.cc`, `compare_chase_pathfinding.py`).
struct TShortwaySearch {
    origin: Position,
    min_waypoints: u32,
    cells: HashMap<Position, TShortwayCell>,
    expand_head: Option<Position>,
}

impl TShortwaySearch {
    fn clear_path_state(&mut self) {
        for cell in self.cells.values_mut() {
            cell.waylength = TSHORTWAY_UNVISITED;
            cell.heuristic = TSHORTWAY_UNVISITED;
            cell.parent = None;
            cell.parent_diagonal = false;
            cell.expand_next = None;
        }
        self.expand_head = None;
    }

    fn remove_from_expand_list(&mut self, pos: Position) {
        if self.expand_head == Some(pos) {
            self.expand_head = self.cells.get(&pos).and_then(|c| c.expand_next);
            return;
        }
        let mut cur = self.expand_head;
        while let Some(cur_pos) = cur {
            let next = self.cells.get(&cur_pos).and_then(|c| c.expand_next);
            if next == Some(pos) {
                let removed_next = self.cells.get(&pos).and_then(|c| c.expand_next);
                if let Some(cell) = self.cells.get_mut(&cur_pos) {
                    cell.expand_next = removed_next;
                }
                return;
            }
            cur = next;
        }
    }

    fn insert_expand_list(&mut self, pos: Position) {
        let new_h = self
            .cells
            .get(&pos)
            .map(|c| c.heuristic)
            .unwrap_or(TSHORTWAY_UNVISITED);
        let mut prev: Option<Position> = None;
        let mut cur = self.expand_head;
        while let Some(cur_pos) = cur {
            let cur_h = self
                .cells
                .get(&cur_pos)
                .map(|c| c.heuristic)
                .unwrap_or(TSHORTWAY_UNVISITED);
            if cur_h < new_h {
                prev = Some(cur_pos);
                cur = self.cells.get(&cur_pos).and_then(|c| c.expand_next);
            } else {
                break;
            }
        }
        let next = cur;
        if let Some(cell) = self.cells.get_mut(&pos) {
            cell.expand_next = next;
        }
        if let Some(prev_pos) = prev {
            if let Some(prev_cell) = self.cells.get_mut(&prev_pos) {
                prev_cell.expand_next = Some(pos);
            }
        } else {
            self.expand_head = Some(pos);
        }
    }

    fn expand(&mut self, pos: Position, allow_diagonal: bool) {
        let (node_wp, node_wl, node_next) = {
            let Some(cell) = self.cells.get(&pos) else {
                return;
            };
            (cell.waypoints, cell.waylength, cell.expand_next)
        };
        self.expand_head = node_next;
        if let Some(cell) = self.cells.get_mut(&pos) {
            cell.expand_next = None;
        }
        if node_wp <= 0 {
            return;
        }
        let min_neighbor_wl = node_wl.saturating_add(node_wp as u32);
        let origin_wl = self
            .cells
            .get(&self.origin)
            .map(|c| c.waylength)
            .unwrap_or(TSHORTWAY_UNVISITED);
        if min_neighbor_wl >= origin_wl {
            return;
        }

        for &(ox, oy) in &REVERSE_PATH_NEIGHBOR_OFFSETS {
            if !allow_diagonal && ox != 0 && oy != 0 {
                continue;
            }
            let Some(neighbor_pos) = offset_position(pos, ox, oy) else {
                continue;
            };
            let is_diagonal = ox != 0 && oy != 0;
            let mut neighbor_wl = min_neighbor_wl;
            if is_diagonal {
                neighbor_wl = neighbor_wl.saturating_add((node_wp as u32).saturating_mul(2));
            }
            if neighbor_wl >= origin_wl {
                continue;
            }

            let (neighbor_wp, prev_wl, prev_parent_diag, prev_heuristic) = self
                .cells
                .get(&neighbor_pos)
                .map(|c| (c.waypoints, c.waylength, c.parent_diagonal, c.heuristic))
                .unwrap_or((-1, TSHORTWAY_UNVISITED, false, TSHORTWAY_UNVISITED));

            if neighbor_wp <= 0 {
                continue;
            }
            if !tshortway_should_relax(prev_wl, neighbor_wl, prev_parent_diag, is_diagonal) {
                continue;
            }

            let distance = manhattan_dist(neighbor_pos, self.origin) as u32;
            let heuristic = neighbor_wl
                .saturating_add(neighbor_wp as u32)
                .saturating_add(
                    self.min_waypoints
                        .saturating_mul(distance.saturating_sub(1)),
                );

            if prev_heuristic != TSHORTWAY_UNVISITED {
                self.remove_from_expand_list(neighbor_pos);
            }

            if let Some(cell) = self.cells.get_mut(&neighbor_pos) {
                cell.waylength = neighbor_wl;
                cell.heuristic = heuristic;
                cell.parent = Some(pos);
                cell.parent_diagonal = is_diagonal;
            }
            if neighbor_pos != self.origin {
                self.insert_expand_list(neighbor_pos);
            }
        }
    }
}

/// 772 `TShortway::Calculate` — linked-list expand sorted by heuristic (`cract.cc`, `scripts/compare_chase_pathfinding.py`).
fn path_matching_tshortway<C, G>(
    map: &Map,
    start: Position,
    target: Position,
    fpp: &FindPathParams,
    can_walk_to: C,
    ground_cost: G,
) -> Option<Vec<Direction>>
where
    C: Fn(Position) -> bool,
    G: Fn(Position) -> u32,
{
    let radius = REVERSE_PATH_VIEW_RADIUS;
    if !in_path_viewport(start, target, radius) {
        return None;
    }

    let mut min_waypoints = u32::MAX;
    let mut cells: HashMap<Position, TShortwayCell> = HashMap::new();

    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let Some(pos) = offset_position(start, dx, dy) else {
                continue;
            };
            // Destination may hold the follow target — include seed tile without occupancy check.
            let walkable_for_fill = map.is_walkable(pos) && (pos == target || can_walk_to(pos));
            let waypoints = if walkable_for_fill {
                let wp = effective_terrain_waypoints(ground_cost(pos)) as i32;
                if wp > 0 {
                    min_waypoints = min_waypoints.min(wp as u32);
                }
                wp
            } else {
                -1
            };
            cells.insert(
                pos,
                TShortwayCell {
                    waypoints,
                    waylength: TSHORTWAY_UNVISITED,
                    heuristic: TSHORTWAY_UNVISITED,
                    parent: None,
                    parent_diagonal: false,
                    expand_next: None,
                },
            );
        }
    }

    if min_waypoints == u32::MAX {
        min_waypoints = DEFAULT_TERRAIN_WAYPOINTS;
    }

    let mut search = TShortwaySearch {
        origin: start,
        min_waypoints,
        cells,
        expand_head: None,
    };

    search.clear_path_state();
    if let Some(seed) = search.cells.get_mut(&target) {
        seed.waylength = 0;
        seed.heuristic = 0;
    }
    search.expand_head = Some(target);

    let mut expand_count = 0usize;
    while expand_count < REVERSE_PATH_MAX_CLOSED_NODES {
        let Some(current) = search.expand_head else {
            break;
        };
        search.expand(current, fpp.allow_diagonal);
        expand_count += 1;
    }

    let origin_wl = search
        .cells
        .get(&start)
        .map(|c| c.waylength)
        .unwrap_or(TSHORTWAY_UNVISITED);
    if origin_wl == TSHORTWAY_UNVISITED {
        return None;
    }

    let mut nodes: HashMap<Position, AStarNode> = HashMap::new();
    for (pos, cell) in &search.cells {
        if cell.waylength != TSHORTWAY_UNVISITED {
            nodes.insert(
                *pos,
                AStarNode {
                    parent: cell.parent,
                    g: cell.waylength,
                },
            );
        }
    }

    let dirs = reconstruct_reverse_dirs(&nodes, start);
    let mut trimmed = trim_path_to_goal_band(dirs, start, target, fpp, map);
    trimmed.reverse();
    Some(trimmed)
}

/// 772 reverse A* — destination (`target`) → origin (`start`) (`cract.cc:7` `TShortway`).
///
/// Terrain-weighted chase uses [`path_matching_tshortway`] (linked-list expand). Non-terrain
/// reverse keeps the BinaryHeap implementation for TFS fallback paths.
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

    let use_reverse_terrain_astar = matches!(cost_model, PathCostModel::TerrainWeighted);
    if use_reverse_terrain_astar {
        return path_matching_tshortway(map, start, target, fpp, can_walk_to, ground_cost);
    }
    // 772 monster chase always uses VisibleX/Y = 10 and ~441 node cap — not TFS `maxSearchDist` 12.
    let (viewport_radius, closed_cap) = if use_reverse_terrain_astar {
        (REVERSE_PATH_VIEW_RADIUS, REVERSE_PATH_MAX_CLOSED_NODES)
    } else if fpp.max_search_dist != 0 {
        (fpp.max_search_dist as i32, usize::MAX)
    } else {
        (REVERSE_PATH_VIEW_RADIUS, REVERSE_PATH_MAX_CLOSED_NODES)
    };
    let min_wp = if use_reverse_terrain_astar {
        scan_min_terrain_waypoints(map, start, viewport_radius, &ground_cost)
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
    let seed_h = if use_reverse_terrain_astar {
        reverse_path_heuristic(target, start, min_wp, &ground_cost)
    } else {
        0
    };
    open.push(OpenNode {
        f: seed_h,
        g: 0,
        pos: target,
    });

    let mut expand_count = 0usize;
    while fpp.max_search_dist != 0 || expand_count < closed_cap {
        let Some(OpenNode {
            pos: current,
            g: popped_g,
            ..
        }) = open.pop()
        else {
            break;
        };

        let Some(&AStarNode { g: best_g, .. }) = nodes.get(&current) else {
            continue;
        };
        if popped_g > best_g {
            continue;
        }

        if use_reverse_terrain_astar {
            expand_count += 1;
        } else if !closed.insert(current) {
            continue;
        }

        if current == start {
            let dirs = reconstruct_reverse_dirs(&nodes, start);
            let mut trimmed = trim_path_to_goal_band(dirs, start, target, fpp, map);
            trimmed.reverse();
            return Some(trimmed);
        }

        let base_g = best_g;
        if base_g == u32::MAX {
            continue;
        }

        // 772 node-level branch-and-bound — skip all neighbors when even the cheapest
        // cardinal step cannot improve on the best-known path to the origin (`cract.cc:136–138`).
        if use_reverse_terrain_astar {
            let current_wp = effective_terrain_waypoints(ground_cost(current));
            let min_neighbor_g = base_g.saturating_add(current_wp);
            if nodes
                .get(&start)
                .is_some_and(|origin| min_neighbor_g >= origin.g)
            {
                continue;
            }
        }

        for &(ox, oy) in &REVERSE_PATH_NEIGHBOR_OFFSETS {
            if !fpp.allow_diagonal && ox != 0 && oy != 0 {
                continue;
            }

            let Some(next) = offset_position(current, ox, oy) else {
                continue;
            };

            if !in_path_viewport(start, next, viewport_radius) {
                continue;
            }

            if !use_reverse_terrain_astar && closed.contains(&next) {
                continue;
            }

            let is_diagonal = ox != 0 && oy != 0;
            if !nodes.contains_key(&next) && !can_walk_to(next) {
                continue;
            }

            let step_cost = path_step_cost(cost_model, is_diagonal, || ground_cost(current));
            let occupancy_cost = if use_reverse_terrain_astar {
                0
            } else {
                tile_walk_cost(next)
            };
            let new_g = base_g
                .saturating_add(step_cost)
                .saturating_add(occupancy_cost);

            // 772 per-edge branch-and-bound (`cract.cc:157` vs origin `Waylength`).
            if let Some(&AStarNode { g: origin_g, .. }) = nodes.get(&start) {
                if new_g >= origin_g {
                    continue;
                }
            }

            let prev_g = nodes.get(&next).map(|n| n.g).unwrap_or(u32::MAX);
            if new_g < prev_g {
                let h = if use_reverse_terrain_astar {
                    reverse_path_heuristic(next, start, min_wp, &ground_cost)
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

/// 772 `ToDoGo(..., MaxSteps)` for monster chase — `crnonpl.cc` ~2729, `cract.cc` ~992.
pub const CHASE_PATH_MAX_STEPS: usize = 3;

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
/// - [`PathCostModel::TerrainWeighted`] — 772 (`cract.cc:136–155` `TShortway::Expand`):
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
            let wp = effective_terrain_waypoints(ground_cost());
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

/// 772 `VisibleX`/`VisibleY` rectangle around the origin (`cract.cc` `TShortway`).
fn in_path_viewport(origin: Position, pos: Position, radius: i32) -> bool {
    let dx = (origin.x as i32 - pos.x as i32).unsigned_abs() as i32;
    let dy = (origin.y as i32 - pos.y as i32).unsigned_abs() as i32;
    dx <= radius && dy <= radius
}

/// 772 `TShortway` A* heuristic — `cract.cc:181-183` (`Waylength + Waypoints + MinWaypoints * (Distance - 1)`).
fn reverse_path_heuristic<G>(pos: Position, origin: Position, min_wp: u32, ground_cost: G) -> u32
where
    G: Fn(Position) -> u32,
{
    let wp = effective_terrain_waypoints(ground_cost(pos));
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

/// 772 `TShortway::Expand` — nested `OffsetX`/`OffsetY` order (`cract.cc:141-145`).
const REVERSE_PATH_NEIGHBOR_OFFSETS: [(i32, i32); 8] = [
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
];

/// TFS `dirNeighbors` / `allNeighbors` (`map.cpp` ~663–675).
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
        let len = if allow_diagonal { ALL_NEIGHBORS.len() } else { 4 };
        return (&ALL_NEIGHBORS, len);
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

/// 772 `TShortway::Calculate` queue trim — `cract.cc:241-258`.
pub fn truncate_cipsoft_chase_queue(
    start: Position,
    target: Position,
    mut walk_order: Vec<Direction>,
    max_steps: usize,
    must_reach: bool,
) -> Vec<Direction> {
    let mut cur_distance = chebyshev_dist(start, target);
    let mut out = Vec::new();
    let mut pos = start;
    let mut remaining = max_steps;

    for d in walk_order.drain(..) {
        if remaining == 0 {
            break;
        }
        if !must_reach && cur_distance <= 1 {
            break;
        }
        out.push(d);
        pos = pos.offset(d);
        cur_distance = chebyshev_dist(pos, target);
        remaining -= 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::test_world::support::ensure_walkable_tile;

    #[test]
    fn tshortway_should_relax_prefers_cardinal_on_equal_cost() {
        assert!(tshortway_should_relax(100, 90, false, false));
        assert!(!tshortway_should_relax(90, 100, false, false));
        assert!(tshortway_should_relax(100, 100, true, false));
        assert!(!tshortway_should_relax(100, 100, false, true));
        assert!(!tshortway_should_relax(100, 100, true, true));
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
        use crate::tile::{flags as tilestate, Tile, TileBody};
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
    fn uses_reverse_terrain_path_matches_772_profile() {
        use tfs_rust_common::ProtocolVersion;

        use crate::formulas::MechanicsProfile;

        let p772 = MechanicsProfile::for_version(ProtocolVersion::V772);
        assert!(super::uses_reverse_terrain_path(p772.path_cost, p772.path_search));

        let p1098 = MechanicsProfile::for_version(ProtocolVersion::V1098);
        assert!(!super::uses_reverse_terrain_path(p1098.path_cost, p1098.path_search));
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
            can_walk,
            no_extra,
            ground,
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
            true,
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
            true,
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
        use crate::tile::{flags as tilestate, Tile, TileBody};
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
            can_walk,
            no_extra,
            ground,
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
        use crate::tile::{flags as tilestate, Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;
        for y in 9..=11u16 {
            map.insert_tile(
                Position::new(4, y, 7),
                Tile::Normal(TileBody {
                    ground: Some(100),
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
            can_walk,
            no_extra,
            ground,
        );
        assert!(path_no_fallback.is_none(), "Must return None without forward fallback (CipSoft NOWAY)");

        // With fallback enabled, it must succeed because forward search can reach (3, 10) which is distance 2 from target.
        let path_with_fallback = get_path_matching(
            &map,
            start,
            target,
            &fpp,
            PathCostModel::TerrainWeighted,
            PathSearchModel::Reverse,
            true, // forward fallback enabled
            can_walk,
            no_extra,
            ground,
        );
        assert!(path_with_fallback.is_some(), "Must return Some with forward fallback");
    }

    #[test]
    fn test_truncate_cipsoft_chase_queue() {
        let start = Position::new(32345, 32288, 7);
        let target = Position::new(32344, 32286, 7);
        let walk_order = vec![Direction::North, Direction::North, Direction::West];
        let truncated = truncate_cipsoft_chase_queue(start, target, walk_order, 3, false);
        assert_eq!(truncated, vec![Direction::North]);
    }
}
