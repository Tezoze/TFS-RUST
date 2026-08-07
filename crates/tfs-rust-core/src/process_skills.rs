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
    ///
    /// Skip wild monsters / NPCs with an empty condition list: they have nothing to tick, and
    /// with ~20k+ spawned creatures a full SlotMap sweep every SkillTimeCounter fire was enough
    /// to push beat lag over the 1000 ms `MoveCreatures` skip threshold when many nearby
    /// monsters were also pathfinding.
    pub(crate) fn process_skills(&mut self) {
        self.scratch_creature_ids.clear();
        self.scratch_creature_ids.extend(
            self.creatures
                .iter()
                .filter(|(_, k)| {
                    if k.base().health <= 0 {
                        return false;
                    }
                    matches!(k, CreatureKind::Player(_)) || !k.base().active_conditions.is_empty()
                })
                .map(|(id, _)| id),
        );

        for cid in std::mem::take(&mut self.scratch_creature_ids) {
            self.process_creature_skills(cid);
            // Phase 4: 1098 defer deleted — both eras run fed regen.
            self.process_player_fed_regen(cid);
            self.process_player_soul_regen(cid);
            self.process_equipment_regeneration(cid);
            // CH-5: flood protection message buffer decrement (1500ms interval).
            self.process_player_message_buffer(cid);
        }
    }

    /// TFS `ConditionRegeneration::executeCondition` — life ring / soft boots (`condition.cpp`).
    ///
    /// `ProcessSkills` cadence is ~1000 ms; accumulate until `health_ticks_ms` / `mana_ticks_ms`.
    fn process_equipment_regeneration(&mut self, cid: CreatureId) {
        const INTERVAL_MS: u32 = 1000;
        let Some(CreatureKind::Player(_)) = self.creatures.get(cid) else {
            return;
        };
        // Inside PZ — TFS regen conditions still tick; fed regen is PZ-gated separately.
        let mut hp_delta = 0i32;
        let mut mana_delta = 0i32;
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            for cond in p.base.active_conditions.iter_mut() {
                if cond.ctype != ConditionType::Regeneration {
                    continue;
                }
                let ConditionData::Regeneration {
                    health_gain,
                    health_ticks_ms,
                    mana_gain,
                    mana_ticks_ms,
                    health_elapsed_ms,
                    mana_elapsed_ms,
                } = &mut cond.data
                else {
                    continue;
                };
                if *health_ticks_ms > 0 && *health_gain > 0 {
                    *health_elapsed_ms = health_elapsed_ms.saturating_add(INTERVAL_MS);
                    while *health_elapsed_ms >= *health_ticks_ms {
                        *health_elapsed_ms -= *health_ticks_ms;
                        hp_delta += *health_gain;
                    }
                }
                if *mana_ticks_ms > 0 && *mana_gain > 0 {
                    *mana_elapsed_ms = mana_elapsed_ms.saturating_add(INTERVAL_MS);
                    while *mana_elapsed_ms >= *mana_ticks_ms {
                        *mana_elapsed_ms -= *mana_ticks_ms;
                        mana_delta += *mana_gain;
                    }
                }
            }
            if hp_delta > 0 {
                let max_h = p.effective_max_health();
                p.base.health = (p.base.health + hp_delta).min(max_h);
            }
            if mana_delta > 0 {
                let max_m = p.effective_max_mana();
                p.mana = (p.mana + mana_delta).min(max_m);
            }
        }
        if hp_delta > 0 || mana_delta > 0 {
            self.send_player_stats(cid);
        }
    }

    fn process_creature_skills(&mut self, cid: CreatureId) {
        let mut dot_events: Vec<(Option<CreatureId>, CombatType, i32)> = Vec::new();
        let mut remove_indices: Vec<usize> = Vec::new();
        let mut ended_ctypes: Vec<ConditionType> = Vec::new();

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
                        let interval = match cond.ctype {
                            ConditionType::Fire => profile.conditions.fire.ticks,
                            ConditionType::Energy => profile.conditions.energy.ticks,
                            _ => 1,
                        }
                        .max(1);
                        // 772 `TSkill::Process` (`crskill.cc:186-193`): Count countdown;
                        // Event only when Count <= 0, then Count = MaxCount.
                        let initialized = cond.skill_max_count > 0;
                        if !initialized {
                            // First tick after apply without SetTimer Count — init, no damage.
                            continue;
                        }
                        if cond.skill_count > 0 {
                            continue;
                        }
                        let round = cond
                            .timer_rounds_left
                            .map(|t| interval - t)
                            .unwrap_or(0);
                        let Some((dmg, max_ticks)) =
                            dot_tick_for_condition(profile, hooks, cond.ctype, round)
                        else {
                            continue;
                        };
                        let ticks_left = cond.timer_rounds_left.unwrap_or(max_ticks);
                        if ticks_left <= 0 {
                            remove_indices.push(idx);
                            ended_ctypes.push(cond.ctype);
                            continue;
                        }
                        let combat = if cond.ctype == ConditionType::Fire {
                            CombatType::Fire
                        } else {
                            CombatType::Energy
                        };
                        // 772 Event: `Damage(GetCreature(*DamageOrigin), …, DAMAGE_FIRE|ENERGY)`
                        // (`crskill.cc:1064,1090`) — instant type + stored origin.
                        let origin = if cond.ctype == ConditionType::Fire {
                            base.fire_damage_origin
                        } else {
                            base.energy_damage_origin
                        };
                        dot_events.push((origin, combat, dmg));
                        if ticks_left <= 1 {
                            remove_indices.push(idx);
                            ended_ctypes.push(cond.ctype);
                        }
                    }
                    ConditionType::Poison => {
                        // 772 `TSkillPoison::Process` Count/MaxCount = 3 (`crskill.cc:976-990`).
                        let initialized = cond.skill_max_count > 0;
                        if !initialized {
                            continue;
                        }
                        if cond.skill_count > 0 {
                            continue;
                        }
                        if let ConditionData::Damage { total_rank } = cond.data {
                            if total_rank <= 0 {
                                remove_indices.push(idx);
                                ended_ctypes.push(cond.ctype);
                                continue;
                            }
                            // 772 Event: instant `DAMAGE_POISON` + `PoisonDamageOrigin`.
                            dot_events.push((base.poison_damage_origin, CombatType::Earth, total_rank));
                            let next = (total_rank * POISON_DECAY_PERCENT) / 100;
                            if next <= 0 {
                                remove_indices.push(idx);
                                ended_ctypes.push(cond.ctype);
                            }
                        }
                    }
                    ConditionType::Haste | ConditionType::Paralyze => {
                        if let Some(left) = cond.timer_rounds_left {
                            if left <= 1 {
                                remove_indices.push(idx);
                                ended_ctypes.push(cond.ctype);
                            }
                        }
                    }
                    ConditionType::Light
                    | ConditionType::Invisible
                    | ConditionType::Outfit
                    | ConditionType::Drunk
                    | ConditionType::ManaShield
                    | ConditionType::Infight => {
                        if let Some(left) = cond.timer_rounds_left {
                            if left <= 1 {
                                remove_indices.push(idx);
                                ended_ctypes.push(cond.ctype);
                            }
                        } else if let ConditionData::Generic { ticks: 0 } = cond.data {
                            remove_indices.push(idx);
                            ended_ctypes.push(cond.ctype);
                        }
                    }
                    _ => {}
                }
            }
        }

        for (origin, combat, dmg) in dot_events {
            if dmg <= 0 {
                continue;
            }
            let damage = CombatDamage {
                primary: (combat, -dmg),
                secondary: (CombatType::Physical, 0),
            };
            // 772 `TSkillHitpoints::Set` → `SendPlayerData` (`crskill.cc:682-683`).
            // Snapshot before apply — death may remove the creature.
            let snap = self.combat_notify_snapshot(cid);
            let hp_before = self
                .creatures
                .get(cid)
                .map(|k| k.base().health)
                .unwrap_or(0);
            let _ = self.combat_execute_with_stimulus(origin, cid, &damage, &CombatParams::default());
            let hp_after = self
                .creatures
                .get(cid)
                .map(|k| k.base().health)
                .unwrap_or(0);
            let damage_done = (hp_before - hp_after).max(0);
            if let Some(snap) = snap {
                self.notify_player_combat_damage(origin, cid, damage_done, combat, snap);
            }
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
                        let interval = match cond.ctype {
                            ConditionType::Fire => self.mechanics.profile.conditions.fire.ticks,
                            ConditionType::Energy => self.mechanics.profile.conditions.energy.ticks,
                            _ => 1,
                        }
                        .max(1);
                        if cond.skill_max_count <= 0 {
                            // Mirror `SetTimer(..., Count=MaxCount)` — start countdown, no Event yet.
                            cond.skill_max_count = interval;
                            cond.skill_count = interval;
                            if cond.timer_rounds_left.is_none() {
                                cond.timer_rounds_left = Some(interval);
                            }
                            continue;
                        }
                        if cond.skill_count > 0 {
                            cond.skill_count -= 1;
                        } else {
                            // Event already applied this tick — reset Count + decrement Cycle.
                            cond.skill_count = cond.skill_max_count;
                            if let Some(left) = cond.timer_rounds_left.as_mut() {
                                *left -= 1;
                            }
                        }
                    }
                    ConditionType::Poison => {
                        if cond.skill_max_count <= 0 {
                            cond.skill_max_count = 3;
                            cond.skill_count = 3;
                            continue;
                        }
                        if cond.skill_count > 0 {
                            cond.skill_count -= 1;
                        } else {
                            cond.skill_count = cond.skill_max_count;
                            if let ConditionData::Damage { total_rank } = &mut cond.data {
                                *total_rank = (*total_rank * POISON_DECAY_PERCENT) / 100;
                            }
                        }
                    }
                    ConditionType::Haste | ConditionType::Paralyze => {
                        if let Some(left) = cond.timer_rounds_left.as_mut() {
                            *left -= 1;
                        }
                    }
                    ConditionType::Regeneration => {
                        // Ticked in `process_equipment_regeneration` (needs HP/mana mutation).
                    }
                    // C++ `ConditionGeneric::executeCondition` — `condition.cpp:315-317` →
                    // `Condition::executeCondition` (`condition.cpp:154-163`): `ticks =
                    // max(0, ticks - interval)`. `ProcessSkills` fires every ~1000 ms
                    // (`SkillTimeCounter`, `subsystem_counters.rs`), so we decrement by
                    // 1000 ms per tick. `YellTicks` (30 000 ms = 30 s) expires after ~30
                    // ticks. CH-5 adds `Muted`/`ChannelMutedTicks` ticking the same way.
                    ConditionType::YellTicks
                    | ConditionType::Muted
                    | ConditionType::ChannelMutedTicks
                    | ConditionType::Invisible
                    | ConditionType::Outfit
                    | ConditionType::Drunk
                    | ConditionType::ManaShield
                    | ConditionType::Light
                    | ConditionType::Infight => {
                        if let Some(left) = cond.timer_rounds_left.as_mut() {
                            *left -= 1;
                        } else if let ConditionData::Generic { ticks } = &mut cond.data {
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
                    (
                        ConditionType::YellTicks
                            | ConditionType::Muted
                            | ConditionType::ChannelMutedTicks,
                        ConditionData::Generic { ticks: 0 }
                    )
                )
            });
        }
        // PC-3a Phase 4b — client notifies on expiry (icons / speed / light / invis).
        // C++ `Player::onEndCondition` + `Condition*::endCondition`; speed via
        // `crskill.cc:366,741,761` `CREATURE_SPEED_CHANGED`.
        ended_ctypes.sort_unstable_by_key(|c| *c as u8);
        ended_ctypes.dedup();
        for ctype in ended_ctypes {
            self.on_condition_ended(cid, ctype);
        }
    }

    /// 772 `TSkillSoulpoints` Process/Event — +1 soul per Interval (`crskill.cc:796-807`,
    /// `crcombat.cc:938-955` arm).
    fn process_player_soul_regen(&mut self, cid: CreatureId) {
        let gained = {
            let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) else {
                return;
            };
            if p.soul_max_count <= 0 || p.soul_cycle <= 0 {
                return;
            }
            if p.soul_count > 0 {
                p.soul_count -= 1;
                return;
            }
            // Event — Change(1), then Count = MaxCount, Cycle -= 1.
            let soul_max = p.vocation_profile.soul_max.max(0);
            p.economy.soul = (p.economy.soul + 1).min(soul_max);
            p.soul_count = p.soul_max_count;
            p.soul_cycle -= 1;
            if p.soul_cycle <= 0 {
                p.soul_cycle = 0;
                p.soul_count = 0;
                p.soul_max_count = 0;
            }
            true
        };
        if gained {
            self.send_player_stats(cid);
        }
    }

    /// Keep `base.speed` aligned with vocation GoStrength after haste/paralyze changes.
    /// Effective walk/wire speed is `base_speed + var_speed + ConditionData::Speed`
    /// ([`crate::walk::walk_timing`] `creature_effective_speed_for_step`) — do not bake
    /// condition deltas into `base.speed` or they double-count.
    pub(crate) fn recompute_speed_from_conditions(base: &mut crate::creature::CreatureBase) {
        base.speed = base.base_speed;
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
                if !has_cannot_be_muted_flag && max_buffer != 0 && p.message_buffer_count > 0 {
                    p.message_buffer_count -= 1;
                }
            }
        }
    }
    fn process_player_fed_regen(&mut self, cid: CreatureId) {
        let (food_remaining, voc_id, pos) = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => (p.food_remaining, p.vocation_id, p.base.position),
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
                // Count=0 MaxCount=1 → Event every ProcessSkills (fast unit test).
                timer_rounds_left: Some(2),
                skill_count: 0,
                skill_max_count: 1,
            },
        );

        world.process_skills();
        let hp1 = world.creatures.get(player).unwrap().base().health;
        assert!(hp1 < 150, "fire tick should deal damage");

        world.process_skills(); // Count 1 → 0
        world.process_skills(); // second Event
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

    /// Fire DoT Events only every `MaxCount` ProcessSkills rounds (772 `crskill.cc:186-193`).
    #[test]
    fn fire_condition_respects_skill_max_count_interval() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let player = insert_player(&mut world, test_player("Smolder", pos));

        apply_condition(
            &mut world.creatures,
            player,
            ActiveCondition {
                id: 1,
                sub_id: 0,
                ctype: ConditionType::Fire,
                data: ConditionData::Damage { total_rank: 10 },
                timer_rounds_left: Some(3),
                skill_count: 2,
                skill_max_count: 2,
            },
        );

        let hp0 = world.creatures.get(player).unwrap().base().health;
        world.process_skills(); // count 2→1
        assert_eq!(
            world.creatures.get(player).unwrap().base().health,
            hp0,
            "no damage while Count > 0"
        );
        world.process_skills(); // count 1→0
        assert_eq!(
            world.creatures.get(player).unwrap().base().health,
            hp0,
            "still no damage on final countdown"
        );
        world.process_skills(); // Count<=0 → Event
        let hp1 = world.creatures.get(player).unwrap().base().health;
        assert!(hp1 < hp0, "damage on Event when Count hits 0");
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
            skill_count: 0,
            skill_max_count: 1, // Event every ProcessSkills for this unit test
        }];
        add_condition_merge(
            &mut conds,
            ActiveCondition {
                id: 1,
                sub_id: 0,
                ctype: ConditionType::Poison,
                data: ConditionData::Damage { total_rank: 20 },
                timer_rounds_left: None,
                skill_count: 0,
                skill_max_count: 1,
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
        assert_eq!(rank, 10, "poison strength should decay 50% per Event");
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
        assert_eq!(
            p.base().health,
            92,
            "knight should gain 2 HP over 12 fed ticks"
        );
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

    /// G1 — soul regen armed when exp share ≥ attacker level (`crcombat.cc:938-955`).
    #[test]
    fn soul_regen_armed_when_exp_at_least_level() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let mut player = test_player("Soulful", pos);
        player.level = 8;
        player.vocation_profile.gain_soul_ticks = 120;
        player.economy.soul = 0;
        player.vocation_profile.soul_max = 100;
        assert_eq!(player.soul_cycle, 0);
        player.arm_soul_regen_timer();
        assert_eq!(player.soul_max_count, 120);
        assert_eq!(player.soul_cycle, 240 / 120);
        assert_eq!(player.soul_count, 120);

        // Promoted interval.
        player.vocation_profile.gain_soul_ticks = 15;
        player.soul_cycle = 0;
        player.soul_count = 0;
        player.soul_max_count = 0;
        player.arm_soul_regen_timer();
        assert_eq!(player.soul_max_count, 15);
        assert_eq!(player.soul_cycle, 240 / 15);
    }

    /// B7 — weaker poison stimulates but does not re-arm (`crmain.cc:586-590`).
    #[test]
    fn weaker_poison_does_not_override_stronger() {
        use tfs_rust_common::enums::CombatType;
        use crate::combat::CombatDamage;

        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let victim = insert_player(&mut world, test_player("Poisoned", pos));
        let attacker = insert_player(
            &mut world,
            test_player("Spider", Position::new(101, 100, 7)),
        );
        ensure_walkable_tile(&mut world.map, Position::new(101, 100, 7), 150);

        let _ = world.combat_execute_with_stimulus(
            Some(attacker),
            victim,
            &CombatDamage {
                primary: (CombatType::PoisonPeriodic, -50),
                secondary: (CombatType::Undefined, 0),
            },
            &CombatParams::default(),
        );
        let rank_before = world
            .creatures
            .get(victim)
            .unwrap()
            .base()
            .active_conditions
            .iter()
            .find_map(|c| match (&c.ctype, &c.data) {
                (ConditionType::Poison, ConditionData::Damage { total_rank }) => Some(*total_rank),
                _ => None,
            })
            .expect("strong poison armed");
        assert_eq!(rank_before, 50);

        let _ = world.combat_execute_with_stimulus(
            Some(attacker),
            victim,
            &CombatDamage {
                primary: (CombatType::PoisonPeriodic, -10),
                secondary: (CombatType::Undefined, 0),
            },
            &CombatParams::default(),
        );
        let rank_after = world
            .creatures
            .get(victim)
            .unwrap()
            .base()
            .active_conditions
            .iter()
            .find_map(|c| match (&c.ctype, &c.data) {
                (ConditionType::Poison, ConditionData::Damage { total_rank }) => Some(*total_rank),
                _ => None,
            })
            .expect("poison still present");
        assert_eq!(rank_after, 50, "weaker poison must not override");
    }

    /// B7 — fire periodic always re-arms Cycle (`crmain.cc:596-603`).
    #[test]
    fn fire_periodic_rearms_unconditionally() {
        use tfs_rust_common::enums::CombatType;
        use crate::combat::CombatDamage;

        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let victim = insert_player(&mut world, test_player("Burning", pos));

        let _ = world.combat_execute_with_stimulus(
            None,
            victim,
            &CombatDamage {
                primary: (CombatType::FirePeriodic, -20), // Cycle = 2
                secondary: (CombatType::Undefined, 0),
            },
            &CombatParams::default(),
        );
        let cycle1 = world
            .creatures
            .get(victim)
            .unwrap()
            .base()
            .active_conditions
            .iter()
            .find(|c| c.ctype == ConditionType::Fire)
            .and_then(|c| c.timer_rounds_left)
            .expect("fire armed");
        assert_eq!(cycle1, 2);

        let _ = world.combat_execute_with_stimulus(
            None,
            victim,
            &CombatDamage {
                primary: (CombatType::FirePeriodic, -80), // Cycle = 8
                secondary: (CombatType::Undefined, 0),
            },
            &CombatParams::default(),
        );
        let cycle2 = world
            .creatures
            .get(victim)
            .unwrap()
            .base()
            .active_conditions
            .iter()
            .find(|c| c.ctype == ConditionType::Fire)
            .and_then(|c| c.timer_rounds_left)
            .expect("fire re-armed");
        assert_eq!(cycle2, 8, "fire must re-arm unconditionally");
    }

    /// B7 — PvP halving excludes `*_PERIODIC` arming hits (`crmain.cc:497-502`).
    #[test]
    fn pvp_periodic_damage_not_halved() {
        use tfs_rust_common::enums::CombatType;
        use crate::combat::CombatDamage;

        let mut world = beat_driven_test_world();
        let a_pos = Position::new(100, 100, 7);
        let b_pos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, a_pos, 150);
        ensure_walkable_tile(&mut world.map, b_pos, 150);
        let attacker = insert_player(&mut world, test_player("A", a_pos));
        let victim = insert_player(&mut world, test_player("B", b_pos));

        let _ = world.combat_execute_with_stimulus(
            Some(attacker),
            victim,
            &CombatDamage {
                primary: (CombatType::PoisonPeriodic, -40),
                secondary: (CombatType::Undefined, 0),
            },
            &CombatParams::default(),
        );
        let rank = world
            .creatures
            .get(victim)
            .unwrap()
            .base()
            .active_conditions
            .iter()
            .find_map(|c| match (&c.ctype, &c.data) {
                (ConditionType::Poison, ConditionData::Damage { total_rank }) => Some(*total_rank),
                _ => None,
            })
            .expect("poison armed at full strength");
        assert_eq!(rank, 40, "periodic arming must not be PvP-halved");
    }

    /// B7 — DoT Event ticks credit stored origin on the damage map (`crskill.cc` Events).
    #[test]
    fn dot_kill_credits_origin() {
        use tfs_rust_common::enums::CombatType;
        use crate::combat::CombatDamage;

        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let mut victim_p = test_player("Victim", pos);
        victim_p.base.health = 5;
        victim_p.base.max_health = 5;
        let victim = insert_player(&mut world, victim_p);
        let attacker = insert_player(
            &mut world,
            test_player("Origin", Position::new(101, 100, 7)),
        );
        ensure_walkable_tile(&mut world.map, Position::new(101, 100, 7), 150);

        let _ = world.combat_execute_with_stimulus(
            Some(attacker),
            victim,
            &CombatDamage {
                primary: (CombatType::PoisonPeriodic, -30),
                secondary: (CombatType::Undefined, 0),
            },
            &CombatParams::default(),
        );
        assert_eq!(
            world
                .creatures
                .get(victim)
                .unwrap()
                .base()
                .poison_damage_origin,
            Some(attacker)
        );

        // Force an immediate poison Event (Count already 0).
        if let Some(kind) = world.creatures.get_mut(victim) {
            if let Some(cond) = kind
                .base_mut()
                .active_conditions
                .iter_mut()
                .find(|c| c.ctype == ConditionType::Poison)
            {
                cond.skill_count = 0;
                cond.skill_max_count = 3;
            }
        }
        world.process_skills();
        let map_dmg = world
            .creatures
            .get(victim)
            .map(|k| k.base().damage_map.damage_by(attacker))
            .unwrap_or(0);
        if world.creatures.contains_key(victim) {
            assert!(
                map_dmg > 0,
                "poison Event must credit poison_damage_origin"
            );
        }
    }
}
