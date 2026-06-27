//! Monster creature appear/move reactions and viewport fan-out.
//!
//! - `Monster::onCreatureMove` — `monster.cpp` (~212).
//! - `Monster::onCreatureAppear` — `monster.cpp` (~159–166).
//! - `TCreature::CreatureMoveStimulus` — `crmain.cc:888` (close-chase restep while `TDAttack` pending).
//! - `Map::getSpectators` move fan-out — `map.cpp` (~264–323, ~386–474).

use slotmap::Key;
use tfs_rust_common::Position;

use crate::chase_debug;
use crate::creature::{CreatureKind, MonsterChaseMode, MonsterState};
use crate::creature_todo::MONSTER_IDLE_WAIT_MS;
use crate::game_world::{creature_can_see, GameWorld};
use crate::ids::CreatureId;
use crate::monster_ai::{chebyshev, MonsterEnqueueAttackResult, MAP_MAX_VIEWPORT};

impl GameWorld {
    pub fn monster_on_creature_appear_self(&mut self, cid: CreatureId) {
        self.monster_update_target_list(cid);
        let keep_sleeping = self.creatures.get(cid).is_some_and(|k| {
            matches!(
                k,
                CreatureKind::Monster(m)
                    if m.harness_preserve_sleep
                        && m.state == MonsterState::Sleeping
                        && m.is_idle
            )
        });
        if !keep_sleeping {
            self.monster_update_idle_status(cid);
        }
        // 772: `TMonster::IdleStimulus` `Strategy[]` acquires targets (`crnonpl.cc:2468`).
        // TFS `searchTarget` on appear is 1098-only (`monster.cpp` ~159).
        if self.beat_driven_loop {
            self.request_idle_stimulus(cid);
        } else {
            self.monster_try_acquire_chase_target(cid, None);
        }
    }
    /// TFS `Map::getSpectators` multifloor Z span — `map.cpp` ~444–462.
    fn spectator_z_range(center_z: u8, multifloor: bool) -> std::ops::RangeInclusive<u8> {
        if !multifloor {
            return center_z..=center_z;
        }
        if center_z > 7 {
            let min_z = center_z.saturating_sub(2);
            let max_z = (center_z + 2).min(15);
            return min_z..=max_z;
        }
        if center_z == 6 {
            return 0..=8;
        }
        if center_z == 7 {
            return 0..=9;
        }
        0..=7
    }

    /// C++ `Map::getSpectators` — spatial viewport box only (`map.cpp` ~386–474).
    /// Used for move/appear fan-out; per-creature `canSee` is checked in `Monster::onCreatureMove`.
    fn collect_spatial_spectators(&self, center: Position, multifloor: bool) -> Vec<CreatureId> {
        let mut out = Vec::new();
        for z in Self::spectator_z_range(center.z, multifloor) {
            self.map.grid.collect_spectators(
                center.x,
                center.y,
                z,
                MAP_MAX_VIEWPORT,
                MAP_MAX_VIEWPORT,
                &mut out,
            );
        }
        out.sort_by_key(|id| id.data().as_ffi());
        out.dedup();
        out
    }

