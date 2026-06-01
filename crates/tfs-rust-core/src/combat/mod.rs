//! Combat dispatch: health / mana / conditions / dispel.
// C++ reference: `combat.cpp` `Combat::doTargetCombat`, `Game::combatChangeHealth`.

pub mod pvp;
pub mod rng;
pub mod math;

use slotmap::SlotMap;

use crate::condition::{add_condition_merge, ActiveCondition};
use crate::creature::CreatureKind;
use crate::ids::CreatureId;
use tfs_rust_common::enums::CombatType;

pub use pvp::{
    can_player_attack_player, is_in_pvp_zone, is_protected, CombatDenyReason, PlayerPvpSnapshot,
};
pub use rng::{normal_random, triangular_random, uniform_random};
pub use math::{
    armor_reduction, attack_speed_ms, condition_tick, defense_value, defense_gate_ms,
    distribute_experience, experience_for_level, melee_damage_after_defense_and_armor, probe_value,
    pvp_exp_cap, req_skill_tries, spell_damage, weapon_damage, DotElement, FightMode,
};

/// Primary + secondary damage packet (TFS `CombatDamage` simplified).
#[derive(Debug, Clone, Copy)]
pub struct CombatDamage {
    pub primary: (CombatType, i32),
    pub secondary: (CombatType, i32),
}

/// Parameters for [`execute`].
#[derive(Debug, Clone)]
pub struct CombatParams {
    pub primary_type: CombatType,
    /// If set, removes matching conditions instead of dealing damage.
    pub dispel: Option<tfs_rust_common::enums::ConditionType>,
    /// If set, merges a new condition onto the target (no HP change in this branch).
    pub apply_condition: Option<ActiveCondition>,
}

impl Default for CombatParams {
    fn default() -> Self {
        Self {
            primary_type: CombatType::Physical,
            dispel: None,
            apply_condition: None,
        }
    }
}

/// Apply combat result to `target`: health/mana/conditions/dispel.
/// Returns `true` if any change was applied.  
// C++ reference: `Game::combatChangeHealth`, `Combat::Combat::doTargetCombat`.
pub fn execute(
    creatures: &mut SlotMap<CreatureId, CreatureKind>,
    attacker: Option<CreatureId>,
    target: CreatureId,
    damage: &CombatDamage,
    params: &CombatParams,
) -> bool {
    if let Some(dt) = params.dispel {
        return dispel_conditions(creatures, target, dt);
    }

    let mut applied_condition = false;
    if let Some(ref cond) = params.apply_condition {
        apply_condition(creatures, target, cond.clone());
        applied_condition = true;
    }

    if params.primary_type == CombatType::ManaDrain || damage.primary.0 == CombatType::ManaDrain {
        return apply_mana_change(creatures, target, damage.primary.1 + damage.secondary.1)
            || applied_condition;
    }

    let total = damage.primary.1 + damage.secondary.1;
    apply_health_delta(creatures, attacker, target, total) || applied_condition
}

fn apply_mana_change(
    creatures: &mut SlotMap<CreatureId, CreatureKind>,
    target: CreatureId,
    delta: i32,
) -> bool {
    let Some(kind) = creatures.get_mut(target) else {
        return false;
    };
    match kind {
        CreatureKind::Player(p) => {
            p.mana = (p.mana + delta).clamp(0, p.max_mana);
            true
        }
        _ => false,
    }
}

fn apply_health_delta(
    creatures: &mut SlotMap<CreatureId, CreatureKind>,
    attacker: Option<CreatureId>,
    target: CreatureId,
    delta: i32,
) -> bool {
    let Some(kind) = creatures.get_mut(target) else {
        return false;
    };
    let base = kind.base_mut();
    let old_hp = base.health;
    let new_hp = (old_hp + delta).clamp(0, base.max_health);
    if new_hp < old_hp {
        if let Some(aid) = attacker {
            *base.damage_map.entry(aid).or_insert(0) += (old_hp - new_hp) as u64;
        }
    }
    base.health = new_hp;
    old_hp != new_hp
}

fn dispel_conditions(
    creatures: &mut SlotMap<CreatureId, CreatureKind>,
    target: CreatureId,
    dtype: tfs_rust_common::enums::ConditionType,
) -> bool {
    let Some(kind) = creatures.get_mut(target) else {
        return false;
    };
    let base = kind.base_mut();
    let before = base.active_conditions.len();
    base.active_conditions.retain(|c| c.ctype != dtype);
    before != base.active_conditions.len()
}

/// Add or merge a condition on the target creature.
pub fn apply_condition(
    creatures: &mut SlotMap<CreatureId, CreatureKind>,
    target: CreatureId,
    cond: ActiveCondition,
) {
    let Some(kind) = creatures.get_mut(target) else {
        return;
    };
    add_condition_merge(&mut kind.base_mut().active_conditions, cond);
}
