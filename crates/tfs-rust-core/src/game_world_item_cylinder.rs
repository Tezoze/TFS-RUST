//! Item cylinder resolution and tile add/remove.
//!
//! - `Game::internalGetCylinder`, `internalGetThing`, `internalAddItem`, `internalRemoveItem` — `game.cpp`.
//! - `Tile::queryAdd` — `tile.cpp`.

use tfs_rust_common::Position;

use crate::cylinder::{Cylinder, CylinderFlags};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use crate::thing::{LookTarget, Thing};

impl GameWorld {
    // === Item Movement (B.4) ===
    // C++ reference: `src/game.cpp` Game::internalMoveItem (~1078), internalAddItem (~1287),
    //                internalRemoveItem (~1376), internalGetCylinder (~197), internalGetThing (~213).

    /// Resolve a client-encoded position to a `Cylinder`.
    // C++ ref: src/game.cpp:197 Game::internalGetCylinder
    pub fn internal_get_cylinder(&self, cid: CreatureId, pos: Position) -> Option<Cylinder> {
        if pos.x != 0xFFFF {
            // Map tile
            if self.map.get_tile(pos).is_some() {
                return Some(Cylinder::Tile { pos });
            }
            return None;
        }
        // Container (y & 0x40) — `game.cpp` `internalGetCylinder` container branch.
        if pos.y & 0x40 != 0 {
            let client_cid = (pos.y & 0x0F) as u8;
            let slot_index = pos.z as i32;
            let container_id = self
                .container_registry
                .get_container_by_cid(cid, client_cid)?;
            return Some(Cylinder::Container {
                item_id: container_id,
                index: slot_index,
            });
        }
        // Inventory slot
        Some(Cylinder::Inventory {
            player_id: cid,
            slot: pos.y as u8,
        })
    }

    /// Resolve a client-encoded position to a `Thing` (item or creature on a tile).
    // C++ ref: src/game.cpp:213 Game::internalGetThing (STACKPOS_MOVE path)
    pub fn internal_get_thing_move(
        &self,
        cid: CreatureId,
        pos: Position,
        _stack_pos: u8,
    ) -> Option<Thing> {
        if pos.x != 0xFFFF {
            let tile = self.map.get_tile(pos)?;
            // STACKPOS_MOVE: prefer top moveable down item, else top visible creature
            if let Some(top_item_id) = tile.get_top_down_item() {
                if let Some(item) = self.items.get(top_item_id) {
                    let it = self.items_db.items.get(&item.item_type);
                    if it.map(|t| t.moveable()).unwrap_or(false) {
                        return Some(Thing::Item(top_item_id));
                    }
                }
            }
            // Fall through to creature
            let body = tile.body();
            if let Some(&creature_id) = body.creatures.last() {
                return Some(Thing::Creature(creature_id));
            }
            return None;
        }
        // Container slot — `internalGetThing` container UI position.
        if pos.y & 0x40 != 0 {
            let client_cid = (pos.y & 0x0F) as u8;
            let slot = pos.z as usize;
            let container_id = self
                .container_registry
                .get_container_by_cid(cid, client_cid)?;
            let c = self.container_registry.get(container_id)?;
            let iid = c.get_item(slot)?;
            return Some(Thing::Item(iid));
        }
        // Inventory — `pos.y` is `slots_t` (`game.cpp` ~320–326).
        let slot = pos.y as u8;
        if let Some(iid) = self.get_player_inventory_item(cid, slot) {
            return Some(Thing::Item(iid));
        }
        None
    }

    /// C++ `Game::internalGetThing` with `STACKPOS_LOOK` — `game.cpp` ~223–224.
    /// Client `stack_pos` is ignored for map tiles (uses `getTopVisibleThing`).
    pub fn internal_get_thing_look(
        &self,
        cid: CreatureId,
        pos: Position,
        _stack_pos: u8,
    ) -> Option<LookTarget> {
        if pos.x != 0xFFFF {
            let tile = self.map.get_tile(pos)?;
            return self.top_visible_look_target_on_tile(tile, cid);
        }
        if pos.y & 0x40 != 0 {
            let client_cid = (pos.y & 0x0F) as u8;
            let slot = pos.z as usize;
            let container_id = self
                .container_registry
                .get_container_by_cid(cid, client_cid)?;
            let c = self.container_registry.get(container_id)?;
            let iid = c.get_item(slot)?;
            return Some(LookTarget::Item(iid));
        }
        let slot = pos.y as u8;
        let iid = self.get_player_inventory_item(cid, slot)?;
        Some(LookTarget::Item(iid))
    }

    /// C++ `Tile::getTopVisibleThing` — `tile.cpp` ~322–347.
    pub(crate) fn top_visible_look_target_on_tile(
        &self,
        tile: &crate::tile::Tile,
        viewer: CreatureId,
    ) -> Option<LookTarget> {
        tile.top_visible_look_target(
            |cid| self.can_see_creature(viewer, cid),
            |iid| self.item_is_opaque_for_look(iid),
        )
    }

