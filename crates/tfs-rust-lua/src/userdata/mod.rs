//! Lua userdata bindings for game objects.

pub mod combat;
pub mod condition;
pub mod container;
pub mod item;
pub mod player;
pub mod spell;
pub mod vocation;
pub mod weapon;

pub use combat::{register_combat_metatable, AreaCombat, CombatDef};
pub use condition::{register_condition_metatable, ConditionBuilder};
pub use container::{register_container_metatable, ContainerRef};
pub use item::register_item_metatable;
pub use player::register_creature_metatable;
pub use spell::{register_spell_metatable, PendingSpell, SpellBuilder};
pub use vocation::{register_vocation_metatable, VocationRef};
pub use weapon::{register_weapon_metatable, PendingWeapon, WeaponBuilder};
