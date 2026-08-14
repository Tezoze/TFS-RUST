//! TFS `Monster::getDistanceStep` helpers + 772 `SearchFlightField`.
//!
//! - `Position::getDistanceX/Y` / `getOffsetX/Y` — `position.h`.
//! - `SearchFlightField` — `info.cc` ~1030.
//!
//! The TFS `getRandomStep` / `getDanceStep` / `getDistanceStep` step pickers were deleted
//! in Phase 3–5 of the unified beat engine effort — both eras now run on the 772 ToDo /
//! IdleStimulus engine, which uses `search_flight_field` for flee and A* (`get_creature_path_to`)
//! for chase. Only the offset/distance primitives and `search_flight_field` remain live.

use tfs_rust_common::Position;
use tfs_rust_common::enums::Direction;

/// TFS `Position::getDistanceX/Y` — absolute axis delta (symmetric in argument order).
pub(crate) fn distance_x(a: Position, b: Position) -> i32 {
    (a.x as i32 - b.x as i32).unsigned_abs() as i32
}

pub(crate) fn distance_y(a: Position, b: Position) -> i32 {
    (a.y as i32 - b.y as i32).unsigned_abs() as i32
}

/// C++ `Position::getOffsetX/Y(creaturePos, targetPos)` — `creature − target`.
pub(crate) fn offset_x(creature: Position, target: Position) -> i32 {
    creature.x as i32 - target.x as i32
}

pub(crate) fn offset_y(creature: Position, target: Position) -> i32 {
    creature.y as i32 - target.y as i32
}

/// 772 `SearchFlightField` — `info.cc` ~1030.
///
/// Sweeps directions away from pursuer:
/// 1. Preferred axial direction.
/// 2. Shuffled remaining 3 cardinal directions.
/// 3. Shuffled 4 diagonal directions.
pub fn search_flight_field<F, S>(
    creature_pos: Position,
    pursuer_pos: Position,
    can_walk: F,
    mut parity_shuffle: S,
) -> Option<Direction>
where
    F: Fn(Direction) -> bool,
    S: FnMut(&mut [Option<Direction>]),
{
    let ox = creature_pos.x as i32 - pursuer_pos.x as i32;
    let oy = creature_pos.y as i32 - pursuer_pos.y as i32;
    let dx = ox.abs();
    let dy = oy.abs();

    let mut dirs: [Option<Direction>; 9] = [None; 9];

    // 1. Prefer axial direction away from the pursuer.
    if dx > dy {
        dirs[0] = Some(if ox < 0 {
            Direction::West
        } else {
            Direction::East
        });
    } else if dx < dy {
        dirs[0] = Some(if oy < 0 {
            Direction::North
        } else {
            Direction::South
        });
    }

    // 2. Fallback to random axial direction away from the pursuer.
    if ox >= 0 {
        dirs[1] = Some(Direction::East);
    }
    if oy <= 0 {
        dirs[2] = Some(Direction::North);
    }
    if ox <= 0 {
        dirs[3] = Some(Direction::West);
    }
    if oy >= 0 {
        dirs[4] = Some(Direction::South);
    }
    // C++ `RandomShuffle(&Dir[1], 4)` — forward Fisher-Yates on the glibc parity stream (Finding 9).
    parity_shuffle(&mut dirs[1..5]);

    // 3. Fallback to diagonal direction away from the pursuer.
    if oy <= ox {
        dirs[5] = Some(Direction::NorthEast);
    }
    if oy <= -ox {
        dirs[6] = Some(Direction::NorthWest);
    }
    if oy >= ox {
        dirs[7] = Some(Direction::SouthWest);
    }
    if oy >= -ox {
        dirs[8] = Some(Direction::SouthEast);
    }
    // C++ `RandomShuffle(&Dir[5], 4)` — forward Fisher-Yates on the glibc parity stream (Finding 9).
    parity_shuffle(&mut dirs[5..9]);

    // Evaluate in order
    for &opt_dir in &dirs {
        if let Some(dir) = opt_dir {
            if can_walk(dir) {
                return Some(dir);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim_glibc_rand::parity_random_shuffle;

    #[test]
    fn flight_field_prefers_axial_then_cardinal_then_diagonal() {
        let from = Position::new(100, 100, 7);
        let pursuer = Position::new(98, 100, 7); // West of us, so ox = 2, oy = 0.
        // dx = 2, dy = 0.
        // Preferred axial: East.
        // Fallback card: East (already preferred), North, South.
        // Fallback diag: NorthEast, SouthEast.

        // 1. All clear -> East.
        let can_walk = |_d: Direction| true;
        assert_eq!(
            search_flight_field(from, pursuer, can_walk, parity_random_shuffle),
            Some(Direction::East)
        );

        // 2. East blocked -> check remaining cardinals (North/South).
        let can_walk = |d: Direction| !matches!(d, Direction::East);
        let res = search_flight_field(from, pursuer, can_walk, parity_random_shuffle).unwrap();
        assert!(matches!(res, Direction::North | Direction::South));

        // 3. East + North + South blocked -> diagonal.
        let can_walk =
            |d: Direction| !matches!(d, Direction::East | Direction::North | Direction::South);
        let res = search_flight_field(from, pursuer, can_walk, parity_random_shuffle).unwrap();
        assert!(matches!(res, Direction::NorthEast | Direction::SouthEast));
    }

    #[test]
    fn flight_field_returns_none_when_all_blocked() {
        let from = Position::new(100, 100, 7);
        let pursuer = Position::new(98, 100, 7);
        let can_walk = |_d: Direction| false;
        assert_eq!(
            search_flight_field(from, pursuer, can_walk, parity_random_shuffle),
            None
        );
    }

    #[test]
    fn flight_field_prefers_north_when_pursuer_south() {
        let from = Position::new(100, 100, 7);
        let pursuer = Position::new(100, 102, 7); // South of us, oy = -2.
        let can_walk = |_d: Direction| true;
        assert_eq!(
            search_flight_field(from, pursuer, can_walk, parity_random_shuffle),
            Some(Direction::North)
        );
    }

    #[test]
    fn distance_x_y_symmetric() {
        let a = Position::new(100, 100, 7);
        let b = Position::new(103, 107, 7);
        assert_eq!(distance_x(a, b), 3);
        assert_eq!(distance_y(a, b), 7);
        assert_eq!(distance_x(b, a), 3);
        assert_eq!(distance_y(b, a), 7);
    }

    #[test]
    fn offset_x_y_signed_creature_minus_target() {
        let creature = Position::new(100, 100, 7);
        let target = Position::new(103, 107, 7);
        assert_eq!(offset_x(creature, target), -3);
        assert_eq!(offset_y(creature, target), -7);
    }
}
