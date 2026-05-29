//! Container `query*` / `addThing` / `removeThing` — TFS `container.cpp` parity helpers.
// C++ ref: `src/container.cpp` `Container::queryAdd`, `queryDestination`, `addThing`, `removeThing`, etc.

use crate::container::{ContainerIterator, ContainerType};
use crate::container_ui::ContainerContentChange;
use crate::creature::CreatureKind;
use crate::cylinder::{Cylinder, CylinderFlags, INDEX_ADD_WHEREVER, INDEX_MOVE_UP, INDEX_WHEREEVER};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use crate::tile::Tile;
/// Result of `Container::queryDestination` (`container.cpp` ~369–428).
pub(crate) enum ContainerDestResolution {
    /// Keep adding to this container (`subCylinder == this` in C++).
    StayHere {
        index: i32,
        dest_stack_item: Option<ItemId>,
    },
    /// Follow nested container or parent (`subCylinder != this`).
    Redirect(Cylinder),
}

impl GameWorld {
    /// Walk `parent_container` to the topmost item in the chain (`Thing::getTopParent` — `thing.cpp`).
    pub(crate) fn top_container_item_id(&self, mut id: ItemId) -> ItemId {
        while let Some(p) = self.container_registry.get(id).and_then(|c| c.parent_container) {
            id = p;
        }
        id
    }

    fn house_invite_blocks_container_add(
        &self,
        container_item_id: ItemId,
        actor: CreatureId,
    ) -> Option<ReturnValue> {
        let root = self.top_container_item_id(container_item_id);
        let pos = self.map.find_item_position(root)?;
        let tile = self.map.get_tile(pos)?;
        let house_id = match tile {
            Tile::House(h) => h.house_id,
            _ => return None,
        };
        let guid = match self.creatures.get(actor)? {
            CreatureKind::Player(p) => p.guid,
            _ => return None,
        };
        if self.houses.is_invited(house_id, guid) {
            None
        } else {
            Some(ReturnValue::PlayerIsNotInvited)
        }
    }

    /// TFS `Item::equals` for stacking — minimal: same type, fluid, charges (`item.cpp`).
    pub(crate) fn items_stack_mergeable(&self, a: ItemId, b: ItemId) -> bool {
        let Some(ia) = self.items.get(a) else {
            return false;
        };
        let Some(ib) = self.items.get(b) else {
            return false;
        };
        if ia.item_type != ib.item_type {
            return false;
        }
        let splash = self.items_db.is_splash_or_fluid_for_server(ia.item_type);
        if splash && ia.fluid_type() != ib.fluid_type() {
            return false;
        }
        true
    }

    /// Recompute `total_weight` / `total_item_count` for one container instance.
    // C++ ref: derived from `getWeight` / `getItemHoldingCount` (`container.cpp`).
    pub(crate) fn refresh_container_derived(&mut self, container_item_id: ItemId) {
        let Some(c) = self.container_registry.get(container_item_id) else {
            return;
        };
        let tw: u32 = c
            .items
            .iter()
            .map(|&ch| self.item_recursive_weight_oz(ch))
            .sum();
        let th = ContainerIterator::new(&self.container_registry, container_item_id).count() as u32;
        if let Some(c) = self.container_registry.get_mut(container_item_id) {
            c.total_weight = tw;
            c.total_item_count = th;
        }
    }

    pub(crate) fn refresh_container_chain(&mut self, mut container_item_id: ItemId) {
        loop {
            self.refresh_container_derived(container_item_id);
            let Some(p) = self
                .container_registry
                .get(container_item_id)
                .and_then(|c| c.parent_container)
            else {
                break;
            };
            container_item_id = p;
        }
    }

    /// Whether `player` carries `container_root` in equipment or nested bags.
    // C++ ref: implicit via `Cylinder` parent chain to `Player`.
    pub(crate) fn player_holds_container_tree(
        &self,
        player: CreatureId,
        container_root: ItemId,
    ) -> bool {
        let Some(CreatureKind::Player(p)) = self.creatures.get(player) else {
            return false;
        };
        for slot_item in p.equipment_slots.iter().flatten() {
            if *slot_item == container_root {
                return true;
            }
            if let Some(c) = self.container_registry.get(*slot_item) {
                if c.is_holding_item(&self.container_registry, container_root) {
                    return true;
                }
            }
        }
        false
    }

