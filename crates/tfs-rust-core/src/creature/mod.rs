//! Creatures: `CreatureBase`, players, monsters, NPCs.
// C++ reference: `creature.cpp`, `player.cpp`, `monster.cpp`, `npc.cpp`.

mod base;
mod kind;
mod light;
mod monster;
mod monster_combat;
mod monster_inventory;
mod move_possible;
mod npc;
mod player;
pub mod vocation;

pub use base::{ChaseMode, CombatList, CombatListEntry, CreatureBase, DamageMap, Outfit};
pub use kind::CreatureKind;
pub use light::LightInfo;
pub use monster::{Monster, MonsterAiConfig, MonsterAiPhase, MonsterState};
pub use monster_combat::{
    MeleeDefenseSnapshot, MonsterCombatSnapshot, MonsterFieldType, MonsterSpell, SpellImpact,
    SpellShape, combat_from_monster_type, creature_immune_poison, defend_fight_mode_for_target,
    drunk_power_from_xml, duration_ms_to_rounds, melee_defense_snapshot, melee_poison_on_hit,
    monster_has_melee_strike, monster_weapon_attack_distance, roll_target_defense,
    runtime_spell_in_attack_range, speed_mdact,
};
pub use monster_inventory::{
    DEFAULT_MONSTER_BAG_TYPE, MonsterInventory, damage_text_color, effective_monster_combat_stats,
};
pub use npc::{Npc, NpcActivity, NpcPlayerSession, NpcRuntimeState, QueuedNpcAddress};
pub use player::{
    OutfitEntry, PSTRG_OUTFITS_RANGE_SIZE, PSTRG_OUTFITS_RANGE_START, Player, PlayerEconomy,
    PlayerInventory, PlayerPersistBaseline, PlayerSkills, PlayerSocial, PlayerWalkAction,
    take_outfits_from_storage, write_outfits_into_storage,
};
pub use tfs_rust_common::PlayerSex;
