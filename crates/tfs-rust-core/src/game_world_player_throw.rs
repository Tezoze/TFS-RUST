//! Client throw / `playerMoveThing` item path.
//!
//! - `Game::playerMoveThing`, `playerMoveItem` — `game.cpp`.
//! - `Map::canThrowObjectTo` — `map.cpp`.

use std::time::Instant;

use tfs_rust_common::{ConnId, Position, WorldType};

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
        // Extract target position + race flag + type in a scoped borrow so `self` is free
        // for the `object_in_range` / `creature_is_peaceful` / `check_push_destination` calls below.
        let (target_pos, unpushable, moving_is_monster) = {
            let Some(target) = self.creatures.get(moving_creature) else {
                return Err(ReturnValue::NotPossible);
            };
            let pos = target.position();
            // 772 Gate A — `CheckMoveObject` race-flag predicate (`operate.cc:439`):
            //   if GetRaceUnpushable(Race) && (WorldType != NON_PVP || !IsPeaceful()) throw NOTMOVABLE
            // Players/NPCs have no `Race` in the 772 sense for this gate — they're never
            // `GetRaceUnpushable`-blocked (P7: the old hardcoded `Npc(_) => false` is removed;
            // NPC pushability is now governed by Gate B/C `MovePossible`, matching the decompile).
            // `race_unpushable` is the race flag only; the NON_PVP peaceful exception
            // (`crmain.cc:900` base, `crnonpl.cc:2295` `TMonster::IsPeaceful`) is applied below.
            let unpush = match target {
                CreatureKind::Monster(m) => m.race_unpushable(),
                CreatureKind::Player(_) | CreatureKind::Npc(_) => false,
            };
            let is_monster = matches!(target, CreatureKind::Monster(_));
            (pos, unpush, is_monster)
        };
        // P12 (C7) — execute-time `ObjectAccessible(CreatureID, Obj, 1)` re-check
        // (`operate.cc:424` inside `CheckMoveObject`, runs before the race-flag
        // gate at `operate.cc:439`). The 1000ms `ToDoMove` wait (P-B) lets the
        // target walk away; 772 rejects at execute, so must Rust. Checks the
        // creature's **current** position, not the enqueue-time `from_pos`.
        if !self.object_in_range(actor, target_pos, 1) {
            return Err(ReturnValue::NotPossible);
        }
        // Gate A — race-flag predicate with NON_PVP peaceful exception.
        if unpushable
            && !(matches!(self.pvp_config.world_type, WorldType::NoPvp)
                && self.creature_is_peaceful(moving_creature))
        {
            return Err(ReturnValue::NotMoveable);
        }

        // 772 `CheckMapDestination` creature-container arm (`operate.cc:493-532`) —
        // Gate C: range cap (P3), elevation-sum floor-change gate (P5/C1), per-creature
        // `MovePossible` (P-D/Gate B), dest AVOID (P6), PZ→non-PZ, and `ThrowPossible`
        // (P9/C6). Runs **before** the TFS-style `tile_query_add_creature` so 772 gates
        // fire with the correct error codes (e.g. AVOID → `NotEnoughRoom`, not
        // `NotPossible`). `target_pos` is the creature's **current** position (not
        // `from_pos`) — `MovePossible` uses `this->posx/posy/posz` internally.
        self.check_push_destination(
            moving_creature,
            from_pos,
            to_pos,
            target_pos,
            moving_is_monster,
        )?;

        let Some(to_tile) = self.map.get_tile(to_pos) else {
            return Err(ReturnValue::NotPossible);
        };
        let rv = crate::walk::tile_query_add_creature(self, to_tile, moving_creature, 0);
        if rv != ReturnValue::NoError {
            return Err(rv);
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
        self.flush_pending_creature_step_events();

        // 772 `NotifyGo` after `MoveObject`.
        self.apply_notify_go_after_relocate(moving_creature, from_pos, to_pos, kick_dir, false);
        self.reschedule_wakeup_for_earliest_walk(moving_creature);

        // 772 `TCreature::Move` `this->Combat.DelayAttack(2000)`.
        if let Some(k) = self.creatures.get_mut(actor) {
            k.base_mut().delay_attack_ms(self.server_ms, 2000);
        }
        Ok(())
    }

    /// 772 `CoordinateFlag(x,y,z, AVOID)` (`operate.cc:525`, `enums.hh:240`).
    /// `AVOID` maps to the `MAGICFIELD` tile-state bit (`walk/walk_tile.rs:656`).
    pub(crate) fn tile_has_avoid(&self, pos: Position) -> bool {
        self.map
            .get_tile(pos)
            .is_some_and(|t| t.body().flags & tilestate::MAGICFIELD != 0)
    }

    /// 772 `CheckMapDestination` creature-container arm (`operate.cc:493-532`).
    ///
    /// Pre: `moving != actor` (pushing another creature), both on map tiles, `to` is a map tile.
    /// `moving_is_monster` selects the `OrigZ == DestZ || Type != MONSTER` guard (C5) —
    /// when the moving creature is a monster changing floors (`dz != 0`), `MovePossible` and
    /// the AVOID/PZ checks are **skipped** (the decompile only runs them for same-floor or
    /// non-monster pushes). `origin` is the moving creature's **current** position (used by
    /// `MovePossible` as `this->posx/posy/posz`).
    fn check_push_destination(
        &mut self,
        moving_creature: CreatureId,
        from: Position,
        to: Position,
        origin: Position,
        moving_is_monster: bool,
    ) -> Result<(), ReturnValue> {
        // P3: 1-tile range cap (`operate.cc:493-496`).
        let dx = (to.x as i32 - from.x as i32).abs();
        let dy = (to.y as i32 - from.y as i32).abs();
        let dz = (to.z as i32 - from.z as i32).abs();
        if dx > 1 || dy > 1 || dz > 1 {
            return Err(ReturnValue::DestinationOutOfReach); // OUTOFRANGE
        }

        // P5 (C1): up/down elevation-sum split (`operate.cc:499-507`).
        // Up-floor checks **origin** elevation sum; down-floor checks **dest** elevation sum.
        // NOT `tile_has_height_n` — that is a hasHeight count; this is `GetHeight` (`info.cc:689`).
        if to.z == from.z - 1 {
            // Up a floor — origin must have enough elevation to jump up.
            let elev = self
                .map
                .get_tile(from)
                .map(|t| {
                    crate::walk::walk_tile::tile_elevation_sum(
                        t.body(),
                        self.items_db.as_ref(),
                        &self.items,
                    )
                })
                .unwrap_or(0);
            if elev < 24 {
                return Err(ReturnValue::NotEnoughRoom); // NOROOM
            }
        } else if to.z == from.z + 1 {
            // Down a floor — dest must have enough elevation to jump down.
            let elev = self
                .map
                .get_tile(to)
                .map(|t| {
                    crate::walk::walk_tile::tile_elevation_sum(
                        t.body(),
                        self.items_db.as_ref(),
                        &self.items,
                    )
                })
                .unwrap_or(0);
            if elev < 24 {
                return Err(ReturnValue::ThereIsNoWay); // NOWAY
            }
        }

        // C5: `MovePossible`, AVOID, and PZ checks are nested inside
        // `if(OrigZ == DestZ || Type != MONSTER)` (`operate.cc:515-532`). When the moving
        // creature is a monster changing floors (`dz != 0`), all three are skipped.
        if dz == 0 || !moving_is_monster {
            // P-D / Gate B — per-creature `MovePossible(Execute=true, OrigZ != DestZ)`
            // (`operate.cc:515-516`). Dispatch on the moving creature's type.
            // C2/C3: thrown results (`ENTERPROTECTIONZONE`, `NOTINVITED`, `EXHAUSTED`)
            // propagate unchanged; `Ok(false)` → `NOROOM` for pushing another creature
            // (`operate.cc:517-518`: `CreatureID != MovingCreature->ID` → `NOROOM`).
            let jump = from.z != to.z;
            let result = match self.creatures.get(moving_creature) {
                Some(CreatureKind::Player(_)) => {
                    self.player_move_possible_push(moving_creature, to, origin, jump)
                }
                Some(CreatureKind::Monster(_)) => {
                    self.monster_move_possible_push(moving_creature, to)
                }
                Some(CreatureKind::Npc(_)) => {
                    self.npc_move_possible_push(moving_creature, to)
                }
                None => Ok(false),
            };
            match result {
                // C2/C3: thrown results propagate unchanged (ENTERPROTECTIONZONE,
                // NOTINVITED, EXHAUSTED) — not remapped to NOROOM.
                Err(rv) => return Err(rv),
                // `Ok(false)` → NOROOM for pushing another creature (`operate.cc:518`).
                Ok(false) => return Err(ReturnValue::NotEnoughRoom),
                Ok(true) => {}
            }

            // P6: dest AVOID (magic field) blocks pushing another creature (`operate.cc:525`).
            if self.tile_has_avoid(to) {
                return Err(ReturnValue::NotEnoughRoom); // NOROOM
            }

            // PZ → non-PZ (`operate.cc:529-530`).
            if self.tile_in_protection_zone(from) && !self.tile_in_protection_zone(to) {
                return Err(ReturnValue::ActionNotPermittedInProtectionZone); // PROTECTIONZONE
            }
        }

        // P9 (C6): `ThrowPossible(Orig, Dest, 1)` (`operate.cc:576`) — NOT trivially true;
        // tests `UNTHROW` on the dest tile + floor-descent loop for `dz != 0`.
        if !self.map.throw_possible(from, to, 1) {
            return Err(ReturnValue::CannotThrow); // CANNOTTHROW
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

#[cfg(test)]
mod push_gate_a_tests {
    //! Phase P-A — 772 Gate A pushability predicate (`operate.cc:439` `CheckMoveObject`).
    use super::*;
    use crate::creature::MonsterAiConfig;
    use crate::sim_harness::{
        beat_driven_world, ensure_walkable_tile, insert_monster_with_config, insert_player,
        test_player,
    };
    use tfs_rust_common::Position;

    /// Walkable `from`/`to` tiles + moving creature registered at `from`. Returns `(actor, mover)`.
    ///
    /// **P-B (P12):** the actor is placed **adjacent** to `from` (within range 1)
    /// so the execute-time `ObjectAccessible(CreatureID, Obj, 1)` re-check passes.
    /// The old P-A setup placed the actor 2 tiles away (only needed for
    /// `DelayAttack`); P12's adjacency re-check now rejects that.
    fn setup_push_arena(
        world: &mut GameWorld,
        from: Position,
        to: Position,
        mover_cfg: MonsterAiConfig,
    ) -> (CreatureId, CreatureId) {
        ensure_walkable_tile(&mut world.map, from, 1);
        ensure_walkable_tile(&mut world.map, to, 1);
        // Actor player — adjacent to `from` for P12's `ObjectAccessible(…, 1)`.
        // Place on the opposite side of `to` to avoid overlapping the destination.
        let actor_pos = Position::new(from.x, from.y.saturating_sub(1), from.z);
        ensure_walkable_tile(&mut world.map, actor_pos, 1);
        let actor = insert_player(world, test_player("Actor", actor_pos));
        let mover = insert_monster_with_config(world, "Mover", from, 200, mover_cfg);
        (actor, mover)
    }

    /// P2: PVP world, `unpushable`-race monster → Gate A blocks with `NotMoveable`
    /// (772 `NOTMOVABLE`), regardless of speed.
    #[test]
    fn pvp_unpushable_race_monster_blocked() {
        let mut world = beat_driven_world();
        // default world_type is Pvp.
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        let cfg = MonsterAiConfig {
            pushable: false,
            ..MonsterAiConfig::default()
        };
        let (actor, mover) = setup_push_arena(&mut world, from, to, cfg);

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::NotMoveable));
    }

    /// P2 core divergence: a `pushable`-race monster with `speed == 0` **passes** Gate A.
    /// 772 `GetRaceUnpushable` consults the race flag only — not speed. The old TFS
    /// `is_pushable()` (`pushable && speed != 0`) would have blocked this; P-A removes that.
    #[test]
    fn speed_zero_pushable_race_passes_gate_a() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        // pushable race (default true), but speed 0.
        let (actor, mover) = setup_push_arena(&mut world, from, to, MonsterAiConfig::default());
        if let Some(k) = world.creatures.get_mut(mover) {
            k.base_mut().speed = 0;
        }

        // Gate A passes; the full push succeeds on the valid arena.
        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Ok(()));
    }

    /// P2 NON_PVP exception: a player-summon of an `unpushable` race is peaceful
    /// (`crnonpl.cc:2295` `TMonster::IsPeaceful`) → pushable in NON_PVP.
    #[test]
    fn nopvp_peaceful_summon_of_unpushable_race_pushable() {
        let mut world = beat_driven_world();
        world.pvp_config.world_type = WorldType::NoPvp;
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        let cfg = MonsterAiConfig {
            pushable: false,
            ..MonsterAiConfig::default()
        };
        let (actor, mover) = setup_push_arena(&mut world, from, to, cfg);
        // Make the mover a player-summon → peaceful.
        if let Some(k) = world.creatures.get_mut(mover) {
            k.base_mut().master = Some(actor);
        }

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Ok(()));
    }

    /// P2: NON_PVP world, `unpushable`-race monster that is **not** peaceful (no player
    /// master) → still blocked. The peaceful exception only covers peaceful creatures.
    #[test]
    fn nopvp_unpushable_non_peaceful_monster_blocked() {
        let mut world = beat_driven_world();
        world.pvp_config.world_type = WorldType::NoPvp;
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        let cfg = MonsterAiConfig {
            pushable: false,
            ..MonsterAiConfig::default()
        };
        let (actor, mover) = setup_push_arena(&mut world, from, to, cfg);
        // No master → not peaceful.

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::NotMoveable));
    }

    /// Baseline: a normal (`pushable`-race) monster is pushable in PVP.
    #[test]
    fn normal_monster_pushable_in_pvp() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        let (actor, mover) = setup_push_arena(&mut world, from, to, MonsterAiConfig::default());

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Ok(()));
    }

    /// P7: NPC pushability is no longer hardcoded `false`. NPCs have no 772 `Race` for
    /// Gate A → `unpushable = false` → passes Gate A. (Gate B/C `MovePossible` govern NPC
    /// pushability, not Gate A.) Verified in NON_PVP where the old code also blocked.
    #[test]
    fn npc_passes_gate_a() {
        use crate::creature::{CreatureBase, Npc, Outfit};
        use tfs_rust_common::enums::{Direction, SkullType};

        let mut world = beat_driven_world();
        world.pvp_config.world_type = WorldType::NoPvp;
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, from, 1);
        ensure_walkable_tile(&mut world.map, to, 1);
        let actor_pos = Position::new(100, 99, 7); // P12: adjacent to `from` (dy=1)
        ensure_walkable_tile(&mut world.map, actor_pos, 1);
        let actor = insert_player(&mut world, test_player("Actor", actor_pos));

        // Build a minimal NPC at `from` (no definition DB entry — `Npc::placeholder`).
        let base = CreatureBase {
            name: "Npc".into(),
            position: from,
            direction: Direction::North,
            health: 100,
            max_health: 100,
            outfit: Outfit::default(),
            speed: 220,
            base_speed: 220,
            var_speed: 0,
            skull: SkullType::None,
            drunkenness: 0,
            active_conditions: Vec::new(),
            walk_queue: Default::default(),
            walk_destinations: Default::default(),
            last_step: None,
            last_step_cost: 1,
            last_step_ground_speed: 150,
            next_wakeup: None,
            last_step_server_ms: None,
            earliest_walk_server_ms: 0,
            earliest_spell_server_ms: 0,
            earliest_multiuse_server_ms: 0,
            cancel_next_walk: false,
            force_update_follow_path: false,
            walk_update_ticks: 0,
            is_updating_path: false,
            has_follow_path: false,
            movement_blocked: false,
            stairhop_blocked_until: None,
            follow_target: None,
            attack_target: None,
            master: None,
            damage_map: Default::default(),
            last_hit_by: None,
            poison_damage_origin: None,
            fire_damage_origin: None,
            energy_damage_origin: None,
            earliest_attack_ms: 0,
            latest_attack_round: 0,
            earliest_defend_ms: 0,
            last_defend_ms: 0,
            learning_points: 0,
            todo: Default::default(),
            chase_mode: Default::default(),
            last_auto_walk_armed_ms: u64::MAX,
        };
        let npc = world
            .creatures
            .insert(CreatureKind::Npc(Npc::placeholder(base)));
        world.map.register_creature_at(from, npc);

        // Gate A passes (NPC has no race flag). The push may fail at a later gate, but it
        // must NOT be `NotMoveable` (the old hardcoded `Npc => false` return).
        let rv = world.player_push_creature(actor, npc, from, to);
        assert_ne!(rv, Err(ReturnValue::NotMoveable));
    }
}