    /// TFS `Container::queryAdd` — `container.cpp` ~243–366.
    pub(crate) fn container_query_add(
        &self,
        container_item_id: ItemId,
        index: i32,
        item_id: ItemId,
        _count: u32,
        flags: CylinderFlags,
        actor: Option<CreatureId>,
    ) -> ReturnValue {
        if flags.contains(CylinderFlags::CHILD_IS_OWNER) {
            return ReturnValue::NoError;
        }
        let Some(cont) = self.container_registry.get(container_item_id) else {
            return ReturnValue::NotPossible;
        };
        if !cont.unlocked {
            return ReturnValue::NotPossible;
        }
        let Some(item) = self.items.get(item_id) else {
            return ReturnValue::NotPossible;
        };
        let it = self.items_db.items.get(&item.item_type);
        if !it.map(|t| t.pickupable()).unwrap_or(false) {
            return ReturnValue::CannotPickup;
        }
        if container_item_id == item_id {
            return ReturnValue::ThisIsImpossible;
        }

        if item.is_store_item()
            && !matches!(
                cont.container_type,
                ContainerType::Depot | ContainerType::StoreInbox
            )
        {
            return ReturnValue::ItemCannotBeMovedThere;
        }

        let cylinder = cont.parent_container.and_then(|pid| self.container_registry.get(pid));
        if self
            .items
            .get(container_item_id)
            .is_some_and(|ci| ci.is_store_item())
            && cylinder.is_some_and(|p| p.container_type == ContainerType::StoreInbox)
        {
            return if item.is_store_item() {
                ReturnValue::ItemCannotBeMovedThere
            } else {
                ReturnValue::CannotMoveItemIsNotStoreItem
            };
        }

        let mut cyl: Option<ItemId> = cont.parent_container;
        if !flags.contains(CylinderFlags::NO_LIMIT) {
            while let Some(pid) = cyl {
                if pid == item_id {
                    return ReturnValue::ThisIsImpossible;
                }
                if let Some(pc) = self.container_registry.get(pid) {
                    if pc.container_type == ContainerType::Inbox {
                        return ReturnValue::ContainerNotEnoughRoom;
                    }
                }
                cyl = self.container_registry.get(pid).and_then(|c| c.parent_container);
            }
            if index == INDEX_WHEREEVER && cont.size() >= cont.capacity as usize {
                return ReturnValue::ContainerNotEnoughRoom;
            }
        } else {
            let mut cyl = cont.parent_container;
            while let Some(pid) = cyl {
                if pid == item_id {
                    return ReturnValue::ThisIsImpossible;
                }
                cyl = self.container_registry.get(pid).and_then(|c| c.parent_container);
            }
        }

        if let Some(actor) = actor {
            if let Some(rv) = self.house_invite_blocks_container_add(container_item_id, actor) {
                return rv;
            }
            let mut root = container_item_id;
            while let Some(p) = self.container_registry.get(root).and_then(|c| c.parent_container) {
                root = p;
            }
            if self.player_holds_container_tree(actor, root) {
                return self.query_add_item_to_inventory(actor, item_id);
            }
        }

        ReturnValue::NoError
    }

    /// TFS `Container::queryRemove` — `container.cpp` ~331–365.
    pub(crate) fn container_query_remove(
        &self,
        container_item_id: ItemId,
        item_id: ItemId,
        count: u32,
        flags: CylinderFlags,
        _actor: Option<CreatureId>,
    ) -> ReturnValue {
        let Some(cont) = self.container_registry.get(container_item_id) else {
            return ReturnValue::NotPossible;
        };
        if !cont.contains(item_id) {
            return ReturnValue::NotPossible;
        }
        let Some(item) = self.items.get(item_id) else {
            return ReturnValue::NotPossible;
        };
        let it = self.items_db.items.get(&item.item_type);
        if count == 0 || (it.map(|t| t.stackable()).unwrap_or(false) && count > item.count as u32) {
            return ReturnValue::NotPossible;
        }
        if !it.map(|t| t.moveable()).unwrap_or(false) && !flags.contains(CylinderFlags::IGNORE_NOT_MOVEABLE)
        {
            return ReturnValue::NotMoveable;
        }
        ReturnValue::NoError
    }

