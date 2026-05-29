//! Lua userdata bindings for game objects.

pub mod item;
pub mod player;

pub use item::register_item_metatable;
pub use player::register_creature_metatable;