#[cfg(test)]
mod push_phase_b_tests {
    //! Phase P-B — `ToDoMove` creature-container delay (P1) + execute-time
    //! adjacency re-check (P12). See `docs/772_PLAYER_PUSH_AUDIT.md` §4 P-B.
    use super::*;
    use crate::creature_todo::CreatureAction;
    use crate::sim_harness::{
        beat_driven_world, ensure_walkable_tile, insert_monster_with_config, insert_player,
        test_player,
    };
    use tfs_rust_common::Position;

    /// Default outfit `look_type` for `insert_monster_with_config` (`Outfit::default`).
    /// `internal_get_thing_move` resolves a creature via `find_tile_creature_by_client_sprite`
    /// when this matches the wire `sprite_id` and no item on the tile matches first.
    const MONSTER_SPRITE: u16 = 136;

    /// Set up a push arena: actor adjacent to `from`, monster at `from`, dest at `to`.
    /// `server_ms` is set to `server_ms`. Returns `(actor, mover, obj)`.
    fn setup_push_arena_pb(
        world: &mut GameWorld,
        server_ms: u64,
        from: Position,
        to: Position,
    ) -> (CreatureId, CreatureId, ActionObjectRef) {
        world.server_ms = server_ms;
        ensure_walkable_tile(&mut world.map, from, 1);
        ensure_walkable_tile(&mut world.map, to, 1);
        // Actor adjacent to `from` (dy=1) for P12's `ObjectAccessible(…, 1)`.
        let actor_pos = Position::new(from.x, from.y.saturating_sub(1), from.z);
        ensure_walkable_tile(&mut world.map, actor_pos, 1);
        let actor = insert_player(world, test_player("Actor", actor_pos));
        let mover = insert_monster_with_config(world, "Mover", from, 200, Default::default());
        let obj = ActionObjectRef {
            pos: from,
            stack_pos: 0,
            sprite_id: MONSTER_SPRITE,
        };
        (actor, mover, obj)
    }

