//! Lua runtime and VM management.
//!
//! This module provides the LuaRuntime struct which owns the mlua::Lua VM
//! and manages script registry and global function registration.

use mlua::Lua;
use std::collections::HashMap;
use std::path::Path;

use crate::context::CreatureRef;
use crate::userdata::{register_creature_metatable, register_item_metatable};

/// Wrapper for mlua::RegistryKey — !Send, must stay on game thread.
#[derive(Debug)]
pub struct CallbackRef(mlua::RegistryKey);

/// Lua runtime owning the VM and script registry.
///
/// This is !Send by design and must live exclusively on the game thread.
pub struct LuaRuntime {
    lua: Lua,
    script_registry: HashMap<String, ()>,
}

impl LuaRuntime {
    /// Create a new Lua runtime with minimal global functions registered.
    ///
    /// # Errors
    ///
    /// Returns an error if VM initialization or lib loading fails.
    pub fn new() -> Result<Self, LuaError> {
        let lua = Lua::new();

        // Register minimal global functions via RegisterLuaFunctions
        let registrar = MinimalGlobalFunctions;
        registrar.register_functions(&lua).map_err(LuaError::Registration)?;

        register_creature_metatable(&lua).map_err(LuaError::Registration)?;
        register_item_metatable(&lua).map_err(LuaError::Registration)?;

        // Load data/lib/*.lua files (fatal if any fail)
        // TODO: Implement lib loading after we have a data directory path

        Ok(Self {
            lua,
            script_registry: HashMap::new(),
        })
    }

    /// Load and compile a Lua script file.
    ///
    /// # Errors
    ///
    /// Returns an error on syntax failure (non-fatal for script loading).
    pub fn load_script(&mut self, path: &str) -> Result<CallbackRef, LuaError> {
        let full_path = Path::new(path);
        let chunk = std::fs::read_to_string(full_path)
            .map_err(|e| LuaError::ScriptIo(full_path.display().to_string(), e.to_string()))?;
        self.lua
            .load(&chunk)
            .set_name(path)
            .exec()
            .map_err(LuaError::Init)?;

        let key = self.lua.create_registry_value(true)?;
        Ok(CallbackRef(key))
    }

    pub fn register_callback(
        &mut self,
        callback_key: String,
        global_function_name: &str,
    ) -> Result<CallbackRef, LuaError> {
        let globals = self.lua.globals();
        let function: mlua::Function = globals
            .get(global_function_name)
            .map_err(|_| LuaError::MissingFunction(global_function_name.to_string()))?;
        let registry_key = self.lua.create_registry_value(function)?;
        let callback = CallbackRef(registry_key);
        self.script_registry.insert(callback_key, ());
        Ok(callback)
    }

    pub fn call_creature_callback(
        &self,
        callback: &CallbackRef,
        creature: crate::context::CreatureId,
    ) -> Result<bool, LuaError> {
        let function: mlua::Function = self
            .lua
            .registry_value(&callback.0)
            .map_err(LuaError::Init)?;
        let player = self
            .lua
            .create_userdata(CreatureRef(creature))
            .map_err(LuaError::Init)?;
        function.call::<bool>(player).map_err(LuaError::Init)
    }

    /// Register `TableName.methodName` from a loaded script (e.g. `Player.onInventoryUpdate`).
    pub fn register_table_method_callback(
        &mut self,
        callback_key: String,
        table_name: &str,
        method_name: &str,
    ) -> Result<CallbackRef, LuaError> {
        let globals = self.lua.globals();
        let table: mlua::Table = globals
            .get(table_name)
            .map_err(|_| LuaError::MissingFunction(format!("{table_name} table")))?;
        let function: mlua::Function = table
            .get(method_name)
            .map_err(|_| LuaError::MissingFunction(format!("{table_name}:{method_name}")))?;
        let registry_key = self.lua.create_registry_value(function)?;
        let callback = CallbackRef(registry_key);
        self.script_registry.insert(callback_key, ());
        Ok(callback)
    }

    /// TFS `Events::eventPlayerOnInventoryUpdate` — `Player:onInventoryUpdate(item, slot, equip)`.
    pub fn call_player_inventory_update(
        &self,
        callback: &CallbackRef,
        player: crate::context::CreatureId,
        item: crate::context::ItemId,
        slot: u8,
        equip: bool,
    ) -> Result<(), LuaError> {
        use crate::context::ItemRef;
        let function: mlua::Function = self
            .lua
            .registry_value(&callback.0)
            .map_err(LuaError::Init)?;
        let player_ud = self
            .lua
            .create_userdata(CreatureRef(player))
            .map_err(LuaError::Init)?;
        let item_ud = self
            .lua
            .create_userdata(ItemRef(item))
            .map_err(LuaError::Init)?;
        function
            .call::<()>( (player_ud, item_ud, slot, equip) )
            .map_err(LuaError::Init)
    }
}

/// Trait for incremental global function registration.
pub trait RegisterLuaFunctions {
    fn register_functions(&self, lua: &Lua) -> Result<(), mlua::Error>;
}

/// Minimal global functions for Track 1 PoC.
struct MinimalGlobalFunctions;

impl RegisterLuaFunctions for MinimalGlobalFunctions {
    fn register_functions(&self, lua: &Lua) -> Result<(), mlua::Error> {
        let globals = lua.globals();

        // debugPrint
        globals.set(
            "debugPrint",
            lua.create_function(|_, msg: String| {
                tracing::debug!("{}", msg);
                Ok(())
            })?,
        )?;

        // configManager (read-only access)
        let config_manager = lua.create_table()?;
        config_manager.set(
            "getString",
            lua.create_function(|_, _key: String| {
                // TODO: Implement config lookup
                Ok(Some(String::new()))
            })?,
        )?;
        config_manager.set(
            "getNumber",
            lua.create_function(|_, _key: String| {
                // TODO: Implement config lookup
                Ok(Some(0))
            })?,
        )?;
        config_manager.set(
            "getBoolean",
            lua.create_function(|_, _key: String| {
                // TODO: Implement config lookup
                Ok(Some(false))
            })?,
        )?;
        globals.set("configManager", config_manager)?;

        Ok(())
    }
}

/// Lua runtime errors.
#[derive(Debug, thiserror::Error)]
pub enum LuaError {
    #[error("VM initialization failed: {0}")]
    Init(#[from] mlua::Error),

    #[error("Function registration failed: {0}")]
    Registration(mlua::Error),

    #[error("Script not found: {0}")]
    ScriptNotFound(String),

    #[error("Script syntax error: {0}")]
    SyntaxError(String),

    #[error("Script IO error for {0}: {1}")]
    ScriptIo(String, String),

    #[error("Missing global Lua function: {0}")]
    MissingFunction(String),

    #[error("Not implemented")]
    NotImplemented,
}
