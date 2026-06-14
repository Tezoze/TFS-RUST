//! Runtime monster combat data converted from content XML spell nodes.
//!
//! C++ reference: `cr.hh` `TSpellData`; `crnonpl.cc:2521-2667` CASTING shape/impact switches;
//! `crcombat.cc:647` `CloseAttack`, `:660` poison-on-hit.

use rand::Rng;

use crate::combat::math::{defense_gate_ms, defense_value, FightMode};
use crate::condition::{ActiveCondition, ConditionData};
use crate::creature::base::CreatureBase;
use crate::creature::CreatureKind;
use crate::formulas::{FormulaHooks, MechanicsProfile};
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
    /// C++ `IMPACT_DRUNKEN` — sets target `drunkenness` (`crnonpl.cc:2553`).
    Drunk {
        drunkness: i32,
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
    /// Area radius for `Origin` / `Angle` shapes — XML `radius`.
    pub radius: i32,
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
    pub immunity_poison: bool,
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
        let radius = parse_attr_i32(node, "radius", 0);
        let min_cycle = parse_attr_i32(node, "mincycle", 0);
        let shape = default_shape_for_node(name, node);
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
            radius,
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
        immunity_poison: mtype.defenses.immunity_poison,
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

/// C++ `TCombat::GetDistance` — `crcombat.cc:309` (1 close/fist, 2 throw, 3 missile/wand).
pub fn monster_weapon_attack_distance(melee_skill: i32, has_ranged_spell: bool) -> i32 {
    if melee_skill > 0 {
        1
    } else if has_ranged_spell {
        3
    } else {
        1
    }
}

/// Target defense/armor inputs for melee (`GetDefendValue` / `GetArmorStrength`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeleeDefenseSnapshot {
    pub defense_skill: i32,
    pub defense_value: i32,
    pub armor: i32,
    /// C++ `GetDefendDamage` fight-mode branch — `crcombat.cc:243-255`.
    pub defend_mode: FightMode,
}

/// Effective defend fight mode — `crcombat.cc:243-255` (`Following || AttackDest == 0` → DEFENSIVE).
pub fn defend_fight_mode_for_target(kind: &CreatureKind) -> FightMode {
    let base = kind.base();
    if base.attack_target.is_none() {
        return FightMode::Defensive;
    }
    // Player fight-mode packet not wired yet; monsters default to BALANCED (`crcombat.cc:13`).
    match kind {
        CreatureKind::Player(_) => FightMode::Balanced,
        _ => FightMode::Balanced,
    }
}

/// Whether periodic poison may apply — `crmain.cc:548-551` `RaceData[Race].NoPoison`.
pub fn creature_immune_poison(kind: &CreatureKind) -> bool {
    match kind {
        CreatureKind::Monster(m) => m.immunity_poison,
        _ => false,
    }
}

/// Read-only melee defense snapshot — call before mutating `creatures`.
pub fn melee_defense_snapshot(kind: &CreatureKind) -> MeleeDefenseSnapshot {
    let defend_mode = defend_fight_mode_for_target(kind);
    match kind {
        CreatureKind::Monster(m) => MeleeDefenseSnapshot {
            defense_skill: 0,
            defense_value: m.defense,
            armor: m.armor,
            defend_mode,
        },
        CreatureKind::Player(p) => MeleeDefenseSnapshot {
            defense_skill: p.skills.shielding,
            defense_value: 0,
            armor: 0,
            defend_mode,
        },
        CreatureKind::Npc(_) => MeleeDefenseSnapshot {
            defense_skill: 0,
            defense_value: 0,
            armor: 0,
            defend_mode,
        },
    }
}

/// C++ `TCombat::GetDefendDamage` — `crcombat.cc:236` (gate + probe roll).
pub fn roll_target_defense<R: Rng + ?Sized>(
    target_base: &mut CreatureBase,
    server_ms: u64,
    profile: &MechanicsProfile,
    hooks: &FormulaHooks,
    rng: &mut R,
    snap: MeleeDefenseSnapshot,
) -> i32 {
    if server_ms < target_base.earliest_defend_ms {
        return 0;
    }
    let gate_ms = defense_gate_ms(profile) as u64;
    target_base.earliest_defend_ms = target_base.last_defend_ms.saturating_add(gate_ms);
    target_base.last_defend_ms = server_ms;
    defense_value(
        profile,
        hooks,
        rng,
        snap.defense_skill,
        snap.defense_value,
        snap.defend_mode,
    )
}

