//! 772 `ProcessSkills` — creature timer-skill tick on the `SkillTimeCounter` subsystem.
//!
//! C++ reference: `crmain.cc:1130` `ProcessSkills`, `crskill.cc` `TSkill*::Event`.

use tfs_rust_common::enums::{CombatType, ConditionType};

use crate::combat::{CombatDamage, CombatParams};
use crate::condition::{dot_tick_for_condition, ConditionData};
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::player::flags::PLAYER_FLAG_CANNOT_BE_MUTED;

/// Poison strength decay per round — `TSkillPoison::FactorPercent` default `0x32` (`crskill.cc:1052`).
const POISON_DECAY_PERCENT: i32 = 50;

impl GameWorld {
    /// C++ `ProcessSkills` — tick timer-skills for every creature (`crmain.cc:1130-1139`).
    pub(crate) fn process_skills(&mut self) {
        let ids: Vec<CreatureId> = self
            .creatures
            .iter()
            .filter(|(_, k)| k.base().health > 0)
            .map(|(id, _)| id)
            .collect();

        for cid in ids {
            self.process_creature_skills(cid);
            // Phase 4: 1098 defer deleted — both eras run fed regen.
            self.process_player_fed_regen(cid);
            // CH-5: flood protection message buffer decrement (1500ms interval).
            self.process_player_message_buffer(cid);
        }
    }