    /// TFS `Container::queryMaxCount` — `container.cpp` ~311–329.
    pub(crate) fn container_query_max_count(
        &self,
        container_item_id: ItemId,
        index: i32,
        item_id: ItemId,
        count: u32,
        flags: CylinderFlags,
    ) -> Result<u32, ReturnValue> {
        let Some(item) = self.items.get(item_id) else {
            return Err(ReturnValue::NotPossible);
        };
        let it = self.items_db.items.get(&item.item_type);
        if flags.contains(CylinderFlags::NO_LIMIT) {
            return Ok(count.max(1));
        }
        let Some(cont) = self.container_registry.get(container_item_id) else {
            return Err(ReturnValue::NotPossible);
        };
        let free_slots = (cont.capacity as i32 - cont.size() as i32).max(0) as u32;

        if it.map(|t| t.stackable()).unwrap_or(false) {
            let mut n = 0u32;
            if index == INDEX_WHEREEVER {
                let mut slot_index: i32 = 0;
                for &container_item in &cont.items {
                    if container_item != item_id
                        && self.items_stack_mergeable(item_id, container_item)
                        && self.items.get(container_item).is_some_and(|ci| ci.count < 100)
                    {
                        if self.container_query_add(
                            container_item_id,
                            slot_index,
                            item_id,
                            count,
                            flags,
                            None,
                        ) == ReturnValue::NoError
                        {
                            let room = 100u32
                                .saturating_sub(self.items.get(container_item).map(|i| i.count).unwrap_or(0) as u32);
                            n = n.saturating_add(room);
                        }
                    }
                    slot_index += 1;
                }
            } else if index >= 0 {
                let idx = index as usize;
                if let Some(dest_id) = cont.get_item(idx) {
                    if self.items_stack_mergeable(item_id, dest_id)
                        && self.items.get(dest_id).is_some_and(|d| d.count < 100)
                        && self.container_query_add(container_item_id, index, item_id, count, flags, None)
                            == ReturnValue::NoError
                    {
                        n = 100u32.saturating_sub(self.items.get(dest_id).map(|i| i.count).unwrap_or(0) as u32);
                    }
                }
            }
            let max_query = free_slots.saturating_mul(100).saturating_add(n);
            if max_query < count {
                return Err(ReturnValue::ContainerNotEnoughRoom);
            }
            Ok(max_query)
        } else {
            if free_slots == 0 {
                return Err(ReturnValue::ContainerNotEnoughRoom);
            }
            Ok(free_slots)
        }
    }

