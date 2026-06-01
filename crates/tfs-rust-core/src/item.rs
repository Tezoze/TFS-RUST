//! World `Item` node with full attribute system.
// C++ reference: `src/item.h` (Item class)

use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::otb::ItemType;
use tfs_rust_db::ItemRecord;

use crate::ids::ItemId;
use crate::item_attributes::{DecayState, ItemAttributes};

/// Position of an item in the world (Tile, Container, or Player inventory)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemPosition {
    /// Item is on a tile at the given position
    Tile { x: u16, y: u16, z: u8 },
    /// Item is inside a container
    Container { container_id: ItemId, slot: usize },
    /// Item is in a player's inventory slot
    Inventory { player_id: u32, slot: u8 },
}

/// Runtime item instance with unique ID and attributes
// C++ ref: `src/item.h` Item class (composition of ItemAttributes)
#[derive(Debug, Clone)]
pub struct Item {
    /// Item type ID (from items.xml/OTB - the client-visible sprite ID)
    pub item_type: u16,
    /// Stack count (for stackable items like gold coins, runes)
    pub count: u16,
    /// Full attribute system (actionId, uniqueId, text, duration, etc.)
    /// Heap-allocated only if the item possesses specific attributes.
    pub attributes: Option<Box<ItemAttributes>>,
}

impl Item {
    /// Create a new item with the given type and count
    pub fn new(item_type: u16, count: u16) -> Self {
        Self {
            item_type,
            count,
            attributes: None,
        }
    }

    /// Create a new item with default count of 1
    pub fn new_single(item_type: u16) -> Self {
        Self::new(item_type, 1)
    }

    // === Convenience accessors for common attributes ===

    pub fn action_id(&self) -> u16 {
        self.attributes.as_deref().map(|a| a.get_action_id()).unwrap_or(0)
    }

    pub fn set_action_id(&mut self, value: u16) {
        self.attributes.get_or_insert_with(|| Box::new(ItemAttributes::new())).set_action_id(value);
    }

    pub fn unique_id(&self) -> u16 {
        self.attributes.as_deref().map(|a| a.get_unique_id()).unwrap_or(0)
    }

    pub fn set_unique_id(&mut self, value: u16) {
        self.attributes.get_or_insert_with(|| Box::new(ItemAttributes::new())).set_unique_id(value);
    }

    pub fn text(&self) -> &str {
        self.attributes.as_deref().map(|a| a.get_text()).unwrap_or("")
    }

    pub fn set_text(&mut self, value: impl Into<String>) {
        self.attributes.get_or_insert_with(|| Box::new(ItemAttributes::new())).set_text(value);
    }

    pub fn description(&self) -> &str {
        self.attributes.as_deref().map(|a| a.get_description()).unwrap_or("")
    }

    pub fn set_description(&mut self, value: impl Into<String>) {
        self.attributes.get_or_insert_with(|| Box::new(ItemAttributes::new())).set_description(value);
    }

    pub fn charges(&self) -> u16 {
        self.attributes.as_deref().map(|a| a.get_charges()).unwrap_or(0)
    }

    pub fn set_charges(&mut self, value: u16) {
        self.attributes.get_or_insert_with(|| Box::new(ItemAttributes::new())).set_charges(value);
    }

    pub fn duration(&self) -> i32 {
        self.attributes.as_deref().map(|a| a.get_duration()).unwrap_or(0)
    }

    pub fn set_duration(&mut self, value: i32) {
        self.attributes.get_or_insert_with(|| Box::new(ItemAttributes::new())).set_duration(value);
    }

    pub fn decaying(&self) -> DecayState {
        self.attributes.as_deref().map(|a| a.get_decaying()).unwrap_or(DecayState::False)
    }

    pub fn set_decaying(&mut self, state: DecayState) {
        self.attributes.get_or_insert_with(|| Box::new(ItemAttributes::new())).set_decaying(state);
    }

    pub fn fluid_type(&self) -> u16 {
        self.attributes.as_deref().map(|a| a.get_fluid_type()).unwrap_or(0)
    }

    pub fn set_fluid_type(&mut self, value: u16) {
        self.attributes.get_or_insert_with(|| Box::new(ItemAttributes::new())).set_fluid_type(value);
    }

