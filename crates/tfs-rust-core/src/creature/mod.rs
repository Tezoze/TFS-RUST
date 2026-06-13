//! Creatures: `CreatureBase`, players, monsters, NPCs.
// C++ reference: `creature.cpp`, `player.cpp`, `monster.cpp`, `npc.cpp`.

mod base;
mod kind;
mod light;
mod monster;
mod monster_combat;
mod npc;
mod player;
pub mod vocation;

pub use base::{CreatureBase, DamageMap, Outfit, WalkTimer};
pub use light::LightInfo;
pub use kind::CreatureKind;
pub use monster::{Monster, MonsterAiConfig, MonsterAiPhase};
pub use monster_combat::{
    combat_from_monster_type, monster_has_melee_strike, runtime_spell_in_attack_range,
    MonsterCombatSnapshot, MonsterSpell, SpellImpact, SpellShape,
};
pub use npc::{Npc, NpcEventsHandler, NullNpcHandler};
pub use player::{
    Player, PlayerEconomy, PlayerInventory, PlayerPersistBaseline, PlayerSkills, PlayerSocial,
    PlayerWalkAction,
};
