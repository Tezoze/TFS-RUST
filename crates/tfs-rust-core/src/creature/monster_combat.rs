//! Runtime monster combat data converted from content XML spell nodes.
//!
//! C++ reference: `cr.hh` `TSpellData`; `crnonpl.cc:2521-2667` CASTING shape/impact switches;
//! `crcombat.cc:647` `CloseAttack`, `:660` poison-on-hit.

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

/// 772 `FIELD_TYPE_*` — `magic.hh:9–11` (fire/poison/energy for monster IMPACT_FIELD).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterFieldType {
    Fire = 1,
    Poison = 2,
    Energy = 3,
}

/// C++ `IMPACT_*` — `crnonpl.cc:2536`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellImpact {
    Damage {
        element: CombatType,
        base: i32,
        variation: i32,
    },
    /// 772 `IMPACT_FIELD` / `TFieldImpact` — places a magic field on the tile.
    Field {
        field_type: MonsterFieldType,
    },
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
        /// XML `force` — passed to place-creature (`placeCreature` extended force).
        force: bool,
    },
    /// C++ `IMPACT_DRUNKEN` — sets target `drunkenness` (`crnonpl.cc:2553`).
    Drunk {
        drunkness: i32,
    },
}

impl SpellImpact {
    /// 772 `TImpact::isAggressive` (`magic.cc:119`) — base returns `true`.
    /// Only `THealingImpact` overrides to `false` (`magic.cc:210`).
    /// Used by the CASTING gate (`crnonpl.cc:2682`):
    /// `if(!Impact->isAggressive() || (this->Target != 0 && this->Target != this->Master))`.
    pub fn is_aggressive(&self) -> bool {
        !matches!(self, SpellImpact::Healing { .. })
    }
}

