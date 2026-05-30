//! Player inventory scan, counts, and weapon resolution.
//!
//! C++ reference: `player.cpp` `getItemTypeCount`, `getAllItemTypeCount`, `removeItemOfType`,
//! `getWeapon`, `getWeaponType`, `getWeaponSkill`.

use std::collections::HashMap;

use crate::container::ContainerIterator;
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::{
    InventorySlot, PLAYER_INVENTORY_SLOT_FIRST, PLAYER_INVENTORY_SLOT_LAST, WEAPON_AMMO,
    WEAPON_DISTANCE, WEAPON_NONE, WEAPON_SHIELD,
};

/// Item location for batch removal — mirrors C++ `Item*` list with known parent cylinder.
#[derive(Debug, Clone, Copy)]
pub(crate) struct InventoryItemRef {
    pub item_id: ItemId,
    pub cylinder: ItemCylinder,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ItemCylinder {
    Inventory { slot: u8 },
    Container { parent_container: ItemId },
}

impl GameWorld {
    /// Immediate parent container holding `child`, if any.
    pub(crate) fn parent_container_of(&self, child: ItemId) -> Option<ItemId> {
        self.container_registry
            .registered_container_ids()
            .find(|&id| {
                self.container_registry
                    .get(id)
                    .is_some_and(|c| c.items.contains(&child))
            })
    }

    /// TFS `Player::getItemTypeCount` — `player.cpp` ~2974–2996.
    pub fn player_get_item_type_count(
        &self,
        cid: CreatureId,
        item_id: u16,
        sub_type: i32,
    ) -> u32 {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return 0;
        };
        let mut count = 0u32;
        for slot in PLAYER_INVENTORY_SLOT_FIRST..=PLAYER_INVENTORY_SLOT_LAST {
            let idx = (slot - 1) as usize;
            let Some(slot_item) = p.equipment_slots[idx] else {
                continue;
            };
            count = count.saturating_add(self.item_count_for_type(slot_item, item_id, sub_type));
            if self
                .items
                .get(slot_item)
                .is_some_and(|i| self.items_db.is_container(i.item_type))
            {
                for child in ContainerIterator::new(&self.container_registry, slot_item) {
                    count = count.saturating_add(self.item_count_for_type(child, item_id, sub_type));
                }
            }
        }
        count
    }

