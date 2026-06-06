//! Lua `ScriptContext` bridge over live `GameWorld` state.
//!
//! - TFS script API surface — `luascript.cpp` / `game.cpp`.

use slotmap::Key;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;

impl tfs_rust_common::ScriptContext for GameWorld {
    fn get_creature(&self, id: tfs_rust_common::ScriptCreatureId) -> Option<tfs_rust_common::ScriptCreatureData> {
        self.creatures.iter().find_map(|(cid, k)| {
            if cid.data().as_ffi() != id {
                return None;
            }
            Some(match k {
            CreatureKind::Player(p) => Some(tfs_rust_common::ScriptCreatureData {
                name: p.base.name.clone(),
                guid: p.guid,
            }),
            CreatureKind::Monster(m) => Some(tfs_rust_common::ScriptCreatureData {
                name: m.base.name.clone(),
                guid: 0, // Monsters don't have GUIDs
            }),
            CreatureKind::Npc(n) => Some(tfs_rust_common::ScriptCreatureData {
                name: n.base.name.clone(),
                guid: 0, // NPCs don't have GUIDs
            }),
            })
        }).flatten()
    }

    fn get_item(&self, id: tfs_rust_common::ScriptItemId) -> Option<tfs_rust_common::ScriptItemRef> {
        self.items
            .iter()
            .find(|(item_id, _)| item_id.data().as_ffi() == id)
            .map(|_| tfs_rust_common::ScriptItemRef(id))
    }

    fn get_config_string(&self, key: &str) -> Option<String> {
        self.config.get_string(key).ok()
    }

    fn get_player_slot_item_id(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
        slot: u8,
    ) -> Option<tfs_rust_common::ScriptItemId> {
        let cid = self
            .creatures
            .iter()
            .find(|(k, _)| k.data().as_ffi() == creature_id)
            .map(|(k, _)| k)?;
        self.get_player_inventory_item(cid, slot)
            .map(|i| i.data().as_ffi())
    }

    fn get_player_capacity(&self, creature_id: tfs_rust_common::ScriptCreatureId) -> Option<u32> {
        let _cid = self
            .creatures
            .iter()
            .find(|(k, _)| k.data().as_ffi() == creature_id)
            .map(|(k, _)| k)?;
        self.player_capacity_u32(_cid)
    }

    fn get_player_free_capacity(&self, creature_id: tfs_rust_common::ScriptCreatureId) -> Option<u32> {
        let _cid = self
            .creatures
            .iter()
            .find(|(k, _)| k.data().as_ffi() == creature_id)
            .map(|(k, _)| k)?;
        self.player_free_capacity_u32(_cid)
    }

    fn get_player_item_type_count(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
        item_id: u16,
        sub_type: i32,
    ) -> Option<u32> {
        let cid = self
            .creatures
            .iter()
            .find(|(k, _)| k.data().as_ffi() == creature_id)
            .map(|(k, _)| k)?;
        Some(self.player_get_item_type_count(cid, item_id, sub_type))
    }

    fn get_item_data(&self, id: tfs_rust_common::ScriptItemId) -> Option<tfs_rust_common::ScriptItemData> {
        let iid = self
            .items
            .iter()
            .find(|(item_id, _)| item_id.data().as_ffi() == id)
            .map(|(k, _)| k)?;
        let item = self.items.get(iid)?;
        let it = self.items_db.items.get(&item.item_type);
        let tw = it.map(|t| t.weight).unwrap_or(0);
        let stack = it.map(|t| t.stackable()).unwrap_or(false);
        let w = item.total_weight_oz(tw, stack);
        Some(tfs_rust_common::ScriptItemData {
            item_type: item.item_type,
            count: item.count,
            weight: w,
            name: it.map(|t| t.name.clone()).unwrap_or_default(),
            action_id: item.action_id(),
            unique_id: u32::from(item.unique_id()),
            is_store_item: item.is_store_item(),
        })
    }

    fn get_item_type_id_by_name(&self, name: &str) -> Option<u16> {
        self.item_type_id_by_name(name)
    }

    fn find_player_item_by_type(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
        item_id: u16,
        depth_search: bool,
        sub_type: i32,
    ) -> Option<tfs_rust_common::ScriptItemId> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        self.find_item_of_type(cid, item_id, depth_search, sub_type)
            .map(GameWorld::item_to_script_id)
    }

    fn is_registered_container(&self, item_id: tfs_rust_common::ScriptItemId) -> bool {
        self.resolve_item_u64(item_id)
            .is_some_and(|i| self.script_is_registered_container(i))
    }

    fn get_container_data(
        &self,
        item_id: tfs_rust_common::ScriptItemId,
    ) -> Option<tfs_rust_common::ScriptContainerData> {
        let iid = self.resolve_item_u64(item_id)?;
        self.script_container_data(iid)
    }

    fn get_container_item_at(
        &self,
        container_id: tfs_rust_common::ScriptItemId,
        index: u32,
    ) -> Option<tfs_rust_common::ScriptItemId> {
        let cid = self.resolve_item_u64(container_id)?;
        self.script_container_item_at(cid, index)
            .map(GameWorld::item_to_script_id)
    }

    fn get_container_items(&self, container_id: tfs_rust_common::ScriptItemId) -> Vec<tfs_rust_common::ScriptItemId> {
        let Some(root) = self.resolve_item_u64(container_id) else {
            return Vec::new();
        };
        self.script_container_items(root)
            .into_iter()
            .map(GameWorld::item_to_script_id)
            .collect()
    }

    fn container_has_item(
        &self,
        container_id: tfs_rust_common::ScriptItemId,
        item_id: tfs_rust_common::ScriptItemId,
    ) -> bool {
        let (Some(root), Some(item)) = (self.resolve_item_u64(container_id), self.resolve_item_u64(item_id)) else {
            return false;
        };
        self.script_container_has_item(root, item)
    }

    fn get_container_item_count_by_id(
        &self,
        container_id: tfs_rust_common::ScriptItemId,
        item_type: u16,
        sub_type: i32,
    ) -> u32 {
        let Some(root) = self.resolve_item_u64(container_id) else {
            return 0;
        };
        self.script_container_item_count_by_id(root, item_type, sub_type)
    }

    fn get_player_container_id(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
        container_id: tfs_rust_common::ScriptItemId,
    ) -> Option<u8> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        let root = self.resolve_item_u64(container_id)?;
        self.script_player_container_id(cid, root)
    }

    fn get_player_container_by_cid(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
        client_cid: u8,
    ) -> Option<tfs_rust_common::ScriptItemId> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        self.script_player_container_by_cid(cid, client_cid)
            .map(GameWorld::item_to_script_id)
    }

    fn get_player_container_index(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
        client_cid: u8,
    ) -> Option<u16> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        self.script_player_container_index(cid, client_cid)
    }

    fn get_item_parent(&self, item_id: tfs_rust_common::ScriptItemId) -> Option<tfs_rust_common::ScriptCylinder> {
        let iid = self.resolve_item_u64(item_id)?;
        self.script_item_parent(iid)
    }

    fn get_item_top_parent(&self, item_id: tfs_rust_common::ScriptItemId) -> Option<tfs_rust_common::ScriptCylinder> {
        let iid = self.resolve_item_u64(item_id)?;
        self.script_item_top_parent(iid)
    }

    fn get_item_position(
        &self,
        item_id: tfs_rust_common::ScriptItemId,
    ) -> Option<tfs_rust_common::Position> {
        let iid = self.resolve_item_u64(item_id)?;
        self.script_item_position(iid)
    }
}
