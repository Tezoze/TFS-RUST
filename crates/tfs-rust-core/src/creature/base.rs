//! Shared creature fields (all creature types).
// C++ reference: `Creature` (`creature.h`).

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use crate::condition::ActiveCondition;
use crate::ids::CreatureId;
use tfs_rust_common::enums::{Direction, SkullType};
use tfs_rust_common::Position;

/// Outfit mirrors TFS `Outfit_t` / player look fields.
#[derive(Debug, Clone)]
pub struct Outfit {
    pub look_type: i32,
    pub look_head: i32,
    pub look_body: i32,
    pub look_legs: i32,
    pub look_feet: i32,
    pub look_addons: i32,
}

impl Default for Outfit {
    fn default() -> Self {
        Self {
            look_type: 136,
            look_head: 0,
            look_body: 0,
            look_legs: 0,
            look_feet: 0,
            look_addons: 0,
        }
    }
}

/// Damage contribution for XP attribution (`Creature::damageMap` in TFS).
pub type DamageMap = HashMap<CreatureId, u64>;

#[derive(Debug)]
pub struct CreatureBase {
    /// Stable id is the `CreatureId` key in `GameWorld::creatures` (not duplicated here).
    pub name: String,
    pub position: Position,
    pub direction: Direction,
    pub health: i32,
    pub max_health: i32,
    pub outfit: Outfit,
    pub speed: i32,
    pub base_speed: i32,
    pub skull: SkullType,
    /// TFS `Creature::drunkenness` — set by `ConditionDrunk` (`condition.cpp` / `creature.h`).
    pub drunkenness: u32,
    /// Active conditions (merged per TFS `addCondition` rules).
    pub active_conditions: Vec<ActiveCondition>,
    /// TFS `Creature::listWalkDir` — consumed from the **back** in `getNextStep` (`creature.cpp`).
    pub walk_queue: VecDeque<Direction>,
    /// TFS `Creature::lastStep` (`OTSYS_TIME()` ms). We store `Instant` for deltas (`creature.cpp` `onCreatureMove`).
    pub last_step: Option<Instant>,
    /// TFS `Creature::lastStepCost` — 1 normal, 2 floor change, 3 diagonal (`creature.cpp` ~490–498).
    pub last_step_cost: u32,
    /// Ground speed of the **destination** tile for the step that ended at `last_step` (tile entered).
    /// OTClient v8 `Creature::getStepDuration` uses `m_lastStepToPosition` = step **destination**
    /// (`tasks/OTClientv8movement.md`); TFS `Creature::getWalkDelay` also uses **current** tile after move.
    pub last_step_ground_speed: u32,
    /// TFS scheduler: next `Game::checkCreatureWalk` — `None` if `eventWalk == 0`.
    pub next_walk_check: Option<Instant>,
    /// When [`GameWorld`](crate::game_world::GameWorld) has `walk_wake_tx`, one-shot `tokio::time::sleep_until`
    /// tasks (`src/scheduler.cpp` Boost.Asio `steady_timer` + `stopEvent`).
    pub walk_timer: Option<tokio::task::JoinHandle<()>>,
    /// TFS `Creature::cancelNextWalk` — cleared in `addEventWalk`, processed in `onWalk` (`creature.cpp`).
    pub cancel_next_walk: bool,
    /// TFS `Creature::forceUpdateFollowPath` — set when `internalMoveCreature` fails (`src/creature.cpp` ~213);
    /// cleared when follow path refreshes (`creature.cpp` ~153–155, ~1077).
    pub force_update_follow_path: bool,
    /// TFS `Creature::movementBlocked` — Lua `setMovementBlocked` (`creature.h`).
    pub movement_blocked: bool,
    pub follow_target: Option<CreatureId>,
    pub attack_target: Option<CreatureId>,
    pub master: Option<CreatureId>,
    pub damage_map: DamageMap,
}

impl Clone for CreatureBase {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            position: self.position,
            direction: self.direction,
            health: self.health,
            max_health: self.max_health,
            outfit: self.outfit.clone(),
            speed: self.speed,
            base_speed: self.base_speed,
            skull: self.skull,
            drunkenness: self.drunkenness,
            active_conditions: self.active_conditions.clone(),
            walk_queue: self.walk_queue.clone(),
            last_step: self.last_step,
            last_step_cost: self.last_step_cost,
            last_step_ground_speed: self.last_step_ground_speed,
            next_walk_check: self.next_walk_check,
            walk_timer: None,
            cancel_next_walk: self.cancel_next_walk,
            force_update_follow_path: self.force_update_follow_path,
            movement_blocked: self.movement_blocked,
            follow_target: self.follow_target,
            attack_target: self.attack_target,
            master: self.master,
            damage_map: self.damage_map.clone(),
        }
    }
}

impl CreatureBase {
    pub fn is_summon(&self) -> bool {
        self.master.is_some()
    }

    pub fn clear_targets(&mut self) {
        self.follow_target = None;
        self.attack_target = None;
    }
}
