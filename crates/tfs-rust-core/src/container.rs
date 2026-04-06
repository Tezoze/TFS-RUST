//! Container implementation for items (bags, chests, etc.)
// C++ reference: `src/container.h`, `src/container.cpp`

use crate::ids::ItemId;
use crate::item::Item;

/// Error type for container operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerError {
    /// Container is full, cannot add more items
    Full,
    /// Invalid slot index
    InvalidSlot,
    /// Item not found in container
    ItemNotFound,
    /// Container is locked/unlocked
    Locked,
    /// Cannot add item (e.g., item is too heavy, or is a container that would create a cycle)
    CannotAdd,
}

/// Type of container for UI and behavior differentiation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerType {
    /// Standard container (bag, backpack, chest)
    Normal,
    /// Depot chest for storage
    Depot,
    /// Inbox for store deliveries
    Inbox,
    /// Store inbox for purchased items
    StoreInbox,
    /// Browse-field (viewing a tile as a container)
    BrowseField,
}

/// A container holds items and provides stack-like operations
// C++ ref: `src/container.h` Container class
#[derive(Debug, Clone)]
pub struct Container {
    /// The item ID of this container (the actual container item in the world)
    pub item_id: ItemId,
    /// Type of container for behavior/UI
    pub container_type: ContainerType,
    /// Items stored in this container, in order (first = bottom, last = top)
    pub items: Vec<ItemId>,
    /// Maximum capacity (number of item slots)
    pub capacity: u32,
    /// Whether the container can be modified (add/remove items)
    pub unlocked: bool,
    /// Whether this container uses pagination (more items than visible slots)
    pub pagination: bool,
    /// Parent container ID (for nested containers)
    pub parent_container: Option<ItemId>,
    /// Players currently viewing this container (cid list)
    pub open_by: Vec<u32>,
    /// Total recursive item count (including items in nested containers)
    pub total_item_count: u32,
}

impl Container {
    /// Create a new container with the given item ID and capacity
    pub fn new(item_id: ItemId, capacity: u32) -> Self {
        Self {
            item_id,
            container_type: ContainerType::Normal,
            items: Vec::with_capacity(capacity.min(64) as usize),
            capacity,
            unlocked: true,
            pagination: false,
            parent_container: None,
            open_by: Vec::new(),
            total_item_count: 0,
        }
    }

    /// Create a depot container
    pub fn new_depot(item_id: ItemId, depot_id: u16, capacity: u32) -> Self {
        let mut container = Self::new(item_id, capacity);
        container.container_type = ContainerType::Depot;
        // Store depot_id in a field or attribute - for now we'll handle this at higher level
        container
    }

    /// Create an inbox container
    pub fn new_inbox(item_id: ItemId, capacity: u32) -> Self {
        let mut container = Self::new(item_id, capacity);
        container.container_type = ContainerType::Inbox;
        container
    }

    /// Create a store inbox container
    pub fn new_store_inbox(item_id: ItemId, capacity: u32) -> Self {
        let mut container = Self::new(item_id, capacity);
        container.container_type = ContainerType::StoreInbox;
        container
    }

    /// Create a browse-field container for viewing a tile
    pub fn new_browse_field(item_id: ItemId, tile_pos: (u16, u16, u8)) -> Self {
        let mut container = Self::new(item_id, 64); // Browse fields have larger capacity
        container.container_type = ContainerType::BrowseField;
        container.pagination = true; // Browse fields always have pagination
        container
    }

    // === Basic accessors ===

    /// Get the number of items directly in this container
    pub fn size(&self) -> usize {
        self.items.len()
    }

    /// Check if the container is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Check if the container has reached capacity
    pub fn is_full(&self) -> bool {
        self.items.len() as u32 >= self.capacity
    }

    /// Get the number of available slots
    pub fn available_slots(&self) -> u32 {
        self.capacity.saturating_sub(self.items.len() as u32)
    }

    /// Check if this container has a parent (is nested)
    pub fn has_parent(&self) -> bool {
        self.parent_container.is_some()
    }

    // === Item operations ===

    /// Get an item by its index (0-based from bottom)
    pub fn get_item(&self, index: usize) -> Option<ItemId> {
        self.items.get(index).copied()
    }

    /// Get the index of an item in this container
    pub fn index_of(&self, item_id: ItemId) -> Option<usize> {
        self.items.iter().position(|&id| id == item_id)
    }

    /// Check if this container contains a specific item
    pub fn contains(&self, item_id: ItemId) -> bool {
        self.items.contains(&item_id)
    }

    /// Add an item to the container (at the top/end)
    pub fn add_item(&mut self, item_id: ItemId) -> Result<(), ContainerError> {
        if !self.unlocked {
            return Err(ContainerError::Locked);
        }
        if self.is_full() {
            return Err(ContainerError::Full);
        }
        self.items.push(item_id);
        self.total_item_count += 1;
        Ok(())
    }

