//! Simulation core: map, entities, config, scheduler hooks.
// C++ reference: `game.cpp`, `map.cpp`, `configmanager.cpp` (see per-module comments).

pub mod combat;
pub mod condition;
pub mod config;
pub mod creature;
mod death;
mod decay;
pub mod event_dispatcher;
pub mod game_loop;
pub mod game_world;
pub mod guild;
pub mod house;
pub mod ids;
pub mod item;
pub mod login;
mod login_out;
pub mod lua_command;
pub mod map;
pub mod matrix_area;
pub mod output_queue;
pub mod party;
pub mod pathfinding;
pub mod protocol_hooks;
pub mod scheduler;
pub mod spawn;
pub mod spell;
pub mod stability;
pub mod tile;
pub mod weapon;
pub mod wildcard;

pub use combat::{
    apply_condition, can_player_attack_player, execute, is_in_pvp_zone, is_protected, CombatDamage,
    CombatDenyReason, CombatParams, PlayerPvpSnapshot,
};
pub use condition::{add_condition_merge, ActiveCondition, ConditionData};
pub use config::ConfigManager;
pub use creature::{
    CreatureBase, CreatureKind, DamageMap, Monster, MonsterAiPhase, Npc, NpcEventsHandler,
    NullNpcHandler, Outfit, Player, PlayerEconomy, PlayerInventory, PlayerSkills, PlayerSocial,
};
pub use event_dispatcher::{EventDispatcher, NullEventDispatcher};
pub use game_loop::{graceful_shutdown, run_game_loop, wait_for_shutdown_signal};
pub use game_world::GameWorld;
pub use guild::{Guild, GuildRank, GuildRegistry, GuildWarTracker};
pub use ids::{CreatureId, ItemId};
pub use item::Item;
pub use lua_command::LuaCommand;
pub use map::Map;
pub use matrix_area::MatrixArea;
pub use party::{split_shared_experience, Party, PartyInviteState};
pub use pathfinding::pathfind;
pub use protocol_hooks::{NullProtocolHooks, ProtocolHooks, SharedProtocolHooks};
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

use std::path::Path;

/// Default entry: full OTBM/config bootstrap is not wired here yet. For Phase 7, run
/// `cargo run -p tfs-rust-net --example game_login_smoke` with `DATABASE_URL` and combine with
/// `run_game_loop(world, cmd_rx, Some(out_registry))` once a `GameWorld` is constructed.
pub async fn run() -> anyhow::Result<()> {
    tracing::info!("tfs-rust-core: use `game_login_smoke` + `run_game_loop` for integrated login/game test.");
    let _ = Path::new("config.lua");
    Ok(())
}