    /// P1: creature-container push enqueues `Wait{1000}` (not `Wait{100}`).
    /// 772 `cract.cc:1156-1159`: `Delay = 1000` for creature containers.
    #[test]
    fn creature_push_enqueues_1000ms_delay() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        let (actor, _mover, obj) = setup_push_arena_pb(&mut world, 5000, from, to);

        world.enqueue_player_move(actor, obj, to, 1).expect("creature push enqueues");

        let todo = &world.creatures.get(actor).unwrap().base().todo;
        assert_eq!(todo.queue.len(), 2, "creature push → [Wait(1000), Move]");
        // Deadline = server_ms + 1000 = 5000 + 1000 = 6000.
        assert!(matches!(
            todo.queue[0],
            CreatureAction::Wait { deadline_ms: 6000 }
        ));
        assert!(matches!(todo.queue[1], CreatureAction::Move { .. }));
    }

    /// P1: creature-container push adds the remaining walk cooldown to the 1000ms delay.
    /// 772 `cract.cc:1157-1159`: `if EarliestWalkTime > ServerMilliseconds:
    /// Delay += (int)(EarliestWalkTime - ServerMilliseconds)`.
    #[test]
    fn creature_push_delay_includes_walk_cooldown() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        let (actor, _mover, obj) = setup_push_arena_pb(&mut world, 5000, from, to);
        // 3000ms walk cooldown remaining → delay = 1000 + 3000 = 4000.
        world
            .creatures
            .get_mut(actor)
            .unwrap()
            .base_mut()
            .earliest_walk_server_ms = 8000;

        world.enqueue_player_move(actor, obj, to, 1).expect("creature push enqueues");

        let todo = &world.creatures.get(actor).unwrap().base().todo;
        // Deadline = server_ms + 1000 + 3000 = 5000 + 4000 = 9000.
        assert!(matches!(
            todo.queue[0],
            CreatureAction::Wait { deadline_ms: 9000 }
        ));
    }

    /// P1: creature-container push to a dest with no ground (BANK) → `Err(NotPossible)`.
    /// 772 `cract.cc:1145-1148`: `DestBank = GetFirstObject(DestX, DestY, DestZ)`;
    /// if `DestBank == NONE || !BANK` → throw NOTACCESSIBLE.
    #[test]
    fn creature_push_dest_without_ground_rejected() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        // Only set up `from` tile + actor; leave `to` without a tile (no ground).
        world.server_ms = 5000;
        ensure_walkable_tile(&mut world.map, from, 1);
        let actor_pos = Position::new(100, 99, 7);
        ensure_walkable_tile(&mut world.map, actor_pos, 1);
        let actor = insert_player(&mut world, test_player("Actor", actor_pos));
        insert_monster_with_config(&mut world, "Mover", from, 200, Default::default());
        let obj = ActionObjectRef {
            pos: from,
            stack_pos: 0,
            sprite_id: MONSTER_SPRITE,
        };

        let rv = world.enqueue_player_move(actor, obj, to, 1);
        assert_eq!(rv, Err(ReturnValue::NotPossible));
    }

    /// P12 (C7): execute-time `ObjectAccessible(…, 1)` re-check — if the target
    /// walks out of range during the 1000ms wait, the push is rejected at execute.
    /// `player_push_creature` checks the creature's **current** position, not
    /// `from_pos`. Here the actor is 2 tiles from the target → rejected.
    #[test]
    fn p12_target_out_of_range_rejected() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, from, 1);
        ensure_walkable_tile(&mut world.map, to, 1);
        // Actor 2 tiles away (dy=2) — NOT within range 1.
        let actor_pos = Position::new(100, 98, 7);
        ensure_walkable_tile(&mut world.map, actor_pos, 1);
        let actor = insert_player(&mut world, test_player("Actor", actor_pos));
        let mover = insert_monster_with_config(&mut world, "Mover", from, 200, Default::default());

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::NotPossible));
    }

    /// P12 (C7): when the target is adjacent, the push succeeds (positive case).
    /// Verifies the adjacency check doesn't over-reject.
    #[test]
    fn p12_target_in_range_succeeds() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, from, 1);
        ensure_walkable_tile(&mut world.map, to, 1);
        // Actor adjacent (dy=1) — within range 1.
        let actor_pos = Position::new(100, 99, 7);
        ensure_walkable_tile(&mut world.map, actor_pos, 1);
        let actor = insert_player(&mut world, test_player("Actor", actor_pos));
        let mover = insert_monster_with_config(&mut world, "Mover", from, 200, Default::default());

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Ok(()));
    }

    /// P1: creature-container push with `earliest_walk_server_ms` in the past
    /// (cooldown already expired) → delay is exactly 1000ms (no negative addition).
    #[test]
    fn creature_push_expired_cooldown_is_exactly_1000ms() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        let (actor, _mover, obj) = setup_push_arena_pb(&mut world, 5000, from, to);
        // Cooldown already expired (earliest_walk < server_ms).
        world
            .creatures
            .get_mut(actor)
            .unwrap()
            .base_mut()
            .earliest_walk_server_ms = 3000;

        world.enqueue_player_move(actor, obj, to, 1).expect("creature push enqueues");

        let todo = &world.creatures.get(actor).unwrap().base().todo;
        // Deadline = 5000 + 1000 = 6000 (no cooldown addition).
        assert!(matches!(
            todo.queue[0],
            CreatureAction::Wait { deadline_ms: 6000 }
        ));
    }
}

