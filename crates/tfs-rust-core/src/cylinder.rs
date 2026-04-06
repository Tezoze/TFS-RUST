//! Cylinder enum - zero-cost dispatch for Tile, Container, and Player inventory
// C++ reference: `src/cylinder.h` (Cylinder hierarchy)
//
// DEVIATION FROM C++: Using enum dispatch instead of trait objects.
// Rationale: Exactly 3 known cylinder types (Tile, Container, Inventory) that
// won't change. Enum dispatch is zero-cost, exhaustive, and more idiomatic Rust.

use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use crate::thing::Thing;
use tfs_rust_common::Position;

/// Special index values for cylinder operations
pub const INDEX_WHEREEVER: i32 = -1;

/// Flags for cylinder operations
// C++ ref: `src/cylinder.h:17-26`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CylinderFlags {
    pub bits: u32,
}

impl CylinderFlags {
    pub const NONE: Self = Self { bits: 0 };
    /// Bypass limits like capacity/container limits, blocking items/creatures etc.
    pub const NO_LIMIT: Self = Self { bits: 1 << 0 };
    /// Bypass movable blocking item checks
    pub const IGNORE_BLOCK_ITEM: Self = Self { bits: 1 << 1 };
    /// Bypass creature checks
    pub const IGNORE_BLOCK_CREATURE: Self = Self { bits: 1 << 2 };
    /// Used by containers to query capacity of the carrier (player)
    pub const CHILD_IS_OWNER: Self = Self { bits: 1 << 3 };
    /// An additional check is done for floor changing/teleport items
    pub const PATHFINDING: Self = Self { bits: 1 << 4 };
    /// Bypass field damage checks
    pub const IGNORE_FIELD_DAMAGE: Self = Self { bits: 1 << 5 };
    /// Bypass check for mobility
    pub const IGNORE_NOT_MOVEABLE: Self = Self { bits: 1 << 6 };
    /// queryDestination will not try to stack items together
    pub const IGNORE_AUTO_STACK: Self = Self { bits: 1 << 7 };

    pub fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) != 0
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }
}

/// Link type for post-add/remove notifications
// C++ ref: `src/cylinder.h:28-33`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CylinderLink {
    Owner = 0,
    Parent = 1,
    TopParent = 2,
    Near = 3,
}

/// Type of cylinder
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CylinderType {
    Tile = 0,
    Container = 1,
    Inventory = 2,
}

/// Cylinder enum - zero-cost dispatch over Tile, Container, and Player inventory
///
/// This replaces the C++ Cylinder virtual class hierarchy with an enum.
/// Each variant holds the identifying information needed to locate the actual
/// storage (items on tile, items in container, or items in player inventory).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cylinder {
    /// Item is on a tile at the given position
    /// Items are stored in the tile's down_items/top_items vectors
    Tile { pos: Position },
    /// Item is inside a container
    /// Items are stored in the container's itemlist
    Container { item_id: ItemId },
    /// Item is in a player's inventory slot
    /// Items are stored in the player's inventory.slots array
    Inventory { player_id: CreatureId, slot: u8 },
}

impl Cylinder {
    /// Get the type of this cylinder
    pub fn cylinder_type(&self) -> CylinderType {
        match self {
            Cylinder::Tile { .. } => CylinderType::Tile,
            Cylinder::Container { .. } => CylinderType::Container,
            Cylinder::Inventory { .. } => CylinderType::Inventory,
        }
    }

    /// Check if this cylinder is a tile
    pub fn is_tile(&self) -> bool {
        matches!(self, Cylinder::Tile { .. })
    }

    /// Check if this cylinder is a container
    pub fn is_container(&self) -> bool {
        matches!(self, Cylinder::Container { .. })
    }

    /// Check if this cylinder is an inventory slot
    pub fn is_inventory(&self) -> bool {
        matches!(self, Cylinder::Inventory { .. })
    }

    /// Get the tile position if this is a tile cylinder
    pub fn as_tile(&self) -> Option<Position> {
        match self {
            Cylinder::Tile { pos } => Some(*pos),
            _ => None,
        }
    }

    /// Get the container item ID if this is a container cylinder
    pub fn as_container(&self) -> Option<ItemId> {
        match self {
            Cylinder::Container { item_id } => Some(*item_id),
            _ => None,
        }
    }

    /// Get the player ID and slot if this is an inventory cylinder
    pub fn as_inventory(&self) -> Option<(CreatureId, u8)> {
        match self {
            Cylinder::Inventory { player_id, slot } => Some((*player_id, *slot)),
            _ => None,
        }
    }

    /// Get a description for error messages
    pub fn description(&self) -> String {
        match self {
            Cylinder::Tile { pos } => format!("tile at ({}, {}, {})", pos.x, pos.y, pos.z),
            Cylinder::Container { item_id } => format!("container {:?}", item_id),
            Cylinder::Inventory { player_id, slot } => format!("player {:?} slot {}", player_id, slot),
        }
    }
}

impl From<Position> for Cylinder {
    fn from(pos: Position) -> Self {
        Cylinder::Tile { pos }
    }
}

impl From<ItemId> for Cylinder {
    fn from(item_id: ItemId) -> Self {
        Cylinder::Container { item_id }
    }
}

/// Virtual cylinder that rejects all operations
/// Used as a placeholder when no valid cylinder exists
#[derive(Debug, Clone, Copy, Default)]
pub struct VirtualCylinder;

impl VirtualCylinder {
    pub fn instance() -> Self {
        Self
    }

    pub fn query_add(&self, _index: i32, _thing: Thing, _count: u32, _flags: CylinderFlags) -> ReturnValue {
        ReturnValue::NotPossible
    }

    pub fn query_remove(&self, _thing: Thing, _count: u32, _flags: CylinderFlags) -> ReturnValue {
        ReturnValue::NotPossible
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cylinder_types() {
        let pos = Position { x: 100, y: 100, z: 7 };
        let tile_cyl: Cylinder = pos.into();
        assert!(tile_cyl.is_tile());
        assert!(!tile_cyl.is_container());
        assert!(!tile_cyl.is_inventory());
        assert_eq!(tile_cyl.as_tile(), Some(pos));
    }

    #[test]
    fn test_cylinder_container() {
        let container_id = ItemId::default();
        let container_cyl: Cylinder = container_id.into();
        assert!(!container_cyl.is_tile());
        assert!(container_cyl.is_container());
        assert!(!container_cyl.is_inventory());
        assert_eq!(container_cyl.as_container(), Some(container_id));
    }

    #[test]
    fn test_cylinder_inventory() {
        let player_id = CreatureId::default();
        let cyl = Cylinder::Inventory { player_id, slot: 1 };
        assert!(!cyl.is_tile());
        assert!(!cyl.is_container());
        assert!(cyl.is_inventory());
        assert_eq!(cyl.as_inventory(), Some((player_id, 1)));
    }

    #[test]
    fn test_virtual_cylinder() {
        let virt = VirtualCylinder::instance();
        assert_eq!(virt.query_add(0, Thing::Item(ItemId::default()), 1, CylinderFlags::NONE), ReturnValue::NotPossible);
    }

    #[test]
    fn test_cylinder_flags() {
        let flags = CylinderFlags::NO_LIMIT.union(CylinderFlags::IGNORE_BLOCK_ITEM);
        assert!(flags.contains(CylinderFlags::NO_LIMIT));
        assert!(flags.contains(CylinderFlags::IGNORE_BLOCK_ITEM));
        assert!(!flags.contains(CylinderFlags::IGNORE_BLOCK_CREATURE));
    }
}
