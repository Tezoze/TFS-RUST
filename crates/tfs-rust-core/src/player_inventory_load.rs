//! Load `player_items` / `player_storeinboxitems` into `Player::equipment_slots` and `ContainerRegistry`.
// C++ reference: `src/iologindata.cpp` `IOLoginData::loadPlayer` (inventory blocks).

use std::collections::HashMap;

use tfs_rust_db::ItemRecord;

use crate::container::{Container, ContainerRegistry};
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::slot_to_array_index;
use crate::item::Item;

impl GameWorld {
    /// Hydrate runtime items from DB rows (`loadItems` + placement loop — `iologindata.cpp`).
    pub fn hydrate_player_inventory_from_db(
        &mut self,
        cid: CreatureId,
        inventory: &[ItemRecord],
        store_inbox: &[ItemRecord],
    ) {
        if self.creatures.get(cid).is_none() {
            return;
        }
        self.load_one_item_table(cid, inventory);
        self.load_store_inbox_table(cid, store_inbox);
        self.recompute_player_inventory_weight(cid);
        self.update_player_items_light(cid, true);
        self.change_creature_light(cid);
    }

    fn container_capacity(&self, server_id: u16) -> u32 {
        self.items_db
            .items
            .get(&server_id)
            .and_then(|t| t.xml_attributes.get("containersize"))
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(20)
            .max(1)
    }

    pub(crate) fn ensure_container_registered(
        &mut self,
        registry: &mut ContainerRegistry,
        container_item_id: ItemId,
    ) {
        if registry.get(container_item_id).is_some() {
            return;
        }
        let Some(item) = self.items.get(container_item_id) else {
            return;
        };
        let cap = self.container_capacity(item.item_type);
        let c = Container::new(container_item_id, cap);
        registry.register(c);
    }

    /// Ensure a container item has a registry entry and recomputed `total_weight` / chain — C++ `Container::totalWeight` after equip.
    pub(crate) fn hydrate_container_if_needed(&mut self, item_id: ItemId) {
        let Some(item) = self.items.get(item_id) else {
            return;
        };
        if !self.items_db.is_container(item.item_type) {
            return;
        }
        let mut reg = std::mem::take(&mut self.container_registry);
        self.ensure_container_registered(&mut reg, item_id);
        self.container_registry = reg;
        self.refresh_container_chain(item_id);
    }

    fn load_one_item_table(&mut self, cid: CreatureId, rows: &[ItemRecord]) {
        if rows.is_empty() {
            return;
        }
        let mut sid_map: HashMap<i32, ItemId> = HashMap::new();
        let mut sorted: Vec<&ItemRecord> = rows.iter().collect();
        sorted.sort_by_key(|r| r.sid);
        sorted.reverse();

        for rec in sorted.iter().copied() {
            let item = match Item::from_player_item_record(ItemId::default(), rec, &self.items_db) {
                Ok(i) => i,
                Err(e) => {
                    tracing::warn!(?e, "skip item row sid={}", rec.sid);
                    continue;
                }
            };
            let iid = self.items.insert(item);
            sid_map.insert(rec.sid, iid);
        }

        let mut registry = std::mem::take(&mut self.container_registry);

        for rec in sorted.iter().copied() {
            let Some(&item_id) = sid_map.get(&rec.sid) else {
                continue;
            };
            let pid = rec.pid;
            if (1..=10).contains(&pid) {
                if let Some(CreatureKind::Player(player)) = self.creatures.get_mut(cid) {
                    if let Some(idx) = slot_to_array_index(pid as u8) {
                        player.equipment_slots[idx] = Some(item_id);
                    }
                }
            } else if let Some(&parent_id) = sid_map.get(&pid) {
                let parent_type = self.items.get(parent_id).map(|i| i.item_type).unwrap_or(0);
                if self.items_db.is_container(parent_type) {
                    self.ensure_container_registered(&mut registry, parent_id);
                    if let Some(cont) = registry.get_mut(parent_id) {
                        let _ = cont.add_item(item_id);
                    }
                    if self.items_db.is_container(self.items.get(item_id).map(|i| i.item_type).unwrap_or(0)) {
                        self.ensure_container_registered(&mut registry, item_id);
                        if let Some(ch) = registry.get_mut(item_id) {
                            ch.parent_container = Some(parent_id);
                        }
                    }
                }
            }
        }

        self.container_registry = registry;
        let ids: Vec<ItemId> = self.container_registry.registered_container_ids().collect();
        for id in ids {
            self.refresh_container_derived(id);
        }
    }