#[cfg(test)]
mod push_phase_c_tests {
    //! Phase P-C — `CheckMapDestination` creature-container arm (`operate.cc:493-532`).
    //! Tests P3 (range cap), P5 (elevation-sum gate), P6 (dest AVOID), P9 (ThrowPossible),
    //! and PZ→non-PZ. See `docs/772_PLAYER_PUSH_AUDIT.md` §4 P-C.
    use super::*;
    use crate::sim_harness::{
        beat_driven_world, ensure_walkable_tile, insert_monster_with_config, insert_player,
        test_player,
    };
    use tfs_rust_common::enums::ZoneType;
    use tfs_rust_common::Position;
    use tfs_rust_content::otb::ItemType;

    /// Item type ID for test height items (avoids collision with 1987/2148).
    const HEIGHT_ITEM_TYPE: u16 = 500;

    /// Create an `ItemType` with `has_height` flag and the given `elevation` value.
    /// 772 `HEIGHT` flag + `ELEVATION` attribute (`enums.hh:760`, `info.cc:689`).
    /// `FLAG_HAS_HEIGHT` = `1 << 3` (private in `ItemType` — use raw value).
    fn height_item_type(elevation: i32) -> ItemType {
        ItemType {
            server_id: HEIGHT_ITEM_TYPE,
            flags: 1 << 3, // FLAG_HAS_HEIGHT
            elevation,
            ..Default::default()
        }
    }

