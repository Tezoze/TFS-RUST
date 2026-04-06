//! Shared creature fields (all creature types).
// C++ reference: `Creature` (`creature.h`).

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use crate::condition::ActiveCondition;
use crate::ids::CreatureId;
use tfs_rust_common::enums::{Direction, SkullType};
use tfs_rust_common::Position;

/// Tokio one-shot aligned with `Creature::eventWalk` (`creature.cpp`) — **not** carried across `Clone`
/// of [`CreatureBase`] (mirrors dropping the scheduler event when copying state).
#[derive(Debug, Default)]
pub struct WalkTimer(Option<tokio::task::JoinHandle<()>>);

impl Clone for WalkTimer {
    fn clone(&self) -> Self {
        Self(None)
    }
}

impl std::ops::Deref for WalkTimer {
    type Target = Option<tokio::task::JoinHandle<()>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for WalkTimer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

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

#[derive(Debug, Clone)]
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
    pub walk_timer: WalkTimer,
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

impl CreatureBase {
    pub fn is_summon(&self) -> bool {
        self.master.is_some()
    }

    pub fn clear_targets(&mut self) {
        self.follow_target = None;
        self.attack_target = None;
    }
}
