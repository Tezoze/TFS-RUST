//! Active conditions and merge rules (TFS `Condition::addCondition` / `updateCondition` simplified).
// C++ reference: `condition.h`, `condition.cpp`.

use tfs_rust_common::enums::ConditionType;

/// Payload for an active condition instance (mirrors major TFS condition subclasses).
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionData {
    Damage {
        /// Total “strength” for merge comparison (higher replaces when same type+subId).
        /// For poison this is the damage *pool* that drains by the per-Event damage.
        /// For fire/energy this is the per-Event damage (fixed).
        total_rank: i32,
        /// 772 `TSkillPoison::FactorPercent` — per-mille drain rate (default 50 = 5% per Event).
        /// Clamped 10..1000 (`crskill.cc:1004-1010`). `0` = use default 50 (unused for fire/energy).
        /// C++ reference: `crskill.cc:977` `Range = (Cycle * FactorPercent) / 1000`.
        factor_percent: i32,
    },
    Speed {
        /// Flat speed delta (positive = haste, negative = paralyze).
        flat_delta: i32,
    },
    Outfit {
        /// Illusion lookType (creature look). `0` when using look_type_ex only.
        look_type: i32,
        /// Item illusion lookTypeEx (TFS `outfit.lookTypeEx` / XML `item=`).
        look_type_ex: u16,
    },
    Light {
        level: u8,
        color: u8,
    },
    Regeneration {
        health_gain: i32,
        /// Interval between HP ticks (ms). `0` → no HP regen from this condition.
        health_ticks_ms: u32,
        mana_gain: i32,
        /// Interval between mana ticks (ms). `0` → no mana regen from this condition.
        mana_ticks_ms: u32,
        health_elapsed_ms: u32,
        mana_elapsed_ms: u32,
    },
    Soul {
        per_tick: i32,
    },
    Attributes {
        melee: i16,
        shielding: i16,
        distance: i16,
        magic: i16,
    },
    SpellCooldown {
        spell_id: u16,
    },
    SpellGroupCooldown {
        group: u8,
    },
    Generic {
        /// Merge key for idempotence tests; larger `ticks` wins on refresh.
        ticks: i32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveCondition {
    pub id: u32,
    pub sub_id: u32,
    pub ctype: ConditionType,
    pub data: ConditionData,
    /// 772 `TSkill::Cycle` — DoT Events remaining (`crskill.cc:179-196`).
    pub timer_rounds_left: Option<i32>,
    /// 772 `TSkill::Count` — `ProcessSkills` countdown to next Event (`crskill.cc:186-193`).
    /// `0` with [`Self::skill_max_count`] `0` means “not initialized” (filled on first tick).
    pub skill_count: i32,
    /// 772 `TSkill::MaxCount` — Event interval in ProcessSkills rounds (fire=8, energy=10, poison=3).
    pub skill_max_count: i32,
}

impl ActiveCondition {
    /// New condition with optional timer rounds (defaults applied on first tick when `None`).
    pub fn new(
        id: u32,
        sub_id: u32,
        ctype: ConditionType,
        data: ConditionData,
        timer_rounds_left: Option<i32>,
    ) -> Self {
        Self {
            id,
            sub_id,
            ctype,
            data,
            timer_rounds_left,
            skill_count: 0,
            skill_max_count: 0,
        }
    }

    /// 772 `SetTimer(skill, Cycle, Count, MaxCount, …)` — `crmain.cc:589-610`.
    pub fn with_skill_timer(mut self, count: i32, max_count: i32) -> Self {
        self.skill_count = count.max(0);
        self.skill_max_count = max_count.max(0);
        self
    }
}

/// Insert or merge with an existing condition of the same `(ctype, sub_id)`.
// C++ reference: `ConditionDamage::addCondition`, `ConditionGeneric::addCondition`.
pub fn add_condition_merge(list: &mut Vec<ActiveCondition>, incoming: ActiveCondition) {
    let pos = list
        .iter()
        .position(|c| c.ctype == incoming.ctype && c.sub_id == incoming.sub_id);
    if let Some(i) = pos {
        merge_into(&mut list[i], &incoming);
    } else {
        list.push(incoming);
    }
}

fn merge_into(existing: &mut ActiveCondition, incoming: &ActiveCondition) {
    use ConditionData::*;

    // 772 poison: re-arm only when `Damage > TimerValue()` (`crmain.cc:586-590`).
    // Weaker poison must not refresh Count/MaxCount / Cycle.
    let poison_strength_gated = existing.ctype == ConditionType::Poison
        && matches!(
            (&existing.data, &incoming.data),
            (Damage { .. }, Damage { .. })
        );

    let mut accept_stronger_poison = false;
    match (&mut existing.data, &incoming.data) {
        (Damage { total_rank: a, factor_percent: fa }, Damage { total_rank: b, factor_percent: fb }) => {
            if *b > *a {
                *a = *b;
                *fa = *fb;
                existing.id = incoming.id;
                accept_stronger_poison = true;
            }
        }
        (Speed { flat_delta: a }, Speed { flat_delta: b }) => {
            // Stronger haste wins for positive; stronger slow for negative (more negative).
            let incoming_stronger = if *a >= 0 && *b >= 0 {
                *b > *a
            } else if *a <= 0 && *b <= 0 {
                *b < *a
            } else {
                b.abs() > a.abs()
            };
            if incoming_stronger || *a == *b {
                *a = *b;
                existing.id = incoming.id;
            }
        }
        (Generic { ticks: a }, Generic { ticks: b }) => {
            if *b >= *a {
                *a = *b;
                existing.id = incoming.id;
            }
        }
        _ => {
            // Fallback: replace payload if incoming id matches “newer” convention (higher id).
            if incoming.id >= existing.id {
                existing.data = incoming.data.clone();
                existing.id = incoming.id;
                if incoming.timer_rounds_left.is_some() {
                    existing.timer_rounds_left = incoming.timer_rounds_left;
                }
            }
        }
    }

    if poison_strength_gated {
        if accept_stronger_poison {
            if incoming.timer_rounds_left.is_some() {
                existing.timer_rounds_left = incoming.timer_rounds_left;
            }
            if incoming.skill_max_count > 0 {
                existing.skill_count = incoming.skill_count;
                existing.skill_max_count = incoming.skill_max_count;
            }
        }
        return;
    }

    if incoming.timer_rounds_left.is_some() {
        existing.timer_rounds_left = incoming.timer_rounds_left;
    }
    // Refresh 772 Count/MaxCount when re-applying fire/energy DoT (unconditional re-arm).
    if incoming.skill_max_count > 0 {
        existing.skill_count = incoming.skill_count;
        existing.skill_max_count = incoming.skill_max_count;
    }
}

/// Per-tick DoT damage for an elemental field condition (B4.6), profile-driven.
///
/// Maps a fire/energy [`ConditionType`] to its `(Event damage, MaxCount interval)` from the
/// active [`MechanicsProfile`] (or a Tier-2 `getConditionTick` override). Returns `None` for
/// condition types without a profiled DoT spec (poison decays differently; haste/paralyze are
/// speed, not DoT).
///
/// C++ reference: 772 `TSkillBurning::Event` / `TSkill::Process` MaxCount
/// (`tibia-game-master/src/crskill.cc:179-196,1064,1090`); TFS `ConditionDamage` domain.
pub fn dot_tick_for_condition(
    profile: &crate::formulas::MechanicsProfile,
    hooks: &crate::formulas::FormulaHooks,
    ctype: ConditionType,
    round: i32,
) -> Option<(i32, i32)> {
    use crate::combat::math::{condition_tick, DotElement};
    match ctype {
        ConditionType::Fire => Some(condition_tick(profile, hooks, DotElement::Fire, round)),
        ConditionType::Energy => Some(condition_tick(profile, hooks, DotElement::Energy, round)),
        _ => None,
    }
}

/// Applying the same merge twice is equivalent to applying it once (for supported variants).
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_merge_idempotent() {
        let mut v = vec![ActiveCondition {
            id: 1,
            sub_id: 0,
            ctype: ConditionType::Pz,
            data: ConditionData::Generic { ticks: 100 },
            timer_rounds_left: None,
            skill_count: 0,
            skill_max_count: 0,
        }];
        let again = ActiveCondition {
            id: 2,
            sub_id: 0,
            ctype: ConditionType::Pz,
            data: ConditionData::Generic { ticks: 100 },
            timer_rounds_left: None,
            skill_count: 0,
            skill_max_count: 0,
        };
        add_condition_merge(&mut v, again.clone());
        let one = v.clone();
        add_condition_merge(&mut v, again);
        assert_eq!(one, v);
    }

    #[test]
    fn dot_tick_uses_profile_and_skips_non_dot() {
        use tfs_rust_common::ProtocolVersion;
        let m = crate::formulas::Mechanics::for_version(ProtocolVersion::V772);
        // Fire 10/8, energy 25/10 from the profile.
        assert_eq!(
            dot_tick_for_condition(&m.profile, &m.hooks, ConditionType::Fire, 0),
            Some((10, 8))
        );
        assert_eq!(
            dot_tick_for_condition(&m.profile, &m.hooks, ConditionType::Energy, 0),
            Some((25, 10))
        );
        // Non-DoT conditions have no profiled tick.
        assert_eq!(
            dot_tick_for_condition(&m.profile, &m.hooks, ConditionType::Haste, 0),
            None
        );
        assert_eq!(
            dot_tick_for_condition(&m.profile, &m.hooks, ConditionType::Pz, 0),
            None
        );
    }
}
