//! Corpus `MOVEMENTEVENT` after cylinder transfer (`moveuse.cc:2263-2287`).
//!
//! `moveuse.dat` Movement rules (not TFS tile `MoveEvent`): lit candelabrum,
//! armed trap, dress-toggle rings. OTB has no flag bit — match known type ids.

use tfs_rust_common::Position;

use crate::cylinder::Cylinder;
use crate::game_world::GameWorld;
use crate::ids::ItemId;

const ITEM_CANDELABRUM_LIT: u16 = 2042;
const ITEM_CANDELABRUM: u16 = 2041;
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
        if ty == ITEM_CANDELABRUM_LIT {
            self.change_item_type(item_id, ITEM_CANDELABRUM);
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
