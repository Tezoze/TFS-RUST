//! Player inventory, skills, economy, social — and level-up.
// C++ reference: `Player` (`player.h` / `player.cpp`).

use std::collections::HashMap;
use std::time::Instant;

use tfs_rust_common::game_packet::{UseItemExPayload, UseItemPayload};
use tfs_rust_common::{Position, CLIENTOS_OTCLIENT_LINUX};
use tfs_rust_db::player::PlayerRecord;
use tfs_rust_db::{ItemRecord, VipEntry};

use crate::ids::ItemId;

use crate::creature::base::CreatureBase;
use crate::creature::light::LightInfo;
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

/// Deferred action after auto-walk completes — TFS `Player::walkTask` (`player.cpp` ~1298).
// C++ reference: `game.cpp` `playerMoveItem` (~977), `playerUseItem` (~2233), `playerUseItemEx` (~2156).
#[derive(Debug, Clone)]
pub enum PlayerWalkAction {
    MoveItem {
        from_pos: Position,
        sprite_id: u16,
        from_stack_pos: u8,
        to_pos: Position,
        count: u8,
    },
    UseItem(UseItemPayload),
    UseItemEx(UseItemExPayload),
}

/// SQL + item payloads copied at login for fields not fully mirrored in runtime `Player`.
// C++ ref: `Player` fields carried across session until `IOLoginData::savePlayer`.
#[derive(Debug, Clone)]
pub struct PlayerPersistBaseline {
    pub player_row: PlayerRecord,
    pub spells: Vec<String>,
    pub storage: Vec<(u32, i32)>,
    pub depot: Vec<ItemRecord>,
    pub inbox: Vec<ItemRecord>,
    /// C++ `Player::lastDepotId` — `-1` skips depot `DELETE`/`INSERT` in `savePlayer`.
    pub last_depot_id: i32,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub base: CreatureBase,
    pub account_id: u32,
    pub guid: u32,
    /// `players.group_id` — `groups.xml` flags (`player.h` `Group`).
    pub group_id: u16,
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
    /// Runtime equipment + store inbox: `CONST_SLOT_HEAD`..=`CONST_SLOT_AMMO` + `CONST_SLOT_STORE_INBOX`.
    /// Array index `i` = slot `i + 1` for 0..9, index 10 = store inbox (`src/creature.h` `slots_t`).
    pub equipment_slots: [Option<ItemId>; 11],
    /// Sum of `Item::getWeight` for slots 1–10 + store inbox contents — `Player::inventoryWeight` (`player.cpp`).
    pub inventory_weight: u32,
    /// Max light from equipped items — `Player::itemsLight` (`player.h`).
    pub items_light: LightInfo,
    /// MoveEvent ability guard per slot — `Player::inventoryAbilities` (`player.h`).
    pub inventory_abilities: [bool; 11],
    /// Active NPC shop session — `Player::shopOwner` (`player.h`); list refresh deferred until shop runtime.
    pub shop_owner: Option<u32>,
    /// `sendVIPEntries` payload from `account_viplist`.
    pub vip_list: Vec<VipEntry>,
    /// When true, other players receive `0` health percent on map (`Player::isHealthHidden` in TFS).
    pub health_hidden: bool,
    /// TFS idle / kick — `resetIdleTime` updates this (`player.cpp`).
    pub last_activity: Instant,
    /// Last server `sendPing` (`0x1D`) — `Player::lastPing` (`player.cpp`).
    pub last_ping_sent: Instant,
    /// Last client pong — `Player::lastPong` / `receivePing` (`player.cpp`).
    pub last_pong_at: Instant,
    /// TFS `nextAction` — `Player::onWalk` blocks actions until this instant (`player.cpp` ~1343).
    pub next_action_until: Option<Instant>,
    /// Pending action stored by `setNextWalkActionTask` — fired from `onWalkComplete` (`player.cpp` ~3390).
    pub walk_action: Option<PlayerWalkAction>,
    /// When `walk_action` should run (`createSchedulerTask(400, ...)` in `game.cpp`).
    pub walk_action_due: Option<Instant>,
    /// Town id → live depot chest root — C++ `Player::depotChests` (`player.h`).
    pub depot_chests: HashMap<u32, ItemId>,
    /// Map locker town id → virtual locker item — C++ `depotLockerMap`.
    pub depot_lockers: HashMap<u32, ItemId>,
    /// C++ `Player::inbox` — lazy-created inbox container item.
    pub inbox_root: Option<ItemId>,
    /// C++ `Player::lastDepotId` — `-1` skips depot save until a depot is opened.
    pub last_depot_id: i32,
    /// Present for characters that logged in via DB; required for `IOLoginData::savePlayer`.
    pub persist: Option<PlayerPersistBaseline>,
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
        self.next_action_until.is_none_or(|t| now >= t)
    }

    /// `Player::getCapacity` — `player.h` ~454–461.
    #[inline]
    pub fn get_capacity_u32_with_flags(&self, cannot_pickup: bool, infinite_capacity: bool) -> u32 {
        if cannot_pickup {
            0
        } else if infinite_capacity {
            u32::MAX
        } else {
            self.capacity.max(0) as u32
        }
    }

    /// `Player::getFreeCapacity` — `player.h` ~463–471.
    #[inline]
    pub fn get_free_capacity_u32_with_flags(
        &self,
        cannot_pickup: bool,
        infinite_capacity: bool,
    ) -> u32 {
        if cannot_pickup {
            0
        } else if infinite_capacity {
            u32::MAX
        } else {
            self.get_capacity_u32_with_flags(false, false)
                .saturating_sub(self.inventory_weight)
        }
    }

    /// TFS `Player::isItemAbilityEnabled` — `player.h`.
    #[inline]
    pub fn is_item_ability_enabled(&self, slot: u8) -> bool {
        crate::inventory::slot_to_array_index(slot)
            .is_some_and(|idx| self.inventory_abilities[idx])
    }

    /// TFS `Player::setItemAbility` — `player.h`.
    pub fn set_item_ability(&mut self, slot: u8, enabled: bool) {
        if let Some(idx) = crate::inventory::slot_to_array_index(slot) {
            self.inventory_abilities[idx] = enabled;
        }
    }
}
