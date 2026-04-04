//! Simulation core: map, entities, config, scheduler hooks.
// C++ reference: `game.cpp`, `map.cpp`, `configmanager.cpp` (see per-module comments).

pub mod config;
mod creature;
mod decay;
pub mod event_dispatcher;
pub mod game_loop;
pub mod game_world;
pub mod house;
pub mod ids;
pub mod item;
pub mod lua_command;
pub mod map;
pub mod output_queue;
pub mod pathfinding;
pub mod scheduler;
pub mod spawn;
pub mod stability;
pub mod tile;
pub mod wildcard;

pub use config::ConfigManager;
pub use creature::{CreatureKind, MonsterStub, NpcStub, PlayerStub};
pub use event_dispatcher::{EventDispatcher, NullEventDispatcher};
pub use game_loop::{graceful_shutdown, run_game_loop, wait_for_shutdown_signal};
pub use game_world::GameWorld;
pub use ids::{CreatureId, ItemId};
pub use item::Item;
pub use lua_command::LuaCommand;
pub use map::Map;
pub use pathfinding::pathfind;
pub use scheduler::Scheduler;
pub use tile::Tile;
pub use wildcard::WildcardTree;

use std::path::Path;

/// Default entry used by the binary until the full server wires `run_game_loop`.
pub async fn run() -> anyhow::Result<()> {
    tracing::info!("tfs-rust-core initialized (Phase 4 scaffolding)");
    let _ = Path::new("config.lua");
    Ok(())
}
