//! Lua scripting infrastructure for Australis.
//!
//! This crate provides the Lua VM, script loading, and userdata bindings
//! for game scripts. It maintains TFS compatibility while using idiomatic Rust.

pub mod chat_channels;
pub mod combat_enums;
pub mod combat_scripts;
pub mod constants;
pub mod context;
pub mod lua_mutation;
pub mod move_events;
pub mod runtime;
pub mod script_loader;
pub mod talkactions;
pub mod timer_events;
pub mod userdata;

// Re-export commonly used types
pub use chat_channels::{ChatChannelDef, load_chat_channel_scripts};
pub use constants::register_constants;
pub use context::{
    CreatureData, CreatureId, ItemData, ItemId, ItemRef, LuaContext, with_lua_context,
};
pub use lua_mutation::{
    CombatExecuteRequest, ConditionApplySpec, LuaMoveDestination, LuaMutation, call_combat_execute,
    call_lua_add_condition, call_lua_add_item, call_lua_add_item_full, call_lua_add_mana,
    call_lua_add_mana_spent, call_lua_container_add_item, call_lua_get_depot_chest,
    call_lua_get_inbox, call_lua_item_decay, call_lua_item_move_to, call_lua_item_remove,
    call_lua_item_transform, call_lua_remove_condition, call_lua_remove_item,
    call_lua_send_cancel_message, call_lua_send_channel_message, call_lua_send_magic_effect,
    call_lua_set_action_id, call_lua_set_in_fight, call_lua_set_store_item,
    call_lua_set_unique_id, register_lua_mutation_applier, set_mutation_bool_result,
    set_mutation_item_result, with_lua_mutation_scope,
};
pub use move_events::{MoveEventEntry, MoveEventKind, MoveEventsRegistry};
pub use runtime::{
    CallbackRef, LuaError, LuaRuntime, PendingChatChannel, PendingTalkAction, RegisterLuaFunctions,
};
pub use script_loader::{CreatureEventType, LoadError, PlayerEventType, ScriptLoader};
pub use talkactions::{TalkActionDef, load_talkaction_scripts};
pub use timer_events::{
    TimerEventDesc, TimerEvents, TimerScheduler, execute_timer_event,
    register_add_event_stop_event, set_timer_scheduler,
};
pub use userdata::{
    ConditionBuilder, ContainerRef, VocationRef, register_condition_metatable,
    register_container_metatable, register_creature_metatable, register_item_metatable,
    register_vocation_metatable,
};
