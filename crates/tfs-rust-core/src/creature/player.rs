//! Player inventory, skills, economy, social — and level-up.
// C++ reference: `Player` (`player.h` / `player.cpp`).

use std::collections::HashMap;
use std::time::Instant;

use tfs_rust_common::game_packet::{UseItemExPayload, UseItemPayload};
use tfs_rust_common::{Position, PlayerSex, CLIENTOS_OTCLIENT_LINUX};
use tfs_rust_db::player::PlayerRecord;
use tfs_rust_db::{ItemRecord, VipEntry};

use crate::ids::ItemId;

use crate::creature::base::CreatureBase;
use crate::creature::light::LightInfo;
use crate::creature::vocation::{
    base_walk_speed, experience_to_next_level, total_experience_for_level, VocationProfile,
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
    /// `accounts.type` — account-level access tier (`enums.h:80-85`
    /// `ACCOUNT_TYPE_NORMAL=1` … `ACCOUNT_TYPE_GOD=6`). Distinct from `group_id`
    /// (which carries `groups.xml` per-character flags). Loaded at login from
    /// `accounts.type` (C++ `iologindata.cpp` `gameworldAuthentication`
    /// `SELECT … type … FROM accounts`). Default `ACCOUNT_TYPE_NORMAL` (1).
    pub account_type: u8,
    /// `players.group_id` — `groups.xml` flags (`player.h` `Group`).
    pub group_id: u16,
    /// `players.sex` — `PLAYERSEX_FEMALE` (0) / `PLAYERSEX_MALE` (1) (`player.h`).
    /// Drives pronoun selection in `Player::getDescription` (`player.cpp:112-116`).
    pub sex: PlayerSex,
    pub vocation_id: i32,
    /// Cached vocation combat block (gains, base_speed, formula, skill multipliers).
    /// Built from `VocationRegistry` at login — `Copy` snapshot for hot-path reads
    /// without threading `&VocationRegistry` through level-up/regen/speed calls.
    pub vocation_profile: VocationProfile,
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
    /// 772 `TConnection::TimeStamp` — last command round (`connections.cc:55`).
    pub last_command_round: u32,
    /// 772 `TConnection::TimeStampAction` — last non-idle command round (`connections.cc:61`).
    pub last_action_round: u32,
    /// 772 `TSkillFed` `Cycle` — food-remaining rounds (`crskill.cc:220` `TimerValue`,
    /// `moveuse.cc:1840` `Skills[SKILL_FED]->TimerValue()`). Decrements each
    /// `ProcessSkills` tick; `0` ⇒ skill inactive ⇒ no HP/mana regen (`crskill.cc:180`).
    pub food_remaining: u32,
    /// 772 `TSkillFed` `Act` — regen interval for `ProcessCreatures` item regen
    /// (`crmain.cc:1087` `RegenInterval = Skills[SKILL_FED]->Get()`).
    /// `0` ⇒ no item regen. Set by eating (each food item sets a fixed interval).
    pub food_level: i32,
    /// 772 `EarliestLogoutRound` — PK-mark clearing timer (`crmain.cc:1102-1105`).
    /// When non-zero and `<= round_nr`, `ClearPlayerkillingMarks` fires and the field
    /// is zeroed. **Stub**: full PK-mark clearing (attacked-players list, aggressor
    /// flag, skull broadcast) is deferred until the PvP aggressor subsystem exists.
    pub earliest_logout_round: u32,
    /// C++ `MessageBufferCount` — flood protection message count (`player.cpp:1064`).
    /// Incremented by `removeMessageBuffer` per say, decremented by `addMessageBuffer`
    /// every 1500ms. Triggers mute escalation when exceeding `maxMessageBuffer`.
    pub message_buffer_count: i32,
    /// C++ `MessageBufferTicks` — accumulator for 1500ms tick interval (`player.cpp:1051`).
    /// Accumulates `onThink` interval; when >= 1500ms, `addMessageBuffer` decrements
    /// `message_buffer_count` and ticks are reset to 0.
    pub message_buffer_ticks: u32,
    /// Last server `sendPing` (`0x1D`) — `Player::lastPing` (`player.cpp`).
    pub last_ping_sent: Instant,
    /// Last client pong — `Player::lastPong` / `receivePing` (`player.cpp`).
    pub last_pong_at: Instant,
    /// TFS `nextAction` — `Player::onWalk` blocks actions until this **logical ms** (`now_ms()` /
    /// `server_ms` on 772). On the logical clock so action gating no longer rides the wall clock
    /// (audit Findings 1/2, Phase 4). C++ `player.cpp` ~1343.
    pub next_action_until: Option<u64>,
    /// Pending action stored by `setNextWalkActionTask` — fired from `onWalkComplete` (`player.cpp` ~3390).
    /// Phase 5: the 1098 reactive `walk_action_due` deadline is deleted; both eras use the ToDoQueue.
    /// `walk_action` remains as a deferred-action marker cleared by `ToDoClear` (audit #3).
    pub walk_action: Option<PlayerWalkAction>,
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
    /// Chase harness — `human.mon` `Defend=5` (`crcombat.cc` `GetDefendValue`).
    pub sim_melee_defense: i32,
    /// `human.mon` `Attack=7` — race-data fist fallback for `GetAttackValue`
    /// (`crcombat.cc:183` `RaceData[Race].Attack`). Mirrors `sim_melee_defense`.
    pub sim_melee_attack: i32,
    /// 772 `TCombat::AttackMode` — fight stance selector (`crcombat.cc:325` `SetAttackMode`).
    /// Default `Balanced`; set by the `0xA7` `FIGHT_MODES` packet (PC-4). PC-1 wires the field;
    /// PC-2's `weapon_damage`/`defense_value` consume it via `apply_attack_mode`/`apply_defense_mode`.
    pub attack_mode: crate::combat::math::FightMode,
}

