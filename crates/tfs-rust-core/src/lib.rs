//! Simulation core: map, entities, config, scheduler hooks.
// C++ reference: `game.cpp`, `map.cpp`, `configmanager.cpp` (see per-module comments).

mod actions;
mod bed;
mod chase_debug;
pub mod chat;
mod clear_field;
pub mod combat;
pub mod condition;
mod condition_blob;
pub mod config;
mod connections;
pub mod container;
mod container_ops;
mod container_ui;
pub mod creature;
mod creature_think;
mod creature_todo;
pub mod cylinder;
mod death;
mod decay;
mod decay_apply;
pub mod event_dispatcher;
mod floor_change_use;
pub mod formulas;
pub mod game_loop;
pub mod game_world;
mod game_world_chat;
mod game_world_inventory;
mod game_world_item_cylinder;
mod game_world_item_move;
mod game_world_lifecycle;
mod game_world_lua_tools;
mod game_world_outfit;
mod game_world_player_rotate;
mod game_world_player_throw;
mod game_world_save;
mod game_world_script;
mod game_world_spectators;
mod game_world_tick;
pub mod guild;
pub mod house;
mod idle_stimulus;
pub mod ids;
pub mod inventory;
pub mod item;
pub mod item_attributes;
mod item_blob;
mod item_constants;
mod item_look;
pub mod login;
mod login_out;
pub mod lua_command;
pub mod lua_event_dispatcher;
pub mod lua_scope;
mod magic_field;
pub mod map;
pub mod matrix_area;
mod monster_ai;
mod monster_distance_step;
mod monster_events;
mod monster_push;
mod monster_spawn_hook;
mod monster_targets;
mod npc;
mod obs;
pub mod output_queue;
pub mod party;
pub mod pathfinding;
mod player;
mod player_lua_context;
mod process_skills;
pub mod protocol_hooks;
pub mod return_value;
mod run_server;
pub mod scheduler;
mod sim_glibc_rand;
/// Headless simulation harness — test/diagnostic only.
/// Compiled when `cfg(test)` or `--features sim`; excluded from production builds.
#[cfg(any(test, feature = "sim"))]
pub mod sim_harness;
pub mod spawn;
mod spawn_lifecycle;
mod spawn_placement;
pub mod spell;
pub mod stability;
mod subsystem_counters;
pub mod talkactions;
#[cfg(test)]
mod test_world;
pub mod thing;
pub mod tile;
mod tile_specials;
mod todo_queue;
pub mod walk;
pub mod walk_action;
pub mod weapon;
pub mod wildcard;
pub mod world_light;

// Phase PM — player module consolidation. The formerly flat `player_*.rs` /
// `game_world_player.rs` files moved into `player/` (see `player/mod.rs`).
// These crate-root aliases keep legacy `crate::player_flags::…` /
// `crate::player_inventory_util::…` / `crate::game_world_player::…` / etc. call
// sites resolving unchanged until they are repointed to `crate::player::…`
// opportunistically. Pure move — no logic edits (`tasks/player-combat-plan.md` PM).
// `#[allow(unused_imports)]` — not every alias has a live `crate::<old_name>::…`
// reference today; all are kept for migration stability.
#[allow(unused_imports)]
pub(crate) use player::combat as player_combat;
#[allow(unused_imports)]
pub(crate) use player::depot as player_depot;
#[allow(unused_imports)]
pub(crate) use player::flags as player_flags;
#[allow(unused_imports)]
pub(crate) use player::inventory::load as player_inventory_load;
#[allow(unused_imports)]
pub(crate) use player::inventory::notifications as player_inventory_notifications;
#[allow(unused_imports)]
pub(crate) use player::inventory::query_add as player_inventory_query_add;
#[allow(unused_imports)]
pub(crate) use player::inventory::util as player_inventory_util;
#[allow(unused_imports)]
pub(crate) use player::ping as player_ping;
#[allow(unused_imports)]
pub(crate) use player::stats as game_world_player;

pub use combat::{
    CombatDamage, CombatDenyReason, CombatListCredit, CombatParams, PlayerPvpSnapshot,
    apply_condition, can_player_attack_player, execute, execute_with_credit, is_in_pvp_zone,
    is_protected,
};
pub use condition::{
    ActiveCondition, ConditionData, DRINK_DRUNK_INTERVAL, DRINK_DRUNK_MAX_LEVEL,
    add_condition_merge, apply_drink_drunk_stack, tick_drunk_skill,
};
pub use config::ConfigManager;
pub use container::{Container, ContainerError, ContainerRegistry, ContainerType, OpenContainer};
pub use creature::{
    ChaseMode, CombatList, CombatListEntry, CreatureBase, CreatureKind, DamageMap, LightInfo,
    Monster, MonsterAiPhase, Npc, Outfit, Player, PlayerEconomy, PlayerInventory,
    PlayerPersistBaseline, PlayerSkills, PlayerSocial,
};
pub use cylinder::{
    Cylinder, CylinderFlags, CylinderLink, CylinderType, INDEX_ADD_WHEREVER, INDEX_MOVE_UP,
    INDEX_WHEREEVER, VirtualCylinder,
};
pub use event_dispatcher::{EventCylinder, EventDispatcher, NullEventDispatcher, TalkActionResult};
pub use formulas::{
    ArmorReduction, ConditionTicks, DamageFormula, DestroyableStoneTuning, DistanceKeep,
    FightModes, FishingSuccessModel, FishingTuning, FormulaHooks, LevelExpModel, Mechanics,
    MechanicsProfile, NpcTuning, PathCostModel, PathSearchModel, SpawnNearPlayer, SpellCoeff,
    TickSpec, WeakestTargetMetric, load_mechanics,
};
pub use game_loop::{graceful_shutdown, run_game_loop, wait_for_shutdown_signal};
pub use game_world::GameWorld;
pub use guild::{Guild, GuildRank, GuildRegistry, GuildWarTracker};
pub use ids::{CreatureId, ItemId};
pub use item::Item;
pub use item_attributes::{
    AttrType, CustomAttrValue, CustomAttributeMap, DecayState, ItemAttrFlags, ItemAttributes,
};
pub use lua_command::LuaCommand;
pub use lua_event_dispatcher::LuaEventDispatcher;
pub use map::Map;
pub use matrix_area::MatrixArea;
pub use party::{Party, PartyInviteState, split_shared_experience};
pub use pathfinding::{
    CREATURE_ON_TILE_PATH_COST, FindPathParams, MAP_NORMAL_WALK_COST, get_path_matching,
    uses_reverse_terrain_path,
};
pub use protocol_hooks::{NullProtocolHooks, ProtocolHooks, SharedProtocolHooks};
pub use return_value::ReturnValue;
pub use run_server::run;
pub use scheduler::Scheduler;
pub use spell::{
    SpellDefinition, SpellFailReason, can_cast_instant, matrix_tile_offsets,
    register_cast_cooldowns,
};
pub use thing::{LookTarget, Thing};
pub use tile::Tile;
pub use weapon::{
    max_melee_damage_monster, max_weapon_damage_distance_core, max_weapon_damage_melee,
    roll_distance_player_damage, roll_melee_player_damage, roll_wand_damage,
};
pub use wildcard::WildcardTree;
