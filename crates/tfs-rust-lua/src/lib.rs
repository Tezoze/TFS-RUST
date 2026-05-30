//! Lua scripting infrastructure for Australis.
//!
//! This crate provides the Lua VM, script loading, and userdata bindings
//! for game scripts. It maintains TFS compatibility while using idiomatic Rust.

pub mod context;
pub mod lua_mutation;
pub mod move_events;
pub mod runtime;
pub mod script_loader;
pub mod userdata;

// Re-export commonly used types
pub use context::{
    CreatureData, CreatureId, ItemData, ItemId, ItemRef, LuaContext, with_lua_context,
};
pub use lua_mutation::{
    call_lua_add_item, call_lua_add_item_full, call_lua_container_add_item, call_lua_get_depot_chest,
    call_lua_get_inbox, call_lua_item_move_to, call_lua_item_remove, call_lua_remove_item,
    call_lua_set_action_id, call_lua_set_store_item, call_lua_set_unique_id,
    register_lua_mutation_applier, set_mutation_bool_result, set_mutation_item_result,
    with_lua_mutation_scope, LuaMoveDestination, LuaMutation,
};
pub use move_events::{MoveEventEntry, MoveEventKind, MoveEventsRegistry};
pub use runtime::{CallbackRef, LuaError, LuaRuntime, RegisterLuaFunctions};
pub use script_loader::{CreatureEventType, LoadError, PlayerEventType, ScriptLoader};
pub use userdata::{
    register_container_metatable, register_creature_metatable, register_item_metatable, ContainerRef,
};
