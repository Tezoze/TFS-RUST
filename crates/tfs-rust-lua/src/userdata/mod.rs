//! Lua userdata bindings for game objects.

pub mod channel;
pub mod container;
pub mod item;
pub mod player;

pub use channel::{register_channel_metatable, ChannelHandle};
pub use container::{register_container_metatable, ContainerRef};
pub use item::register_item_metatable;
pub use player::register_creature_metatable;
