//! Player inventory, skills, economy, social — and level-up.
// C++ reference: `Player` (`player.h` / `player.cpp`).

use std::collections::HashMap;
use std::time::Instant;

use tfs_rust_common::CLIENTOS_OTCLIENT_LINUX;
use tfs_rust_db::{ItemRecord, VipEntry};

use crate::creature::base::CreatureBase;
use crate::creature::vocation::{
    base_walk_speed, experience_to_next_level, recalculate_vitals, total_experience_for_level,
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
    /// `accounts.premium_ends_at` (unix seconds).
    pub premium_ends_at: u32,
    /// Stamina minutes for `0xA0` stats (`players.stamina`).
    pub stamina_minutes: u16,
    /// Offline training time in ms (`players.offlinetraining_time` / C++ `offlineTrainingTime`).
    pub offline_training_ms: u32,
    /// Spell id → game tick when off cooldown.
    pub spell_cooldown_end: HashMap<u16, u64>,
    /// Spell group → game tick when group is off cooldown.
    pub spell_group_cooldown_end: HashMap<u8, u64>,
    /// First-packet OS id (`protocolgame.cpp`); used for OTClient vs official behaviour.
    pub operating_system: u16,
    /// `0` = not OTCv8; otherwise client build from first-packet probe.
    pub otclient_v8: u16,
    /// GM / spectator ghost — hidden from other players’ maps (`Player::isInGhostMode` in TFS).
    pub ghost_mode: bool,
    /// Equipment + store inbox slot snapshot from DB (`player_items` / `player_storeinboxitems`).
    pub inventory_slots: [Option<ItemRecord>; 11],
    /// `sendVIPEntries` payload from `account_viplist`.
    pub vip_list: Vec<VipEntry>,
    /// When true, other players receive `0` health percent on map (`Player::isHealthHidden` in TFS).
    pub health_hidden: bool,
    /// TFS idle / kick — `resetIdleTime` updates this (`player.cpp`).
    pub last_activity: Instant,
    /// TFS `nextAction` — `Player::onWalk` blocks actions until this instant (`player.cpp` ~1343).
    pub next_action_until: Option<Instant>,
}

impl Player {
    /// `NetworkMessage::addItem(..., withDescription)` / OTCv8 item template: empty string before duration.
    /// C++ sets `withDescription` from `otclientV8` (probe after `"OTCv8"`); if the probe is missing,
    /// OTClient still identifies via `operatingSystem >= CLIENTOS_OTCLIENT_LINUX`.
    #[inline]
    pub fn item_with_description(&self) -> bool {
        self.otclient_v8 != 0 || self.operating_system >= CLIENTOS_OTCLIENT_LINUX
    }

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
            let sp = base_walk_speed(self.vocation_id, self.level);
            self.base.speed = sp;
            self.base.base_speed = sp;
        }
    }

    pub fn exp_to_next_level(&self) -> u64 {
        experience_to_next_level(self.level)
    }

    /// TFS `Player::canDoAction` / `nextAction` comparison (`player.cpp`).
    #[inline]
    pub fn timed_action_ready(&self, now: Instant) -> bool {
        self.next_action_until.map_or(true, |t| now >= t)
    }
}
