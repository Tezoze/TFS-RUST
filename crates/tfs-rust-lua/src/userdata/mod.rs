//! Lua userdata bindings for game objects.

pub mod combat;
pub mod condition;
pub mod container;
pub mod group;
pub mod item;
pub mod item_type;
pub mod player;
pub mod position;
pub mod spell;
pub mod vocation;
pub mod weapon;

pub use combat::{register_combat_metatable, AreaCombat, CombatDef};
pub use condition::{register_condition_metatable, ConditionBuilder};
pub use container::{register_container_metatable, ContainerRef};
pub use group::{register_group_metatable, GroupRef};
pub use item::register_item_metatable;
pub use item_type::{register_item_type_constructor, register_item_type_metatable, ItemTypeRef};
pub use player::register_creature_metatable;
pub use position::{register_position_metatable, PositionRef};
pub use spell::{register_spell_metatable, PendingSpell, SpellBuilder};
pub use vocation::{register_vocation_metatable, VocationRef};
pub use weapon::{register_weapon_metatable, PendingWeapon, WeaponBuilder};
