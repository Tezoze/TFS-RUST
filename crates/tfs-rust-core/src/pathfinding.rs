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
/// 772 monster path viewport half-extent — `VisibleX`/`VisibleY` = 10 (`cract.cc:1093` `TShortway`).
pub const REVERSE_PATH_VIEW_RADIUS: i32 = 10;
/// 772 player path viewport half-extent — `VisibleX`/`VisibleY` = 7 for `Type == PLAYER`
/// (`cract.cc:1093-1094`: `int VisibleX = (this->Type == PLAYER) ? 7 : 10;`).
pub const PLAYER_PATH_VIEW_RADIUS: i32 = 7;

/// Closed-node cap for a 772 `TShortway` viewport — `(2*radius+1)^2` (`cract.cc` `TShortway`).
/// 441 for monsters (radius 10), 225 for players (radius 7).
fn viewport_closed_cap(radius: i32) -> usize {
    let side = (2 * radius + 1).max(1) as usize;
    side.saturating_mul(side)
}
/// 772 default BANK `Waypoints` when unset — matches `ground_speed_for_item` / `NotifyGo` default.
pub const DEFAULT_TERRAIN_WAYPOINTS: u32 = 150;

/// Effective per-tile waypoint for `TShortway` — OTB `ITEM_ATTR_SPEED` / 772 `WAYPOINTS`.
///
/// `0` (missing OTB speed) maps to [`DEFAULT_TERRAIN_WAYPOINTS`], not `1`. Matches
/// `NotifyGo` / `ground_speed_for_item` and passable FillMap defaults (`fillmap_terrain_waypoints_at`).
/// Unpass grounds are excluded before this helper (FillMap / MovePossible).
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
    Partial {
        dist: i32,
    },
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

/// 772 `TShortway::FillMap` initial `MinWaypoints` before viewport scan (`cract.cc:81`).
const FILLMAP_MIN_WAYPOINTS_SEED: u32 = 1000;

/// TFS `Map::getPathMatching` / 772 `TShortway` — creature-aware via callbacks.
///
/// `search` selects expansion direction (1098 forward / 772 reverse). Edge costs come from
/// `cost_model` (B2): fixed 10/25 for TFS, terrain waypoints + diagonal ×3 for CipSoft.
/// `view_radius` is the 772 `TShortway` viewport half-extent — 7 for players, 10 for monsters
/// (`cract.cc:1093-1094`). Ignored by 1098 forward search.
#[allow(clippy::too_many_arguments)]
pub fn get_path_matching<C, T, G>(
    map: &Map,
    start: Position,
    target: Position,
    fpp: &FindPathParams,
    cost_model: PathCostModel,
    search: PathSearchModel,
    forward_fallback: bool,
    view_radius: i32,
    can_walk_to: C,
    tile_walk_cost: T,
    ground_cost: G,
    scratch: Option<&mut TShortwayScratch>,
) -> Option<Vec<Direction>>
where
    C: Fn(Position) -> bool,
    T: Fn(Position) -> u32,
    G: Fn(Position) -> u32,
{
    use std::rc::Rc;
    let ground = Rc::new(ground_cost);
    let ground_for_cost = Rc::clone(&ground);
    let ground_for_fill = ground;
    get_path_matching_with_fill(
        map,
        start,
        target,
        fpp,
        cost_model,
        search,
        forward_fallback,
        view_radius,
        can_walk_to,
        tile_walk_cost,
        move |pos| ground_for_cost(pos),
        move |pos| {
            let raw = ground_for_fill(pos);
            if raw == 0 {
                -1
            } else {
                raw as i32
            }
        },
        scratch,
    )
}

