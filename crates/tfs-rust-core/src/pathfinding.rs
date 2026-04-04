//! A* pathfinding on the map grid (8 directions).
// C++ reference: `map.cpp` `AStarNodes`, `PathFinding`.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use tfs_rust_common::{enums::Direction, Position};

use crate::map::Map;

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

/// Returns step directions from `from` to `to`, or `None` if unreachable.
/// Costs: cardinal 10, diagonal 14 (common integer sqrt2 approximation).
pub fn pathfind(map: &Map, from: Position, to: Position) -> Option<Vec<Direction>> {
    if from.z != to.z {
        return None;
    }
    if from == to {
        return Some(Vec::new());
    }

    let mut open: BinaryHeap<OpenNode> = BinaryHeap::new();
    let mut came: HashMap<Position, (Position, Direction)> = HashMap::new();
    let mut g_score: HashMap<Position, u32> = HashMap::new();

    let h = heuristic(from, to);
    g_score.insert(from, 0);
    open.push(OpenNode { f: h, pos: from });

    const DIRS: [(i32, i32, Direction, u32); 8] = [
        (0, -1, Direction::North, 10),
        (1, 0, Direction::East, 10),
        (0, 1, Direction::South, 10),
        (-1, 0, Direction::West, 10),
        (-1, 1, Direction::SouthWest, 14),
        (1, 1, Direction::SouthEast, 14),
        (-1, -1, Direction::NorthWest, 14),
        (1, -1, Direction::NorthEast, 14),
    ];

    while let Some(OpenNode { pos: current, .. }) = open.pop() {
        if current == to {
            return Some(reconstruct(&came, from, to));
        }
        let base = *g_score.get(&current).unwrap_or(&u32::MAX);
        if base == u32::MAX {
            continue;
        }

        for (dx, dy, dir, step_cost) in DIRS {
            let nx = current.x as i32 + dx;
            let ny = current.y as i32 + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            let next = Position {
                x: nx as u16,
                y: ny as u16,
                z: current.z,
            };
            if !map.is_walkable(next) {
                continue;
            }
            // C++ often blocks diagonal corner cutting if adjacent cardinals blocked.
            if step_cost == 14 {
                let ox = current.x as i32;
                let oy = current.y as i32;
                let a = Position {
                    x: (ox + dx) as u16,
                    y: current.y,
                    z: current.z,
                };
                let b = Position {
                    x: current.x,
                    y: (oy + dy) as u16,
                    z: current.z,
                };
                if !map.is_walkable(a) || !map.is_walkable(b) {
                    continue;
                }
            }

            let tentative = base + step_cost;
            let prev = g_score.get(&next).copied().unwrap_or(u32::MAX);
            if tentative < prev {
                came.insert(next, (current, dir));
                g_score.insert(next, tentative);
                let f = tentative + heuristic(next, to);
                open.push(OpenNode { f, pos: next });
            }
        }
    }

    None
}

fn heuristic(a: Position, b: Position) -> u32 {
    let dx = (a.x as i32 - b.x as i32).unsigned_abs();
    let dy = (a.y as i32 - b.y as i32).unsigned_abs();
    10 * dx.max(dy)
}

fn reconstruct(
    came: &HashMap<Position, (Position, Direction)>,
    start: Position,
    goal: Position,
) -> Vec<Direction> {
    let mut out = Vec::new();
    let mut cur = goal;
    while cur != start {
        if let Some(&(prev, dir)) = came.get(&cur) {
            out.push(dir);
            cur = prev;
        } else {
            break;
        }
    }
    out.reverse();
    out
}