    /// First non-`lookThrough` item in the look stack walk.
    fn item_is_opaque_for_look(&self, item_id: ItemId) -> bool {
        let Some(item) = self.items.get(item_id) else {
            return true;
        };
        !self
            .items_db
            .items
            .get(&item.item_type)
            .map(|t| t.look_through())
            .unwrap_or(false)
    }

    /// Query if a tile can accept an item.
    // C++ ref: src/tile.cpp:629-702 Tile::queryAdd for items
    pub(crate) fn query_add_item_to_tile(
        &self,
        pos: Position,
        item_id: ItemId,
        flags: CylinderFlags,
    ) -> ReturnValue {
        let Some(tile) = self.map.get_tile(pos) else {
            return ReturnValue::NotPossible;
        };
        // Max items check
        if tile.total_item_count() >= 0xFFFF {
            return ReturnValue::NotPossible;
        }
        if flags.contains(CylinderFlags::NO_LIMIT) {
            return ReturnValue::NoError;
        }
        let Some(item) = self.items.get(item_id) else {
            return ReturnValue::NotPossible;
        };
        let it = self.items_db.items.get(&item.item_type);
        let is_hangable = it.map(|t| t.is_hangable()).unwrap_or(false);
        // Non-hangable items need ground
        if tile.body().ground.is_none() && !is_hangable {
            return ReturnValue::NotPossible;
        }
        // Blocking item can't be placed where non-ghost creatures are
        let is_blocking = it.map(|t| t.block_solid()).unwrap_or(false);
        if is_blocking && !flags.contains(CylinderFlags::IGNORE_BLOCK_CREATURE) {
            let body = tile.body();
            if !body.creatures.is_empty() {
                return ReturnValue::NotEnoughRoom;
            }
        }
        ReturnValue::NoError
    }

    /// Validate that an item exists in the specified cylinder.
    pub(crate) fn validate_item_in_cylinder(
        &self,
        cylinder: &Cylinder,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        match cylinder {
            Cylinder::Tile { pos } => {
                let tile = self.map.get_tile(*pos).ok_or(ReturnValue::NotPossible)?;
                if !tile.has_item(item_id) {
                    return Err(ReturnValue::NotPossible);
                }
                Ok(())
            }
            Cylinder::Container {
                item_id: container_id,
                ..
            } => {
                let c = self
                    .container_registry
                    .get(*container_id)
                    .ok_or(ReturnValue::NotPossible)?;
                if !c.contains(item_id) {
                    return Err(ReturnValue::NotPossible);
                }
                Ok(())
            }
            Cylinder::Inventory { player_id, slot } => {
                self.validate_inventory_item(*player_id, *slot, item_id)
            }
        }
    }