/// Like [`get_path_matching`] but supplies 772 `TShortway::FillMap` terrain weights (`cract.cc:89-103`).
///
/// `view_radius` is the 772 `TShortway` viewport half-extent — 7 for players, 10 for monsters
/// (`cract.cc:1093-1094`). Ignored by 1098 forward search.
#[allow(clippy::too_many_arguments)]
pub fn get_path_matching_with_fill<C, T, G, F>(
    map: &Map,
    start: Position,
    target: Position,
    fpp: &FindPathParams,
    cost_model: PathCostModel,
    search: PathSearchModel,
    forward_fallback: bool,
    view_radius: i32,
    can_walk_to: C,
    tile_walk_cost: T,
    ground_cost: G,
    fill_waypoints: F,
    scratch: Option<&mut TShortwayScratch>,
) -> Option<Vec<Direction>>
where
    C: Fn(Position) -> bool,
    T: Fn(Position) -> u32,
    G: Fn(Position) -> u32,
    F: Fn(Position) -> i32,
{
    let mut local_scratch = TShortwayScratch::new();
    let scratch = scratch.unwrap_or(&mut local_scratch);
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
                view_radius,
                &can_walk_to,
                &tile_walk_cost,
                &ground_cost,
                &fill_waypoints,
                scratch,
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

    nodes.insert(start, AStarNode { parent: None, g: 0 });
    open.push(OpenNode {
        f: 0,
        g: 0,
        pos: start,
    });

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
#[derive(Clone)]
struct TShortwayCell {
    waypoints: i32,
    /// Signed path cost — `cract.cc` `TShortwayPoint::Waylength` (`INT_MAX` = unvisited).
    waylength: i32,
    /// Signed expand key — `cract.cc` `TShortwayPoint::Heuristic` (`INT_MAX` = not queued).
    heuristic: i32,
    parent: Option<u16>,
    /// Incoming edge to this cell was diagonal — used for equal-cost cardinal tie-break.
    parent_diagonal: bool,
    expand_next: Option<u16>,
    /// Cell is inside the current outer matrix and maps to a valid world position.
    in_matrix: bool,
    /// Generation stamp for path-search fields — avoids clearing 529 cells per search.
    path_gen: u32,
}

fn tshortway_cell_default() -> TShortwayCell {
    TShortwayCell {
        waypoints: -1,
        waylength: TSHORTWAY_UNVISITED_WL,
        heuristic: TSHORTWAY_UNVISITED_H,
        parent: None,
        parent_diagonal: false,
        expand_next: None,
        in_matrix: false,
        path_gen: 0,
    }
}

/// Reusable 772 `TShortway` viewport buffer — **game thread only**.
///
/// C++ reference: `cract.cc` `TShortway` matrix + linked-list expand set.
pub struct TShortwayScratch {
    cells: Box<[TShortwayCell]>,
    search_gen: u32,
}

impl Default for TShortwayScratch {
    fn default() -> Self {
        Self::new()
    }
}

impl TShortwayScratch {
    pub fn new() -> Self {
        Self {
            cells: vec![tshortway_cell_default(); TSHORTWAY_MAX_CELLS].into_boxed_slice(),
            search_gen: 0,
        }
    }

    fn begin_search(&mut self) -> u32 {
        self.search_gen = self.search_gen.wrapping_add(1);
        self.search_gen
    }
}

const TSHORTWAY_UNVISITED_WL: i32 = i32::MAX;
const TSHORTWAY_UNVISITED_H: i32 = i32::MAX;

/// Monster viewport outer half-extent (`Visible+1` with `Visible=10`) — max dense buffer size.
const TSHORTWAY_MAX_OUTER: i32 = REVERSE_PATH_VIEW_RADIUS + 1;
const TSHORTWAY_MAX_SIDE: usize = (2 * TSHORTWAY_MAX_OUTER + 1) as usize;
const TSHORTWAY_MAX_CELLS: usize = TSHORTWAY_MAX_SIDE * TSHORTWAY_MAX_SIDE;

/// Prefer cardinal when two relaxations reach the same `waylength` (`cract.cc` strict `<` keeps
/// first-seen; linked-list expand order can still tie — cardinals match live 772 chase traces).
#[cfg(test)]
fn tshortway_should_relax(prev_waylength: u32, new_waylength: u32) -> bool {
    new_waylength < prev_waylength
}

fn tshortway_cell_idx(dx: i32, dy: i32, outer: i32) -> Option<usize> {
    if dx < -outer || dx > outer || dy < -outer || dy > outer {
        return None;
    }
    let side = (2 * outer + 1) as usize;
    Some(((dy + outer) as usize) * side + ((dx + outer) as usize))
}

fn tshortway_rel(origin: Position, pos: Position) -> (i32, i32) {
    (pos.x as i32 - origin.x as i32, pos.y as i32 - origin.y as i32)
}

/// 772 `TShortway` search state — dense matrix + linked-list open set (`cract.cc`).
struct TShortwaySearch<'a> {
    origin: Position,
    outer: i32,
    origin_idx: u16,
    min_waypoints: u32,
    cells: &'a mut [TShortwayCell],
    search_gen: u32,
    expand_head: Option<u16>,
}

