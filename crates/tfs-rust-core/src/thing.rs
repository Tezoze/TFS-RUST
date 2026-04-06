//! Thing enum - can be an Item or a Creature
// C++ reference: `src/thing.h`

use crate::ids::{CreatureId, ItemId};

/// A thing is either an Item or a Creature
// C++ ref: `src/thing.h` Thing class hierarchy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Thing {
    Item(ItemId),
    Creature(CreatureId),
}

impl Thing {
    /// Check if this thing is an item
    pub fn is_item(&self) -> bool {
        matches!(self, Thing::Item(_))
    }

    /// Check if this thing is a creature
    pub fn is_creature(&self) -> bool {
        matches!(self, Thing::Creature(_))
    }

    /// Get the item ID if this is an item
    pub fn as_item(&self) -> Option<ItemId> {
        match self {
            Thing::Item(id) => Some(*id),
            _ => None,
        }
    }

    /// Get the creature ID if this is a creature
    pub fn as_creature(&self) -> Option<CreatureId> {
        match self {
            Thing::Creature(id) => Some(*id),
            _ => None,
        }
    }
}

impl From<ItemId> for Thing {
    fn from(id: ItemId) -> Self {
        Thing::Item(id)
    }
}

impl From<CreatureId> for Thing {
    fn from(id: CreatureId) -> Self {
        Thing::Creature(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thing_item() {
        let item_id = ItemId::default();
        let thing = Thing::Item(item_id);
        assert!(thing.is_item());
        assert!(!thing.is_creature());
        assert_eq!(thing.as_item(), Some(item_id));
        assert_eq!(thing.as_creature(), None);
    }

    #[test]
    fn test_thing_creature() {
        let creature_id = CreatureId::default();
        let thing = Thing::Creature(creature_id);
        assert!(!thing.is_item());
        assert!(thing.is_creature());
        assert_eq!(thing.as_creature(), Some(creature_id));
        assert_eq!(thing.as_item(), None);
    }

    #[test]
    fn test_thing_from() {
        let item_id = ItemId::default();
        let creature_id = CreatureId::default();

        let thing1: Thing = item_id.into();
        assert!(thing1.is_item());

        let thing2: Thing = creature_id.into();
        assert!(thing2.is_creature());
    }
}
