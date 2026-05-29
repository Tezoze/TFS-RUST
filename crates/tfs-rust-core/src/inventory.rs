//! Player equipment slots and `Game::getSlotType` / `slots_t` mapping.
// C++ reference: `src/creature.h` `slots_t`, `src/game.cpp` `getSlotType`, `src/items.h` `SlotPositionBits`.

use tfs_rust_content::otb::ItemType;

/// TFS `slots_t` / `CONST_SLOT_*` — wire and array index use 1..=11.
// C++ ref: `src/creature.h` lines 18–34
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventorySlot {
    Wherever = 0,
    Head = 1,
    Necklace = 2,
    Backpack = 3,
    Armor = 4,
    Right = 5,
    Left = 6,
    Legs = 7,
    Feet = 8,
    Ring = 9,
    Ammo = 10,
    StoreInbox = 11,
}

/// `SlotPositionBits` — `src/items.h`
pub const SLOTP_HEAD: u32 = 1 << 0;
pub const SLOTP_NECKLACE: u32 = 1 << 1;
pub const SLOTP_BACKPACK: u32 = 1 << 2;
pub const SLOTP_ARMOR: u32 = 1 << 3;
pub const SLOTP_RIGHT: u32 = 1 << 4;
pub const SLOTP_LEFT: u32 = 1 << 5;
pub const SLOTP_LEGS: u32 = 1 << 6;
pub const SLOTP_FEET: u32 = 1 << 7;
pub const SLOTP_RING: u32 = 1 << 8;
pub const SLOTP_AMMO: u32 = 1 << 9;
pub const SLOTP_TWO_HAND: u32 = 1 << 11;
pub const SLOTP_HAND: u32 = SLOTP_LEFT | SLOTP_RIGHT;

/// C++ `WeaponType_t` — `src/const.h`
pub const WEAPON_NONE: u8 = 0;
pub const WEAPON_SHIELD: u8 = 4;

/// Map `pid` / `CONST_SLOT_*` to array index `0..=10` for `[Option<ItemId>; 11]`.
#[inline]
pub fn slot_to_array_index(slot: u8) -> Option<usize> {
    if (1..=11).contains(&slot) {
        Some((slot - 1) as usize)
    } else {
        None
    }
}

/// Subset of `Player::queryAdd` per-slot mask — `player.cpp` ~2440–2583.
/// Hand slots (5/6) accept `SLOTP_RIGHT` / `SLOTP_LEFT` or `SLOTP_TWO_HAND` (two-handed weapons).
pub fn item_fits_equipment_slot(slot: u8, it: &ItemType) -> bool {
    let sp = it.slot_position;
    match slot {
        1 => sp & SLOTP_HEAD != 0,
        2 => sp & SLOTP_NECKLACE != 0,
        3 => sp & SLOTP_BACKPACK != 0,
        4 => sp & SLOTP_ARMOR != 0,
        5 => (sp & SLOTP_RIGHT != 0) || (sp & SLOTP_TWO_HAND != 0),
        6 => (sp & SLOTP_LEFT != 0) || (sp & SLOTP_TWO_HAND != 0),
        7 => sp & SLOTP_LEGS != 0,
        8 => sp & SLOTP_FEET != 0,
        9 => sp & SLOTP_RING != 0,
        10 => sp & SLOTP_AMMO != 0,
        11 => true, // store inbox — full rules deferred
        _ => false,
    }
}

/// `Game::getSlotType` — `src/game.cpp` ~1822–1847
pub fn slot_type_for_item_type(it: &ItemType) -> u8 {
    if it.weapon_type == WEAPON_SHIELD {
        return InventorySlot::Right as u8;
    }
    let sp = it.slot_position;
    if sp & SLOTP_HEAD != 0 {
        return InventorySlot::Head as u8;
    }
    if sp & SLOTP_NECKLACE != 0 {
        return InventorySlot::Necklace as u8;
    }
    if sp & SLOTP_ARMOR != 0 {
        return InventorySlot::Armor as u8;
    }
    if sp & SLOTP_LEGS != 0 {
        return InventorySlot::Legs as u8;
    }
    if sp & SLOTP_FEET != 0 {
        return InventorySlot::Feet as u8;
    }
    if sp & SLOTP_RING != 0 {
        return InventorySlot::Ring as u8;
    }
    if sp & SLOTP_AMMO != 0 {
        return InventorySlot::Ammo as u8;
    }
    if sp & SLOTP_TWO_HAND != 0 || sp & SLOTP_LEFT != 0 {
        return InventorySlot::Left as u8;
    }
    InventorySlot::Right as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_armor_slot_masks() {
        let mut it = ItemType::default();
        it.slot_position = SLOTP_HEAD;
        assert!(item_fits_equipment_slot(1, &it));
        assert!(!item_fits_equipment_slot(5, &it));
        assert!(!item_fits_equipment_slot(6, &it));
    }

    #[test]
    fn hand_default_fits_left_right() {
        let it = ItemType::default();
        assert!(item_fits_equipment_slot(5, &it));
        assert!(item_fits_equipment_slot(6, &it));
    }
}
