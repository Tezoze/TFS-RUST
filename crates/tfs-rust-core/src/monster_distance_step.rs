//! TFS `Monster::getDistanceStep` / `getRandomStep` — `monster.cpp` ~1277, ~1386.
//!
//! Offset convention matches C++ `Position::getOffsetX/Y(creaturePos, targetPos)`:
//! `creature.x - target.x`, `creature.y - target.y`.

use rand::Rng;
use rand::seq::SliceRandom;
use tfs_rust_common::enums::Direction;
use tfs_rust_common::Position;

fn distance_x(creature: Position, target: Position) -> i32 {
    (creature.x as i32 - target.x as i32).unsigned_abs() as i32
}

fn distance_y(creature: Position, target: Position) -> i32 {
    (creature.y as i32 - target.y as i32).unsigned_abs() as i32
}

/// C++ `Position::getOffsetX/Y(creaturePos, targetPos)`.
fn offset_x(creature: Position, target: Position) -> i32 {
    creature.x as i32 - target.x as i32
}

fn offset_y(creature: Position, target: Position) -> i32 {
    creature.y as i32 - target.y as i32
}

fn boolean_random(rng: &mut impl Rng) -> bool {
    rng.gen_bool(0.5)
}

fn pick_random_dir(
    rng: &mut impl Rng,
    a: Direction,
    b: Direction,
) -> Direction {
    if boolean_random(rng) {
        a
    } else {
        b
    }
}

fn try_walk<F>(can_walk: &F, dir: Direction) -> Option<Direction>
where
    F: Fn(Direction) -> bool,
{
    if can_walk(dir) {
        Some(dir)
    } else {
        None
    }
}

fn try_walk_pair<F>(
    can_walk: &F,
    a: Direction,
    b: Direction,
    rng: &mut impl Rng,
) -> Option<Direction>
where
    F: Fn(Direction) -> bool,
{
    let a_ok = can_walk(a);
    let b_ok = can_walk(b);
    if a_ok && b_ok {
        return Some(pick_random_dir(rng, a, b));
    }
    if a_ok {
        return Some(a);
    }
    if b_ok {
        return Some(b);
    }
    None
}

/// TFS `Monster::getRandomStep` — `monster.cpp` ~1277.
pub fn get_random_step<F>(can_walk: F, rng: &mut impl Rng) -> Option<Direction>
where
    F: Fn(Direction) -> bool,
{
    let mut dirs = [
        Direction::North,
        Direction::West,
        Direction::East,
        Direction::South,
    ];
    dirs.shuffle(rng);
    for dir in dirs {
        if can_walk(dir) {
            return Some(dir);
        }
    }
    None
}

/// TFS `Monster::getDanceStep` — `monster.cpp` ~1295.
pub fn get_dance_step<F, G>(
    creature_pos: Position,
    target_pos: Position,
    keep_attack: bool,
    keep_distance: bool,
    can_walk: F,
    can_use_attack_from: G,
    can_use_attack_now: bool,
    rng: &mut impl Rng,
) -> Option<Direction>
where
    F: Fn(Direction) -> bool,
    G: Fn(Position) -> bool,
{
    let offset_x = offset_x(creature_pos, target_pos);
    let offset_y = offset_y(creature_pos, target_pos);
    let distance_x = offset_x.unsigned_abs() as i32;
    let distance_y = offset_y.unsigned_abs() as i32;
    let center_to_dist = distance_x.max(distance_y);

    let mut dir_list: Vec<Direction> = Vec::new();

    let mut try_dir = |dir: Direction, dest: Position| {
        if !can_walk(dir) {
            return;
        }
        if keep_attack {
            let ok = !can_use_attack_now || can_use_attack_from(dest);
            if !ok {
                return;
            }
        }
        dir_list.push(dir);
    };

    if !keep_distance || offset_y >= 0 {
        let tmp_dist = distance_x.max((creature_pos.y as i32 - 1 - target_pos.y as i32).unsigned_abs() as i32);
        if tmp_dist == center_to_dist {
            try_dir(
                Direction::North,
                Position::new(creature_pos.x, creature_pos.y.saturating_sub(1), creature_pos.z),
            );
        }
    }

    if !keep_distance || offset_y <= 0 {
        let tmp_dist = distance_x.max((creature_pos.y as i32 + 1 - target_pos.y as i32).unsigned_abs() as i32);
        if tmp_dist == center_to_dist {
            try_dir(
                Direction::South,
                Position::new(creature_pos.x, creature_pos.y.saturating_add(1), creature_pos.z),
            );
        }
    }

    if !keep_distance || offset_x <= 0 {
        let tmp_dist = ((creature_pos.x as i32 + 1 - target_pos.x as i32).unsigned_abs() as i32).max(distance_y);
        if tmp_dist == center_to_dist {
            try_dir(
                Direction::East,
                Position::new(creature_pos.x.saturating_add(1), creature_pos.y, creature_pos.z),
            );
        }
    }

    if !keep_distance || offset_x >= 0 {
        let tmp_dist = ((creature_pos.x as i32 - 1 - target_pos.x as i32).unsigned_abs() as i32).max(distance_y);
        if tmp_dist == center_to_dist {
            try_dir(
                Direction::West,
                Position::new(creature_pos.x.saturating_sub(1), creature_pos.y, creature_pos.z),
            );
        }
    }

    if dir_list.is_empty() {
        return None;
    }
    dir_list.shuffle(rng);
    Some(dir_list[rng.gen_range(0..dir_list.len())])
}

