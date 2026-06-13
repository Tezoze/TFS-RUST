//! Runtime monster combat data converted from content XML spell nodes.
//!
//! C++ reference: `cr.hh` `TSpellData`; `crnonpl.cc:2521-2667` CASTING shape/impact switches.

use tfs_rust_common::enums::{CombatType, ConditionType, ShootEffect};
use tfs_rust_content::monsters::{MonsterSpellNode, MonsterType};
use tracing::debug;

/// C++ `SHAPE_*` — `crnonpl.cc:2609`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellShape {
    Actor,
    Victim,
    Origin,
    Destination,
    Angle,
}

/// C++ `IMPACT_*` — `crnonpl.cc:2536`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellImpact {
    Damage {
        element: CombatType,
        base: i32,
        variation: i32,
    },
    Field,
    Healing {
        base: i32,
        variation: i32,
    },
    Speed {
        percent: i32,
        variation: i32,
        duration: i32,
    },
    Condition {
        condition: ConditionType,
        cycle: i32,
        min_cycle: i32,
    },
    Summon {
        race: String,
        max: i32,
    },
}

/// Runtime spell entry for idle CASTING (E4) and target-range checks.
///
/// C++ reference: `cr.hh:55` `TSpellData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterSpell {
    /// Cast gate: `rand() % Delay == 0` — `crnonpl.cc:2527`.
    pub delay: i32,
    pub range: i32,
    pub min_cycle: i32,
    pub shape: SpellShape,
    pub impact: SpellImpact,
    pub shoot_effect: Option<u8>,
    pub area_effect: Option<u8>,
}

/// Combat stats copied from [`MonsterType`] at spawn (melee + defenses + non-melee spells).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MonsterCombatSnapshot {
    pub melee_skill: i32,
    pub melee_attack: i32,
    pub poison_cycles: i32,
    pub armor: i32,
    pub defense: i32,
    pub spells: Vec<MonsterSpell>,
}

impl MonsterSpell {
    /// Converts a non-melee attack/defense XML node. Returns `None` for melee nodes and unknown names.
    pub fn try_from_node(node: &MonsterSpellNode) -> Option<Self> {
        let name = node
            .attributes
            .get("name")
            .map(String::as_str)
            .unwrap_or(node.element.as_str());
        if name.eq_ignore_ascii_case("melee") {
            return None;
        }

        let delay = parse_attr_i32(node, "delay", 0);
        let range = parse_attr_i32(node, "range", 0);
        let min_cycle = parse_attr_i32(node, "mincycle", 0);
        let shape = default_shape_for_name(name, range);
        let impact = parse_spell_impact(name, node)?;
        let shoot_effect = spell_child_attr(node, "shooteffect")
            .as_deref()
            .and_then(parse_shoot_effect_name);
        let area_effect = spell_child_attr(node, "areaeffect")
            .as_deref()
            .and_then(parse_area_effect_name);

        Some(Self {
            delay,
            range,
            min_cycle,
            shape,
            impact,
            shoot_effect,
            area_effect,
        })
    }
}

/// Build combat snapshot from parsed monster type (spawn path).
pub fn combat_from_monster_type(mtype: &MonsterType) -> MonsterCombatSnapshot {
    let mut snap = MonsterCombatSnapshot {
        armor: mtype.defenses.armor.unwrap_or(0),
        defense: mtype.defenses.defense.unwrap_or(0),
        ..MonsterCombatSnapshot::default()
    };

    for node in &mtype.attack_spells {
        let name = node
            .attributes
            .get("name")
            .map(String::as_str)
            .unwrap_or(node.element.as_str());
        if name.eq_ignore_ascii_case("melee") {
            snap.melee_skill = parse_attr_i32(node, "skill", 0);
            snap.melee_attack = parse_attr_i32(node, "attack", 0);
            snap.poison_cycles = parse_attr_i32(node, "poisoncycles", 0);
        } else if let Some(spell) = MonsterSpell::try_from_node(node) {
            snap.spells.push(spell);
        }
    }

    snap
}

/// Whether a runtime spell can reach `distance` (Chebyshev tiles).
pub fn runtime_spell_in_attack_range(spell: &MonsterSpell, distance: u32) -> bool {
    if spell.range <= 0 {
        false
    } else {
        distance <= spell.range as u32
    }
}

/// Whether the monster has a melee strike at adjacency (`SKILL_FIST > 0`, `crnonpl.cc:2705`).
pub fn monster_has_melee_strike(melee_skill: i32, distance: u32) -> bool {
    melee_skill > 0 && distance <= 1
}

fn parse_spell_impact(name: &str, node: &MonsterSpellNode) -> Option<SpellImpact> {
    if name.eq_ignore_ascii_case("poisoncondition") {
        return Some(SpellImpact::Condition {
            condition: ConditionType::Poison,
            cycle: parse_attr_i32(node, "cycle", 0),
            min_cycle: parse_attr_i32(node, "mincycle", 0),
        });
    }
    if name.eq_ignore_ascii_case("firecondition") {
        return Some(SpellImpact::Condition {
            condition: ConditionType::Fire,
            cycle: parse_attr_i32(node, "cycle", 0),
            min_cycle: parse_attr_i32(node, "mincycle", 0),
        });
    }
    if name.eq_ignore_ascii_case("energycondition") {
        return Some(SpellImpact::Condition {
            condition: ConditionType::Energy,
            cycle: parse_attr_i32(node, "cycle", 0),
            min_cycle: parse_attr_i32(node, "mincycle", 0),
        });
    }

    debug!(spell_name = name, "skipping unknown monster attack spell");
    None
}

