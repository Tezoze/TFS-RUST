//! Lua script read helpers — parent cylinders, container queries, item resolution.
//!
//! C++ reference: `luascript.cpp` item/container/player read accessors; `Item::getParent` / `getTopParent` — `item.cpp`.

use slotmap::Key;
use tfs_rust_common::{ScriptContainerData, ScriptCylinder, ScriptItemId};

use crate::container::ContainerIterator;
use crate::creature::CreatureKind;
use crate::cylinder::Cylinder;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};

impl GameWorld {
    /// O(1) resolve via `KeyData::from_ffi` — slotmap keys are reconstructable
    /// from their `as_ffi()` bits (slot index + generation). `.get()` validates
    /// the generation, returning `None` for stale/freed slots.
    pub(crate) fn resolve_item_u64(&self, id: u64) -> Option<ItemId> {
        let key = ItemId::from(slotmap::KeyData::from_ffi(id));
        if self.items.get(key).is_some() {
            Some(key)
        } else {
            None
        }
    }

    pub(crate) fn item_to_script_id(iid: ItemId) -> ScriptItemId {
        iid.data().as_ffi()
    }

    pub(crate) fn creature_to_script_id(cid: CreatureId) -> u64 {
        cid.data().as_ffi()
    }

    /// O(1) resolve — same approach as `resolve_item_u64`.
    pub(crate) fn resolve_creature_from_script(&self, id: u64) -> Option<CreatureId> {
        let key = CreatureId::from(slotmap::KeyData::from_ffi(id));
        if self.creatures.get(key).is_some() {
            Some(key)
        } else {
            None
        }
    }

    /// TFS `Item::getParent` cylinder — `item.cpp` / `luascript.cpp` `luaItemGetParent`.
    pub fn script_item_parent(&self, item_id: ItemId) -> Option<ScriptCylinder> {
        match self.items.get(item_id)?.parent? {
            Cylinder::Inventory { player_id, .. } => Some(ScriptCylinder::Player(
                Self::creature_to_script_id(player_id),
            )),
            Cylinder::Container {
                item_id: parent, ..
            } => Some(ScriptCylinder::Container(Self::item_to_script_id(parent))),
            Cylinder::Tile { pos } => Some(ScriptCylinder::Tile(pos)),
        }
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
        match self.items.get(item_id)?.parent {
            Some(Cylinder::Tile { pos }) => Some(pos),
            Some(Cylinder::Inventory { player_id, .. }) => {
                Some(self.creatures.get(player_id)?.position())
            }
            Some(Cylinder::Container {
                item_id: parent, ..
            }) => self.script_item_position(parent),
            None => None,
        }
    }

    pub fn script_is_registered_container(&self, item_id: ItemId) -> bool {
        self.container_registry.get(item_id).is_some()
    }

    pub fn script_container_data(&self, container_id: ItemId) -> Option<ScriptContainerData> {
        let cont = self.container_registry.get(container_id)?;
        // 772: `getItemHoldingCount` on a depot locker excludes the chest itself
        // (`moveuse.cc:640`: `CountObjects(Con) - 1`). TFS 1098 includes it.
        let holding_count = if cont.depot_locker_town_id.is_some()
            && matches!(
                self.mechanics.profile.depot_locker_structure,
                crate::formulas::DepotLockerStructure::ClassicDepotChest
            ) {
            cont.total_item_count.saturating_sub(1)
        } else {
            cont.total_item_count
        };
        Some(ScriptContainerData {
            size: cont.size() as u32,
            capacity: cont.capacity,
            empty_slots: cont.available_slots(),
            item_holding_count: holding_count,
            corpse_owner: self
                .items
                .get(container_id)
                .map(|i| {
                    i.attributes
                        .as_deref()
                        .map(|a| a.get_corpse_owner())
                        .unwrap_or(0)
                })
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
            count =
                count.saturating_add(self.item_count_for_type_script(child, item_type, sub_type));
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

    pub fn script_player_container_id(&self, cid: CreatureId, container_id: ItemId) -> Option<u8> {
        self.container_registry
            .get_cid_for_container(cid, container_id)
    }

    pub fn script_player_container_by_cid(
        &self,
        cid: CreatureId,
        client_cid: u8,
    ) -> Option<ItemId> {
        self.container_registry
            .get_container_by_cid(cid, client_cid)
    }

    pub fn script_player_container_index(&self, cid: CreatureId, client_cid: u8) -> Option<u16> {
        self.container_registry
            .get_container_first_index(cid, client_cid)
    }

    /// Resolve parent [`Cylinder`] for Lua `item:moveTo` / `item:remove` / decay apply.
    ///
    /// O(1) via [`Item::parent`] (772 `TObject::Container` outcome). Hubs maintain the field.
    /// When unset (legacy loot/corpse paths), fall back to a registry/slot/map scan.
    pub fn resolve_item_parent_cylinder(&self, item_id: ItemId) -> Option<Cylinder> {
        if let Some(p) = self.items.get(item_id).and_then(|i| i.parent) {
            return Some(p);
        }
        self.discover_item_parent(item_id)
    }

    /// Locate which cylinder currently holds `item_id` when [`Item::parent`] is stale/None.
    pub(crate) fn discover_item_parent(&self, item_id: ItemId) -> Option<Cylinder> {
        for parent_id in self.container_registry.registered_container_ids() {
            if self
                .container_registry
                .get(parent_id)
                .is_some_and(|c| c.items.iter().any(|&id| id == item_id))
            {
                return Some(Cylinder::Container {
                    item_id: parent_id,
                    index: crate::cylinder::INDEX_WHEREEVER,
                });
            }
        }
        for (cid, kind) in self.creatures.iter() {
            let CreatureKind::Player(p) = kind else {
                continue;
            };
            for (idx, slot_item) in p.equipment_slots.iter().enumerate() {
                if *slot_item == Some(item_id) {
                    return Some(Cylinder::Inventory {
                        player_id: cid,
                        slot: (idx as u8).saturating_add(1),
                    });
                }
            }
        }
        self.map
            .find_item_position(item_id)
            .map(|pos| Cylinder::Tile { pos })
    }

    /// Like [`Self::resolve_item_parent_cylinder`], but writes back a discovered parent.
    pub(crate) fn resolve_or_repair_item_parent(&mut self, item_id: ItemId) -> Option<Cylinder> {
        if let Some(p) = self.items.get(item_id).and_then(|i| i.parent) {
            return Some(p);
        }
        let discovered = self.discover_item_parent(item_id)?;
        if let Some(item) = self.items.get_mut(item_id) {
            item.parent = Some(discovered);
        }
        Some(discovered)
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
