//! Item cylinder resolution and tile add/remove.
//!
//! - `Game::internalGetCylinder`, `internalGetThing`, `internalAddItem`, `internalRemoveItem` — `game.cpp`.
//! - `Tile::queryAdd` — `tile.cpp`.

use tfs_rust_common::Position;

use crate::creature::CreatureKind;
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
    /// 772 `GetTopObject(x, y, z, true)` — first non-BANK/CLIP/BOTTOM/TOP/creature object.
    /// `ignore` is an optional `ItemId` to skip (used by the 772 `Move` `Ignore` parameter).
    pub(crate) fn get_top_object_for_move(
        &self,
        pos: Position,
        ignore: Option<ItemId>,
    ) -> Option<ItemId> {
        let tile = self.map.get_tile(pos)?;
        let body = tile.body();

        // Ground is always skipped (BANK).
        // Cip order: ground → top items → creatures → down items.
        for &iid in &body.top_items {
            if ignore == Some(iid) {
                continue;
            }
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
            if ignore == Some(iid) {
                continue;
            }
            if let Some(i) = self.items.get(iid) {
                if let Some(t) = self.items_db.items.get(&i.item_type) {
                    // Cip BOTTOM (pools) sit below creatures; LOW (incl. magic fields) after.
                    // Skip BOTTOM / always-on-top / immovable when picking the top move object.
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
            if let Some(top_item_id) = self.get_top_object_for_move(pos, None) {
                if self.validate_item_sprite(top_item_id, sprite_id) {
                    return Some(Thing::Item(top_item_id));
                }
            }
            if let Some(item_id) = self.find_tile_item_by_client_sprite(pos, sprite_id) {
                return Some(Thing::Item(item_id));
            }
            // TVP 772 `internalGetThing` STACKPOS_MOVE (`game.cpp:233-240`): when no
            // moveable item matches, returns `getBottomVisibleCreature(player)` — the
            // first visible creature on the tile, **without sprite matching**. The
            // `spriteId` parameter is unused for the creature path. The client sends
            // a sprite id that may differ from the stored outfit `lookType` (e.g. NPC
            // Sam: stored lookType=131, client sends 99), so matching by sprite would
            // wrongly reject valid pushes.
            return self.find_tile_bottom_visible_creature(pos, cid);
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

    /// 772 `Tile::getBottomVisibleCreature` (`tile.cpp:295-314`) — first visible creature
    /// on `pos`, iterating top-first (reverse), skipping invisible + ghost-mode players.
    /// No sprite matching — matches TVP 772 `internalGetThing` STACKPOS_MOVE behavior.
    pub(crate) fn find_tile_bottom_visible_creature(
        &self,
        pos: Position,
        _viewer: CreatureId,
    ) -> Option<Thing> {
        let tile = self.map.get_tile(pos)?;
        let body = tile.body();
        for &creature_id in body.creatures.iter().rev() {
            let Some(c) = self.creatures.get(creature_id) else {
                continue;
            };
            // Skip invisible creatures and ghost-mode players (772 `tile.cpp:306-311`).
            if let CreatureKind::Player(p) = c {
                if p.ghost_mode {
                    continue;
                }
            }
            return Some(Thing::Creature(creature_id));
        }
        None
    }

    /// 772 `GetObject` `getDisguise()` match for `CUseObject` (use path still uses sprite
    /// matching). Not used by the move/throw path — see `find_tile_bottom_visible_creature`.
    #[allow(dead_code)]
    pub(crate) fn find_tile_creature_by_client_sprite(
        &self,
        pos: Position,
        sprite_id: u16,
    ) -> Option<Thing> {
        let tile = self.map.get_tile(pos)?;
        let body = tile.body();
        for &creature_id in body.creatures.iter().rev() {
            let Some(c) = self.creatures.get(creature_id) else {
                continue;
            };
            if c.base().outfit.look_type as u16 == sprite_id {
                return Some(Thing::Creature(creature_id));
            }
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
    pub(crate) fn item_is_opaque_for_look(&self, item_id: ItemId) -> bool {
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

    /// 772 `Move` (`operate.cc:1311-1319`) — a failed `Merge` falls through to a separate
    /// stack; only `DESTROYED` (source/dest missing) is left for the caller to hit later.
    /// Returns `Some(target)` only when the merge is legal.
    pub(crate) fn tile_merge_target(
        &self,
        item_id: ItemId,
        pos: Position,
        count: u16,
    ) -> Option<ItemId> {
        let target = self.get_top_object_for_move(pos, None)?;
        self.merge_check(item_id, target, count).ok().map(|_| target)
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
                if t.is_unlay() {
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
                    if t.is_unlay() {
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

    /// 772 `ObjectInRange` — `posz == ObjZ && |dx| <= Range && |dy| <= Range`
    /// (`info.cc:247-249`). Inventory/container sources are always "in range".
    pub(crate) fn object_in_range(
        &self,
        actor: CreatureId,
        pos: Position,
        range: u32,
    ) -> bool {
        if pos.x == 0xFFFF {
            return true;
        }
        let Some(k) = self.creatures.get(actor) else {
            return false;
        };
        let pp = k.position();
        if pp.z != pos.z {
            return false;
        }
        let dx = (pp.x as i32 - pos.x as i32).unsigned_abs();
        let dy = (pp.y as i32 - pos.y as i32).unsigned_abs();
        dx <= range && dy <= range
    }

    /// 772 `INVENTORY_ANY` resolution (`cract.cc:501-547`).
    /// Scans equipment slots first, then nested containers, and returns the first
    /// cylinder that can accept `count` items. `count == 0` is treated as `1`.
    pub(crate) fn resolve_inventory_any(
        &mut self,
        cid: CreatureId,
        item_id: ItemId,
        count: u32,
        flags: CylinderFlags,
    ) -> Result<Cylinder, ReturnValue> {
        let count = count.max(1);
        let Some(item_type) = self.items.get(item_id).map(|i| i.item_type) else {
            return Err(ReturnValue::NotPossible);
        };
        let stackable = self
            .items_db
            .items
            .get(&item_type)
            .map(|it| it.stackable())
            .unwrap_or(false);

        // Pass 1: first fitting equipment slot (1..=10, which naturally prefers non-hand/ammo).
        for slot in crate::inventory::PLAYER_INVENTORY_SLOT_FIRST
            ..=crate::inventory::PLAYER_INVENTORY_SLOT_LAST
        {
            if self.player_query_add(cid, slot, item_id, count, flags) != ReturnValue::NoError {
                continue;
            }
            // For same-type stackable destinations, ensure there is room to merge.
            if let Some(dest_id) = self.get_player_inventory_item(cid, slot) {
                if self.items_stack_mergeable(item_id, dest_id)
                    && self.items.get(dest_id).is_some_and(|d| d.count >= 100)
                {
                    continue;
                }
            }
            return Ok(Cylinder::Inventory {
                player_id: cid,
                slot,
            });
        }

        // Pass 2: nested containers (equipment first, then children).
        let mut containers: Vec<ItemId> = Vec::new();
        for slot in crate::inventory::PLAYER_INVENTORY_SLOT_FIRST
            ..=crate::inventory::PLAYER_INVENTORY_SLOT_LAST
        {
            let Some(iid) = self.get_player_inventory_item(cid, slot) else {
                continue;
            };
            if iid == item_id {
                continue;
            }
            if self
                .items_db
                .is_container(self.items.get(iid).map(|i| i.item_type).unwrap_or(0))
            {
                containers.push(iid);
            }
        }

        let mut i = 0;
        while i < containers.len() {
            let container_id = containers[i];
            i += 1;

            // 772 `CheckContainerDestination` for auto-stack: prefer an existing partial stack.
            if stackable {
                let Some(cont) = self.container_registry.get(container_id) else {
                    continue;
                };
                let cont_items: Vec<ItemId> = cont.items.clone();
                for (idx, &child) in cont_items.iter().enumerate() {
                    if child == item_id {
                        continue;
                    }
                    if self.items_stack_mergeable(item_id, child)
                        && self.items.get(child).is_some_and(|c| c.count < 100)
                        && self.container_query_add(
                            container_id,
                            idx as i32,
                            item_id,
                            count,
                            flags,
                            Some(cid),
                        ) == ReturnValue::NoError
                    {
                        return Ok(Cylinder::Container {
                            item_id: container_id,
                            index: idx as i32,
                        });
                    }
                }
            }

            if self.container_query_add(
                container_id,
                crate::cylinder::INDEX_WHEREEVER,
                item_id,
                count,
                flags,
                Some(cid),
            ) == ReturnValue::NoError
            {
                return Ok(Cylinder::Container {
                    item_id: container_id,
                    index: crate::cylinder::INDEX_WHEREEVER,
                });
            }

            let Some(cont) = self.container_registry.get(container_id) else {
                continue;
            };
            for &child in &cont.items {
                if child == item_id {
                    continue;
                }
                let child_type = self.items.get(child).map(|i| i.item_type).unwrap_or(0);
                if self.items_db.is_container(child_type) {
                    containers.push(child);
                }
            }
        }

        Err(ReturnValue::NotEnoughRoom)
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
        flags: CylinderFlags,
    ) -> Result<ItemId, ReturnValue> {
        let (pos, extra) = crate::walk::query_destination_chain(&self.map, pos);
        let flags = if extra & crate::walk::FLAG_NOLIMIT != 0 {
            flags.union(CylinderFlags::NO_LIMIT)
        } else {
            flags
        };
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

        // Try stackable merge with the top object only (T22). 772 `Move` calls
        // `GetTopObject(Con, true)` before `Merge`; `Merge` throws `TOOMANYPARTS`,
        // `NOMATCH`, or `NOTCUMULABLE` when the merge is impossible.
        // 772 only auto-merges when `OldCon != Con` (T23), signaled by IGNORE_AUTO_STACK.
        if is_stackable
            && !flags.contains(CylinderFlags::IGNORE_AUTO_STACK)
            && !flags.contains(CylinderFlags::NO_MERGE)
        {
            if let Some(target_id) = self.tile_merge_target(item_id, pos, item_count) {
                if let Some(target) = self.items.get_mut(target_id) {
                    target.count = target.count.saturating_add(item_count);
                }
                // Get stack pos for update packet
                let (tvp_stack, cip_stack) = self.item_stack_pos_pair(pos, target_id);
                self.broadcast_tile_item_update(pos, target_id, tvp_stack, cip_stack);

                // Fully merged — remove the source item from SlotMap
                let _ = self.events.on_step_in(None, target_id, item_type, pos, pos);

                self.cancel_item_decay(item_id);
                self.items.remove(item_id);
                self.start_decay(target_id);
                return Ok(target_id);
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
        let is_magic_field = item_type_info.map(|t| t.is_magic_field()).unwrap_or(false);
        let always_on_top = item_type_info.map(|t| t.always_on_top()).unwrap_or(false);
        let new_order = item_type_info.map(|t| t.always_on_top_order).unwrap_or(0);

        // TFS `Tile::addThing` magic-field replace (`tile.cpp:917-938`) / 772 `CreateField`.
        if is_magic_field {
            self.remove_replaceable_magic_fields_on_tile(pos);
        }

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
            if let Some(it) = self.items_db.items.get(&item_type) {
                crate::map::apply_item_tile_flags(tile.body_mut(), it, &self.items_db);
            }
        }

        if let Some(item) = self.items.get_mut(item_id) {
            item.parent = Some(crate::cylinder::Cylinder::Tile { pos });
        }

        // Stackpos of the item as it now sits on the tile — TVP vs Cip map-container order.
        let (tvp_stack, cip_stack) = self.item_stack_pos_pair(pos, item_id);

        // Broadcast add
        self.broadcast_tile_item_add(pos, item_id, tvp_stack, cip_stack);

        // TFS `AddItemField` / `onStepInField` — fields under standing creatures deal DoT.
        // (Lua `onStepInField` is a C++ native; movements.xml cannot register it as a Lua global.)
        if is_magic_field {
            self.apply_magic_field_to_tile_creatures(pos, item_id);
        } else {
            let _ = self.events.on_step_in(None, item_id, item_type, pos, pos);
        }
        self.start_decay(item_id);
        Ok(item_id)
    }

    /// Detach `item_id` from the tile without destroying the SlotMap entry (cylinder moves).
    ///
    /// Resets tile flags while the item is still on the tile — TFS `Tile::removeThing` →
    /// `resetTileFlags` (`tile.cpp:1537-1596`). Call sites that previously used only
    /// `remove_item_by_id` left stale `BLOCKSOLID` / path flags (e.g. after moving a table).
    pub(crate) fn detach_item_from_tile(
        &mut self,
        pos: Position,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        let item_type = self
            .items
            .get(item_id)
            .map(|i| i.item_type)
            .ok_or(ReturnValue::NotPossible)?;
        let (tvp_stack, cip_stack) = self.item_stack_pos_pair(pos, item_id);
        if let Some(old_it) = self.items_db.items.get(&item_type).cloned() {
            if let Some(tile) = self.map.get_tile(pos) {
                let rem = crate::map::tile_remaining_props(
                    tile.body(),
                    &self.items,
                    &self.items_db,
                    item_id,
                );
                if let Some(tile) = self.map.get_tile_mut(pos) {
                    crate::map::reset_item_tile_flags(
                        tile.body_mut(),
                        &old_it,
                        &rem,
                        &self.items_db,
                    );
                }
            }
        }
        let tile = self.map.get_tile_mut(pos).ok_or(ReturnValue::NotPossible)?;
        if tile.remove_item_by_id(item_id).is_none() {
            return Err(ReturnValue::NotPossible);
        }
        self.broadcast_tile_item_remove(pos, tvp_stack, cip_stack);
        if let Some(item) = self.items.get_mut(item_id) {
            item.parent = None;
        }
        Ok(())
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
            self.detach_item_from_tile(pos, item_id)?;
            self.cancel_item_decay(item_id);
            self.items.remove(item_id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod detach_tile_flag_tests {
    use super::*;
    use crate::cylinder::{Cylinder, CylinderFlags};
    use crate::item::Item;
    use crate::sim_harness::minimal_world;
    use crate::tile::{flags, Tile};
    use tfs_rust_common::Position;
    use tfs_rust_content::otb::ItemType;

    fn register_type(world: &mut GameWorld, item_type_id: u16, mut it: ItemType) {
        it.id = item_type_id;
        it.server_id = item_type_id;
        let mut items = std::collections::HashMap::clone(&world.items_db.items);
        items.insert(item_type_id, it);
        let client_to_server = std::collections::HashMap::clone(&world.items_db.client_to_server);
        world.items_db = std::sync::Arc::new(tfs_rust_content::items::ItemDatabase {
            items,
            client_to_server,
        });
    }

    /// Tile→Tile move must clear source `BLOCKSOLID` (tables / furniture).
    /// Regression: move used `remove_item_by_id` without `resetTileFlags`.
    #[test]
    fn tile_to_tile_move_clears_source_blocksolid() {
        let mut world = minimal_world();
        const TABLE: u16 = 9201;
        let mut table = ItemType::default();
        table.block_solid_override = Some(true);
        table.moveable_override = Some(true);
        register_type(&mut world, TABLE, table);

        let from = Position::new(60, 60, 7);
        let to = Position::new(61, 60, 7);
        for pos in [from, to] {
            world.map.insert_tile(pos, Tile::empty_normal());
            if let Some(tile) = world.map.get_tile_mut(pos) {
                tile.body_mut().ground = Some(100);
            }
        }

        let iid = world.items.insert(Item::new_single(TABLE));
        world
            .internal_add_item_to_tile(from, iid, CylinderFlags::NO_LIMIT)
            .expect("place table");
        assert_ne!(
            world.map.get_tile(from).unwrap().body().flags & flags::BLOCKSOLID,
            0,
            "table sets BLOCKSOLID on source"
        );

        world
            .internal_move_item(
                None,
                Cylinder::Tile { pos: from },
                Cylinder::Tile { pos: to },
                iid,
                1,
                CylinderFlags::NO_LIMIT,
                None,
            )
            .expect("move table");

        assert_eq!(
            world.map.get_tile(from).unwrap().body().flags & flags::BLOCKSOLID,
            0,
            "source tile must be walkable after table leaves"
        );
        assert_ne!(
            world.map.get_tile(to).unwrap().body().flags & flags::BLOCKSOLID,
            0,
            "dest tile must take BLOCKSOLID from the table"
        );
        assert!(
            world
                .map
                .get_tile(from)
                .unwrap()
                .body()
                .down_items
                .is_empty(),
            "table gone from source"
        );
    }
}
