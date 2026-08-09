//! Player depot chest, inbox, and virtual locker runtime.
// C++ reference: `src/player.cpp` (`getDepotChest`, `getDepotLocker`, `getInbox`, `isNearDepotBox`, `getMaxDepotItems`)
//                `src/depotchest.cpp`, `src/depotlocker.cpp`, `src/inbox.cpp`

use tfs_rust_common::Position;

use crate::container::{Container, ContainerRegistry, ContainerType};
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::item::Item;
use crate::item_constants::{ITEM_DEPOT, ITEM_INBOX, ITEM_LOCKER1, ITEM_MARKET};
use crate::tile::flags as tilestate;

const INBOX_CAPACITY: u32 = 30;

impl GameWorld {
    /// C++ `Player::getMaxDepotItems` — `player.cpp` ~4667.
    pub fn player_get_max_depot_items(&self, cid: CreatureId) -> u32 {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return 2000;
        };
        if let Some(g) = self.groups.groups.get(&p.group_id) {
            if g.max_depot_items != 0 {
                return g.max_depot_items;
            }
        }
        if self.player_is_premium(cid) {
            self.config.depot_premium_limit().unwrap_or(10_000)
        } else {
            self.config.depot_free_limit().unwrap_or(2000)
        }
    }

    /// C++ `Player::setLastDepotId` — enables depot save on logout.
    pub fn player_set_last_depot_id(&mut self, cid: CreatureId, depot_id: u32) {
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.last_depot_id = i32::try_from(depot_id).unwrap_or(-1);
            if let Some(ref mut persist) = p.persist {
                persist.last_depot_id = p.last_depot_id;
            }
        }
    }

    /// C++ `Player::isNearDepotBox` — `player.cpp` ~792.
    pub fn player_is_near_depot_box(&self, cid: CreatureId) -> bool {
        let Some(pos) = self.creatures.get(cid).map(|k| k.position()) else {
            return false;
        };
        for cx in -1i32..=1 {
            for cy in -1i32..=1 {
                let x = i32::from(pos.x)
                    .saturating_add(cx)
                    .clamp(0, u16::MAX as i32) as u16;
                let y = i32::from(pos.y)
                    .saturating_add(cy)
                    .clamp(0, u16::MAX as i32) as u16;
                let check = Position::new(x, y, pos.z);
                if let Some(tile) = self.map.get_tile(check) {
                    if tile.body().flags & tilestate::DEPOT != 0 {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Whether `root` is a depot chest, inbox, or virtual locker owned by `player`.
    pub fn player_owns_depot_root(&self, cid: CreatureId, root: ItemId) -> bool {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return false;
        };
        if p.inbox_root == Some(root) {
            return true;
        }
        if p.depot_chests.values().any(|&id| id == root) {
            return true;
        }
        p.depot_lockers.values().any(|&id| id == root)
    }

    /// Whether any ancestor of `item_id` is player-owned depot/inbox/locker.
    pub fn player_owns_depot_container_tree(&self, cid: CreatureId, item_id: ItemId) -> bool {
        let mut cur = Some(item_id);
        while let Some(id) = cur {
            if self.player_owns_depot_root(cid, id) {
                return true;
            }
            cur = self
                .container_registry
                .get(id)
                .and_then(|c| c.parent_container);
        }
        false
    }

    /// Resolve depot id from a map locker item — `DepotLocker::getDepotId` / `ATTR_DEPOT_ID`.
    pub fn depot_id_from_locker_item(&self, item_id: ItemId, fallback_town_id: i32) -> u32 {
        if let Some(item) = self.items.get(item_id) {
            if item.attributes.as_deref().is_some_and(|a| a.has_depot_id()) {
                return u32::from(item.depot_id());
            }
        }
        if fallback_town_id >= 0 {
            return fallback_town_id as u32;
        }
        0
    }

    /// C++ `Player::getInbox` — `player.h` / constructor `player.cpp` ~37.
    pub fn player_get_inbox(&mut self, cid: CreatureId, auto_create: bool) -> Option<ItemId> {
        if let Some(CreatureKind::Player(p)) = self.creatures.get(cid) {
            if let Some(iid) = p.inbox_root {
                return Some(iid);
            }
        }
        if !auto_create {
            return None;
        }
        let iid = self.items.insert(Item::new_single(ITEM_INBOX));
        let mut reg = std::mem::take(&mut self.container_registry);
        reg.register(Container::new_inbox(iid, INBOX_CAPACITY));
        self.container_registry = reg;
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.inbox_root = Some(iid);
        }
        Some(iid)
    }

    /// C++ `Player::getDepotChest` — `player.cpp` ~810.
    pub fn player_get_depot_chest(
        &mut self,
        cid: CreatureId,
        town_id: u32,
        auto_create: bool,
    ) -> Option<ItemId> {
        if let Some(CreatureKind::Player(p)) = self.creatures.get(cid) {
            if let Some(&iid) = p.depot_chests.get(&town_id) {
                return Some(iid);
            }
        }
        if !auto_create {
            return None;
        }
        let max_items = self.player_get_max_depot_items(cid);
        let cap = self.container_capacity(ITEM_DEPOT);
        let iid = self.items.insert(Item::new_single(ITEM_DEPOT));
        let mut reg = std::mem::take(&mut self.container_registry);
        reg.register(Container::new_depot(iid, town_id, cap, max_items));
        self.container_registry = reg;
        if let Some(town) = self.map.towns.get(&town_id) {
            if let Some(item) = self.items.get_mut(iid) {
                item.set_description(format!("Depot of {}.", town.name));
            }
        }
        self.refresh_container_derived(iid);
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.depot_chests.insert(town_id, iid);
        }
        Some(iid)
    }

    /// C++ `Player::getDepotLocker` — `player.cpp` ~826.
    /// 772: locker contains only a single depot chest (`crplayer.cc:2004` `LoadDepot`).
    /// 1098: locker contains market + inbox + unified depot with per-town chests.
    pub fn player_get_depot_locker(&mut self, cid: CreatureId, depot_id: u32) -> Option<ItemId> {
        let existing = self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Player(p) => p.depot_lockers.get(&depot_id).copied(),
            _ => None,
        });
        if let Some(locker_id) = existing {
            self.sync_depot_locker_contents(cid, locker_id);
            return Some(locker_id);
        }

        use crate::formulas::DepotLockerStructure;
        match self.mechanics.profile.depot_locker_structure {
            DepotLockerStructure::ClassicDepotChest => {
                self.create_classic_depot_locker(cid, depot_id)
            }
            DepotLockerStructure::TfsMarketInbox => {
                self.create_tfs_depot_locker(cid, depot_id)
            }
        }
    }

    /// 772 depot locker — single depot chest directly inside the locker
    /// (`crplayer.cc:2004` `LoadDepot` → `Create(Con, GetSpecialObject(DEPOT_CHEST), 0)`).
    fn create_classic_depot_locker(
        &mut self,
        cid: CreatureId,
        depot_id: u32,
    ) -> Option<ItemId> {
        let locker_cap = self.container_capacity(ITEM_LOCKER1);
        let locker_iid = self.items.insert(Item::new_single(ITEM_LOCKER1));

        // Reuse existing depot chest if login already created one with loaded items.
        // `load_depot_table` calls `player_get_depot_chest` which populates
        // `player.depot_chests` but does NOT create a locker. Without this reuse,
        // creating a new empty chest would overwrite the reference to the chest
        // that holds all loaded depot items, orphaning them.
        let chest_iid = self
            .creatures
            .get(cid)
            .and_then(|k| match k {
                CreatureKind::Player(p) => p.depot_chests.get(&depot_id).copied(),
                _ => None,
            })
            .unwrap_or_else(|| {
                let max_items = self.player_get_max_depot_items(cid);
                let chest_cap = self.container_capacity(ITEM_DEPOT);
                let new_chest = self.items.insert(Item::new_single(ITEM_DEPOT));
                let mut reg = std::mem::take(&mut self.container_registry);
                let mut chest = Container::new_depot(new_chest, depot_id, chest_cap, max_items);
                chest.parent_container = Some(locker_iid);
                reg.register(chest);
                self.container_registry = reg;
                if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                    p.depot_chests.insert(depot_id, new_chest);
                }
                new_chest
            });

        let mut reg = std::mem::take(&mut self.container_registry);
        let mut locker = Container::new(locker_iid, locker_cap);
        locker.depot_locker_town_id = Some(depot_id);
        reg.register(locker);
        self.container_registry = reg;

        // Re-parent the existing chest under the new locker.
        Self::link_child_in_registry(&mut self.container_registry, &mut self.items, locker_iid, chest_iid);

        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.depot_lockers.insert(depot_id, locker_iid);
        }

        self.refresh_container_chain(locker_iid);
        Some(locker_iid)
    }

    /// 1098 / TFS depot locker — market + inbox + unified depot with per-town chests.
    fn create_tfs_depot_locker(
        &mut self,
        cid: CreatureId,
        depot_id: u32,
    ) -> Option<ItemId> {
        let locker_cap = self.container_capacity(ITEM_LOCKER1);
        let max_items = self.player_get_max_depot_items(cid);
        let locker_iid = self.items.insert(Item::new_single(ITEM_LOCKER1));
        let market_iid = self.items.insert(Item::new_single(ITEM_MARKET));
        let inbox_id = self.player_get_inbox(cid, true)?;
        let town_count = self.map.towns.len().max(1) as u32;
        let uni_iid = self.items.insert(Item::new_single(ITEM_DEPOT));

        let mut reg = std::mem::take(&mut self.container_registry);
        let mut locker = Container::new(locker_iid, locker_cap);
        locker.depot_locker_town_id = Some(depot_id);
        reg.register(locker);

        let mut uni = Container::new(uni_iid, town_count);
        uni.max_depot_items = max_items;
        reg.register(uni);

        Self::link_child_in_registry(&mut reg, &mut self.items, locker_iid, market_iid);
        Self::link_child_in_registry(&mut reg, &mut self.items, locker_iid, inbox_id);
        Self::link_child_in_registry(&mut reg, &mut self.items, locker_iid, uni_iid);

        let mut town_ids: Vec<u32> = self.map.towns.keys().copied().collect();
        town_ids.sort_unstable_by(|a, b| b.cmp(a));
        if town_ids.is_empty() {
            town_ids.push(depot_id);
        }
        self.container_registry = reg;
        for town_id in town_ids {
            let chest_id = self.player_get_depot_chest(cid, town_id, true)?;
            let mut reg = std::mem::take(&mut self.container_registry);
            Self::link_child_in_registry(&mut reg, &mut self.items, uni_iid, chest_id);
            self.container_registry = reg;
        }

        self.refresh_container_chain(uni_iid);
        self.refresh_container_chain(locker_iid);

        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.depot_lockers.insert(depot_id, locker_iid);
        }
        Some(locker_iid)
    }

    /// Re-parent inbox and depot chests when reopening an existing virtual locker.
    fn sync_depot_locker_contents(&mut self, cid: CreatureId, locker_id: ItemId) {
        use crate::formulas::DepotLockerStructure;
        // 772 lockers have a single depot chest at index 0 — no inbox/market to sync.
        if self.mechanics.profile.depot_locker_structure
            == DepotLockerStructure::ClassicDepotChest
        {
            return;
        }
        let inbox_id = match self.player_get_inbox(cid, false) {
            Some(id) => id,
            None => return,
        };
        let uni_id = {
            let Some(locker) = self.container_registry.get(locker_id) else {
                return;
            };
            locker.items.get(2).copied()
        };
        let Some(uni_id) = uni_id else {
            return;
        };

        let mut reg = std::mem::take(&mut self.container_registry);
        Self::ensure_child_in_container(&mut reg, &mut self.items, locker_id, inbox_id, 1);
        let town_ids: Vec<u32> = self
            .creatures
            .get(cid)
            .and_then(|k| match k {
                CreatureKind::Player(p) => Some(p.depot_chests.keys().copied().collect()),
                _ => None,
            })
            .unwrap_or_default();
        for town_id in town_ids {
            if let Some(chest_id) = self.player_get_depot_chest(cid, town_id, false) {
                Self::ensure_child_in_container(&mut reg, &mut self.items, uni_id, chest_id, usize::MAX);
            }
        }
        self.container_registry = reg;
        self.refresh_container_chain(uni_id);
        self.refresh_container_chain(locker_id);
    }

    fn link_child_in_registry(
        reg: &mut ContainerRegistry,
        items: &mut slotmap::SlotMap<ItemId, Item>,
        parent_id: ItemId,
        child_id: ItemId,
    ) {
        if let Some(parent) = reg.get_mut(parent_id) {
            if !parent.contains(child_id) {
                let _ = parent.add_item(child_id);
            }
        }
        if let Some(child) = reg.get_mut(child_id) {
            child.parent_container = Some(parent_id);
        }
        if let Some(item) = items.get_mut(child_id) {
            item.parent = Some(crate::cylinder::Cylinder::Container {
                item_id: parent_id,
                index: crate::cylinder::INDEX_WHEREEVER,
            });
        }
    }

    fn ensure_child_in_container(
        reg: &mut ContainerRegistry,
        items: &mut slotmap::SlotMap<ItemId, Item>,
        parent_id: ItemId,
        child_id: ItemId,
        slot: usize,
    ) {
        if let Some(parent) = reg.get(parent_id) {
            if parent.contains(child_id) {
                if let Some(child) = reg.get_mut(child_id) {
                    child.parent_container = Some(parent_id);
                }
                if let Some(item) = items.get_mut(child_id) {
                    item.parent = Some(crate::cylinder::Cylinder::Container {
                        item_id: parent_id,
                        index: crate::cylinder::INDEX_WHEREEVER,
                    });
                }
                return;
            }
        }
        if let Some(parent) = reg.get_mut(parent_id) {
            if slot == usize::MAX {
                let _ = parent.add_item(child_id);
            } else {
                let _ = parent.insert_item(slot, child_id);
            }
        }
        if let Some(child) = reg.get_mut(child_id) {
            child.parent_container = Some(parent_id);
        }
        if let Some(item) = items.get_mut(child_id) {
            item.parent = Some(crate::cylinder::Cylinder::Container {
                item_id: parent_id,
                index: crate::cylinder::INDEX_WHEREEVER,
            });
        }
    }

    pub(crate) fn container_capacity(&self, server_id: u16) -> u32 {
        self.items_db
            .items
            .get(&server_id)
            .and_then(|t| t.xml_attributes.get("containersize"))
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(20)
            .max(1)
    }

    /// Universal depot parent + limit for `DepotChest::queryAdd` — `depotchest.cpp`.
    pub(crate) fn depot_limit_holder(&self, depot_chest_id: ItemId) -> Option<(ItemId, u32)> {
        let chest = self.container_registry.get(depot_chest_id)?;
        if chest.container_type != ContainerType::Depot {
            return None;
        }
        let max = chest.max_depot_items;
        if let Some(parent_id) = chest.parent_container {
            return Some((parent_id, max));
        }
        Some((depot_chest_id, max))
    }

    /// C++ `DepotChest::queryAdd` holding count for moved item.
    pub(crate) fn depot_add_count_for_item(
        &self,
        depot_chest_id: ItemId,
        item_id: ItemId,
        count: u32,
    ) -> u32 {
        let Some(item) = self.items.get(item_id) else {
            return 1;
        };
        let stackable = self
            .items_db
            .items
            .get(&item.item_type)
            .is_some_and(|t| t.stackable());
        if stackable && u32::from(item.count) != count {
            return 1;
        }
        if self.item_depot_top_parent(item_id) != Some(depot_chest_id) {
            if self.items_db.is_container(item.item_type) {
                return self
                    .container_registry
                    .get(item_id)
                    .map(|c| c.total_item_count.saturating_add(1))
                    .unwrap_or(1);
            }
            return 1;
        }
        1
    }

    fn item_depot_top_parent(&self, item_id: ItemId) -> Option<ItemId> {
        let mut cur = item_id;
        loop {
            let parent = self.container_registry.get(cur)?.parent_container?;
            let parent_cont = self.container_registry.get(parent)?;
            if parent_cont.container_type == ContainerType::Depot {
                return Some(parent);
            }
            cur = parent;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfs_rust_common::Position;

    use crate::container::ContainerType;
    use crate::item_constants::ITEM_DEPOT;
    use crate::test_world::support::{insert_player, minimal_world, test_player};
    use crate::tile::{flags as tilestate, Tile, TileBody};
    use tfs_rust_common::enums::ZoneType;

    #[test]
    fn is_near_depot_box_detects_depot_tile_flag() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        world.map.insert_tile(
            pos,
            Tile::Normal(TileBody {
                ground: Some(100),

                ground_item: None,
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: tilestate::DEPOT,
                zone: ZoneType::Normal,
            }),
        );
        let cid = insert_player(&mut world, test_player("depot", pos));
        assert!(world.player_is_near_depot_box(cid));
    }

    #[test]
    fn depot_chest_lazy_create_and_owns() {
        let mut world = minimal_world();
        let pos = Position::new(50, 50, 7);
        let cid = insert_player(&mut world, test_player("depot", pos));
        let chest = world
            .player_get_depot_chest(cid, 1, true)
            .expect("depot chest");
        assert!(world.player_owns_depot_root(cid, chest));
        let cont = world.container_registry.get(chest).expect("registered");
        assert_eq!(cont.container_type, ContainerType::Depot);
        assert_eq!(
            world.items.get(chest).map(|i| i.item_type),
            Some(ITEM_DEPOT)
        );
    }

    #[test]
    fn player_set_last_depot_id_updates_runtime() {
        let mut world = minimal_world();
        let pos = Position::new(50, 50, 7);
        let cid = insert_player(&mut world, test_player("depot", pos));
        world.player_set_last_depot_id(cid, 5);
        assert_eq!(
            world.creatures.get(cid).and_then(|k| match k {
                CreatureKind::Player(p) => Some(p.last_depot_id),
                _ => None,
            }),
            Some(5)
        );
    }

    /// Regression: opening a depot locker after login must reuse the chest
    /// that `load_depot_table` already created with loaded items, not create
    /// a new empty chest that orphans the old one.
    #[test]
    fn classic_depot_locker_reuses_existing_chest() {
        let mut world = minimal_world();
        // Force 772 depot locker structure (single chest inside locker).
        world.mechanics.profile.depot_locker_structure =
            crate::formulas::DepotLockerStructure::ClassicDepotChest;
        let pos = Position::new(50, 50, 7);
        let cid = insert_player(&mut world, test_player("depot", pos));

        // Simulate login: create a depot chest and add an item to it.
        let chest = world
            .player_get_depot_chest(cid, 1, true)
            .expect("depot chest");
        let coin = world.items.insert(Item::new_single(2148));
        if let Some(cont) = world.container_registry.get_mut(chest) {
            let _ = cont.add_item(coin);
        }
        world.refresh_container_derived(chest);

        // Now open the depot locker (as the player would by clicking the depot).
        let locker = world
            .player_get_depot_locker(cid, 1)
            .expect("depot locker");

        // The locker should contain the SAME chest, not a new empty one.
        let locker_cont = world.container_registry.get(locker).expect("locker registered");
        assert_eq!(locker_cont.items.len(), 1);
        assert_eq!(locker_cont.items[0], chest);

        // The chest should still contain the coin.
        let chest_cont = world.container_registry.get(chest).expect("chest registered");
        assert_eq!(chest_cont.items.len(), 1);
        assert_eq!(chest_cont.items[0], coin);

        // player.depot_chests should still point to the original chest.
        let stored_chest = world
            .creatures
            .get(cid)
            .and_then(|k| match k {
                CreatureKind::Player(p) => p.depot_chests.get(&1).copied(),
                _ => None,
            })
            .expect("depot_chests entry");
        assert_eq!(stored_chest, chest);
    }
}