    /// Insert an item at a specific position
    pub fn insert_item(&mut self, index: usize, item_id: ItemId) -> Result<(), ContainerError> {
        if !self.unlocked {
            return Err(ContainerError::Locked);
        }
        if self.is_full() {
            return Err(ContainerError::Full);
        }
        if index > self.items.len() {
            return Err(ContainerError::InvalidSlot);
        }
        self.items.insert(index, item_id);
        self.total_item_count += 1;
        Ok(())
    }

    /// Remove an item from the container by index
    pub fn remove_item(&mut self, index: usize) -> Result<ItemId, ContainerError> {
        if !self.unlocked {
            return Err(ContainerError::Locked);
        }
        if index >= self.items.len() {
            return Err(ContainerError::InvalidSlot);
        }
        let item_id = self.items.remove(index);
        self.total_item_count = self.total_item_count.saturating_sub(1);
        Ok(item_id)
    }

    /// Remove a specific item from the container
    pub fn remove_specific_item(&mut self, item_id: ItemId) -> Result<usize, ContainerError> {
        if !self.unlocked {
            return Err(ContainerError::Locked);
        }
        match self.index_of(item_id) {
            Some(index) => {
                self.items.remove(index);
                self.total_item_count = self.total_item_count.saturating_sub(1);
                Ok(index)
            }
            None => Err(ContainerError::ItemNotFound),
        }
    }

    /// Update/replace an item at a specific index
    pub fn update_item(&mut self, index: usize, new_item_id: ItemId) -> Result<ItemId, ContainerError> {
        if index >= self.items.len() {
            return Err(ContainerError::InvalidSlot);
        }
        let old_item_id = std::mem::replace(&mut self.items[index], new_item_id);
        Ok(old_item_id)
    }

    /// Swap two items by their indices
    pub fn swap_items(&mut self, index1: usize, index2: usize) -> Result<(), ContainerError> {
        if index1 >= self.items.len() || index2 >= self.items.len() {
            return Err(ContainerError::InvalidSlot);
        }
        self.items.swap(index1, index2);
        Ok(())
    }

    // === Viewer tracking ===

    /// Register a player as viewing this container
    pub fn add_viewer(&mut self, player_cid: u32) {
        if !self.open_by.contains(&player_cid) {
            self.open_by.push(player_cid);
        }
    }

    /// Unregister a player from viewing this container
    pub fn remove_viewer(&mut self, player_cid: u32) {
        self.open_by.retain(|&cid| cid != player_cid);
    }

    /// Check if a specific player is viewing this container
    pub fn is_viewer(&self, player_cid: u32) -> bool {
        self.open_by.contains(&player_cid)
    }

    /// Get the number of players viewing this container
    pub fn viewer_count(&self) -> usize {
        self.open_by.len()
    }

    /// Check if any players are viewing this container
    pub fn has_viewers(&self) -> bool {
        !self.open_by.is_empty()
    }

    // === Pagination support ===

    /// Get items for a specific page (for pagination)
    pub fn get_page(&self, start_index: usize, count: usize) -> Vec<ItemId> {
        self.items.iter().skip(start_index).take(count).copied().collect()
    }

    /// Get the total number of items (for pagination UI)
    pub fn total_items(&self) -> usize {
        self.items.len()
    }

    // === Query methods (for Cylinder trait integration) ===

    /// Query if an item can be added at a specific index
    pub fn query_add(&self, _index: i32, _item_id: ItemId) -> Result<(), ContainerError> {
        if !self.unlocked {
            return Err(ContainerError::Locked);
        }
        if self.is_full() {
            return Err(ContainerError::Full);
        }
        Ok(())
    }

    /// Query if an item can be removed
    pub fn query_remove(&self, _item_id: ItemId) -> Result<(), ContainerError> {
        if !self.unlocked {
            return Err(ContainerError::Locked);
        }
        Ok(())
    }
}

/// Container registry for managing open containers
#[derive(Debug, Default)]
pub struct ContainerRegistry {
    /// Map from container item ID to Container
    containers: std::collections::HashMap<ItemId, Container>,
    /// Map from player CID to their open container IDs (ordered by cid)
    player_containers: std::collections::HashMap<u32, Vec<ItemId>>,
    /// Next container CID (0-255) for client communication
    next_cid: u8,
}

