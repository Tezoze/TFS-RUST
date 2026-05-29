//! World `Item` node with full attribute system.
// C++ reference: `src/item.h` (Item class)

use tfs_rust_content::items::ItemDatabase;
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
    /// Unique runtime ID for this item instance
    pub id: ItemId,
    /// Item type ID (from items.xml/OTB - the client-visible sprite ID)
    pub item_type: u16,
    /// Stack count (for stackable items like gold coins, runes)
    pub count: u16,
    /// Full attribute system (actionId, uniqueId, text, duration, etc.)
    pub attributes: ItemAttributes,
}

impl Item {
    /// Create a new item with the given type and count
    pub fn new(id: ItemId, item_type: u16, count: u16) -> Self {
        Self {
            id,
            item_type,
            count,
            attributes: ItemAttributes::new(),
        }
    }

    /// Create a new item with default count of 1
    pub fn new_single(id: ItemId, item_type: u16) -> Self {
        Self::new(id, item_type, 1)
    }

    // === Convenience accessors for common attributes ===

    pub fn action_id(&self) -> u16 {
        self.attributes.get_action_id()
    }

    pub fn set_action_id(&mut self, value: u16) {
        self.attributes.set_action_id(value);
    }

    pub fn unique_id(&self) -> u16 {
        self.attributes.get_unique_id()
    }

    pub fn set_unique_id(&mut self, value: u16) {
        self.attributes.set_unique_id(value);
    }

    pub fn text(&self) -> &str {
        self.attributes.get_text()
    }

    pub fn set_text(&mut self, value: impl Into<String>) {
        self.attributes.set_text(value);
    }

    pub fn description(&self) -> &str {
        self.attributes.get_description()
    }

    pub fn set_description(&mut self, value: impl Into<String>) {
        self.attributes.set_description(value);
    }

    pub fn charges(&self) -> u16 {
        self.attributes.get_charges()
    }

    pub fn set_charges(&mut self, value: u16) {
        self.attributes.set_charges(value);
    }

    pub fn duration(&self) -> i32 {
        self.attributes.get_duration()
    }

    pub fn set_duration(&mut self, value: i32) {
        self.attributes.set_duration(value);
    }

    pub fn decaying(&self) -> DecayState {
        self.attributes.get_decaying()
    }

    pub fn set_decaying(&mut self, state: DecayState) {
        self.attributes.set_decaying(state);
    }

    pub fn fluid_type(&self) -> u16 {
        self.attributes.get_fluid_type()
    }

    pub fn set_fluid_type(&mut self, value: u16) {
        self.attributes.set_fluid_type(value);
    }

    /// Check if this item has a specific attribute set
    pub fn has_attribute(&self, flag: crate::item_attributes::ItemAttrFlags) -> bool {
        crate::item_attributes::ItemAttrFlags::from_bits_truncate(self.attributes.attribute_bits())
            .contains(flag)
    }

    /// Get the client-visible count (for stackable items)
    pub fn client_count(&self) -> u8 {
        // Client expects count as u8, capped at 255
        self.count.min(255) as u8
    }

    /// `Item::getWeight` — `src/item.cpp` (~930–936); `type_weight` is OTB `ItemType::weight` (1/100 oz).
    pub fn total_weight_oz(&self, type_weight: u32, stackable: bool) -> u32 {
        let w = self.attributes.base_weight_oz(type_weight);
        if stackable {
            w.saturating_mul(self.count.max(1) as u32)
        } else {
            w
        }
    }

    /// Load from DB row (`IOLoginData::loadItems` — `src/iologindata.cpp`).
    pub fn from_player_item_record(
        id: ItemId,
        rec: &ItemRecord,
        items_db: &ItemDatabase,
    ) -> tfs_rust_common::Result<Self> {
        let count = rec.count.max(0).min(10000) as u16;
        let mut item = Item::new(id, rec.itemtype, count);
        let is_container = items_db.is_container(rec.itemtype);
        if !rec.attributes.is_empty() {
            match crate::item_blob::parse_item_blob(&rec.attributes, is_container) {
                Ok(p) => {
                    item.attributes = p.attrs;
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

    /// TFS `Item::isStoreItem`
    #[inline]
    pub fn is_store_item(&self) -> bool {
        self.attributes.is_store_item()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ItemId;

    #[test]
    fn test_item_creation() {
        let item = Item::new_single(ItemId::default(), 100);
        assert_eq!(item.item_type, 100);
        assert_eq!(item.count, 1);
    }

    #[test]
    fn test_item_with_count() {
        let item = Item::new(ItemId::default(), 100, 100);
        assert_eq!(item.count, 100);
        assert_eq!(item.client_count(), 100);
    }

    #[test]
    fn test_item_attributes() {
        let mut item = Item::new_single(ItemId::default(), 100);

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
        let single = Item::new_single(ItemId::default(), 100);
        assert!(!single.is_stack());

        let stack = Item::new(ItemId::default(), 100, 5);
        assert!(stack.is_stack());
    }
}
