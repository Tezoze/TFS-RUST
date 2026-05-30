//! Inventory cylinder moves, equip UI updates, quick-equip, look-at.
// C++ reference: `src/game.cpp` `internalMoveItem`, `playerEquipItem`, `playerLookAt`.

use tfs_rust_common::ConnId;
use tfs_rust_common::Position;
use tfs_rust_net::outgoing_extra::{send_inventory_item_template, send_inventory_slot_empty, send_text_message_simple};

use crate::container_ui::ContainerContentChange;
use crate::creature::CreatureKind;
use crate::item_look::{item_get_description_cpp, look_distance_tfs};
use crate::cylinder::{Cylinder, CylinderFlags, INDEX_WHEREEVER};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::slot_type_for_item_type;
use crate::item::Item;
use crate::return_value::ReturnValue;
use crate::thing::LookTarget;
use slotmap::Key;

impl GameWorld {
    pub(crate) fn conn_id_for_creature(&self, cid: CreatureId) -> Option<ConnId> {
        self.conn_to_creature
            .iter()
            .find(|(_, &c)| c == cid)
            .map(|(&conn, _)| conn)
    }

    /// `Player` inventory slot item — `Game::internalGetThing` inventory branch (`game.cpp` ~320–326).
    pub fn get_player_inventory_item(&self, cid: CreatureId, slot: u8) -> Option<ItemId> {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return None;
        };
        let idx = crate::inventory::slot_to_array_index(slot)?;
        p.equipment_slots[idx]
    }

    pub(crate) fn validate_inventory_item(
        &self,
        cid: CreatureId,
        slot: u8,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        let Some(have) = self.get_player_inventory_item(cid, slot) else {
            return Err(ReturnValue::NotPossible);
        };
        if have != item_id {
            return Err(ReturnValue::NotPossible);
        }
        Ok(())
    }

    /// Capacity-only path for child containers (`Player::queryAdd` + `FLAG_CHILDISOWNER`).
    pub(crate) fn query_add_item_to_inventory(
        &self,
        cid: CreatureId,
        item_id: ItemId,
    ) -> ReturnValue {
        let count = self.items.get(item_id).map(|i| i.count as u32).unwrap_or(1);
        self.player_query_add(
            cid,
            crate::inventory::InventorySlot::Wherever as u8,
            item_id,
            count,
            CylinderFlags::CHILD_IS_OWNER,
        )
    }

    pub(crate) fn internal_remove_item_from_inventory_slot(
        &mut self,
        cid: CreatureId,
        slot: u8,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        self.validate_inventory_item(cid, slot, item_id)?;
        self.events.on_player_deequip(cid, item_id, slot);
        let idx = crate::inventory::slot_to_array_index(slot).ok_or(ReturnValue::NotPossible)?;
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.equipment_slots[idx] = None;
        }
        Ok(())
    }

    pub(crate) fn internal_add_item_to_inventory_slot(
        &mut self,
        cid: CreatureId,
        slot: u8,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        let idx = crate::inventory::slot_to_array_index(slot).ok_or(ReturnValue::NotPossible)?;
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.equipment_slots[idx] = Some(item_id);
        }
        self.events.on_player_equip(cid, item_id, slot);
        // C++: carrying a bag updates parent chain / `totalWeight` — ground→inventory must keep registry in sync for look/capacity.
        self.hydrate_container_if_needed(item_id);
        Ok(())
    }

    /// Remove a specific item instance from a registered container (`container.cpp`).
    pub(crate) fn internal_remove_item_from_container(
        &mut self,
        container_item_id: ItemId,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        self.container_detach_item(container_item_id, item_id)
    }

    /// Resolve `CreatureId` from Lua / protocol `KeyData::as_ffi` bits.
    pub(crate) fn resolve_creature_u64(&self, id: u64) -> Option<CreatureId> {
        self.creatures
            .iter()
            .find(|(k, _)| k.data().as_ffi() == id)
            .map(|(k, _)| k)
    }

    /// Lua `Player:addItem` — adds a new stack into the backpack container (`luascript.cpp` `Player::addItem` subset).
    pub fn lua_script_add_item(
        &mut self,
        creature_u64: u64,
        item_type: u16,
        count: u16,
    ) -> Result<(), String> {
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "creature not found".to_string())?;
        let count = count.max(1);
        let new_item = Item::new(ItemId::default(), item_type, count);
        let iid = self.items.insert(new_item);
        if self.query_add_item_to_inventory(cid, iid) != ReturnValue::NoError {
            self.items.remove(iid);
            return Err("not enough capacity".into());
        }
        let backpack = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.equipment_slots[2],
            _ => None,
        };
        let Some(bp) = backpack else {
            self.items.remove(iid);
            return Err("no backpack".into());
        };
        let mut registry = std::mem::take(&mut self.container_registry);
        self.ensure_container_registered(&mut registry, bp);
        self.container_registry = registry;
        let cont = self
            .container_registry
            .get_mut(bp)
            .ok_or_else(|| "backpack container missing".to_string())?;
        if cont.add_item(iid).is_err() {
            self.items.remove(iid);
            return Err("backpack full".into());
        }
        self.refresh_container_chain(bp);
        self.recompute_player_inventory_weight(cid);
        Ok(())
    }

    /// Lua `Player:removeItem` — removes from backpack contents only (subset until full inventory scan exists).
    pub fn lua_script_remove_item(
        &mut self,
        creature_u64: u64,
        item_type: u16,
        mut count: u32,
    ) -> Result<(), String> {
        if count == 0 {
            return Ok(());
        }
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "creature not found".to_string())?;
        let backpack = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.equipment_slots[2],
            _ => None,
        };
        let Some(bp) = backpack else {
            return Err("no backpack".into());
        };
        let mut registry = std::mem::take(&mut self.container_registry);
        self.ensure_container_registered(&mut registry, bp);
        self.container_registry = registry;
        self.lua_remove_item_type_from_container(bp, item_type, &mut count)?;
        if count > 0 {
            return Err("not enough items".into());
        }
        self.recompute_player_inventory_weight(cid);
        Ok(())
    }

    fn lua_remove_item_type_from_container(
        &mut self,
        container_item_id: ItemId,
        item_type: u16,
        count: &mut u32,
    ) -> Result<(), String> {
        if *count == 0 {
            return Ok(());
        }
        let child_ids: Vec<ItemId> = self
            .container_registry
            .get(container_item_id)
            .map(|c| c.items.clone())
            .ok_or_else(|| "container not registered".to_string())?;
        for iid in child_ids {
            if *count == 0 {
                break;
            }
            let ty = self.items.get(iid).map(|i| i.item_type);
            if ty == Some(item_type) {
                let n = self.items.get(iid).map(|i| i.count as u32).unwrap_or(1);
                let take = n.min(*count);
                if take == n {
                    self.internal_remove_item_from_container(container_item_id, iid)
                        .map_err(|_| "remove from container failed".to_string())?;
                    self.items.remove(iid);
                } else if let Some(it) = self.items.get_mut(iid) {
                    it.count -= take as u16;
                }
                *count -= take;
            } else if ty.is_some_and(|t| self.items_db.is_container(t)) {
                self.lua_remove_item_type_from_container(iid, item_type, count)?;
            }
        }
        self.refresh_container_chain(container_item_id);
        Ok(())
    }

    pub(crate) fn broadcast_player_inventory_slot(
        &mut self,
        cid: CreatureId,
        slot: u8,
        item_id: Option<ItemId>,
    ) {
        let Some(conn) = self.conn_id_for_creature(cid) else {
            return;
        };
        let with_desc = self
            .creatures
            .get(cid)
            .and_then(|k| match k {
                CreatureKind::Player(p) => Some(p.item_with_description()),
                _ => None,
            })
            .unwrap_or(false);

        if let Some(iid) = item_id {
            let Some(item) = self.items.get(iid) else {
                self.enqueue_outgoing(conn, send_inventory_slot_empty(slot).into_bytes());
                return;
            };
            let sid = item.item_type;
            let cid_client = self.items_db.client_id_for_server(sid);
            if cid_client == 0 {
                self.enqueue_outgoing(conn, send_inventory_slot_empty(slot).into_bytes());
                return;
            }
            let cnt = item.client_count().max(1);
            let stackable = self.items_db.stackable_for_server(sid);
            let splash = self.items_db.is_splash_or_fluid_for_server(sid);
            let anim = self.items_db.is_animation_for_server(sid);
            self.enqueue_outgoing(
                conn,
                send_inventory_item_template(
                    slot,
                    cid_client,
                    cnt,
                    stackable,
                    splash && !stackable,
                    anim,
                    with_desc,
                )
                .into_bytes(),
            );
        } else {
            self.enqueue_outgoing(conn, send_inventory_slot_empty(slot).into_bytes());
        }
        self.send_player_stats(cid);
    }

    /// `Game::playerEquipItem` — `game.cpp` ~1851–1877.
    pub fn player_quick_equip(&mut self, conn_id: ConnId, cid: CreatureId, sprite_id: u16) {
        let Some(server_id) = self.items_db.server_id_for_client(sprite_id) else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };
        let Some(it) = self.items_db.items.get(&server_id) else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };
        let target_slot = slot_type_for_item_type(it);
        let backpack_id = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.equipment_slots[2],
            _ => None,
        };
        let Some(backpack_id) = backpack_id else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };
        let bp_type = self.items.get(backpack_id).map(|i| i.item_type).unwrap_or(0);
        if !self.items_db.is_container(bp_type) {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        }
        let mut registry = std::mem::take(&mut self.container_registry);
        self.ensure_container_registered(&mut registry, backpack_id);
        self.container_registry = registry;

        let Some(found_id) = self.search_item_in_container_by_type(backpack_id, server_id) else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };

        if let Some(equipped_id) = self.get_player_inventory_item(cid, target_slot) {
            if self.items.get(equipped_id).map(|i| i.item_type) == Some(server_id) {
                let n = self.items.get(equipped_id).map(|i| i.count).unwrap_or(1);
                let from = Cylinder::Inventory {
                    player_id: cid,
                    slot: target_slot,
                };
                let to = Cylinder::Container {
                    item_id: backpack_id,
                    index: INDEX_WHEREEVER,
                };
                if self
                    .internal_move_item(Some(cid), from, to, equipped_id, n, CylinderFlags::NONE)
                    .is_err()
                {
                    self.send_cancel_message(conn_id, ReturnValue::NotPossible);
                }
                return;
            }
            self.send_cancel_message(conn_id, ReturnValue::NeedExchange);
            return;
        }

        let n = self.items.get(found_id).map(|i| i.count).unwrap_or(1);
        let from = Cylinder::Container {
            item_id: backpack_id,
            index: INDEX_WHEREEVER,
        };
        let to = Cylinder::Inventory {
            player_id: cid,
            slot: target_slot,
        };
        if self
            .internal_move_item(Some(cid), from, to, found_id, n, CylinderFlags::NONE)
            .is_err()
        {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
        }
    }

    /// C++ `ITEM_STACK_SIZE` — `items.h`.
    const ITEM_STACK_MAX: u16 = 100;

    /// Remaining count that can merge into `merge_id` before hitting the stack cap.
    pub(crate) fn stack_merge_room(&self, merge_id: ItemId) -> u16 {
        Self::ITEM_STACK_MAX.saturating_sub(self.items.get(merge_id).map(|i| i.count).unwrap_or(0))
    }

    /// TFS stack-merge room check — `Game::internalMoveItem` merge paths (`game.cpp`).
    pub(crate) fn ensure_stack_merge_room(
        &self,
        merge_id: ItemId,
        m_move: u16,
        not_enough: ReturnValue,
    ) -> Result<(), ReturnValue> {
        if self.stack_merge_room(merge_id) < m_move {
            return Err(not_enough);
        }
        Ok(())
    }

    /// Partial merge: subtract from source stack and add to destination stack.
    pub(crate) fn transfer_stack_merge_counts(
        &mut self,
        source_id: ItemId,
        merge_id: ItemId,
        m_move: u16,
    ) {
        if let Some(src) = self.items.get_mut(source_id) {
            src.count = src.count.saturating_sub(m_move);
        }
        if let Some(t) = self.items.get_mut(merge_id) {
            t.count = t.count.saturating_add(m_move);
        }
    }

    /// Full merge after source detach: add moved count to the destination stack only.
    pub(crate) fn add_to_stack_merge_target(&mut self, merge_id: ItemId, m_move: u16) {
        if let Some(t) = self.items.get_mut(merge_id) {
            t.count = t.count.saturating_add(m_move);
        }
    }

    /// Hydrate container registry and notify clients that a stack slot changed.
    pub(crate) fn notify_container_stack_merge(&mut self, container_id: ItemId, merge_id: ItemId) {
        self.hydrate_container_if_needed(container_id);
        let merge_slot = self
            .get_thing_index_in_container(container_id, merge_id)
            .map(|i| i as u16)
            .unwrap_or(0);
        self.notify_container_content_changed(
            container_id,
            ContainerContentChange::Update { slot: merge_slot },
        );
    }

    fn search_item_in_container_by_type(
        &self,
        container_item_id: ItemId,
        server_type: u16,
    ) -> Option<ItemId> {
        let c = self.container_registry.get(container_item_id)?;
        for &iid in &c.items {
            if self.items.get(iid).map(|i| i.item_type) == Some(server_type) {
                return Some(iid);
            }
            let child_type = self.items.get(iid).map(|i| i.item_type)?;
            if self.items_db.is_container(child_type) {
                if let Some(found) = self.search_item_in_container_by_type(iid, server_type) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// `Game::playerLookAt` — `game.cpp` ~3156–3187.
    pub fn player_look_at(&mut self, conn_id: ConnId, cid: CreatureId, pos: Position, stack_pos: u8) {
        let Some(target) = self.internal_get_thing_look(cid, pos, stack_pos) else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };

        let player_pos = self
            .creatures
            .get(cid)
            .map(|k| k.position())
            .unwrap_or(pos);
        // C++ `thing->getPosition()` for map targets; inventory/container use player tile.
        let thing_pos = if pos.x == 0xFFFF { player_pos } else { pos };
        // C++ `player->canSee(thingPos)` — `ProtocolGame::canSee` (`protocolgame.cpp` ~796).
        if !self.can_see_position(cid, thing_pos) {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        }
        let look_d = look_distance_tfs(player_pos, thing_pos);

        let msg = match target {
            LookTarget::Creature(_) => "You see nothing special.".to_string(),
            LookTarget::Ground(ground_type) => {
                let ephemeral = Item::new_single(ItemId::default(), ground_type);
                if let Some(it) = self.items_db.items.get(&ground_type) {
                    format!(
                        "You see {}",
                        item_get_description_cpp(&ephemeral, it, it.weight, look_d, None)
                    )
                } else {
                    format!("You see an item of type {ground_type}.")
                }
            }
            LookTarget::Item(item_id) => {
                self.hydrate_container_if_needed(item_id);
                let w = self.item_recursive_weight_oz(item_id);
                let Some(item) = self.items.get(item_id) else {
                    self.send_cancel_message(conn_id, ReturnValue::NotPossible);
                    return;
                };
                let container_capacity = self
                    .container_registry
                    .get(item_id)
                    .map(|c| c.capacity);
                if let Some(it) = self.items_db.items.get(&item.item_type) {
                    format!(
                        "You see {}",
                        item_get_description_cpp(item, it, w, look_d, container_capacity)
                    )
                } else {
                    format!("You see an item of type {}.", item.item_type)
                }
            }
        };
        self.enqueue_outgoing(conn_id, send_text_message_simple(22, &msg).into_bytes());
    }
}
