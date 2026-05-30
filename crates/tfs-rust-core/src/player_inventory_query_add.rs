//! TFS `Player` inventory cylinder queries — `player.cpp` ~2397–2841.
// C++ reference: `Player::queryAdd`, `queryMaxCount`, `queryRemove`, `queryDestination`, `hasCapacity`.

use crate::config::ConfigManager;
use crate::container::ContainerIterator;
use crate::creature::CreatureKind;
use crate::cylinder::{Cylinder, CylinderFlags, INDEX_WHEREEVER};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::{
    InventorySlot, SLOTP_AMMO, SLOTP_ARMOR, SLOTP_BACKPACK, SLOTP_FEET, SLOTP_HEAD, SLOTP_LEFT,
    SLOTP_LEGS, SLOTP_NECKLACE, SLOTP_RING, SLOTP_RIGHT, SLOTP_TWO_HAND, WEAPON_NONE, WEAPON_SHIELD,
};
use crate::return_value::ReturnValue;
use tfs_rust_content::otb::ItemType;

/// C++ `WEAPON_AMMO` — `src/const.h`
const WEAPON_AMMO: u8 = 7;

/// `CONST_SLOT_FIRST`..=`CONST_SLOT_LAST` — `creature.h` (excludes store inbox 11).
const PLAYER_INVENTORY_SLOT_FIRST: u8 = 1;
const PLAYER_INVENTORY_SLOT_LAST: u8 = 10;

/// Result of `Player::queryDestination` — `player.cpp` ~2718–2841.
pub(crate) enum PlayerDestResolution {
    StayHere {
        slot: u8,
        dest_stack_item: Option<ItemId>,
    },
    Redirect(Cylinder),
}

/// Occupied equipment slot metadata for hand-conflict checks.
#[derive(Debug, Clone, Copy)]
struct OccupiedSlot {
    item_id: ItemId,
    slot_position: u32,
    weapon_type: u8,
    count: u16,
}

/// C++ `ConfigManager::CLASSIC_EQUIPMENT_SLOTS` — `config.lua` `classicEquipmentSlots`.
pub fn classic_equipment_slots_from_config(config: &ConfigManager) -> bool {
    crate::config::get_bool_or(config, "classicEquipmentSlots", false).unwrap_or(false)
}

/// Default `ReturnValue` before the per-slot `switch` — `player.cpp` ~2422–2438.
fn player_query_add_default_ret(classic_equipment_slots: bool, slot_position: u32) -> ReturnValue {
    if (slot_position & SLOTP_HEAD != 0)
        || (slot_position & SLOTP_NECKLACE != 0)
        || (slot_position & SLOTP_BACKPACK != 0)
        || (slot_position & SLOTP_ARMOR != 0)
        || (slot_position & SLOTP_LEGS != 0)
        || (slot_position & SLOTP_FEET != 0)
        || (slot_position & SLOTP_RING != 0)
    {
        ReturnValue::CannotBeDressed
    } else if slot_position & SLOTP_TWO_HAND != 0 {
        ReturnValue::PutThisObjectInBothHands
    } else if (slot_position & SLOTP_RIGHT != 0) || (slot_position & SLOTP_LEFT != 0) {
        if classic_equipment_slots {
            ReturnValue::PutThisObjectInYourHand
        } else {
            ReturnValue::CannotBeDressed
        }
    } else {
        ReturnValue::NotPossible
    }
}

/// Weapon / two-hand conflict when equipping into a hand while the opposite hand is occupied.
fn hand_slot_conflict_ret(
    moving_item_id: ItemId,
    moving_count: u16,
    weapon_type: u8,
    opposite: OccupiedSlot,
) -> ReturnValue {
    if opposite.slot_position & SLOTP_TWO_HAND != 0 {
        return ReturnValue::DropTwoHandedItem;
    }
    if opposite.item_id == moving_item_id && moving_count == opposite.count {
        return ReturnValue::NoError;
    }
    if opposite.weapon_type == WEAPON_SHIELD && weapon_type == WEAPON_SHIELD {
        return ReturnValue::CanOnlyUseOneShield;
    }
    if opposite.weapon_type == WEAPON_NONE
        || weapon_type == WEAPON_NONE
        || opposite.weapon_type == WEAPON_SHIELD
        || opposite.weapon_type == WEAPON_AMMO
        || weapon_type == WEAPON_SHIELD
        || weapon_type == WEAPON_AMMO
    {
        ReturnValue::NoError
    } else {
        ReturnValue::CanOnlyUseOneWeapon
    }
}