    fn process_creature_skills(&mut self, cid: CreatureId) {
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
                    // C++ `ConditionGeneric::executeCondition` — `condition.cpp:315-317` →
                    // `Condition::executeCondition` (`condition.cpp:154-163`): `ticks =
                    // max(0, ticks - interval)`. `ProcessSkills` fires every ~1000 ms
                    // (`SkillTimeCounter`, `subsystem_counters.rs`), so we decrement by
                    // 1000 ms per tick. `YellTicks` (30 000 ms = 30 s) expires after ~30
                    // ticks. CH-5 adds `Muted`/`ChannelMutedTicks` ticking the same way.
                    ConditionType::YellTicks | ConditionType::Muted | ConditionType::ChannelMutedTicks => {
                        if let ConditionData::Generic { ticks } = &mut cond.data {
                            *ticks = (*ticks).saturating_sub(1000);
                        }
                    }
                    _ => {}
                }
            }
            // Remove expired `ConditionGeneric` (YellTicks, Muted, ChannelMutedTicks) conditions after tick-down.
            base.active_conditions.retain(|c| {
                !matches!(
                    (c.ctype, &c.data),
                    (ConditionType::YellTicks | ConditionType::Muted | ConditionType::ChannelMutedTicks, ConditionData::Generic { ticks: 0 })
                )
            });
            if speed_expired {
                Self::recompute_speed_from_conditions(base);
            }
        }
        // C++ `crskill.cc:366,741,761` `CREATURE_SPEED_CHANGED` — announce when a speed
        // condition expires and `base.speed` is recomputed.
        if speed_expired {
            self.announce_creature_speed(cid);
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

    /// C++ `TSkillFed::Event` — vocation HP/mana regen (`crskill.cc:812-885`).
    ///
    /// Gates (matching the reference):
    /// - **Protection zone**: return early inside a PZ (`crskill.cc:819`).
    /// - **Food remaining**: `SKILL_FED` `Cycle == 0` ⇒ skill inactive ⇒ no regen
    ///   (`crskill.cc:180`, `crskill.cc:877`).
    ///
    /// Cadence comes from `vocations.xml` (`gainhpticks`/`gainhpamount`/
    /// `gainmanaticks`/`gainmanaamount`) via `VocationRegistry::fed_regen_params`, not a
    /// hardcoded table. `TSkill::Process` decrements `Cycle` *before* `Event` runs
    /// (`crskill.cc:186-191`), so the modulo is taken on the post-decrement value —
    /// regen fires when the remaining-food counter hits a multiple of the vocation's
    /// tick interval (counting down to 0).

    /// C++ `Player::onThink` message buffer tick — `player.cpp:1314-1318`.
    ///
    /// Accumulates the `ProcessSkills` interval (1000ms) and calls `addMessageBuffer`
    /// every 1500ms to decrement the flood protection buffer count.
    fn process_player_message_buffer(&mut self, cid: CreatureId) {
        let has_cannot_be_muted_flag = self.player_has_flag(cid, PLAYER_FLAG_CANNOT_BE_MUTED);

        // C++ `MessageBufferTicks += interval;` — `player.cpp:1314`.
        // `ProcessSkills` fires every ~1000ms, so we add 1000ms per tick.
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.message_buffer_ticks += 1000;

            // C++ `if (MessageBufferTicks >= 1500) { MessageBufferTicks = 0; addMessageBuffer(); }`
            // — `player.cpp:1315-1318`.
            if p.message_buffer_ticks >= 1500 {
                p.message_buffer_ticks = 0;
                let max_buffer = self.chat_config.max_message_buffer as i32;

                // C++ `if (MessageBufferCount > 0 && g_config.getNumber(ConfigManager::MAX_MESSAGEBUFFER) != 0 && !hasFlag(PlayerFlag_CannotBeMuted))`
                // — `player.cpp:1352`.
                if !has_cannot_be_muted_flag && max_buffer != 0 {
                    if p.message_buffer_count > 0 {
                        p.message_buffer_count -= 1;
                    }
                }
            }
        }
    }
    fn process_player_fed_regen(&mut self, cid: CreatureId) {
        let (food_remaining, voc_id, pos) = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => {
                (p.food_remaining, p.vocation_id, p.base.position)
            }
            _ => return,
        };

        // PZ gate — `crskill.cc:819`.
        if self.tile_in_protection_zone(pos) {
            return;
        }
        // Food-remaining gate — `SKILL_FED` inactive (`crskill.cc:180`).
        if food_remaining == 0 {
            return;
        }

        // `TSkill::Process` decrements `Cycle` then calls `Event` (`crskill.cc:186-191`).
        let timer = food_remaining - 1;

        let (hp_ticks, hp_amount, mana_ticks, mana_amount) =
            self.vocations.fed_regen_params(voc_id);

        let hp_gain = if hp_ticks > 0 && timer % hp_ticks == 0 {
            hp_amount
        } else {
            0
        };
        let mana_gain = if mana_ticks > 0 && timer % mana_ticks == 0 {
            mana_amount
        } else {
            0
        };

        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.food_remaining = timer;
            if hp_gain > 0 {
                p.base.health = (p.base.health + hp_gain).min(p.base.max_health);
            }
            if mana_gain > 0 {
                p.mana = (p.mana + mana_gain).min(p.max_mana);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use tfs_rust_common::enums::ConditionType;
    use tfs_rust_common::{Position, ZoneType};

    use tfs_rust_content::vocations::{VocationDef, VocationRegistry};

    use crate::combat::{apply_condition, CombatParams};
    use crate::condition::{add_condition_merge, ActiveCondition, ConditionData};
    use crate::creature::CreatureKind;
    use crate::map::Map;
    use crate::test_world::support::{
        beat_driven_test_world, ensure_walkable_tile, insert_player, test_player,
    };
    use crate::tile::{Tile, TileBody};

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

        world.process_skills();
        let hp1 = world.creatures.get(player).unwrap().base().health;
        assert!(hp1 < 150, "fire tick should deal damage");

        world.process_skills();
        let hp2 = world.creatures.get(player).unwrap().base().health;
        assert!(hp2 < hp1, "second fire tick should deal more damage");

        world.process_skills();
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

        world.process_skills();
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

    /// Build a `VocationRegistry` with a single knight vocation (id=4) matching
    /// `data/XML/vocations.xml`: `gainhpticks=6 gainhpamount=1 gainmanaticks=6 gainmanaamount=2`.
    fn knight_vocation_db() -> Arc<VocationRegistry> {
        let mut vocations = HashMap::new();
        vocations.insert(
            4u16,
            VocationDef {
                id: 4,
                client_id: 1,
                name: "Knight".into(),
                description: "a knight".into(),
                from_vocation: 4,
                gain_cap: 25,
                gain_hp: 15,
                gain_mana: 5,
                gain_hp_ticks: 6,
                gain_hp_amount: 1,
                gain_mana_ticks: 6,
                gain_mana_amount: 2,
                mana_multiplier: 3.0,
                attack_speed_ms: 2000,
                base_speed: 70,
                soul_max: 100,
                gain_soul_ticks: 120,
                allow_pvp: false,
                base_hp: 150,
                base_mana: 0,
                base_cap: 400,
                formula: tfs_rust_content::vocations::VocationFormula::default(),
                skill_multipliers: [1.1, 1.1, 1.1, 1.1, 1.4, 1.1, 1.1],
            },
        );
        Arc::new(VocationRegistry { vocations })
    }

    /// Insert a protection-zone ground tile at `pos` (mirrors `ensure_walkable_tile`
    /// but with `ZoneType::Protection` — `crskill.cc:819` PZ gate).
    fn ensure_pz_tile(map: &mut Map, pos: Position, ground_type: u16) {
        map.insert_tile(
            pos,
            Tile::Normal(TileBody {
                ground: Some(ground_type),
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: 0,
                zone: ZoneType::Protection,
            }),
        );
    }

    /// F3: `TSkillFed::Event` regen reads `gainhpticks`/`gainmanaticks`/amounts from
    /// `vocations.lua` (via `VocationRegistry`), keys the modulo off the decrementing
    /// food counter, and regenerates HP/mana while food remains (`crskill.cc:812-885`).
    #[test]
    fn fed_regen_uses_vocation_xml_params_and_food_counter() {
        let mut world = beat_driven_test_world();
        world.vocations = knight_vocation_db();

        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);

        let mut player = test_player("Knight", pos);
        player.vocation_id = 4;
        player.base.health = 90;
        player.base.max_health = 100;
        player.mana = 40;
        player.max_mana = 50;
        player.food_remaining = 12;
        let pid = insert_player(&mut world, player);

        // 12 ticks: regen fires at food_remaining=6 and =0 (knight 6/6 cadence),
        // giving +2 HP / +4 mana, and the food counter drains to 0.
        for _ in 0..12 {
            world.process_skills();
        }
        let p = world.creatures.get(pid).unwrap();
        assert_eq!(p.base().health, 92, "knight should gain 2 HP over 12 fed ticks");
        let CreatureKind::Player(p) = world.creatures.get(pid).unwrap() else {
            panic!("not a player");
        };
        assert_eq!(p.mana, 44, "knight should gain 4 mana over 12 fed ticks");
        assert_eq!(p.food_remaining, 0, "food counter should drain to 0");

        // Food exhausted ⇒ `SKILL_FED` inactive ⇒ no further regen (`crskill.cc:180`).
        for _ in 0..6 {
            world.process_skills();
        }
        let p = world.creatures.get(pid).unwrap();
        assert_eq!(p.base().health, 92, "no regen after food runs out");
    }

    /// F3: `TSkillFed::Event` returns early inside a protection zone (`crskill.cc:819`).
    #[test]
    fn fed_regen_skipped_in_protection_zone() {
        let mut world = beat_driven_test_world();
        world.vocations = knight_vocation_db();

        let pos = Position::new(100, 100, 7);
        ensure_pz_tile(&mut world.map, pos, 150);

        let mut player = test_player("PzKnight", pos);
        player.vocation_id = 4;
        player.base.health = 90;
        player.base.max_health = 100;
        player.mana = 40;
        player.max_mana = 50;
        player.food_remaining = 12;
        let pid = insert_player(&mut world, player);

        for _ in 0..12 {
            world.process_skills();
        }
        let p = world.creatures.get(pid).unwrap();
        assert_eq!(p.base().health, 90, "no HP regen inside a protection zone");
        let CreatureKind::Player(p) = world.creatures.get(pid).unwrap() else {
            panic!("not a player");
        };
        assert_eq!(p.mana, 40, "no mana regen inside a protection zone");
        assert_eq!(
            p.food_remaining, 12,
            "PZ gate returns before decrementing food"
        );
    }

    /// F3: with no food remaining, `SKILL_FED` is inactive and no regen occurs
    /// (`crskill.cc:180`, `crskill.cc:877`).
    #[test]
    fn fed_regen_skipped_when_food_exhausted() {
        let mut world = beat_driven_test_world();
        world.vocations = knight_vocation_db();

        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);

        let mut player = test_player("Hungry", pos);
        player.vocation_id = 4;
        player.base.health = 90;
        player.base.max_health = 100;
        player.mana = 40;
        player.max_mana = 50;
        player.food_remaining = 0;
        let pid = insert_player(&mut world, player);

        for _ in 0..12 {
            world.process_skills();
        }
        let p = world.creatures.get(pid).unwrap();
        assert_eq!(p.base().health, 90, "no regen with food_remaining = 0");
        let CreatureKind::Player(p) = world.creatures.get(pid).unwrap() else {
            panic!("not a player");
        };
        assert_eq!(p.mana, 40, "no mana regen with food_remaining = 0");
    }
}