impl<'a> TShortwaySearch<'a> {
    fn cell(&self, idx: u16) -> &TShortwayCell {
        &self.cells[idx as usize]
    }

    fn cell_mut(&mut self, idx: u16) -> &mut TShortwayCell {
        &mut self.cells[idx as usize]
    }

    fn cell_waylength(&self, idx: u16) -> i32 {
        let cell = self.cell(idx);
        if cell.path_gen == self.search_gen {
            cell.waylength
        } else {
            TSHORTWAY_UNVISITED_WL
        }
    }

    fn cell_heuristic(&self, idx: u16) -> i32 {
        let cell = self.cell(idx);
        if cell.path_gen == self.search_gen {
            cell.heuristic
        } else {
            TSHORTWAY_UNVISITED_H
        }
    }

    fn touch_path_cell(&mut self, idx: u16) -> &mut TShortwayCell {
        let gen = self.search_gen;
        let cell = self.cell_mut(idx);
        // New search generation: reset path fields. Leaving stale `expand_next` /
        // `parent` from a prior search corrupts the linked-list open set — common
        // after floor-change wake storms where many monsters repath on one scratch.
        if cell.path_gen != gen {
            cell.path_gen = gen;
            cell.waylength = TSHORTWAY_UNVISITED_WL;
            cell.heuristic = TSHORTWAY_UNVISITED_H;
            cell.parent = None;
            cell.parent_diagonal = false;
            cell.expand_next = None;
        }
        cell
    }

    fn idx_of(&self, pos: Position) -> Option<u16> {
        let (dx, dy) = tshortway_rel(self.origin, pos);
        tshortway_cell_idx(dx, dy, self.outer).map(|i| i as u16)
    }

    fn pos_of(&self, idx: u16) -> Option<Position> {
        let side = (2 * self.outer + 1) as i32;
        let i = idx as i32;
        let dx = (i % side) - self.outer;
        let dy = (i / side) - self.outer;
        offset_position(self.origin, dx, dy)
    }

    fn remove_from_expand_list(&mut self, idx: u16) {
        if self.expand_head == Some(idx) {
            self.expand_head = self.cell(idx).expand_next;
            return;
        }
        let mut cur = self.expand_head;
        while let Some(cur_idx) = cur {
            let next = self.cell(cur_idx).expand_next;
            if next == Some(idx) {
                let removed_next = self.cell(idx).expand_next;
                self.cell_mut(cur_idx).expand_next = removed_next;
                return;
            }
            cur = next;
        }
    }

    fn insert_expand_list(&mut self, idx: u16) {
        let new_h = self.cell_heuristic(idx);
        let mut prev: Option<u16> = None;
        let mut cur = self.expand_head;
        while let Some(cur_idx) = cur {
            // Stale link from a prior search — treat as end of list.
            if self.cell(cur_idx).path_gen != self.search_gen {
                cur = None;
                break;
            }
            let cur_h = self.cell_heuristic(cur_idx);
            if cur_h < new_h {
                prev = Some(cur_idx);
                cur = self.cell(cur_idx).expand_next;
            } else {
                break;
            }
        }
        let next = cur;
        self.cell_mut(idx).expand_next = next;
        if let Some(prev_idx) = prev {
            self.cell_mut(prev_idx).expand_next = Some(idx);
        } else {
            self.expand_head = Some(idx);
        }
    }