/// Runtime spell entry for idle CASTING (E4) and target-range checks.
///
/// C++ reference: `cr.hh:55` `TSpellData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterSpell {
    /// Cast gate: `rand() % Delay == 0` — `crnonpl.cc:2527`.
    pub delay: i32,
    pub range: i32,
    /// Disc radius for `Origin` / `Destination` — 772 `ExecuteCircleSpell` rings `0..=R`.
    ///
    /// TFS XML `radius` uses `AreaCombat::setupArea(R)` (rings `1..=R`); that is one greater
    /// than the 772 ring index (`setupArea(R)` ≡ `disc_offsets(R-1)`). Stored value is the
    /// 772 radius after conversion at parse time.
    pub radius: i32,
    /// Forward range for `Angle` (beam) shapes — XML `length`.
    /// 772 `AngleShapeSpell` `Range` (`magic.cc:550`, `ShapeParam2`).
    pub length: i32,
    /// Cone half-angle for `Angle` shapes — XML `spread`.
    /// 772 `AngleShapeSpell` `Angle` (`magic.cc:550`, `ShapeParam1`); `Left/Right = ±Forward*Angle/90`.
    pub spread: i32,
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
    /// `<immunity fire="1"/>` — `crmain.cc:549` `RaceData[Race].NoBurning`.
    pub immunity_fire: bool,
    /// `<immunity energy="1"/>` — `crmain.cc:550` `RaceData[Race].NoEnergy`.
    pub immunity_energy: bool,
    /// `<immunity lifedrain="1"/>` — `crmain.cc:619` `RaceData[Race].NoLifeDrain`. PC-3 (M3′).
    pub immunity_life_drain: bool,
    /// `<immunity invisible="1"/>` — `crmain.cc:1493` `RaceData[Race].SeeInvisible`.
    pub see_invisible: bool,
    /// `<immunity physical="1"/>` — `crmain.cc:615` `RaceData[Race].NoHit`.
    pub immunity_physical: bool,
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
        // Shape detection needs the raw TFS XML radius (`radius > 0` → Origin/Destination).
        // Disc execution uses 772 ring index: `setupArea(R)` ≡ rings `0..=R-1`
        // (`circles.rs` / `combat.cpp:1391`; giant spider XML `radius="1"` → `.mon` `Destination(...,0,...)`).
        let xml_radius = parse_attr_i32(node, "radius", 0);
        let length = parse_attr_i32(node, "length", 0);
        let spread = parse_attr_i32(node, "spread", 0);
        let min_cycle = parse_attr_i32(node, "mincycle", 0);
        let shape = default_shape_for_node(name, node);
        let radius = match shape {
            SpellShape::Origin | SpellShape::Destination if xml_radius > 0 => xml_radius - 1,
            _ => xml_radius,
        };
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
            length,
            spread,
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
        immunity_fire: mtype.defenses.immunity_fire,
        immunity_energy: mtype.defenses.immunity_energy,
        immunity_life_drain: mtype.defenses.immunity_life_drain,
        see_invisible: mtype.defenses.see_invisible,
        immunity_physical: mtype.defenses.immunity_physical,
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

    // XML `<summons>` → 772 CASTING `IMPACT_SUMMON` spells (`crnonpl.cc:2647`).
    // Origin radius 0 so `ExecuteCircleSpell` calls `handleField` once at the actor
    // (`magic.cc:503` / `:483`). Delay modulus matches `SpellData.Delay`.
    for block in &mtype.summons {
        let max = if block.max > 0 {
            block.max as i32
        } else {
            mtype.max_summons.max(1) as i32
        };
        snap.spells.push(MonsterSpell {
            delay: block.delay.max(1),
            range: 0,
            radius: 0,
            length: 0,
            spread: 0,
            min_cycle: 0,
            shape: SpellShape::Origin,
            impact: SpellImpact::Summon {
                race: block.name.clone(),
                max,
                force: block.force,
            },
            shoot_effect: None,
            area_effect: None,
        });
    }

    // 772 `RaceData.Spells` is one list (attack then defense) — `crnonpl.cc:2521-2667`.
    // TFS XML splits `<attacks>` / `<defenses>`; merge at spawn so idle CASTING does not
    // rebuild defense spells each pass (audit IDLE-1).
    for node in &mtype.defenses.spells {
        if let Some(spell) = MonsterSpell::try_from_node(node) {
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
    /// True when the defender has a shield equipped — gates shielding `Increase`
    /// (`crcombat.cc:259` `Shield != NONE`).
    pub has_shield: bool,
}

/// Effective defend fight mode — `crcombat.cc:245-247`.
///
/// `Following || AttackDest == 0` → `ATTACK_MODE_DEFENSIVE`.
/// Player `Following` is `follow_target.is_some()` (set by `SetAttackDest(…, Follow=true)`).
/// Monster chase via `follow_target` is **not** `Combat.Following` — monsters stay Balanced
/// when they have an attack target (`idle_stimulus` / summon inheritance comments).
pub fn defend_fight_mode_for_target(kind: &CreatureKind) -> FightMode {
    let base = kind.base();
    let following = matches!(kind, CreatureKind::Player(_)) && base.follow_target.is_some();
    if following || base.attack_target.is_none() {
        return FightMode::Defensive;
    }
    // Player fight-mode from `0xA7` packet; monsters default to BALANCED (`crcombat.cc:13`).
    match kind {
        CreatureKind::Player(p) => p.attack_mode,
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
            // C++ `GetDefendValue` with no shield/weapon → `WEAPON_NONE` → `SKILL_FIST`
            // (`crcombat.cc:213,153`); `ProbeValue` uses `Skills[SKILL_FIST]->Get()` which for
            // monsters is their `FistFighting` skill (= `race_melee_skill` / `melee_skill`).
            // World-aware path: [`GameWorld::melee_defense_snapshot_for`].
            defense_skill: m.race_melee_skill,
            defense_value: m.race_defense,
            armor: m.race_armor,
            defend_mode,
            has_shield: false,
        },
        CreatureKind::Player(p) => MeleeDefenseSnapshot {
            // C++ `GetDefendValue` without shield — `WEAPON_NONE` → `SKILL_FIST` (`crcombat.cc:191-217`).
            // World-aware path sets `has_shield` via `melee_defense_snapshot_for`.
            defense_skill: p.skills.fist,
            defense_value: p.sim_melee_defense,
            armor: 0,
            defend_mode,
            has_shield: false,
        },
        CreatureKind::Npc(_) => MeleeDefenseSnapshot {
            defense_skill: 0,
            defense_value: 0,
            armor: 0,
            defend_mode,
            has_shield: false,
        },
    }
}

