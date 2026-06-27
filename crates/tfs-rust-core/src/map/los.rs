//! Grid line-of-sight (same floor).
// C++ reference: `map.cpp` `Map::isSightClear`, `canThrowObjectTo`.

use tfs_rust_common::Position;

use super::Map;
use crate::tile::flags as tilestate;

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
    /// C++ `Map::isSightClear` (same floor) — adjacent tiles skip line checks (`map.cpp` ~571–575).
    pub fn is_sight_clear(&self, from: Position, to: Position) -> bool {
        if from.z != to.z {
            return false;
        }
        let dx = (from.x as i32 - to.x as i32).unsigned_abs();
        let dy = (from.y as i32 - to.y as i32).unsigned_abs();
        if dx < 2 && dy < 2 {
            return true;
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

    /// 772 line-of-sight — `ThrowPossible` (`info.cc:1154`).
    ///
    /// Major-axis linear interpolation (a different tile set than Bresenham on diagonals), checking
    /// only the `UNTHROW` (projectile-block) flag — **not** `BLOCKSOLID`/`BLOCKPATH` — plus the
    /// multi-floor `MinZ` stepping and the `HOOKEAST`/`HOOKSOUTH` `StartT=0` origin special case.
    /// All monster/combat callers use `power = 0` (`crnonpl.cc:2798`).
    pub fn throw_possible(&self, orig: Position, dest: Position, power: i32) -> bool {
        let orig_x = orig.x as i32;
        let orig_y = orig.y as i32;
        let orig_z = orig.z as i32;
        let dest_x = dest.x as i32;
        let dest_y = dest.y as i32;
        let dest_z = dest.z as i32;

        // `MinZ` = highest floor we can throw from; walk up looking for a bank ceiling.
        let mut min_z = (orig_z - power).max(0);
        let mut cur_z = orig_z - 1;
        while cur_z >= min_z {
            if self.column_has_floor(orig_x, orig_y, cur_z) {
                min_z = cur_z + 1;
                break;
            }
            cur_z -= 1;
        }

        let max_t = (dest_x - orig_x).abs().max((dest_y - orig_y).abs());

        // HOOK origin special case: throwing west through a HOOKEAST, or north through a HOOKSOUTH,
        // lets the line start at the origin tile itself (`info.cc:1175-1177`).
        let mut start_t = 1;
        if (dest_x < orig_x && self.tile_has_flag(orig, tilestate::HOOKEAST))
            || (dest_y < orig_y && self.tile_has_flag(orig, tilestate::HOOKSOUTH))
        {
            start_t = 0;
        }

        while min_z <= dest_z {
            let mut last_x = orig_x;
            let mut last_y = orig_y;
            if (dest_x != orig_x || dest_y != orig_y) && max_t > 0 {
                let mut t = start_t;
                while t <= max_t {
                    let cur_x = (orig_x * (max_t - t) + dest_x * t) / max_t;
                    let cur_y = (orig_y * (max_t - t) + dest_y * t) / max_t;
                    if self.column_blocks_throw(cur_x, cur_y, min_z) {
                        break;
                    }
                    last_x = cur_x;
                    last_y = cur_y;
                    t += 1;
                }
            }

            if last_x == dest_x && last_y == dest_y {
                // Vertical: the destination column must be open from `min_z` down to `dest_z`.
                let mut last_z = min_z;
                while last_z < dest_z {
                    if self.column_has_floor(dest_x, dest_y, last_z) {
                        break;
                    }
                    last_z += 1;
                }
                if last_z == dest_z {
                    return true;
                }
            }

            min_z += 1;
        }

        false
    }

    /// `CoordinateFlag(x,y,z, UNTHROW)` — projectile blocker at a column position (empty = clear).
    fn column_blocks_throw(&self, x: i32, y: i32, z: i32) -> bool {
        if !(0..=u16::MAX as i32).contains(&x) || !(0..=u16::MAX as i32).contains(&y) {
            return true;
        }
        self.get_tile(Position {
            x: x as u16,
            y: y as u16,
            z: z as u8,
        })
        .is_some_and(|t| t.body().flags & tilestate::UNTHROW != 0)
    }

    /// `GetFirstObject(x,y,z).getFlag(BANK)` — a ground/floor tile occupies the column position.
    fn column_has_floor(&self, x: i32, y: i32, z: i32) -> bool {
        if !(0..=u16::MAX as i32).contains(&x) || !(0..=u16::MAX as i32).contains(&y) || z < 0 {
            return false;
        }
        self.get_tile(Position {
            x: x as u16,
            y: y as u16,
            z: z as u8,
        })
        .is_some_and(|t| t.body().ground.is_some())
    }

    fn tile_has_flag(&self, pos: Position, flag: u32) -> bool {
        self.get_tile(pos)
            .is_some_and(|t| t.body().flags & flag != 0)
    }
}
