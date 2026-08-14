//! Combat dispatch: health / mana / conditions / dispel.
// C++ reference: `combat.cpp` `Combat::doTargetCombat`, `Game::combatChangeHealth`.

pub mod aoe;
pub mod circles;
pub mod math;
pub mod pvp;
pub mod rng;

use slotmap::SlotMap;

use crate::condition::{ActiveCondition, add_condition_merge};
use crate::creature::CreatureKind;
use crate::ids::CreatureId;
use tfs_rust_common::enums::CombatType;

pub use circles::{DISC_RINGS, MAX_DISC_RADIUS, disc_offsets, disc_tile_count};
pub use math::{
    DotElement, FightMode, armor_reduction, attack_speed_ms, classic_probe_sample,
    classic_probe_sample_raw, condition_tick, defense_gate_ms, defense_value,
    distribute_experience, experience_for_level, formula_skill_damage_bounds,
    formula_skill_weapon_max, melee_damage_after_defense_and_armor, probe_damage_ceiling,
    probe_hit, probe_value, pvp_kill_experience_amount, req_skill_tries, spell_damage,
    spell_damage_range, spell_formula_multiplier, weapon_damage,
};
pub use pvp::{
    CombatDenyReason, PlayerPvpSnapshot, can_player_attack_player, is_in_pvp_zone, is_protected,
};
pub use rng::{
    normal_random, normal_random_glibc, triangular_random, triangular_random_glibc, uniform_random,
    uniform_random_glibc,
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
    /// H1 — Target armor value for Physical damage subtraction in the shared path.
    /// When `Some`, `combat_execute_with_stimulus` draws `armor_reduction` and subtracts
    /// it after PvP-half/absorb, before mana shield (`crmain.cc:624-630`).
    /// Callers that pre-rolled armor (legacy) leave this `None`.
    pub armor: Option<i32>,
}

impl Default for CombatParams {
    fn default() -> Self {
        Self {
            primary_type: CombatType::Physical,
            dispel: None,
            apply_condition: None,
            armor: None,
        }
    }
}

/// Optional combat-list credit context for [`execute`] / [`apply_health_delta`].
///
/// 772 records clamped HP loss after absorb (`crmain.cc:690-703`) and splits summon
/// damage half to attacker + responsible master.
#[derive(Debug, Clone, Copy)]
pub struct CombatListCredit {
    pub round_nr: u32,
    /// Master (or self) responsible for the attack — when ≠ attacker, each gets `dmg/2`.
    pub responsible: Option<CreatureId>,
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
    execute_with_credit(creatures, attacker, target, damage, params, None)
}

/// Like [`execute`], but records combat-list credit with round/summon split when `credit` is set.
pub fn execute_with_credit(
    creatures: &mut SlotMap<CreatureId, CreatureKind>,
    attacker: Option<CreatureId>,
    target: CreatureId,
    damage: &CombatDamage,
    params: &CombatParams,
    credit: Option<CombatListCredit>,
) -> bool {
    if let Some(dt) = params.dispel {
        return dispel_conditions(creatures, target, dt);
    }

    let mut applied_condition = false;
    if let Some(ref cond) = params.apply_condition {
        // When Lua / melee poison proc applies a DoT condition with a known attacker,
        // store `*DamageOrigin` so Event ticks credit the killer (`crmain.cc:587-609`).
        if let Some(aid) = attacker {
            match cond.ctype {
                tfs_rust_common::enums::ConditionType::Poison => {
                    if let Some(kind) = creatures.get_mut(target) {
                        kind.base_mut().poison_damage_origin = Some(aid);
                    }
                }
                tfs_rust_common::enums::ConditionType::Fire => {
                    if let Some(kind) = creatures.get_mut(target) {
                        kind.base_mut().fire_damage_origin = Some(aid);
                    }
                }
                tfs_rust_common::enums::ConditionType::Energy => {
                    if let Some(kind) = creatures.get_mut(target) {
                        kind.base_mut().energy_damage_origin = Some(aid);
                    }
                }
                _ => {}
            }
        }
        apply_condition(creatures, target, cond.clone());
        applied_condition = true;
    }

    if params.primary_type == CombatType::ManaDrain || damage.primary.0 == CombatType::ManaDrain {
        return apply_mana_change(creatures, target, damage.primary.1 + damage.secondary.1)
            || applied_condition;
    }

    let total = damage.primary.1 + damage.secondary.1;
    apply_health_delta(creatures, attacker, target, total, credit) || applied_condition
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
    credit: Option<CombatListCredit>,
) -> bool {
    // Belt-and-suspenders: NPCs never take HP loss (TFS `Npc::isAttackable` false).
    // Primary gate is `combat_execute_with_stimulus`; this covers direct `combat::execute` callers.
    if delta < 0 && matches!(creatures.get(target), Some(CreatureKind::Npc(_))) {
        return false;
    }
    let Some(kind) = creatures.get_mut(target) else {
        return false;
    };
    let old_hp = kind.base().health;
    // 772 `Damage == HitPoints` — exact lethal (not overkill) gates amulet-of-loss (`crmain.cc:792`).
    let exact_lethal = delta < 0 && delta == -old_hp;
    let new_hp = (old_hp + delta).clamp(0, kind.base().max_health);
    {
        let base = kind.base_mut();
        if new_hp < old_hp {
            let lost = (old_hp - new_hp) as u64;
            if let Some(aid) = attacker {
                base.last_hit_by = Some(aid);
                if let Some(c) = credit {
                    // 772 summon split (`crmain.cc:698-703`): half to attacker, half to responsible.
                    let responsible = c.responsible.unwrap_or(aid);
                    if responsible != aid {
                        let half = lost / 2;
                        base.damage_map.add(aid, half, c.round_nr);
                        base.damage_map.add(responsible, half, c.round_nr);
                    } else {
                        base.damage_map.add(aid, lost, c.round_nr);
                    }
                } else {
                    // Callers without round context still record damage (timestamp 0).
                    base.damage_map.add(aid, lost, 0);
                }
            }
        }
        base.health = new_hp;
    }
    if exact_lethal {
        if let CreatureKind::Player(p) = kind {
            p.exact_lethal_blow = true;
        }
    }
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
    // TFS `Npc::isImmune(ConditionType_t)` when `!attackable` (`npc.h:305-307`).
    if matches!(creatures.get(target), Some(CreatureKind::Npc(_))) {
        return;
    }
    let Some(kind) = creatures.get_mut(target) else {
        return;
    };
    add_condition_merge(&mut kind.base_mut().active_conditions, cond);
}

