//! Container implementation for items (bags, chests, etc.)
// C++ reference: `src/container.h`, `src/container.cpp`

use std::collections::{HashMap, VecDeque};

use crate::ids::{CreatureId, ItemId};

/// Open container window state — `Player::openContainers` (`player.h`, `player.cpp`).
// C++ ref: `OpenContainer` struct — `containerId` + `index` (first visible slot / scroll).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenContainer {
    pub container_id: ItemId,
    pub first_index: u16,
}

/// Max simultaneous container windows per player (client cid nibble 0–15).
pub const MAX_CONTAINER_WINDOWS: u8 = 16;

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

/// Recursive item iterator — matches TFS `ContainerIterator` (`container.h`, `container.cpp` ~762–795).
// C++ ref: `ContainerIterator` — `std::list<Container*> over`, DFS-like order over nested item lists.
pub struct ContainerIterator<'a> {
    registry: &'a ContainerRegistry,
    over: VecDeque<ItemId>,
    index: usize,
}

impl<'a> ContainerIterator<'a> {
    /// Start iteration over all items held in `root_container_item_id` (not including the container item itself).
    pub fn new(registry: &'a ContainerRegistry, root_container_item_id: ItemId) -> Self {
        let mut over = VecDeque::new();
        if let Some(c) = registry.get(root_container_item_id) {
            if !c.items.is_empty() {
                over.push_back(root_container_item_id);
            }
        }
        Self {
            registry,
            over,
            index: 0,
        }
    }

    #[inline]
    pub fn has_next(&self) -> bool {
        !self.over.is_empty()
    }

    fn peek(&self) -> Option<ItemId> {
        let cid = *self.over.front()?;
        let c = self.registry.get(cid)?;
        c.items.get(self.index).copied()
    }

    /// Advance to the next item (C++ `ContainerIterator::advance`).
    pub fn advance(&mut self) {
        let Some(&front_cid) = self.over.front() else {
            return;
        };
        let Some(cur_item) = self
            .registry
            .get(front_cid)
            .and_then(|c| c.items.get(self.index).copied())
        else {
            return;
        };
        if let Some(cc) = self.registry.get(cur_item) {
            if !cc.items.is_empty() {
                self.over.push_back(cur_item);
            }
        }
        self.index += 1;
        while let Some(&fc) = self.over.front() {
            let clen = self.registry.get(fc).map(|c| c.items.len()).unwrap_or(0);
            if self.index < clen {
                break;
            }
            self.over.pop_front();
            self.index = 0;
        }
    }
}

impl<'a> Iterator for ContainerIterator<'a> {
    type Item = ItemId;

