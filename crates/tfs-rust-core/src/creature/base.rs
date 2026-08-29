//! Shared creature fields (all creature types).
// C++ reference: `Creature` (`creature.h`).

use std::collections::VecDeque;
use std::time::Instant;

use crate::condition::ActiveCondition;
use crate::creature_todo::CreatureTodo;
use crate::ids::CreatureId;
use tfs_rust_common::Position;
use tfs_rust_common::enums::{Direction, SkullType};

/// C++ `TCombat::ChaseMode` — `crcombat.cc:338`; 1098 ignores (era gating via profile).
///
/// Lives on [`CreatureBase`] so both players and monsters share the unified ToDo/`CanToDoAttack`
/// chase path (Phase 0 walk-engine unification). 772 players use `Close` when following.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChaseMode {
    /// `CHASE_MODE_NONE` — idle dist arms or pre-melee reset (`crnonpl.cc:2711`).
    #[default]
    None,
    /// `CHASE_MODE_CLOSE` — `CanToDoAttack` close walk (`crcombat.cc:496`).
    Close,
    /// `CHASE_MODE_RANGE` — E4 wires keep-distance combat walk (`crcombat.cc:500`).
    Range,
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
///
/// 772 stores a fixed-capacity ring with timestamps (`TCombat::CombatList[20]`,
/// `crcombat.cc:862-906`); the HashMap alias is gone.
pub type DamageMap = CombatList;

/// One slot in the 772 combat list (`TCombatEntry` — `cr.hh:362-366`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CombatListEntry {
    pub id: Option<CreatureId>,
    pub damage: u64,
    pub timestamp_round: u32,
}

/// Fixed-capacity combat damage ring — 772 `TCombat::CombatList` + `ActCombatEntry`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatList {
    entries: Vec<CombatListEntry>,
    /// Next write index when appending a new attacker (`ActCombatEntry`).
    act_entry: usize,
    /// Running total of recorded damage (`TCombat::CombatDamage`).
    pub combat_damage: u64,
}

impl Default for CombatList {
    fn default() -> Self {
        Self::with_slots(CombatList::DEFAULT_SLOTS)
    }
}

impl CombatList {
    /// 772 `NARRAY(CombatList)` = 20 (`cr.hh:417`).
    pub const DEFAULT_SLOTS: usize = 20;

    pub fn with_slots(n: usize) -> Self {
        let n = n.max(1);
        Self {
            entries: vec![CombatListEntry::default(); n],
            act_entry: 0,
            combat_damage: 0,
        }
    }

    pub fn clear(&mut self) {
        for e in &mut self.entries {
            *e = CombatListEntry::default();
        }
        self.act_entry = 0;
        self.combat_damage = 0;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        !self.entries.iter().any(|e| e.id.is_some())
    }