    /// TFS `Player::getAllItemTypeCount` — `player.cpp` ~3049–3066.
    pub fn player_get_all_item_type_count(
        &self,
        cid: CreatureId,
        count_map: &mut HashMap<u16, u32>,
    ) {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };
        for slot in PLAYER_INVENTORY_SLOT_FIRST..=PLAYER_INVENTORY_SLOT_LAST {
            let idx = (slot - 1) as usize;
            let Some(slot_item) = p.equipment_slots[idx] else {
                continue;
            };
            self.add_item_type_to_map(count_map, slot_item);
            if self
                .items
                .get(slot_item)
                .is_some_and(|i| self.items_db.is_container(i.item_type))
            {
                for child in ContainerIterator::new(&self.container_registry, slot_item) {
                    self.add_item_type_to_map(count_map, child);
                }
            }
        }
    }

    fn add_item_type_to_map(&self, count_map: &mut HashMap<u16, u32>, iid: ItemId) {
        let Some(item) = self.items.get(iid) else {
            return;
        };
        let Some(it) = self.items_db.items.get(&item.item_type) else {
            return;
        };
        let n = item.count_by_type(it, -1);
        *count_map.entry(item.item_type).or_insert(0) = count_map
            .get(&item.item_type)
            .copied()
            .unwrap_or(0)
            .saturating_add(n);
    }

    /// Scan items matching `item_id` / `sub_type` in C++ `removeItemOfType` order.
    pub(crate) fn collect_items_of_type(
        &self,
        cid: CreatureId,
        item_id: u16,
        sub_type: i32,
        ignore_equipped: bool,
    ) -> Vec<InventoryItemRef> {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return Vec::new();
        };
        let slots = p.equipment_slots;
        let mut out = Vec::new();

        for slot in PLAYER_INVENTORY_SLOT_FIRST..=PLAYER_INVENTORY_SLOT_LAST {
            let idx = (slot - 1) as usize;
            let Some(slot_item) = slots[idx] else {
                continue;
            };
            let is_container = self
                .items
                .get(slot_item)
                .map(|i| self.items_db.is_container(i.item_type))
                .unwrap_or(false);

            if !ignore_equipped {
                if let Some(item) = self.items.get(slot_item) {
                    if item.item_type == item_id {
                        let item_count = self.item_count_for_type(slot_item, item_id, sub_type);
                        if item_count > 0 {
                            out.push(InventoryItemRef {
                                item_id: slot_item,
                                cylinder: ItemCylinder::Inventory { slot },
                            });
                        }
                    } else if is_container {
                        self.push_container_matches(slot_item, item_id, sub_type, &mut out);
                    }
                }
            } else if is_container {
                self.push_container_matches(slot_item, item_id, sub_type, &mut out);
            }
        }
        out
    }

    fn item_count_for_type(&self, iid: ItemId, item_id: u16, sub_type: i32) -> u32 {
        let Some(item) = self.items.get(iid) else {
            return 0;
        };
        if item.item_type != item_id {
            return 0;
        }
        let Some(it) = self.items_db.items.get(&item_id) else {
            return 0;
        };
        item.count_by_type(it, sub_type)
    }

    /// Sum matching counts in [`Self::collect_items_of_type`] order.
    pub(crate) fn sum_collected_item_counts(
        &self,
        entries: &[InventoryItemRef],
        item_id: u16,
        sub_type: i32,
    ) -> u32 {
        entries
            .iter()
            .map(|e| self.item_count_for_type(e.item_id, item_id, sub_type))
            .sum()
    }

    fn push_container_matches(
        &self,
        container_root: ItemId,
        item_id: u16,
        sub_type: i32,
        out: &mut Vec<InventoryItemRef>,
    ) {
        for child in ContainerIterator::new(&self.container_registry, container_root) {
            if let Some(item) = self.items.get(child) {
                if item.item_type == item_id {
                    let item_count = self.item_count_for_type(child, item_id, sub_type);
                    if item_count > 0 {
                        if let Some(parent) = self.parent_container_of(child) {
                            out.push(InventoryItemRef {
                                item_id: child,
                                cylinder: ItemCylinder::Container {
                                    parent_container: parent,
                                },
                            });
                        }
                    }
                }
            }
        }
    }

    /// TFS `Player::getWeapon(slots_t)` — `player.cpp` ~195–217.
    pub fn player_get_weapon_in_slot(
        &self,
        cid: CreatureId,
        slot: u8,
        ignore_ammo: bool,
    ) -> Option<ItemId> {
        let iid = self.get_player_inventory_item(cid, slot)?;
        let item = self.items.get(iid)?;
        let it = self.items_db.items.get(&item.item_type)?;
        let weapon_type = it.weapon_type;
        if matches!(weapon_type, WEAPON_NONE | WEAPON_SHIELD | WEAPON_AMMO) {
            return None;
        }
        if !ignore_ammo && weapon_type == WEAPON_DISTANCE && it.ammo_type != 0 {
            let ammo_slot = InventorySlot::Ammo as u8;
            let ammo_item = self.get_player_inventory_item(cid, ammo_slot)?;
            let ammo = self.items.get(ammo_item)?;
            let ammo_it = self.items_db.items.get(&ammo.item_type)?;
            if ammo_it.ammo_type != it.ammo_type {
                return None;
            }
            return Some(ammo_item);
        }
        Some(iid)
    }

    /// TFS `Player::getWeapon()` — `player.cpp` ~220–231.
    pub fn player_get_weapon(&self, cid: CreatureId, ignore_ammo: bool) -> Option<ItemId> {
        self.player_get_weapon_in_slot(cid, InventorySlot::Left as u8, ignore_ammo)
            .or_else(|| {
                self.player_get_weapon_in_slot(cid, InventorySlot::Right as u8, ignore_ammo)
            })
    }

    /// TFS `Player::getWeaponType()` — `player.cpp` ~234–240.
    pub fn player_get_weapon_type(&self, cid: CreatureId) -> u8 {
        let Some(iid) = self.player_get_weapon(cid, false) else {
            return WEAPON_NONE;
        };
        self.items
            .get(iid)
            .and_then(|i| self.items_db.items.get(&i.item_type))
            .map(|t| t.weapon_type)
            .unwrap_or(WEAPON_NONE)
    }

    /// TFS `Player::getWeaponSkill` — `player.cpp` ~243–278.
    pub fn player_get_weapon_skill(&self, cid: CreatureId, item_id: Option<ItemId>) -> i32 {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return 0;
        };
        let item_id = match item_id {
            Some(id) => id,
            None => return p.skills.fist,
        };
        let Some(item) = self.items.get(item_id) else {
            return p.skills.fist;
        };
        let Some(it) = self.items_db.items.get(&item.item_type) else {
            return 0;
        };
        match it.weapon_type {
            crate::inventory::WEAPON_SWORD => p.skills.sword,
            crate::inventory::WEAPON_CLUB => p.skills.club,
            crate::inventory::WEAPON_AXE => p.skills.axe,
            crate::inventory::WEAPON_DISTANCE => p.skills.dist,
            _ => 0,
        }
    }
}