    /// Creatures within `Creature::canSee` of `center` (monster `updateTargetList` / spawn scan).
    pub(crate) fn collect_creature_spectators(
        &mut self,
        center: Position,
        multifloor: bool,
    ) -> Vec<CreatureId> {
        let range = i32::from(MAP_MAX_VIEWPORT);
        self.collect_spatial_spectators(center, multifloor)
            .into_iter()
            .filter(|&other| {
                let Some(other_pos) = self.creatures.get(other).map(|k| k.position()) else {
                    return false;
                };
                creature_can_see(center, other_pos, range, range)
            })
            .collect()
    }
    /// Monsters that should receive `Monster::onCreatureMove` for a move (`map.cpp` ~264–323).
    fn monsters_witnessing_move(
        &mut self,
        old_pos: Position,
        new_pos: Position,
    ) -> Vec<CreatureId> {
        let mut ids: Vec<CreatureId> = self
            .collect_spatial_spectators(old_pos, true)
            .into_iter()
            .chain(self.collect_spatial_spectators(new_pos, true))
            .filter(|&id| {
                self.creatures
                    .get(id)
                    .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
            })
            .collect();
        if self.harness_real_map {
            // Bowl dual-monster — match C++ drain / JSONL order (spawn 2 before spawn 1).
            ids.sort_by(|a, b| {
                let order = |id: CreatureId| {
                    self.creatures
                        .get(id)
                        .and_then(|k| match k {
                            CreatureKind::Monster(m) => Some(m.harness_spawn_order),
                            _ => None,
                        })
                        .unwrap_or(0)
                };
                order(*b).cmp(&order(*a))
            });
        } else {
            ids.sort_by_key(|id| id.data().as_ffi());
        }
        ids.dedup();
        ids
    }

    /// TFS `Monster::onCreatureMove` — `monster.cpp` ~212.
    pub fn monster_on_creature_move(
        &mut self,
        monster_id: CreatureId,
        creature_id: CreatureId,
        old_pos: Position,
        new_pos: Position,
    ) {
        if !self.creatures.contains_key(monster_id) {
            return;
        }

        if creature_id == monster_id {
            self.monster_sleep_wake_on_creature_move(monster_id, creature_id);
            self.monster_update_target_list(monster_id);
            self.monster_update_idle_status(monster_id);
            return;
        }

        if self.beat_driven_loop {
            self.monster_sleep_wake_on_creature_move(monster_id, creature_id);
        }

        let monster_pos = match self.creatures.get(monster_id) {
            Some(k) => k.position(),
            None => return,
        };
        let range = i32::from(MAP_MAX_VIEWPORT);
        let can_see_new = creature_can_see(monster_pos, new_pos, range, range);
        let can_see_old = creature_can_see(monster_pos, old_pos, range, range);

        if can_see_new && !can_see_old {
            self.monster_on_creature_found(monster_id, creature_id, true);
        } else if !can_see_new && can_see_old {
            self.monster_remove_creature_from_lists(monster_id, creature_id);
        }

        self.monster_update_idle_status(monster_id);

        let (is_summon, follow, has_path) = match self.creatures.get(monster_id) {
            Some(CreatureKind::Monster(m)) => (
                m.base.is_summon(),
                m.base.follow_target,
                m.base.has_follow_path,
            ),
            _ => return,
        };

        if follow == Some(creature_id) {
            if self.beat_driven_loop {
                if let (Some(CreatureKind::Monster(m)), Some(target_pos)) = (
                    self.creatures.get(monster_id),
                    self.creatures.get(creature_id).map(|k| k.position()),
                ) {
                    let cheb = chebyshev(m.base.position, target_pos);
                    chase_debug::log_creature_move_stimulus(
                        self.chase_trace_tick(),
                        monster_id,
                        m.base.name.as_str(),
                        creature_id.data().as_ffi(),
                        "move_stimulus",
                        cheb,
                    );
                }
            }
            self.monster_on_follow_creature_moved(monster_id, creature_id, new_pos, has_path);
            self.monster_combat_creature_move_stimulus(monster_id, creature_id);
            self.monster_harness_close_kite_restep_on_target_move(monster_id, creature_id);
            self.monster_close_chase_clear_pending_go_on_target_flee(monster_id, creature_id);
            let target_visible = self
                .creatures
                .get(creature_id)
                .map(|k| creature_can_see(monster_pos, k.position(), range, range))
                .unwrap_or(false);
            if new_pos.z != old_pos.z || !target_visible {
                if let Some(k) = self.creatures.get_mut(monster_id) {
                    if k.base().follow_target == Some(creature_id) {
                        k.base_mut().clear_follow_for_target(creature_id);
                    }
                    if k.base().attack_target == Some(creature_id) {
                        k.base_mut().clear_attack_for_target(creature_id);
                    }
                }
            }
            return;
        }

        // Dual harness co-chase — log when a sibling monster moves (`chase_path_cip_realmap.log` @400/2000).
        if self.harness_real_map
            && self.beat_driven_loop
            && creature_id != monster_id
        {
            let log_co_monster = self.creatures.get(creature_id).is_some_and(|k| {
                matches!(k, CreatureKind::Monster(m) if m.harness_spawn_order > 0)
            }) && self.creatures.get(monster_id).is_some_and(|k| {
                matches!(k, CreatureKind::Monster(m) if m.harness_spawn_order > 0)
            });
            if log_co_monster {
                if let (Some(CreatureKind::Monster(m)), Some(mover_pos)) = (
                    self.creatures.get(monster_id),
                    self.creatures.get(creature_id).map(|k| k.position()),
                ) {
                    let cheb = chebyshev(m.base.position, mover_pos);
                    chase_debug::log_creature_move_stimulus(
                        self.chase_trace_tick(),
                        monster_id,
                        m.base.name.as_str(),
                        creature_id.data().as_ffi(),
                        "move_stimulus",
                        cheb,
                    );
                }
            }
        }

        // TFS `Monster::onCreatureMove` — `monster.cpp` ~287–289: `selectTarget(creature)` only
        // when we have no follow (1098). 772 defers to idle `Strategy[]` (`crnonpl.cc:2468`).
        if !is_summon
            && can_see_new
            && self.monster_is_opponent(monster_id, creature_id)
            && follow.is_none()
        {
            self.monster_ensure_opponent_listed(monster_id, creature_id);
            if self.beat_driven_loop {
                self.monster_schedule_chase_after_opponent_add(monster_id, Some(creature_id));
            } else {
                self.monster_select_target(monster_id, creature_id);
            }
        }

        if self.beat_driven_loop
            && creature_id != monster_id
            && follow.is_some()
            && follow != Some(creature_id)
            && self.monster_chase_stalled_without_wakeup(monster_id)
        {
            self.request_idle_stimulus(monster_id);
        }
    }

