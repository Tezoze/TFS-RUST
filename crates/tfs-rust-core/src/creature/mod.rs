//! Creatures: `CreatureBase`, players, monsters, NPCs.
// C++ reference: `creature.cpp`, `player.cpp`, `monster.cpp`, `npc.cpp`.

mod base;
mod kind;
mod light;
mod monster;
mod monster_combat;
mod monster_inventory;
mod npc;
mod player;
pub mod vocation;

pub use base::{CreatureBase, DamageMap, Outfit, WalkTimer};
pub use light::LightInfo;
pub use kind::CreatureKind;
pub use monster::{Monster, MonsterAiConfig, MonsterAiPhase, MonsterChaseMode, MonsterState};
pub use monster_inventory::{effective_monster_combat_stats, MonsterInventory, DEFAULT_MONSTER_BAG_TYPE};
pub use monster_combat::{
    combat_from_monster_type, creature_immune_poison, defend_fight_mode_for_target,
    melee_defense_snapshot, melee_poison_on_hit, monster_has_melee_strike,
    monster_weapon_attack_distance, roll_target_defense, runtime_spell_in_attack_range,
    MeleeDefenseSnapshot, MonsterCombatSnapshot, MonsterSpell, SpellImpact, SpellShape,
};
pub use npc::{Npc, NpcEventsHandler, NullNpcHandler};
pub use player::{
    Player, PlayerEconomy, PlayerInventory, PlayerPersistBaseline, PlayerSkills, PlayerSocial,
    PlayerWalkAction,
};
