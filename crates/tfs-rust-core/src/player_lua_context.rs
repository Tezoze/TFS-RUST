//! Lua script read helpers — parent cylinders, container queries, item resolution.
//!
//! C++ reference: `luascript.cpp` item/container/player read accessors; `Item::getParent` / `getTopParent` — `item.cpp`.

use tfs_rust_common::{ScriptContainerData, ScriptCylinder, ScriptItemId};
use slotmap::Key;

use crate::container::ContainerIterator;
use crate::creature::CreatureKind;
use crate::cylinder::{Cylinder, INDEX_WHEREEVER};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::{PLAYER_INVENTORY_SLOT_FIRST, PLAYER_INVENTORY_SLOT_LAST};

impl GameWorld {
    pub(crate) fn resolve_item_u64(&self, id: u64) -> Option<ItemId> {
        self.items
            .iter()
            .find(|(item_id, _)| item_id.data().as_ffi() == id)
            .map(|(k, _)| k)
    }

    pub(crate) fn item_to_script_id(iid: ItemId) -> ScriptItemId {
        iid.data().as_ffi()
    }

    pub(crate) fn creature_to_script_id(cid: CreatureId) -> u64 {
        cid.data().as_ffi()
    }

    pub(crate) fn resolve_creature_from_script(&self, id: u64) -> Option<CreatureId> {
        self.creatures
            .iter()
            .find(|(k, _)| k.data().as_ffi() == id)
            .map(|(k, _)| k)
    }

    /// TFS `Item::getParent` cylinder — `item.cpp` / `luascript.cpp` `luaItemGetParent`.
    pub fn script_item_parent(&self, item_id: ItemId) -> Option<ScriptCylinder> {
        for (cid, kind) in self.creatures.iter() {
            let CreatureKind::Player(p) = kind else {
                continue;
            };
            for slot in PLAYER_INVENTORY_SLOT_FIRST..=PLAYER_INVENTORY_SLOT_LAST {
                let idx = (slot - 1) as usize;
                if p.equipment_slots[idx] == Some(item_id) {
                    return Some(ScriptCylinder::Player(Self::creature_to_script_id(cid)));
                }
            }
        }
        if let Some(parent) = self.parent_container_of(item_id) {
            return Some(ScriptCylinder::Container(Self::item_to_script_id(parent)));
        }
        if let Some(pos) = self.map.find_item_position(item_id) {
            return Some(ScriptCylinder::Tile(pos));
        }
        None
    }

    /// TFS `Item::getTopParent` — `item.cpp` ~283–299.
    pub fn script_item_top_parent(&self, item_id: ItemId) -> Option<ScriptCylinder> {
        let mut current = item_id;
        loop {
            let parent = self.script_item_parent(current)?;
            match parent {
                ScriptCylinder::Player(_) => return Some(parent),
                ScriptCylinder::Tile(_) => return Some(parent),
                ScriptCylinder::Container(container_id) => {
                    let parent_iid = self.resolve_item_u64(container_id)?;
                    if self.script_item_parent(parent_iid).is_some() {
                        current = parent_iid;
                    } else {
                        return Some(parent);
                    }
                }
            }
        }
    }

    /// TFS `Item::getPosition` — position of top parent tile or owning player.
    pub fn script_item_position(&self, item_id: ItemId) -> Option<tfs_rust_common::Position> {
        if let Some(pos) = self.map.find_item_position(item_id) {
            return Some(pos);
        }
        if let Some(top) = self.script_item_top_parent(item_id) {
            match top {
                ScriptCylinder::Player(cid_u64) => {
                    let cid = self.resolve_creature_from_script(cid_u64)?;
                    return Some(self.creatures.get(cid)?.position());
                }
                ScriptCylinder::Tile(pos) => return Some(pos),
                ScriptCylinder::Container(container_id) => {
                    let root = self.resolve_item_u64(container_id)?;
                    return self.script_item_position(root);
                }
            }
        }
        None
    }