    fn expand(&mut self, idx: u16, allow_diagonal: bool) {
        let (node_wp, node_wl, node_next, node_pos) = {
            let cell = self.cell(idx);
            if !cell.in_matrix {
                return;
            }
            let Some(pos) = self.pos_of(idx) else {
                return;
            };
            (cell.waypoints, cell.waylength, cell.expand_next, pos)
        };
        self.expand_head = node_next;
        self.cell_mut(idx).expand_next = None;
        // `cract.cc:137-140` — no `Waypoints<=0` early out; dest seed may hold `Waypoints=-1`.
        let min_neighbor_wl = node_wl + node_wp;
        let origin_wl_i = self.cell_waylength(self.origin_idx);
        if min_neighbor_wl >= origin_wl_i {
            return;
        }

        for &(ox, oy) in &REVERSE_PATH_NEIGHBOR_OFFSETS {
            if !allow_diagonal && ox != 0 && oy != 0 {
                continue;
            }
            let Some(neighbor_pos) = offset_position(node_pos, ox, oy) else {
                continue;
            };
            let Some(neighbor_idx) = self.idx_of(neighbor_pos) else {
                continue;
            };
            if !self.cell(neighbor_idx).in_matrix {
                continue;
            }
            let is_diagonal = ox != 0 && oy != 0;
            let mut neighbor_wl = min_neighbor_wl;
            if is_diagonal {
                neighbor_wl += node_wp * 2;
            }
            if neighbor_wl >= origin_wl_i {
                continue;
            }

            let neighbor_wp = self.cell(neighbor_idx).waypoints;
            let prev_wl = self.cell_waylength(neighbor_idx);
            let prev_heuristic = self.cell_heuristic(neighbor_idx);

            // `cract.cc:158-202` — relax any neighbor with a shorter waylength; only enqueue
            // expand when `Waypoints != -1` and not the origin cell.
            if neighbor_wl >= prev_wl {
                continue;
            }

            {
                let cell = self.touch_path_cell(neighbor_idx);
                cell.waylength = neighbor_wl;
                cell.parent = Some(idx);
                cell.parent_diagonal = is_diagonal;
            }

            if neighbor_wp <= 0 || neighbor_idx == self.origin_idx {
                continue;
            }

            // `cract.cc:181-184` — signed sum; negative `Waylength` is valid during reverse expand.
            let (ndx, ndy) = tshortway_rel(self.origin, neighbor_pos);
            let distance = ndx.unsigned_abs() as i32 + ndy.unsigned_abs() as i32;
            let heuristic =
                neighbor_wl + neighbor_wp + (self.min_waypoints as i32) * (distance - 1);

            if prev_heuristic != TSHORTWAY_UNVISITED_H {
                self.remove_from_expand_list(neighbor_idx);
            }

            {
                let cell = self.touch_path_cell(neighbor_idx);
                cell.heuristic = heuristic;
            }
            self.insert_expand_list(neighbor_idx);
        }
    }
}

