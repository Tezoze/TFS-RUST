//! Item moves between cylinders (inventory, container, tile).
//!
//! - `Game::internalMoveItem` — `game.cpp` ~1078.

use crate::creature::CreatureKind;
use crate::cylinder::{Cylinder, CylinderFlags, CylinderLink};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::item::Item;
use crate::player_inventory_notifications::NotificationParent;
use crate::return_value::ReturnValue;

impl GameWorld {
    /// Move an item between cylinders. Handles tile↔tile with stackable merge/split.
    /// Returns the ItemId that ended up at the destination.
    // C++ ref: src/game.cpp:1078 Game::internalMoveItem
    pub fn internal_move_item(
        &mut self,
        acting_player: Option<CreatureId>,
        from_cylinder: Cylinder,
        to_cylinder: Cylinder,
        item_id: ItemId,
        count: u16,
        flags: CylinderFlags,
    ) -> Result<ItemId, ReturnValue> {
        // Validate source has the item
        self.validate_item_in_cylinder(&from_cylinder, item_id)?;
        let source_parent = from_cylinder.as_container();

        let (to_work, mut to_merge_item) =
            self.resolve_move_destination(to_cylinder, item_id, source_parent, flags)?;

        // For tile destinations, check queryAdd
        if let Cylinder::Tile { pos } = to_work {
            let rv = self.query_add_item_to_tile(pos, item_id, flags);
            if rv.is_error() {
                return Err(rv);
            }
        }
        if let Cylinder::Inventory {
            player_id: to_pid,
            slot: to_slot,
        } = to_work
        {
            let move_count = self
                .items
                .get(item_id)
                .map(|i| (i.count as u32).min(u32::from(count)))
                .unwrap_or(1);
            let rv = self.player_query_add(to_pid, to_slot, item_id, move_count, flags);
            match rv {
                ReturnValue::NeedExchange => {
                    self.try_resolve_inventory_need_exchange(
                        acting_player,
                        &from_cylinder,
                        to_pid,
                        to_slot,
                        item_id,
                        to_merge_item,
                        flags,
                    )?;
                    // C++ `toItem = nullptr` after swap — `game.cpp` ~1157.
                    to_merge_item = None;
                }
                ReturnValue::NoError => {}
                _ => return Err(rv),
            }
        }
        if let Cylinder::Container { item_id: cid, index } = to_work {
            let m_pre = self.items.get(item_id).map(|i| i.count).unwrap_or(1);
            let m_pre = m_pre.min(count);
            let ret = self.container_query_add(
                cid,
                index,
                item_id,
                u32::from(m_pre),
                flags,
                acting_player,
            );
            if ret.is_error() {
                return Err(ret);
            }
        }

        let item = self.items.get(item_id).ok_or(ReturnValue::NotPossible)?;
        let is_stackable = self.items_db.items.get(&item.item_type).map(|t| t.stackable()).unwrap_or(false);
        let item_count = item.count;
        let item_type = item.item_type;

        let m = if is_stackable { count.min(item_count) } else { item_count };

        if to_merge_item == Some(item_id) {
            return Ok(item_id);
        }

        let max_query_count: u32 = match to_work {
            Cylinder::Container { item_id: cid, index } => {
                self.container_query_max_count(cid, index, item_id, u32::from(m), flags)?
            }
            Cylinder::Inventory {
                player_id: to_pid,
                slot: to_slot,
            } => self.player_query_max_count(
                to_pid,
                Self::player_max_count_index(to_slot),
                item_id,
                u32::from(m),
                flags,
            )?,
            _ => u32::from(m),
        };
        let mut m_move = m.min(max_query_count as u16);

        if let Some(merge_id) = to_merge_item {
            m_move = m_move.min(self.stack_merge_room(merge_id));
        }

        if let Cylinder::Inventory { player_id, .. } = &from_cylinder {
            let rv = self.player_query_remove(*player_id, item_id, u32::from(m_move), flags);
            if rv.is_error() {
                return Err(rv);
            }
        }

        match (&from_cylinder, &to_work) {
            (Cylinder::Tile { pos: from_pos }, Cylinder::Tile { pos: to_pos }) => {
                let from_pos = *from_pos;
                let to_pos = *to_pos;

                if is_stackable && m < item_count {
                    // Partial stack move — reduce source count, create new item for destination
                    if let Some(src) = self.items.get_mut(item_id) {
                        src.count -= m;
                    }
                    let src_stack_pos = self.map.get_tile(from_pos)
                        .and_then(|t| t.get_item_stack_pos(item_id))
                        .unwrap_or(0);
                    self.broadcast_tile_item_update(from_pos, item_id, src_stack_pos);

                    // Create new item for the moved portion
                    let new_item = Item::new(item_type, m);
                    let new_id = self.items.insert(new_item);
                    self.internal_add_item_to_tile(to_pos, new_id, flags)?;
                    Ok(new_id)
                } else {
                    // Full move
                    let stack_pos = self.map.get_tile(from_pos)
                        .and_then(|t| t.get_item_stack_pos(item_id))
                        .unwrap_or(0);
                    let tile = self.map.get_tile_mut(from_pos).ok_or(ReturnValue::NotPossible)?;
                    if tile.remove_item_by_id(item_id).is_none() {
                        return Err(ReturnValue::NotPossible);
                    }
                    self.broadcast_tile_item_remove(from_pos, stack_pos);
                    self.internal_add_item_to_tile(to_pos, item_id, flags)
                }
            }
            (Cylinder::Tile { pos: from_pos }, Cylinder::Container { .. }) => {
                let Cylinder::Container {
                    item_id: dest_cid,
                    index: dest_idx,
                } = to_work
                else {
                    return Err(ReturnValue::NotPossible);
                };
                let from_pos = *from_pos;
                if is_stackable && m_move < item_count {
                    if let Some(merge_id) = to_merge_item {
                        if merge_id == item_id {
                            return Ok(item_id);
                        }
                        self.ensure_stack_merge_room(
                            merge_id,
                            m_move,
                            ReturnValue::ContainerNotEnoughRoom,
                        )?;
                        self.merge_partial_stack_counts(item_id, merge_id, m_move);
                        let src_stack_pos = self
                            .map
                            .get_tile(from_pos)
                            .and_then(|t| t.get_item_stack_pos(item_id))
                            .unwrap_or(0);
                        self.broadcast_tile_item_update(from_pos, item_id, src_stack_pos);
                        self.notify_container_stack_merge(dest_cid, merge_id);
                        return Ok(merge_id);
                    }
                    return Err(ReturnValue::NotPossible);
                }
                let dest_has_room = self
                    .container_registry
                    .get(dest_cid)
                    .map(|c| !c.is_full())
                    .ok_or(ReturnValue::NotPossible)?;
                if !dest_has_room {
                    return Err(ReturnValue::ContainerNotEnoughRoom);
                }
                let stack_pos = self
                    .map
                    .get_tile(from_pos)
                    .and_then(|t| t.get_item_stack_pos(item_id))
                    .unwrap_or(0);
                let tile = self.map.get_tile_mut(from_pos).ok_or(ReturnValue::NotPossible)?;
                if tile.remove_item_by_id(item_id).is_none() {
                    return Err(ReturnValue::NotPossible);
                }
                self.broadcast_tile_item_remove(from_pos, stack_pos);
                if let Some(merge_id) = to_merge_item {
                    self.ensure_stack_merge_room(
                        merge_id,
                        m_move,
                        ReturnValue::ContainerNotEnoughRoom,
                    )?;
                    self.merge_detached_stack_counts(merge_id, m_move);
                    self.items.remove(item_id);
                    self.notify_container_stack_merge(dest_cid, merge_id);
                    return Ok(merge_id);
                }
                self.container_add_thing(dest_cid, dest_idx, item_id)?;
                Ok(item_id)
            }
            (Cylinder::Container { .. }, Cylinder::Tile { pos: to_pos }) => {
                let Cylinder::Container {
                    item_id: from_cid,
                    ..
                } = from_cylinder
                else {
                    return Err(ReturnValue::NotPossible);
                };
                let to_pos = *to_pos;
                if is_stackable && m_move < item_count {
                    let rv = self.container_query_remove(
                        from_cid,
                        item_id,
                        u32::from(m_move),
                        flags,
                        acting_player,
                    );
                    if rv.is_error() {
                        return Err(rv);
                    }
                    self.container_remove_thing(from_cid, item_id, u32::from(m_move))?;
                    let new_item = Item::new(item_type, m_move);
                    let new_id = self.items.insert(new_item);
                    self.internal_add_item_to_tile(to_pos, new_id, flags)?;
                    return Ok(new_id);
                }
                let rv = self.container_query_remove(
                    from_cid,
                    item_id,
                    u32::from(m_move),
                    flags,
                    acting_player,
                );
                if rv.is_error() {
                    return Err(rv);
                }
                self.container_detach_item(from_cid, item_id)?;
                self.internal_add_item_to_tile(to_pos, item_id, flags)
            }
            (Cylinder::Container { .. }, Cylinder::Container { .. }) => {
                let Cylinder::Container {
                    item_id: from_cid,
                    ..
                } = from_cylinder
                else {
                    return Err(ReturnValue::NotPossible);
                };
                let Cylinder::Container {
                    item_id: dest_cid,
                    index: dest_idx,
                } = to_work
                else {
                    return Err(ReturnValue::NotPossible);
                };
                let dest_has_room = self
                    .container_registry
                    .get(dest_cid)
                    .map(|c| !c.is_full())
                    .ok_or(ReturnValue::NotPossible)?;
                if !dest_has_room {
                    return Err(ReturnValue::ContainerNotEnoughRoom);
                }
                if is_stackable && m_move < item_count {
                    let rv = self.container_query_remove(
                        from_cid,
                        item_id,
                        u32::from(m_move),
                        flags,
                        acting_player,
                    );
                    if rv.is_error() {
                        return Err(rv);
                    }
                    self.container_remove_thing(from_cid, item_id, u32::from(m_move))?;
                    if let Some(merge_id) = to_merge_item {
                        if merge_id == item_id {
                            return Ok(item_id);
                        }
                        self.ensure_stack_merge_room(
                            merge_id,
                            m_move,
                            ReturnValue::ContainerNotEnoughRoom,
                        )?;
                        self.merge_detached_stack_counts(merge_id, m_move);
                        self.notify_container_stack_merge(dest_cid, merge_id);
                        return Ok(merge_id);
                    }
                    let new_item = Item::new(item_type, m_move);
                    let new_id = self.items.insert(new_item);
                    self.container_add_thing(dest_cid, dest_idx, new_id)?;
                    return Ok(new_id);
                }
                let rv = self.container_query_remove(
                    from_cid,
                    item_id,
                    u32::from(m_move),
                    flags,
                    acting_player,
                );
                if rv.is_error() {
                    return Err(rv);
                }
                if let Some(merge_id) = to_merge_item {
                    self.ensure_stack_merge_room(
                        merge_id,
                        m_move,
                        ReturnValue::ContainerNotEnoughRoom,
                    )?;
                    self.container_remove_thing(from_cid, item_id, u32::from(m_move))?;
                    self.merge_detached_stack_counts(merge_id, m_move);
                    self.notify_container_stack_merge(dest_cid, merge_id);
                    return Ok(merge_id);
                }
                let dest_has_room = self
                    .container_registry
                    .get(dest_cid)
                    .map(|c| !c.is_full())
                    .ok_or(ReturnValue::NotPossible)?;
                if !dest_has_room {
                    return Err(ReturnValue::ContainerNotEnoughRoom);
                }
                self.container_detach_item(from_cid, item_id)?;
                self.container_add_thing(dest_cid, dest_idx, item_id)?;
                Ok(item_id)
            }
            (
                Cylinder::Container {
                    item_id: from_container,
                    ..
                },
                Cylinder::Inventory {
                    player_id,
                    slot,
                },
            ) => {
                let cid = *player_id;
                let slot = *slot;
                let from_container = *from_container;
                if is_stackable && m_move < item_count {
                    let rv = self.container_query_remove(
                        from_container,
                        item_id,
                        u32::from(m_move),
                        flags,
                        acting_player,
                    );
                    if rv.is_error() {
                        return Err(rv);
                    }
                    if self.get_player_inventory_item(cid, slot).is_some() {
                        return Err(ReturnValue::NeedExchange);
                    }
                    self.container_remove_thing(from_container, item_id, u32::from(m_move))?;
                    let new_item = Item::new(item_type, m_move);
                    let new_id = self.items.insert(new_item);
                    self.equip_item_to_inventory_slot(
                        cid,
                        slot,
                        new_id,
                        NotificationParent::Container(from_container),
                    )?;
                    return Ok(new_id);
                }
                let rv = self.container_query_remove(
                    from_container,
                    item_id,
                    u32::from(m_move),
                    flags,
                    acting_player,
                );
                if rv.is_error() {
                    return Err(rv);
                }
                if let Some(dest_id) = self.get_player_inventory_item(cid, slot) {
                    if dest_id == item_id {
                        return Ok(item_id);
                    }
                    let idx = self
                        .get_thing_index_in_container(from_container, item_id)
                        .ok_or(ReturnValue::NotPossible)? as usize;
                    self.container_detach_item(from_container, item_id)?;
                    self.unequip_item_from_inventory_slot(
                        cid,
                        slot,
                        dest_id,
                        NotificationParent::Container(from_container),
                    )?;
                    self.container_insert_item_at(from_container, idx, dest_id)?;
                    self.equip_item_to_inventory_slot(
                        cid,
                        slot,
                        item_id,
                        NotificationParent::Container(from_container),
                    )?;
                    return Ok(item_id);
                }
                self.container_detach_item(from_container, item_id)?;
                self.equip_item_to_inventory_slot(
                    cid,
                    slot,
                    item_id,
                    NotificationParent::Container(from_container),
                )?;
                Ok(item_id)
            }
            (
                Cylinder::Inventory {
                    player_id,
                    slot,
                },
                Cylinder::Container {
                    item_id: to_container,
                    index: to_idx,
                },
            ) => {
                let cid = *player_id;
                let slot = *slot;
                let to_container = *to_container;
                let to_idx = *to_idx;

                if let Some(merge_id) = to_merge_item {
                    if merge_id == item_id {
                        return Ok(item_id);
                    }
                    self.ensure_stack_merge_room(
                        merge_id,
                        m_move,
                        ReturnValue::ContainerNotEnoughRoom,
                    )?;
                    if is_stackable && m_move < item_count {
                        self.merge_partial_stack_counts(item_id, merge_id, m_move);
                        self.notify_player_container_tree_changed(
                            cid,
                            to_container,
                            merge_id,
                            false,
                            NotificationParent::Player,
                        );
                        self.notify_container_stack_merge(to_container, merge_id);
                        self.broadcast_player_inventory_slot(cid, slot, Some(item_id));
                        return Ok(merge_id);
                    }
                    self.unequip_item_from_inventory_slot(
                        cid,
                        slot,
                        item_id,
                        NotificationParent::Container(to_container),
                    )?;
                    self.merge_detached_stack_counts(merge_id, m_move);
                    self.items.remove(item_id);
                    self.notify_container_stack_merge(to_container, merge_id);
                    return Ok(merge_id);
                }

                if is_stackable && m_move < item_count {
                    let new_item = Item::new(item_type, m_move);
                    let new_id = self.items.insert(new_item);
                    if let Some(src) = self.items.get_mut(item_id) {
                        src.count = src.count.saturating_sub(m_move);
                    }
                    self.broadcast_player_inventory_slot(cid, slot, Some(item_id));
                    self.hydrate_container_if_needed(to_container);
                    self.container_add_thing(to_container, to_idx, new_id)?;
                    self.notify_player_container_tree_changed(
                        cid,
                        to_container,
                        new_id,
                        true,
                        NotificationParent::Player,
                    );
                    return Ok(new_id);
                }
                let dest_has_room = self
                    .container_registry
                    .get(to_container)
                    .map(|c| !c.is_full())
                    .ok_or(ReturnValue::NotPossible)?;
                if !dest_has_room {
                    return Err(ReturnValue::ContainerNotEnoughRoom);
                }
                self.unequip_item_from_inventory_slot(
                    cid,
                    slot,
                    item_id,
                    NotificationParent::Container(to_container),
                )?;
                self.hydrate_container_if_needed(to_container);
                self.container_add_thing(to_container, to_idx, item_id)?;
                Ok(item_id)
            }
            (Cylinder::Tile { pos: from_pos }, Cylinder::Inventory { player_id, slot }) => {
                let from_pos = *from_pos;
                let cid = *player_id;
                let slot = *slot;

                if let Some(merge_id) = to_merge_item {
                    if merge_id == item_id {
                        return Ok(item_id);
                    }
                    self.ensure_stack_merge_room(
                        merge_id,
                        m_move,
                        ReturnValue::NotEnoughCapacity,
                    )?;
                    if is_stackable && m_move < item_count {
                        // Partial: source stack stays on tile; only counts change.
                        self.merge_partial_stack_counts(item_id, merge_id, m_move);
                        let src_stack_pos = self
                            .map
                            .get_tile(from_pos)
                            .and_then(|t| t.get_item_stack_pos(item_id))
                            .unwrap_or(0);
                        self.broadcast_tile_item_update(from_pos, item_id, src_stack_pos);
                        self.player_post_add_notification(
                            cid,
                            merge_id,
                            slot,
                            CylinderLink::TopParent,
                            NotificationParent::Tile(from_pos),
                        );
                        self.broadcast_player_inventory_slot(cid, slot, Some(merge_id));
                        return Ok(merge_id);
                    }
                    let stack_pos = self
                        .map
                        .get_tile(from_pos)
                        .and_then(|t| t.get_item_stack_pos(item_id))
                        .unwrap_or(0);
                    let tile = self.map.get_tile_mut(from_pos).ok_or(ReturnValue::NotPossible)?;
                    if tile.remove_item_by_id(item_id).is_none() {
                        return Err(ReturnValue::NotPossible);
                    }
                    self.broadcast_tile_item_remove(from_pos, stack_pos);
                    // Full: source removed from tile above — only bump hand stack.
                    self.merge_detached_stack_counts(merge_id, m_move);
                    self.player_post_add_notification(
                        cid,
                        merge_id,
                        slot,
                        CylinderLink::TopParent,
                        NotificationParent::Tile(from_pos),
                    );
                    self.broadcast_player_inventory_slot(cid, slot, Some(merge_id));
                    return Ok(merge_id);
                }

                if is_stackable && m_move < item_count {
                    if self.get_player_inventory_item(cid, slot).is_some() {
                        return Err(ReturnValue::NeedExchange);
                    }
                    if let Some(src) = self.items.get_mut(item_id) {
                        src.count -= m_move;
                    }
                    let src_stack_pos = self
                        .map
                        .get_tile(from_pos)
                        .and_then(|t| t.get_item_stack_pos(item_id))
                        .unwrap_or(0);
                    self.broadcast_tile_item_update(from_pos, item_id, src_stack_pos);
                    let new_item = Item::new(item_type, m_move);
                    let new_id = self.items.insert(new_item);
                    self.equip_item_to_inventory_slot(
                        cid,
                        slot,
                        new_id,
                        NotificationParent::Tile(from_pos),
                    )?;
                    return Ok(new_id);
                }
                if let Some(dest_id) = self.get_player_inventory_item(cid, slot) {
                    if dest_id == item_id {
                        return Ok(item_id);
                    }
                    let stack_pos = self.map.get_tile(from_pos)
                        .and_then(|t| t.get_item_stack_pos(item_id))
                        .unwrap_or(0);
                    let tile = self.map.get_tile_mut(from_pos).ok_or(ReturnValue::NotPossible)?;
                    if tile.remove_item_by_id(item_id).is_none() {
                        return Err(ReturnValue::NotPossible);
                    }
                    self.broadcast_tile_item_remove(from_pos, stack_pos);
                    self.unequip_item_from_inventory_slot(
                        cid,
                        slot,
                        dest_id,
                        NotificationParent::Tile(from_pos),
                    )?;
                    self.internal_add_item_to_tile(from_pos, dest_id, flags)?;
                    self.equip_item_to_inventory_slot(
                        cid,
                        slot,
                        item_id,
                        NotificationParent::Tile(from_pos),
                    )?;
                    return Ok(item_id);
                }
                let stack_pos = self.map.get_tile(from_pos)
                    .and_then(|t| t.get_item_stack_pos(item_id))
                    .unwrap_or(0);
                let tile = self.map.get_tile_mut(from_pos).ok_or(ReturnValue::NotPossible)?;
                if tile.remove_item_by_id(item_id).is_none() {
                    return Err(ReturnValue::NotPossible);
                }
                self.broadcast_tile_item_remove(from_pos, stack_pos);
                self.equip_item_to_inventory_slot(
                    cid,
                    slot,
                    item_id,
                    NotificationParent::Tile(from_pos),
                )?;
                Ok(item_id)
            }
            (Cylinder::Inventory { player_id, slot }, Cylinder::Tile { pos: to_pos }) => {
                let cid = *player_id;
                let slot = *slot;
                let to_pos = *to_pos;
                self.unequip_item_from_inventory_slot(
                    cid,
                    slot,
                    item_id,
                    NotificationParent::Tile(to_pos),
                )?;
                self.internal_add_item_to_tile(to_pos, item_id, flags)?;
                Ok(item_id)
            }
            (
                Cylinder::Inventory {
                    player_id: from_pid,
                    slot: from_slot,
                },
                Cylinder::Inventory {
                    player_id: to_pid,
                    slot: to_slot,
                },
            ) => {
                // `Game::internalMoveItem` inventory↔inventory — `game.cpp` ~1078 (Player cylinders).
                if *from_pid != *to_pid {
                    return Err(ReturnValue::NotPossible);
                }
                let cid = *from_pid;
                if *from_slot == *to_slot {
                    return Ok(item_id);
                }
                if is_stackable && m < item_count {
                    return Err(ReturnValue::NotPossible);
                }
                let dest_id = self.get_player_inventory_item(cid, *to_slot);
                if let Some(did) = dest_id {
                    if did == item_id {
                        return Ok(item_id);
                    }
                    let dest_count = self.items.get(did).map(|i| i.count as u32).unwrap_or(1);
                    let rv = self.player_query_add(cid, *from_slot, did, dest_count, flags);
                    if rv != ReturnValue::NoError {
                        return Err(rv);
                    }
                    let idx_f = crate::inventory::slot_to_array_index(*from_slot)
                        .ok_or(ReturnValue::NotPossible)?;
                    let idx_t = crate::inventory::slot_to_array_index(*to_slot)
                        .ok_or(ReturnValue::NotPossible)?;
                    if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                        p.equipment_slots[idx_f] = Some(did);
                        p.equipment_slots[idx_t] = Some(item_id);
                    }
                    self.player_post_remove_notification(
                        cid,
                        item_id,
                        *from_slot,
                        CylinderLink::Owner,
                        NotificationParent::Player,
                    );
                    self.player_post_add_notification(
                        cid,
                        item_id,
                        *to_slot,
                        CylinderLink::Owner,
                        NotificationParent::Player,
                    );
                    self.player_post_remove_notification(
                        cid,
                        did,
                        *to_slot,
                        CylinderLink::Owner,
                        NotificationParent::Player,
                    );
                    self.player_post_add_notification(
                        cid,
                        did,
                        *from_slot,
                        CylinderLink::Owner,
                        NotificationParent::Player,
                    );
                    self.broadcast_player_inventory_slot(cid, *from_slot, Some(did));
                    self.broadcast_player_inventory_slot(cid, *to_slot, Some(item_id));
                    return Ok(item_id);
                }
                self.unequip_item_from_inventory_slot(
                    cid,
                    *from_slot,
                    item_id,
                    NotificationParent::Player,
                )?;
                self.equip_item_to_inventory_slot(
                    cid,
                    *to_slot,
                    item_id,
                    NotificationParent::Player,
                )?;
                Ok(item_id)
            }
        }
    }
}
