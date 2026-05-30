//! Use-item floor changes (ladders, grates, holes) without the full actions loader.
//!
//! C++ reference: `data/actions/scripts/other/teleport.lua` via `Actions::useItem`;
//! `Position:moveUpstairs` — `data/lib/core/position.lua`; `Game::internalTeleport` — `game.cpp` ~1784.

use tfs_rust_common::enums::Direction;
use tfs_rust_common::Position;

use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::walk::player_can_stand_at;

/// Items that move the player **up** one floor when used (`teleport.lua` `upFloorIds`).
const UP_FLOOR_USE_TYPES: &[u16] = &[1386, 3678, 5543, 22845, 22846];

/// All item types bound to `other/teleport.lua` in `data/actions/actions.xml`.
const TELEPORT_ACTION_USE_TYPES: &[u16] = &[430, 1369, 1386, 3678, 5543, 22845, 22846];

/// Whether `UseItem` should run the native teleport action for this item type.
pub(crate) fn is_teleport_floor_use_item(item_type: u16) -> bool {
    TELEPORT_ACTION_USE_TYPES.contains(&item_type)
}

/// Resolve destination for `teleport.lua` — `moveUpstairs()` or same (x,y) one floor down.
pub(crate) fn resolve_teleport_use_destination(
    world: &GameWorld,
    cid: CreatureId,
    item_type: u16,
    from: Position,
) -> Position {
    if UP_FLOOR_USE_TYPES.contains(&item_type) {
        position_move_upstairs(world, cid, from)
    } else {
        Position {
            x: from.x,
            y: from.y,
            z: from.z.saturating_add(1),
        }
    }
}

/// TFS `Position:moveUpstairs` — `data/lib/core/position.lua`.
fn position_move_upstairs(world: &GameWorld, cid: CreatureId, from: Position) -> Position {
    if from.z == 0 {
        return from;
    }
    let base = Position {
        x: from.x,
        y: from.y,
        z: from.z - 1,
    };

    let candidate = |dir: Direction| -> Option<Position> {
        let p = base.offset(dir);
        if player_can_stand_at(world, cid, p) {
            Some(p)
        } else {
            None
        }
    };

    if let Some(p) = candidate(Direction::South) {
        return p;
    }

    for dir in [
        Direction::North,
        Direction::East,
        Direction::NorthEast,
        Direction::West,
        Direction::NorthWest,
        Direction::SouthEast,
    ] {
        if let Some(p) = candidate(dir) {
            return p;
        }
    }

    base.offset(Direction::South)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder_and_grate_are_teleport_use_items() {
        assert!(is_teleport_floor_use_item(1386));
        assert!(is_teleport_floor_use_item(430));
        assert!(!is_teleport_floor_use_item(2160));
    }
}