/// 772 `TShortway::Calculate` — linked-list expand sorted by heuristic (`cract.cc`, `scripts/compare_chase_pathfinding.py`).
///
/// `view_radius` is the `TShortway` viewport half-extent — 7 for players, 10 for monsters
/// (`cract.cc:1093-1094`: `int VisibleX = (this->Type == PLAYER) ? 7 : 10;`).
fn path_matching_tshortway<C, F>(
    scratch: &mut TShortwayScratch,
    _map: &Map,
    start: Position,
    target: Position,
    fpp: &FindPathParams,
    view_radius: i32,
    can_walk_to: C,
    fill_waypoints: F,
) -> Option<Vec<Direction>>
where
    C: Fn(Position) -> bool,
    F: Fn(Position) -> i32,
{
    let radius = view_radius;
    if !in_path_viewport(start, target, radius) {
        return None;
    }

    // `cract.cc:79-114` — matrix spans `±(Visible+1)`; only inner `±Visible` gets terrain fill.
    let outer_radius = radius.saturating_add(1);
    debug_assert!(outer_radius <= TSHORTWAY_MAX_OUTER);
    let side = (2 * outer_radius + 1) as usize;
    let cell_count = side * side;

    let search_gen = scratch.begin_search();
    let cells = &mut scratch.cells;
    let mut min_waypoints = FILLMAP_MIN_WAYPOINTS_SEED;

    for dy in -outer_radius..=outer_radius {
        for dx in -outer_radius..=outer_radius {
            let Some(idx) = tshortway_cell_idx(dx, dy, outer_radius) else {
                continue;
            };
            let cell = &mut cells[idx];
            cell.in_matrix = false;
            cell.waypoints = -1;
            // Drop stale open-set links from prior searches on this scratch.
            cell.expand_next = None;
            cell.parent = None;
            cell.parent_diagonal = false;
            let Some(pos) = offset_position(start, dx, dy) else {
                continue;
            };
            cell.in_matrix = true;
            let _ = pos;
        }
    }

    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let Some(idx) = tshortway_cell_idx(dx, dy, outer_radius) else {
                continue;
            };
            if !cells[idx].in_matrix {
                continue;
            }
            let Some(pos) = offset_position(start, dx, dy) else {
                continue;
            };
            // `TShortway::FillMap` — stack-head BANK `WAYPOINTS` then `MovePossible(Execute=false)` (`cract.cc:89-103`).
            let mut waypoints = fill_waypoints(pos);
            if waypoints > 0 && !can_walk_to(pos) {
                waypoints = -1;
            }
            if waypoints > 0 {
                min_waypoints = min_waypoints.min(waypoints as u32);
            }
            cells[idx].waypoints = waypoints;
        }
    }

    // C++ leaves `MinWaypoints = 1000` when no positive tile was scanned (`cract.cc:81`).
    // Do not substitute `DEFAULT_TERRAIN_WAYPOINTS` (150) — that is NotifyGo / ground_speed only.

    let origin_idx = tshortway_cell_idx(0, 0, outer_radius)? as u16;
    let target_idx = {
        let (tdx, tdy) = tshortway_rel(start, target);
        tshortway_cell_idx(tdx, tdy, outer_radius)? as u16
    };

    let mut search = TShortwaySearch {
        origin: start,
        outer: outer_radius,
        origin_idx,
        min_waypoints,
        cells,
        search_gen,
        expand_head: None,
    };
    // Only the active `cell_count` slots matter; rest stay unused for smaller viewports.
    let _ = cell_count;

    {
        let seed = search.touch_path_cell(target_idx);
        seed.waylength = 0;
        seed.heuristic = 0;
    }
    search.expand_head = Some(target_idx);

    // C++ runs until expand list is empty; cap is a safety guard only.
    // 772 viewport tile budget scales with `VisibleX/Y` — 441 for monsters (10), 225 for players (7).
    let closed_cap = viewport_closed_cap(radius);
    let mut expand_count = 0usize;
    while search.expand_head.is_some() {
        if expand_count >= closed_cap.saturating_mul(2) {
            break;
        }
        let current = search.expand_head.unwrap();
        search.expand(current, fpp.allow_diagonal);
        expand_count += 1;
    }

    if search.cell_waylength(origin_idx) == TSHORTWAY_UNVISITED_WL {
        return None;
    }

    // C++ `TShortway::Calculate` walks the predecessor chain directly — no goal-band trim.
    Some(reconstruct_reverse_dirs_dense(&search, origin_idx))
}

fn reconstruct_reverse_dirs_dense(search: &TShortwaySearch<'_>, origin_idx: u16) -> Vec<Direction> {
    let mut dir_list = Vec::new();
    let mut cur_idx = origin_idx;
    while let Some(next_idx) = {
        let cell = search.cell(cur_idx);
        if cell.path_gen == search.search_gen {
            cell.parent
        } else {
            None
        }
    } {
        let Some(cur) = search.pos_of(cur_idx) else {
            break;
        };
        let Some(next) = search.pos_of(next_idx) else {
            break;
        };
        dir_list.push(walk_queue_direction(cur, next));
        cur_idx = next_idx;
    }
    dir_list
}

