//! Grid line-of-sight (same floor).
// C++ reference: `map.cpp` `Map::isSightClear`, `canThrowObjectTo`.

use tfs_rust_common::Position;

use super::Map;

/// Integer Bresenham line on the (x, y) grid; includes endpoints.
pub fn walk_grid_line(a: Position, b: Position) -> Vec<Position> {
    if a.z != b.z {
        return Vec::new();
    }
    let mut out = Vec::new();
    let x0 = a.x as i32;
    let y0 = a.y as i32;
    let x1 = b.x as i32;
    let y1 = b.y as i32;
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = x0;
    let mut y = y0;
    loop {
        out.push(Position {
            x: x as u16,
            y: y as u16,
            z: a.z,
        });
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
    out
}

impl Map {
    /// C++ `Map::isSightClear` — walks the grid and checks blocking tiles.
    pub fn is_sight_clear(&self, from: Position, to: Position) -> bool {
        if from.z != to.z {
            return false;
        }
        for p in walk_grid_line(from, to) {
            if p == from || p == to {
                continue;
            }
            if self.blocks_sight(p) {
                return false;
            }
        }
        true
    }

    /// Throw / shoot LOS (stricter range checks can be added in combat phase).
    pub fn can_throw_to(&self, from: Position, to: Position, max_range: u32) -> bool {
        if from.z != to.z {
            return false;
        }
        if from.distance_to(&to) > max_range {
            return false;
        }
        self.is_sight_clear(from, to)
    }
}
