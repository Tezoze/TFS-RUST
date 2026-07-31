//! Client throw / `playerMoveThing` item path.
//!
//! - `Game::playerMoveThing`, `playerMoveItem` — `game.cpp`.
//! - `Map::canThrowObjectTo` — `map.cpp`.

use std::time::Instant;

use tfs_rust_common::{ConnId, Position};

use crate::creature::CreatureKind;
use crate::creature_todo::{ActionObjectRef, CreatureAction};
use crate::cylinder::{Cylinder, CylinderFlags};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use crate::thing::Thing;
use crate::tile::flags as tilestate;

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
        // 772 `receiving.cc:258` silently rejects `Type.isMapContainer()` (sprite 0).
        if sprite_id == 0 {
            return Ok(());
        }

        // 772 `receiving.cc` `CheckSpecialCoordinates` / `CheckVisibility`.
        if from_pos.x != 0xFFFF {
            if from_pos.z > 15 || to_pos.z > 15 {
                return Err(ReturnValue::NotPossible);
            }
            if !self.can_see_position(cid, from_pos) {
                return Err(ReturnValue::NotPossible);
            }
        }

        // Resolve source thing
        let Some(thing) = self.internal_get_thing_move(cid, from_pos, from_stack_pos, sprite_id) else {
            return Err(ReturnValue::NotPossible);
        };

        match thing {
            Thing::Creature(moving_creature) => {
                // 772 `TCreature::Move` (`cract.cc:475`) — self-move or push another creature.
                if from_pos.x == 0xFFFF || to_pos.x == 0xFFFF {
                    return Err(ReturnValue::NotPossible);
                }
                if moving_creature == cid {
                    // 772 `Obj == this->CrObject` → `this->Go(DestX, DestY, DestZ)`.
                    self.setup_player_walk_to_target(cid, to_pos, now)?;
                    if self.creatures.get(cid).is_some_and(|k| !k.base().walk_queue.is_empty()) {
                        if let Some(k) = self.creatures.get_mut(cid) {
                            k.base_mut().todo.queue.push_front(CreatureAction::Go);
                        }
                        if self.todo_start_go_delay(cid, true) {
                            self.schedule_immediate_todo_wakeup(cid);
                        }
                    }
                    Ok(())
                } else {
                    self.player_push_creature(cid, moving_creature, from_pos, to_pos)
                }
            }
            Thing::Item(item_id) => {
                // 772 `receiving.cc:258` silently rejects `CUMULATIVE && Count == 0`.
                if count == 0 {
                    let item_type = self.items.get(item_id).map(|i| i.item_type).unwrap_or(0);
                    let is_stackable = self
                        .items_db
                        .items
                        .get(&item_type)
                        .map(|t| t.stackable())
                        .unwrap_or(false);
                    if is_stackable {
                        return Ok(());
                    }
                }
                self.player_move_item(
                    conn_id,
                    cid,
                    from_pos,
                    sprite_id,
                    from_stack_pos,
                    to_pos,
                    count,
                    item_id,
                    now,
                )
            }
        }
    }

    /// 772 `TCreature::Move` push-other branch (`cract.cc:489`).
    /// Pushes `moving_creature` from `from_pos` to `to_pos` if it can occupy the tile.
    fn player_push_creature(
        &mut self,
        actor: CreatureId,
        moving_creature: CreatureId,
        from_pos: Position,
        to_pos: Position,
    ) -> Result<(), ReturnValue> {
        let Some(target) = self.creatures.get(moving_creature) else {
            return Err(ReturnValue::NotPossible);
        };
        let pushable = match target {
            CreatureKind::Monster(m) => m.is_pushable(),
            CreatureKind::Player(_) => true,
            CreatureKind::Npc(_) => false,
        };
        if !pushable {
            return Err(ReturnValue::NotPossible);
        }

        let Some(to_tile) = self.map.get_tile(to_pos) else {
            return Err(ReturnValue::NotPossible);
        };
        let rv = crate::walk::tile_query_add_creature(self, to_tile, moving_creature, 0);
        if rv != ReturnValue::NoError {
            return Err(rv);
        }

        // 772 `CheckMapDestination` height-24 gate for up/down creature pushes.
        if crate::walk::walk_tile::tile_has_height_n(
            to_pos,
            to_tile.body(),
            self.items_db.as_ref(),
            &self.items,
            24,
        ) {
            return Err(ReturnValue::NotPossible);
        }

        // 772 `CheckMapDestination` protection-zone gate: reject PZ -> non-PZ pushes.
        if self.tile_in_protection_zone(from_pos) && !self.tile_in_protection_zone(to_pos) {
            return Err(ReturnValue::NotPossible);
        }

        let old_creatures = self
            .map
            .get_tile(from_pos)
            .map(|t| t.body().creatures.clone())
            .unwrap_or_default();

        let kick_dir = crate::walk::direction_from_positions(from_pos, to_pos);

        // 772 `NotifyTurn(Con)` (state only, no 0x6B) before `MoveObject`.
        if let Some(k) = self.creatures.get_mut(moving_creature) {
            crate::walk::set_direction_from_step_for_kick(from_pos, to_pos, k);
        }

        // 772 `AnnounceMovingCreature` — `sendMoveCreature` (0x6D) before `MoveObject`.
        self.broadcast_spectator_move(moving_creature, from_pos, to_pos, &old_creatures);

        // 772 `MoveObject`.
        self.move_creature_on_map(moving_creature, from_pos, to_pos);

        // 772 `NotifyGo` after `MoveObject`.
        self.apply_notify_go_after_relocate(moving_creature, from_pos, to_pos, kick_dir, false);
        self.reschedule_wakeup_for_earliest_walk(moving_creature);

        // 772 `TCreature::Move` `this->Combat.DelayAttack(2000)`.
        if let Some(k) = self.creatures.get_mut(actor) {
            k.base_mut().delay_attack_ms(self.server_ms, 2000);
        }
        Ok(())
    }

    /// 772 `Move` HANG+hook destination walk-to-reach (`operate.cc:538-573`).
    /// Picks the hangable item into an inventory slot, walks to the hook tile, then
    /// re-enqueues a `Move` that will land the item on the hook once the player is in range.
    fn hang_hook_walk_to_reach(
        &mut self,
        cid: CreatureId,
        from_cylinder: Cylinder,
        from_pos: Position,
        to_pos: Position,
        item_id: ItemId,
        count: u8,
        sprite_id: u16,
        now: Instant,
    ) -> Result<(), ReturnValue> {
        // Source must be a map tile — inventory/container to an out-of-range hook is a hard fail.
        if from_pos.x == 0xFFFF {
            return Err(ReturnValue::CannotThrow);
        }

        let (is_stackable, item_count) = {
            let Some(item) = self.items.get(item_id) else {
                return Err(ReturnValue::CannotThrow);
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                return Err(ReturnValue::CannotThrow);
            };
            (it.stackable(), item.count)
        };
        let move_count = if is_stackable { count as u32 } else { item_count as u32 };

        // Find a single inventory slot that can accept this item right now.
        let mut temp_slot = None;
        for slot in 1..=11 {
            if self.player_query_add(cid, slot, item_id, move_count, CylinderFlags::NONE)
                == ReturnValue::NoError
            {
                temp_slot = Some(slot);
                break;
            }
        }
        let Some(temp_slot) = temp_slot else {
            return Err(ReturnValue::CannotThrow);
        };

        // Pick the item up into the temporary inventory slot.
        let temp_cylinder = Cylinder::Inventory {
            player_id: cid,
            slot: temp_slot,
        };
        let moved_id = self
            .internal_move_item(
                Some(cid),
                from_cylinder,
                temp_cylinder,
                item_id,
                if is_stackable { count as u16 } else { item_count },
                CylinderFlags::NONE,
                None,
            )
            .map_err(|_| ReturnValue::CannotThrow)?;

        // Walk to the hook tile so the next `Move` passes `is_hang_hook_accessible`.
        let walk_result = self.setup_player_walk_to_target(cid, to_pos, now);
        if walk_result.is_err() {
            // No path to the hook — put the item back on the ground.
            let _ = self.internal_move_item(
                Some(cid),
                temp_cylinder,
                from_cylinder,
                moved_id,
                u16::MAX,
                CylinderFlags::NO_MERGE,
                None,
            );
            return walk_result.map_err(|_| ReturnValue::ThereIsNoWay);
        }

        let new_obj = ActionObjectRef {
            pos: Position::new(0xFFFF, temp_slot as u16, 0),
            stack_pos: 0,
            sprite_id,
        };

        let has_steps = self
            .creatures
            .get(cid)
            .is_some_and(|k| !k.base().walk_queue.is_empty());
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut()
                .todo
                .queue
                .push_front(CreatureAction::Move {
                    obj: new_obj,
                    dest: to_pos,
                    count,
                });
            if has_steps {
                k.base_mut().todo.queue.push_front(CreatureAction::Go);
            }
        }
        if has_steps && self.todo_start_go_delay(cid, true) {
            self.schedule_immediate_todo_wakeup(cid);
        }
        Ok(())
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
        now: Instant,
    ) -> Result<(), ReturnValue> {
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
        } else {
            return Err(ReturnValue::NotPossible);
        }

        // Resolve cylinders
        let Some(from_cylinder) = self.internal_get_cylinder(cid, from_pos) else {
            return Err(ReturnValue::NotPossible);
        };
        let to_cylinder = if to_pos.x == 0xFFFF && to_pos.y == 0 {
            self.resolve_inventory_any(cid, item_id, count as u32, CylinderFlags::NONE)?
        } else {
            self.internal_get_cylinder(cid, to_pos)
                .ok_or(ReturnValue::NotPossible)?
        };

        let Some(player_pos) = self.creatures.get(cid).map(|p| p.position()) else {
            return Err(ReturnValue::NotPossible);
        };

        // Source z-level check — TFS uses `mapFromPos` (`game.cpp` ~965).
        if from_pos.x != 0xFFFF && player_pos.z != from_pos.z {
            let rv = if player_pos.z > from_pos.z {
                ReturnValue::FirstGoUpStairs
            } else {
                ReturnValue::FirstGoDownStairs
            };
            return Err(rv);
        }

        let map_to_pos = match to_cylinder {
            Cylinder::Tile { pos } => pos,
            Cylinder::Container { .. } | Cylinder::Inventory { .. } => player_pos,
        };

        // 772 `CheckMapDestination` HANG hook destination range check (`operate.cc:538-573`).
        // The generic ObjectInRange/ThrowPossible/IsMapBlocked checks now live in
        // `internal_move_item` so Lua and monster moves also pay them.
        if to_pos.x != 0xFFFF {
            if let Some(tile) = self.map.get_tile(map_to_pos) {
                let body = tile.body();
                if (body.flags & (tilestate::HOOKEAST | tilestate::HOOKSOUTH)) != 0 {
                    if let Some(it) = self.items_db.items.get(&self.items.get(item_id).map(|i| i.item_type).unwrap_or(0)) {
                        if it.is_hangable()
                            && !self.is_hang_hook_accessible(map_to_pos, player_pos, body.flags)
                        {
                            return self.hang_hook_walk_to_reach(
                                cid,
                                from_cylinder,
                                from_pos,
                                to_pos,
                                item_id,
                                count,
                                sprite_id,
                                now,
                            );
                        }
                    }
                }
            }
        }

        let dest_id = match to_cylinder {
            Cylinder::Inventory { player_id, slot } => {
                self.get_player_inventory_item(player_id, slot)
            }
            _ => None,
        };

        // Snapshot source count to detect partial merges (772 `cract.cc:578-599`).
        let source_before = self.items.get(item_id).map(|i| i.count as u32).unwrap_or(1);

        let result = self.internal_move_item(
            Some(cid),
            from_cylinder,
            to_cylinder,
            item_id,
            count as u16,
            CylinderFlags::NONE,
            None,
        );

        let result = match result {
            Ok(r) => Ok(r),
            Err(rv) => match dest_id {
                Some(dest_id)
                    if Self::is_inventory_move_catch(rv) && Some(dest_id) != Some(item_id) =>
                {
                    // 772 catch-and-swap (cract.cc:607-623): move the occupying dest item
                    // back to the source cylinder, then retry the original move while
                    // ignoring the swapped item during CheckTopMoveObject/Merge.
                    self.internal_move_item(
                        Some(cid),
                        to_cylinder,
                        from_cylinder,
                        dest_id,
                        GameWorld::MOVE_ALL,
                        CylinderFlags::NONE,
                        None,
                    )?;
                    self.internal_move_item(
                        Some(cid),
                        from_cylinder,
                        to_cylinder,
                        item_id,
                        count as u16,
                        CylinderFlags::NONE,
                        Some(dest_id),
                    )
                }
                _ => Err(rv),
            },
        };

        result?;

        // 772 `TCreature::Move` merge-then-continue: if only part of the request merged,
        // the rest continues as a separate `Move` with the merge target suppressed.
        let source_after = self.items.get(item_id).map(|i| i.count as u32).unwrap_or(0);
        let requested = (count as u32).min(source_before);
        let moved = source_before - source_after;
        if moved > 0
            && moved < requested
            && self.items.get(item_id).is_some()
            && to_cylinder != from_cylinder
        {
            self.internal_move_item(
                Some(cid),
                from_cylinder,
                to_cylinder,
                item_id,
                (requested - moved) as u16,
                CylinderFlags::NO_MERGE,
                None,
            )?;
        }

        Ok(())
    }

    /// 772 `TCreature::Move` catch-and-swap result list (cract.cc:610):
    /// `NOROOM` / `HANDSNOTFREE` / `HANDBLOCKED` / `ONEWEAPONONLY`.
    fn is_inventory_move_catch(rv: ReturnValue) -> bool {
        matches!(
            rv,
            ReturnValue::NotEnoughRoom
                | ReturnValue::BothHandsNeedToBeFree
                | ReturnValue::PutThisObjectInYourHand
                | ReturnValue::CannotBeDressed
                | ReturnValue::CanOnlyUseOneWeapon
                | ReturnValue::CanOnlyUseOneShield
                | ReturnValue::DropTwoHandedItem
        )
    }



}