    /// Set up a push arena: actor adjacent to `from`, monster at `from`, dest at `to`.
    /// Optionally registers a height item type with the given elevation in the items_db.
    fn setup_push_arena_pc(
        world: &mut GameWorld,
        from: Position,
        to: Position,
        height_elevation: Option<i32>,
    ) -> (CreatureId, CreatureId) {
        ensure_walkable_tile(&mut world.map, from, 1);
        ensure_walkable_tile(&mut world.map, to, 1);
        let actor_pos = Position::new(from.x, from.y.saturating_sub(1), from.z);
        ensure_walkable_tile(&mut world.map, actor_pos, 1);
        let actor = insert_player(world, test_player("Actor", actor_pos));
        let mover = insert_monster_with_config(world, "Mover", from, 200, Default::default());
        if let Some(elev) = height_elevation {
            let mut new_db = (*world.items_db).clone();
            new_db.items.insert(HEIGHT_ITEM_TYPE, height_item_type(elev));
            world.items_db = std::sync::Arc::new(new_db);
        }
        (actor, mover)
    }

    /// Place a height item (type `HEIGHT_ITEM_TYPE`) on the tile at `pos`.
    fn place_height_item(world: &mut GameWorld, pos: Position) {
        let item_id = world.items.insert(crate::item::Item::new_single(HEIGHT_ITEM_TYPE));
        if let Some(tile) = world.map.get_tile_mut(pos) {
            tile.body_mut().down_items.push(item_id);
        }
    }

    /// Set a tile's zone to Protection.
    fn set_pz_tile(world: &mut GameWorld, pos: Position) {
        if let Some(tile) = world.map.get_tile_mut(pos) {
            tile.body_mut().zone = ZoneType::Protection;
        }
    }

    /// Set a tile's flags to include MAGICFIELD (772 AVOID).
    fn set_avoid_tile(world: &mut GameWorld, pos: Position) {
        if let Some(tile) = world.map.get_tile_mut(pos) {
            tile.body_mut().flags |= tilestate::MAGICFIELD;
        }
    }

    /// Set a tile's flags to include UNTHROW.
    fn set_unthrow_tile(world: &mut GameWorld, pos: Position) {
        if let Some(tile) = world.map.get_tile_mut(pos) {
            tile.body_mut().flags |= tilestate::UNTHROW;
        }
    }

    /// P3: push 2 tiles away → `DestinationOutOfReach` (772 `OUTOFRANGE`).
    #[test]
    fn p3_two_tile_push_rejected() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(102, 100, 7); // dx=2 > 1
        let (actor, mover) = setup_push_arena_pc(&mut world, from, to, None);

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::DestinationOutOfReach));
    }

    /// P3: push 1 tile away → succeeds (positive case).
    #[test]
    fn p3_one_tile_push_succeeds() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7); // dx=1
        let (actor, mover) = setup_push_arena_pc(&mut world, from, to, None);

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Ok(()));
    }

    /// P5 (C1): push up a floor from a low-elevation-sum origin → `NotEnoughRoom`
    /// (772 `NOROOM`). Origin elevation sum < 24.
    #[test]
    fn p5_push_up_low_elevation_origin_rejected() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(100, 100, 6); // up a floor (to.z = from.z - 1)
        let (actor, mover) = setup_push_arena_pc(&mut world, from, to, Some(23));
        // Place a height item with elevation 23 on the origin tile → sum = 23 < 24.
        place_height_item(&mut world, from);
        // Dest tile at z=6 must exist.
        ensure_walkable_tile(&mut world.map, to, 1);

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::NotEnoughRoom)); // NOROOM
    }

    /// P5 (C1): push up a floor from a sufficient-elevation-sum origin → passes the
    /// elevation gate (elevation sum ≥ 24). May fail at a later gate, but not NOROOM.
    #[test]
    fn p5_push_up_sufficient_elevation_passes_height_gate() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(100, 100, 6);
        let (actor, mover) = setup_push_arena_pc(&mut world, from, to, Some(24));
        place_height_item(&mut world, from); // sum = 24 ≥ 24
        ensure_walkable_tile(&mut world.map, to, 1);

        let rv = world.player_push_creature(actor, mover, from, to);
        // Should NOT be NotEnoughRoom (elevation gate passed). May fail at ThrowPossible
        // or MovePossible (P-D), but the height gate itself passes.
        assert_ne!(rv, Err(ReturnValue::NotEnoughRoom));
    }

    /// P5 (C1): push down a floor to a low-elevation-sum dest → `ThereIsNoWay`
    /// (772 `NOWAY`). Dest elevation sum < 24.
    #[test]
    fn p5_push_down_low_elevation_dest_rejected() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(100, 100, 8); // down a floor (to.z = from.z + 1)
        let (actor, mover) = setup_push_arena_pc(&mut world, from, to, Some(23));
        // Place a height item with elevation 23 on the DEST tile → sum = 23 < 24.
        place_height_item(&mut world, to);
        ensure_walkable_tile(&mut world.map, to, 1);

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::ThereIsNoWay)); // NOWAY
    }

    /// P5 (C1): same-floor push ignores elevation entirely — no height items needed.
    /// The elevation gate only fires for `dz != 0`.
    #[test]
    fn p5_same_floor_push_ignores_elevation() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7); // dz = 0
        let (actor, mover) = setup_push_arena_pc(&mut world, from, to, None);
        // No height items anywhere — elevation sum = 0. Same-floor push should succeed.

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Ok(()));
    }

    /// P6: push onto a magic-field tile (772 `AVOID`) → `NotEnoughRoom` (772 `NOROOM`).
    /// `AVOID` maps to `MAGICFIELD` tile-state (`walk/walk_tile.rs:656`).
    #[test]
    fn p6_push_onto_magic_field_rejected() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        let (actor, mover) = setup_push_arena_pc(&mut world, from, to, None);
        set_avoid_tile(&mut world, to);

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::NotEnoughRoom)); // NOROOM
    }

    /// PZ → non-PZ: pushing a creature from a PZ tile to a non-PZ tile →
    /// `ActionNotPermittedInProtectionZone` (772 `PROTECTIONZONE`).
    #[test]
    fn pz_to_non_pz_push_rejected() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        let (actor, mover) = setup_push_arena_pc(&mut world, from, to, None);
        set_pz_tile(&mut world, from);

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::ActionNotPermittedInProtectionZone));
    }

    /// P9 (C6): push onto a `UNTHROW` dest tile → `CannotThrow` (772 `CANNOTTHROW`).
    /// Requires a floor tile below the origin so `ThrowPossible` checks the dest at z.
    #[test]
    fn p9_push_onto_unthrow_dest_rejected() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        let (actor, mover) = setup_push_arena_pc(&mut world, from, to, None);
        // Place a ground tile at z=6 below the origin so `ThrowPossible` checks z=7.
        ensure_walkable_tile(&mut world.map, Position::new(100, 100, 6), 1);
        set_unthrow_tile(&mut world, to);

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::CannotThrow));
    }

    /// C5: monster changing floors (dz != 0) skips the AVOID/PZ guard.
    /// A monster pushed down a floor onto an AVOID tile should NOT be rejected by
    /// the AVOID check (the guard `dz == 0 || !moving_is_monster` is false).
    /// It may fail at the elevation gate or ThrowPossible, but not at AVOID.
    #[test]
    fn c5_monster_floor_change_skips_avoid_guard() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(100, 100, 8); // down a floor, dz=1
        let (actor, mover) = setup_push_arena_pc(&mut world, from, to, Some(24));
        // Place height item on dest with elevation 24 → passes elevation gate.
        place_height_item(&mut world, to);
        ensure_walkable_tile(&mut world.map, to, 1);
        set_avoid_tile(&mut world, to); // AVOID on dest — should be skipped for monster floor-change

        let rv = world.player_push_creature(actor, mover, from, to);
        // Should NOT be NotEnoughRoom (AVOID is skipped for monster floor-change).
        // May fail at ThrowPossible, but the AVOID check must not fire.
        assert_ne!(rv, Err(ReturnValue::NotEnoughRoom));
    }
}

