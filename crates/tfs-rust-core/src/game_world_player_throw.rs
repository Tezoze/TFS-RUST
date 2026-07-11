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
        let Some(thing) = self.internal_get_thing_move(cid, from_pos, from_stack_pos) else {
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
        let item_throw_range;
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
            // C++ ref: src/item.h:828-829 Item::getThrowRange (pickupable ? 15 : 2)
            item_throw_range = if item_is_pickupable { 15u32 } else { 2u32 };
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
        if !item_is_pickupable && player_pos.z != map_to_pos.z {
            return Err(ReturnValue::DestinationOutOfReach);
        }

        let to_dx = (player_pos.x as i32 - map_to_pos.x as i32).unsigned_abs();
        let to_dy = (player_pos.y as i32 - map_to_pos.y as i32).unsigned_abs();
        if to_dx > item_throw_range || to_dy > item_throw_range {
            return Err(ReturnValue::DestinationOutOfReach);
        }

        // C++ ref: src/game.cpp:1058 `canThrowObjectTo(mapFromPos, mapToPos, true, false, throwRange, throwRange)`
        if !self.can_throw_object_to(map_from_pos, map_to_pos, item_throw_range) {
            return Err(ReturnValue::CannotThrow);
        }

        // Check if destination tile can accept the thrown item
        if to_pos.x != 0xFFFF && !self.can_throw_to_tile(map_to_pos, item_id) {
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

    // C++ ref: src/map.cpp:486-494 `Map::canThrowObjectTo` + `isSightClear` / `isTileClear`
    fn can_throw_object_to(&self, from: Position, to: Position, throw_range: u32) -> bool {
        if from.z != to.z {
            return false;
        }
        let dx = (from.x as i32 - to.x as i32).unsigned_abs();
        let dy = (from.y as i32 - to.y as i32).unsigned_abs();
        if dx > throw_range || dy > throw_range {
            return false;
        }
        // C++ `isSightClear` — adjacent tiles skip line checks (`map.cpp` ~573).
        if dx < 2 && dy < 2 {
            return true;
        }
        for p in crate::map::walk_grid_line(from, to) {
            if p == from || p == to {
                continue;
            }
            if !self.is_tile_clear_for_throw(p, false) {
                return false;
            }
        }
        true
    }

    // Check if the destination tile can accept thrown items
    // C++ ref: Part of Tile::queryAdd logic for thrown items
    fn can_throw_to_tile(&self, pos: Position, _item_id: ItemId) -> bool {
        let Some(tile) = self.map.get_tile(pos) else {
            // No tile means you can't throw there
            return false;
        };

        // Check tile flags first
        let body = tile.body();
        if body.flags & (crate::tile::flags::BLOCK_PROJECTILE | crate::tile::flags::BLOCK_SOLID)
            != 0
        {
            return false;
        }

        // Check ground item
        if let Some(ground_id) = body.ground {
            let ground_item = self.items_db.items.get(&ground_id);
            if let Some(ground_type) = ground_item {
                if ground_type.block_projectile() || ground_type.block_solid() {
                    return false;
                }
            }
        }

        // Check all items on the tile
        for &iid in body.top_items.iter().chain(body.down_items.iter()) {
            let Some(item) = self.items.get(iid) else {
                continue;
            };
            if let Some(item_type) = self.items_db.items.get(&item.item_type) {
                if item_type.block_projectile() || item_type.block_solid() {
                    return false;
                }
            }
        }

        true
    }

    // C++ ref: src/map.cpp:496-508 Map::isTileClear, src/tile.cpp:27-40 Tile::hasProperty
    fn is_tile_clear_for_throw(&self, pos: Position, block_floor: bool) -> bool {
        let Some(tile) = self.map.get_tile(pos) else {
            return true;
        };

        let body = tile.body();

        if block_floor && body.ground.is_some() {
            return false;
        }

        // C++ `CONST_PROP_BLOCKPROJECTILE` → Rust `UNTHROW` (set from `block_projectile()`).
        // The legacy `BLOCK_PROJECTILE` alias maps to `BLOCKPATH` (pathfinding) — wrong flag.
        if body.flags & crate::tile::flags::UNTHROW != 0 {
            return false;
        }

        if let Some(ground_id) = body.ground {
            let ground_blocks = self
                .items_db
                .items
                .get(&ground_id)
                .map(|it| it.block_projectile())
                .unwrap_or(false);
            if ground_blocks {
                return false;
            }
        }

        for &iid in body.top_items.iter().chain(body.down_items.iter()) {
            let Some(item) = self.items.get(iid) else {
                continue;
            };
            let blocks = self
                .items_db
                .items
                .get(&item.item_type)
                .map(|it| it.block_projectile())
                .unwrap_or(false);
            if blocks {
                return false;
            }
        }

        true
    }
}
