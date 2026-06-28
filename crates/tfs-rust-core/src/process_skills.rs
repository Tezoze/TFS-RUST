//! 772 `ProcessSkills` — creature timer-skill tick on the `SkillTimeCounter` subsystem.
//!
//! C++ reference: `crmain.cc:1130` `ProcessSkills`, `crskill.cc` `TSkill*::Event`.

use tfs_rust_common::enums::{CombatType, ConditionType};

use crate::combat::{CombatDamage, CombatParams};
use crate::condition::{dot_tick_for_condition, ConditionData};
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// Poison strength decay per round — `TSkillPoison::FactorPercent` default `0x32` (`crskill.cc:1052`).
const POISON_DECAY_PERCENT: i32 = 50;

impl GameWorld {
    /// C++ `ProcessSkills` — tick timer-skills for every creature (`crmain.cc:1130-1139`).
    pub(crate) fn process_skills_772(&mut self) {
        let ids: Vec<CreatureId> = self
            .creatures
            .iter()
            .filter(|(_, k)| k.base().health > 0)
            .map(|(id, _)| id)
            .collect();

        for cid in ids {
            self.process_creature_skills_772(cid);
            if self.beat_driven_loop {
                self.process_player_fed_regen_772(cid);
            }
        }
    }

    fn process_creature_skills_772(&mut self, cid: CreatureId) {
        let mut dot_events: Vec<(Option<CreatureId>, CombatType, i32)> = Vec::new();
        let mut remove_indices: Vec<usize> = Vec::new();
        let mut speed_expired = false;

        {
            let Some(kind) = self.creatures.get(cid) else {
                return;
            };
            let base = kind.base();
            let profile = &self.mechanics.profile;
            let hooks = &self.mechanics.hooks;

            for (idx, cond) in base.active_conditions.iter().enumerate() {
                match cond.ctype {
                    ConditionType::Fire | ConditionType::Energy => {
                        let round = cond
                            .timer_rounds_left
                            .map(|t| profile.conditions.fire.ticks - t)
                            .unwrap_or(0);
                        let Some((dmg, max_ticks)) =
                            dot_tick_for_condition(profile, hooks, cond.ctype, round)
                        else {
                            continue;
                        };
                        let ticks_left = cond.timer_rounds_left.unwrap_or(max_ticks);
                        if ticks_left <= 0 {
                            remove_indices.push(idx);
                            continue;
                        }
                        let combat = if cond.ctype == ConditionType::Fire {
                            CombatType::Fire
                        } else {
                            CombatType::Energy
                        };
                        dot_events.push((None, combat, dmg));
                        if ticks_left <= 1 {
                            remove_indices.push(idx);
                        }
                    }
                    ConditionType::Poison => {
                        if let ConditionData::Damage { total_rank } = cond.data {
                            if total_rank <= 0 {
                                remove_indices.push(idx);
                                continue;
                            }
                            dot_events.push((None, CombatType::Earth, total_rank));
                            let next = (total_rank * POISON_DECAY_PERCENT) / 100;
                            if next <= 0 {
                                remove_indices.push(idx);
                            }
                        }
                    }
                    ConditionType::Haste | ConditionType::Paralyze => {
                        if let Some(left) = cond.timer_rounds_left {
                            if left <= 1 {
                                remove_indices.push(idx);
                                speed_expired = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        for (_, combat, dmg) in dot_events {
            if dmg <= 0 {
                continue;
            }
            let damage = CombatDamage {
                primary: (combat, -dmg),
                secondary: (CombatType::Physical, 0),
            };
            let _ = self.combat_execute_with_stimulus(None, cid, &damage, &CombatParams::default());
        }

        if let Some(kind) = self.creatures.get_mut(cid) {
            let base = kind.base_mut();
            remove_indices.sort_unstable();
            remove_indices.dedup();
            for idx in remove_indices.into_iter().rev() {
                if idx < base.active_conditions.len() {
                    base.active_conditions.remove(idx);
                }
            }
            // Decrement timers and apply poison decay after damage pass.
            for cond in base.active_conditions.iter_mut() {
                match cond.ctype {
                    ConditionType::Fire | ConditionType::Energy => {
                        if let Some(left) = cond.timer_rounds_left.as_mut() {
                            *left -= 1;
                        } else {
                            let max_ticks = match cond.ctype {
                                ConditionType::Fire => self.mechanics.profile.conditions.fire.ticks,
                                ConditionType::Energy => {
                                    self.mechanics.profile.conditions.energy.ticks
                                }
                                _ => 0,
                            };
                            cond.timer_rounds_left = Some(max_ticks - 1);
                        }
                    }
                    ConditionType::Poison => {
                        if let ConditionData::Damage { total_rank } = &mut cond.data {
                            *total_rank = (*total_rank * POISON_DECAY_PERCENT) / 100;
                        }
                    }
                    ConditionType::Haste | ConditionType::Paralyze => {
                        if let Some(left) = cond.timer_rounds_left.as_mut() {
                            *left -= 1;
                        }
                    }
                    _ => {}
                }
            }
            if speed_expired {
                Self::recompute_speed_from_conditions(base);
            }
        }
    }

    fn recompute_speed_from_conditions(base: &mut crate::creature::CreatureBase) {
        let base_speed = base.base_speed;
        let mut delta = 0i32;
        for cond in &base.active_conditions {
            if let ConditionData::Speed { flat_delta } = cond.data {
                delta += flat_delta;
            }
        }
        base.speed = base_speed + delta;
    }

    /// C++ `TSkillFed::Event` — vocation HP/mana regen (`crskill.cc:851-885`).
    fn process_player_fed_regen_772(&mut self, cid: CreatureId) {
        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) else {
            return;
        };
        p.skills_fed_timer = p.skills_fed_timer.saturating_add(1);
        let timer = p.skills_fed_timer;
        let voc_id = p.vocation_id.max(0) as u32;
        drop(p);

        let (hp_ticks, mana_ticks, hp_amount, mana_amount) = fed_regen_cadence(voc_id);

        let mut hp_gain = 0i32;
        let mut mana_gain = 0i32;
        if hp_ticks > 0 && timer.is_multiple_of(hp_ticks) {
            hp_gain = hp_amount;
        }
        if mana_ticks > 0 && timer.is_multiple_of(mana_ticks) {
            mana_gain = mana_amount;
        }
        if hp_gain == 0 && mana_gain == 0 {
            return;
        }

        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            if hp_gain > 0 {
                p.base.health = (p.base.health + hp_gain).min(p.base.max_health);
            }
            if mana_gain > 0 {
                p.mana = (p.mana + mana_gain).min(p.max_mana);
            }
        }
    }
}

/// Map vocation to regen cadence — 772 `TSkillFed::Event` profession table (`crskill.cc:851-874`).
fn fed_regen_cadence(vocation_id: u32) -> (u32, u32, i32, i32) {
    match vocation_id {
        // Knight / Elite Knight
        4 => (6, 6, 1, 2),
        8 => (4, 6, 1, 2),
        // Paladin / Royal Paladin
        3 => (6, 3, 1, 2),
        7 => (6, 3, 1, 2),
        // Sorcerer / Druid / promoted
        1 | 2 | 5 | 6 => (12, 2, 1, 2),
        _ => (12, 6, 1, 2),
    }
}

#[cfg(test)]
mod tests {
    use tfs_rust_common::enums::ConditionType;
    use tfs_rust_common::Position;

    use crate::combat::{apply_condition, CombatParams};
    use crate::condition::{add_condition_merge, ActiveCondition, ConditionData};
    use crate::creature::CreatureKind;
    use crate::test_world::support::{
        beat_driven_test_world, ensure_walkable_tile, insert_player, test_player,
    };

    #[test]
    fn fire_condition_ticks_damage_and_expires() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let player = insert_player(&mut world, test_player("Pyro", pos));

        apply_condition(
            &mut world.creatures,
            player,
            ActiveCondition {
                id: 1,
                sub_id: 0,
                ctype: ConditionType::Fire,
                data: ConditionData::Damage { total_rank: 10 },
                timer_rounds_left: Some(2),
            },
        );

        world.process_skills_772();
        let hp1 = world.creatures.get(player).unwrap().base().health;
        assert!(hp1 < 150, "fire tick should deal damage");

        world.process_skills_772();
        let hp2 = world.creatures.get(player).unwrap().base().health;
        assert!(hp2 < hp1, "second fire tick should deal more damage");

        world.process_skills_772();
        let has_fire = world.creatures.get(player).is_some_and(|k| {
            k.base()
                .active_conditions
                .iter()
                .any(|c| c.ctype == ConditionType::Fire)
        });
        assert!(!has_fire, "fire condition should expire after ticks");
    }

    #[test]
    fn poison_decays_strength_each_round() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let player = insert_player(&mut world, test_player("Snake", pos));

        let mut conds = vec![ActiveCondition {
            id: 1,
            sub_id: 0,
            ctype: ConditionType::Poison,
            data: ConditionData::Damage { total_rank: 20 },
            timer_rounds_left: None,
        }];
        add_condition_merge(
            &mut conds,
            ActiveCondition {
                id: 1,
                sub_id: 0,
                ctype: ConditionType::Poison,
                data: ConditionData::Damage { total_rank: 20 },
                timer_rounds_left: None,
            },
        );
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.active_conditions = conds;
        }

        world.process_skills_772();
        let rank = world
            .creatures
            .get(player)
            .and_then(|k| {
                k.base().active_conditions.iter().find_map(|c| {
                    if c.ctype == ConditionType::Poison {
                        if let ConditionData::Damage { total_rank } = c.data {
                            Some(total_rank)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
            })
            .unwrap_or(0);
        assert_eq!(rank, 10, "poison strength should decay 50% per round");
    }
}