    /// TFS `Container::queryDestination` — `container.cpp` ~369–428.
    pub(crate) fn container_query_destination(
        &self,
        container_item_id: ItemId,
        index: &mut i32,
        item_id: ItemId,
        source_parent_container: Option<ItemId>,
        flags: CylinderFlags,
    ) -> Result<ContainerDestResolution, ReturnValue> {
        let Some(cont) = self.container_registry.get(container_item_id) else {
            return Err(ReturnValue::NotPossible);
        };
        if !cont.unlocked {
            return Ok(ContainerDestResolution::StayHere {
                index: *index,
                dest_stack_item: None,
            });
        }

        if *index == INDEX_MOVE_UP {
            *index = INDEX_WHEREEVER;
            if let Some(parent_id) = cont.parent_container {
                return Ok(ContainerDestResolution::Redirect(Cylinder::Container {
                    item_id: parent_id,
                    index: INDEX_WHEREEVER,
                }));
            }
            return Ok(ContainerDestResolution::StayHere {
                index: INDEX_WHEREEVER,
                dest_stack_item: None,
            });
        }

        if *index == INDEX_ADD_WHEREVER {
            *index = INDEX_WHEREEVER;
        } else if *index >= cont.capacity as i32 {
            *index = INDEX_WHEREEVER;
        }

        let Some(item) = self.items.get(item_id) else {
            return Err(ReturnValue::NotPossible);
        };

        let mut dest_from_index: Option<ItemId> = None;
        if *index != INDEX_WHEREEVER {
            let idx = *index as usize;
            if let Some(at_slot) = cont.get_item(idx) {
                if self.container_registry.get(at_slot).is_some() {
                    *index = INDEX_WHEREEVER;
                    return Ok(ContainerDestResolution::Redirect(Cylinder::Container {
                        item_id: at_slot,
                        index: INDEX_WHEREEVER,
                    }));
                }
                dest_from_index = Some(at_slot);
            }
        }

        let auto_stack = !flags.contains(CylinderFlags::IGNORE_AUTO_STACK);
        let stackable = self
            .items_db
            .items
            .get(&item.item_type)
            .map(|t| t.stackable())
            .unwrap_or(false);

        if auto_stack && stackable && source_parent_container != Some(container_item_id) {
            if let Some(dest_id) = dest_from_index {
                if self.items_stack_mergeable(item_id, dest_id)
                    && self.items.get(dest_id).is_some_and(|d| d.count < 100)
                {
                    return Ok(ContainerDestResolution::StayHere {
                        index: *index,
                        dest_stack_item: Some(dest_id),
                    });
                }
            }

            let mut n: i32 = 0;
            for &list_item in &cont.items {
                if list_item != item_id
                    && self.items_stack_mergeable(item_id, list_item)
                    && self.items.get(list_item).is_some_and(|d| d.count < 100)
                {
                    return Ok(ContainerDestResolution::StayHere {
                        index: n,
                        dest_stack_item: Some(list_item),
                    });
                }
                n += 1;
            }
        }

        Ok(ContainerDestResolution::StayHere {
            index: *index,
            dest_stack_item: None,
        })
    }