/// 772 reverse A* — destination (`target`) → origin (`start`) (`cract.cc:7` `TShortway`).
///
/// Terrain-weighted chase uses [`path_matching_tshortway`] (linked-list expand). Non-terrain
/// reverse keeps the BinaryHeap implementation for TFS fallback paths.
/// `view_radius` is the `TShortway` viewport half-extent — 7 for players, 10 for monsters
/// (`cract.cc:1093-1094`).
#[allow(clippy::too_many_arguments)]
fn path_matching_reverse<C, T, G, F>(
    map: &Map,
    start: Position,
    target: Position,
    fpp: &FindPathParams,
    cost_model: PathCostModel,
    view_radius: i32,
    can_walk_to: C,
    tile_walk_cost: T,
    ground_cost: G,
    fill_waypoints: F,
    scratch: &mut TShortwayScratch,
) -> Option<Vec<Direction>>
where
    C: Fn(Position) -> bool,
    T: Fn(Position) -> u32,
    G: Fn(Position) -> u32,
    F: Fn(Position) -> i32,
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
        return path_matching_tshortway(
            scratch,
            map,
            start,
            target,
            fpp,
            view_radius,
            can_walk_to,
            fill_waypoints,
        );
    }
    // 772 reverse viewport + tile budget scale with `VisibleX/Y` — 10/441 for monsters, 7/225 for players.
    let (viewport_radius, closed_cap) = if fpp.max_search_dist != 0 {
        (fpp.max_search_dist as i32, usize::MAX)
    } else {
        (view_radius, viewport_closed_cap(view_radius))
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
    nodes.insert(target, AStarNode { parent: None, g: 0 });
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
            let trimmed = trim_path_to_goal_band(dirs, start, target, fpp, map);
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
fn path_in_search_box(
    start: Position,
    test: Position,
    target: Position,
    fpp: &FindPathParams,
) -> bool {
    if fpp.full_path_search {
        let dx = (test.x as i32 - target.x as i32).abs();
        let dy = (test.y as i32 - target.y as i32).abs();
        return dx <= fpp.max_target_dist && dy <= fpp.max_target_dist;
    }

    let offset_x = start.x as i32 - target.x as i32;
    let offset_y = start.y as i32 - target.y as i32;

    let dx_max = if offset_x >= 0 {
        fpp.max_target_dist
    } else {
        0
    };
    if (test.x as i32) > (target.x as i32) + dx_max {
        return false;
    }
    let dx_min = if offset_x <= 0 {
        fpp.max_target_dist
    } else {
        0
    };
    if (test.x as i32) < (target.x as i32) - dx_min {
        return false;
    }

    let dy_max = if offset_y >= 0 {
        fpp.max_target_dist
    } else {
        0
    };
    if (test.y as i32) > (target.y as i32) + dy_max {
        return false;
    }
    let dy_min = if offset_y <= 0 {
        fpp.max_target_dist
    } else {
        0
    };
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
fn path_step_cost(
    model: PathCostModel,
    is_diagonal: bool,
    ground_cost: impl FnOnce() -> u32,
) -> u32 {
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
    (a.x as i32 - b.x as i32).unsigned_abs() as i32
        + (a.y as i32 - b.y as i32).unsigned_abs() as i32
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
        let len = if allow_diagonal {
            ALL_NEIGHBORS.len()
        } else {
            4
        };
        return (&ALL_NEIGHBORS, len);
    };

    let dx = prev.x as i32 - current.x as i32;
    let dy = prev.y as i32 - current.y as i32;
    let idx = if dy == 0 {
        if dx == -1 {
            3
        } else {
            1
        }
    } else if !allow_diagonal || dx == 0 {
        if dy == -1 {
            0
        } else {
            2
        }
    } else if dy == -1 {
        if dx == -1 {
            6
        } else {
            7
        }
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
fn reconstruct_reverse_dirs(
    nodes: &HashMap<Position, AStarNode>,
    origin: Position,
) -> Vec<Direction> {
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

/// 772 `TShortway::Calculate` Go-queue trim — `cract.cc:282-301`.
///
/// Stops when `MaxSteps` is exhausted, or when `!MustReach && Chebyshev ≤ 1`
/// (`CurDistance > 1` in C++). Dist keep-band is **not** encoded here — callers
/// pass `MaxSteps = Distance − target_distance` (`crnonpl.cc` dist chase).
pub fn truncate_tshortway_go_queue(
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
        // C++ `MustReach || CurDistance > 1` — always adjacent stop, never a keep-band.
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
#[path = "pathfinding_tests.rs"]
mod tests;