    /// `player_storeinboxitems` — `iologindata.cpp` ~508–533.
    fn load_store_inbox_table(&mut self, cid: CreatureId, rows: &[ItemRecord]) {
        if rows.is_empty() {
            return;
        }
        let anchor_rec = rows
            .iter()
            .find(|r| (0..100).contains(&r.pid))
            .or_else(|| rows.first());

        let mut sid_map: HashMap<i32, ItemId> = HashMap::new();
        let mut sorted: Vec<&ItemRecord> = rows.iter().collect();
        sorted.sort_by_key(|r| r.sid);
        sorted.reverse();

        for rec in sorted.iter().copied() {
            let item = match Item::from_player_item_record(ItemId::default(), rec, &self.items_db) {
                Ok(i) => i,
                Err(e) => {
                    tracing::warn!(?e, "skip store inbox sid={}", rec.sid);
                    continue;
                }
            };
            let iid = self.items.insert(item);
            sid_map.insert(rec.sid, iid);
        }

        if let Some(rec) = anchor_rec {
            if let Some(&iid) = sid_map.get(&rec.sid) {
                if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                    p.equipment_slots[10] = Some(iid);
                }
            }
        }

        let mut registry = std::mem::take(&mut self.container_registry);

        let store_root = self
            .creatures
            .get(cid)
            .and_then(|k| match k {
                CreatureKind::Player(p) => p.equipment_slots[10],
                _ => None,
            });

        for rec in sorted.iter().copied() {
            let Some(&item_id) = sid_map.get(&rec.sid) else {
                continue;
            };
            let pid = rec.pid;
            if (0..100).contains(&pid) {
                if let Some(root) = store_root {
                    if item_id == root {
                        continue;
                    }
                    if self.items_db.is_container(self.items.get(root).map(|i| i.item_type).unwrap_or(0)) {
                        self.ensure_container_registered(&mut registry, root);
                        if let Some(cont) = registry.get_mut(root) {
                            let _ = cont.add_item(item_id);
                        }
                        if self.items_db.is_container(self.items.get(item_id).map(|i| i.item_type).unwrap_or(0)) {
                            self.ensure_container_registered(&mut registry, item_id);
                            if let Some(ch) = registry.get_mut(item_id) {
                                ch.parent_container = Some(root);
                            }
                        }
                    }
                }
            } else if let Some(&parent_id) = sid_map.get(&pid) {
                let parent_type = self.items.get(parent_id).map(|i| i.item_type).unwrap_or(0);
                if self.items_db.is_container(parent_type) {
                    self.ensure_container_registered(&mut registry, parent_id);
                    if let Some(cont) = registry.get_mut(parent_id) {
                        let _ = cont.add_item(item_id);
                    }
                    if self.items_db.is_container(self.items.get(item_id).map(|i| i.item_type).unwrap_or(0)) {
                        self.ensure_container_registered(&mut registry, item_id);
                        if let Some(ch) = registry.get_mut(item_id) {
                            ch.parent_container = Some(parent_id);
                        }
                    }
                }
            }
        }

        self.container_registry = registry;
        let ids: Vec<ItemId> = self.container_registry.registered_container_ids().collect();
        for id in ids {
            self.refresh_container_derived(id);
        }
    }

    /// TFS `Player::updateInventoryWeight` — `player.cpp` ~419–436.
    pub fn recompute_player_inventory_weight(&mut self, cid: CreatureId) {
        let Some(CreatureKind::Player(player)) = self.creatures.get(cid) else {
            return;
        };
        let slots = player.equipment_slots;
        let mut total = 0u32;
        for i in 0..11 {
            if let Some(iid) = slots[i] {
                total = total.saturating_add(self.item_recursive_weight_oz(iid));
            }
        }
        if let Some(CreatureKind::Player(player)) = self.creatures.get_mut(cid) {
            player.inventory_weight = total;
        }
    }

    pub(crate) fn item_recursive_weight_oz(&self, id: ItemId) -> u32 {
        let Some(item) = self.items.get(id) else {
            return 0;
        };
        let it = self.items_db.items.get(&item.item_type);
        let tw = it.map(|t| t.weight).unwrap_or(0);
        let stack = it.map(|t| t.stackable()).unwrap_or(false);
        let mut w = item.total_weight_oz(tw, stack);
        // C++ `Container::getWeight` — only container item types aggregate child weight (`item.cpp` / `container.cpp`).
        if !self.items_db.is_container(item.item_type) {
            return w;
        }
        if let Some(c) = self.container_registry.get(id) {
            for &ch in &c.items {
                w = w.saturating_add(self.item_recursive_weight_oz(ch));
            }
        }
        w
    }
}