impl ContainerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new container
    pub fn register(&mut self, container: Container) -> ItemId {
        let id = container.item_id;
        self.containers.insert(id, container);
        id
    }

    /// Get a container by its item ID
    pub fn get(&self, item_id: ItemId) -> Option<&Container> {
        self.containers.get(&item_id)
    }

    /// Get a mutable container by its item ID
    pub fn get_mut(&mut self, item_id: ItemId) -> Option<&mut Container> {
        self.containers.get_mut(&item_id)
    }

    /// Remove a container from the registry
    pub fn remove(&mut self, item_id: ItemId) -> Option<Container> {
        // Also remove from all player container lists
        for containers in self.player_containers.values_mut() {
            containers.retain(|&id| id != item_id);
        }
        self.containers.remove(&item_id)
    }

    /// Assign a new client CID for a player opening a container
    pub fn assign_cid(&mut self, player_cid: u32, container_id: ItemId) -> u8 {
        let cid = self.next_cid;
        self.next_cid = self.next_cid.wrapping_add(1);
        
        // Track which containers this player has open
        self.player_containers
            .entry(player_cid)
            .or_default()
            .push(container_id);
        
        // Add player as viewer
        if let Some(container) = self.containers.get_mut(&container_id) {
            container.add_viewer(player_cid);
        }
        
        cid
    }

    /// Release a CID when a player closes a container
    pub fn release_cid(&mut self, player_cid: u32, cid: u8) {
        // Remove player from container viewers
        if let Some(containers) = self.player_containers.get(&player_cid) {
            if let Some(&container_id) = containers.get(cid as usize) {
                if let Some(container) = self.containers.get_mut(&container_id) {
                    container.remove_viewer(player_cid);
                }
            }
        }
        
        // Remove from player's container list
        if let Some(containers) = self.player_containers.get_mut(&player_cid) {
            if (cid as usize) < containers.len() {
                containers.remove(cid as usize);
            }
        }
    }

    /// Get container ID by player CID and client CID
    pub fn get_container_by_cid(&self, player_cid: u32, cid: u8) -> Option<ItemId> {
        self.player_containers
            .get(&player_cid)
            .and_then(|containers| containers.get(cid as usize).copied())
    }

    /// Get all containers open by a player
    pub fn get_player_containers(&self, player_cid: u32) -> &[ItemId] {
        self.player_containers
            .get(&player_cid)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Close all containers for a player (e.g., on logout)
    pub fn close_all_for_player(&mut self, player_cid: u32) {
        if let Some(containers) = self.player_containers.remove(&player_cid) {
            for container_id in containers {
                if let Some(container) = self.containers.get_mut(&container_id) {
                    container.remove_viewer(player_cid);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_creation() {
        let container = Container::new(ItemId::default(), 20);
        assert_eq!(container.capacity, 20);
        assert!(container.is_empty());
        assert!(!container.is_full());
    }

    #[test]
    fn test_add_remove_items() {
        let mut container = Container::new(ItemId::default(), 5);
        let item1 = ItemId::default();
        let item2 = ItemId::default();

        // Add items
        assert!(container.add_item(item1).is_ok());
        assert!(container.add_item(item2).is_ok());
        assert_eq!(container.size(), 2);

        // Remove by index
        let removed = container.remove_item(0).unwrap();
        assert_eq!(removed, item1);
        assert_eq!(container.size(), 1);
    }

    #[test]
    fn test_container_full() {
        let mut container = Container::new(ItemId::default(), 2);
        
        assert!(container.add_item(ItemId::default()).is_ok());
        assert!(container.add_item(ItemId::default()).is_ok());
        assert!(container.is_full());
        
        // Should fail when full
        assert!(matches!(container.add_item(ItemId::default()), Err(ContainerError::Full)));
    }

    #[test]
    fn test_locked_container() {
        let mut container = Container::new(ItemId::default(), 5);
        container.unlocked = false;

        assert!(matches!(container.add_item(ItemId::default()), Err(ContainerError::Locked)));
        assert!(matches!(container.remove_item(0), Err(ContainerError::Locked)));
    }

    #[test]
    fn test_viewer_tracking() {
        let mut container = Container::new(ItemId::default(), 5);
        
        container.add_viewer(100);
        container.add_viewer(101);
        assert!(container.is_viewer(100));
        assert!(container.is_viewer(101));
        assert!(!container.is_viewer(102));
        
        container.remove_viewer(100);
        assert!(!container.is_viewer(100));
    }

    #[test]
    fn test_container_registry() {
        let mut registry = ContainerRegistry::new();
        let container = Container::new(ItemId::default(), 10);
        let item_id = container.item_id;
        
        registry.register(container);
        assert!(registry.get(item_id).is_some());
        
        let cid = registry.assign_cid(100, item_id);
        assert_eq!(cid, 0); // First CID should be 0
        
        let retrieved = registry.get_container_by_cid(100, cid);
        assert_eq!(retrieved, Some(item_id));
    }
}