    /// TFS `Creature::onCreatureMove` follow-target branch — `creature.cpp` ~619–637.
    ///
    /// 772: dist-chase (`target_distance > 1`) may re-arm idle on follow-target move (`dist_follow_move`).
    /// Close-chase target moves defer to idle segment drain and `CreatureMoveStimulus` (`crmain.cc:919-961`)
    /// — not `monster_chase_queue_stale` / empty-queue idle repath on every kite tile (lesson 37).
    fn monster_on_follow_creature_moved(
        &mut self,
        monster_id: CreatureId,
        creature_id: CreatureId,
        new_pos: Position,
        has_path: bool,
    ) {
        if !self
            .creatures
            .get(monster_id)
            .is_some_and(|k| k.base().follow_target.is_some())
        {
            return;
        }
        // 772 idle drain owns repath even without an in-flight path (P0-1 / freeze fix).
        if !has_path && !self.mechanics.profile.follow_repath_without_path && !self.beat_driven_loop
        {
            return;
        }
        if self
            .creatures
            .get(monster_id)
            .is_some_and(|k| k.base().is_updating_path)
        {
            return;
        }

        let should_repath = if self.beat_driven_loop {
            self.creatures.get(monster_id).is_some_and(|k| {
                let CreatureKind::Monster(m) = k else {
                    return false;
                };
                let target_distance = self.monster_effective_target_distance(m.target_distance);
                target_distance > 1
                    && m.base.follow_target == Some(creature_id)
                    && self.monster_can_use_attack(monster_id, m.base.position, creature_id)
            })
        } else {
            true
        };

        if !should_repath {
            return;
        }

        // C++ `CreatureMoveStimulus` close-chase clears in-flight attack todo before repath
        // (`crmain.cc:946-951` `ToDoClear` + re-queue) — only when a stale queue blocks yield.
        if let Some(k) = self.creatures.get_mut(monster_id) {
            let base = k.base_mut();
            if self.beat_driven_loop {
                if !base.todo.queue.is_empty() {
                    base.todo.queue.clear();
                    base.todo.locked = false;
                }
                // C++ `CreatureMoveStimulus` preempts goal `ToDoWait` — do not defer repath.
                base.next_wakeup = None;
            }
            base.walk_queue.clear();
            base.walk_update_ticks = 0;
            base.is_updating_path = false;
            base.force_update_follow_path = true;
        }
        if self.beat_driven_loop {
            self.monster_idle_stimulus(monster_id);
            if let (Some(CreatureKind::Monster(m)), Some(target_pos)) = (
                self.creatures.get(monster_id),
                self.creatures.get(creature_id).map(|k| k.position()),
            ) {
                let cheb = chebyshev(m.base.position, target_pos);
                chase_debug::log_creature_move_stimulus(
                    self.chase_trace_tick(),
                    monster_id,
                    m.base.name.as_str(),
                    creature_id.data().as_ffi(),
                    "dist_follow_move",
                    cheb,
                );
            }
        } else {
            self.monster_follow_repath_now(monster_id, Some("target_move"));
        }
    }

