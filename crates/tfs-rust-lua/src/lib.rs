//! Lua scripting infrastructure for Australis.
//!
//! This crate provides the Lua VM, script loading, and userdata bindings
//! for game scripts. It maintains TFS compatibility while using idiomatic Rust.

pub mod actions;
pub mod chat_channels;
mod class_registry;
pub mod combat_enums;
pub mod combat_scripts;
pub mod constants;
pub mod context;
mod instruction_budget;
pub mod lua_defs;
pub mod lua_mutation;
pub mod move_events;
pub mod npc_dialogue;
pub mod npc_loader;
pub mod npc_type;
pub mod runtime;
pub mod script_loader;
mod stdlib_allowlist;
pub mod talkactions;
pub mod timer_events;
pub mod userdata;

// Re-export commonly used types
pub use actions::{
    ActionDef, assert_required_data_globals, inject_door_tables_from_global, inject_era_formulas,
    load_action_scripts, load_data_lib,
};
pub use chat_channels::{ChatChannelDef, load_chat_channel_scripts};
pub use constants::register_constants;
pub use context::{
    CreatureData, CreatureId, ItemData, ItemId, ItemRef, LuaContext, with_lua_context,
};
pub use instruction_budget::DEFAULT_LUA_INSTRUCTION_BUDGET;
pub use lua_mutation::{
    CombatExecuteRequest, ConditionApplySpec, LuaMoveDestination, LuaMutation, call_combat_execute,
    call_do_challenge_creature, call_do_target_combat_health, call_lua_add_condition,
    call_lua_add_health, call_lua_add_item, call_lua_add_item_ex, call_lua_add_item_full,
    call_lua_add_mana, call_lua_add_mana_spent, call_lua_add_skill_tries, call_lua_bank_deposit,
    call_lua_bank_withdraw, call_lua_container_add_item, call_lua_game_create_item,
    call_lua_game_create_tile, call_lua_get_depot_chest, call_lua_get_inbox, call_lua_item_decay,
    call_lua_item_move_to, call_lua_item_remove, call_lua_item_transform, call_lua_npc_say,
    call_lua_npc_set_focus, call_lua_remove_condition, call_lua_remove_item,
    call_lua_send_cancel_message, call_lua_send_channel_message, call_lua_send_magic_effect,
    call_lua_set_action_id, call_lua_set_in_fight, call_lua_set_store_item, call_lua_set_unique_id,
    call_lua_tile_add_item, register_lua_mutation_applier, set_mutation_bool_result,
    set_mutation_i32_result, set_mutation_item_result, with_lua_mutation_scope,
};
pub use move_events::{
    MoveEventDef, MoveEventEntry, MoveEventKind, MoveEventsRegistry, load_move_event_scripts,
    merge_move_event_defs,
};
pub use npc_dialogue::{NpcDialogueProgram, register_npc_dialogue};
pub use npc_type::{NpcTypeBuilder, PendingNpc, register_npc_type};
pub use runtime::{
    CallbackRef, DEFAULT_LUA_MEMORY_LIMIT_BYTES, LuaError, LuaRuntime, PendingAction,
    PendingChatChannel, PendingMoveEvent, PendingTalkAction, RegisterLuaFunctions,
};
pub use script_loader::{CreatureEventType, LoadError, PlayerEventType, ScriptLoader};
pub use talkactions::{TalkActionDef, load_all_talkaction_scripts, load_talkaction_scripts};
pub use timer_events::{
    TimerEventDesc, TimerEvents, TimerScheduler, execute_timer_event,
    register_add_event_stop_event, set_timer_scheduler,
};
pub use userdata::{
    ConditionBuilder, ContainerRef, NpcRef, VocationRef, register_condition_metatable,
    register_container_metatable, register_creature_metatable, register_item_metatable,
    register_npc_metatable, register_vocation_metatable,
};
