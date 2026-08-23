//! Lua userdata bindings for game objects.

pub mod combat;
pub mod condition;
pub mod container;
pub mod group;
pub mod item;
pub mod item_type;
pub mod monster;
pub mod monster_type;
pub mod npc;
pub mod player;
pub mod position;
pub mod spell;
pub mod tile;
pub mod town;
pub mod vocation;
pub mod weapon;

pub use combat::{AreaCombat, CombatDef, register_combat_metatable};
pub use condition::{ConditionBuilder, register_condition_metatable};
pub use container::{ContainerRef, register_container_metatable};
pub use group::{GroupRef, register_group_metatable};
pub use item::register_item_metatable;
pub use item_type::{ItemTypeRef, register_item_type_constructor, register_item_type_metatable};
pub use monster::{MonsterRef, register_monster_metatable};
pub use monster_type::{MonsterTypeRef, register_monster_type_constructor};
pub use npc::{NpcRef, register_npc_metatable};
pub use player::{register_creature_metatable, register_outfit_constructor};
pub use position::{PositionRef, register_position_metatable};
pub use spell::{PendingSpell, SpellBuilder, register_spell_metatable};
pub use tile::{HouseRef, TileRef, register_tile_constructor};
pub use town::{TownRef, register_town_constructor};
pub use vocation::{VocationRef, register_vocation_constructor, register_vocation_metatable};
pub use weapon::{PendingWeapon, WeaponBuilder, register_weapon_metatable};