/// Pure slot rules from `Player::queryAdd` `switch (index)` — `player.cpp` ~2440–2593.
pub(crate) fn evaluate_player_inventory_slot_query(
    index: u8,
    classic_equipment_slots: bool,
    item_type: &ItemType,
    moving_item_id: ItemId,
    moving_count: u16,
    left: Option<OccupiedSlot>,
    right: Option<OccupiedSlot>,
) -> ReturnValue {
    let slot_position = item_type.slot_position;
    let weapon_type = item_type.weapon_type;
    let mut ret = player_query_add_default_ret(classic_equipment_slots, slot_position);

    match index {
        x if x == InventorySlot::Head as u8 => {
            if slot_position & SLOTP_HEAD != 0 {
                ret = ReturnValue::NoError;
            }
        }
        x if x == InventorySlot::Necklace as u8 => {
            if slot_position & SLOTP_NECKLACE != 0 {
                ret = ReturnValue::NoError;
            }
        }
        x if x == InventorySlot::Backpack as u8 => {
            if slot_position & SLOTP_BACKPACK != 0 {
                ret = ReturnValue::NoError;
            }
        }
        x if x == InventorySlot::Armor as u8 => {
            if slot_position & SLOTP_ARMOR != 0 {
                ret = ReturnValue::NoError;
            }
        }
        x if x == InventorySlot::Right as u8 => {
            if slot_position & SLOTP_RIGHT != 0 {
                if !classic_equipment_slots {
                    if weapon_type != WEAPON_SHIELD {
                        ret = ReturnValue::CannotBeDressed;
                    } else if let Some(left_item) = left {
                        if (left_item.slot_position | slot_position) & SLOTP_TWO_HAND != 0 {
                            ret = ReturnValue::BothHandsNeedToBeFree;
                        } else {
                            ret = ReturnValue::NoError;
                        }
                    } else {
                        ret = ReturnValue::NoError;
                    }
                } else if slot_position & SLOTP_TWO_HAND != 0 {
                    if let Some(left_item) = left {
                        if left_item.item_id != moving_item_id {
                            ret = ReturnValue::BothHandsNeedToBeFree;
                        } else {
                            ret = ReturnValue::NoError;
                        }
                    } else {
                        ret = ReturnValue::NoError;
                    }
                } else if let Some(left_item) = left {
                    ret = hand_slot_conflict_ret(moving_item_id, moving_count, weapon_type, left_item);
                } else {
                    ret = ReturnValue::NoError;
                }
            }
        }
        x if x == InventorySlot::Left as u8 => {
            if slot_position & SLOTP_LEFT != 0 {
                if !classic_equipment_slots {
                    if matches!(weapon_type, WEAPON_NONE | WEAPON_SHIELD | WEAPON_AMMO) {
                        ret = ReturnValue::CannotBeDressed;
                    } else if right.is_some() && slot_position & SLOTP_TWO_HAND != 0 {
                        ret = ReturnValue::BothHandsNeedToBeFree;
                    } else {
                        ret = ReturnValue::NoError;
                    }
                } else if slot_position & SLOTP_TWO_HAND != 0 {
                    if let Some(right_item) = right {
                        if right_item.item_id != moving_item_id {
                            ret = ReturnValue::BothHandsNeedToBeFree;
                        } else {
                            ret = ReturnValue::NoError;
                        }
                    } else {
                        ret = ReturnValue::NoError;
                    }
                } else if let Some(right_item) = right {
                    ret = hand_slot_conflict_ret(moving_item_id, moving_count, weapon_type, right_item);
                } else {
                    ret = ReturnValue::NoError;
                }
            }
        }
        x if x == InventorySlot::Legs as u8 => {
            if slot_position & SLOTP_LEGS != 0 {
                ret = ReturnValue::NoError;
            }
        }
        x if x == InventorySlot::Feet as u8 => {
            if slot_position & SLOTP_FEET != 0 {
                ret = ReturnValue::NoError;
            }
        }
        x if x == InventorySlot::Ring as u8 => {
            if slot_position & SLOTP_RING != 0 {
                ret = ReturnValue::NoError;
            }
        }
        x if x == InventorySlot::Ammo as u8 => {
            if (slot_position & SLOTP_AMMO != 0) || classic_equipment_slots {
                ret = ReturnValue::NoError;
            }
        }
        x if x == InventorySlot::Wherever as u8 => {
            ret = ReturnValue::NotEnoughRoom;
        }
        _ => {
            ret = ReturnValue::NotPossible;
        }
    }

    ret
}