    /// 772 `AddDamageToCombatList` (`crcombat.cc:862-878`).
    pub fn add(&mut self, attacker: CreatureId, damage: u64, round_nr: u32) {
        if damage == 0 {
            return;
        }
        self.combat_damage = self.combat_damage.saturating_add(damage);
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == Some(attacker)) {
            e.damage = e.damage.saturating_add(damage);
            e.timestamp_round = round_nr;
            return;
        }
        let idx = self.act_entry % self.entries.len();
        self.entries[idx] = CombatListEntry {
            id: Some(attacker),
            damage,
            timestamp_round: round_nr,
        };
        self.act_entry = (idx + 1) % self.entries.len();
    }

    /// 772 `GetDamageByCreature` (`crcombat.cc:884-892`).
    pub fn damage_by(&self, id: CreatureId) -> u64 {
        self.entries
            .iter()
            .find(|e| e.id == Some(id))
            .map(|e| e.damage)
            .unwrap_or(0)
    }

    /// 772 `GetMostDangerousAttacker` (`crcombat.cc:895-906`):
    /// `(RoundNr - TimeStamp) < window`, strictly greater damage, ties keep lowest index.
    pub fn most_dangerous(&self, round_nr: u32, window: u32) -> Option<CreatureId> {
        let mut best_id = None;
        let mut best_dmg = 0u64;
        for e in &self.entries {
            let Some(id) = e.id else { continue };
            if round_nr.wrapping_sub(e.timestamp_round) >= window {
                continue;
            }
            if e.damage > best_dmg {
                best_dmg = e.damage;
                best_id = Some(id);
            }
        }
        best_id
    }

    /// Live (id-present) entries — for exp share (772 does **not** apply the window filter).
    pub fn iter_active(&self) -> impl Iterator<Item = (CreatureId, u64, u32)> + '_ {
        self.entries
            .iter()
            .filter_map(|e| e.id.map(|id| (id, e.damage, e.timestamp_round)))
    }

    /// Compatibility helper used by tests that previously `insert`ed into a HashMap.
    pub fn insert(&mut self, id: CreatureId, damage: u64) {
        self.add(id, damage, 0);
    }

    pub fn get(&self, id: &CreatureId) -> Option<&u64> {
        // Lifetime workaround: return via damage_by pattern — callers use `.copied()`.
        // Prefer `damage_by`. Kept for HashMap-like call sites during migration.
        self.entries
            .iter()
            .find(|e| e.id.as_ref() == Some(id))
            .map(|e| &e.damage)
    }

    pub fn keys(&self) -> impl Iterator<Item = CreatureId> + '_ {
        self.entries.iter().filter_map(|e| e.id)
    }
}

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
    /// TFS `Creature::varSpeed` — equipment / `changeSpeed` delta (`creature.h`).
    /// Effective walk speed is `base_speed + var_speed` + active `ConditionData::Speed`
    /// (see `walk_timing::creature_effective_speed_for_step`). `speed` tracks vocation GoStrength.
    pub var_speed: i32,
    pub skull: SkullType,
    /// TFS `Creature::drunkenness` — set by `ConditionDrunk` (`condition.cpp` / `creature.h`).
    pub drunkenness: u32,
    /// Active conditions (merged per TFS `addCondition` rules).
    pub active_conditions: Vec<ActiveCondition>,
    /// TFS `Creature::listWalkDir` — consumed from the **back** in `getNextStep` (`creature.cpp`).
    pub walk_queue: VecDeque<Direction>,
    /// 772 absolute-destination overlay — parallel to `walk_queue`, stores the absolute
    /// `Position` each queued step lands on. C++ `TDGo` stores absolute coordinates
    /// (`receiving.cc:141-160`); Rust stores `Direction`s. This overlay lets `on_walk`
    /// verify adjacency after a mid-walk push (`cract.cc:386-389` `Distance > 1 → NOTACCESSIBLE`,
    /// audit #4). Populated only on the player 772 beat-driven path; always empty for
    /// monsters/NPCs and on the 1098 TFS path.
    pub walk_destinations: VecDeque<Position>,
    /// TFS `Creature::lastStep` (`OTSYS_TIME()` ms). We store `Instant` for deltas (`creature.cpp` `onCreatureMove`).
    pub last_step: Option<Instant>,
    /// TFS `Creature::lastStepCost` — 1 normal, 2 floor change, 3 diagonal (`creature.cpp` ~490–498).
    pub last_step_cost: u32,
    /// Ground speed of the **destination** tile for the step that ended at `last_step` (tile entered).
    /// OTClient v8 `Creature::getStepDuration` uses `m_lastStepToPosition` = step **destination**
    /// (`tasks/OTClientv8movement.md`); TFS `Creature::getWalkDelay` also uses **current** tile after move.
    pub last_step_ground_speed: u32,
    /// 772 `NextWakeup` — logical `ServerMilliseconds` deadline (`cract.cc:968`).
    /// Phase 5: the 1098 `Instant`-based `next_walk_check` + `walk_timer` are deleted; both
    /// eras schedule steps via the ToDoQueue on this logical deadline.
    pub next_wakeup: Option<u64>,
    /// 772 step anchor in logical time (paired with `last_step`).
    pub last_step_server_ms: Option<u64>,
    /// 772 `EarliestWalkTime` — earliest logical ms the next `TDGo` may run (`cr.hh:631`, `cract.cc:912–916`).
    pub earliest_walk_server_ms: u64,
    /// 772 `EarliestSpellTime` — spell exhaustion gate (`cr.hh:629`, `magic.cc:770–772` `CheckMana`).
    pub earliest_spell_server_ms: u64,
    /// 772 `EarliestMultiuseTime` — two-object use gate (`cr.hh:630`, `cract.cc:765`, `927–928`).
    pub earliest_multiuse_server_ms: u64,
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
    /// Last creature that dealt HP damage — 772 `Attacker` on killing blow (`crmain.cc:822`).
    pub last_hit_by: Option<CreatureId>,
    /// 772 `PoisonDamageOrigin` — credited on poison Event ticks (`crmain.cc:587`, `crskill.cc`).
    pub poison_damage_origin: Option<CreatureId>,
    /// 772 `FireDamageOrigin` — credited on burning Event ticks (`crmain.cc:599`).
    pub fire_damage_origin: Option<CreatureId>,
    /// 772 `EnergyDamageOrigin` — credited on energy Event ticks (`crmain.cc:609`).
    pub energy_damage_origin: Option<CreatureId>,
    /// C++ `TCombat::EarliestAttackTime` — `crcombat.cc:523` `DelayAttack`.
    pub earliest_attack_ms: u64,
    /// C++ `TCombat::LatestAttackTime` — `crcombat.cc:513-522` delayed `StopAttack`.
    ///
    /// `0` = inactive. Non-zero is a `RoundNr` deadline: when `RoundNr` exceeds it,
    /// `Attack()` calls `StopAttack(0)` and returns without striking (`crcombat.cc:551-553`).
    /// Cleared on `SetAttackDest` with `!Follow` (`crcombat.cc:438`). Units are
    /// [`GameWorld::round_nr`](crate::game_world::GameWorld::round_nr), not `server_ms`.
    pub latest_attack_round: u32,
    /// C++ `TCombat::EarliestDefendTime` — `crcombat.cc:236` `GetDefendDamage` gate.
    pub earliest_defend_ms: u64,
    /// C++ `TCombat::LastDefendTime` — paired with `EarliestDefendTime` (`crcombat.cc:241-242`).
    pub last_defend_ms: u64,
    /// C++ `TCombat::LearningPoints` — 772 skill-learning window (`crcombat.cc:526`
    /// `ActivateLearning` sets 30; `crskill.cc:549` `Probe`/`ProbeValue` decrement + call
    /// `Increase(1)` while > 0). PC-2 wires the field + `ActivateLearning` on a damaging
    /// strike; the `Increase(1)` skill-exp side-effect (per-skill tries counters +
    /// `req_skill_tries` leveling) is PC-5 scope (§0.5).
    pub learning_points: i32,
    /// 772 per-creature ToDo action list (772 idle-driven AI).
    pub todo: CreatureTodo,
    /// 772 combat chase mode — `TCombat::ChaseMode` (`crcombat.cc:338`); 1098 ignores.
    /// Shared by players and monsters on the unified ToDo/`CanToDoAttack` path.
    pub chase_mode: ChaseMode,
    /// OTClient auto-walk workaround: server_ms when the last `CGoPath` was armed.
    /// OTClient sends `0x69` (StopAutoWalk) 2–200 ms after each `0x64` (AutoWalk) on map-click;
    /// the stop is meant for the *previous* walk, not the fresh one. If a `StopAutoWalk`
    /// arrives within 400 ms of a freshly-armed walk, ignore it.
    pub last_auto_walk_armed_ms: u64,
    /// TFS `Creature::lootDrop` — `creature.h` `setDropLoot`.
    pub drop_loot: bool,
    /// TFS `Creature::skillLoss` — `creature.h` `setSkillLoss`.
    pub skill_loss: bool,
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
    /// Phase 5: both eras use `next_wakeup` + [`ToDoQueue`](crate::todo_queue::ToDoQueue); the
    /// 1098 `next_walk_check` + Tokio timer path is deleted.
    pub fn walk_timer_idle(&self) -> bool {
        self.next_wakeup.is_none()
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

    /// C++ `TCombat::ActivateLearning` — `crcombat.cc:526`: `LearningPoints = 30`.
    /// Called by `CloseAttack` when `DamageDone > 0` (`crcombat.cc:655`).
    pub fn activate_learning(&mut self) {
        self.learning_points = 30;
    }

    /// C++ `TCreature::IsInvisible()` — true while an `Invisible` condition is active
    /// (`crnonpl.cc:2221`, `crnonpl.cc:2429`). Checked by `MovePossible` and `IdleStimulus`
    /// lose-target against the mover's `RaceData[Race].SeeInvisible` flag.
    pub fn is_invisible(&self) -> bool {
        self.active_conditions
            .iter()
            .any(|c| c.ctype == tfs_rust_common::enums::ConditionType::Invisible)
    }
}

