//! Script read context trait — shared by core and lua crates (no circular dependency).
//!
//! C++ reference: `LuaScriptInterface` read accessors resolving userdata IDs to game objects.

use crate::Position;

/// Lua / script creature handle (`slotmap` key bits as u64).
pub type ScriptCreatureId = u64;

/// Lua / script item handle (`slotmap` key bits as u64).
pub type ScriptItemId = u64;

/// Creature fields for script read APIs (`Creature:getName`, …).
#[derive(Clone, Debug)]
pub struct ScriptCreatureData {
    pub name: String,
    pub guid: u32,
}

/// Item fields for script read APIs (`Item:getId` / `getType` / …).
#[derive(Clone, Debug)]
pub struct ScriptItemData {
    pub item_type: u16,
    pub count: u16,
    pub weight: u32,
    pub name: String,
    pub action_id: u16,
    pub unique_id: u32,
    pub is_store_item: bool,
}

/// ID handle wrapper for creatures passed to Lua userdata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptCreatureRef(pub ScriptCreatureId);

/// ID handle wrapper for items passed to Lua userdata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptItemRef(pub ScriptItemId);

/// Cylinder reference for `Item:getParent` / `getTopParent` — `luascript.cpp` `pushCylinder`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptCylinder {
    Player(ScriptCreatureId),
    Container(ScriptItemId),
    Tile(Position),
}

/// Container read snapshot for Lua `Container:*` methods.
#[derive(Clone, Debug)]
pub struct ScriptContainerData {
    pub size: u32,
    pub capacity: u32,
    pub empty_slots: u32,
    pub item_holding_count: u32,
    pub corpse_owner: u32,
}

/// Read-only game object resolution during script execution.
///
/// Implemented by `GameWorld` in `tfs-rust-core`. Lua userdata resolves handles via
/// thread-local scope set by `tfs-rust-lua::with_lua_context`.
pub trait ScriptContext {
    fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData>;
    fn get_item(&self, id: ScriptItemId) -> Option<ScriptItemRef>;
    fn get_config_string(&self, key: &str) -> Option<String>;
    /// `Player` equipment slot — raw item id (`slotmap` key bits as u64).
    fn get_player_slot_item_id(
        &self,
        creature_id: ScriptCreatureId,
        slot: u8,
    ) -> Option<ScriptItemId> {
        let _ = (creature_id, slot);
        None
    }
    fn get_player_capacity(&self, creature_id: ScriptCreatureId) -> Option<u32> {
        let _ = creature_id;
        None
    }
    fn get_player_free_capacity(&self, creature_id: ScriptCreatureId) -> Option<u32> {
        let _ = creature_id;
        None
    }
    fn get_player_item_type_count(
        &self,
        creature_id: ScriptCreatureId,
        item_id: u16,
        sub_type: i32,
    ) -> Option<u32> {
        let _ = (creature_id, item_id, sub_type);
        None
    }
    fn get_item_data(&self, id: ScriptItemId) -> Option<ScriptItemData> {
        let _ = id;
        None
    }

    /// Resolve server item type id from name — `Item::items.getItemIdByName`.
    fn get_item_type_id_by_name(&self, name: &str) -> Option<u16> {
        let _ = name;
        None
    }

    /// TFS `player:getItemById` — `Game::findItemOfType(player, …)`.
    fn find_player_item_by_type(
        &self,
        creature_id: ScriptCreatureId,
        item_id: u16,
        depth_search: bool,
        sub_type: i32,
    ) -> Option<ScriptItemId> {
        let _ = (creature_id, item_id, depth_search, sub_type);
        None
    }

    fn is_registered_container(&self, item_id: ScriptItemId) -> bool {
        let _ = item_id;
        false
    }

    fn get_container_data(&self, item_id: ScriptItemId) -> Option<ScriptContainerData> {
        let _ = item_id;
        None
    }

    fn get_container_item_at(&self, container_id: ScriptItemId, index: u32) -> Option<ScriptItemId> {
        let _ = (container_id, index);
        None
    }

    fn get_container_items(&self, container_id: ScriptItemId) -> Vec<ScriptItemId> {
        let _ = container_id;
        Vec::new()
    }

    fn container_has_item(&self, container_id: ScriptItemId, item_id: ScriptItemId) -> bool {
        let _ = (container_id, item_id);
        false
    }

    fn get_container_item_count_by_id(
        &self,
        container_id: ScriptItemId,
        item_type: u16,
        sub_type: i32,
    ) -> u32 {
        let _ = (container_id, item_type, sub_type);
        0
    }

    fn get_player_container_id(
        &self,
        creature_id: ScriptCreatureId,
        container_id: ScriptItemId,
    ) -> Option<u8> {
        let _ = (creature_id, container_id);
        None
    }

    fn get_player_container_by_cid(
        &self,
        creature_id: ScriptCreatureId,
        client_cid: u8,
    ) -> Option<ScriptItemId> {
        let _ = (creature_id, client_cid);
        None
    }

    fn get_player_container_index(
        &self,
        creature_id: ScriptCreatureId,
        client_cid: u8,
    ) -> Option<u16> {
        let _ = (creature_id, client_cid);
        None
    }

    fn get_item_parent(&self, item_id: ScriptItemId) -> Option<ScriptCylinder> {
        let _ = item_id;
        None
    }

    fn get_item_top_parent(&self, item_id: ScriptItemId) -> Option<ScriptCylinder> {
        let _ = item_id;
        None
    }

    fn get_item_position(&self, item_id: ScriptItemId) -> Option<Position> {
        let _ = item_id;
        None
    }
}
