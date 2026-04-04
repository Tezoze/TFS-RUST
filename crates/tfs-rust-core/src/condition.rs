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
}
