//! Client throw / `playerMoveThing` item path.
//!
//! - `Game::playerMoveThing`, `playerMoveItem` — `game.cpp`.
//! - `Map::canThrowObjectTo` — `map.cpp`.

use std::time::Instant;

use tfs_rust_common::{ConnId, Position};

use crate::cylinder::{Cylinder, CylinderFlags};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use crate::thing::Thing;

impl GameWorld {
    // === B.5: Player Throw (item move from client) ===
    // C++ ref: src/game.cpp:644 Game::playerMoveThing, :905 Game::playerMoveItem

    /// Handle `parseThrow` — player moves a thing from one position to another.
    // C++ ref: src/game.cpp Game::playerMoveThing — signature mirrors the protocol call.
    ///
    /// F8 S4/S7 — returns `Result<(), ReturnValue>` so the ToDo `Execute` arm can apply the
    /// C++ `RESULT` catch (`cract.cc:870-889`). `Err(rv)` = hard failure; `Ok(())` =
    /// success **or** walk-to-reach deferral (1098 reactive path — `try_walk_to_and_action`
    /// sets `walk_action` and returns; the 772 ToDo path uses `Go`-prepend via
    /// `execute_player_move` instead).
    #[allow(clippy::too_many_arguments)]
    pub fn player_move_thing(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        from_pos: Position,
        sprite_id: u16,
        from_stack_pos: u8,
        to_pos: Position,
        count: u8,
        now: Instant,
    ) -> Result<(), ReturnValue> {
        if from_pos == to_pos {
            return Ok(());
        }
        // Resolve source thing
        let Some(thing) = self.internal_get_thing_move(cid, from_pos, from_stack_pos, sprite_id) else {
            return Err(ReturnValue::NotPossible);
        };

        match thing {
            Thing::Creature(_moving_creature) => {
                // Creature move — already handled by walk system for players;
                // NPC/monster push is Phase 9+.
                tracing::debug!("player_move_thing: creature move not yet wired");
                Ok(())
            }
            Thing::Item(item_id) => self.player_move_item(
                conn_id,
                cid,
                from_pos,
                sprite_id,
                from_stack_pos,
                to_pos,
                count,
                item_id,
                now,
            ),
        }
    }

    /// Handle the item branch of playerMoveThing.
    // C++ ref: src/game.cpp:905 Game::playerMoveItem
    #[allow(clippy::too_many_arguments)]
    fn player_move_item(
        &mut self,
        _conn_id: ConnId,
        cid: CreatureId,
        from_pos: Position,
        sprite_id: u16,
        _from_stack_pos: u8,
        to_pos: Position,
        count: u8,
        item_id: ItemId,
        _now: Instant,
    ) -> Result<(), ReturnValue> {
        let item_is_pickupable;
        // Verify client sprite ID matches
        if let Some(item) = self.items.get(item_id) {
            let it = self.items_db.items.get(&item.item_type);
            let client_id = it.map(|t| t.client_id).unwrap_or(0);
            if client_id != sprite_id {
                return Err(ReturnValue::NotPossible);
            }
            // Check moveable
            let is_moveable = it.map(|t| t.moveable()).unwrap_or(false);
            if !is_moveable {
                return Err(ReturnValue::NotMoveable);
            }
            item_is_pickupable = it.map(|t| t.pickupable()).unwrap_or(false);
        } else {
            return Err(ReturnValue::NotPossible);
        }

        // Resolve cylinders
        let Some(from_cylinder) = self.internal_get_cylinder(cid, from_pos) else {
            return Err(ReturnValue::NotPossible);
        };
        let Some(to_cylinder) = self.internal_get_cylinder(cid, to_pos) else {
            return Err(ReturnValue::NotPossible);
        };

        let Some(player_pos) = self.creatures.get(cid).map(|p| p.position()) else {
            return Err(ReturnValue::NotPossible);
        };

        let map_from_pos = match from_cylinder {
            Cylinder::Tile { pos } => pos,
            Cylinder::Container { .. } | Cylinder::Inventory { .. } => player_pos,
        };
        let map_to_pos = match to_cylinder {
            Cylinder::Tile { pos } => pos,
            Cylinder::Container { .. } | Cylinder::Inventory { .. } => player_pos,
        };

        // Range check — player must be able to see source
        if from_pos.x != 0xFFFF {
            // Z-level check — TFS uses `mapFromPos` (`game.cpp` ~965).
            if player_pos.z != map_from_pos.z {
                let rv = if player_pos.z > map_from_pos.z {
                    ReturnValue::FirstGoUpStairs
                } else {
                    ReturnValue::FirstGoDownStairs
                };
                return Err(rv);
            }
            // Distance check — the ToDo `Move` execute arm handles walk-to-reach via
            // `Go`-prepend before dispatching here. By this point the player is adjacent
            // (dx <= 1 && dy <= 1). The C++ reactive walk path (`game.cpp` ~970–983) is
            // handled by the enqueue-time `ObjectInRange(1)` check in `enqueue_player_move`.
        }

        // C++ ref: src/game.cpp:1046-1060 Game::playerMoveItem
        // 772 CheckMapDestination: non-takeable items have an ObjectInRange(2) gate;
        // takeable items have no fixed throw range (only ThrowPossible LOS).
        if !item_is_pickupable {
            let to_dx = (player_pos.x as i32 - map_to_pos.x as i32).unsigned_abs();
            let to_dy = (player_pos.y as i32 - map_to_pos.y as i32).unsigned_abs();
            if to_dx > 2 || to_dy > 2 {
                return Err(ReturnValue::DestinationOutOfReach);
            }
        }

        // C++ ref: src/game.cpp:1058 `canThrowObjectTo(...)`, `info.cc:1154` ThrowPossible
        if !self.map.throw_possible(map_from_pos, map_to_pos, 1) {
            return Err(ReturnValue::CannotThrow);
        }

        // Check if destination tile can accept the thrown item
        // C++ ref: `operate.cc:451` `IsMapBlocked`.
        if to_pos.x != 0xFFFF && self.is_map_blocked(map_to_pos, item_id) {
            return Err(ReturnValue::NotEnoughRoom);
        }

        let result = self.internal_move_item(
            Some(cid),
            from_cylinder,
            to_cylinder,
            item_id,
            count as u16,
            CylinderFlags::NONE,
        );
        result?;
        Ok(())
    }



}