impl GameWorld {
    /// TFS `Player::queryAdd` — `player.cpp` ~2397–2617.
    pub(crate) fn player_query_add(
        &mut self,
        cid: CreatureId,
        index: u8,
        item_id: ItemId,
        count: u32,
        flags: CylinderFlags,
    ) -> ReturnValue {
        let (item_type, item_count, is_store_item) = {
            let Some(item) = self.items.get(item_id) else {
                return ReturnValue::NotPossible;
            };
            (item.item_type, item.count, item.is_store_item())
        };

        if flags.contains(CylinderFlags::CHILD_IS_OWNER) {
            if flags.contains(CylinderFlags::NO_LIMIT)
                || self.player_has_capacity(cid, item_id, count, flags)
            {
                return ReturnValue::NoError;
            }
            return ReturnValue::NotEnoughCapacity;
        }

        let Some(it) = self.items_db.items.get(&item_type) else {
            return ReturnValue::NotPossible;
        };
        if !it.pickupable() {
            return ReturnValue::CannotPickup;
        }
        if is_store_item {
            return ReturnValue::ItemCannotBeMovedThere;
        }

        let classic = classic_equipment_slots_from_config(&self.config);
        let left = self.occupied_slot(cid, InventorySlot::Left as u8);
        let right = self.occupied_slot(cid, InventorySlot::Right as u8);

        let mut ret = evaluate_player_inventory_slot_query(
            index,
            classic,
            it,
            item_id,
            item_count,
            left,
            right,
        );

        if ret != ReturnValue::NoError && ret != ReturnValue::NotEnoughRoom {
            return ret;
        }

        if !self.player_has_capacity(cid, item_id, count, flags) {
            return ReturnValue::NotEnoughCapacity;
        }

        if index != InventorySlot::Wherever as u8 {
            let probe =
                crate::lua_scope::fire_on_player_equip_check(self, cid, item_id, index);
            if probe != ReturnValue::NoError {
                return probe;
            }
        }

        if index == InventorySlot::Wherever as u8 {
            return ret;
        }

        if let Some(dest_id) = self.get_player_inventory_item(cid, index) {
            let dest_stackable = self
                .items_db
                .items
                .get(&self.items.get(dest_id).map(|i| i.item_type).unwrap_or(0))
                .map(|t| t.stackable())
                .unwrap_or(false);
            let dest_type = self.items.get(dest_id).map(|i| i.item_type).unwrap_or(0);
            if !dest_stackable || dest_type != item_type {
                return ReturnValue::NeedExchange;
            }
        }

        ret
    }

    /// TFS `Player::hasCapacity` — `player.cpp` ~2380–2395.
    pub(crate) fn player_has_capacity(
        &self,
        cid: CreatureId,
        item_id: ItemId,
        count: u32,
        flags: CylinderFlags,
    ) -> bool {
        if flags.contains(CylinderFlags::NO_LIMIT) {
            return true;
        }
        if self.player_has_flag(cid, crate::player_flags::PLAYER_FLAG_CANNOT_PICKUP_ITEM) {
            return false;
        }
        if self.player_has_flag(cid, crate::player_flags::PLAYER_FLAG_HAS_INFINITE_CAPACITY) {
            return true;
        }
        if self.player_carries_item(cid, item_id) {
            return true;
        }
        let Some(item) = self.items.get(item_id) else {
            return false;
        };
        let weight = self.player_query_weight_for_capacity(item_id, item, count);
        let Some(free) = self.player_free_capacity_u32(cid) else {
            return false;
        };
        weight <= free
    }

    fn player_query_weight_for_capacity(
        &self,
        item_id: ItemId,
        item: &crate::item::Item,
        count: u32,
    ) -> u32 {
        if self.items_db.is_container(item.item_type) {
            return self.item_recursive_weight_oz(item_id);
        }
        let it = self.items_db.items.get(&item.item_type);
        let tw = it.map(|t| t.weight).unwrap_or(0);
        let stackable = it.map(|t| t.stackable()).unwrap_or(false);
        if stackable {
            let unit = item.attributes.base_weight_oz(tw);
            unit.saturating_mul(count.max(1))
        } else {
            item.total_weight_oz(tw, false)
        }
    }