    pub(crate) fn resolve_container_move_destination(
        &self,
        mut to: Cylinder,
        item_id: ItemId,
        source_parent: Option<ItemId>,
        flags: CylinderFlags,
    ) -> Result<(Cylinder, Option<ItemId>), ReturnValue> {
        let mut to_item: Option<ItemId> = None;
        let mut floor_n = 0u32;
        loop {
            match to {
                Cylinder::Container {
                    item_id: cid,
                    mut index,
                } => {
                    let res =
                        self.container_query_destination(cid, &mut index, item_id, source_parent, flags)?;
                    match res {
                        ContainerDestResolution::Redirect(next) => {
                            to = next;
                            floor_n += 1;
                            if floor_n >= 16 {
                                break;
                            }
                            continue;
                        }
                        ContainerDestResolution::StayHere {
                            index: idx,
                            dest_stack_item,
                        } => {
                            to_item = dest_stack_item;
                            to = Cylinder::Container {
                                item_id: cid,
                                index: idx,
                            };
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
        Ok((to, to_item))
    }

    pub(crate) fn get_thing_index_in_container(
        &self,
        container_item_id: ItemId,
        item_id: ItemId,
    ) -> Option<i32> {
        let c = self.container_registry.get(container_item_id)?;
        c.index_of(item_id).map(|i| i as i32)
    }

    /// Remove an item from a container list but keep the `ItemId` in `GameWorld::items` (move to tile / other container).
    pub(crate) fn container_detach_item(
        &mut self,
        container_item_id: ItemId,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        let slot_idx = self
            .get_thing_index_in_container(container_item_id, item_id)
            .ok_or(ReturnValue::NotPossible)? as u16;
        {
            let cont = self
                .container_registry
                .get_mut(container_item_id)
                .ok_or(ReturnValue::NotPossible)?;
            cont
                .remove_specific_item(item_id)
                .map_err(|_| ReturnValue::NotPossible)?;
        }
        if let Some(ch) = self.container_registry.get_mut(item_id) {
            ch.parent_container = None;
        }
        self.refresh_container_chain(container_item_id);
        self.notify_container_content_changed(
            container_item_id,
            ContainerContentChange::Remove { slot: slot_idx },
        );
        Ok(())
    }

    /// TFS `Container::removeThing` — `container.cpp` ~435–462.
    pub(crate) fn container_remove_thing(
        &mut self,
        container_item_id: ItemId,
        item_id: ItemId,
        count: u32,
    ) -> Result<(), ReturnValue> {
        let idx = self
            .get_thing_index_in_container(container_item_id, item_id)
            .ok_or(ReturnValue::NotPossible)? as usize;
        let is_stackable = self
            .items
            .get(item_id)
            .and_then(|i| self.items_db.items.get(&i.item_type))
            .map(|t| t.stackable())
            .unwrap_or(false);
        let item_count = self.items.get(item_id).map(|i| i.count).unwrap_or(0);

        if is_stackable && count < item_count as u32 {
            if let Some(item) = self.items.get_mut(item_id) {
                item.count = item_count.saturating_sub(count as u16);
            }
            self.refresh_container_chain(container_item_id);
            self.notify_container_content_changed(
                container_item_id,
                ContainerContentChange::Update {
                    slot: idx as u16,
                },
            );
            return Ok(());
        } else {
            {
                let cont = self
                    .container_registry
                    .get_mut(container_item_id)
                    .ok_or(ReturnValue::NotPossible)?;
                cont.remove_item(idx).map_err(|_| ReturnValue::NotPossible)?;
            }
            if let Some(ch) = self.container_registry.get_mut(item_id) {
                ch.parent_container = None;
            }
            self.items.remove(item_id);
        }
        self.refresh_container_chain(container_item_id);
        self.notify_container_content_changed(
            container_item_id,
            ContainerContentChange::Remove { slot: idx as u16 },
        );
        Ok(())
    }

    /// TFS `Container::addThing` — `container.cpp` ~414–433 (`push_front`).
    pub(crate) fn container_add_thing(
        &mut self,
        container_item_id: ItemId,
        index: i32,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        let cap = self
            .container_registry
            .get(container_item_id)
            .map(|c| c.capacity)
            .ok_or(ReturnValue::NotPossible)?;
        if index >= cap as i32 {
            return Err(ReturnValue::NotPossible);
        }
        {
            let cont = self
                .container_registry
                .get_mut(container_item_id)
                .ok_or(ReturnValue::NotPossible)?;
            if cont.is_full() {
                return Err(ReturnValue::ContainerNotEnoughRoom);
            }
            cont.items.insert(0, item_id);
        }
        if self.items_db.is_container(self.items.get(item_id).map(|i| i.item_type).unwrap_or(0)) {
            let mut reg = std::mem::take(&mut self.container_registry);
            self.ensure_container_registered(&mut reg, item_id);
            self.container_registry = reg;
            if let Some(ch) = self.container_registry.get_mut(item_id) {
                ch.parent_container = Some(container_item_id);
            }
        }
        self.refresh_container_chain(container_item_id);
        self.notify_container_content_changed(
            container_item_id,
            ContainerContentChange::Add { slot: 0 },
        );
        Ok(())
    }

    /// Insert at explicit index (exchange / parity with `itemlist.insert` — not only `push_front`).
    pub(crate) fn container_insert_item_at(
        &mut self,
        container_item_id: ItemId,
        index: usize,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        let cap = self
            .container_registry
            .get(container_item_id)
            .map(|c| c.capacity as usize)
            .ok_or(ReturnValue::NotPossible)?;
        if index > cap {
            return Err(ReturnValue::NotPossible);
        }
        {
            let cont = self
                .container_registry
                .get_mut(container_item_id)
                .ok_or(ReturnValue::NotPossible)?;
            if cont.items.len() >= cap {
                return Err(ReturnValue::ContainerNotEnoughRoom);
            }
            if index > cont.items.len() {
                return Err(ReturnValue::NotPossible);
            }
            cont.items.insert(index, item_id);
        }
        if self.items_db.is_container(self.items.get(item_id).map(|i| i.item_type).unwrap_or(0)) {
            let mut reg = std::mem::take(&mut self.container_registry);
            self.ensure_container_registered(&mut reg, item_id);
            self.container_registry = reg;
            if let Some(ch) = self.container_registry.get_mut(item_id) {
                ch.parent_container = Some(container_item_id);
            }
        }
        self.refresh_container_chain(container_item_id);
        self.notify_container_content_changed(
            container_item_id,
            ContainerContentChange::Add {
                slot: index as u16,
            },
        );
        Ok(())
    }
}
