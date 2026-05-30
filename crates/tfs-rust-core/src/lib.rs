//! Simulation core: map, entities, config, scheduler hooks.
// C++ reference: `game.cpp`, `map.cpp`, `configmanager.cpp` (see per-module comments).

pub mod combat;
pub mod condition;
pub mod config;
pub mod creature;
mod creature_think;
mod death;
mod decay;
pub mod event_dispatcher;
pub mod game_loop;
pub mod game_world;
pub mod guild;
pub mod house;
pub mod ids;
pub mod item;
pub mod item_attributes;
mod item_blob;
mod item_constants;
mod item_look;
pub mod inventory;
mod player_flags;
mod player_inventory_load;
mod player_inventory_notifications;
mod player_inventory_query_add;
mod player_inventory_util;
mod player_lua_context;
mod player_depot;
mod floor_change_use;
#[cfg(test)]
mod test_world;
mod player_ping;
mod game_world_inventory;
mod game_world_save;
mod container_ui;
pub mod container;
mod container_ops;
pub mod cylinder;
pub mod thing;
pub mod return_value;
pub mod login;
mod login_out;
pub mod lua_command;
pub mod lua_event_dispatcher;
pub mod lua_scope;
pub mod map;
pub mod matrix_area;
pub mod output_queue;
pub mod party;
pub mod pathfinding;
pub mod protocol_hooks;
mod run_server;
pub mod scheduler;
pub mod spawn;
pub mod spell;
pub mod stability;
pub mod tile;
pub mod walk;
pub mod walk_action;
pub mod weapon;
pub mod wildcard;
pub mod world_light;

pub use combat::{
    apply_condition, can_player_attack_player, execute, is_in_pvp_zone, is_protected, CombatDamage,
    CombatDenyReason, CombatParams, PlayerPvpSnapshot,
};
pub use condition::{add_condition_merge, ActiveCondition, ConditionData};
pub use config::ConfigManager;
pub use creature::{
    CreatureBase, CreatureKind, DamageMap, LightInfo, Monster, MonsterAiPhase, Npc, NpcEventsHandler,
    NullNpcHandler, Outfit, Player, PlayerEconomy, PlayerInventory, PlayerPersistBaseline,
    PlayerSkills, PlayerSocial,
};
pub use event_dispatcher::{EventDispatcher, NullEventDispatcher};
pub use game_loop::{graceful_shutdown, run_game_loop, wait_for_shutdown_signal};
pub use game_world::GameWorld;
pub use guild::{Guild, GuildRank, GuildRegistry, GuildWarTracker};
pub use ids::{CreatureId, ItemId};
pub use container::{Container, ContainerError, ContainerRegistry, ContainerType, OpenContainer};
pub use cylinder::{
    Cylinder, CylinderFlags, CylinderLink, CylinderType, VirtualCylinder, INDEX_ADD_WHEREVER, INDEX_MOVE_UP,
    INDEX_WHEREEVER,
};
pub use item::{Item, ItemPosition};
pub use item_attributes::{AttrType, CustomAttrValue, CustomAttributeMap, DecayState, ItemAttrFlags, ItemAttributes};
pub use return_value::ReturnValue;
pub use thing::{LookTarget, Thing};
pub use lua_command::LuaCommand;
pub use lua_event_dispatcher::LuaEventDispatcher;
pub use map::Map;
pub use matrix_area::MatrixArea;
pub use party::{split_shared_experience, Party, PartyInviteState};
pub use pathfinding::{get_path_matching, FindPathParams, CREATURE_ON_TILE_PATH_COST, MAP_NORMAL_WALK_COST};
pub use protocol_hooks::{NullProtocolHooks, ProtocolHooks, SharedProtocolHooks};
pub use run_server::run;
pub use scheduler::Scheduler;
pub use spell::{
    can_cast_instant, matrix_tile_offsets, register_cast_cooldowns, SpellDefinition,
    SpellFailReason,
};
pub use tile::Tile;
pub use weapon::{
    max_melee_damage_monster, max_weapon_damage_distance_core, max_weapon_damage_melee,
    roll_distance_player_damage, roll_melee_player_damage, roll_wand_damage,
};
pub use wildcard::WildcardTree;

