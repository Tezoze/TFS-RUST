//! TFS `Player::queryAdd` for equipment slots — `player.cpp` ~2397–2617.
// C++ reference: `src/player.cpp` `Player::queryAdd`, `Player::hasCapacity` (~2380–2395).

use crate::config::ConfigManager;
use crate::creature::CreatureKind;
use crate::cylinder::CylinderFlags;
use crate::event_dispatcher::EventDispatcher;
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
        &self,
        cid: CreatureId,
        index: u8,
        item_id: ItemId,
        count: u32,
        flags: CylinderFlags,
    ) -> ReturnValue {
        let Some(item) = self.items.get(item_id) else {
            return ReturnValue::NotPossible;
        };

        if flags.contains(CylinderFlags::CHILD_IS_OWNER) {
            if flags.contains(CylinderFlags::NO_LIMIT)
                || self.player_has_capacity(cid, item_id, count, flags)
            {
                return ReturnValue::NoError;
            }
            return ReturnValue::NotEnoughCapacity;
        }

        let Some(it) = self.items_db.items.get(&item.item_type) else {
            return ReturnValue::NotPossible;
        };
        if !it.pickupable() {
            return ReturnValue::CannotPickup;
        }
        if item.is_store_item() {
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
            item.count,
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
            let probe = self
                .events
                .on_player_equip_check(cid, item_id, index);
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
            if !dest_stackable || dest_type != item.item_type {
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
        if self.player_carries_item(cid, item_id) {
            return true;
        }
        let Some(item) = self.items.get(item_id) else {
            return false;
        };
        let weight = self.player_query_weight_for_capacity(item_id, item, count);
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return false;
        };
        weight <= p.get_free_capacity_u32()
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

    fn test_config(lua_source: &str) -> ConfigManager {
        let path = std::env::temp_dir().join(format!(
            "tfs_classic_equipment_slots_test_{}.lua",
            std::process::id()
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
        let left_id = SlotMap::<ItemId, ()>::with_key().insert(());
        let left = OccupiedSlot {
            item_id: left_id,
            slot_position: SLOTP_LEFT,
            weapon_type: 1,
            count: 1,
        };
        let ret = evaluate_player_inventory_slot_query(
            InventorySlot::Left as u8,
            false,
            &two_hand_type(),
            ItemId::default(),
            1,
            Some(left),
            None,
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
        let cfg = test_config("");
        assert!(!classic_equipment_slots_from_config(&cfg));
    }

    #[test]
    fn classic_equipment_slots_config_reads_key() {
        let cfg = test_config("classicEquipmentSlots = true");
        assert!(classic_equipment_slots_from_config(&cfg));
    }
}
