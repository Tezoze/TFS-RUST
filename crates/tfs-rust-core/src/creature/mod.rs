//! Creatures: `CreatureBase`, players, monsters, NPCs.
// C++ reference: `creature.cpp`, `player.cpp`, `monster.cpp`, `npc.cpp`.

mod base;
mod kind;
mod light;
mod monster;
mod npc;
mod player;
pub mod vocation;

pub use base::{CreatureBase, DamageMap, Outfit, WalkTimer};
pub use light::LightInfo;
pub use kind::CreatureKind;

pub(crate) use kind::creature_id_eq_slice;
pub use monster::{Monster, MonsterAiPhase};
pub use npc::{Npc, NpcEventsHandler, NullNpcHandler};
pub use player::{
    Player, PlayerEconomy, PlayerInventory, PlayerPersistBaseline, PlayerSkills, PlayerSocial,
    PlayerWalkAction,
};
