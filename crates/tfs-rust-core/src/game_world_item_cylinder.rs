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

    /// 772 `GetTopObject(x, y, z, true)` — first non-BANK/CLIP/BOTTOM/TOP/creature object.
    /// Used for the top moveable check in the Move path.
    // C++ ref: `info.cc:366-388` `GetTopObject`
    pub(crate) fn get_top_object_for_move(&self, pos: Position) -> Option<ItemId> {
        let tile = self.map.get_tile(pos)?;
        let body = tile.body();

        // Ground is always skipped (BANK).
        // Cip order: ground → top items → creatures → down items.
        for &iid in &body.top_items {
            if let Some(i) = self.items.get(iid) {
                if let Some(t) = self.items_db.items.get(&i.item_type) {
                    // TOP / CLIP / BOTTOM render categories are always-on-top.
                    // `CheckTopMoveObject` only accepts the first *moveable* object (T4).
                    if !t.always_on_top() && t.moveable() {
                        return Some(iid);
                    }
                }
            }
        }
        // Creatures are skipped when Move=true.
        for &iid in &body.down_items {
            if let Some(i) = self.items.get(iid) {
                if let Some(t) = self.items_db.items.get(&i.item_type) {
                    // BOTTOM priority items (splashes / magic fields) sit below creatures.
                    // Break at `PRIORITY_LOW` (`is_cip_priority_bottom`) and skip `!moveable()`.
                    if !t.is_cip_priority_bottom() && !t.always_on_top() && t.moveable() {
                        return Some(iid);
                    }
                }
            }
        }
        None
    }

    /// Resolve a client-encoded position to a `Thing` (item or creature on a tile).
    // C++ ref: src/game.cpp:213 Game::internalGetThing (STACKPOS_MOVE path)
    pub fn internal_get_thing_move(
        &self,
        cid: CreatureId,
        pos: Position,
        _stack_pos: u8,
        sprite_id: u16,
    ) -> Option<Thing> {
        if pos.x != 0xFFFF {
            self.map.get_tile(pos)?;
            // 772 `CMoveObject` sets `RNum = 1` and `GetObject` walks the tile list for the
            // client `TypeID` (`info.cc:398-432`), allowing a buried item to be moved by sprite.
            // The top moveable candidate is `GetTopObject(true)` (`info.cc:366-388`).
            if let Some(top_item_id) = self.get_top_object_for_move(pos) {
                if self.validate_item_sprite(top_item_id, sprite_id) {
                    return Some(Thing::Item(top_item_id));
                }
            }
            if let Some(item_id) = self.find_tile_item_by_client_sprite(pos, sprite_id) {
                return Some(Thing::Item(item_id));
            }
            // 772 `GetTopObject(true)` for `Move` skips creatures unless they are
            // creature-containers; without creature push the fallback must not match
            // a creature that doesn't correspond to the client sprite.
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
        // C++ ref: `operate.cc:451` `IsMapBlocked` — BANK/UNPASS/UNLAY/HANG hooks.
        if self.is_map_blocked(pos, item_id) {
            return ReturnValue::NotPossible;
        }
        let Some(item) = self.items.get(item_id) else {
            return ReturnValue::NotPossible;
        };
        let it = self.items_db.items.get(&item.item_type);
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

    /// 772 `IsMapBlocked` — destination tile validity for thrown/placed items (`operate.cc:451`).
    /// Returns `true` when the tile cannot accept `item_id`.
    pub(crate) fn is_map_blocked(&self, pos: Position, item_id: ItemId) -> bool {
        let Some(tile) = self.map.get_tile(pos) else {
            return true;
        };
        let body = tile.body();
        let Some(item) = self.items.get(item_id) else {
            return true;
        };
        let Some(it) = self.items_db.items.get(&item.item_type) else {
            return true;
        };

        let is_unpass = it.block_solid();
        let is_hang = it.is_hangable();
        let has_bank = body.ground.is_some();
        let has_hook = (body.flags
            & (crate::tile::flags::HOOKEAST | crate::tile::flags::HOOKSOUTH))
            != 0;

        let mut has_unpass = false;
        let mut has_unlay = false;
        let mut has_hang = false;

        // Ground contributes to the coordinate flags as well.
        if let Some(ground_id) = body.ground {
            if let Some(t) = self.items_db.items.get(&ground_id) {
                if t.block_solid() {
                    has_unpass = true;
                }
                if t.xml_attributes.get("unlay").map(|v| v == "true").unwrap_or(false) {
                    has_unlay = true;
                }
                if t.is_hangable() {
                    has_hang = true;
                }
            }
        }

        for &iid in body.top_items.iter().chain(body.down_items.iter()) {
            if let Some(i) = self.items.get(iid) {
                if let Some(t) = self.items_db.items.get(&i.item_type) {
                    if t.block_solid() {
                        has_unpass = true;
                    }
                    if t.xml_attributes.get("unlay").map(|v| v == "true").unwrap_or(false) {
                        has_unlay = true;
                    }
                    if t.is_hangable() {
                        has_hang = true;
                    }
                }
            }
        }

        if has_bank && !has_unpass {
            return false;
        }
        if !is_unpass {
            if has_bank && !has_unlay {
                return false;
            }
            if is_hang && has_hook && !has_hang {
                return false;
            }
        }
        true
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

        // Try stackable merge with the top object only (T22) and only if the full
        // count fits (T17). 772 `Move` calls `GetTopObject(Con, true)` before `Merge`
        // and `Merge` throws `TOOMANYPARTS` when `DestCount + Count > 100` for tiles.
        if is_stackable {
            if let Some(target_id) = self.get_top_object_for_move(pos) {
                if let Some(existing) = self.items.get(target_id) {
                    if existing.item_type == item_type
                        && existing.count < 100
                        && (existing.count as u32 + item_count as u32) <= 100
                    {
                        if let Some(target) = self.items.get_mut(target_id) {
                            target.count += item_count;
                        }
                        // Get stack pos for update packet
                        let (tvp_stack, cip_stack) = self.item_stack_pos_pair(pos, target_id);
                        self.broadcast_tile_item_update(pos, target_id, tvp_stack, cip_stack);

                        // Fully merged — remove the source item from SlotMap
                        self.cancel_item_decay(item_id);
                        self.items.remove(item_id);
                        self.start_decay(target_id);
                        return Ok(target_id);
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
        let (tvp_stack, cip_stack) = self.item_stack_pos_pair(pos, item_id);

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
            let (tvp_stack, cip_stack) = self.item_stack_pos_pair(pos, item_id);
            self.broadcast_tile_item_update(pos, item_id, tvp_stack, cip_stack);
        } else {
            // Full removal
            let (tvp_stack, cip_stack) = self.item_stack_pos_pair(pos, item_id);
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