    /// Clear stale close-chase todos when the target left the adjacent band and arm chase inline.
    ///
    /// C++ `CreatureMoveStimulus` handles locked `TDAttack`; pending `ToDoGo` / goal `Wait` while
    /// the target kites away must not block restep (`crmain.cc:888-961`).
    fn monster_close_chase_clear_pending_go_on_target_flee(
        &mut self,
        monster_id: CreatureId,
        target_id: CreatureId,
    ) {
        if !self.beat_driven_loop {
            return;
        }
        let snapshot = self.creatures.get(monster_id).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            if m.is_fleeing() {
                return None;
            }
            if m.base.attack_target != Some(target_id) {
                return None;
            }
            if !matches!(m.state, MonsterState::Attacking | MonsterState::Panic) {
                return None;
            }
            let target_pos = self.creatures.get(target_id)?.position();
            let dist = chebyshev(m.base.position, target_pos);
            if dist <= 1 {
                return None;
            }
            let target_distance = self.monster_effective_target_distance(m.target_distance);
            if target_distance > 1 {
                return None;
            }
            Some((
                m.chase_mode,
                m.base.todo.has_attack(),
                m.base.todo.has_go(),
                m.base.todo.locked,
                target_pos,
            ))
        });
        let Some((chase_mode, has_attack, has_go, todo_locked, target_pos)) = snapshot else {
            return;
        };
        if chase_mode != MonsterChaseMode::Close {
            return;
        }
        // Locked `TDAttack` path — handled by [`Self::monster_combat_creature_move_stimulus`].
        if has_attack && todo_locked && !has_go {
            return;
        }
        // Mid-batch segment drain — C++ does not idle-repath on every kite tile.
        if self.monster_close_chase_batch_in_flight(monster_id) {
            return;
        }
        // Stale single `Go` with empty walk_queue — replace with fresh chase Go.
        if !has_go {
            return;
        }
        if !self.monster_chase_needs_attacking_close_repath(monster_id, target_pos) {
            return;
        }
        if let Some(k) = self.creatures.get_mut(monster_id) {
            let base = k.base_mut();
            base.todo.queue.clear();
            base.todo.locked = false;
            base.walk_queue.clear();
            base.has_follow_path = false;
            base.force_update_follow_path = true;
            base.next_wakeup = None;
        }
        self.monster_idle_stimulus(monster_id);
        if let (Some(CreatureKind::Monster(m)), Some(target_pos)) = (
            self.creatures.get(monster_id),
            self.creatures.get(target_id).map(|k| k.position()),
        ) {
            let cheb = chebyshev(m.base.position, target_pos);
            chase_debug::log_creature_move_stimulus(
                self.chase_trace_tick(),
                monster_id,
                m.base.name.as_str(),
                target_id.data().as_ffi(),
                "close_flee_clear",
                cheb,
            );
        }
    }

    /// C++ `TCreature::CreatureMoveStimulus` — `crmain.cc:888-920`.
    ///
    /// While close-chasing with a pending attack todo, target kiting away clears the queue and
    /// re-arms `ToDoAttack` (Wait 200 ms then CanToDoAttack walk + strike). Walk re-steps on
    /// every target move; strike cadence gates only `TDAttack`, not the close `ToDoGo`.
    fn monster_combat_creature_move_stimulus(
        &mut self,
        monster_id: CreatureId,
        target_id: CreatureId,
    ) {
        if !self.beat_driven_loop {
            return;
        }
        self.monster_idle_prepare_combat_chase(monster_id);
        let snapshot = self.creatures.get(monster_id).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            let attack_id = m.base.attack_target?;
            if attack_id != target_id {
                return None;
            }
            let target_pos = self.creatures.get(target_id)?.position();
            Some((
                m.chase_mode,
                m.state,
                m.base.position,
                target_pos,
                m.base.todo.has_attack(),
                m.base.todo.has_go(),
                m.base.todo.locked,
            ))
        });
        let Some((chase_mode, state, pos, target_pos, has_attack, has_go, todo_locked)) =
            snapshot
        else {
            return;
        };
        if chase_mode != MonsterChaseMode::Close {
            return;
        }
        if !has_attack || has_go {
            return;
        }
        // C++ `LockToDo` during `TDAttack` drain (`crmain.cc:930-933`); real-map harness
        // `player_walk` may arrive same tick after defer unlock — still re-arm when attacking.
        let harness_attacking_kite = self.harness_real_map
            && matches!(state, MonsterState::Attacking | MonsterState::Panic);
        if !todo_locked && !harness_attacking_kite {
            return;
        }
        // C++ `CreatureMoveStimulus` — only when strike is >200ms away (`crmain.cc:924`).
        if self
            .creatures
            .get(monster_id)
            .is_some_and(|k| k.base().earliest_attack_ms <= self.server_ms.saturating_add(200))
        {
            return;
        }
        if chebyshev(pos, target_pos) <= 1 {
            return;
        }
        if let Some(k) = self.creatures.get_mut(monster_id) {
            let base = k.base_mut();
            base.todo.queue.clear();
            base.walk_queue.clear();
            base.has_follow_path = false;
        }
        if !self.enqueue_creature_wait(monster_id, 200) {
            return;
        }
        chase_debug::log_creature_move_stimulus(
            self.chase_trace_tick(),
            monster_id,
            self.creatures
                .get(monster_id)
                .map(|k| k.base().name.as_str())
                .unwrap_or("?"),
            target_id.data().as_ffi(),
            "combat_move_rearm",
            chebyshev(pos, target_pos),
        );
        match self.monster_enqueue_todo_attack_actions(monster_id) {
            MonsterEnqueueAttackResult::Enqueued => {
                self.schedule_immediate_todo_wakeup(monster_id);
            }
            MonsterEnqueueAttackResult::Retry => {
                self.idle_enqueue_wait_and_start(monster_id, MONSTER_IDLE_WAIT_MS);
            }
            MonsterEnqueueAttackResult::Noway => {
                self.idle_stimulus(monster_id);
            }
            MonsterEnqueueAttackResult::Failed => {
                self.monster_combat_handle_close_chase_blocked(monster_id);
            }
        }
    }

    /// Real-map combat-under-kite — re-arm chase when player leaves cheb 1 during stand/phase C.
    ///
    /// C++ `CreatureMoveStimulus` @8400/9000 on cyclops bowl (`crmain.cc:920-961`); harness path
    /// when `TDAttack` is not mid-lock but strike is >200ms away and dist 2–4.
    fn monster_harness_close_kite_restep_on_target_move(
        &mut self,
        monster_id: CreatureId,
        target_id: CreatureId,
    ) {
        if !self.harness_real_map || !self.beat_driven_loop {
            return;
        }
        let snapshot = self.creatures.get(monster_id).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            if m.harness_spawn_order == 0 || m.is_fleeing() {
                return None;
            }
            if m.base.attack_target != Some(target_id) || m.target_distance > 1 {
                return None;
            }
            if !matches!(m.state, MonsterState::Attacking | MonsterState::Panic) {
                return None;
            }
            if m.chase_mode != MonsterChaseMode::Close {
                return None;
            }
            let target_pos = self.creatures.get(target_id)?.position();
            let cheb = chebyshev(m.base.position, target_pos);
            if cheb < 2 || cheb > 4 {
                return None;
            }
            if m.base.todo.has_go() || self.monster_close_chase_batch_in_flight(monster_id) {
                return None;
            }
            if m.base.earliest_attack_ms <= self.server_ms.saturating_add(200) {
                return None;
            }
            Some((cheb, m.base.todo.locked, m.base.todo.has_attack()))
        });
        let Some((cheb, todo_locked, has_attack)) = snapshot else {
            return;
        };
        if todo_locked && has_attack {
            return;
        }
        if let Some(k) = self.creatures.get_mut(monster_id) {
            let base = k.base_mut();
            base.todo.queue.clear();
            base.todo.locked = false;
            base.walk_queue.clear();
            base.has_follow_path = false;
            base.force_update_follow_path = true;
            base.next_wakeup = None;
        }
        if !self.enqueue_creature_wait(monster_id, 200) {
            return;
        }
        chase_debug::log_creature_move_stimulus(
            self.chase_trace_tick(),
            monster_id,
            self.creatures
                .get(monster_id)
                .map(|k| k.base().name.as_str())
                .unwrap_or("?"),
            target_id.data().as_ffi(),
            "combat_move_rearm",
            cheb,
        );
        match self.monster_enqueue_todo_attack_actions(monster_id) {
            MonsterEnqueueAttackResult::Enqueued => {
                self.schedule_immediate_todo_wakeup(monster_id);
            }
            MonsterEnqueueAttackResult::Retry => {
                self.idle_enqueue_wait_and_start(monster_id, MONSTER_IDLE_WAIT_MS);
            }
            MonsterEnqueueAttackResult::Noway => {
                self.monster_idle_stimulus(monster_id);
            }
            MonsterEnqueueAttackResult::Failed => {
                self.monster_combat_handle_close_chase_blocked(monster_id);
            }
        }
    }

    /// TFS `Monster::onFollowCreatureComplete` — `monster.cpp` ~599.
    pub(crate) fn monster_on_follow_creature_complete(
        &mut self,
        cid: CreatureId,
        target_id: CreatureId,
    ) {
        let (has_path, is_summon) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (m.base.has_follow_path, m.base.is_summon()),
            _ => return,
        };
        let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) else {
            return;
        };
        let idx = m.opponent_ids.iter().position(|&id| id == target_id);
        let Some(idx) = idx else {
            return;
        };
        m.opponent_ids.remove(idx);
        if has_path {
            m.opponent_ids.insert(0, target_id);
        } else if !is_summon {
            m.opponent_ids.push(target_id);
        }
    }
    /// TFS `Monster::onCreatureEnter` via `onCreatureAppear` spectator fan-out — `monster.cpp` ~435.
    pub fn monster_notify_creature_enter_viewport(
        &mut self,
        creature_id: CreatureId,
        pos: Position,
    ) {
        let monsters: Vec<CreatureId> = self
            .collect_spatial_spectators(pos, true)
            .into_iter()
            .filter(|&id| {
                id != creature_id
                    && self
                        .creatures
                        .get(id)
                        .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
            })
            .collect();
        self.monster_viewport_notify_depth += 1;
        for monster_id in monsters {
            // C++ `Monster::onCreatureAppear` → `onCreatureEnter` for each spatial spectator (`monster.cpp` ~167).
            self.monster_on_creature_found(monster_id, creature_id, true);
        }
        self.monster_viewport_notify_depth = self.monster_viewport_notify_depth.saturating_sub(1);
    }

    /// Notify monsters near a creature move (`Map::moveCreature` spectator fan-out).
    pub fn monster_dispatch_creature_move(
        &mut self,
        moved: CreatureId,
        old_pos: Position,
        new_pos: Position,
    ) {
        let monsters = self.monsters_witnessing_move(old_pos, new_pos);
        for monster_id in monsters {
            self.monster_on_creature_move(monster_id, moved, old_pos, new_pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use tfs_rust_common::enums::Direction;
    use tfs_rust_common::Position;

    use crate::creature::{CreatureKind, MonsterChaseMode, MonsterState};
    use crate::creature_todo::CreatureAction;
    use crate::test_world::support::{
        beat_driven_test_world, ensure_walkable_tile, insert_monster, insert_player,
        test_player, TEST_SYNTHETIC_GROUND_WP,
    };

    #[test]
    fn test_772_close_flee_clear_skips_inflight_go() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        let ppos_new = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos_new, TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Cyclops", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.walk_queue.push_back(Direction::East);
            m.base.walk_queue.push_back(Direction::East);
            m.base.todo.queue.push_back(CreatureAction::Go);
            m.base.todo.queue.push_back(CreatureAction::Attack);
        }

        let queue_len_before = world.creatures.get(monster).unwrap().base().walk_queue.len();
        let todo_go_before = world.creatures.get(monster).unwrap().base().todo.has_go();

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_new;
        }
        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_new, player);
        world.monster_dispatch_creature_move(player, ppos, ppos_new);

        let queue_len_after = world.creatures.get(monster).unwrap().base().walk_queue.len();
        let todo_go_after = world.creatures.get(monster).unwrap().base().todo.has_go();

        assert_eq!(queue_len_before, 2);
        assert_eq!(queue_len_after, 2, "in-flight walk_queue must not be cleared");
        assert!(todo_go_before);
        assert!(todo_go_after, "in-flight ToDoGo must not be cleared");
    }

    #[test]
    fn test_772_close_flee_clear_skips_mid_batch_between_go_exec() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        let ppos_new = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos_new, TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Cyclops", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            // Post-Go-execute inter-step gap: Go popped, Attack still queued, segment wakeup pending.
            m.base.todo.queue.push_back(CreatureAction::Attack);
            m.base.todo.locked = true;
            m.base.next_wakeup = Some(world.server_ms + 200);
        }

        let attack_queued_before = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .todo
            .has_attack();

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_new;
        }
        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_new, player);
        world.monster_dispatch_creature_move(player, ppos, ppos_new);

        let base = world.creatures.get(monster).unwrap().base();
        assert!(attack_queued_before);
        assert!(
            base.todo.has_attack(),
            "mid-batch segment drain must not idle-repath on target kite step"
        );
        assert!(
            base.todo.locked,
            "todo lock must survive target move during batch drain"
        );
    }

    #[test]
    fn test_772_opponent_move_defers_select_target() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        let ppos_new = Position::new(103, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos_new, TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = true;
            m.opponent_ids.push(player);
        }

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_new;
        }
        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_new, player);
        world.monster_dispatch_creature_move(player, ppos, ppos_new);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().follow_target.is_none()),
            "772 must not sync selectTarget on opponent move"
        );
        assert!(
            world.creatures.get(monster).is_some_and(|k| {
                k.base().next_wakeup.is_some() || !k.base().todo.is_empty()
            }),
            "772 must defer target via idle yield, not sync select"
        );
    }
}
