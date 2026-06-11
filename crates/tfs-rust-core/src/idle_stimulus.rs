//! 772 drain-triggered idle AI — `IdleStimulus` on ToDo queue drain.
//!
//! - `TCreature::IdleStimulus` — virtual dispatch after `Execute` drains the action list.
//! - `TMonster::IdleStimulus` — `crnonpl.cc:2386`.
//!
//! Profile-gated via `GameWorld::beat_driven_loop` (same flag as P2 ToDo walk).

use std::time::Instant;

use crate::chase_debug;
use crate::creature::CreatureKind;
use crate::creature_think::EVENT_CREATURE_THINK_INTERVAL_MS;
use crate::creature_todo::{trace_creature_todo, CreatureAction};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::monster_ai::MonsterIdleChaseRepathOutcome;
use crate::monster_targets::TargetSearchType;

impl GameWorld {
    /// 772 `TCreature::IdleStimulus` — dispatch on creature kind.
    pub(crate) fn idle_stimulus(&mut self, cid: CreatureId) {
        if !self.beat_driven_loop {
            return;
        }
        if !self.creatures.contains_key(cid) {
            return;
        }
        if self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().todo.locked)
        {
            return;
        }
        match self.creatures.get(cid) {
            Some(CreatureKind::Monster(_)) => {
                trace_creature_todo(self, cid, "idle_stimulus_enter");
                self.monster_idle_stimulus(cid);
                trace_creature_todo(self, cid, "idle_stimulus_exit");
            }
            _ => {}
        }
    }

    /// Request idle when the action queue is drained — sync or deferred to next wakeup.
    pub(crate) fn request_idle_stimulus(&mut self, cid: CreatureId) {
        if !self.beat_driven_loop {
            return;
        }
        if !self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
        {
            return;
        }
        if !self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().walk_timer_idle(self.beat_driven_loop))
        {
            return;
        }
        if !self.creature_todo_queue_empty(cid) {
            return;
        }
        trace_creature_todo(self, cid, "request_idle_stimulus");
        self.idle_stimulus(cid);
    }

    /// 772 `TMonster::IdleStimulus` — chase/repath/roam decisions (772 only).
    pub(crate) fn monster_idle_stimulus(&mut self, cid: CreatureId) {
        if !self.creatures.contains_key(cid) {
            return;
        }
        if self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if m.wants_lua_think()))
        {
            return;
        }

        let (is_idle, is_summon, has_opponents, follow, fleeing, pos) = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
                return;
            };
            (
                m.is_idle,
                m.base.is_summon(),
                !m.opponent_ids.is_empty(),
                m.base.follow_target,
                m.is_fleeing(),
                m.base.position,
            )
        };

        if is_idle {
            return;
        }

        if is_summon {
            self.monster_think_summon_stub(cid);
        } else if has_opponents {
            if follow.is_none() {
                let _ = self.monster_search_target(cid, TargetSearchType::Default);
            }
            // 772 chase repath: segment drain + target-move queue hysteresis only — not TFS
            // `monster_ensure_follow_band` (1098 think / walk-complete guard).
            if fleeing {
                let attack = self
                    .creatures
                    .get(cid)
                    .and_then(|k| k.base().attack_target);
                if let Some(target_id) = attack {
                    if !self.monster_can_use_attack(cid, pos, target_id) {
                        let _ = self.monster_search_target(cid, TargetSearchType::AttackRange);
                    }
                }
            }
        }

        self.monster_on_think_target(cid, EVENT_CREATURE_THINK_INTERVAL_MS);
        // 1098: `onThink` drives `updateLookDirection` once per tick.
        // 772: avoid force-facing while an active chase batch is running; let walk direction
        // carry facing, and only snap toward target when not chasing.
        if !self.beat_driven_loop {
            self.monster_update_look_direction(cid);
        } else if self
            .creatures
            .get(cid)
            .is_some_and(|k| {
                let base = k.base();
                base.attack_target.is_some()
                    && base.walk_queue.is_empty()
                    && base.todo.is_empty()
                    && base.follow_target.is_none()
            })
        {
            self.monster_update_look_direction(cid);
        }

        if !self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().health > 0 && k.base().walk_timer_idle(self.beat_driven_loop))
        {
            return;
        }

        self.monster_idle_prepare_and_enqueue_go(cid);
    }

    /// When idle should run [`GameWorld::monster_idle_chase_repath`] for an active chase (772 only).
    pub(crate) fn monster_idle_chase_needs_repath(
        &mut self,
        cid: CreatureId,
    ) -> (bool, Option<&'static str>) {
        let Some(k) = self.creatures.get(cid) else {
            return (false, None);
        };
        let base = k.base();
        if base.force_update_follow_path {
            if let Some(follow_id) = base.follow_target {
                let pos = k.position();
                if let Some(target_pos) = self.creatures.get(follow_id).map(|t| t.position()) {
                    let (fleeing, target_distance) = match self.creatures.get(cid) {
                        Some(CreatureKind::Monster(m)) => (
                            m.is_fleeing(),
                            self.monster_effective_target_distance(m.target_distance),
                        ),
                        _ => return (true, Some("force_update")),
                    };
                    if self.monster_at_follow_goal(
                        cid,
                        follow_id,
                        pos,
                        target_pos,
                        fleeing,
                        target_distance,
                    ) {
                        if let Some(k) = self.creatures.get_mut(cid) {
                            k.base_mut().force_update_follow_path = false;
                        }
                        return (false, None);
                    }
                }
            }
            return (true, Some("force_update"));
        }
        if !base.walk_queue.is_empty() {
            return (false, None);
        }
        if !base.has_follow_path {
            return (true, Some("idle_drain"));
        }
        let Some(follow_id) = base.follow_target else {
            return (false, None);
        };
        let pos = k.position();
        let Some(target_pos) = self.creatures.get(follow_id).map(|t| t.position()) else {
            return (false, None);
        };
        let (fleeing, target_distance) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                m.is_fleeing(),
                self.monster_effective_target_distance(m.target_distance),
            ),
            _ => return (false, None),
        };
        if self.monster_at_follow_goal(cid, follow_id, pos, target_pos, fleeing, target_distance) {
            return (false, None);
        }
        (true, Some("off_band"))
    }

    /// Fill walk queue from follow/repath or roam intent, then enqueue `Go` + heap arm.
    fn monster_idle_prepare_and_enqueue_go(&mut self, cid: CreatureId) {
        let (chasing, pos, name) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                m.base.follow_target.is_some(),
                m.base.position,
                m.base.name.clone(),
            ),
            _ => return,
        };

        let mut todo_via: Option<&str> = None;
        if chasing {
            let (needs_repath, repath_reason) = self.monster_idle_chase_needs_repath(cid);
            if needs_repath {
                match self.monster_idle_chase_repath(cid, repath_reason) {
                    MonsterIdleChaseRepathOutcome::PathQueued => {
                        todo_via = repath_reason;
                    }
                    MonsterIdleChaseRepathOutcome::AtGoal => {}
                    MonsterIdleChaseRepathOutcome::Noway => {
                        self.monster_on_chase_noway_772(cid);
                        if chase_debug::chase_path_debug_enabled() {
                            chase_debug::log_branch(
                                self.tick_counter,
                                cid,
                                name.as_str(),
                                "roam",
                                pos,
                                pos,
                                false,
                                1,
                                None,
                            );
                        }
                        todo_via = Some("roam");
                    }
                }
            }
        }

        let roaming = self.creatures.get(cid).is_some_and(|k| {
            matches!(
                k,
                CreatureKind::Monster(m) if !m.is_idle && m.base.follow_target.is_none()
            )
        });
        if roaming && todo_via.is_none() {
            if chase_debug::chase_path_debug_enabled() {
                chase_debug::log_branch(
                    self.tick_counter,
                    cid,
                    name.as_str(),
                    "roam",
                    pos,
                    pos,
                    false,
                    1,
                    None,
                );
            }
            todo_via = Some("roam");
        }

        let should_enqueue = self.creatures.get(cid).is_some_and(|k| {
            !k.base().walk_queue.is_empty()
                || self.monster_should_keep_dance_walk_alive(cid)
                || roaming
        });

        if should_enqueue {
            trace_creature_todo(self, cid, "idle_enqueue_go");
            self.idle_enqueue_go_and_start(cid, true, todo_via);
        }
    }

    /// Execute one `CreatureAction::Go` for 772 monsters — returns true if an action ran.
    pub(crate) fn execute_creature_todo_go(&mut self, cid: CreatureId) -> bool {
        let action = {
            let Some(k) = self.creatures.get_mut(cid) else {
                return false;
            };
            if k.base().todo.locked {
                return false;
            }
            k.base_mut().todo.queue.pop_front()
        };
        let Some(CreatureAction::Go) = action else {
            return false;
        };

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().todo.locked = true;
        }

        trace_creature_todo(self, cid, "execute_go");
        let now = Instant::now();
        self.on_walk(cid, false, now, None);

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().todo.locked = false;
        }

        trace_creature_todo(self, cid, "execute_go_done");
        true
    }

    /// After Go execute: schedule next Go or run idle when the action queue drains.
    pub(crate) fn finish_creature_todo_execute(&mut self, cid: CreatureId) {
        if !self.creature_uses_todo_execute(cid) {
            return;
        }

        let walk_queue_has_more = self
            .creatures
            .get(cid)
            .is_some_and(|k| !k.base().walk_queue.is_empty());

        if walk_queue_has_more {
            if self.todo_start_go_delay(cid, false) {
                self.schedule_immediate_todo_wakeup(cid);
            }
            return;
        }

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().has_follow_path = false;
        }

        if self.creature_todo_queue_empty(cid) {
            self.idle_stimulus(cid);
        }
    }

    /// Run one queued Go action (772 monsters).
    pub(crate) fn run_monster_todo_execute(&mut self, cid: CreatureId) {
        if self.execute_creature_todo_go(cid) {
            self.finish_creature_todo_execute(cid);
        }
    }
}