    pub fn script_is_registered_container(&self, item_id: ItemId) -> bool {
        self.container_registry.get(item_id).is_some()
    }

    pub fn script_container_data(&self, container_id: ItemId) -> Option<ScriptContainerData> {
        let cont = self.container_registry.get(container_id)?;
        Some(ScriptContainerData {
            size: cont.size() as u32,
            capacity: cont.capacity,
            empty_slots: cont.available_slots(),
            item_holding_count: cont.total_item_count,
            corpse_owner: self
                .items
                .get(container_id)
                .map(|i| i.attributes.as_deref().map(|a| a.get_corpse_owner()).unwrap_or(0))
                .unwrap_or(0),
        })
    }

    pub fn script_container_item_at(&self, container_id: ItemId, index: u32) -> Option<ItemId> {
        self.container_registry
            .get(container_id)
            .and_then(|c| c.get_item(index as usize))
    }

    pub fn script_container_items(&self, container_id: ItemId) -> Vec<ItemId> {
        self.container_registry
            .get(container_id)
            .map(|c| c.items.clone())
            .unwrap_or_default()
    }

    pub fn script_container_has_item(&self, container_id: ItemId, item_id: ItemId) -> bool {
        self.container_registry
            .get(container_id)
            .is_some_and(|c| c.is_holding_item(&self.container_registry, item_id))
    }

    pub fn script_container_item_count_by_id(
        &self,
        container_id: ItemId,
        item_type: u16,
        sub_type: i32,
    ) -> u32 {
        let mut count = 0u32;
        for child in ContainerIterator::new(&self.container_registry, container_id) {
            count = count.saturating_add(self.item_count_for_type_script(child, item_type, sub_type));
        }
        count
    }

    fn item_count_for_type_script(&self, iid: ItemId, item_type: u16, sub_type: i32) -> u32 {
        let Some(item) = self.items.get(iid) else {
            return 0;
        };
        if item.item_type != item_type {
            return 0;
        }
        let Some(it) = self.items_db.items.get(&item_type) else {
            return 0;
        };
        item.count_by_type(it, sub_type)
    }

    pub fn script_player_container_id(
        &self,
        cid: CreatureId,
        container_id: ItemId,
    ) -> Option<u8> {
        self.container_registry.get_cid_for_container(cid, container_id)
    }

    pub fn script_player_container_by_cid(
        &self,
        cid: CreatureId,
        client_cid: u8,
    ) -> Option<ItemId> {
        self.container_registry.get_container_by_cid(cid, client_cid)
    }

    pub fn script_player_container_index(
        &self,
        cid: CreatureId,
        client_cid: u8,
    ) -> Option<u16> {
        self.container_registry.get_container_first_index(cid, client_cid)
    }

    /// Resolve parent [`Cylinder`] for Lua `item:moveTo` / `item:remove`.
    pub fn resolve_item_parent_cylinder(&self, item_id: ItemId) -> Option<Cylinder> {
        for (cid, kind) in self.creatures.iter() {
            let CreatureKind::Player(p) = kind else {
                continue;
            };
            for slot in PLAYER_INVENTORY_SLOT_FIRST..=PLAYER_INVENTORY_SLOT_LAST {
                let idx = (slot - 1) as usize;
                if p.equipment_slots[idx] == Some(item_id) {
                    return Some(Cylinder::Inventory {
                        player_id: cid,
                        slot,
                    });
                }
            }
        }
        if let Some(parent) = self.parent_container_of(item_id) {
            return Some(Cylinder::Container {
                item_id: parent,
                index: INDEX_WHEREEVER,
            });
        }
        if let Some(pos) = self.map.find_item_position(item_id) {
            return Some(Cylinder::Tile { pos });
        }
        None
    }

    pub fn item_type_id_by_name(&self, name: &str) -> Option<u16> {
        let needle = name.to_ascii_lowercase();
        self.items_db
            .items
            .values()
            .find(|it| it.name.eq_ignore_ascii_case(name) || it.name.to_ascii_lowercase() == needle)
            .map(|it| it.server_id)
    }
}