#[cfg(test)]
mod combat_list_tests {
    use super::*;
    use slotmap::SlotMap;

    fn cid(n: u32) -> CreatureId {
        let mut sm: SlotMap<CreatureId, ()> = SlotMap::with_key();
        let mut last = None;
        for _ in 0..=n {
            last = Some(sm.insert(()));
        }
        last.expect("cid")
    }

    #[test]
    fn combat_list_evicts_oldest_when_full() {
        let mut list = CombatList::with_slots(3);
        let a = cid(0);
        let b = cid(1);
        let c = cid(2);
        let d = cid(3);
        list.add(a, 10, 1);
        list.add(b, 20, 2);
        list.add(c, 30, 3);
        // 4th entry wraps and displaces slot 0 (oldest write cursor).
        list.add(d, 40, 4);
        assert_eq!(list.damage_by(a), 0, "oldest slot evicted");
        assert_eq!(list.damage_by(d), 40);
        assert_eq!(list.damage_by(b), 20);
        assert_eq!(list.damage_by(c), 30);
    }

    #[test]
    fn most_dangerous_attacker_ignores_stale_entries() {
        let mut list = CombatList::with_slots(5);
        let stale = cid(0);
        let fresh = cid(1);
        list.add(stale, 1000, 1);
        list.add(fresh, 10, 100);
        assert_eq!(list.most_dangerous(100, 60), Some(fresh));
        assert_eq!(list.most_dangerous(50, 60), Some(stale));
    }

    #[test]
    fn combat_list_records_clamped_damage_via_add() {
        let mut list = CombatList::default();
        let a = cid(0);
        list.add(a, 50, 1);
        list.add(a, 25, 2);
        assert_eq!(list.damage_by(a), 75);
        assert_eq!(list.combat_damage, 75);
        // Re-add updates timestamp.
        assert_eq!(
            list.iter_active()
                .find(|(id, _, _)| *id == a)
                .map(|(_, _, ts)| ts),
            Some(2)
        );
    }
}