/// TFS `Game::combatGetTypeInfo` non-physical hit-effect mapping.
///
/// Returns the raw client wire effect byte for a typed damage hit. Physical hits
/// are handled separately by `physical_hit_effect` keyed on the victim's blood
/// family (`creature/monster_inventory.rs`).
///
/// C++ reference:
/// - 1098 `Game::combatGetTypeInfo` — `src/game.cpp:3999-4065`.
/// - 772 `TCreature::Damage` typed branch — `tibia-game-master/src/crmain.cc:744-754`.
pub fn combat_type_hit_effect(combat_type: CombatType) -> Option<u8> {
    match combat_type {
        CombatType::Energy => Some(12), // CONST_ME_ENERGYHIT / EFFECT_ENERGY_HIT
        CombatType::Earth => Some(9),   // CONST_ME_GREEN_RINGS / EFFECT_POISON
        CombatType::Fire => Some(16),   // CONST_ME_HITBYFIRE / EFFECT_FIRE
        CombatType::LifeDrain => Some(14), // CONST_ME_MAGIC_RED / EFFECT_MAGIC_RED
        CombatType::ManaDrain => Some(14), // EFFECT_MAGIC_RED — `crmain.cc:655`
        _ => None,
    }
}

#[cfg(test)]
mod summon_split_tests {
    use super::*;
    use crate::creature::CreatureKind;
    use crate::test_world::support::{
        beat_driven_test_world, ensure_walkable_tile, insert_player, test_player,
    };
    use tfs_rust_common::Position;

    /// B8 — summon damage splits half to attacker + master (`crmain.cc:698-703`).
    #[test]
    fn summon_damage_splits_between_attacker_and_master() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        let master_pos = Position::new(101, 100, 7);
        let summon_pos = Position::new(102, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        ensure_walkable_tile(&mut world.map, master_pos, 150);
        ensure_walkable_tile(&mut world.map, summon_pos, 150);

        let mut victim = test_player("Victim", pos);
        victim.base.health = 100;
        victim.base.max_health = 100;
        let victim_id = insert_player(&mut world, victim);
        let master_id = insert_player(&mut world, test_player("Master", master_pos));
        let summon_id = insert_player(&mut world, test_player("Summon", summon_pos));
        if let Some(CreatureKind::Player(s)) = world.creatures.get_mut(summon_id) {
            s.base.master = Some(master_id);
        }

        let credit = CombatListCredit {
            round_nr: 10,
            responsible: Some(master_id),
        };
        let _ = execute_with_credit(
            &mut world.creatures,
            Some(summon_id),
            victim_id,
            &CombatDamage {
                primary: (CombatType::Physical, -21),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams::default(),
            Some(credit),
        );
        let map = &world.creatures.get(victim_id).unwrap().base().damage_map;
        // 21 → half 10 each (integer division).
        assert_eq!(map.damage_by(summon_id), 10);
        assert_eq!(map.damage_by(master_id), 10);
    }
}