    fn player_carries_item(&self, cid: CreatureId, item_id: ItemId) -> bool {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return false;
        };
        for &slot_item in p.equipment_slots.iter().flatten() {
            if slot_item == item_id {
                return true;
            }
            if let Some(c) = self.container_registry.get(slot_item) {
                if c.is_holding_item(&self.container_registry, item_id) {
                    return true;
                }
            }
        }
        false
    }

    fn occupied_slot(&self, cid: CreatureId, slot: u8) -> Option<OccupiedSlot> {
        let dest_id = self.get_player_inventory_item(cid, slot)?;
        let dest_item = self.items.get(dest_id)?;
        let dest_type = self.items_db.items.get(&dest_item.item_type)?;
        Some(OccupiedSlot {
            item_id: dest_id,
            slot_position: dest_type.slot_position,
            weapon_type: dest_type.weapon_type,
            count: dest_item.count,
        })
    }

    /// C++ `Player::getThingIndex` — `player.cpp` ~2954–2961 (equipment slots 1–10 only).
    pub(crate) fn inventory_thing_index(&self, player_id: CreatureId, item_id: ItemId) -> Option<u8> {
        for slot in PLAYER_INVENTORY_SLOT_FIRST..=PLAYER_INVENTORY_SLOT_LAST {
            if self.get_player_inventory_item(player_id, slot) == Some(item_id) {
                return Some(slot);
            }
        }
        None
    }

    /// Trade item excluded from auto-destination — `player.cpp` ~2737. Stub until trade is ported.
    fn player_trade_item(&self, _player_id: CreatureId) -> Option<ItemId> {
        None
    }

    fn player_skip_destination_item(
        &self,
        candidate: ItemId,
        moving_item_id: ItemId,
        trade_item: Option<ItemId>,
    ) -> bool {
        candidate == moving_item_id || trade_item == Some(candidate)
    }

    /// Map inventory `Cylinder::Inventory` slot to `queryMaxCount` index (`0` → `INDEX_WHEREEVER`).
    pub(crate) fn player_max_count_index(slot: u8) -> i32 {
        if slot == InventorySlot::Wherever as u8 {
            INDEX_WHEREEVER
        } else {
            i32::from(slot)
        }
    }

    /// TFS `Player::queryRemove` — `player.cpp` ~2695–2716.
    pub(crate) fn player_query_remove(
        &self,
        player_id: CreatureId,
        item_id: ItemId,
        count: u32,
        flags: CylinderFlags,
    ) -> ReturnValue {
        if self.inventory_thing_index(player_id, item_id).is_none() {
            return ReturnValue::NotPossible;
        }
        let Some(item) = self.items.get(item_id) else {
            return ReturnValue::NotPossible;
        };
        let stackable = self
            .items_db
            .items
            .get(&item.item_type)
            .map(|t| t.stackable())
            .unwrap_or(false);
        if count == 0 || (stackable && count > item.count as u32) {
            return ReturnValue::NotPossible;
        }
        let moveable = self
            .items_db
            .items
            .get(&item.item_type)
            .map(|t| t.moveable())
            .unwrap_or(false);
        if !moveable && !flags.contains(CylinderFlags::IGNORE_NOT_MOVEABLE) {
            return ReturnValue::NotMoveable;
        }
        ReturnValue::NoError
    }

    /// TFS `Player::queryMaxCount` — `player.cpp` ~2619–2693.
    pub(crate) fn player_query_max_count(
        &mut self,
        player_id: CreatureId,
        index: i32,
        item_id: ItemId,
        count: u32,
        flags: CylinderFlags,
    ) -> Result<u32, ReturnValue> {
        let Some(item) = self.items.get(item_id) else {
            return Err(ReturnValue::NotPossible);
        };
        let item_count = item.count as u32;
        let stackable = self
            .items_db
            .items
            .get(&item.item_type)
            .map(|t| t.stackable())
            .unwrap_or(false);

        let max_query_count = if index == INDEX_WHEREEVER {
            let mut n = 0u32;
            for slot_index in PLAYER_INVENTORY_SLOT_FIRST..=PLAYER_INVENTORY_SLOT_LAST {
                if let Some(inventory_item) = self.get_player_inventory_item(player_id, slot_index) {
                    let inv_type = self.items.get(inventory_item).map(|i| i.item_type).unwrap_or(0);
                    if self.items_db.is_container(inv_type) {
                        if let Ok(q) = self.container_query_max_count(
                            inventory_item,
                            INDEX_WHEREEVER,
                            item_id,
                            item_count,
                            flags,
                        ) {
                            n = n.saturating_add(q);
                        }
                        let nested_containers: Vec<ItemId> =
                            ContainerIterator::new(&self.container_registry, inventory_item).collect();
                        for nested in nested_containers {
                            let nested_type =
                                self.items.get(nested).map(|i| i.item_type).unwrap_or(0);
                            if self.items_db.is_container(nested_type) {
                                if let Ok(q) = self.container_query_max_count(
                                    nested,
                                    INDEX_WHEREEVER,
                                    item_id,
                                    item_count,
                                    flags,
                                ) {
                                    n = n.saturating_add(q);
                                }
                            }
                        }
                    } else if stackable
                        && self.items_stack_mergeable(item_id, inventory_item)
                        && self.items.get(inventory_item).is_some_and(|i| i.count < 100)
                    {
                        let remainder = 100u32.saturating_sub(
                            self.items.get(inventory_item).map(|i| i.count).unwrap_or(0) as u32,
                        );
                        if self.player_query_add(
                            player_id,
                            slot_index,
                            item_id,
                            remainder,
                            flags,
                        ) == ReturnValue::NoError
                        {
                            n = n.saturating_add(remainder);
                        }
                    }
                } else if self.player_query_add(
                    player_id,
                    slot_index,
                    item_id,
                    item_count,
                    flags,
                ) == ReturnValue::NoError
                {
                    if stackable {
                        n = n.saturating_add(100);
                    } else {
                        n = n.saturating_add(1);
                    }
                }
            }
            n
        } else {
            let slot = index as u8;
            let mut max = 0u32;
            if let Some(dest_id) = self.get_player_inventory_item(player_id, slot) {
                if stackable
                    && self.items_stack_mergeable(item_id, dest_id)
                    && self.items.get(dest_id).is_some_and(|d| d.count < 100)
                {
                    max = 100u32.saturating_sub(self.items.get(dest_id).map(|d| d.count).unwrap_or(0) as u32);
                }
            } else if self.player_query_add(player_id, slot, item_id, count, flags)
                == ReturnValue::NoError
            {
                if stackable {
                    return Ok(100);
                }
                return Ok(1);
            }
            max
        };

        if max_query_count < count {
            Err(ReturnValue::NotEnoughRoom)
        } else {
            Ok(max_query_count)
        }
    }

    /// TFS `Player::queryDestination` — `player.cpp` ~2718–2841.
    pub(crate) fn player_query_destination(
        &mut self,
        player_id: CreatureId,
        slot: u8,
        item_id: ItemId,
        flags: CylinderFlags,
    ) -> Result<PlayerDestResolution, ReturnValue> {
        if slot == InventorySlot::Wherever as u8 || slot == INDEX_WHEREEVER as u8 {
            return self.player_query_destination_wherever(player_id, item_id, flags);
        }

        if let Some(dest_id) = self.get_player_inventory_item(player_id, slot) {
            if dest_id == item_id {
                return Ok(PlayerDestResolution::StayHere {
                    slot,
                    dest_stack_item: None,
                });
            }
            let dest_type = self.items.get(dest_id).map(|i| i.item_type).unwrap_or(0);
            if self.items_db.is_container(dest_type) {
                return Ok(PlayerDestResolution::Redirect(Cylinder::Container {
                    item_id: dest_id,
                    index: INDEX_WHEREEVER,
                }));
            }
            return Ok(PlayerDestResolution::StayHere {
                slot,
                dest_stack_item: Some(dest_id),
            });
        }

        Ok(PlayerDestResolution::StayHere {
            slot,
            dest_stack_item: None,
        })
    }

    fn player_query_destination_wherever(
        &mut self,
        player_id: CreatureId,
        item_id: ItemId,
        flags: CylinderFlags,
    ) -> Result<PlayerDestResolution, ReturnValue> {
        let Some(item) = self.items.get(item_id) else {
            return Ok(PlayerDestResolution::StayHere {
                slot: InventorySlot::Wherever as u8,
                dest_stack_item: None,
            });
        };

        let trade_item = self.player_trade_item(player_id);
        let auto_stack = !flags.contains(CylinderFlags::IGNORE_AUTO_STACK);
        let stackable = self
            .items_db
            .items
            .get(&item.item_type)
            .map(|t| t.stackable())
            .unwrap_or(false);
        let item_count = item.count as u32;

        let mut containers: Vec<ItemId> = Vec::new();

        for slot_index in PLAYER_INVENTORY_SLOT_FIRST..=PLAYER_INVENTORY_SLOT_LAST {
            let Some(inventory_item) = self.get_player_inventory_item(player_id, slot_index) else {
                if self.player_query_add(player_id, slot_index, item_id, item_count, flags)
                    == ReturnValue::NoError
                {
                    return Ok(PlayerDestResolution::StayHere {
                        slot: slot_index,
                        dest_stack_item: None,
                    });
                }
                continue;
            };

            if self.player_skip_destination_item(inventory_item, item_id, trade_item) {
                continue;
            }

            let inv_type = self.items.get(inventory_item).map(|i| i.item_type).unwrap_or(0);
            if self.items_db.is_container(inv_type) {
                if auto_stack && stackable {
                    if self.player_query_add(player_id, slot_index, item_id, item_count, CylinderFlags::NONE)
                        == ReturnValue::NoError
                        && self.items_stack_mergeable(item_id, inventory_item)
                        && self.items.get(inventory_item).is_some_and(|i| i.count < 100)
                    {
                        return Ok(PlayerDestResolution::StayHere {
                            slot: slot_index,
                            dest_stack_item: Some(inventory_item),
                        });
                    }
                    containers.push(inventory_item);
                } else {
                    containers.push(inventory_item);
                }
            } else if auto_stack && stackable {
                if self.player_query_add(player_id, slot_index, item_id, item_count, CylinderFlags::NONE)
                    == ReturnValue::NoError
                    && self.items_stack_mergeable(item_id, inventory_item)
                    && self.items.get(inventory_item).is_some_and(|i| i.count < 100)
                {
                    return Ok(PlayerDestResolution::StayHere {
                        slot: slot_index,
                        dest_stack_item: Some(inventory_item),
                    });
                }
            }
        }

        let mut i = 0usize;
        while i < containers.len() {
            let tmp_container = containers[i];
            i += 1;

            let Some(cont) = self.container_registry.get(tmp_container) else {
                continue;
            };
            let cont_capacity = cont.capacity;
            let cont_size = cont.size();
            let cont_items: Vec<ItemId> = cont.items.clone();

            if !auto_stack || !stackable {
                let free = cont_capacity.saturating_sub(cont_size as u32);
                let mut n = free;
                while n > 0 {
                    let try_index = cont_capacity.saturating_sub(n) as i32;
                    if self.container_query_add(
                        tmp_container,
                        try_index,
                        item_id,
                        item_count,
                        flags,
                        None,
                    ) == ReturnValue::NoError
                    {
                        return Ok(PlayerDestResolution::Redirect(Cylinder::Container {
                            item_id: tmp_container,
                            index: try_index,
                        }));
                    }
                    n -= 1;
                }
                for &list_item in &cont_items {
                    let child_type = self.items.get(list_item).map(|it| it.item_type).unwrap_or(0);
                    if self.items_db.is_container(child_type) {
                        containers.push(list_item);
                    }
                }
                continue;
            }

            let mut n: i32 = 0;
            for &tmp_item in &cont_items {
                if self.player_skip_destination_item(tmp_item, item_id, trade_item) {
                    n += 1;
                    continue;
                }
                if self.items_stack_mergeable(item_id, tmp_item)
                    && self.items.get(tmp_item).is_some_and(|t| t.count < 100)
                {
                    return Ok(PlayerDestResolution::Redirect(Cylinder::Container {
                        item_id: tmp_container,
                        index: n,
                    }));
                }
                let child_type = self.items.get(tmp_item).map(|it| it.item_type).unwrap_or(0);
                if self.items_db.is_container(child_type) {
                    containers.push(tmp_item);
                }
                n += 1;
            }

            if (n as u32) < cont_capacity
                && self.container_query_add(tmp_container, n, item_id, item_count, flags, None)
                    == ReturnValue::NoError
            {
                return Ok(PlayerDestResolution::Redirect(Cylinder::Container {
                    item_id: tmp_container,
                    index: n,
                }));
            }
        }

        Ok(PlayerDestResolution::StayHere {
            slot: InventorySlot::Wherever as u8,
            dest_stack_item: None,
        })
    }

    /// C++ `internalMoveItem` NeedExchange block — `game.cpp` ~1118–1159 (inventory destination).
    pub(crate) fn try_resolve_inventory_need_exchange(
        &mut self,
        acting_player: Option<CreatureId>,
        from_cylinder: &Cylinder,
        to_pid: CreatureId,
        to_slot: u8,
        moving_item_id: ItemId,
        to_merge_item: Option<ItemId>,
        flags: CylinderFlags,
    ) -> Result<(), ReturnValue> {
        let exchange_id = to_merge_item
            .or_else(|| self.get_player_inventory_item(to_pid, to_slot))
            .filter(|&id| id != moving_item_id);
        let Some(exchange_id) = exchange_id else {
            return Err(ReturnValue::NeedExchange);
        };
        let exchange_count = self
            .items
            .get(exchange_id)
            .map(|i| i.count as u32)
            .unwrap_or(1);

        let can_add_to_source = match from_cylinder {
            Cylinder::Inventory {
                player_id,
                slot,
            } => self.player_query_add(*player_id, *slot, exchange_id, exchange_count, CylinderFlags::NONE),
            Cylinder::Container {
                item_id: from_cid,
                ..
            } => {
                let idx = self
                    .get_thing_index_in_container(*from_cid, moving_item_id)
                    .unwrap_or(INDEX_WHEREEVER);
                self.container_query_add(
                    *from_cid,
                    idx,
                    exchange_id,
                    exchange_count,
                    CylinderFlags::NONE,
                    acting_player,
                )
            }
            Cylinder::Tile { pos } => self.query_add_item_to_tile(*pos, exchange_id, CylinderFlags::NONE),
            _ => ReturnValue::NotPossible,
        };
        if can_add_to_source != ReturnValue::NoError {
            return Err(can_add_to_source);
        }

        let max_exchange = match from_cylinder {
            Cylinder::Inventory { player_id, slot } => self.player_query_max_count(
                *player_id,
                Self::player_max_count_index(*slot),
                exchange_id,
                exchange_count,
                CylinderFlags::NONE,
            ),
            Cylinder::Container {
                item_id: from_cid,
                index,
            } => self.container_query_max_count(
                *from_cid,
                *index,
                exchange_id,
                exchange_count,
                CylinderFlags::NONE,
            ),
            // C++ `Tile::queryMaxCount` — `tile.cpp` ~706–709.
            Cylinder::Tile { .. } => Ok(exchange_count),
            _ => Err(ReturnValue::NotPossible),
        };
        let Ok(max_exchange) = max_exchange else {
            return Err(max_exchange.unwrap_err());
        };
        if max_exchange == 0 {
            return Err(ReturnValue::NotEnoughRoom);
        }

        if self.player_query_remove(to_pid, exchange_id, exchange_count, flags) != ReturnValue::NoError
        {
            return Err(ReturnValue::NotPossible);
        }

        match from_cylinder {
            Cylinder::Inventory {
                player_id,
                slot,
            } => {
                self.internal_remove_item_from_inventory_slot(to_pid, to_slot, exchange_id)?;
                self.broadcast_player_inventory_slot(to_pid, to_slot, None);
                self.internal_add_item_to_inventory_slot(*player_id, *slot, exchange_id)?;
                self.broadcast_player_inventory_slot(*player_id, *slot, Some(exchange_id));
            }
            Cylinder::Container {
                item_id: from_cid,
                index: from_idx,
            } => {
                self.internal_remove_item_from_inventory_slot(to_pid, to_slot, exchange_id)?;
                self.broadcast_player_inventory_slot(to_pid, to_slot, None);
                self.container_add_thing(*from_cid, *from_idx, exchange_id)?;
            }
            Cylinder::Tile { pos } => {
                self.internal_remove_item_from_inventory_slot(to_pid, to_slot, exchange_id)?;
                self.broadcast_player_inventory_slot(to_pid, to_slot, None);
                self.internal_add_item_to_tile(*pos, exchange_id, flags)?;
            }
            _ => return Err(ReturnValue::NotPossible),
        }

        let move_count = self
            .items
            .get(moving_item_id)
            .map(|i| i.count as u32)
            .unwrap_or(1);
        let rv = self.player_query_add(to_pid, to_slot, moving_item_id, move_count, flags);
        if rv != ReturnValue::NoError {
            return Err(rv);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::SLOTP_HAND;
    use slotmap::SlotMap;
    use tfs_rust_content::otb::ItemType;

    fn sword_type() -> ItemType {
        let mut it = ItemType::default();
        it.slot_position = SLOTP_LEFT | SLOTP_RIGHT;
        it.weapon_type = 1; // sword
        it
    }

    fn shield_type() -> ItemType {
        let mut it = ItemType::default();
        it.slot_position = SLOTP_RIGHT;
        it.weapon_type = WEAPON_SHIELD;
        it
    }

    fn two_hand_type() -> ItemType {
        let mut it = ItemType::default();
        it.slot_position = SLOTP_TWO_HAND | SLOTP_LEFT | SLOTP_RIGHT;
        it.weapon_type = 1;
        it
    }

    fn head_type() -> ItemType {
        let mut it = ItemType::default();
        it.slot_position = SLOTP_HEAD;
        it
    }

    fn shovel_type() -> ItemType {
        let mut it = ItemType::default();
        it.slot_position = SLOTP_HAND;
        it.weapon_type = WEAPON_NONE;
        it
    }

    fn test_config(lua_source: &str, tag: &str) -> ConfigManager {
        let path = std::env::temp_dir().join(format!(
            "tfs_classic_equipment_slots_test_{}_{}.lua",
            std::process::id(),
            tag,
        ));
        std::fs::write(&path, lua_source).expect("write temp config.lua");
        ConfigManager::load(&path).expect("load temp config.lua")
    }

    #[test]
    fn head_item_fits_head_slot() {
        let ret = evaluate_player_inventory_slot_query(
            InventorySlot::Head as u8,
            false,
            &head_type(),
            ItemId::default(),
            1,
            None,
            None,
        );
        assert_eq!(ret, ReturnValue::NoError);
    }

    #[test]
    fn non_classic_sword_in_right_slot_cannot_be_dressed() {
        let ret = evaluate_player_inventory_slot_query(
            InventorySlot::Right as u8,
            false,
            &sword_type(),
            ItemId::default(),
            1,
            None,
            None,
        );
        assert_eq!(ret, ReturnValue::CannotBeDressed);
    }

    #[test]
    fn non_classic_shield_in_right_slot_ok() {
        let ret = evaluate_player_inventory_slot_query(
            InventorySlot::Right as u8,
            false,
            &shield_type(),
            ItemId::default(),
            1,
            None,
            None,
        );
        assert_eq!(ret, ReturnValue::NoError);
    }

    #[test]
    fn two_handed_with_left_occupied_needs_free_hands() {
        // C++ `CONST_SLOT_LEFT` + non-classic: `inventory[CONST_SLOT_RIGHT]` blocks two-hand
        // (`player.cpp` ~2522–2523).
        let right_id = SlotMap::<ItemId, ()>::with_key().insert(());
        let right = OccupiedSlot {
            item_id: right_id,
            slot_position: SLOTP_RIGHT,
            weapon_type: 1,
            count: 1,
        };
        let ret = evaluate_player_inventory_slot_query(
            InventorySlot::Left as u8,
            false,
            &two_hand_type(),
            ItemId::default(),
            1,
            None,
            Some(right),
        );
        assert_eq!(ret, ReturnValue::BothHandsNeedToBeFree);
    }

    #[test]
    fn dual_weapon_blocked() {
        let mut sm: SlotMap<ItemId, ()> = SlotMap::with_key();
        let left_id = sm.insert(());
        let move_id = sm.insert(());
        let left = OccupiedSlot {
            item_id: left_id,
            slot_position: SLOTP_LEFT,
            weapon_type: 1,
            count: 1,
        };
        let ret = evaluate_player_inventory_slot_query(
            InventorySlot::Right as u8,
            true,
            &sword_type(),
            move_id,
            1,
            Some(left),
            None,
        );
        assert_eq!(ret, ReturnValue::CanOnlyUseOneWeapon);
    }

    #[test]
    fn dual_shield_blocked() {
        let mut sm: SlotMap<ItemId, ()> = SlotMap::with_key();
        let left_id = sm.insert(());
        let move_id = sm.insert(());
        let left = OccupiedSlot {
            item_id: left_id,
            slot_position: SLOTP_LEFT,
            weapon_type: WEAPON_SHIELD,
            count: 1,
        };
        let ret = evaluate_player_inventory_slot_query(
            InventorySlot::Right as u8,
            true,
            &shield_type(),
            move_id,
            1,
            Some(left),
            None,
        );
        assert_eq!(ret, ReturnValue::CanOnlyUseOneShield);
    }

    #[test]
    fn classic_ammo_slot_accepts_non_ammo() {
        let mut generic = ItemType::default();
        generic.slot_position = SLOTP_HAND;
        let ret = evaluate_player_inventory_slot_query(
            InventorySlot::Ammo as u8,
            true,
            &generic,
            ItemId::default(),
            1,
            None,
            None,
        );
        assert_eq!(ret, ReturnValue::NoError);
    }

    /// Regression: tools (e.g. shovel) in hand slots — `player.cpp` ~2516–2552 with classic slots.
    #[test]
    fn classic_hand_slots_accept_tools_like_tfs() {
        let shovel = shovel_type();
        for slot in [InventorySlot::Left as u8, InventorySlot::Right as u8] {
            let ret = evaluate_player_inventory_slot_query(
                slot,
                true,
                &shovel,
                ItemId::default(),
                1,
                None,
                None,
            );
            assert_eq!(ret, ReturnValue::NoError, "slot {slot}");
        }
    }

    #[test]
    fn non_classic_hand_slots_reject_tools_like_tfs() {
        let shovel = shovel_type();
        for slot in [InventorySlot::Left as u8, InventorySlot::Right as u8] {
            let ret = evaluate_player_inventory_slot_query(
                slot,
                false,
                &shovel,
                ItemId::default(),
                1,
                None,
                None,
            );
            assert_eq!(ret, ReturnValue::CannotBeDressed, "slot {slot}");
        }
    }

    #[test]
    fn non_classic_left_shield_cannot_be_dressed() {
        let ret = evaluate_player_inventory_slot_query(
            InventorySlot::Left as u8,
            false,
            &shield_type(),
            ItemId::default(),
            1,
            None,
            None,
        );
        assert_eq!(ret, ReturnValue::CannotBeDressed);
    }

    #[test]
    fn classic_equipment_slots_config_default() {
        let cfg = test_config("", "default");
        assert!(!classic_equipment_slots_from_config(&cfg));
    }

    #[test]
    fn classic_equipment_slots_config_reads_key() {
        let cfg = test_config("classicEquipmentSlots = true", "enabled");
        assert!(classic_equipment_slots_from_config(&cfg));
    }

    #[test]
    fn player_max_count_index_maps_wherever() {
        assert_eq!(
            GameWorld::player_max_count_index(InventorySlot::Wherever as u8),
            INDEX_WHEREEVER
        );
        assert_eq!(GameWorld::player_max_count_index(3), 3);
    }

    #[test]
    fn player_inventory_slot_range_matches_creature_h() {
        assert_eq!(PLAYER_INVENTORY_SLOT_FIRST, 1);
        assert_eq!(PLAYER_INVENTORY_SLOT_LAST, 10);
    }
}
