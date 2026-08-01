//! F8 S4 — `TDTurn` executor: rotate a rotatable *item* (`CTurnObject`).
//!
//! C++ reference (772 mechanics = `tibia-game-master/src/`):
//! - `cract.cc:771-777` `TCreature::Turn(Object)` — re-validate `Obj.exists()` → throw
//!   `DESTROYED`; call `::Turn(this->ID, Obj)`.
//! - `operate.cc:2562-2583` `Turn(CreatureID, Object)` — `ObjectAccessible(..., 1)` →
//!   `NOTACCESSIBLE`; `ObjType.getFlag(ROTATE)` → `NOTTURNABLE`; `RotateTarget =
//!   ObjType.getAttribute(ROTATETARGET)`; `Change(Obj, RotateTarget, 0)`.
//! - `operate.cc:1534-1632` `Change(Object, NewType, Value)` — in-place type transform
//!   (`ChangeObject(Obj, NewType)`) + broadcast.
//!
//! This is **not** `CRotate` (player facing direction, `receiving.cc:213`, already
//! immediate via `GamePacket::Turn` → `player_turn_request`). See F8 §0.1 F2.

use crate::creature_todo::ActionObjectRef;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::item_look::look_distance_tfs;
use crate::return_value::ReturnValue;

impl GameWorld {
    /// F8 S4 — `TDTurn` execute arm. Rotates a rotatable item in-place (e.g. wall torch,
    /// rope) by setting `item.item_type = rotate_to` and broadcasting `0x6B`
    /// (`sendUpdateTileItem`). Re-validates the object at execute time (mirrors C++
    /// `Obj.exists()` in `cract.cc:772` + `ObjectAccessible` in `operate.cc:2568`).
    ///
    /// Returns `Err(NotPossible)` on any failure (C++ `DESTROYED`/`NOTACCESSIBLE`/
    /// `NOTTURNABLE` all map to `ReturnValue::NotPossible` per the `walk/mod.rs:1506`
    /// convention). The caller (`execute_creature_todo_action`) applies the C++ `RESULT`
    /// catch (`cract.cc:870-889`) on `Err`.
    pub(crate) fn player_rotate_item(
        &mut self,
        cid: CreatureId,
        obj: ActionObjectRef,
    ) -> Result<(), ReturnValue> {
        let is_map_tile = obj.pos.x != 0xFFFF;

        // Re-validate: resolve the item at the wire location + sprite match.
        // C++ `Obj.exists()` (`cract.cc:772`) + `ObjectAccessible` (`operate.cc:2568`).
        let item_id = self.resolve_use_object(cid, obj.pos, obj.stack_pos, obj.sprite_id);
        let Some(item_id) = item_id else {
            return Err(ReturnValue::NotPossible);
        };
        if !self.validate_item_sprite(item_id, obj.sprite_id) {
            return Err(ReturnValue::NotPossible);
        }

        // Range check for map tiles — C++ `ObjectAccessible(CreatureID, Obj, 1)`
        // (`operate.cc:2568`). Inventory/container items are always accessible.
        if is_map_tile {
            let Some(player_pos) = self.creatures.get(cid).map(|k| k.position()) else {
                return Err(ReturnValue::NotPossible);
            };
            if look_distance_tfs(player_pos, obj.pos) > 1 {
                return Err(ReturnValue::NotPossible);
            }
        }

        // Check `rotatable()` flag — C++ `ObjType.getFlag(ROTATE)` (`operate.cc:2573`).
        // `NOTTURNABLE` (772 RESULT 57, `enums.hh:449`) → `NotPossible` (no exact Rust
        // variant; matches the existing convention).
        let item_type = self.items.get(item_id).map(|i| i.item_type).unwrap_or(0);
        let Some(it) = self.items_db.items.get(&item_type) else {
            return Err(ReturnValue::NotPossible);
        };
        if !it.rotatable() {
            return Err(ReturnValue::NotPossible);
        }

        // `RotateTarget = ObjType.getAttribute(ROTATETARGET)` (`operate.cc:2577`).
        // `rotate_to` is the server id of the rotated form (XML `rotateto` attribute,
        // `items.rs:725-729`). 0 = no rotation target → C++ logs an error and the
        // `Change` would destroy the item; we treat it as `NOTPOSSIBLE`.
        let rotate_to = it.rotate_to;
        if rotate_to == 0 {
            tracing::warn!(
                item_type,
                "player_rotate_item: rotatable item has rotate_to=0 (NOTTURNABLE)"
            );
            return Err(ReturnValue::NotPossible);
        }

        // `Change(Obj, RotateTarget, 0)` — in-place type transform
        // (`operate.cc:1605` `ChangeObject(Obj, NewType)`). The item stays at the same
        // tile/stack position; only `item_type` changes. For simple rotatable items
        // (wall torches, ropes) the full `Change` container/weight logic
        // (`operate.cc:1541-1629`) doesn't apply — they're non-container, non-cumulative
        // map items, so a direct `item_type` assignment is the outcome-equivalent path.
        if let Some(item) = self.items.get_mut(item_id) {
            item.item_type = rotate_to;
        }

        // Broadcast `sendUpdateTileItem` (0x6B) to spectators — `operate.cc:1605`
        // `ChangeObject` triggers the tile update. Only map tiles have spectators;
        // inventory/container item updates are handled by their own refresh paths
        // (not yet needed for rotatable items, which are overwhelmingly map objects).
        if is_map_tile {
            self.broadcast_tile_item_update(obj.pos, item_id, obj.stack_pos, obj.stack_pos);
        }
        Ok(())
    }

    /// F8 S4 — resolve the `ItemId` for a `Turn` action's `ActionObjectRef` (test helper).
    #[cfg(test)]
    pub(crate) fn resolve_rotate_item_id(
        &self,
        cid: CreatureId,
        obj: ActionObjectRef,
    ) -> Option<crate::ids::ItemId> {
        let item_id = self.resolve_use_object(cid, obj.pos, obj.stack_pos, obj.sprite_id);
        item_id.filter(|&id| self.validate_item_sprite(id, obj.sprite_id))
    }
}