    pub fn depot_id(&self) -> u16 {
        self.attributes.as_deref().map(|a| a.get_depot_id()).unwrap_or(0)
    }

    pub fn set_depot_id(&mut self, value: u16) {
        self.attributes.get_or_insert_with(|| Box::new(ItemAttributes::new())).set_depot_id(value);
    }

    /// Check if this item has a specific attribute set
    pub fn has_attribute(&self, flag: crate::item_attributes::ItemAttrFlags) -> bool {
        self.attributes.as_deref().is_some_and(|a| {
            crate::item_attributes::ItemAttrFlags::from_bits_truncate(a.attribute_bits()).contains(flag)
        })
    }

    /// Get the client-visible count (for stackable items)
    pub fn client_count(&self) -> u8 {
        // Client expects count as u8, capped at 255
        self.count.min(255) as u8
    }

    pub fn total_weight_oz(&self, type_weight: u32, stackable: bool) -> u32 {
        let w = self.attributes.as_deref().map(|a| a.base_weight_oz(type_weight)).unwrap_or(type_weight);
        if stackable {
            w.saturating_mul(self.count.max(1) as u32)
        } else {
            w
        }
    }

    /// Load from DB row (`IOLoginData::loadItems` — `src/iologindata.cpp`).
    pub fn from_player_item_record(
        _id: ItemId, // Kept for API compatibility, though no longer stored in Item
        rec: &ItemRecord,
        items_db: &ItemDatabase,
    ) -> tfs_rust_common::Result<Self> {
        let count = rec.count.clamp(0, 10000) as u16;
        let mut item = Item::new(rec.itemtype, count);
        let is_container = items_db.is_container(rec.itemtype);
        if !rec.attributes.is_empty() {
            match crate::item_blob::parse_item_blob(&rec.attributes, is_container) {
                Ok(p) => {
                    item.attributes = Some(Box::new(p.attrs));
                    if let Some(st) = p.subtype_override {
                        item.count = u16::from(st).max(1);
                    }
                }
                Err(e) => {
                    tracing::warn!(?e, itemtype = rec.itemtype, "item blob parse failed");
                }
            }
        }
        Ok(item)
    }

    /// Check if this item is stackable based on count > 1
    pub fn is_stack(&self) -> bool {
        self.count > 1
    }

    #[inline]
    pub fn is_store_item(&self) -> bool {
        self.attributes.as_deref().is_some_and(|a| a.is_store_item())
    }

    /// TFS `Item::getSubType` — `item.cpp` ~341–352.
    pub fn get_sub_type(&self, it: &ItemType) -> u16 {
        if it.is_fluid_container() || it.is_splash() {
            self.fluid_type()
        } else if it.stackable() {
            self.count
        } else if it.charges != 0 {
            self.charges()
        } else {
            self.count
        }
    }

    /// TFS `Item::countByType` — `item.h` ~981–987.
    pub fn count_by_type(&self, it: &ItemType, sub_type: i32) -> u32 {
        if sub_type == -1 || sub_type == i32::from(self.get_sub_type(it)) {
            u32::from(self.count.max(1))
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_creation() {
        let item = Item::new_single(100);
        assert_eq!(item.item_type, 100);
        assert_eq!(item.count, 1);
    }

    #[test]
    fn test_item_with_count() {
        let item = Item::new(100, 100);
        assert_eq!(item.count, 100);
        assert_eq!(item.client_count(), 100);
    }

    #[test]
    fn test_item_attributes() {
        let mut item = Item::new_single(100);

        item.set_action_id(123);
        assert_eq!(item.action_id(), 123);

        item.set_unique_id(456);
        assert_eq!(item.unique_id(), 456);

        item.set_text("Hello");
        assert_eq!(item.text(), "Hello");

        item.set_charges(10);
        assert_eq!(item.charges(), 10);
    }

    #[test]
    fn test_item_stack() {
        let single = Item::new_single(100);
        assert!(!single.is_stack());

        let stack = Item::new(100, 5);
        assert!(stack.is_stack());
    }

    #[test]
    fn count_by_type_matches_sub_type() {
        let it = ItemType::default();
        let item = Item::new(100, 42);
        assert_eq!(item.count_by_type(&it, 42), 42);
        assert_eq!(item.count_by_type(&it, 41), 0);
        assert_eq!(item.count_by_type(&it, -1), 42);
    }
}
