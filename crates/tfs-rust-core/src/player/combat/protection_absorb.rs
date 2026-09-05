//! Player equipment absorb + jewelry WearOut.
//!
//! Pack surface: TFS `Player::blockHit` absorbPercent + `transformItem` charges
//! (`tvp-772/gameserver/src/player.cpp` ~1703–1731).
//! Corpus: `TCreature::Damage` PROTECTION+CLOTHES+BODYPOSITION+WearOut (`crmain.cc:540-574`).

use tfs_rust_common::enums::CombatType;
use tfs_rust_content::item_abilities::combat_absorb_index;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};

impl GameWorld {
    /// Reduce incoming damage from equipped `absorb_percent`, then WearOut chargeable jewelry.
    /// Sequential per item (`crmain.cc` / TFS `blockHit`). No-op for non-players.
    pub(crate) fn apply_player_protection_absorb(
        &mut self,
        target: CreatureId,
        combat_type: CombatType,
        mut primary: i32,
        mut secondary: i32,
    ) -> (i32, i32) {
        let slots = match self.creatures.get(target) {
            Some(CreatureKind::Player(p)) => p.equipment_slots,
            _ => return (primary, secondary),
        };
        let absorb_idx = combat_absorb_index(combat_type);
        let mut wear: Vec<ItemId> = Vec::new();
        for slot_iid in slots.iter().flatten().copied() {
            let Some(item) = self.items.get(slot_iid) else {
                continue;
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                continue;
            };
            let pct = i32::from(it.abilities.absorb_percent[absorb_idx]);
            if pct == 0 {
                continue;
            }
            if it.charges > 0 && item.charges() == 0 {
                continue;
            }
            if primary >= 0 && secondary >= 0 {
                continue;
            }
            let factor = 100 - pct;
            primary = (primary * factor) / 100;
            secondary = (secondary * factor) / 100;
            if it.charges > 0 {
                wear.push(slot_iid);
            }
        }
        for iid in wear {
            self.player_protection_charge_wearout(target, iid);
        }
        (primary, secondary)
    }

    /// 772 WearOut `RemainingUses--` / destroy at 1 (`crmain.cc:554-564`).
    /// Uses the charges attribute, not stack `count` (weapons use count in strike wearout).
    fn player_protection_charge_wearout(&mut self, cid: CreatureId, iid: ItemId) {
        let charges = self.items.get(iid).map(|i| i.charges()).unwrap_or(0);
        if charges == 0 {
            return;
        }
        let slot = self.equipment_slot_for_item(cid, iid);
        let destroy = charges <= 1;
        if !destroy && let Some(item) = self.items.get_mut(iid) {
            item.set_charges(charges - 1);
        }
        let Some(slot) = slot else {
            return;
        };
        if destroy {
            let _ = self.internal_remove_item_from_inventory_slot(cid, slot, iid);
            self.items.remove(iid);
            self.broadcast_player_inventory_slot(cid, slot, None);
        } else {
            self.broadcast_player_inventory_slot(cid, slot, Some(iid));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{CombatDamage, CombatParams};
    use crate::inventory::InventorySlot;
    use crate::sim_harness::{insert_player, minimal_world, test_player};
    use tfs_rust_common::Position;
    use tfs_rust_content::item_abilities::combat_absorb_index;
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

    #[test]
    fn might_ring_absorbs_physical_and_wears_a_charge() {
        let mut world = minimal_world();
        let cid = insert_player(&mut world, test_player("Might", Position::new(100, 100, 7)));
        let mut it = ItemType {
            id: 2164,
            charges: 20,
            ..Default::default()
        };
        it.abilities.absorb_percent[combat_absorb_index(CombatType::Physical)] = 20;
        register_type(&mut world, 2164, it);
        let typed = world.items_db.items.get(&2164).expect("type");
        let iid = world
            .items
            .insert(crate::item::Item::from_item_type(typed, 1));
        let slot = InventorySlot::Ring as u8;
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            p.equipment_slots[crate::inventory::slot_to_array_index(slot).unwrap()] = Some(iid);
            p.base.health = 200;
            p.base.max_health = 200;
        }
        assert_eq!(world.items.get(iid).map(|i| i.charges()), Some(20));

        let applied = world.combat_execute_with_stimulus(
            None,
            cid,
            &CombatDamage {
                primary: (CombatType::Physical, -100),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams::default(),
        );
        assert_eq!(applied, 80, "20% absorb: 100 → 80");
        assert_eq!(world.items.get(iid).map(|i| i.charges()), Some(19));
        assert!(
            world.creatures.get(cid).is_some_and(
                |k| matches!(k, CreatureKind::Player(p) if p.equipment_slots[8] == Some(iid))
            ),
            "ring still equipped"
        );
    }

    #[test]
    fn last_protection_charge_destroys_jewelry() {
        let mut world = minimal_world();
        let cid = insert_player(&mut world, test_player("Ssa", Position::new(100, 100, 7)));
        let mut it = ItemType {
            id: 2197,
            charges: 5,
            ..Default::default()
        };
        it.abilities.absorb_percent[combat_absorb_index(CombatType::Physical)] = 80;
        register_type(&mut world, 2197, it);
        let mut item = crate::item::Item::new_single(2197);
        item.set_charges(1);
        let iid = world.items.insert(item);
        let slot = InventorySlot::Necklace as u8;
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            p.equipment_slots[crate::inventory::slot_to_array_index(slot).unwrap()] = Some(iid);
            p.base.health = 200;
            p.base.max_health = 200;
        }

        let _ = world.combat_execute_with_stimulus(
            None,
            cid,
            &CombatDamage {
                primary: (CombatType::Physical, -50),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams::default(),
        );
        assert!(world.items.get(iid).is_none(), "last charge destroys");
        assert!(
            world.creatures.get(cid).is_some_and(
                |k| matches!(k, CreatureKind::Player(p) if p.equipment_slots[1].is_none())
            ),
            "necklace slot empty"
        );
    }
}
