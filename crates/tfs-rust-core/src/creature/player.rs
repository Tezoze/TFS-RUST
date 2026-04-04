//! Player inventory, skills, economy, social — and level-up.
// C++ reference: `Player` (`player.h` / `player.cpp`).

use crate::creature::base::CreatureBase;
use crate::creature::vocation::{
    experience_to_next_level, recalculate_vitals, total_experience_for_level,
};

#[derive(Debug, Clone, Default)]
pub struct PlayerInventory {
    /// Placeholder until container slots are modeled (Phase 7+).
    pub capacity_slots: u16,
}

#[derive(Debug, Clone)]
pub struct PlayerSkills {
    pub fist: i32,
    pub club: i32,
    pub sword: i32,
    pub axe: i32,
    pub dist: i32,
    pub shielding: i32,
    pub fishing: i32,
    pub maglevel: i32,
}

#[derive(Debug, Clone)]
pub struct PlayerEconomy {
    pub balance: u64,
    pub soul: i32,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerSocial {
    pub party_id: Option<u32>,
    pub guild_id: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub base: CreatureBase,
    pub account_id: u32,
    pub guid: u32,
    pub vocation_id: i32,
    pub level: i32,
    pub experience: u64,
    pub mana: i32,
    pub max_mana: i32,
    pub capacity: i32,
    pub inventory: PlayerInventory,
    pub skills: PlayerSkills,
    pub economy: PlayerEconomy,
    pub social: PlayerSocial,
    pub town_id: i32,
}

impl Player {
    pub fn add_experience(&mut self, amount: u64) {
        self.experience = self.experience.saturating_add(amount);
        while self.level < 2000
            && self.experience >= total_experience_for_level((self.level + 1) as u32)
        {
            self.level += 1;
            let (max_hp, max_mana, cap) = recalculate_vitals(self.vocation_id, self.level);
            self.base.max_health = max_hp;
            self.base.health = self.base.health.min(max_hp).max(1);
            self.max_mana = max_mana;
            self.mana = self.mana.min(max_mana);
            self.capacity = cap;
        }
    }

    pub fn exp_to_next_level(&self) -> u64 {
        experience_to_next_level(self.level)
    }
}