/// Poison-on-hit condition after `CloseAttack` — `crcombat.cc:660`.
pub fn melee_poison_on_hit<R: Rng + ?Sized>(
    rng: &mut R,
    poison_cycles: i32,
    attack_roll: i32,
    defense_roll: i32,
    damage_done: i32,
) -> Option<ActiveCondition> {
    if poison_cycles <= 0 {
        return None;
    }
    let proc = damage_done > 0
        || (attack_roll > defense_roll && crate::sim_glibc_rand::parity_rand_mod(5) == 0);
    if !proc {
        return None;
    }
    let half = poison_cycles / 2;
    let poison_dmg = if crate::sim_glibc_rand::sim_glibc_rng_enabled() {
        crate::sim_glibc_rand::sim_random(half, poison_cycles)
    } else {
        crate::combat::uniform_random(rng, half, poison_cycles)
    };
    if poison_dmg <= 0 {
        return None;
    }
    Some(ActiveCondition {
        id: 0,
        sub_id: 0,
        ctype: ConditionType::Poison,
        data: ConditionData::Damage {
            total_rank: poison_dmg,
        },
    })
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
    if name.eq_ignore_ascii_case("fire") {
        let (base, variation) = parse_min_max_damage(node);
        return Some(SpellImpact::Damage {
            element: CombatType::Fire,
            base,
            variation,
        });
    }
    if name.eq_ignore_ascii_case("energy") {
        let (base, variation) = parse_min_max_damage(node);
        return Some(SpellImpact::Damage {
            element: CombatType::Energy,
            base,
            variation,
        });
    }
    if name.eq_ignore_ascii_case("lifedrain") {
        let (base, variation) = parse_min_max_damage(node);
        return Some(SpellImpact::Damage {
            element: CombatType::LifeDrain,
            base,
            variation,
        });
    }
    if name.eq_ignore_ascii_case("physical") {
        let (base, variation) = parse_min_max_damage(node);
        return Some(SpellImpact::Damage {
            element: CombatType::Physical,
            base,
            variation,
        });
    }
    if name.eq_ignore_ascii_case("healing") {
        let (base, variation) = parse_min_max_healing(node);
        return Some(SpellImpact::Healing { base, variation });
    }
    if name.eq_ignore_ascii_case("speed") {
        return Some(SpellImpact::Speed {
            percent: parse_attr_i32(node, "speed", 0),
            variation: parse_attr_i32(node, "speedvariation", 0),
            duration: parse_attr_i32(node, "duration", 0),
        });
    }
    if name.eq_ignore_ascii_case("drunk") {
        return Some(SpellImpact::Drunk {
            drunkness: parse_attr_i32(node, "drunkness", 0),
        });
    }

    debug!(spell_name = name, "skipping unknown monster attack spell");
    None
}

/// TVP `<attack min= max=>` — store midpoint + half-span for uniform roll at cast time.
fn parse_min_max_damage(node: &MonsterSpellNode) -> (i32, i32) {
    let min = parse_attr_i32(node, "min", 0);
    let max = parse_attr_i32(node, "max", 0);
    let min_dmg = min.abs().min(max.abs());
    let max_dmg = min.abs().max(max.abs());
    let base = (min_dmg + max_dmg) / 2;
    let variation = (max_dmg - min_dmg) / 2;
    (base, variation)
}

fn parse_min_max_healing(node: &MonsterSpellNode) -> (i32, i32) {
    let min = parse_attr_i32(node, "min", 0);
    let max = parse_attr_i32(node, "max", 0);
    let min_heal = min.min(max);
    let max_heal = min.max(max);
    let base = (min_heal + max_heal) / 2;
    let variation = (max_heal - min_heal) / 2;
    (base.max(0), variation.max(0))
}