/// TFS `Monster::getDistanceStep` — `monster.cpp` ~1386.
pub fn get_distance_step<F>(
    creature_pos: Position,
    target_pos: Position,
    target_distance: i32,
    flee: bool,
    sight_clear: bool,
    can_walk: F,
    rng: &mut impl Rng,
) -> Option<Direction>
where
    F: Fn(Direction) -> bool,
{
    let dx = distance_x(creature_pos, target_pos);
    let dy = distance_y(creature_pos, target_pos);
    let distance = dx.max(dy);

    if !flee && (distance > target_distance || !sight_clear) {
        return None;
    }
    if !flee && distance == target_distance {
        return None;
    }

    let offsetx = offset_x(creature_pos, target_pos);
    let offsety = offset_y(creature_pos, target_pos);

    if offsetx == 0 && offsety == 0 {
        return get_random_step(can_walk, rng);
    }

    if dx == dy {
        return diagonal_distance_step(offsetx, offsety, flee, &can_walk, rng);
    }

    if dy > dx {
        let player_dir = if offsety < 0 {
            Direction::South
        } else {
            Direction::North
        };
        return vertical_distance_step(player_dir, offsetx, flee, &can_walk, rng);
    }

    let player_dir = if offsetx < 0 {
        Direction::East
    } else {
        Direction::West
    };
    horizontal_distance_step(player_dir, offsety, flee, &can_walk, rng)
}

fn diagonal_distance_step<F>(
    offsetx: i32,
    offsety: i32,
    flee: bool,
    can_walk: &F,
    rng: &mut impl Rng,
) -> Option<Direction>
where
    F: Fn(Direction) -> bool,
{
    if offsetx >= 1 && offsety >= 1 {
        // player NW — escape SE, S, E
        if let Some(dir) = try_walk_pair(can_walk, Direction::South, Direction::East, rng) {
            return Some(dir);
        }
        if let Some(dir) = try_walk(can_walk, Direction::SouthEast) {
            return Some(dir);
        }
        if flee {
            if let Some(dir) = try_walk_pair(can_walk, Direction::North, Direction::West, rng) {
                return Some(dir);
            }
        }
        if can_walk(Direction::West) && can_walk(Direction::SouthWest) {
            return Some(Direction::West);
        }
        if can_walk(Direction::North) && can_walk(Direction::NorthEast) {
            return Some(Direction::North);
        }
        return Some(Direction::North);
    }

    if offsetx <= -1 && offsety <= -1 {
        // player SE — escape NW, W, N
        if let Some(dir) = try_walk_pair(can_walk, Direction::West, Direction::North, rng) {
            return Some(dir);
        }
        if let Some(dir) = try_walk(can_walk, Direction::NorthWest) {
            return Some(dir);
        }
        if flee {
            if let Some(dir) = try_walk_pair(can_walk, Direction::South, Direction::East, rng) {
                return Some(dir);
            }
        }
        if can_walk(Direction::South) && can_walk(Direction::SouthWest) {
            return Some(Direction::South);
        }
        if can_walk(Direction::East) && can_walk(Direction::NorthEast) {
            return Some(Direction::East);
        }
        return Some(Direction::East);
    }

    if offsetx >= 1 && offsety <= -1 {
        // player SW — escape NE, N, E
        if let Some(dir) = try_walk_pair(can_walk, Direction::North, Direction::East, rng) {
            return Some(dir);
        }
        if let Some(dir) = try_walk(can_walk, Direction::NorthEast) {
            return Some(dir);
        }
        if flee {
            if let Some(dir) = try_walk_pair(can_walk, Direction::South, Direction::West, rng) {
                return Some(dir);
            }
        }
        if can_walk(Direction::West) && can_walk(Direction::NorthWest) {
            return Some(Direction::West);
        }
        if can_walk(Direction::South) && can_walk(Direction::SouthEast) {
            return Some(Direction::South);
        }
        return Some(Direction::South);
    }

    if offsetx <= -1 && offsety >= 1 {
        // player NE — escape SW, S, W
        if let Some(dir) = try_walk_pair(can_walk, Direction::West, Direction::South, rng) {
            return Some(dir);
        }
        if let Some(dir) = try_walk(can_walk, Direction::SouthWest) {
            return Some(dir);
        }
        if flee {
            if let Some(dir) = try_walk_pair(can_walk, Direction::North, Direction::East, rng) {
                return Some(dir);
            }
        }
        if can_walk(Direction::East) && can_walk(Direction::SouthEast) {
            return Some(Direction::East);
        }
        if can_walk(Direction::North) && can_walk(Direction::NorthWest) {
            return Some(Direction::North);
        }
        return Some(Direction::North);
    }

    None
}