#[cfg(test)]
mod tests {
    use tfs_rust_common::Position;

    use crate::creature::CreatureKind;
    use crate::creature_think::EVENT_CREATURE_THINK_INTERVAL_MS;
    use crate::test_world::support::{
        ensure_walkable_tile, insert_monster, insert_player, minimal_world,
        test_player,
    };

    fn beat_driven_test_world() -> crate::game_world::GameWorld {
        let mut world = minimal_world();
        world.mechanics =
            crate::formulas::Mechanics::for_version(tfs_rust_common::ProtocolVersion::V772);
        world.beat_driven_loop = true;
        world.walk_wake_tx = None;
        world.server_ms = 0;
        world
    }

    /// Phase A — idle enqueues Go on drain; think no longer arms walk on 772.
    #[test]
    fn idle_stimulus_enqueues_go_for_active_monster() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        for x in 101..=104 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 2148);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.attack_target = Some(player);
        }
        world.add_creature_think_check(monster);
        assert!(
            world.monster_set_follow_creature(monster, Some(player)),
            "set_follow must succeed in view"
        );

        let has_go = world
            .creatures
            .get(monster)
            .is_some_and(|k| k.base().todo.has_go());
        let armed = world
            .creatures
            .get(monster)
            .and_then(|k| k.base().next_wakeup)
            .is_some();
        assert!(
            has_go || armed,
            "772 set_follow must enqueue Go or schedule wakeup via idle"
        );

        if world
            .creatures
            .get(monster)
            .is_some_and(|k| k.base().todo.has_go())
        {
            world.execute_creature_todo_go(monster);
        }

        world.monster_native_on_think(monster, EVENT_CREATURE_THINK_INTERVAL_MS);
        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go()),
            "772 think must not enqueue Go actions"
        );
    }

    /// Phase A — duplicate Go / heap entries suppressed when wakeup already armed.
    #[test]
    fn idle_go_enqueue_respects_wakeup_gate() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 2148);
        let monster = insert_monster(&mut world, "Rat", pos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }

        assert!(world.enqueue_creature_go(monster));
        world.todo_start_from_action(monster, 500);
        let wakeup = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .next_wakeup
            .expect("wakeup armed");
        let heap_len = world.todo_queue.len();

        assert!(!world.enqueue_creature_go(monster), "duplicate Go rejected");
        world.request_idle_stimulus(monster);

        assert_eq!(
            world.creatures.get(monster).unwrap().base().next_wakeup,
            Some(wakeup)
        );
        assert_eq!(world.todo_queue.len(), heap_len);
    }

    /// Phase A — process_creature_todo runs idle when action queue empty on wakeup.
    #[test]
    fn process_creature_todo_runs_idle_on_empty_queue() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(108, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        for x in 101..=108 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 2148);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 220);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }
        world.add_creature_think_check(monster);

        world.schedule_creature_wakeup(monster, 0);
        world.process_creature_todo(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go() || k.base().next_wakeup.is_some()),
            "drain with empty queue must idle-enqueue chase Go"
        );
    }

    /// Phase A — segment drain clears `has_follow_path` so idle repaths on next wakeup.
    #[test]
    fn idle_repaths_after_segment_drain_clears_follow_path() {
        let mut world = beat_driven_test_world();
        world.mechanics.profile.follow_repath_without_path = true;

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(108, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        for x in 101..=108 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 2148);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 220);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = true;
            m.base.walk_queue.clear();
        }

        world.finish_creature_todo_execute(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| !k.base().walk_queue.is_empty() || k.base().todo.has_go()),
            "772 finish must idle-repath after segment drain (has_follow_path cleared)"
        );
    }

    /// 772 active monster without follow enqueues roam Go from idle (TFS `getRandomStep` arm).
    #[test]
    fn idle_stimulus_enqueues_roam_for_active_monster_without_follow() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new((100 + dx) as u16, (100 + dy) as u16, 7),
                    2148,
                );
            }
        }

        let monster = insert_monster(&mut world, "Wolf", pos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go() || k.base().next_wakeup.is_some()),
            "772 idle must enqueue roam Go for active monster without follow"
        );
    }

    /// Blocked dance / stand-still at melee goal must not force a chase repath on next idle.
    #[test]
    fn force_update_at_follow_goal_skips_idle_repath() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = true;
            m.base.force_update_follow_path = true;
            m.base.walk_queue.clear();
        }

        let (needs, reason) = world.monster_idle_chase_needs_repath(monster);
        assert!(!needs, "at-goal force_update must not schedule repath");
        assert!(reason.is_none());
        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().force_update_follow_path),
            "stale force_update must be cleared at follow goal"
        );
    }

    /// 1098 regression — think still arms walk when not beat-driven.
    #[test]
    fn think_arm_still_runs_on_1098() {
        let mut world = minimal_world();
        assert!(!world.beat_driven_loop);

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);
        for x in 101..=104 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 2148);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.attack_target = Some(player);
        }
        world.add_creature_think_check(monster);
        assert!(world.monster_set_follow_creature(monster, Some(player)));

        world.monster_native_on_think(monster, EVENT_CREATURE_THINK_INTERVAL_MS);

        let armed = world.creatures.get(monster).is_some_and(|k| {
            k.base().next_walk_check.is_some() || !k.base().walk_queue.is_empty()
        });
        assert!(armed, "1098 think must still arm monster walk");
    }

    /// A0 — TShortway NOWAY clears chase target and enqueues roam Go same idle tick.
    #[test]
    fn test_772_chase_noway_clears_target_and_roams() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().follow_target.is_none()),
            "NOWAY must clear follow target"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .is_empty(),
            "NOWAY must not populate walk queue via greedy step"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go() || k.base().next_wakeup.is_some()),
            "NOWAY must enqueue roam Go on same idle tick"
        );
    }
}