/// Shape from XML attrs — `crnonpl.cc:2609`.
fn default_shape_for_node(name: &str, node: &MonsterSpellNode) -> SpellShape {
    let radius = parse_attr_i32(node, "radius", 0);
    let range = parse_attr_i32(node, "range", 0);
    let target = node
        .attributes
        .get("target")
        .and_then(|s| s.parse::<i32>().ok());
    if radius > 0 && target == Some(0) {
        return SpellShape::Origin;
    }
    if target == Some(1) || range > 1 {
        return SpellShape::Victim;
    }
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
    } else if name.eq_ignore_ascii_case("death") {
        Some(ShootEffect::Unknown as u8)
    } else if name.eq_ignore_ascii_case("spear") {
        Some(ShootEffect::Spear as u8)
    } else if name.eq_ignore_ascii_case("bolt") {
        Some(ShootEffect::Bolt as u8)
    } else if name.eq_ignore_ascii_case("arrow") {
        Some(ShootEffect::Arrow as u8)
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
            radius: 0,
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

    fn test_creature_base() -> crate::creature::base::CreatureBase {
        use std::collections::VecDeque;
        use tfs_rust_common::enums::{Direction, SkullType};
        use tfs_rust_common::Position;

        crate::creature::base::CreatureBase {
            name: "Test".into(),
            position: Position::new(100, 100, 7),
            direction: Direction::South,
            health: 100,
            max_health: 100,
            outfit: Default::default(),
            speed: 200,
            base_speed: 200,
            skull: SkullType::None,
            drunkenness: 0,
            active_conditions: Vec::new(),
            walk_queue: VecDeque::new(),
            last_step: None,
            last_step_cost: 1,
            last_step_ground_speed: 150,
            next_walk_check: None,
            next_wakeup: None,
            last_step_server_ms: None,
            walk_timer: Default::default(),
            cancel_next_walk: false,
            force_update_follow_path: false,
            walk_update_ticks: 0,
            is_updating_path: false,
            has_follow_path: false,
            movement_blocked: false,
            stairhop_blocked_until: None,
            follow_target: None,
            attack_target: None,
            master: None,
            damage_map: Default::default(),
            think_check_bucket: None,
            earliest_attack_ms: 0,
            earliest_defend_ms: 0,
            last_defend_ms: 0,
            todo: Default::default(),
        }
    }

    #[test]
    fn test_defense_gate_allows_pair_then_blocks() {
        use crate::formulas::{FormulaHooks, Mechanics};
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        use tfs_rust_common::ProtocolVersion;

        let mechanics = Mechanics::for_version(ProtocolVersion::V772);
        let hooks = FormulaHooks::default();
        let mut base = test_creature_base();
        let snap = MeleeDefenseSnapshot {
            defense_skill: 0,
            defense_value: 10,
            armor: 0,
            defend_mode: FightMode::Balanced,
        };
        let mut rng = StdRng::seed_from_u64(7);

        let _ = roll_target_defense(
            &mut base,
            1000,
            &mechanics.profile,
            &hooks,
            &mut rng,
            snap,
        );
        assert_eq!(base.last_defend_ms, 1000);
        assert_eq!(base.earliest_defend_ms, 2000);

        let _ = roll_target_defense(
            &mut base,
            2100,
            &mechanics.profile,
            &hooks,
            &mut rng,
            snap,
        );
        assert_eq!(base.last_defend_ms, 2100);
        assert_eq!(base.earliest_defend_ms, 3000);

        let blocked = roll_target_defense(
            &mut base,
            2200,
            &mechanics.profile,
            &hooks,
            &mut rng,
            snap,
        );
        assert_eq!(blocked, 0, "defense must gate until LastDefendTime + 2000 ms");
    }

    #[test]
    fn test_defend_fight_mode_non_attacker_is_defensive() {
        use crate::creature::{CreatureKind, Monster};
        use tfs_rust_common::Position;

        let base = test_creature_base();
        let m = Monster::with_config(base, Position::new(100, 100, 7), MonsterAiConfig::default());
        let snap = melee_defense_snapshot(&CreatureKind::Monster(m));
        assert_eq!(snap.defend_mode, FightMode::Defensive);
    }

    #[test]
    fn test_e4_marid_fire_energy_parse() {
        let mtype = load_monster_type("marid");
        let cfg = MonsterAiConfig::from_monster_type(&mtype);
        let fire = cfg
            .spells
            .iter()
            .find(|s| matches!(s.impact, SpellImpact::Damage { element: CombatType::Fire, .. }))
            .expect("marid fire spell");
        assert_eq!(fire.delay, 2);
        assert_eq!(fire.range, 7);
        assert_eq!(fire.shape, SpellShape::Victim);
        assert!(matches!(
            fire.impact,
            SpellImpact::Damage {
                element: CombatType::Fire,
                base: 75,
                variation: 35,
            }
        ));
        assert_eq!(fire.shoot_effect, Some(ShootEffect::Fire as u8));

        let energy_cond = cfg
            .spells
            .iter()
            .find(|s| {
                matches!(
                    s.impact,
                    SpellImpact::Condition {
                        condition: ConditionType::Energy,
                        ..
                    }
                )
            })
            .expect("marid energycondition");
        assert_eq!(energy_cond.shape, SpellShape::Origin);
        assert_eq!(energy_cond.radius, 3);
    }

    #[test]
    fn test_e4_drunk_and_speed_parse() {
        let node = spell_node(
            "drunk",
            &[("delay", "5"), ("range", "7"), ("duration", "60000"), ("drunkness", "120")],
            &[("shooteffect", "energy")],
        );
        let spell = MonsterSpell::try_from_node(&node).expect("drunk spell");
        assert_eq!(spell.shape, SpellShape::Victim);
        assert!(matches!(spell.impact, SpellImpact::Drunk { drunkness: 120 }));

        let speed_node = spell_node(
            "speed",
            &[
                ("delay", "8"),
                ("duration", "15000"),
                ("speed", "-75"),
                ("speedvariation", "25"),
                ("range", "7"),
            ],
            &[],
        );
        let speed = MonsterSpell::try_from_node(&speed_node).expect("speed spell");
        assert!(matches!(
            speed.impact,
            SpellImpact::Speed {
                percent: -75,
                variation: 25,
                duration: 15000,
            }
        ));
    }

    #[test]
    fn test_cobra_poison_immunity_from_xml() {
        let mtype = load_monster_type("cobra");
        let cfg = MonsterAiConfig::from_monster_type(&mtype);
        assert!(cfg.immunity_poison, "cobra.xml immunity poison=1");
        let rat = MonsterAiConfig::from_monster_type(&load_monster_type("rat"));
        assert!(!rat.immunity_poison);
    }

    #[test]
    fn test_creature_immune_poison_respects_spawn_flag() {
        use crate::creature::{CreatureKind, Monster};
        use tfs_rust_common::Position;

        let mut cfg = MonsterAiConfig::default();
        cfg.immunity_poison = true;
        let m = Monster::with_config(
            test_creature_base(),
            Position::new(100, 100, 7),
            cfg,
        );
        assert!(creature_immune_poison(&CreatureKind::Monster(m)));
    }
}