#[cfg(test)]
mod push_phase_d_tests {
    //! Phase P-D — per-creature `MovePossible(Execute=true)` push predicates (Gate B).
    //! Tests P4: home range, house, PZ, GO_STRENGTH, summon anti-crowd (C4), kick EXHAUSTED
    //! (C3), NPC radius, player PZ-enter (C2), player house-invite (C2).
    //! See `docs/772_PLAYER_PUSH_AUDIT.md` §4 P-D.
    use super::*;
    use crate::creature::{CreatureBase, MonsterAiConfig, MonsterState, Npc, Outfit, Player};
    use crate::sim_harness::{
        beat_driven_world, ensure_walkable_tile, insert_monster_with_config, insert_player,
        synthetic_ground_item_type, test_player,
    };
    use crate::tile::{HouseTile, Tile, TileBody};
    use tfs_rust_common::enums::{Direction, SkullType, ZoneType};
    use tfs_rust_common::Position;

    /// Register ground type 1 as a BANK ground tile in the items_db.
    /// `beat_driven_world()` doesn't register ground type 1 by default — needed for
    /// `tile_is_bank_and_passable` (772 `BANK` flag) in player `MovePossible`.
    fn register_ground_bank(world: &mut GameWorld) {
        let mut new_db = (*world.items_db).clone();
        new_db.items.insert(1, synthetic_ground_item_type(1, 150));
        world.items_db = std::sync::Arc::new(new_db);
    }

    /// Insert a player AND register them on the map tile (unlike `insert_player` which only
    /// adds to the SlotMap). Needed for tile-creature lookups in `MovePossible`/kick loop.
    fn insert_player_on_tile(world: &mut GameWorld, player: Player, pos: Position) -> CreatureId {
        let cid = world.creatures.insert(CreatureKind::Player(player));
        world.map.register_creature_at(pos, cid);
        cid
    }

    /// Set a tile's zone to Protection.
    fn set_pz_tile(world: &mut GameWorld, pos: Position) {
        if let Some(tile) = world.map.get_tile_mut(pos) {
            tile.body_mut().zone = ZoneType::Protection;
            tile.body_mut().flags |= tilestate::PROTECTIONZONE;
        }
    }

    /// Insert a house tile at `pos` with the given `house_id`.
    fn insert_house_tile(world: &mut GameWorld, pos: Position, house_id: u32) {
        world.map.insert_tile(
            pos,
            Tile::House(HouseTile {
                inner: TileBody {
                    ground: Some(1),
                    ground_item: None,
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: 0,
                    zone: ZoneType::Normal,
                },
                house_id,
            }),
        );
    }

    /// Set up a push arena: actor adjacent to `from`, monster at `from`, dest at `to`.
    /// Returns `(actor, mover)`.
    fn setup_push_arena_pd(
        world: &mut GameWorld,
        from: Position,
        to: Position,
    ) -> (CreatureId, CreatureId) {
        ensure_walkable_tile(&mut world.map, from, 1);
        ensure_walkable_tile(&mut world.map, to, 1);
        let actor_pos = Position::new(from.x, from.y.saturating_sub(1), from.z);
        ensure_walkable_tile(&mut world.map, actor_pos, 1);
        let actor = insert_player(world, test_player("Actor", actor_pos));
        let mover = insert_monster_with_config(world, "Mover", from, 200, MonsterAiConfig::default());
        (actor, mover)
    }