fn vertical_distance_step<F>(
    player_dir: Direction,
    offsetx: i32,
    flee: bool,
    can_walk: &F,
    rng: &mut impl Rng,
) -> Option<Direction>
where
    F: Fn(Direction) -> bool,
{
    match player_dir {
        Direction::North => {
            if let Some(dir) = try_walk(can_walk, Direction::South) {
                return Some(dir);
            }
            let w = can_walk(Direction::West);
            let e = can_walk(Direction::East);
            if w && e && offsetx == 0 {
                return Some(pick_random_dir(rng, Direction::West, Direction::East));
            }
            if w && offsetx <= 0 {
                return Some(Direction::West);
            }
            if e && offsetx >= 0 {
                return Some(Direction::East);
            }
            if flee {
                if let Some(dir) = try_walk_pair(can_walk, Direction::West, Direction::East, rng) {
                    return Some(dir);
                }
            }
            let sw = can_walk(Direction::SouthWest);
            let se = can_walk(Direction::SouthEast);
            if sw || se {
                if sw && se {
                    return Some(pick_random_dir(rng, Direction::SouthWest, Direction::SouthEast));
                }
                if w {
                    return Some(Direction::West);
                }
                if sw {
                    return Some(Direction::SouthWest);
                }
                if e {
                    return Some(Direction::East);
                }
                return Some(Direction::SouthEast);
            }
            if flee {
                return try_walk(can_walk, Direction::North);
            }
        }
        Direction::South => {
            if let Some(dir) = try_walk(can_walk, Direction::North) {
                return Some(dir);
            }
            let w = can_walk(Direction::West);
            let e = can_walk(Direction::East);
            if w && e && offsetx == 0 {
                return Some(pick_random_dir(rng, Direction::West, Direction::East));
            }
            if w && offsetx <= 0 {
                return Some(Direction::West);
            }
            if e && offsetx >= 0 {
                return Some(Direction::East);
            }
            if flee {
                if let Some(dir) = try_walk_pair(can_walk, Direction::West, Direction::East, rng) {
                    return Some(dir);
                }
            }
            let nw = can_walk(Direction::NorthWest);
            let ne = can_walk(Direction::NorthEast);
            if nw || ne {
                if nw && ne {
                    return Some(pick_random_dir(rng, Direction::NorthWest, Direction::NorthEast));
                }
                if w {
                    return Some(Direction::West);
                }
                if nw {
                    return Some(Direction::NorthWest);
                }
                if e {
                    return Some(Direction::East);
                }
                return Some(Direction::NorthEast);
            }
            if flee {
                return try_walk(can_walk, Direction::South);
            }
        }
        _ => {}
    }
    None
}

