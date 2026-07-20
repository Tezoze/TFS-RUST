//! Monster AI (native Rust; Lua `onThink` only if registered).
// C++ reference: `monster.cpp` `Monster::onThink`, `searchTarget`, `getDistanceStep`.

use std::collections::HashSet;
use std::sync::Arc;

use crate::creature::base::CreatureBase;
use crate::creature::monster_combat::{combat_from_monster_type, MonsterSpell};
use crate::creature::monster_inventory::MonsterInventory;
use crate::ids::CreatureId;
use tfs_rust_common::enums::BloodType;
use tfs_rust_common::Position;
use tfs_rust_content::monsters::{MonsterType, MonsterTypeFlags};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterAiPhase {
    Idle,
    Chase,
    Flee,
    ReturnToSpawn,
}

/// 772 reference `STATE` — `enums.hh`; used by both eras (Phase 3: 1098 now runs 772 monster AI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MonsterState {
    /// Spawn default — `crnonpl.cc:1516` `TNonplayer` constructor.
    #[default]
    Sleeping,
    /// Awake non-combat — `crnonpl.cc:2388`.
    Idle,
    /// Hit while not in combat posture — E5 sets; `crnonpl.cc:2278`.
    UnderAttack,
    /// Melee posture — `crnonpl.cc:2706`.
    Attacking,
    /// Flee posture after damage — E5 sets; `crnonpl.cc:2282`.
    Panic,
}

/// AI flags and combat data copied from [`MonsterType`] at spawn.
#[derive(Debug, Clone)]
pub struct MonsterAiConfig {
    pub target_distance: i32,
    pub run_away_health: i32,
    pub static_attack_chance: u32,
    pub can_push_creatures: bool,
    pub can_push_items: bool,
    pub pushable: bool,
    pub is_hostile: bool,
    /// C++ `MonsterType::changeTargetSpeed` — `monsters.h`.
    pub change_target_speed: u32,
    /// C++ `MonsterType::changeTargetChance` — `monsters.h`.
    pub change_target_chance: i32,
    /// 772 `RaceData[].LoseTarget` — random target drop per idle (`crnonpl.cc:2381`).
    pub lose_target_percent: u8,
    /// 772 cumulative `Strategy[]` thresholds (`crnonpl.cc:2424`); default nearest-only.
    pub strategy_nearest: u8,
    pub strategy_health: u8,
    pub strategy_damage: u8,
    /// 772 `RaceData[].Talks` — idle talk RNG gate (`crnonpl.cc:2392`).
    pub talks: u8,
    /// TVP-772 `<attack name="melee" skill= attack=>` — feeds `probe_value` / `max_melee_damage_monster`.
    pub melee_skill: i32,
    pub melee_attack: i32,
    /// Melee poison tail cycles — `crcombat.cc:660`, `<attack poisoncycles=`.
    pub poison_cycles: i32,
    /// `<defenses armor= defense=>` — `crcombat.cc:285`, `GetDefendDamage`.
    pub armor: i32,
    pub defense: i32,
    /// `<immunity poison="1"/>` at spawn — `crmain.cc:548` `NoPoison`.
    pub immunity_poison: bool,
    /// `<immunity fire="1"/>` — `crmain.cc:549` `NoBurning`.
    pub immunity_fire: bool,
    /// `<immunity energy="1"/>` — `crmain.cc:550` `NoEnergy`.
    pub immunity_energy: bool,
    /// `<immunity lifedrain="1"/>` — `crmain.cc:619` `NoLifeDrain`. PC-3 (M3′).
    pub immunity_life_drain: bool,
    /// `<immunity invisible="1"/>` — `crmain.cc:1493` `SeeInvisible`.
    pub see_invisible: bool,
    /// `<immunity physical="1"/>` — `crmain.cc:615` `RaceData[Race].NoHit`. Physical damage
    /// immunity: `Damage(PHYSICAL)` emits `EFFECT_BLOCK_HIT` (4) and returns 0.
    pub immunity_physical: bool,
    /// Non-melee attacks from `<attacks>` — idle CASTING (E4).
    pub spells: Vec<MonsterSpell>,
    /// 772 `RaceData[].Talk` text list — `<voices><voice sentence="…"/></voices>`
    /// (`crnonpl.cc:2442`, `crmain.cc:1551`). Empty = no broadcast (gate still draws for RNG parity).
    pub talk_texts: Arc<[String]>,
}

