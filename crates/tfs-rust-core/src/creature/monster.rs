//! Monster AI (native Rust; Lua `onThink` only if registered).
// C++ reference: `monster.cpp` `Monster::onThink`, `searchTarget`, `getDistanceStep`.

use std::collections::HashSet;

use crate::creature::base::CreatureBase;
use crate::ids::CreatureId;
use tfs_rust_common::Position;
use tfs_rust_content::monsters::MonsterTypeFlags;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterAiPhase {
    Idle,
    Chase,
    Flee,
    ReturnToSpawn,
}

/// AI flags copied from [`MonsterTypeFlags`] at spawn (`monsters.h` defaults).
#[derive(Debug, Clone, Copy)]
pub struct MonsterAiConfig {
    pub target_distance: i32,
    pub run_away_health: i32,
    pub static_attack_chance: u32,
    pub can_push_creatures: bool,
    pub can_push_items: bool,
    pub is_hostile: bool,
    /// C++ `MonsterType::changeTargetSpeed` — `monsters.h`.
    pub change_target_speed: u32,
    /// C++ `MonsterType::changeTargetChance` — `monsters.h`.
    pub change_target_chance: i32,
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
            is_hostile: d.is_hostile,
            change_target_speed: d.change_target_speed,
            change_target_chance: d.change_target_chance,
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
            is_hostile: f.is_hostile,
            change_target_speed: f.change_target_speed,
            change_target_chance: f.change_target_chance,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Monster {
    pub base: CreatureBase,
    pub spawn_position: Position,
    pub ai_phase: MonsterAiPhase,
    pub think_interval_ms: u32,
    /// Script registration: only if contains `onThink` does core invoke Lua think (Phase 8).
    pub registered_events: HashSet<String>,
    pub target_distance: i32,
    pub run_away_health: i32,
    pub static_attack_chance: u32,
    pub can_push_creatures: bool,
    pub can_push_items: bool,
    pub is_hostile: bool,
    pub is_idle: bool,
    pub walking_to_spawn: bool,
    pub change_target_speed: u32,
    pub change_target_chance: i32,
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
            ai_phase: MonsterAiPhase::Idle,
            think_interval_ms: 1000,
            registered_events: HashSet::new(),
            target_distance: config.target_distance,
            run_away_health: config.run_away_health,
            static_attack_chance: config.static_attack_chance,
            can_push_creatures: config.can_push_creatures,
            can_push_items: config.can_push_items,
            is_hostile: config.is_hostile,
            is_idle: true,
            walking_to_spawn: false,
            change_target_speed: config.change_target_speed,
            change_target_chance: config.change_target_chance,
            target_change_ticks: 0,
            target_change_cooldown: 0,
            challenge_focus_duration: 0,
            opponent_ids: Vec::new(),
            friend_ids: Vec::new(),
        }
    }

    pub fn wants_lua_think(&self) -> bool {
        self.registered_events.contains("onThink")
    }

    /// TFS `Monster::isFleeing` — `monster.h` ~154.
    pub fn is_fleeing(&self) -> bool {
        !self.base.is_summon()
            && self.run_away_health > 0
            && self.base.health <= self.run_away_health
            && self.challenge_focus_duration == 0
    }
}