    fn next(&mut self) -> Option<ItemId> {
        if !self.has_next() {
            return None;
        }
        let id = self.peek()?;
        self.advance();
        Some(id)
    }
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
    /// Players currently viewing this container (protocol player id — same as `CreatureId` keying)
    pub open_by: Vec<CreatureId>,
    /// Recursive holding count — matches `getItemHoldingCount` / `ContainerIterator` (`container.cpp`).
    pub total_item_count: u32,
    /// Sum of `Item::getWeight()` for all descendants — matches `totalWeight` (`container.h`).
    // C++ ref: `Container::totalWeight` — updated via `updateItemWeight`.
    pub total_weight: u32,
    /// Town id for `DepotChest` save `pid` — `player_depotitems`.
    pub depot_town_id: Option<u32>,
    /// Map locker town id for virtual `DepotLocker` instances.
    pub depot_locker_town_id: Option<u32>,
    /// C++ `DepotChest::maxDepotItems` — shared limit across universal depot parent.
    pub max_depot_items: u32,
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
            total_weight: 0,
            depot_town_id: None,
            depot_locker_town_id: None,
            max_depot_items: 0,
        }
    }

    /// Create a depot chest container for a town.
    pub fn new_depot(
        item_id: ItemId,
        depot_town_id: u32,
        capacity: u32,
        max_depot_items: u32,
    ) -> Self {
        let mut container = Self::new(item_id, capacity);
        container.container_type = ContainerType::Depot;
        container.depot_town_id = Some(depot_town_id);
        container.max_depot_items = max_depot_items;
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
    pub fn new_browse_field(item_id: ItemId, _tile_pos: (u16, u16, u8)) -> Self {
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

    /// Check if this container contains a specific item (direct child only)
    pub fn contains(&self, item_id: ItemId) -> bool {
        self.items.contains(&item_id)
    }

    /// TFS `Container::isHoldingItem` — `container.cpp` ~258–266
    pub fn is_holding_item(&self, registry: &ContainerRegistry, target: ItemId) -> bool {
        ContainerIterator::new(registry, self.item_id).any(|id| id == target)
    }

    /// Add an item to the container (at the top/end) — **structural only**; use `GameWorld` to update weight/counts.
    pub fn add_item(&mut self, item_id: ItemId) -> Result<(), ContainerError> {
        if !self.unlocked {
            return Err(ContainerError::Locked);
        }
        if self.is_full() {
            return Err(ContainerError::Full);
        }
        self.items.push(item_id);
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
        let slot = self.items.get_mut(index).ok_or(ContainerError::InvalidSlot)?;
        let old_item_id = std::mem::replace(slot, new_item_id);
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
    pub fn add_viewer(&mut self, player: CreatureId) {
        if !self.open_by.contains(&player) {
            self.open_by.push(player);
        }
    }

    /// Unregister a player from viewing this container
    pub fn remove_viewer(&mut self, player: CreatureId) {
        self.open_by.retain(|&cid| cid != player);
    }

    /// Check if a specific player is viewing this container
    pub fn is_viewer(&self, player: CreatureId) -> bool {
        self.open_by.contains(&player)
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
        self.items
            .iter()
            .skip(start_index)
            .take(count)
            .copied()
            .collect()
    }

    /// Get the total number of items (for pagination UI)
    pub fn total_items(&self) -> usize {
        self.items.len()
    }

    // === Query methods (legacy stubs — full logic in `container_ops.rs` / `GameWorld`) ===

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
    containers: HashMap<ItemId, Container>,
    /// TFS `Player::openContainers` — client cid (`0..MAX_CONTAINER_WINDOWS`) → open state.
    player_open: HashMap<CreatureId, HashMap<u8, OpenContainer>>,
}

impl ContainerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn alloc_free_cid(map: &HashMap<u8, OpenContainer>) -> Option<u8> {
        (0u8..MAX_CONTAINER_WINDOWS).find(|c| !map.contains_key(c))
    }

    /// All registered container item instance ids (for recomputing derived fields after load).
    pub fn registered_container_ids(&self) -> impl Iterator<Item = ItemId> + '_ {
        self.containers.keys().copied()
    }

    /// Open container windows for a player: `(client_cid, container_item_id)`.
    pub fn open_container_entries(&self, player: CreatureId) -> Vec<(u8, ItemId)> {
        self.player_open
            .get(&player)
            .map(|m| m.iter().map(|(&cid, oc)| (cid, oc.container_id)).collect())
            .unwrap_or_default()
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

    /// Remove a container from the registry; returns `(player, client_cid)` for each closed UI window.
    // C++ ref: closing views when container item is destroyed — `Player::onRemoveContainer`.
    pub fn remove(&mut self, item_id: ItemId) -> (Option<Container>, Vec<(CreatureId, u8)>) {
        let mut closed = Vec::new();
        let players: Vec<CreatureId> = self.player_open.keys().copied().collect();
        for pid in players {
            let Some(map) = self.player_open.get(&pid) else {
                continue;
            };
            let to_drop: Vec<u8> = map
                .iter()
                .filter(|(_, oc)| oc.container_id == item_id)
                .map(|(&cid, _)| cid)
                .collect();
            for cid in to_drop {
                if self.close_container_for_player(pid, cid) {
                    closed.push((pid, cid));
                }
            }
        }
        let c = self.containers.remove(&item_id);
        (c, closed)
    }

    /// TFS `Player::addContainer` — open, replace contents of an existing cid, or set scroll; returns client cid.
    // C++ ref: `player.cpp` `Player::addContainer` — if `openContainers[cid]` exists, swap to the new container.
    pub fn add_container(
        &mut self,
        player: CreatureId,
        container_id: ItemId,
        preferred_cid: Option<u8>,
        first_index: u16,
    ) -> Option<u8> {
        let map = self.player_open.entry(player).or_default();

        let cid = if let Some(pc) = preferred_cid {
            if pc < MAX_CONTAINER_WINDOWS {
                pc
            } else {
                Self::alloc_free_cid(map)?
            }
        } else {
            Self::alloc_free_cid(map)?
        };

        if let Some(old_oc) = map.get_mut(&cid) {
            let old_id = old_oc.container_id;
            if old_id != container_id {
                if let Some(c) = self.containers.get_mut(&old_id) {
                    c.remove_viewer(player);
                }
                if let Some(c) = self.containers.get_mut(&container_id) {
                    c.add_viewer(player);
                }
                old_oc.container_id = container_id;
            }
            old_oc.first_index = first_index;
            return Some(cid);
        }

        map.insert(
            cid,
            OpenContainer {
                container_id,
                first_index,
            },
        );

        if let Some(container) = self.containers.get_mut(&container_id) {
            container.add_viewer(player);
        }

        Some(cid)
    }

    /// TFS `Player::setContainerIndex` — pagination scroll offset.
    pub fn set_container_index(&mut self, player: CreatureId, cid: u8, first_index: u16) -> bool {
        let Some(m) = self.player_open.get_mut(&player) else {
            return false;
        };
        let Some(oc) = m.get_mut(&cid) else {
            return false;
        };
        oc.first_index = first_index;
        true
    }

    /// TFS `Player::closeContainer` — returns the container item id that was closed, if any.
    pub fn close_container_for_player(&mut self, player: CreatureId, cid: u8) -> bool {
        let container_id = {
            let Some(m) = self.player_open.get_mut(&player) else {
                return false;
            };
            let Some(oc) = m.remove(&cid) else {
                return false;
            };
            oc.container_id
        };
        if let Some(c) = self.containers.get_mut(&container_id) {
            c.remove_viewer(player);
        }
        if self
            .player_open
            .get(&player)
            .is_some_and(|m| m.is_empty())
        {
            self.player_open.remove(&player);
        }
        true
    }

    /// Legacy name — `Player::closeContainer`.
    #[inline]
    pub fn release_cid(&mut self, player: CreatureId, cid: u8) -> bool {
        self.close_container_for_player(player, cid)
    }

    /// TFS `Player::getContainerByID` — resolve client cid → container item.
    #[inline]
    pub fn get_container_by_cid(&self, player: CreatureId, cid: u8) -> Option<ItemId> {
        self.player_open
            .get(&player)
            .and_then(|m| m.get(&cid))
            .map(|oc| oc.container_id)
    }

    /// TFS `Player::getContainerID` — resolve container item → client cid for this player.
    pub fn get_cid_for_container(&self, player: CreatureId, container_id: ItemId) -> Option<u8> {
        self.player_open.get(&player).and_then(|m| {
            m.iter()
                .find(|(_, oc)| oc.container_id == container_id)
                .map(|(&cid, _)| cid)
        })
    }

    /// TFS `Player::getContainerIndex` — scroll offset for a cid.
    pub fn get_container_first_index(&self, player: CreatureId, cid: u8) -> Option<u16> {
        self.player_open
            .get(&player)
            .and_then(|m| m.get(&cid))
            .map(|oc| oc.first_index)
    }

    /// All open container roots for a player (for iteration).
    pub fn open_container_roots(&self, player: CreatureId) -> impl Iterator<Item = ItemId> + '_ {
        self.player_open
            .get(&player)
            .into_iter()
            .flat_map(|m| m.values())
            .map(|oc| oc.container_id)
    }

    /// Close all containers for a player (e.g., on logout); returns `(client_cid, container_id)` pairs.
    pub fn close_all_for_player(&mut self, player: CreatureId) -> Vec<(u8, ItemId)> {
        let Some(map) = self.player_open.remove(&player) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (cid, oc) in map {
            if let Some(container) = self.containers.get_mut(&oc.container_id) {
                container.remove_viewer(player);
            }
            out.push((cid, oc.container_id));
        }
        out
    }

    /// Assign a new client CID for a player opening a container (compat — uses `add_container` with scroll 0).
    #[inline]
    pub fn assign_cid(&mut self, player: CreatureId, container_id: ItemId) -> Option<u8> {
        self.add_container(player, container_id, None, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

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

        assert!(container.add_item(item1).is_ok());
        assert!(container.add_item(item2).is_ok());
        assert_eq!(container.size(), 2);

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

        assert!(matches!(
            container.add_item(ItemId::default()),
            Err(ContainerError::Full)
        ));
    }

    #[test]
    fn test_locked_container() {
        let mut container = Container::new(ItemId::default(), 5);
        container.unlocked = false;

        assert!(matches!(
            container.add_item(ItemId::default()),
            Err(ContainerError::Locked)
        ));
        assert!(matches!(
            container.remove_item(0),
            Err(ContainerError::Locked)
        ));
    }

    #[test]
    fn test_viewer_tracking() {
        let mut container = Container::new(ItemId::default(), 5);
        let p1 = CreatureId::default();
        let mut sm: SlotMap<CreatureId, u8> = SlotMap::with_capacity_and_key(4);
        let p2 = sm.insert(0);
        let p3 = sm.insert(0);

        container.add_viewer(p1);
        container.add_viewer(p2);
        assert!(container.is_viewer(p1));
        assert!(container.is_viewer(p2));
        assert!(!container.is_viewer(p3));

        container.remove_viewer(p1);
        assert!(!container.is_viewer(p1));
    }

    #[test]
    fn test_container_registry() {
        let mut registry = ContainerRegistry::new();
        let container = Container::new(ItemId::default(), 10);
        let item_id = container.item_id;
        let player = CreatureId::default();

        registry.register(container);
        assert!(registry.get(item_id).is_some());

        let cid = registry.assign_cid(player, item_id);
        assert_eq!(cid, Some(0));

        let retrieved = registry.get_container_by_cid(player, cid.unwrap_or(0));
        assert_eq!(retrieved, Some(item_id));
    }

    #[test]
    fn container_iterator_order_matches_cpp_pattern() {
        // Root [A, B], A inner [A1] → order A, B, A1 (see `ContainerIterator::advance`).
        let mut sm: SlotMap<ItemId, u8> = SlotMap::with_capacity_and_key(16);
        let root = sm.insert(0);
        let a = sm.insert(0);
        let b = sm.insert(0);
        let a1 = sm.insert(0);

        let mut inner = Container::new(a, 8);
        inner.add_item(a1).unwrap();
        let mut outer = Container::new(root, 8);
        outer.add_item(a).unwrap();
        outer.add_item(b).unwrap();

        let mut registry = ContainerRegistry::new();
        registry.register(outer);
        registry.register(inner);

        let order: Vec<ItemId> = ContainerIterator::new(&registry, root).collect();
        assert_eq!(order, vec![a, b, a1]);
    }

    #[test]
    fn is_holding_item_nested() {
        let mut sm: SlotMap<ItemId, u8> = SlotMap::with_capacity_and_key(16);
        let root = sm.insert(0);
        let a = sm.insert(0);
        let a1 = sm.insert(0);

        let mut inner = Container::new(a, 8);
        inner.add_item(a1).unwrap();
        let mut outer = Container::new(root, 8);
        outer.add_item(a).unwrap();

        let mut registry = ContainerRegistry::new();
        registry.register(outer);
        registry.register(inner);

        let c = registry.get(root).unwrap();
        assert!(c.is_holding_item(&registry, a1));
        assert!(!c.is_holding_item(&registry, root));
    }

    /// C++ `Player::addContainer` — occupied `cid` swaps to the new container and transfers viewers.
    #[test]
    fn add_container_same_cid_replaces_and_moves_viewers() {
        let mut sm: SlotMap<ItemId, u8> = SlotMap::with_capacity_and_key(8);
        let c1 = sm.insert(0);
        let c2 = sm.insert(0);
        let player = CreatureId::default();

        let mut registry = ContainerRegistry::new();
        registry.register(Container::new(c1, 10));
        registry.register(Container::new(c2, 10));

        assert_eq!(registry.add_container(player, c1, Some(3), 0), Some(3));
        assert!(registry.get(c1).unwrap().is_viewer(player));
        assert!(!registry.get(c2).unwrap().is_viewer(player));

        assert_eq!(registry.add_container(player, c2, Some(3), 0), Some(3));
        assert!(!registry.get(c1).unwrap().is_viewer(player));
        assert!(registry.get(c2).unwrap().is_viewer(player));
        assert_eq!(registry.get_container_by_cid(player, 3), Some(c2));
    }
}
