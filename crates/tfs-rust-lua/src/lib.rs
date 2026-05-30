//! Lua scripting infrastructure for Australis.
//!
//! This crate provides the Lua VM, script loading, and userdata bindings
//! for game scripts. It maintains TFS compatibility while using idiomatic Rust.

pub mod context;
pub mod lua_mutation;
pub mod runtime;
pub mod script_loader;
pub mod userdata;

// Re-export commonly used types
pub use context::{
    CreatureData, CreatureId, ItemData, ItemId, ItemRef, LuaContext, with_lua_context,
};
pub use lua_mutation::{
    call_lua_add_item, call_lua_remove_item, register_lua_mutation_applier, with_lua_mutation_scope,
    LuaMutation,
};
pub use runtime::{LuaRuntime, CallbackRef, RegisterLuaFunctions, LuaError};
pub use script_loader::{ScriptLoader, CreatureEventType, PlayerEventType, LoadError};
pub use userdata::{register_creature_metatable, register_item_metatable};
