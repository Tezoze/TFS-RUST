//! Lua userdata bindings for game objects.

pub mod condition;
pub mod container;
pub mod item;
pub mod player;
pub mod vocation;

pub use condition::{register_condition_metatable, ConditionBuilder};
pub use container::{register_container_metatable, ContainerRef};
pub use item::register_item_metatable;
pub use player::register_creature_metatable;
pub use vocation::{register_vocation_metatable, VocationRef};