impl Default for MonsterAiConfig {
    fn default() -> Self {
        let d = MonsterTypeFlags::default();
        Self {
            target_distance: d.target_distance,
            run_away_health: d.run_away_health,
            static_attack_chance: d.static_attack_chance,
            can_push_creatures: d.can_push_creatures,
            can_push_items: d.can_push_items,
            pushable: d.pushable,
            is_hostile: d.is_hostile,
            change_target_speed: d.change_target_speed,
            change_target_chance: d.change_target_chance,
            lose_target_percent: 0,
            strategy_nearest: 100,
            strategy_health: 0,
            strategy_damage: 0,
            talks: 0,
            melee_skill: 0,
            melee_attack: 0,
            poison_cycles: 0,
            armor: 0,
            defense: 0,
            immunity_poison: false,
            immunity_fire: false,
            immunity_energy: false,
            immunity_life_drain: false,
            see_invisible: false,
            immunity_physical: false,
            spells: Vec::new(),
            talk_texts: Arc::from([]),
        }
    }
}

impl From<MonsterTypeFlags> for MonsterAiConfig {
    fn from(f: MonsterTypeFlags) -> Self {
        Self {
            target_distance: f.target_distance,
            run_away_health: f.run_away_health,
            static_attack_chance: f.static_attack_chance,
            can_push_creatures: f.can_push_creatures,
            can_push_items: f.can_push_items,
            pushable: f.pushable,
            is_hostile: f.is_hostile,
            change_target_speed: f.change_target_speed,
            change_target_chance: f.change_target_chance,
            lose_target_percent: 0,
            strategy_nearest: 100,
            strategy_health: 0,
            strategy_damage: 0,
            talks: 0,
            melee_skill: 0,
            melee_attack: 0,
            poison_cycles: 0,
            armor: 0,
            defense: 0,
            immunity_poison: false,
            immunity_fire: false,
            immunity_energy: false,
            immunity_life_drain: false,
            see_invisible: false,
            immunity_physical: false,
            spells: Vec::new(),
            talk_texts: Arc::from([]),
        }
    }
}

impl MonsterAiConfig {
    /// Full spawn config: movement flags + combat snapshot from parsed monster type.
    pub fn from_monster_type(mtype: &MonsterType) -> Self {
        let mut cfg = Self::from(mtype.flags);
        let combat = combat_from_monster_type(mtype);
        cfg.melee_skill = combat.melee_skill;
        cfg.melee_attack = combat.melee_attack;
        cfg.poison_cycles = combat.poison_cycles;
        cfg.armor = combat.armor;
        cfg.defense = combat.defense;
        cfg.immunity_poison = combat.immunity_poison;
        cfg.immunity_fire = combat.immunity_fire;
        cfg.immunity_energy = combat.immunity_energy;
        cfg.see_invisible = combat.see_invisible;
        cfg.immunity_physical = combat.immunity_physical;
        cfg.spells = combat.spells;
        cfg.talk_texts = Arc::from(mtype.talk_texts.clone());
        // 772 `RaceData[].Talks` is the count of `Talk` entries (`crmain.cc:1552`).
        cfg.talks = cfg.talk_texts.len().min(u8::MAX as usize) as u8;
        cfg
    }
}

