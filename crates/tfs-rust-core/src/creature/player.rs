//! Player inventory, skills, economy, social — and level-up.
// C++ reference: `Player` (`player.h` / `player.cpp`).

use std::collections::HashMap;
use std::time::Instant;

use tfs_rust_common::game_packet::{UseItemExPayload, UseItemPayload};
use tfs_rust_common::{PlayerSex, Position, CLIENTOS_OTCLIENT_LINUX};
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
    /// Per-level tries toward next skill level (DB `skill_*_tries`).
    pub fist_tries: u64,
    pub club_tries: u64,
    pub sword_tries: u64,
    pub axe_tries: u64,
    pub dist_tries: u64,
    pub shielding_tries: u64,
    pub fishing_tries: u64,
    /// Tries toward next magic level (DB `manaspent`).
    pub manaspent: u64,
}

impl Default for PlayerSkills {
    fn default() -> Self {
        Self {
            fist: 10,
            club: 10,
            sword: 10,
            axe: 10,
            dist: 10,
            shielding: 10,
            fishing: 10,
            maglevel: 0,
            fist_tries: 0,
            club_tries: 0,
            sword_tries: 0,
            axe_tries: 0,
            dist_tries: 0,
            shielding_tries: 0,
            fishing_tries: 0,
            manaspent: 0,
        }
    }
}

impl PlayerSkills {
    /// Zero-tries copy with the given skill levels (test / stub helpers).
    pub fn with_levels(
        fist: i32,
        club: i32,
        sword: i32,
        axe: i32,
        dist: i32,
        shielding: i32,
        fishing: i32,
        maglevel: i32,
    ) -> Self {
        Self {
            fist,
            club,
            sword,
            axe,
            dist,
            shielding,
            fishing,
            maglevel,
            ..Self::default()
        }
    }
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
    /// 772 `PartyLeavingRound` — non-zero after leave; CheckFormer window +5 rounds.
    pub party_leaving_round: u32,
    /// Party id retained for CheckFormer after leave (`GetPartyLeader(true)`).
    pub former_party_id: Option<u32>,
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

/// TFS `OutfitEntry` — unlocked lookType (+ addons on 1098) (`player.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutfitEntry {
    pub look_type: u16,
    /// Bitmask of owned addons (1098). Always `0` on 772.
    pub addons: u8,
}

/// Reserved player-storage keys for outfit ownership (`src/const.h`).
pub const PSTRG_OUTFITS_RANGE_START: u32 = 10_001_000;
pub const PSTRG_OUTFITS_RANGE_SIZE: u32 = 500;

#[inline]
pub fn storage_key_is_outfit(key: u32) -> bool {
    key >= PSTRG_OUTFITS_RANGE_START
        && key.saturating_sub(PSTRG_OUTFITS_RANGE_START) <= PSTRG_OUTFITS_RANGE_SIZE
}

/// Pull `OUTFITS_RANGE` keys out of DB storage into [`OutfitEntry`] list (TFS `addStorageValue`).
pub fn take_outfits_from_storage(storage: &mut Vec<(u32, i32)>) -> Vec<OutfitEntry> {
    let mut outfits = Vec::new();
    storage.retain(|(key, value)| {
        if storage_key_is_outfit(*key) {
            let look_type = ((*value as u32) >> 16) as u16;
            let addons = (*value as u32 & 0xFF) as u8;
            outfits.push(OutfitEntry { look_type, addons });
            false
        } else {
            true
        }
    });
    outfits
}