/// Ranged condition spells at a target default to `SHAPE_VICTIM` — `crnonpl.cc:2609`.
fn default_shape_for_name(name: &str, range: i32) -> SpellShape {
    if name.ends_with("condition") && range > 1 {
        SpellShape::Victim
    } else {
        SpellShape::Actor
    }
}

fn parse_attr_i32(node: &MonsterSpellNode, key: &str, default: i32) -> i32 {
    node.attributes
        .get(key)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn spell_child_attr(node: &MonsterSpellNode, key: &str) -> Option<String> {
    node.attribute_children
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.clone())
}

fn parse_shoot_effect_name(name: &str) -> Option<u8> {
    if name.eq_ignore_ascii_case("poison") {
        Some(ShootEffect::PoisonArrow as u8)
    } else if name.eq_ignore_ascii_case("fire") {
        Some(ShootEffect::Fire as u8)
    } else if name.eq_ignore_ascii_case("energy") {
        Some(ShootEffect::Energy as u8)
    } else {
        debug!(shooteffect = name, "unknown monster shooteffect");
        None
    }
}

fn parse_area_effect_name(name: &str) -> Option<u8> {
    debug!(areaeffect = name, "monster areaeffect not mapped yet");
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    use tfs_rust_content::items::ItemDatabase;
    use tfs_rust_content::monsters::MonsterDatabase;

    use crate::creature::MonsterAiConfig;

    fn empty_items() -> ItemDatabase {
        ItemDatabase {
            items: Default::default(),
            client_to_server: Default::default(),
        }
    }

    fn spell_node(name: &str, attrs: &[(&str, &str)], children: &[(&str, &str)]) -> MonsterSpellNode {
        MonsterSpellNode {
            element: "attack".into(),
            attributes: attrs
                .iter()
                .chain(std::iter::once(&("name", name)))
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            attribute_children: children
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn load_monster_type(index_name: &str) -> MonsterType {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let items = empty_items();
        let db = MonsterDatabase::load_dir(
            &Path::new(manifest).join("../../data/monster"),
            &items,
        )
        .expect("load monsters");
        db.monsters
            .get(&index_name.to_lowercase())
            .cloned()
            .unwrap_or_else(|| panic!("missing monster type {index_name}"))
    }

    #[test]
    fn test_e0_rat_combat_from_xml() {
        let mtype = load_monster_type("rat");
        let cfg = MonsterAiConfig::from_monster_type(&mtype);
        assert_eq!(cfg.melee_skill, 15);
        assert_eq!(cfg.melee_attack, 7);
        assert_eq!(cfg.defense, 3);
        assert_eq!(cfg.armor, 1);
        assert_eq!(cfg.poison_cycles, 0);
        assert!(cfg.spells.is_empty());
    }

    #[test]
    fn test_e0_cobra_poison_spell_from_xml() {
        let mtype = load_monster_type("cobra");
        let cfg = MonsterAiConfig::from_monster_type(&mtype);
        assert_eq!(cfg.melee_skill, 23);
        assert_eq!(cfg.melee_attack, 15);
        assert_eq!(cfg.poison_cycles, 100);
        assert_eq!(cfg.spells.len(), 1);
        let spell = &cfg.spells[0];
        assert_eq!(spell.delay, 4);
        assert_eq!(spell.range, 5);
        assert_eq!(spell.min_cycle, 6);
        assert_eq!(spell.shape, SpellShape::Victim);
        assert!(matches!(
            spell.impact,
            SpellImpact::Condition {
                condition: ConditionType::Poison,
                cycle: 20,
                min_cycle: 6,
            }
        ));
        assert_eq!(spell.shoot_effect, Some(ShootEffect::PoisonArrow as u8));
    }

    #[test]
    fn test_e0_unknown_spell_skipped() {
        let node = spell_node("weirdattack", &[("delay", "1")], &[]);
        assert!(MonsterSpell::try_from_node(&node).is_none());
        let mut mtype = load_monster_type("rat");
        mtype.attack_spells.push(node);
        let snap = combat_from_monster_type(&mtype);
        assert_eq!(snap.spells.len(), 0);
        assert_eq!(snap.melee_skill, 15);
    }

    #[test]
    fn test_e0_runtime_spell_range() {
        let spell = MonsterSpell {
            delay: 4,
            range: 5,
            min_cycle: 6,
            shape: SpellShape::Victim,
            impact: SpellImpact::Condition {
                condition: ConditionType::Poison,
                cycle: 20,
                min_cycle: 6,
            },
            shoot_effect: None,
            area_effect: None,
        };
        assert!(runtime_spell_in_attack_range(&spell, 5));
        assert!(!runtime_spell_in_attack_range(&spell, 6));
        assert!(monster_has_melee_strike(15, 1));
        assert!(!monster_has_melee_strike(15, 2));
        assert!(!monster_has_melee_strike(0, 1));
    }
}