    /// P4: push a monster out of its home range → `NotEnoughRoom` (772 `NOROOM`).
    /// `crnonpl.cc:2148-2151`: `MonsterhomeInRange(this->Home, x, y, z)` → false.
    #[test]
    fn p4_push_monster_out_of_home_range_rejected() {
        let mut world = beat_driven_world();
        // Spawn at (100,100,7); monster moved to (101,100,7); push to (102,100,7).
        // Dest (102,100,7) is distance 2 from spawn → outside home_radius=1.
        let from = Position::new(101, 100, 7);
        let to = Position::new(102, 100, 7);
        ensure_walkable_tile(&mut world.map, from, 1);
        ensure_walkable_tile(&mut world.map, to, 1);
        let actor_pos = Position::new(101, 99, 7);
        ensure_walkable_tile(&mut world.map, actor_pos, 1);
        let actor = insert_player(&mut world, test_player("Actor", actor_pos));
        let mover = insert_monster_with_config(&mut world, "Mover", from, 200, MonsterAiConfig::default());
        // Set spawn at (100,100,7) and home_radius=1.
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.spawn_position = Position::new(100, 100, 7);
            m.home_radius = 1;
        }

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::NotEnoughRoom)); // NOROOM
    }

    /// P4: push a monster into a house tile → `NotEnoughRoom` (772 `NOROOM`).
    /// `crnonpl.cc:2168`: `IsHouse(x, y, z)` → false.
    #[test]
    fn p4_push_monster_into_house_rejected() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        let (actor, mover) = setup_push_arena_pd(&mut world, from, to);
        // Replace dest with a house tile.
        insert_house_tile(&mut world, to, 1);

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::NotEnoughRoom)); // NOROOM
    }

    /// P4: push a monster into a PZ tile → `NotEnoughRoom` (772 `NOROOM`).
    /// `crnonpl.cc:2168`: `IsProtectionZone(x, y, z)` → false.
    #[test]
    fn p4_push_monster_into_pz_rejected() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        let (actor, mover) = setup_push_arena_pd(&mut world, from, to);
        set_pz_tile(&mut world, to);

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::NotEnoughRoom)); // NOROOM
    }

    /// P4: push a monster with `GO_STRENGTH < 0` (`speed < 0`) → `NotEnoughRoom`.
    /// `crnonpl.cc:2162`: `Skills[SKILL_GO_STRENGTH]->Act < 0` → false.
    #[test]
    fn p4_push_monster_negative_go_strength_rejected() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        let (actor, mover) = setup_push_arena_pd(&mut world, from, to);
        // GO_STRENGTH < 0 → speed < 0.
        if let Some(k) = world.creatures.get_mut(mover) {
            k.base_mut().speed = -1;
        }

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::NotEnoughRoom)); // NOROOM
    }

    /// P4/C4: push a summon currently 2 tiles from master onto a tile adjacent to master
    /// → `NotEnoughRoom` (anti-crowd rule, NOT a leash/distance cap).
    /// `crnonpl.cc:2171-2181`: summon not-adjacent → dest-adjacent rejected.
    #[test]
    fn p4_c4_summon_anti_crowd_rejected() {
        let mut world = beat_driven_world();
        // Master at (100,100,7); summon at (102,100,7) (manhattan 2); push to (101,100,7)
        // (manhattan 1 from master — adjacent).
        let master_pos = Position::new(100, 100, 7);
        let from = Position::new(102, 100, 7);
        let to = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, master_pos, 1);
        ensure_walkable_tile(&mut world.map, from, 1);
        ensure_walkable_tile(&mut world.map, to, 1);
        let actor_pos = Position::new(102, 99, 7);
        ensure_walkable_tile(&mut world.map, actor_pos, 1);
        let master = insert_player(&mut world, test_player("Master", master_pos));
        let actor = insert_player(&mut world, test_player("Actor", actor_pos));
        let mover = insert_monster_with_config(&mut world, "Summon", from, 200, MonsterAiConfig::default());
        // Make the mover a summon (master set) — not ATTACKING/PANIC.
        if let Some(k) = world.creatures.get_mut(mover) {
            k.base_mut().master = Some(master);
        }

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::NotEnoughRoom)); // NOROOM (anti-crowd)
    }

    /// P4/C4: push a summon currently adjacent to master onto another adjacent tile →
    /// succeeds (anti-crowd only rejects not-adjacent → adjacent, not adjacent → adjacent).
    #[test]
    fn p4_c4_summon_adjacent_to_adjacent_succeeds() {
        let mut world = beat_driven_world();
        // Master at (100,100,7); summon at (101,100,7) (manhattan 1 — adjacent);
        // push to (101,99,7) (manhattan 1 from master — still adjacent).
        let master_pos = Position::new(100, 100, 7);
        let from = Position::new(101, 100, 7);
        let to = Position::new(101, 99, 7);
        ensure_walkable_tile(&mut world.map, master_pos, 1);
        ensure_walkable_tile(&mut world.map, from, 1);
        ensure_walkable_tile(&mut world.map, to, 1);
        // Actor at (102,100,7) — adjacent to `from`.
        let actor_pos = Position::new(102, 100, 7);
        ensure_walkable_tile(&mut world.map, actor_pos, 1);
        let master = insert_player(&mut world, test_player("Master", master_pos));
        let actor = insert_player(&mut world, test_player("Actor", actor_pos));
        let mover = insert_monster_with_config(&mut world, "Summon", from, 200, MonsterAiConfig::default());
        if let Some(k) = world.creatures.get_mut(mover) {
            k.base_mut().master = Some(master);
        }

        let rv = world.player_push_creature(actor, mover, from, to);
        // Anti-crowd does NOT fire (currently adjacent → dest adjacent is allowed).
        assert_eq!(rv, Ok(()));
    }

    /// P4/C2: push a player into a PZ during `EarliestProtectionZoneRound` →
    /// `PlayerIsPzLocked` (772 `ENTERPROTECTIONZONE`), NOT `NotEnoughRoom`.
    /// `crplayer.cc:366-369`: throws `ENTERPROTECTIONZONE`.
    #[test]
    fn p4_c2_push_player_into_pz_while_locked_rejected() {
        let mut world = beat_driven_world();
        register_ground_bank(&mut world);
        world.round_nr = 100;
        // Moving player at (100,100,7) — NOT in PZ; push to (101,100,7) — IS in PZ.
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, from, 1);
        ensure_walkable_tile(&mut world.map, to, 1);
        set_pz_tile(&mut world, to);
        // Actor adjacent to the moving player.
        let actor_pos = Position::new(100, 99, 7);
        ensure_walkable_tile(&mut world.map, actor_pos, 1);
        let actor = insert_player(&mut world, test_player("Actor", actor_pos));
        let mover = insert_player_on_tile(&mut world, test_player("Mover", from), from);
        // Set EarliestProtectionZoneRound > round_nr.
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(mover) {
            p.earliest_protection_zone_round = 160;
        }

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::PlayerIsPzLocked)); // ENTERPROTECTIONZONE
    }

    /// P4/C2: push a player into an uninvited house → `PlayerIsNotInvited`
    /// (772 `NOTINVITED`), NOT `NotEnoughRoom`.
    /// `crplayer.cc:372-377`: throws `NOTINVITED`.
    #[test]
    fn p4_c2_push_player_into_uninvited_house_rejected() {
        let mut world = beat_driven_world();
        register_ground_bank(&mut world);
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, from, 1);
        let actor_pos = Position::new(100, 99, 7);
        ensure_walkable_tile(&mut world.map, actor_pos, 1);
        let actor = insert_player(&mut world, test_player("Actor", actor_pos));
        let mover = insert_player_on_tile(&mut world, test_player("Mover", from), from);
        // Replace dest with a house tile (house_id=1).
        insert_house_tile(&mut world, to, 1);
        // Register house 1 with no owner/guests → player is not invited.
        world.houses.ensure_houses([1]);

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::PlayerIsNotInvited)); // NOTINVITED
    }

    /// P4/C3: push an ATTACKING monster with `KickCreatures` onto a player-blocker tile
    /// → `YouAreExhausted` (772 `EXHAUSTED`) with `Target = 0` side effect.
    /// `crnonpl.cc:2236-2238`: `this->Target = 0; throw EXHAUSTED`.
    #[test]
    fn p4_c3_push_monster_onto_player_blocker_exhausted() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, from, 1);
        ensure_walkable_tile(&mut world.map, to, 1);
        let actor_pos = Position::new(100, 99, 7);
        ensure_walkable_tile(&mut world.map, actor_pos, 1);
        let actor = insert_player(&mut world, test_player("Actor", actor_pos));
        let mover = insert_monster_with_config(
            &mut world,
            "Kicker",
            from,
            200,
            MonsterAiConfig {
                can_push_creatures: true,
                ..MonsterAiConfig::default()
            },
        );
        // Monster is ATTACKING with a target (a different player, not the blocker).
        let target_pos = Position::new(100, 102, 7);
        ensure_walkable_tile(&mut world.map, target_pos, 1);
        let target = insert_player(&mut world, test_player("Target", target_pos));
        // Blocker player on the dest tile — must be registered on the map for the kick loop.
        let blocker = insert_player_on_tile(&mut world, test_player("Blocker", to), to);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
        }

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Err(ReturnValue::YouAreExhausted)); // EXHAUSTED
        // C3: Target = 0 side effect applied at the throw site.
        if let Some(CreatureKind::Monster(m)) = world.creatures.get(mover) {
            assert!(
                m.base.attack_target.is_none() && m.base.follow_target.is_none(),
                "Target must be cleared after EXHAUSTED (crnonpl.cc:2237)"
            );
        }
    }

    /// P4: push an NPC outside its radius → `NotEnoughRoom` (772 `NOROOM`).
    /// `crnonpl.cc:1672-1679`: `abs(x - startx) <= Radius` → false.
    #[test]
    fn p4_push_npc_outside_radius_rejected() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, from, 1);
        ensure_walkable_tile(&mut world.map, to, 1);
        let actor_pos = Position::new(100, 99, 7);
        ensure_walkable_tile(&mut world.map, actor_pos, 1);
        let actor = insert_player(&mut world, test_player("Actor", actor_pos));
        // Build a minimal NPC at `from` with radius=0 (only home tile passes).
        let base = CreatureBase {
            name: "Npc".into(),
            position: from,
            direction: Direction::North,
            health: 100,
            max_health: 100,
            outfit: Outfit::default(),
            speed: 220,
            base_speed: 220,
            var_speed: 0,
            skull: SkullType::None,
            drunkenness: 0,
            active_conditions: Vec::new(),
            walk_queue: Default::default(),
            walk_destinations: Default::default(),
            last_step: None,
            last_step_cost: 1,
            last_step_ground_speed: 150,
            next_wakeup: None,
            last_step_server_ms: None,
            earliest_walk_server_ms: 0,
            earliest_spell_server_ms: 0,
            earliest_multiuse_server_ms: 0,
            cancel_next_walk: false,
            force_update_follow_path: false,
            walk_update_ticks: 0,
            is_updating_path: false,
            has_follow_path: false,
            movement_blocked: false,
            stairhop_blocked_until: None,
            follow_target: None,
            attack_target: None,
            master: None,
            damage_map: Default::default(),
            last_hit_by: None,
            poison_damage_origin: None,
            fire_damage_origin: None,
            energy_damage_origin: None,
            earliest_attack_ms: 0,
            latest_attack_round: 0,
            earliest_defend_ms: 0,
            last_defend_ms: 0,
            learning_points: 0,
            todo: Default::default(),
            chase_mode: Default::default(),
            last_auto_walk_armed_ms: u64::MAX,
        };
        let mut npc = Npc::placeholder(base);
        // radius=0 → only the home tile itself passes.
        npc.runtime.radius = 0;
        npc.runtime.home_position = from;
        let npc_id = world.creatures.insert(CreatureKind::Npc(npc));
        crate::login_out::assign_creature_wire_id(&mut world, npc_id);
        world.map.register_creature_at(from, npc_id);

        let rv = world.player_push_creature(actor, npc_id, from, to);
        assert_eq!(rv, Err(ReturnValue::NotEnoughRoom)); // NOROOM
    }

    /// P4: push a monster within its home range → succeeds (positive case for home range).
    #[test]
    fn p4_push_monster_within_home_range_succeeds() {
        let mut world = beat_driven_world();
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        let (actor, mover) = setup_push_arena_pd(&mut world, from, to);
        // Set spawn at (100,100,7) and home_radius=1. Dest (101,100,7) is distance 1 — within range.
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.spawn_position = from;
            m.home_radius = 1;
        }

        let rv = world.player_push_creature(actor, mover, from, to);
        assert_eq!(rv, Ok(()));
    }
}
