//! `data/lib/core/creature.lua` natives — closest free tile, PZ access, summons, outfits.
//!
//! Pack surface: `Creature.getClosestFreePosition`, `canAccessPz`, `addSummon`, …
//! C++ reference: `creature.cpp` `getPathTo`; `creature.h` `setDropLoot` / `setSkillLoss`;
//! `luascript.cpp` `luaCreatureGetPathTo`, `luaCreatureAddSummon`.

use tfs_rust_common::Position;
use tfs_rust_common::enums::{ConditionType, Direction};

use crate::combat::apply_condition;
use crate::condition::{ActiveCondition, ConditionData};
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::game_world_chat::condition_type_from_lua;
use crate::ids::CreatureId;
use crate::player_flags::{PLAYER_FLAG_CAN_ILLUSION_ALL, flags_for_group, has_player_flag};
use crate::spawn_placement::spiral_free_field_positions;

/// Damage list kind — `data/lib/core/constants.lua` `DAMAGELIST_*`.
pub const DAMAGELIST_EXPONENTIAL_DAMAGE: i32 = 0;
pub const DAMAGELIST_LOGARITHMIC_DAMAGE: i32 = 1;
pub const DAMAGELIST_VARYING_PERIOD: i32 = 2;
pub const DAMAGELIST_CONSTANT_PERIOD: i32 = 3;

const ACCOUNT_TYPE_GOD: u8 = 6;

impl GameWorld {
    /// `Creature.getClosestFreePosition` — spiral probe via [`search_free_field`] tile rules.
    ///
    /// C++ reference: pack `creature.lua` spiral; tile probe aligns with 772
    /// `SearchFreeField` (`info.cc:761`, `spawn_placement.rs`).
    pub fn get_closest_free_position(
        &self,
        from: CreatureId,
        center: Position,
        max_radius: i32,
        must_be_reachable: bool,
    ) -> Position {
        let max_radius = max_radius.max(0);
        for pos in spiral_free_field_positions(center, max_radius) {
            if !self.is_free_field_tile(pos) {
                continue;
            }
            if must_be_reachable
                && self
                    .get_creature_path_to(from, pos, 0, 1, usize::MAX)
                    .is_none()
            {
                continue;
            }
            return pos;
        }
        Position::new(0, 0, 0)
    }

    /// God bypass for `Player.getClosestFreePosition` — `data/lib/core/player.lua`.
    pub fn player_get_closest_free_position(
        &self,
        cid: CreatureId,
        center: Position,
        max_radius: i32,
        must_be_reachable: bool,
    ) -> Position {
        if let Some(CreatureKind::Player(p)) = self.creatures.get(cid) {
            let group_access = self
                .groups
                .groups
                .get(&p.group_id)
                .is_some_and(|g| g.access);
            if group_access && p.account_type >= ACCOUNT_TYPE_GOD {
                return center;
            }
        }
        self.get_closest_free_position(cid, center, max_radius, must_be_reachable)
    }

    /// `Creature:canAccessPz` — `creature.lua`.
    pub fn creature_can_access_pz(&self, cid: CreatureId) -> bool {
        match self.creatures.get(cid) {
            Some(CreatureKind::Monster(_)) => false,
            Some(CreatureKind::Player(p)) => {
                p.earliest_protection_zone_round <= self.round_nr
            }
            Some(CreatureKind::Npc(_)) => true,
            None => false,
        }
    }

    /// `creature:removeSummon(monster)` — clears master and restores loot/skill flags.
    pub fn lua_script_remove_summon(
        &mut self,
        master_u64: u64,
        summon_u64: u64,
    ) -> Result<bool, String> {
        let master = self
            .resolve_creature_u64(master_u64)
            .ok_or_else(|| "removeSummon: master not found".to_string())?;
        let summon = self
            .resolve_creature_u64(summon_u64)
            .ok_or_else(|| "removeSummon: summon not found".to_string())?;
        let Some(CreatureKind::Monster(m)) = self.creatures.get(summon) else {
            return Ok(false);
        };
        if m.base.master != Some(master) {
            return Ok(false);
        }
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(summon) {
            m.base.clear_targets();
            m.base.master = None;
            m.base.drop_loot = true;
            m.base.skill_loss = true;
        }
        Ok(true)
    }

