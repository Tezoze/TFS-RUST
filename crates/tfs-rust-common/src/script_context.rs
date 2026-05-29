//! Script read context trait — shared by core and lua crates (no circular dependency).
//!
//! C++ reference: `LuaScriptInterface` read accessors resolving userdata IDs to game objects.

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
}

/// ID handle wrapper for creatures passed to Lua userdata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptCreatureRef(pub ScriptCreatureId);

/// ID handle wrapper for items passed to Lua userdata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptItemRef(pub ScriptItemId);

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
    fn get_item_data(&self, id: ScriptItemId) -> Option<ScriptItemData> {
        let _ = id;
        None
    }
}
