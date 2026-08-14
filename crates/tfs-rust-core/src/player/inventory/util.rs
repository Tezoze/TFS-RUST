//! Player inventory scan, counts, and weapon resolution.
//!
//! C++ reference: `player.cpp` `getItemTypeCount`, `getAllItemTypeCount`, `removeItemOfType`,
//! `getWeapon`, `getWeaponType`, `getWeaponSkill`.
//! 772 outcomes: `TCombat::GetWeapon` / `GetAmmo` — `crcombat.cc:36-126` (`CombatWeapons` snapshot).

use std::collections::HashMap;

use crate::container::ContainerIterator;
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::{
    InventorySlot, PLAYER_INVENTORY_SLOT_FIRST, PLAYER_INVENTORY_SLOT_LAST, WEAPON_AMMO,
    WEAPON_DISTANCE, WEAPON_NONE, WEAPON_SHIELD,
};
use crate::player::combat::values::{CombatWeapons, HandWeapon, classify_weapon};
use tfs_rust_content::otb::ItemType;

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
    pub fn player_get_item_type_count(&self, cid: CreatureId, item_id: u16, sub_type: i32) -> u32 {
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
                    count =
                        count.saturating_add(self.item_count_for_type(child, item_id, sub_type));
                }
            }
        }
        count
    }

    /// 772 `CountInventoryObjects` parity for NPC `Count(...)` predicates.
    ///
    /// Matches `CountObjects(Creature->CrObject, Type, Value)` (`info.cc:579-588`).
    /// `Value` is only checked for liquid containers (`CONTAINERLIQUIDTYPE`) and keys
    /// (`KEYNUMBER`); other item types match by `item_id` alone. C++ does **not**
    /// propagate `Value` into nested containers — it passes `0` — so children are
    /// checked against `0` here (`info.cc:553`, BUG(fusion)).
    pub fn player_get_item_type_count_npc(
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
            count =
                count.saturating_add(self.item_count_for_type_npc(slot_item, item_id, sub_type));
            if self
                .items
                .get(slot_item)
                .is_some_and(|i| self.items_db.is_container(i.item_type))
            {
                for child in ContainerIterator::new(&self.container_registry, slot_item) {
                    // C++ BUG(fusion): nested containers are checked with Value = 0.
                    count = count.saturating_add(self.item_count_for_type_npc(child, item_id, 0));
                }
            }
        }
        count
    }

    fn item_count_for_type_npc(&self, iid: ItemId, item_id: u16, sub_type: i32) -> u32 {
        let Some(item) = self.items.get(iid) else {
            return 0;
        };
        if item.item_type != item_id {
            return 0;
        }
        let Some(it) = self.items_db.items.get(&item_id) else {
            return 0;
        };
        if it.is_fluid_container() || it.is_key() {
            // C++ `CountObjects`: liquid container type must equal Value; key number
            // (`KEYNUMBER`, stored as `action_id`) must equal Value.
            let actual = if it.is_key() {
                i32::from(item.action_id())
            } else {
                i32::from(item.get_sub_type(it))
            };
            if sub_type == -1 || sub_type == actual {
                u32::from(item.count.max(1))
            } else {
                0
            }
        } else {
            u32::from(item.count.max(1))
        }
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

    /// TFS `Game::findItemOfType` on player cylinder — `game.cpp` ~1442–1487.
    ///
    /// Scans equipment slots `PLAYER_INVENTORY_SLOT_FIRST..=LAST` (store inbox excluded,
    /// matching C++ `Player::getLastIndex()` = `CONST_SLOT_LAST + 1`).
    pub fn find_item_of_type(
        &self,
        cid: CreatureId,
        item_id: u16,
        depth_search: bool,
        sub_type: i32,
    ) -> Option<ItemId> {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return None;
        };
        let mut pending_containers: Vec<ItemId> = Vec::new();

        for slot in PLAYER_INVENTORY_SLOT_FIRST..=PLAYER_INVENTORY_SLOT_LAST {
            let idx = (slot - 1) as usize;
            let Some(slot_item) = p.equipment_slots[idx] else {
                continue;
            };
            if self.item_matches_find_type(slot_item, item_id, sub_type) {
                return Some(slot_item);
            }
            if depth_search
                && self
                    .items
                    .get(slot_item)
                    .is_some_and(|i| self.items_db.is_container(i.item_type))
            {
                pending_containers.push(slot_item);
            }
        }

        let mut i = 0usize;
        while i < pending_containers.len() {
            let root = pending_containers[i];
            i += 1;
            if let Some(cont) = self.container_registry.get(root) {
                for &child in &cont.items {
                    if self.item_matches_find_type(child, item_id, sub_type) {
                        return Some(child);
                    }
                    if self
                        .items
                        .get(child)
                        .is_some_and(|it| self.items_db.is_container(it.item_type))
                    {
                        pending_containers.push(child);
                    }
                }
            }
        }
        None
    }

    fn item_matches_find_type(&self, iid: ItemId, item_id: u16, sub_type: i32) -> bool {
        let Some(item) = self.items.get(iid) else {
            return false;
        };
        if item.item_type != item_id {
            return false;
        }
        if sub_type == -1 {
            return true;
        }
        let Some(it) = self.items_db.items.get(&item_id) else {
            return false;
        };
        i32::from(item.get_sub_type(it)) == sub_type
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

    /// Whether this hand-slot item passes 772 `RESTRICTLEVEL` / `RESTRICTPROFESSION`
    /// (`crcombat.cc:62-76`). Wands/rods keep Lua `WandDef` for strike content; item-flag
    /// gates still skip underleveled / wrong-voc gear from the combat snapshot.
    fn combat_weapon_passes_restrict_gates(&self, cid: CreatureId, it: &ItemType) -> bool {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return false;
        };
        if it.min_req_level > 0 && (p.level as u32) < it.min_req_level {
            return false;
        }
        if !it.voc_equip_names.is_empty() {
            let Some(voc) = self.vocations.get(p.vocation_id) else {
                return false;
            };
            let name = voc.name.to_ascii_lowercase();
            if !it
                .voc_equip_names
                .iter()
                .any(|n| n.eq_ignore_ascii_case(&name))
            {
                return false;
            }
        }
        true
    }

    /// 772 `TCombat::GetWeapon` — `crcombat.cc:36-102`.
    ///
    /// Walks hand slots in ascending index order (Right=5, Left=6). Later slots overwrite
    /// the same category. Non-exclusive: multi-flag items populate several fields.
    pub(crate) fn player_get_combat_weapons(&self, cid: CreatureId) -> CombatWeapons {
        let mut w = CombatWeapons::default();
        for slot in [InventorySlot::Right as u8, InventorySlot::Left as u8] {
            let Some(iid) = self.get_player_inventory_item(cid, slot) else {
                continue;
            };
            let Some(item) = self.items.get(iid) else {
                continue;
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                continue;
            };
            if !self.combat_weapon_passes_restrict_gates(cid, it) {
                continue;
            }
            match classify_weapon(it.weapon_type, it.ammo_type) {
                Some(HandWeapon::Shield) => {
                    w.shield = Some(iid);
                }
                Some(HandWeapon::Close) => {
                    w.close = Some(iid);
                    w.fist = false;
                }
                Some(HandWeapon::Missile) => {
                    w.missile = Some(iid);
                    w.fist = false;
                }
                Some(HandWeapon::Throw) => {
                    w.throw_ = Some(iid);
                    w.fist = false;
                }
                Some(HandWeapon::Wand) => {
                    // 772 `GetWeapon` skips a wand whose Lua `WandDef` level/vocation
                    // gates fail (`crcombat.cc:36-102` continue). The gate lives on
                    // `WandDef` / `player_meets_wand_requirements` — not item XML.
                    if !self.player_meets_wand_requirements(cid, item.item_type) {
                        continue;
                    }
                    w.wand = Some(iid);
                    w.fist = false;
                }
                None => {}
            }
        }
        w
    }

    /// 772 `TCombat::GetAmmo` — `crcombat.cc:104-126`.
    pub(crate) fn player_get_ammo(&self, cid: CreatureId, weapons: &mut CombatWeapons) {
        if weapons.missile.is_none() {
            if weapons.throw_.is_some() {
                weapons.ammo = weapons.throw_;
            } else if weapons.wand.is_some() {
                weapons.ammo = weapons.wand;
            }
            return;
        }
        weapons.ammo = None;
        let Some(bow_iid) = weapons.missile else {
            return;
        };
        let Some(bow) = self.items.get(bow_iid) else {
            return;
        };
        let Some(bow_it) = self.items_db.items.get(&bow.item_type) else {
            return;
        };
        let Some(ammo_iid) = self.get_player_inventory_item(cid, InventorySlot::Ammo as u8) else {
            return;
        };
        let Some(ammo) = self.items.get(ammo_iid) else {
            return;
        };
        let Some(ammo_it) = self.items_db.items.get(&ammo.item_type) else {
            return;
        };
        if ammo_it.weapon_type == WEAPON_AMMO && ammo_it.ammo_type == bow_it.ammo_type {
            weapons.ammo = Some(ammo_iid);
        }
    }

    /// `GetWeapon` + `GetAmmo` snapshot.
    pub(crate) fn player_resolve_combat_weapons(&self, cid: CreatureId) -> CombatWeapons {
        let mut w = self.player_get_combat_weapons(cid);
        self.player_get_ammo(cid, &mut w);
        w
    }

    /// TFS `Player::getWeapon(slots_t)` — `player.cpp` ~195–217.
    /// Kept for non-combat TFS callers; combat paths use [`Self::player_resolve_combat_weapons`].
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

    /// Returns the equipped shield `ItemId` — 772 `TCombat::Shield` via `GetWeapon`.
    /// C++ `crcombat.cc:265`. Used by shield wearout (M11, `crcombat.cc:265-281`).
    /// Last hand slot wins (ascending overwrite).
    pub fn player_get_shield(&self, cid: CreatureId) -> Option<ItemId> {
        self.player_get_combat_weapons(cid).shield
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
        use crate::player::combat::SkillNr;
        let item_id = match item_id {
            Some(id) => id,
            None => return p.skill_level(SkillNr::Fist),
        };
        let Some(item) = self.items.get(item_id) else {
            return p.skill_level(SkillNr::Fist);
        };
        let Some(it) = self.items_db.items.get(&item.item_type) else {
            return 0;
        };
        match it.weapon_type {
            crate::inventory::WEAPON_SWORD => p.skill_level(SkillNr::Sword),
            crate::inventory::WEAPON_CLUB => p.skill_level(SkillNr::Club),
            crate::inventory::WEAPON_AXE => p.skill_level(SkillNr::Axe),
            crate::inventory::WEAPON_DISTANCE => p.skill_level(SkillNr::Distance),
            _ => 0,
        }
    }
}

#[cfg(test)]
mod find_item_of_type_tests {
    use super::*;
    use crate::container::Container;
    use crate::creature::CreatureKind;
    use crate::item::Item;
    use crate::test_world::support::{minimal_world, test_player};
    use tfs_rust_common::Position;

    fn insert_player_with_backpack(world: &mut GameWorld, gold_type: u16) -> CreatureId {
        let cid = world.creatures.insert(CreatureKind::Player(test_player(
            "FindTest",
            Position::new(100, 100, 7),
        )));
        let bp = world.items.insert(Item::new_single(1987));
        let gold = world.items.insert(Item::new(gold_type, 10));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            p.equipment_slots[2] = Some(bp);
        }
        let mut reg = std::mem::take(&mut world.container_registry);
        reg.register(Container::new(bp, 20));
        reg.get_mut(bp).unwrap().add_item(gold).expect("add gold");
        world.container_registry = reg;
        cid
    }

    #[test]
    fn find_item_in_backpack() {
        let mut world = minimal_world();
        let cid = insert_player_with_backpack(&mut world, 2148);
        assert!(world.find_item_of_type(cid, 2148, true, -1).is_some());
        assert!(world.find_item_of_type(cid, 2148, false, -1).is_none());
    }
}