    /// `Creature:setMonsterOutfit(monster, time)` — `CONDITION_OUTFIT` via native apply.
    pub fn lua_script_set_monster_outfit(
        &mut self,
        creature_u64: u64,
        monster_name: &str,
        ticks_ms: i32,
    ) -> Result<bool, String> {
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "setMonsterOutfit: creature not found".to_string())?;
        let Some(mtype) = self.monsters_db.get_by_name(monster_name) else {
            return Ok(false);
        };
        if matches!(self.creatures.get(cid), Some(CreatureKind::Player(_)))
            && !self.player_may_illusion_monster(cid, monster_name)
        {
            return Ok(false);
        }
        let look_type = mtype.outfit.look_type as i32;
        self.apply_outfit_condition(cid, look_type, 0, ticks_ms);
        Ok(true)
    }

    /// `Creature:setItemOutfit(item, time)` — item lookTypeEx illusion.
    pub fn lua_script_set_item_outfit(
        &mut self,
        creature_u64: u64,
        item_type: u16,
        ticks_ms: i32,
    ) -> Result<bool, String> {
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "setItemOutfit: creature not found".to_string())?;
        if self.items_db.items.get(&item_type).is_none() {
            return Ok(false);
        }
        self.apply_outfit_condition(cid, 0, item_type, ticks_ms);
        Ok(true)
    }

    /// `Creature:addDamageCondition(target, type, list, damage, period, rounds)`.
    pub fn lua_script_add_damage_condition(
        &mut self,
        attacker_u64: u64,
        target_u64: u64,
        ctype: i32,
        list: i32,
        damage: i32,
        period: i32,
        rounds: i32,
    ) -> Result<bool, String> {
        if damage <= 0 {
            return Ok(false);
        }
        let target = self
            .resolve_creature_u64(target_u64)
            .ok_or_else(|| "addDamageCondition: target not found".to_string())?;
        let rust_ctype = condition_type_from_lua(ctype);
        if rust_ctype == ConditionType::None {
            return Ok(false);
        }
        if self.creature_is_condition_immune(target, rust_ctype) {
            return Ok(false);
        }
        let cond = build_damage_list_condition(rust_ctype, list, damage, period, rounds, self);
        apply_condition(&mut self.creatures, target, cond);
        self.on_condition_started(target, rust_ctype);
        let _ = attacker_u64;
        Ok(true)
    }

    /// Path exists check for `getPathTo` / reachability probes.
    pub fn creature_has_path_to(
        &self,
        cid: CreatureId,
        target: Position,
        min_target_dist: i32,
        max_target_dist: i32,
        max_search_dist: i32,
    ) -> Option<Vec<Direction>> {
        let max_steps = if max_search_dist > 0 {
            max_search_dist as usize
        } else {
            usize::MAX
        };
        self.get_creature_path_to(cid, target, min_target_dist, max_target_dist, max_steps)
    }

    fn player_may_illusion_monster(&self, cid: CreatureId, monster_name: &str) -> bool {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return true;
        };
        let flags = flags_for_group(&self.groups, p.group_id);
        if has_player_flag(flags, PLAYER_FLAG_CAN_ILLUSION_ALL) {
            return true;
        }
        self.monsters_db
            .get_by_name(monster_name)
            .is_some_and(|m| m.flags.illusionable)
    }

    fn apply_outfit_condition(
        &mut self,
        cid: CreatureId,
        look_type: i32,
        look_type_ex: u16,
        ticks_ms: i32,
    ) {
        let rounds = if ticks_ms > 0 {
            Some((ticks_ms.max(1) + 999) / 1000)
        } else {
            None
        };
        let cond = ActiveCondition::new(
            0,
            0,
            ConditionType::Outfit,
            ConditionData::Outfit {
                look_type,
                look_type_ex,
            },
            rounds,
        );
        apply_condition(&mut self.creatures, cid, cond);
        self.on_condition_started(cid, ConditionType::Outfit);
    }

    fn creature_is_condition_immune(&self, cid: CreatureId, ctype: ConditionType) -> bool {
        match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => match ctype {
                ConditionType::Poison => m.immunity_poison,
                ConditionType::Fire => m.immunity_fire,
                ConditionType::Energy => m.immunity_energy,
                _ => false,
            },
            Some(CreatureKind::Npc(_)) => matches!(
                ctype,
                ConditionType::Poison | ConditionType::Fire | ConditionType::Energy
            ),
            _ => false,
        }
    }
}

fn build_damage_list_condition(
    ctype: ConditionType,
    list: i32,
    damage: i32,
    period: i32,
    rounds: i32,
    world: &GameWorld,
) -> ActiveCondition {
    let period_ms = period.max(1);
    match list {
        DAMAGELIST_CONSTANT_PERIOD => {
            let interval = ((period_ms * 1000).max(1000) + 999) / 1000;
            ActiveCondition::new(
                0,
                0,
                ctype,
                ConditionData::Damage {
                    total_rank: damage,
                    factor_percent: if ctype == ConditionType::Poison {
                        50
                    } else {
                        0
                    },
                },
                Some(rounds.max(1)),
            )
            .with_skill_timer(interval, interval)
        }
        DAMAGELIST_VARYING_PERIOD => {
            let lo = period_ms;
            let hi = rounds.max(lo);
            let interval = world.parity_random(lo, hi);
            ActiveCondition::new(
                0,
                0,
                ctype,
                ConditionData::Damage {
                    total_rank: damage,
                    factor_percent: 50,
                },
                Some(1),
            )
            .with_skill_timer(interval, interval)
        }
        DAMAGELIST_LOGARITHMIC_DAMAGE => {
            let rank = damage.max(1);
            ActiveCondition::new(
                0,
                0,
                ctype,
                ConditionData::Damage {
                    total_rank: rank,
                    factor_percent: 50,
                },
                Some(rank),
            )
            .with_skill_timer(4, 4)
        }
        DAMAGELIST_EXPONENTIAL_DAMAGE | _ => {
            let rank = damage.max(1);
            ActiveCondition::new(
                0,
                0,
                ctype,
                ConditionData::Damage {
                    total_rank: rank,
                    factor_percent: 50,
                },
                Some(rank),
            )
            .with_skill_timer(4, 4)
        }
    }
}