impl crate::game_world::GameWorld {
    /// World-aware target defense/armor snapshot for melee strikes — PC-2.
    ///
    /// Players: [`Self::player_get_defend_value`] + [`Self::player_get_armor_strength`].
    /// Monsters: [`Self::monster_get_defend_value`] + [`Self::monster_get_armor_strength`]
    /// (`GetDefendValue` / `GetArmorStrength`, `crcombat.cc:191`, `:286`).
    pub(crate) fn melee_defense_snapshot_for(
        &self,
        target_id: crate::ids::CreatureId,
    ) -> MeleeDefenseSnapshot {
        let Some(kind) = self.creatures.get(target_id) else {
            return MeleeDefenseSnapshot {
                defense_skill: 0,
                defense_value: 0,
                armor: 0,
                defend_mode: FightMode::Defensive,
                has_shield: false,
            };
        };
        match kind {
            crate::creature::CreatureKind::Player(p) => {
                let (def_value, skill_nr) = self.player_get_defend_value(target_id);
                let armor = self.player_get_armor_strength(target_id);
                let has_shield = self.player_get_shield(target_id).is_some();
                MeleeDefenseSnapshot {
                    defense_skill: p.skill_level(skill_nr),
                    defense_value: def_value,
                    armor,
                    defend_mode: defend_fight_mode_for_target(kind),
                    has_shield,
                }
            }
            crate::creature::CreatureKind::Monster(_) => {
                let (def_value, def_skill) = self.monster_get_defend_value(target_id);
                let armor = self.monster_get_armor_strength(target_id);
                let has_shield = self.monster_has_shield(target_id);
                MeleeDefenseSnapshot {
                    defense_skill: def_skill,
                    defense_value: def_value,
                    armor,
                    defend_mode: defend_fight_mode_for_target(kind),
                    has_shield,
                }
            }
            _ => melee_defense_snapshot(kind),
        }
    }
}

/// C++ `TCombat::GetDefendDamage` — `crcombat.cc:236` (gate + probe roll).
pub fn roll_target_defense(
    target_base: &mut CreatureBase,
    server_ms: u64,
    profile: &MechanicsProfile,
    hooks: &FormulaHooks,
    snap: MeleeDefenseSnapshot,
    parity: &crate::sim_glibc_rand::GlibcRngState,
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
        snap.defense_skill,
        snap.defense_value,
        snap.defend_mode,
        parity,
    )
}