/// Rewrite reserved outfit keys into `storage` for save (`Player::genReservedStorageRange`).
pub fn write_outfits_into_storage(storage: &mut Vec<(u32, i32)>, outfits: &[OutfitEntry]) {
    storage.retain(|(k, _)| !storage_key_is_outfit(*k));
    let mut base_key = PSTRG_OUTFITS_RANGE_START;
    for entry in outfits {
        base_key += 1;
        let value = ((u32::from(entry.look_type) << 16) | u32::from(entry.addons)) as i32;
        storage.push((base_key, value));
    }
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
    /// Cached `PlayerFlag_SetMaxSpeed` from the character's group. When true,
    /// base walk speed is pinned to 1500 (TFS `PLAYER_MAX_SPEED`).
    pub set_max_speed: bool,
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
    /// Condition / spell light — `Creature::internalLight` (`creature.h`).
    /// `Player::getCreatureLight` returns max(internal, items).
    pub internal_light: LightInfo,
    /// MoveEvent ability guard per slot — `Player::inventoryAbilities` (`player.h`).
    pub inventory_abilities: [bool; 11],
    /// Equipment skill modifiers — 772 `TSkill::DAct` (`crskill.cc:19-25`).
    pub dact_skills: [i32; 7],
    /// Timed/magic skill modifiers — 772 `TSkill::MDAct` (Event zeroes only this term).
    pub mdact_skills: [i32; 7],
    /// Last resolved `CombatWeapons` snapshot for `CheckCombatValues` (`crcombat.cc:128-147`).
    pub last_combat_weapons: crate::player::combat::values::CombatWeapons,
    /// TFS `Player::varStats` — equipment stat modifiers (`stats_t`, `player.h`).
    pub var_stats: [i32; 4],
    /// TFS `Player::conditionSuppressions` — `addConditionSuppressions` (`player.cpp`).
    pub condition_suppressions: u32,
    /// Active NPC shop session — `Player::shopOwner` (`player.h`); list refresh deferred until shop runtime.
    pub shop_owner: Option<u32>,
    /// `sendVIPEntries` payload from `account_viplist`.
    pub vip_list: Vec<VipEntry>,
    /// Owned/unlocked outfit entries — TFS `Player::outfits` (`player.h`).
    pub outfits: Vec<OutfitEntry>,
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
    /// is zeroed.
    pub earliest_logout_round: u32,
    /// 772 `AttackedPlayers` — victims this player has marked (yellow for them).
    /// Session-only; cleared / moved to former on `ClearPlayerkillingMarks`.
    pub attacked_players: Vec<crate::ids::CreatureId>,
    /// 772 `FormerAttackedPlayers` — copy after clear; justifies for +5 rounds.
    pub former_attacked_players: Vec<crate::ids::CreatureId>,
    /// 772 `Aggressor` — white skull while true (`crplayer.cc:1485-1488`).
    pub aggressor: bool,
    /// 772 `FormerAggressor` — copied on clear; justifies for +5 rounds.
    pub former_aggressor: bool,
    /// 772 `FormerLogoutRound` — round when marks were cleared (`crplayer.cc:1621`).
    pub former_logout_round: u32,
    /// 772 `PlayerData::PlayerkillerEnd` — unix seconds; non-zero ⇒ red skull display
    /// and always-justified target (`crplayer.cc:1445-1458,1658`). Persisted as
    /// `players.skulltime`. Assigned by `RecordMurder` / `CheckPlayerkilling`.
    pub playerkiller_end: i64,
    /// 772 `PlayerData::MurderTimestamps[20]` — unjust kill wall-clock ring (`cr.hh:147`).
    /// Persisted as `players.murder_timestamps` CSV.
    pub murder_timestamps: [i64; 20],
    /// 772 `TCreature::LoggingOut` — `crmain.cc:405` `StartLogout`.
    ///
    /// When set, `ProcessCreatures` removes the character once `logout_allowed` /
    /// `LogoutPossible` succeeds (`crmain.cc:1113-1124`). Dead-connection
    /// `StopFight=false` leaves the body on the map (still fighting until
    /// `LatestAttackTime`) until the combat logout lock expires.
    pub logging_out: bool,
    /// 772 `TCreature::LogoutAllowed` — set by `LogoutPossible` success or `StartLogout(Force)`.
    pub logout_allowed: bool,
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
    /// 772 `TCombat::SecureMode` — PVP safety toggle (`crcombat.cc:348` `SetSecureMode`).
    /// When `true` and `WorldType == Pvp`, attacking an unmarked player is blocked
    /// (`crcombat.cc:374-381,563-568`). Set by the `0xA7` `FIGHT_MODES` packet (PC-4).
    pub secure_mode: bool,
    /// 772 `EarliestProtectionZoneRound` — PZ-entry block after combat (`crmain.cc:439-443`).
    /// Set by `BlockLogout(Delay, BlockProtectionZone=true)`; when `> RoundNr`, the player
    /// cannot enter a protection zone (`tile_query_add_player` → `PlayerIsPzLocked`,
    /// `crplayer.cc:366-369`).
    pub earliest_protection_zone_round: u32,
    /// 772 `TPlayer::OldState` — last icons byte sent via `CheckState` / `0xA2` (`crplayer.cc:1249`).
    /// Compared before `send_player_icons` so we only emit on change.
    pub client_icons: u16,
    /// `players.blessings` bitfield — TFS domain `Player::blessings` (`player.cpp`).
    /// Bits 0–4 = five blessings; bit 5 = twist of fate. Drives death-loss reduction (PC-5).
    pub blessings: i8,
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
    ///
    /// M13: each level-up adds `gain_hp`/`gain_mana` to *current* HP/mana (772
    /// `TSkillAdd::Advance`, `crskill.cc:667-678`), then clamps to the new max —
    /// not a full refill (1098 does refill; era-gate later).
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
            self.base.health = (self.base.health + self.vocation_profile.gain_hp)
                .min(max_hp)
                .max(1);
            self.max_mana = max_mana;
            self.mana = (self.mana + self.vocation_profile.gain_mana)
                .min(max_mana)
                .max(0);
            self.capacity = cap;
            let sp = base_walk_speed(
                step_speed_model,
                &self.vocation_profile,
                self.level,
                self.set_max_speed,
            );
            self.base.speed = sp;
            self.base.base_speed = sp;
        }
        self.level != old_level
    }

    /// Remove experience and apply level-down recalculation (`Player::removeExperience`-style outcome).
    /// C++ `Player::removeExperience` — level-down loop. Returns `true` if level changed.
    /// M13: subtracts `gain_hp`/`gain_mana` from current on each level lost (`Advance` inverse).
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
            self.base.health = (self.base.health - self.vocation_profile.gain_hp)
                .min(max_hp)
                .max(1);
            self.max_mana = max_mana;
            self.mana = (self.mana - self.vocation_profile.gain_mana)
                .min(max_mana)
                .max(0);
            self.capacity = cap;
            let sp = base_walk_speed(
                step_speed_model,
                &self.vocation_profile,
                self.level,
                self.set_max_speed,
            );
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

    /// 772 `TSkill::Act` — raw stored skill value for `Probe` gate (`crskill.cc:560`).
    #[inline]
    pub fn skill_act(&self, skill: crate::player::combat::SkillNr) -> i32 {
        skill.level(&self.skills)
    }

    /// 772 `TSkill::Get` — `crskill.cc:19-25`: `max(Act, Min) + MDAct + DAct`.
    ///
    /// Uses classic `skillTuning.minLevel` (10 for combat skills). Prefer
    /// [`Self::skill_level_profile`] when the active `MechanicsProfile` may override mins.
    #[inline]
    pub fn skill_level(&self, skill: crate::player::combat::SkillNr) -> i32 {
        let min = crate::formulas::SkillTriesTuning::classic().min_level[skill.try_index()];
        self.skill_level_with_min(skill, min)
    }

    /// `Get` with an explicit Min floor (profile / test override).
    #[inline]
    pub fn skill_level_with_min(&self, skill: crate::player::combat::SkillNr, min: i32) -> i32 {
        let idx = skill.try_index();
        let floored = self.skill_act(skill).max(min);
        (floored + self.mdact_skills[idx] + self.dact_skills[idx]).max(0)
    }

    /// `Get` using the active profile's `skill_tries.min_level`.
    #[inline]
    pub fn skill_level_profile(
        &self,
        skill: crate::player::combat::SkillNr,
        profile: &crate::formulas::MechanicsProfile,
    ) -> i32 {
        self.skill_level_with_min(skill, profile.skill_tries.min_level[skill.try_index()])
    }

    /// 772 `TSkillProbe::Event` — zero only `MDAct`, leave equipment `DAct` intact.
    #[inline]
    pub fn clear_skill_mdact(&mut self, skill: crate::player::combat::SkillNr) {
        self.mdact_skills[skill.try_index()] = 0;
    }

    /// TFS `Player::getMagicLevel` — base maglevel + `varStats[STAT_MAGICPOINTS]`.
    #[inline]
    pub fn magic_level(&self) -> i32 {
        (self.skills.maglevel
            + self.var_stats[tfs_rust_content::item_abilities::STAT_MAGICPOINTS])
            .max(0)
    }

    /// Effective max HP including `varStats[STAT_MAXHITPOINTS]`.
    #[inline]
    pub fn effective_max_health(&self) -> i32 {
        (self.base.max_health
            + self.var_stats[tfs_rust_content::item_abilities::STAT_MAXHITPOINTS])
            .max(0)
    }

    /// Effective max mana including `varStats[STAT_MAXMANAPOINTS]`.
    #[inline]
    pub fn effective_max_mana(&self) -> i32 {
        (self.max_mana + self.var_stats[tfs_rust_content::item_abilities::STAT_MAXMANAPOINTS])
            .max(0)
    }

    /// Effective soul including `varStats[STAT_SOULPOINTS]` (clamped ≥ 0).
    #[inline]
    pub fn effective_soul(&self) -> i32 {
        (self.economy.soul + self.var_stats[tfs_rust_content::item_abilities::STAT_SOULPOINTS])
            .max(0)
    }
}