    /// Add an item to a tile, handling stackable merge.
    /// Returns the ItemId that ended up on the tile (may differ if merged into existing stack).
    // C++ ref: src/game.cpp:1287 Game::internalAddItem (tile path)
    pub fn internal_add_item_to_tile(
        &mut self,
        pos: Position,
        item_id: ItemId,
        _flags: CylinderFlags,
    ) -> Result<ItemId, ReturnValue> {
        let is_stackable;
        let item_type;
        let item_count;
        {
            let item = self.items.get(item_id).ok_or(ReturnValue::NotPossible)?;
            item_type = item.item_type;
            item_count = item.count;
            is_stackable = self
                .items_db
                .items
                .get(&item.item_type)
                .map(|t| t.stackable())
                .unwrap_or(false);
        }

        // Try stackable merge
        if is_stackable {
            let tile = self.map.get_tile(pos).ok_or(ReturnValue::NotPossible)?;
            // Look for an existing stack of the same type
            let mut merge_target: Option<ItemId> = None;
            for &did in &tile.body().down_items {
                if let Some(existing) = self.items.get(did) {
                    if existing.item_type == item_type && existing.count < 100 {
                        merge_target = Some(did);
                        break;
                    }
                }
            }
            if let Some(target_id) = merge_target {
                let target_count = self.items.get(target_id).map(|i| i.count).unwrap_or(0);
                let can_add = (100u16).saturating_sub(target_count).min(item_count);
                if can_add > 0 {
                    if let Some(target) = self.items.get_mut(target_id) {
                        target.count += can_add;
                    }
                    // Get stack pos for update packet
                    let (tvp_stack, cip_stack) = self
                        .map
                        .get_tile(pos)
                        .map(|t| {
                            (
                                t.get_item_stack_pos_ordered(target_id, false).unwrap_or(0),
                                t.get_item_stack_pos_ordered(target_id, true).unwrap_or(0),
                            )
                        })
                        .unwrap_or((0, 0));
                    self.broadcast_tile_item_update(pos, target_id, tvp_stack, cip_stack);

                    let remainder = item_count.saturating_sub(can_add);
                    if remainder == 0 {
                        // Fully merged — remove the source item from SlotMap
                        self.cancel_item_decay(item_id);
                        self.items.remove(item_id);
                        self.start_decay(target_id);
                        return Ok(target_id);
                    }
                    // Partial merge — update source item count and add remainder to tile
                    if let Some(item) = self.items.get_mut(item_id) {
                        item.count = remainder;
                    }
                }
            }
        }

        // Add item to tile. Always-on-top items (splashes/pools, ladders, signs, borders) live in
        // the top-item group, which the client renders BEFORE creatures — C++ `Tile::addThing`
        // (`tile.cpp:1455`). Routing them into down_items (after creatures) shifts every creature's
        // client stackpos by one, desyncing subsequent creature moves (e.g. a blood splash under a
        // player froze their movement).
        //
        // Top items are inserted sorted by `alwaysOnTopOrder` (ascending), matching C++
        // `Tile::addThing` (`tile.cpp:898-906`): the new item is inserted before the first existing
        // top item with `alwaysOnTopOrder >= new`. The 772 client's `0x6A` (add tile item) omits
        // stackpos, so the client places the item by `.dat` `alwaysOnTopOrder` — if our server
        // vector order doesn't match, `0x6C` remove hits the wrong client stackpos (e.g. a splash
        // and ladder both order 2: server appends splash after ladder, client inserts splash
        // before ladder → remove at server stackpos deletes the ladder on the client).
        let item_type_info = self.items_db.items.get(&item_type);
        let always_on_top = item_type_info.map(|t| t.always_on_top()).unwrap_or(false);
        let new_order = item_type_info.map(|t| t.always_on_top_order).unwrap_or(0);

        {
            let tile = self.map.get_tile_mut(pos).ok_or(ReturnValue::NotPossible)?;
            if always_on_top {
                // Compute the sorted insertion index: first position where existing order >= new.
                // C++ `tile.cpp:901`: `if (itemType.alwaysOnTopOrder <= Item::items[(*it)->getID()].alwaysOnTopOrder)`.
                let insert_at = tile
                    .body()
                    .top_items
                    .iter()
                    .position(|&existing_id| {
                        let existing_order = self
                            .items
                            .get(existing_id)
                            .and_then(|it| self.items_db.items.get(&it.item_type))
                            .map(|t| t.always_on_top_order)
                            .unwrap_or(0);
                        new_order <= existing_order
                    })
                    .unwrap_or_else(|| tile.body().top_items.len());
                tile.add_top_item_at(item_id, insert_at);
            } else {
                tile.add_item(item_id);
            }
        }

        if let Some(item) = self.items.get_mut(item_id) {
            item.parent = Some(crate::cylinder::Cylinder::Tile { pos });
        }

        // Stackpos of the item as it now sits on the tile — TVP vs Cip map-container order.
        let (tvp_stack, cip_stack) = self
            .map
            .get_tile(pos)
            .map(|t| {
                (
                    t.get_item_stack_pos_ordered(item_id, false).unwrap_or(0),
                    t.get_item_stack_pos_ordered(item_id, true).unwrap_or(0),
                )
            })
            .unwrap_or((0, 0));

        // Broadcast add
        self.broadcast_tile_item_add(pos, item_id, tvp_stack, cip_stack);

        self.start_decay(item_id);
        Ok(item_id)
    }

    /// Remove an item (or count of a stackable) from a tile.
    // C++ ref: src/game.cpp:1376 Game::internalRemoveItem
    pub fn internal_remove_item_from_tile(
        &mut self,
        pos: Position,
        item_id: ItemId,
        count: u16,
    ) -> Result<(), ReturnValue> {
        let item = self.items.get(item_id).ok_or(ReturnValue::NotPossible)?;
        let is_stackable = self
            .items_db
            .items
            .get(&item.item_type)
            .map(|t| t.stackable())
            .unwrap_or(false);
        let item_count = item.count;

        if is_stackable && count < item_count {
            // Partial removal — just reduce count and send update
            if let Some(item) = self.items.get_mut(item_id) {
                item.count -= count;
            }
            let (tvp_stack, cip_stack) = self
                .map
                .get_tile(pos)
                .map(|t| {
                    (
                        t.get_item_stack_pos_ordered(item_id, false).unwrap_or(0),
                        t.get_item_stack_pos_ordered(item_id, true).unwrap_or(0),
                    )
                })
                .unwrap_or((0, 0));
            self.broadcast_tile_item_update(pos, item_id, tvp_stack, cip_stack);
        } else {
            // Full removal
            let (tvp_stack, cip_stack) = self
                .map
                .get_tile(pos)
                .map(|t| {
                    (
                        t.get_item_stack_pos_ordered(item_id, false).unwrap_or(0),
                        t.get_item_stack_pos_ordered(item_id, true).unwrap_or(0),
                    )
                })
                .unwrap_or((0, 0));
            let tile = self.map.get_tile_mut(pos).ok_or(ReturnValue::NotPossible)?;
            if tile.remove_item_by_id(item_id).is_none() {
                return Err(ReturnValue::NotPossible);
            }
            self.broadcast_tile_item_remove(pos, tvp_stack, cip_stack);
            // Remove from SlotMap
            self.cancel_item_decay(item_id);
            if let Some(item) = self.items.get_mut(item_id) {
                item.parent = None;
            }
            self.items.remove(item_id);
        }
        Ok(())
    }
}
