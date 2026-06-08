//! Active conditions and merge rules (TFS `Condition::addCondition` / `updateCondition` simplified).
// C++ reference: `condition.h`, `condition.cpp`.

use tfs_rust_common::enums::ConditionType;

/// Payload for an active condition instance (mirrors major TFS condition subclasses).
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionData {
    Damage {
        /// Total “strength” for merge comparison (higher replaces when same type+subId).
        total_rank: i32,
    },
    Speed {
        /// Flat speed delta (positive = haste, negative = paralyze).
        flat_delta: i32,
    },
    Outfit {
        look_type: i32,
    },
    Light {
        level: u8,
        color: u8,
    },
    Regeneration {
        health_per_tick: i32,
        mana_per_tick: i32,
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

    match (&mut existing.data, &incoming.data) {
        (Damage { total_rank: a }, Damage { total_rank: b }) => {
            if *b > *a {
                *a = *b;
                existing.id = incoming.id;
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
            }
        }
    }
}

/// Per-tick DoT damage for an elemental field condition (B4.6), profile-driven.
///
/// Maps a fire/energy [`ConditionType`] to its `(damage_per_tick, tick_count)` from the active
/// [`MechanicsProfile`] (or a Tier-2 `getConditionTick` override). Returns `None` for condition
/// types without a profiled DoT spec (poison decays differently; haste/paralyze are speed, not DoT).
/// This is the seam Phase G ticking will call once `ConditionDamage` ticks are implemented.
///
/// C++ reference: 772 `TSkillBurning::Event` (10/8) / `TSkillEnergy::Event` (25/10)
/// (`tibia-game-master/src/crskill.cc:1064,1090`); TFS `ConditionDamage` (`condition.cpp:1330`).
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
        }];
        let again = ActiveCondition {
            id: 2,
            sub_id: 0,
            ctype: ConditionType::Pz,
            data: ConditionData::Generic { ticks: 100 },
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
        assert_eq!(dot_tick_for_condition(&m.profile, &m.hooks, ConditionType::Haste, 0), None);
        assert_eq!(dot_tick_for_condition(&m.profile, &m.hooks, ConditionType::Pz, 0), None);
    }
}