fn horizontal_distance_step<F>(
    player_dir: Direction,
    offsety: i32,
    flee: bool,
    can_walk: &F,
    rng: &mut impl Rng,
) -> Option<Direction>
where
    F: Fn(Direction) -> bool,
{
    match player_dir {
        Direction::West => {
            if let Some(dir) = try_walk(can_walk, Direction::East) {
                return Some(dir);
            }
            let n = can_walk(Direction::North);
            let s = can_walk(Direction::South);
            if n && s && offsety == 0 {
                return Some(pick_random_dir(rng, Direction::North, Direction::South));
            }
            if n && offsety <= 0 {
                return Some(Direction::North);
            }
            if s && offsety >= 0 {
                return Some(Direction::South);
            }
            if flee {
                if let Some(dir) = try_walk_pair(can_walk, Direction::North, Direction::South, rng) {
                    return Some(dir);
                }
            }
            let se = can_walk(Direction::SouthEast);
            let ne = can_walk(Direction::NorthEast);
            if se || ne {
                if se && ne {
                    return Some(pick_random_dir(rng, Direction::SouthEast, Direction::NorthEast));
                }
                if s {
                    return Some(Direction::South);
                }
                if se {
                    return Some(Direction::SouthEast);
                }
                if n {
                    return Some(Direction::North);
                }
                return Some(Direction::NorthEast);
            }
            if flee {
                return try_walk(can_walk, Direction::West);
            }
        }
        Direction::East => {
            if let Some(dir) = try_walk(can_walk, Direction::West) {
                return Some(dir);
            }
            let n = can_walk(Direction::North);
            let s = can_walk(Direction::South);
            if n && s && offsety == 0 {
                return Some(pick_random_dir(rng, Direction::North, Direction::South));
            }
            if n && offsety <= 0 {
                return Some(Direction::North);
            }
            if s && offsety >= 0 {
                return Some(Direction::South);
            }
            if flee {
                if let Some(dir) = try_walk_pair(can_walk, Direction::North, Direction::South, rng) {
                    return Some(dir);
                }
            }
            let nw = can_walk(Direction::NorthWest);
            let sw = can_walk(Direction::SouthWest);
            if nw || sw {
                if nw && sw {
                    return Some(pick_random_dir(rng, Direction::NorthWest, Direction::SouthWest));
                }
                if n {
                    return Some(Direction::North);
                }
                if nw {
                    return Some(Direction::NorthWest);
                }
                if s {
                    return Some(Direction::South);
                }
                return Some(Direction::SouthWest);
            }
            if flee {
                return try_walk(can_walk, Direction::East);
            }
        }
        _ => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn chebyshev(a: Position, b: Position) -> i32 {
        distance_x(a, b).max(distance_y(a, b))
    }

    #[test]
    fn flee_step_increases_distance_when_escape_open() {
        let from = Position::new(10, 10, 7);
        let target = Position::new(10, 12, 7);
        let can_walk = |_d: Direction| true;
        let mut rng = StdRng::seed_from_u64(1);
        let dir = get_distance_step(from, target, 1, true, true, can_walk, &mut rng).unwrap();
        let next = from.offset(dir);
        assert!(chebyshev(next, target) > chebyshev(from, target));
    }

    #[test]
    fn flee_player_east_monster_steps_west() {
        let from = Position::new(100, 100, 7);
        let target = Position::new(101, 100, 7);
        let can_walk = |_d: Direction| true;
        let mut rng = StdRng::seed_from_u64(42);
        let dir = get_distance_step(from, target, 1, true, true, can_walk, &mut rng).unwrap();
        assert_eq!(dir, Direction::West);
    }

    #[test]
    fn flee_blocked_primary_escape_tries_perpendicular() {
        let from = Position::new(10, 10, 7);
        let target = Position::new(10, 12, 7);
        let can_walk = |d: Direction| !matches!(d, Direction::North);
        let mut rng = StdRng::seed_from_u64(2);
        let dir = get_distance_step(from, target, 1, true, true, can_walk, &mut rng).unwrap();
        assert!(matches!(dir, Direction::West | Direction::East));
    }

    #[test]
    fn dance_step_picks_valid_lateral_when_chase_queue_empty() {
        let creature = Position::new(10, 10, 7);
        let target = Position::new(12, 10, 7);
        let can_walk = |d: Direction| matches!(d, Direction::North | Direction::South);
        let can_attack = |_from: Position| true;
        let mut rng = StdRng::seed_from_u64(99);
        let dir = get_dance_step(
            creature,
            target,
            true,
            true,
            can_walk,
            can_attack,
            false,
            &mut rng,
        );
        assert!(matches!(dir, Some(Direction::North) | Some(Direction::South)));
    }
}