#[derive(Debug, Clone)]
pub struct Monster {
    pub base: CreatureBase,
    pub spawn_position: Position,
    /// CipSoft `TMonsterhome::Radius` — per-home roam radius (axis box) used by the non-attacking
    /// `MovePossible` leash (`crnonpl.cc:2148-2157`, `MonsterhomeInRange`). `0` = unset → fall back
    /// to the global despawn radius. Set from the spawn zone `radius` at spawn time.
    pub home_radius: i32,
    pub ai_phase: MonsterAiPhase,
    pub think_interval_ms: u32,
    /// Script registration: only if contains `onThink` does core invoke Lua think (Phase 8).
    pub registered_events: HashSet<String>,
    pub target_distance: i32,
    pub run_away_health: i32,
    pub static_attack_chance: u32,
    pub can_push_creatures: bool,
    pub can_push_items: bool,
    pub pushable: bool,
    pub is_hostile: bool,
    pub is_idle: bool,
    /// E6 harness — keep `Sleeping` posture until `player_damage` when scenario sets `monster_state sleeping`.
    pub harness_preserve_sleep: bool,
    /// 772 combat/lifecycle posture — `enums.hh` `STATE`; 1098 ignores.
    pub state: MonsterState,
    /// Last `(state, chase_mode)` emitted to chase JSONL — harness dedupe only.
    /// `chase_mode` lives on [`CreatureBase`] (shared with players); this dedupe key reads it.
    pub(crate) last_combat_trace: Option<(MonsterState, crate::creature::ChaseMode)>,
    /// Last `IdleStimulus` drain ms — one pass per beat (`crnonpl.cc:2345`).
    pub(crate) idle_stimulus_last_ms: Option<u64>,
    pub walking_to_spawn: bool,
    pub change_target_speed: u32,
    pub change_target_chance: i32,
    pub lose_target_percent: u8,
    pub strategy_nearest: u8,
    pub strategy_health: u8,
    pub strategy_damage: u8,
    pub talks: u8,
    /// 772 `RaceData[].Talk` text list — `<voices>` (`crnonpl.cc:2442`). Empty = no broadcast.
    pub talk_texts: Arc<[String]>,
    pub melee_skill: i32,
    pub melee_attack: i32,
    pub poison_cycles: i32,
    /// Effective armor after `CheckCombatValues` (race + equipped). Prefer
    /// [`GameWorld::monster_get_armor_strength`] for strike snapshots.
    pub armor: i32,
    /// Race/fist defense when no shield/weapon; overwritten only by race spawn.
    /// Strike snapshots use [`GameWorld::monster_get_defend_value`].
    pub defense: i32,
    /// Immutable race armor from `<defenses armor=>` / `RaceData.Armor` — `GetArmorStrength` base.
    pub race_armor: i32,
    /// Immutable race defense from `<defenses defense=>` / `RaceData.Defend`.
    pub race_defense: i32,
    /// Immutable race melee attack — `CheckCombatValues` weapon override base.
    pub race_melee_attack: i32,
    /// Immutable race fist skill — `ProbeValue` skill for unequipped defend.
    pub race_melee_skill: i32,
    pub immunity_poison: bool,
    /// `<immunity fire="1"/>` — `crmain.cc:549` `NoBurning`.
    pub immunity_fire: bool,
    /// `<immunity energy="1"/>` — `crmain.cc:550` `NoEnergy`.
    pub immunity_energy: bool,
    /// `<immunity lifedrain="1"/>` — `crmain.cc:619` `NoLifeDrain`. PC-3 (M3′).
    pub immunity_life_drain: bool,
    /// `<immunity invisible="1"/>` — `crmain.cc:1493` `SeeInvisible`.
    pub see_invisible: bool,
    /// `<immunity physical="1"/>` — `crmain.cc:615` `RaceData[Race].NoHit`. Physical damage
    /// immunity: `Damage(PHYSICAL)` emits `EFFECT_BLOCK_HIT` (4) and returns 0.
    pub immunity_physical: bool,
    pub spells: Vec<MonsterSpell>,
    /// Race XP grant on death — `MonsterType.experience` / `crcombat.cc:908`.
    pub experience: u32,
    /// `<look corpse=…>` item id for death drop — `crmain.cc:204`.
    pub corpse_id: u16,
    /// Blood family — on-hit effect + death/hit splash liquid. Set at spawn from `MonsterType.race`
    /// (`crmain.cc:706-776` on-hit, `crmain.cc:207-226` death pool).
    pub blood: BloodType,
    /// Spawn-rolled bag + equip — `crnonpl.cc:2050`.
    pub inventory: MonsterInventory,
    /// C++ `Monster::targetChangeTicks` — `monster.cpp` `onThinkTarget`.
    pub target_change_ticks: u32,
    /// C++ `Monster::targetChangeCooldown`.
    pub target_change_cooldown: u32,
    /// C++ `Monster::challengeFocusDuration` — blocks flee while challenged.
    pub challenge_focus_duration: u32,
    /// C++ `Monster::targetList` — live hostile creature ids in view.
    pub opponent_ids: Vec<CreatureId>,
    /// C++ `Monster::friendList`.
    pub friend_ids: Vec<CreatureId>,
    /// C++ `Monster::setID` — auto-incrementing wire id starting at `0x40000000`
    /// (`monster.h:43-46`, `monster.cpp:18`). Never reused across spawns, preventing
    /// wire-id collisions when SlotMap slots are recycled (dead dragon → new skeleton
    /// at the same slot would share a wire-id without this, making the client show the
    /// stale dragon sprite with no name/HP bar).
    pub wire_id: u32,
}

