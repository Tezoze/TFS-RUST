//! Monster AI (native Rust; Lua `onThink` only if registered).
// C++ reference: `monster.cpp` `Monster::doAttacking`, `think`, `onThink`.

use std::collections::HashSet;

use crate::creature::base::CreatureBase;
use tfs_rust_common::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterAiPhase {
    Idle,
    Chase,
    Flee,
    ReturnToSpawn,
}

#[derive(Debug, Clone)]
pub struct Monster {
    pub base: CreatureBase,
    pub spawn_position: Position,
    pub ai_phase: MonsterAiPhase,
    pub think_interval_ms: u32,
    /// Script registration: only if contains `onThink` does core invoke Lua think (Phase 8).
    pub registered_events: HashSet<String>,
    pub friend_list: Vec<String>,
    pub target_list: Vec<String>,
}

impl Monster {
    pub fn new(mut base: CreatureBase, spawn: Position) -> Self {
        base.damage_map.clear();
        Self {
            base,
            spawn_position: spawn,
            ai_phase: MonsterAiPhase::Idle,
            think_interval_ms: 1000,
            registered_events: HashSet::new(),
            friend_list: Vec::new(),
            target_list: Vec::new(),
        }
    }

    pub fn wants_lua_think(&self) -> bool {
        self.registered_events.contains("onThink")
    }
}
