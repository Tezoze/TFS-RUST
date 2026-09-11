//! Corpus `MOVEMENTEVENT` after cylinder transfer (`moveuse.cc:2263-2287`).
//!
//! `moveuse.dat` Movement rules (not TFS tile `MoveEvent`): eternal lit
//! candelabrum 2057→2042, armed trap, dress-toggle rings. OTB has no flag bit
//! — match known type ids. Normal lit 2042 is ChangeUse only (stays lit on move).

use tfs_rust_common::Position;

use crate::cylinder::Cylinder;
use crate::game_world::GameWorld;
use crate::ids::ItemId;

/// Eternal (non-expiring) lit candelabrum — pack id for corpus 2927.
const ITEM_CANDELABRUM_ETERNAL: u16 = 2057;
/// Expiring lit candelabrum — pack id for corpus 2912 (`Change(Obj1,2912)`).
const ITEM_CANDELABRUM_LIT: u16 = 2042;
const ITEM_TRAP_ARMED: u16 = 2579;
const ITEM_TRAP: u16 = 2578;
const CONST_ME_POFF: u8 = 3;

/// Inactive → active (dressed) and reverse for 772 ring pairs (OTB).
const RING_DRESS: &[(u16, u16)] = &[
    (2165, 2202), // stealth
    (2166, 2203), // power
    (2167, 2204), // energy
    (2168, 2205), // life
    (2169, 2206), // time
    (2207, 2210), // sword
    (2208, 2211), // axe
    (2209, 2212), // club
];

impl GameWorld {
    /// After a successful Move/Create onto a container or tile cylinder.
    pub(crate) fn fire_movement_event(&mut self, item_id: ItemId) {
        let Some(ty) = self.items.get(item_id).map(|i| i.item_type) else {
            return;
        };
        // Corpus `moveuse.dat` Movement: eternal lit → expiring lit (stays lit).
        // Normal lit 2042 has ChangeUse only — Use toggles to 2041, Move does not.
        if ty == ITEM_CANDELABRUM_ETERNAL {
            self.change_item_type(item_id, ITEM_CANDELABRUM_LIT);
            return;
        }
        if ty == ITEM_TRAP_ARMED {
            let pos = self.movement_event_pos(item_id);
            self.change_item_type(item_id, ITEM_TRAP);
            if let Some(pos) = pos {
                self.broadcast_magic_effect(pos, CONST_ME_POFF);
            }
            return;
        }
        let dressed = self.item_is_dressed(item_id);
        for &(inactive, active) in RING_DRESS {
            if ty == inactive && dressed {
                self.change_item_type(item_id, active);
                return;
            }
            if ty == active && !dressed {
                self.change_item_type(item_id, inactive);
                return;
            }
        }
    }

    fn movement_event_pos(&self, item_id: ItemId) -> Option<Position> {
        match self.items.get(item_id)?.parent {
            Some(Cylinder::Tile { pos }) => Some(pos),
            _ => self
                .creatures
                .get(self.item_holding_player(item_id)?)
                .map(|k| k.position()),
        }
    }

    fn item_holding_player(&self, item_id: ItemId) -> Option<crate::ids::CreatureId> {
        match self.items.get(item_id)?.parent {
            Some(Cylinder::Inventory { player_id, .. }) => Some(player_id),
            Some(Cylinder::Container { item_id: cid, .. }) => self.item_holding_player(cid),
            _ => None,
        }
    }

    fn item_is_dressed(&self, item_id: ItemId) -> bool {
        matches!(
            self.items.get(item_id).and_then(|i| i.parent),
            Some(Cylinder::Inventory { .. })
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cylinder::{Cylinder, CylinderFlags};
    use crate::item::Item;
    use crate::sim_harness::{ensure_walkable_tile, minimal_world};
    use std::sync::Arc;
    use tfs_rust_common::Position;
    use tfs_rust_content::otb::ItemType;

    fn register_types(world: &mut GameWorld, ids: &[u16]) {
        let db = Arc::make_mut(&mut world.items_db);
        for &id in ids {
            db.items.entry(id).or_insert_with(ItemType::default);
        }
    }

    /// Regression: moving a lit candelabrum on the ground must not unlight it.
    /// `moveuse.dat` Movement is 2057→2042, not 2042→2041 (that is ChangeUse).
    #[test]
    fn lit_candelabrum_stays_lit_when_moved_on_ground() {
        let mut world = minimal_world();
        register_types(&mut world, &[2041, 2042, 2057]);
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, from, 100);
        ensure_walkable_tile(&mut world.map, to, 100);

        let iid = world.items.insert(Item::new_single(2042));
        world
            .internal_add_item_to_tile(from, iid, CylinderFlags::NONE)
            .expect("place lit candelabrum");
        assert_eq!(world.items.get(iid).map(|i| i.item_type), Some(2042));

        world
            .internal_move_item(
                None,
                Cylinder::Tile { pos: from },
                Cylinder::Tile { pos: to },
                iid,
                1,
                CylinderFlags::NONE,
                None,
            )
            .expect("move on ground");
        assert_eq!(
            world.items.get(iid).map(|i| i.item_type),
            Some(2042),
            "expiring lit candelabrum stays lit on move"
        );
    }

    #[test]
    fn eternal_candelabrum_becomes_expiring_on_tile_add() {
        let mut world = minimal_world();
        register_types(&mut world, &[2041, 2042, 2057]);
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let iid = world.items.insert(Item::new_single(2057));
        world
            .internal_add_item_to_tile(pos, iid, CylinderFlags::NONE)
            .expect("place eternal candelabrum");
        assert_eq!(world.items.get(iid).map(|i| i.item_type), Some(2042));
    }
}