impl Monster {
    pub fn new(base: CreatureBase, spawn: Position) -> Self {
        Self::with_config(base, spawn, MonsterAiConfig::default())
    }

    pub fn with_config(mut base: CreatureBase, spawn: Position, config: MonsterAiConfig) -> Self {
        base.damage_map.clear();
        Self {
            base,
            spawn_position: spawn,
            home_radius: 0,
            ai_phase: MonsterAiPhase::Idle,
            think_interval_ms: 1000,
            registered_events: HashSet::new(),
            target_distance: config.target_distance,
            run_away_health: config.run_away_health,
            static_attack_chance: config.static_attack_chance,
            can_push_creatures: config.can_push_creatures,
            can_push_items: config.can_push_items,
            pushable: config.pushable,
            is_hostile: config.is_hostile,
            is_idle: true,
            harness_preserve_sleep: false,
            state: MonsterState::Sleeping,
            last_combat_trace: None,
            idle_stimulus_last_ms: None,
            walking_to_spawn: false,
            change_target_speed: config.change_target_speed,
            change_target_chance: config.change_target_chance,
            lose_target_percent: config.lose_target_percent,
            strategy_nearest: config.strategy_nearest,
            strategy_health: config.strategy_health,
            strategy_damage: config.strategy_damage,
            talks: config.talks,
            talk_texts: config.talk_texts,
            melee_skill: config.melee_skill,
            melee_attack: config.melee_attack,
            poison_cycles: config.poison_cycles,
            armor: config.armor,
            defense: config.defense,
            race_armor: config.armor,
            race_defense: config.defense,
            race_melee_attack: config.melee_attack,
            race_melee_skill: config.melee_skill,
            immunity_poison: config.immunity_poison,
            immunity_fire: config.immunity_fire,
            immunity_energy: config.immunity_energy,
            immunity_life_drain: config.immunity_life_drain,
            see_invisible: config.see_invisible,
            immunity_physical: config.immunity_physical,
            spells: config.spells,
            experience: 0,
            corpse_id: 0,
            blood: BloodType::Blood,
            inventory: MonsterInventory::default(),
            target_change_ticks: 0,
            target_change_cooldown: 0,
            challenge_focus_duration: 0,
            opponent_ids: Vec::new(),
            friend_ids: Vec::new(),
            wire_id: 0,
        }
    }

    pub fn wants_lua_think(&self) -> bool {
        self.registered_events.contains("onThink")
    }

    /// TFS `Monster::isPushable` — `monster.h` (`pushable && baseSpeed != 0`).
    pub fn is_pushable(&self) -> bool {
        self.pushable && self.base.speed != 0
    }

    /// 772 `TMonster::IsFleeing` — `crnonpl.cc:3136` (HP threshold only; PANIC is separate).
    pub fn is_fleeing(&self) -> bool {
        if self.base.is_summon() || self.challenge_focus_duration > 0 {
            return false;
        }
        self.run_away_health > 0 && self.base.health <= self.run_away_health
    }
}