/// Poison-on-hit condition after `CloseAttack` — `crcombat.cc:660`.
pub fn melee_poison_on_hit(
    poison_cycles: i32,
    attack_roll: i32,
    defense_roll: i32,
    damage_done: i32,
    parity: &crate::sim_glibc_rand::GlibcRngState,
) -> Option<ActiveCondition> {
    if poison_cycles <= 0 {
        return None;
    }
    let proc = damage_done > 0
        || (attack_roll > defense_roll && {
            #[cfg(any(test, feature = "sim"))]
            {
                if crate::sim_glibc_rand::sim_glibc_rng_enabled() {
                    crate::sim_glibc_rand::sim_rand_mod(5) == 0
                } else {
                    parity.rand_mod(5) == 0
                }
            }
            #[cfg(not(any(test, feature = "sim")))]
            {
                parity.rand_mod(5) == 0
            }
        });
    if !proc {
        return None;
    }
    let half = poison_cycles / 2;
    let poison_dmg = crate::combat::rng::uniform_random_glibc(parity, half, poison_cycles);
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
        timer_rounds_left: None,
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
    if name.eq_ignore_ascii_case("poisonfield") {
        return Some(SpellImpact::Field {
            field_type: MonsterFieldType::Poison,
        });
    }
    if name.eq_ignore_ascii_case("firefield") {
        return Some(SpellImpact::Field {
            field_type: MonsterFieldType::Fire,
        });
    }
    if name.eq_ignore_ascii_case("energyfield") {
        return Some(SpellImpact::Field {
            field_type: MonsterFieldType::Energy,
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

/// Shape from XML attrs — `crnonpl.cc:2609` `SHAPE_*`; TFS data-pack mapping
/// (`monsters.cpp:191` `length`+`spread` → directional area = 772 `SHAPE_ANGLE`;
/// `monsters.cpp:217` `radius` → area; `target="1"` → needs victim tile).
fn default_shape_for_node(name: &str, node: &MonsterSpellNode) -> SpellShape {
    let radius = parse_attr_i32(node, "radius", 0);
    let range = parse_attr_i32(node, "range", 0);
    let length = parse_attr_i32(node, "length", 0);
    let target = node
        .attributes
        .get("target")
        .and_then(|s| s.parse::<i32>().ok());
    // TFS `length`+`spread` → `AreaCombat::setupArea` + `needDirection` = 772 beam (`SHAPE_ANGLE`).
    if length > 0 {
        return SpellShape::Angle;
    }
    if radius > 0 && target == Some(0) {
        return SpellShape::Origin;
    }
    // TFS `target="1"` + `radius` → area around victim tile = 772 `SHAPE_DESTINATION`
    // (`DestinationShapeSpell` → `CircleShapeSpell` at victim pos, `magic.cc:537`).
    if target == Some(1) && radius > 0 {
        return SpellShape::Destination;
    }
    // Single-target on the victim: XML `target="1"` **or** any positive `range`
    // (including melee-range `range="1"` — ghoul lifedrain / 772 `Victim (1,0,0)`).
    // Previously `range > 1` left `range="1"` as Actor → life drain hit the caster.
    if target == Some(1) || range > 0 {
        return SpellShape::Victim;
    }
    if name.ends_with("condition") && range > 0 {
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

/// TFS/TVP `shooteffect` name → 772 `CONST_ANI_*` wire byte (`tools.cpp` `shootTypeNames`).
///
/// `"poison"` / `"earth"` → `CONST_ANI_POISON` (15); `"poisonarrow"` → `CONST_ANI_POISONARROW` (6).
/// Do not collapse these — giant spider / cobra use poison missile, not the arrow.
fn parse_shoot_effect_name(name: &str) -> Option<u8> {
    if name.eq_ignore_ascii_case("poison") || name.eq_ignore_ascii_case("earth") {
        Some(ShootEffect::Poison as u8)
    } else if name.eq_ignore_ascii_case("poisonarrow") {
        Some(ShootEffect::PoisonArrow as u8)
    } else if name.eq_ignore_ascii_case("fire") {
        Some(ShootEffect::Fire as u8)
    } else if name.eq_ignore_ascii_case("energy") {
        Some(ShootEffect::Energy as u8)
    } else if name.eq_ignore_ascii_case("death") {
        Some(ShootEffect::Death as u8)
    } else if name.eq_ignore_ascii_case("spear") {
        Some(ShootEffect::Spear as u8)
    } else if name.eq_ignore_ascii_case("bolt") {
        Some(ShootEffect::Bolt as u8)
    } else if name.eq_ignore_ascii_case("arrow") {
        Some(ShootEffect::Arrow as u8)
    } else if name.eq_ignore_ascii_case("burstarrow") {
        Some(ShootEffect::BurstArrow as u8)
    } else if name.eq_ignore_ascii_case("throwingstar") {
        Some(ShootEffect::ThrowingStar as u8)
    } else if name.eq_ignore_ascii_case("snowball") {
        Some(ShootEffect::Snowball as u8)
    } else if name.eq_ignore_ascii_case("powerbolt") {
        Some(ShootEffect::PowerBolt as u8)
    } else {
        debug!(shooteffect = name, "unknown monster shooteffect");
        None
    }
}

/// TFS data-pack `areaeffect` name → 772 `CONST_ME_*` wire byte
/// (`tools.cpp:497` `magicEffectNames`; 772 `const.h:11-35`). Returns the raw
/// on-wire byte used by `broadcast_magic_effect` (`sendMagicEffect`).
/// Names beyond the 772 client range (>25) are dropped — they would not render.
fn parse_area_effect_name(name: &str) -> Option<u8> {
    let byte = match name.to_ascii_lowercase().as_str() {
        "redspark" => 1,        // CONST_ME_DRAWBLOOD
        "bluebubble" => 2,      // CONST_ME_LOSEENERGY
        "poff" => 3,            // CONST_ME_POFF
        "yellowspark" => 4,     // CONST_ME_BLOCKHIT
        "explosionarea" => 5,   // CONST_ME_EXPLOSIONAREA
        "explosion" => 6,       // CONST_ME_EXPLOSIONHIT
        "firearea" => 7,        // CONST_ME_FIREAREA
        "yellowbubble" => 8,    // CONST_ME_YELLOW_RINGS
        "greenbubble" => 9,     // CONST_ME_GREEN_RINGS
        "blackspark" => 10,     // CONST_ME_HITAREA
        "teleport" => 11,       // CONST_ME_TELEPORT
        "energy" => 12,         // CONST_ME_ENERGYHIT
        "blueshimmer" => 13,    // CONST_ME_MAGIC_BLUE
        "redshimmer" => 14,     // CONST_ME_MAGIC_RED
        "greenshimmer" => 15,   // CONST_ME_MAGIC_GREEN
        "fire" => 16,           // CONST_ME_HITBYFIRE
        "greenspark" => 17,     // CONST_ME_HITBYPOISON
        "mortarea" => 18,       // CONST_ME_MORTAREA
        "greennote" => 19,      // CONST_ME_SOUND_GREEN
        "rednote" => 20,        // CONST_ME_SOUND_RED
        "poison" => 21,         // CONST_ME_POISONAREA
        "yellownote" => 22,     // CONST_ME_SOUND_YELLOW
        "purplenote" => 23,     // CONST_ME_SOUND_PURPLE
        "bluenote" => 24,       // CONST_ME_SOUND_BLUE
        "whitenote" => 25,      // CONST_ME_SOUND_WHITE
        _ => {
            debug!(areaeffect = name, "monster areaeffect not mapped to a 772 effect");
            return None;
        }
    };
    Some(byte)
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

    fn spell_node(
        name: &str,
        attrs: &[(&str, &str)],
        children: &[(&str, &str)],
    ) -> MonsterSpellNode {
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
        let db = MonsterDatabase::load_dir(&Path::new(manifest).join("../../data/monster"), &items)
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

    /// XML `<summons>` merges into CASTING as `SpellImpact::Summon` / Origin r=0
    /// (`crnonpl.cc:2647`, `magic.cc` `TSummonImpact`).
    #[test]
    fn test_giant_spider_summon_spell_from_xml() {
        let mtype = load_monster_type("giant spider");
        assert_eq!(mtype.max_summons, 2);
        assert_eq!(mtype.summons.len(), 1);
        assert_eq!(mtype.summons[0].name, "Poison Spider");
        assert_eq!(mtype.summons[0].delay, 10);
        assert_eq!(mtype.summons[0].max, 2);

        let cfg = MonsterAiConfig::from_monster_type(&mtype);
        let summon = cfg
            .spells
            .iter()
            .find(|s| matches!(s.impact, SpellImpact::Summon { .. }))
            .expect("summon spell from <summons>");
        assert_eq!(summon.delay, 10);
        assert_eq!(summon.shape, SpellShape::Origin);
        assert_eq!(summon.radius, 0);
        assert!(matches!(
            &summon.impact,
            SpellImpact::Summon { race, max, force: false }
                if race == "Poison Spider" && *max == 2
        ));
    }

    /// Giant spider `poisonfield` → `SpellImpact::Field { Poison }` Destination
    /// (`crnonpl.cc:2598` `IMPACT_FIELD` / `TFieldImpact`).
    ///
    /// 772 `giantspider.mon`: `Destination (7, 15, 0, 0) -> Field (2) : 6`
    /// — range 7, `CONST_ANI_POISON` (15), disc radius 0 (XML `radius="1"` → `setupArea(1)`).
    #[test]
    fn test_giant_spider_poisonfield_from_xml() {
        let mtype = load_monster_type("giant spider");
        let cfg = MonsterAiConfig::from_monster_type(&mtype);
        let field = cfg
            .spells
            .iter()
            .find(|s| matches!(s.impact, SpellImpact::Field { .. }))
            .expect("poisonfield spell");
        assert_eq!(field.delay, 6);
        assert_eq!(field.range, 7);
        assert_eq!(field.radius, 0, "TFS radius=1 → 772 disc radius 0 (center only)");
        assert_eq!(field.shape, SpellShape::Destination);
        assert!(matches!(
            field.impact,
            SpellImpact::Field {
                field_type: MonsterFieldType::Poison
            }
        ));
        assert_eq!(field.shoot_effect, Some(ShootEffect::Poison as u8));
    }

    /// Field casters: TFS XML `radius` / `shooteffect` → 772 `.mon` Destination params.
    ///
    /// Refs: `demon.mon` `Destination (7,4,0,0) -> Field (1)`; `dragonlord.mon`
    /// `(7,4,3,0)`; `warlock.mon` `(7,4,1,0)` + `(7,4,0,0)`; `demodras.mon` `(7,4,5,0)`.
    #[test]
    fn test_field_casters_radius_and_shoot_match_772_mon() {
        let cases: &[(&str, &[(i32, i32, u8, MonsterFieldType)])] = &[
            // name, [(delay, 772_radius, shoot, field)]
            (
                "demon",
                &[(7, 0, ShootEffect::Fire as u8, MonsterFieldType::Fire)],
            ),
            (
                "dragon lord",
                &[(7, 3, ShootEffect::Fire as u8, MonsterFieldType::Fire)],
            ),
            (
                "fire elemental",
                &[(3, 0, ShootEffect::Fire as u8, MonsterFieldType::Fire)],
            ),
            (
                "witch",
                &[(8, 0, ShootEffect::Fire as u8, MonsterFieldType::Fire)],
            ),
            (
                "warlock",
                &[
                    (5, 1, ShootEffect::Fire as u8, MonsterFieldType::Fire),
                    (7, 0, ShootEffect::Fire as u8, MonsterFieldType::Fire),
                ],
            ),
            (
                "orshabaal",
                &[(11, 3, ShootEffect::Fire as u8, MonsterFieldType::Fire)],
            ),
            (
                "demodras",
                &[(10, 5, ShootEffect::Fire as u8, MonsterFieldType::Fire)],
            ),
            (
                "minotaur mage",
                &[(9, 0, ShootEffect::Energy as u8, MonsterFieldType::Energy)],
            ),
            (
                "merlkin",
                // 772 uses energy missile (5) for poison field — XML `shooteffect="energy"`.
                &[(7, 0, ShootEffect::Energy as u8, MonsterFieldType::Poison)],
            ),
            (
                "the old widow",
                &[(11, 3, ShootEffect::Poison as u8, MonsterFieldType::Poison)],
            ),
        ];

        for &(name, expected) in cases {
            let mtype = load_monster_type(name);
            let cfg = MonsterAiConfig::from_monster_type(&mtype);
            let mut fields: Vec<_> = cfg
                .spells
                .iter()
                .filter(|s| matches!(s.impact, SpellImpact::Field { .. }))
                .collect();
            fields.sort_by_key(|s| s.delay);
            assert_eq!(
                fields.len(),
                expected.len(),
                "{name}: field spell count"
            );
            for (spell, &(delay, radius, shoot, field_type)) in fields.iter().zip(expected.iter()) {
                assert_eq!(spell.delay, delay, "{name} delay");
                assert_eq!(spell.radius, radius, "{name} 772 disc radius");
                assert_eq!(spell.shape, SpellShape::Destination, "{name} shape");
                assert_eq!(spell.shoot_effect, Some(shoot), "{name} shoot");
                assert!(
                    matches!(
                        spell.impact,
                        SpellImpact::Field { field_type: ft } if ft == field_type
                    ),
                    "{name} field type"
                );
            }
        }
    }

    #[test]
    fn test_dragon_merges_defense_spells_at_spawn() {
        let mtype = load_monster_type("dragon");
        let cfg = MonsterAiConfig::from_monster_type(&mtype);
        assert_eq!(cfg.spells.len(), 3, "2 attacks + 1 healing defense");
        assert!(matches!(
            cfg.spells.last().map(|s| &s.impact),
            Some(SpellImpact::Healing { .. })
        ));
    }

    /// Audit #7: many spell-capable monsters share prebuilt spells; idle passes do not rebuild.
    #[test]
    fn spell_idle_burst_uses_prebuilt_spells() {
        use std::time::Instant;

        use crate::test_world::support::{
            beat_driven_test_world, ensure_walkable_tile, insert_monster_with_config,
        };
        use tfs_rust_common::Position;

        let mtype = load_monster_type("dragon");
        let cfg = MonsterAiConfig::from_monster_type(&mtype);
        let spell_len = cfg.spells.len();
        assert!(spell_len >= 2, "dragon must have merged attack/defense spells");

        let mut world = beat_driven_test_world();
        let mut monsters = Vec::new();
        for i in 0..32 {
            let pos = Position::new(200 + (i % 8) as u16, 200 + (i / 8) as u16, 7);
            ensure_walkable_tile(&mut world.map, pos, 100);
            let id = insert_monster_with_config(&mut world, "Dragon", pos, 200, cfg.clone());
            monsters.push(id);
        }

        // Snapshot spell lengths at spawn — IDLE-1 must not rebuild per pass.
        let spawn_lens: Vec<usize> = monsters
            .iter()
            .map(|&id| {
                world
                    .creatures
                    .get(id)
                    .and_then(|k| match k {
                        crate::creature::CreatureKind::Monster(m) => Some(m.spells.len()),
                        _ => None,
                    })
                    .unwrap_or(0)
            })
            .collect();
        assert!(spawn_lens.iter().all(|&n| n == spell_len));

        let start = Instant::now();
        for &id in &monsters {
            for _ in 0..4 {
                world.request_idle_stimulus(id);
                // Drain-style: run idle if queue empty.
                if world.creature_todo_queue_empty(id) {
                    world.monster_idle_stimulus(id);
                }
            }
        }
        let elapsed = start.elapsed();

        for (i, &id) in monsters.iter().enumerate() {
            let len = world
                .creatures
                .get(id)
                .and_then(|k| match k {
                    crate::creature::CreatureKind::Monster(m) => Some(m.spells.len()),
                    _ => None,
                })
                .unwrap_or(0);
            assert_eq!(
                len, spawn_lens[i],
                "idle must not rebuild/grow spell vec for monster {i}"
            );
        }
        assert!(
            elapsed.as_secs() < 3,
            "32×4 idle spell passes must stay fast (took {elapsed:?})"
        );
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
        // cobra.xml `shooteffect="poison"` / cobra.mon `Victim (5, 15, 0)` → CONST_ANI_POISON.
        assert_eq!(spell.shoot_effect, Some(ShootEffect::Poison as u8));
    }

    /// Dragon fire wave (`length`+`spread`) → `Angle`; fireball (`target`+`range`+`radius`)
    /// → `Destination`. 772 refs: `dragon.mon` `Angle(30,8,7)` / `Destination(7,4,3,7)`.
    #[test]
    fn test_e0_dragon_fire_spells_shape_mapping() {
        let mtype = load_monster_type("dragon");
        let cfg = MonsterAiConfig::from_monster_type(&mtype);
        // melee + two fire spells (wave + fireball).
        assert!(cfg.spells.len() >= 2, "dragon must parse its fire spells");

        let wave = cfg
            .spells
            .iter()
            .find(|s| s.shape == SpellShape::Angle)
            .expect("dragon fire wave must parse as Angle (length+spread)");
        assert_eq!(wave.length, 8, "length → 772 Range");
        assert_eq!(wave.spread, 3, "spread → 772 Angle/10");
        assert_eq!(wave.delay, 9);
        assert_eq!(wave.area_effect, Some(7), "firearea → CONST_ME_FIREAREA 7");
        assert!(matches!(wave.impact, SpellImpact::Damage { element: CombatType::Fire, .. }));

        let fireball = cfg
            .spells
            .iter()
            .find(|s| s.shape == SpellShape::Destination)
            .expect("dragon fireball must parse as Destination (target+radius)");
        assert_eq!(fireball.range, 7);
        // XML `radius="4"` → 772 disc radius 3 (`Destination (7, 4, 3, 7)`).
        assert_eq!(fireball.radius, 3);
        assert_eq!(fireball.shoot_effect, Some(ShootEffect::Fire as u8));
        assert!(matches!(fireball.impact, SpellImpact::Damage { element: CombatType::Fire, .. }));
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
            length: 0,
            spread: 0,
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
            var_speed: 0,
            skull: SkullType::None,
            drunkenness: 0,
            active_conditions: Vec::new(),
            walk_queue: VecDeque::new(),
            walk_destinations: VecDeque::new(),
            last_step: None,
            last_step_cost: 1,
            last_step_ground_speed: 150,
            next_wakeup: None,
            last_step_server_ms: None,
            earliest_walk_server_ms: 0,
            earliest_spell_server_ms: 0,
            earliest_multiuse_server_ms: 0,
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
            earliest_attack_ms: 0,
        latest_attack_round: 0,
            earliest_defend_ms: 0,
            last_defend_ms: 0,
            learning_points: 0,
            todo: Default::default(),
            chase_mode: Default::default(),
            last_auto_walk_armed_ms: u64::MAX,
        }
    }

    #[test]
    fn test_defense_gate_allows_pair_then_blocks() {
        use crate::formulas::{FormulaHooks, Mechanics};
        use crate::sim_glibc_rand::GlibcRngState;
        use tfs_rust_common::ProtocolVersion;

        let mechanics = Mechanics::for_version(ProtocolVersion::V772);
        let hooks = FormulaHooks::default();
        let mut base = test_creature_base();
        let snap = MeleeDefenseSnapshot {
            defense_skill: 0,
            defense_value: 10,
            armor: 0,
            defend_mode: FightMode::Balanced,
            has_shield: false,
        };
        let parity = GlibcRngState::seed(7);

        let _ = roll_target_defense(
            &mut base,
            1000,
            &mechanics.profile,
            &hooks,
            snap,
            &parity,
        );
        assert_eq!(base.last_defend_ms, 1000);
        assert_eq!(base.earliest_defend_ms, 2000);

        let _ = roll_target_defense(
            &mut base,
            2100,
            &mechanics.profile,
            &hooks,
            snap,
            &parity,
        );
        assert_eq!(base.last_defend_ms, 2100);
        assert_eq!(base.earliest_defend_ms, 3000);

        let blocked = roll_target_defense(
            &mut base,
            2200,
            &mechanics.profile,
            &hooks,
            snap,
            &parity,
        );
        assert_eq!(
            blocked, 0,
            "defense must gate until LastDefendTime + 2000 ms"
        );
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

    /// Player Following forces DEFENSIVE even when AttackDest is set (`crcombat.cc:245-247`).
    #[test]
    fn test_defend_fight_mode_player_following_is_defensive() {
        use crate::creature::CreatureKind;
        use crate::ids::CreatureId;
        use crate::sim_harness::sim_hero_player;
        use tfs_rust_common::Position;

        let dummy = CreatureId::default();
        let mut player = sim_hero_player("Hero", Position::new(100, 100, 7));
        player.attack_mode = FightMode::Offensive;
        player.base.attack_target = Some(dummy);
        player.base.follow_target = Some(dummy);
        assert_eq!(
            defend_fight_mode_for_target(&CreatureKind::Player(player)),
            FightMode::Defensive,
            "Following must force DEFENSIVE defend mode"
        );
    }

    /// Monster chase `follow_target` is not Combat.Following — keep Balanced when attacking.
    #[test]
    fn test_defend_fight_mode_monster_chase_follow_stays_balanced() {
        use crate::creature::{CreatureKind, Monster};
        use crate::ids::CreatureId;
        use tfs_rust_common::Position;

        let mut base = test_creature_base();
        let dummy = CreatureId::default();
        base.attack_target = Some(dummy);
        base.follow_target = Some(dummy); // pathfinding chase, not Following
        let m = Monster::with_config(base, Position::new(100, 100, 7), MonsterAiConfig::default());
        assert_eq!(
            defend_fight_mode_for_target(&CreatureKind::Monster(m)),
            FightMode::Balanced
        );
    }

    #[test]
    fn test_e4_marid_fire_energy_parse() {
        let mtype = load_monster_type("marid");
        let cfg = MonsterAiConfig::from_monster_type(&mtype);
        let fire = cfg
            .spells
            .iter()
            .find(|s| {
                matches!(
                    s.impact,
                    SpellImpact::Damage {
                        element: CombatType::Fire,
                        ..
                    }
                )
            })
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
        // XML `radius="3"` → 772 `Origin (2, …)` disc radius (`marid.mon`).
        assert_eq!(energy_cond.radius, 2);

        let lifedrain = cfg
            .spells
            .iter()
            .find(|s| {
                matches!(
                    s.impact,
                    SpellImpact::Damage {
                        element: CombatType::LifeDrain,
                        ..
                    }
                )
            })
            .expect("marid lifedrain");
        assert_eq!(lifedrain.shoot_effect, Some(ShootEffect::Death as u8));
    }

    /// Ghoul XML `lifedrain … range="1"` → 772 `Victim (1,0,0)` (`ghoul.mon`), not Actor.
    /// Regression: `range > 1` left melee-range drains as self-casts.
    #[test]
    fn test_e0_ghoul_lifedrain_range_1_is_victim() {
        let mtype = load_monster_type("ghoul");
        let cfg = MonsterAiConfig::from_monster_type(&mtype);
        let drain = cfg
            .spells
            .iter()
            .find(|s| {
                matches!(
                    s.impact,
                    SpellImpact::Damage {
                        element: CombatType::LifeDrain,
                        ..
                    }
                )
            })
            .expect("ghoul lifedrain");
        assert_eq!(drain.range, 1);
        assert_eq!(
            drain.shape,
            SpellShape::Victim,
            "range=1 must be Victim (player), not Actor (self)"
        );
    }

    #[test]
    fn test_e4_drunk_and_speed_parse() {
        let node = spell_node(
            "drunk",
            &[
                ("delay", "5"),
                ("range", "7"),
                ("duration", "60000"),
                ("drunkness", "120"),
            ],
            &[("shooteffect", "energy")],
        );
        let spell = MonsterSpell::try_from_node(&node).expect("drunk spell");
        assert_eq!(spell.shape, SpellShape::Victim);
        assert!(matches!(
            spell.impact,
            SpellImpact::Drunk { drunkness: 120 }
        ));

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
        let m = Monster::with_config(test_creature_base(), Position::new(100, 100, 7), cfg);
        assert!(creature_immune_poison(&CreatureKind::Monster(m)));
    }
}
