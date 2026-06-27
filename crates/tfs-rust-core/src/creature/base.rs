//! Shared creature fields (all creature types).
// C++ reference: `Creature` (`creature.h`).

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use crate::condition::ActiveCondition;
use crate::creature_todo::CreatureTodo;
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
    /// 772 `NextWakeup` — logical `ServerMilliseconds` deadline (`cract.cc:968`).
    pub next_wakeup: Option<u64>,
    /// 772 step anchor in logical time (paired with `last_step`).
    pub last_step_server_ms: Option<u64>,
    /// 772 `EarliestWalkTime` — earliest logical ms the next `TDGo` may run (`cr.hh:631`, `cract.cc:912–916`).
    pub earliest_walk_server_ms: u64,
    /// 772 `EarliestSpellTime` — spell exhaustion gate (`cr.hh:629`, `magic.cc:770–772` `CheckMana`).
    pub earliest_spell_server_ms: u64,
    /// 772 `EarliestMultiuseTime` — two-object use gate (`cr.hh:630`, `cract.cc:765`, `927–928`).
    pub earliest_multiuse_server_ms: u64,
    /// When [`GameWorld`](crate::game_world::GameWorld) has `walk_wake_tx`, one-shot `tokio::time::sleep_until`
    /// tasks (`src/scheduler.cpp` Boost.Asio `steady_timer` + `stopEvent`).
    pub walk_timer: WalkTimer,
    /// TFS `Creature::cancelNextWalk` — cleared in `addEventWalk`, processed in `onWalk` (`creature.cpp`).
    pub cancel_next_walk: bool,
    /// TFS `Creature::forceUpdateFollowPath` — set when `internalMoveCreature` fails (`src/creature.cpp` ~213);
    /// cleared when follow path refreshes (`creature.cpp` ~153–155, ~1077).
    pub force_update_follow_path: bool,
    /// TFS `Creature::walkUpdateTicks` — ms accumulated toward follow path refresh (`creature.cpp` ~150).
    pub walk_update_ticks: u32,
    /// TFS `Creature::isUpdatingPath` — set when follow path should recompute (`creature.cpp` ~156–161).
    pub is_updating_path: bool,
    /// TFS `Creature::hasFollowPath` — path queued in `listWalkDir` (`creature.h` ~530).
    pub has_follow_path: bool,
    /// TFS `Creature::movementBlocked` — Lua `setMovementBlocked` (`creature.h`).
    pub movement_blocked: bool,
    /// TFS `Player::onCreatureMove` stairhop delay — `CONDITION_PACIFIED` for `STAIRHOP_DELAY` ms
    /// (default 2000 ms) added whenever `oldPos.z != newPos.z` (`player.cpp` ~1392–1398).
    /// Movement requests are rejected while `Instant::now() < stairhop_blocked_until`.
    pub stairhop_blocked_until: Option<Instant>,
    pub follow_target: Option<CreatureId>,
    pub attack_target: Option<CreatureId>,
    pub master: Option<CreatureId>,
    pub damage_map: DamageMap,
    /// C++ `inCheckCreaturesVector` bucket (`game.cpp` `addCreatureCheck`); `None` = not scheduled.
    pub think_check_bucket: Option<u8>,
    /// C++ `TCombat::EarliestAttackTime` — `crcombat.cc:523` `DelayAttack`.
    pub earliest_attack_ms: u64,
    /// C++ `TCombat::EarliestDefendTime` — `crcombat.cc:236` `GetDefendDamage` gate.
    pub earliest_defend_ms: u64,
    /// C++ `TCombat::LastDefendTime` — paired with `EarliestDefendTime` (`crcombat.cc:241-242`).
    pub last_defend_ms: u64,
    /// 772 per-creature ToDo action list (772 idle-driven AI).
    pub todo: CreatureTodo,
}

impl CreatureBase {
    pub fn is_summon(&self) -> bool {
        self.master.is_some()
    }

    pub fn clear_targets(&mut self) {
        self.follow_target = None;
        self.attack_target = None;
    }

    /// TFS `Creature::onCreatureDisappear` — follow half (`creature.cpp` ~465–467).
    pub fn clear_follow_for_target(&mut self, target: CreatureId) {
        if self.follow_target == Some(target) {
            self.follow_target = None;
        }
    }

    /// TFS `Creature::onCreatureDisappear` — attack half (`creature.cpp` ~460–462).
    pub fn clear_attack_for_target(&mut self, target: CreatureId) {
        if self.attack_target == Some(target) {
            self.attack_target = None;
        }
    }

    /// True when no walk deadline is armed — safe to call `addEventWalk` / `monster_arm_event_walk`.
    ///
    /// 1098: `next_walk_check` + Tokio timer. 772: `next_wakeup` + [`ToDoQueue`](crate::todo_queue::ToDoQueue).
    pub fn walk_timer_idle(&self, beat_driven_loop: bool) -> bool {
        if beat_driven_loop {
            self.next_wakeup.is_none()
        } else {
            self.next_walk_check.is_none()
        }
    }

    /// C++ `TCombat::DelayAttack` — `crcombat.cc:523`.
    pub fn delay_attack_ms(&mut self, server_ms: u64, ms: u64) {
        self.earliest_attack_ms = self.earliest_attack_ms.max(server_ms.saturating_add(ms));
    }

    /// Whether an attack todo may execute or be enqueued (`cract.cc:909` `TDAttack`).
    pub fn attack_ready_at(&self, server_ms: u64, earliest_spell_ms: u64) -> bool {
        server_ms >= self.earliest_attack_ms.max(earliest_spell_ms)
    }

    /// C++ `CheckMana` — `magic.cc:770–772` max-assign spell exhaustion.
    pub fn delay_spell_ms(&mut self, server_ms: u64, delay_ms: u64) {
        let earliest = server_ms.saturating_add(delay_ms);
        self.earliest_spell_server_ms = self.earliest_spell_server_ms.max(earliest);
    }

    /// C++ `Use` two-object path — `cract.cc:765` `EarliestMultiuseTime = ServerMilliseconds + 1000`.
    pub fn delay_multiuse_ms(&mut self, server_ms: u64, delay_ms: u64) {
        let earliest = server_ms.saturating_add(delay_ms);
        self.earliest_multiuse_server_ms = self.earliest_multiuse_server_ms.max(earliest);
    }

    /// C++ `TDGo` / walk-step gate — `cract.cc:918–921`.
    #[inline]
    pub fn walk_action_ready_at(&self, server_ms: u64) -> bool {
        server_ms >= self.earliest_walk_server_ms
    }

    /// C++ spell cast gate — `magic.cc:3399` et al.
    #[inline]
    pub fn spell_ready_at(&self, server_ms: u64) -> bool {
        server_ms >= self.earliest_spell_server_ms
    }

    /// C++ `TDUse` two-object gate — `cract.cc:927–928`.
    #[inline]
    pub fn multiuse_ready_at(&self, server_ms: u64) -> bool {
        server_ms >= self.earliest_multiuse_server_ms
    }

    /// Latest blocking earliest-time among walk / spell / multiuse (for deferred walk-action reschedule).
    pub fn earliest_action_block_ms(&self, server_ms: u64) -> Option<u64> {
        [
            self.earliest_walk_server_ms,
            self.earliest_spell_server_ms,
            self.earliest_multiuse_server_ms,
        ]
        .into_iter()
        .filter(|&t| t > server_ms)
        .max()
    }
}