impl Player {
    /// `NetworkMessage::addItem(..., withDescription)` / OTCv8 item template: empty string before duration.
    /// C++ sets `withDescription` from `otclientV8` (probe after `"OTCv8"`); if the probe is missing,
    /// OTClient still identifies via `operatingSystem >= CLIENTOS_OTCLIENT_LINUX`.
    #[inline]
    pub fn item_with_description(&self) -> bool {
        self.otclient_v8 != 0 || self.operating_system >= CLIENTOS_OTCLIENT_LINUX
    }

    /// OTClient connection flag — TVP gates several wire quirks on
    /// `operatingSystem >= CLIENTOS_OTCLIENT_LINUX` (`protocolgame.cpp:171,265`,
    /// `player_ping.rs:61`). Used by the walk dispatch to route OTClient-on-772
    /// floor changes through TVP's teleport contract (remove + `0x64`), since
    /// OTClient tracks the local player as a tile creature and cannot reconcile
    /// the decompile `NotifyGo` incremental `SendFloors`/`SendRow` stream
    /// (`docs/772_FLOOR_CHANGE_CLIENT_TARGETS.md` §6).
    #[inline]
    pub fn is_otclient(&self) -> bool {
        self.otclient_v8 != 0 || self.operating_system >= CLIENTOS_OTCLIENT_LINUX
    }

    /// C++ `Player::addExperience` — level-up loop updates HP/mana/cap/speed
    /// (`crskill.cc` `Event`). Returns `true` if any level changed (caller should
    /// `announce_creature_speed` — C++ `cract.cc:1637` `CREATURE_SPEED_CHANGED`).
    pub fn add_experience(
        &mut self,
        amount: u64,
        step_speed_model: crate::formulas::StepSpeedModel,
    ) -> bool {
        let old_level = self.level;
        self.experience = self.experience.saturating_add(amount);
        while self.level < 2000
            && self.experience >= total_experience_for_level((self.level + 1) as u32)
        {
            self.level += 1;
            let (max_hp, max_mana, cap) = self.vocation_profile.recalculate_vitals(self.level);
            self.base.max_health = max_hp;
            self.base.health = self.base.health.min(max_hp).max(1);
            self.max_mana = max_mana;
            self.mana = self.mana.min(max_mana);
            self.capacity = cap;
            let sp = base_walk_speed(step_speed_model, &self.vocation_profile, self.level);
            self.base.speed = sp;
            self.base.base_speed = sp;
        }
        self.level != old_level
    }

    /// Remove experience and apply level-down recalculation (`Player::removeExperience`-style outcome).
    /// C++ `Player::removeExperience` — level-down loop. Returns `true` if level changed.
    pub fn remove_experience(
        &mut self,
        amount: u64,
        step_speed_model: crate::formulas::StepSpeedModel,
    ) -> bool {
        let old_level = self.level;
        self.experience = self.experience.saturating_sub(amount);
        while self.level > 1 && self.experience < total_experience_for_level(self.level as u32) {
            self.level -= 1;
            let (max_hp, max_mana, cap) = self.vocation_profile.recalculate_vitals(self.level);
            self.base.max_health = max_hp;
            self.base.health = self.base.health.min(max_hp).max(1);
            self.max_mana = max_mana;
            self.mana = self.mana.min(max_mana);
            self.capacity = cap;
            let sp = base_walk_speed(step_speed_model, &self.vocation_profile, self.level);
            self.base.speed = sp;
            self.base.base_speed = sp;
        }
        self.level != old_level
    }

    pub fn exp_to_next_level(&self) -> u64 {
        experience_to_next_level(self.level)
    }

    /// TFS `Player::canDoAction` / `nextAction` comparison (`player.cpp`). `now_ms` is logical.
    #[inline]
    pub fn timed_action_ready(&self, now_ms: u64) -> bool {
        self.next_action_until.is_none_or(|t| now_ms >= t)
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
        crate::inventory::slot_to_array_index(slot).is_some_and(|idx| self.inventory_abilities[idx])
    }

    /// TFS `Player::setItemAbility` — `player.h`.
    pub fn set_item_ability(&mut self, slot: u8, enabled: bool) {
        if let Some(idx) = crate::inventory::slot_to_array_index(slot) {
            self.inventory_abilities[idx] = enabled;
        }
    }
}
