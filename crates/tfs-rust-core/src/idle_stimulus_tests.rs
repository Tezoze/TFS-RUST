    use tfs_rust_common::enums::{CombatType, ConditionType, Direction};
    use tfs_rust_common::Position;

    use crate::combat::{CombatDamage, CombatParams};
    use crate::creature::{
        CreatureKind, MonsterAiConfig, ChaseMode, MonsterSpell, MonsterState, SpellImpact,
        SpellShape,
    };
    use crate::creature_todo::{CreatureAction, MONSTER_IDLE_WAIT_MS};
    use crate::game_world::GameWorld;
    use crate::idle_stimulus::MonsterIdleWalkBranch;
    use crate::ids::CreatureId;
    use crate::monster_ai::{MonsterCombatCloseChaseEnqueue, MonsterEnqueueAttackResult};
    use crate::test_world::support::{
        dist_idle_monster_config, beat_driven_test_world, ensure_walkable_tile,
        insert_monster, insert_monster_with_config, insert_player, insert_spectator_player,
        minimal_world, test_player, TEST_SYNTHETIC_GROUND_WP,
    };

    /// Same-floor creature outside the 10-tile targeting box — `CanSeeFloor` awake without a target.
    fn register_distant_floor_spectator(world: &mut GameWorld, near: Position) -> CreatureId {
        let far = Position::new(near.x.saturating_add(15), near.y, near.z);
        ensure_walkable_tile(&mut world.map, far, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(world, test_player("Spectator", far));
        world.map.register_creature_at(far, player);
        player
    }

    /// Phase A — idle enqueues Go on drain; think no longer arms walk on 772.
    #[test]
    fn idle_stimulus_enqueues_go_for_active_monster() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        for x in 101..=104 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
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

        // 772 ToDo/IdleStimulus engine owns Go enqueue — no per-creature think sweep.
        // Verify the idle path did not re-arm Go after the drain.
        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go()),
            "772 idle drain must not re-enqueue Go after execute"
        );
    }

    /// Phase A — duplicate Go / heap entries suppressed when wakeup already armed.
    #[test]
    fn idle_go_enqueue_respects_wakeup_gate() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
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

    /// RC2 — idle stimulus must always schedule a wakeup, even when no action was queued.
    /// C++ `IdleStimulus` idle-wandering catch-all always ends with `ToDoWait(1000) + ToDoStart()`
    /// (`crnonpl.cc:2938–2939`). Without it, a monster with no target and no roam step stalls.
    #[test]
    fn rc2_idle_stimulus_always_schedules_wakeup_when_no_action_queued() {
        let mut world = beat_driven_test_world();
        // Single walkable tile — monster is surrounded by non-walkable, so roam will fail.
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Rat", pos, 200);

        // A second non-summon monster on the same floor prevents sleep
        // (`should_sleep = false` in `monster_idle_acquire_target`) but is NOT a valid
        // target (filtered out in the target selection loop), so the monster falls through
        // to the idle-wandering catch-all with no target and no Go queued.
        let bystander_pos = Position::new(102, 100, 7);
        ensure_walkable_tile(&mut world.map, bystander_pos, TEST_SYNTHETIC_GROUND_WP);
        let _bystander = insert_monster(&mut world, "Spider", bystander_pos, 200);
        world.map.register_creature_at(bystander_pos, _bystander);

        // No target, no opponents — idle stimulus will try to roam and fail (Hold).
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.state = MonsterState::Idle;
        }

        // Clear any pre-existing wakeup from spawn.
        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().next_wakeup = None;
        }

        world.monster_idle_stimulus(monster);

        let has_wakeup = world
            .creatures
            .get(monster)
            .is_some_and(|k| k.base().next_wakeup.is_some());
        assert!(
            has_wakeup,
            "RC2: idle stimulus must schedule a 1000 ms wakeup even when roam fails (Hold)"
        );
    }

    /// RC2 — idle stimulus must not double-schedule when a wakeup was already armed by a branch.
    #[test]
    fn rc2_idle_stimulus_does_not_double_schedule_when_already_armed() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        for x in 101..=104 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }

        world.monster_idle_stimulus(monster);

        // Chase branch should have armed a wakeup (Go + ToDoStart).
        let wakeup_after_chase = world
            .creatures
            .get(monster)
            .and_then(|k| k.base().next_wakeup)
            .expect("chase branch armed a wakeup");
        let todo_len = world
            .creatures
            .get(monster)
            .map(|k| k.base().todo.queue.len())
            .unwrap_or(0);

        // Run idle stimulus again — the trailing tail must NOT overwrite the existing wakeup.
        world.monster_idle_stimulus(monster);
        let wakeup_after_second = world
            .creatures
            .get(monster)
            .and_then(|k| k.base().next_wakeup);

        assert_eq!(
            wakeup_after_second,
            Some(wakeup_after_chase),
            "RC2: trailing tail must not overwrite an already-armed wakeup"
        );
        // Todo list should not have an extra Wait stacked from the tail.
        let todo_len_after = world
            .creatures
            .get(monster)
            .map(|k| k.base().todo.queue.len())
            .unwrap_or(0);
        assert!(
            todo_len_after <= todo_len + 1,
            "RC2: trailing tail must not stack extra Wait actions when already armed"
        );
    }

    /// RC2 — the trailing wakeup delay matches C++ `ToDoWait(1000)` (`crnonpl.cc:2938`).
    #[test]
    fn rc2_idle_trailing_wait_is_1000ms() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Rat", pos, 200);

        // Bystander prevents sleep without being a valid target (same as test above).
        let bystander_pos = Position::new(102, 100, 7);
        ensure_walkable_tile(&mut world.map, bystander_pos, TEST_SYNTHETIC_GROUND_WP);
        let _bystander = insert_monster(&mut world, "Spider", bystander_pos, 200);
        world.map.register_creature_at(bystander_pos, _bystander);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.state = MonsterState::Idle;
        }
        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().next_wakeup = None;
        }

        world.monster_idle_stimulus(monster);

        // The trailing tail enqueues a Wait(1000) — verify the todo head is a 1000 ms Wait.
        let has_1000ms_wait = world.creatures.get(monster).is_some_and(|k| {
            k.base().todo.queue.iter().any(|a| {
                matches!(a, CreatureAction::Wait { delay_ms } if *delay_ms == MONSTER_IDLE_WAIT_MS)
            })
        });
        assert!(
            has_1000ms_wait,
            "RC2: trailing tail must enqueue ToDoWait(1000), matching C++ crnonpl.cc:2938"
        );
    }

    /// Phase A — process_creature_todo runs idle when action queue empty on wakeup.
    #[test]
    fn process_creature_todo_runs_idle_on_empty_queue() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(108, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        for x in 101..=108 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
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
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        for x in 101..=108 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
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
                    TEST_SYNTHETIC_GROUND_WP,
                );
            }
        }

        let monster = insert_monster(&mut world, "Wolf", pos, 200);
        register_distant_floor_spectator(&mut world, pos);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }

        world.monster_idle_stimulus(monster);

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(todo.has_go(), "772 idle must enqueue roam Go");
        assert!(todo.has_wait(), "772 roam must enqueue Wait(1000) after Go");
    }

    /// Blocked dance / stand-still at melee goal must not force a chase repath on next idle.
    #[test]
    fn force_update_at_follow_goal_skips_idle_repath() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

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

    #[test]
    fn test_772_classify_roam_without_follow() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Wolf", pos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::Roam
        );
    }

    #[test]
    fn test_772_classify_flee_before_melee() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.health = 10;
            m.run_away_health = 20;
            m.flee_opening_melee_dance_done = true;
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::Flee
        );
    }

    /// X3 — first adjacent idle while `runonhealth` flee is active still classifies `MeleeDance`.
    #[test]
    fn test_772_adjacent_fleeing_first_idle_melee_dances_then_flee() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let monster = insert_monster(&mut world, "Dragon", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.health = 10;
            m.run_away_health = 20;
            m.flee_opening_melee_dance_done = false;
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeDance
        );

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.flee_opening_melee_dance_done = true;
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::Flee
        );
    }

    /// X3 — melee-only band (`targetdistance=1`) uses close `melee_dance`, not dist arms.
    #[test]
    fn test_772_classify_melee_dance_when_throw_not_possible_at_adjacent() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let mut cfg = MonsterAiConfig {
            is_hostile: true,
            target_distance: 1,
            melee_skill: 68,
            ..MonsterAiConfig::default()
        };
        cfg.spells.push(MonsterSpell {
            delay: 2000,
            range: 7,
            radius: 0,
            min_cycle: 0,
            shape: SpellShape::Victim,
            impact: SpellImpact::Damage {
                element: CombatType::Physical,
                base: 10,
                variation: 10,
            },
            shoot_effect: None,
            area_effect: None,
        });
        let monster = insert_monster_with_config(&mut world, "Dragon", mpos, 200, cfg);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }

        assert!(
            world.monster_can_use_attack(monster, mpos, player),
            "melee strike still counts for canUseAttack at cheb=1"
        );
        assert!(
            world.monster_throw_possible(monster, mpos, player),
            "ranged spell still in band at cheb=1"
        );
        assert!(
            !world.monster_idle_uses_dist_branch(monster, mpos, player, 1),
            "targetdistance=1 keeps close branch even when throw is possible"
        );
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeDance
        );
    }

    #[test]
    fn test_772_classify_master_follow() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let master = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.master = Some(master);
            m.base.follow_target = Some(master);
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MasterFollow
        );
    }

    #[test]
    fn test_772_classify_melee_vs_dist() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos_melee = Position::new(103, 100, 7);
        let ppos_dist = Position::new(106, 100, 7);
        for x in 99..=106 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let melee_monster = insert_monster_with_config(
            &mut world,
            "FixtureIdleChase772",
            mpos,
            200,
            MonsterAiConfig {
                is_hostile: false,
                ..MonsterAiConfig::default()
            },
        );
        let melee_player = insert_player(&mut world, test_player("Hero1", ppos_melee));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(melee_monster) {
            m.is_idle = false;
            m.base.follow_target = Some(melee_player);
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(melee_monster),
            MonsterIdleWalkBranch::MeleeChase
        );

        let dist_monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            dist_idle_monster_config(4),
        );
        let dist_player = insert_player(&mut world, test_player("Hero2", ppos_dist));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(dist_monster) {
            m.is_idle = false;
            m.base.follow_target = Some(dist_player);
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(dist_monster),
            MonsterIdleWalkBranch::DistChase
        );
    }

    #[test]
    fn test_772_classify_dist_dance_at_band() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        for x in 100..=104 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            dist_idle_monster_config(4),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::DistDance
        );
    }

    #[test]
    fn test_772_classify_melee_dance_adjacent() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeDance,
            "follow without attack_target may still rand(0,4) dance"
        );
    }

    #[test]
    fn test_772_attacking_posture_keeps_melee_dance_at_adjacent() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeDance,
            "ATTACKING melee still rand(0,4) dances at cheb==1"
        );
    }

    /// Flee arm uses `SearchFlightField` (single step), not a multi-step `TShortway` batch.
    #[test]
    fn test_772_flee_uses_flight_field_not_shortway() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(99, 100, 7), TEST_SYNTHETIC_GROUND_WP);

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.health = 10;
            m.run_away_health = 20;
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        world.monster_idle_stimulus(monster);

        let queue_len = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .len();
        assert!(
            queue_len <= 1,
            "flee idle must queue at most one flight-field step, got {queue_len}"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go() || k.base().next_wakeup.is_some()),
            "flee idle must enqueue Go"
        );
    }

    /// P0-4 — melee chase at cheb==2 uses reference `must:false, max:3`; trim stops at cheb≤1.
    ///
    /// Uses default spawn (`melee_skill==0`, state not `Attacking`) so classify stays `MeleeChase`;
    /// fist monsters in `Attacking` skip idle chase — see `test_e3_attacking_skips_idle_melee_chase`.
    #[test]
    fn test_772_melee_chase_cheb2_must_false_max_three() {
        use crate::monster_ai::{monster_idle_chase_step_budget, MonsterIdleChaseRepathOutcome};
        use crate::pathfinding::CHASE_PATH_MAX_STEPS;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeChase
        );
        let (max_steps, must_reach) = monster_idle_chase_step_budget(true, false, 2, 1);
        assert_eq!((max_steps, must_reach), (CHASE_PATH_MAX_STEPS, false));

        let outcome =
            world.monster_idle_chase_repath(monster, Some("idle_drain"), max_steps, must_reach);
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert_eq!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .len(),
            1,
            "melee chase at cheb==2 must queue one step (trim at cheb≤1), not must:true NOWAY"
        );
    }

    /// A2 regression — farther melee chase still allows up to 3 steps.
    #[test]
    fn test_772_melee_chase_cheb4_three_steps() {
        use crate::monster_ai::{monster_idle_chase_step_budget, MonsterIdleChaseRepathOutcome};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        for x in 100..=104u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        let (max_steps, must_reach) = monster_idle_chase_step_budget(true, false, 4, 1);
        assert_eq!((max_steps, must_reach), (3, false));

        let outcome =
            world.monster_idle_chase_repath(monster, Some("idle_drain"), max_steps, must_reach);
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert_eq!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .len(),
            3,
            "open-line melee chase at cheb==4 should queue three steps"
        );
    }

    /// A3 — dist chase step budget is `cheb - target_distance`, not global `CHASE_PATH_MAX_STEPS`.
    #[test]
    fn test_772_dist_chase_step_budget_from_target_distance() {
        use crate::monster_ai::{monster_idle_chase_step_budget, MonsterIdleChaseRepathOutcome};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos_band4 = Position::new(106, 100, 7);
        let ppos_band3 = Position::new(106, 110, 7);
        for x in 100..=106u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let dist_monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            dist_idle_monster_config(4),
        );
        let dist_player = insert_player(&mut world, test_player("Hero4", ppos_band4));
        world.map.register_creature_at(ppos_band4, dist_player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(dist_monster) {
            m.is_idle = false;
            m.base.follow_target = Some(dist_player);
            m.base.attack_target = Some(dist_player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(dist_monster),
            MonsterIdleWalkBranch::DistChase
        );
        let (max_steps, must_reach) = monster_idle_chase_step_budget(false, true, 6, 4);
        assert_eq!((max_steps, must_reach), (2, false));

        let outcome = world.monster_idle_chase_repath(
            dist_monster,
            Some("idle_drain"),
            max_steps,
            must_reach,
        );
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert_eq!(
            world
                .creatures
                .get(dist_monster)
                .unwrap()
                .base()
                .walk_queue
                .len(),
            2,
            "dist chase at cheb==6 with band 4 should queue two steps"
        );

        let mpos_band3 = Position::new(100, 110, 7);
        for x in 100..=106u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 110, 7), 150);
        }
        let band3_monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos_band3,
            200,
            dist_idle_monster_config(3),
        );
        let band3_player = insert_player(&mut world, test_player("Hero3", ppos_band3));
        world.map.register_creature_at(ppos_band3, band3_player);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(band3_monster) {
            m.is_idle = false;
            m.base.follow_target = Some(band3_player);
            m.base.attack_target = Some(band3_player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        let (max_steps, must_reach) = monster_idle_chase_step_budget(false, true, 6, 3);
        assert_eq!((max_steps, must_reach), (3, false));
        let outcome = world.monster_idle_chase_repath(
            band3_monster,
            Some("idle_drain"),
            max_steps,
            must_reach,
        );
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert_eq!(
            world
                .creatures
                .get(band3_monster)
                .unwrap()
                .base()
                .walk_queue
                .len(),
            3,
            "dist chase at cheb==6 with band 3 should queue three steps"
        );
    }

    /// A2 / X5 — failed melee dance at band must not re-enqueue Go on 772 idle Hold.
    #[test]
    fn test_772_idle_hold_no_dance_poll() {
        use crate::tile::{flags as tilestate, Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);
        for (x, y) in [(99, 100), (101, 100), (100, 99), (100, 101)] {
            world.map.insert_tile(
                Position::new(x, y, 7),
                Tile::Normal(TileBody {
                    ground: Some(150),
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeDance,
            "ATTACKING melee still attempts rand(0,4) dance at cheb==1"
        );

        world.monster_idle_stimulus(monster);

        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go()),
            "blocked dance tiles must not enqueue spurious Go"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_attack()),
            "stick-fight must enqueue Attack when dance cannot move"
        );
        assert!(world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .is_empty());
    }

    /// A0 — TShortway NOWAY clears chase target and enqueues roam Go same idle tick.
    #[test]
    fn test_772_chase_noway_clears_target_and_roams() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new((100 + dx) as u16, (100 + dy) as u16, 7),
                    TEST_SYNTHETIC_GROUND_WP,
                );
            }
        }
        ensure_walkable_tile(&mut world.map, ppos, 150);

        let monster = insert_monster_with_config(
            &mut world,
            "FixtureIdleChase772",
            mpos,
            200,
            MonsterAiConfig {
                is_hostile: false,
                ..MonsterAiConfig::default()
            },
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeChase,
            "non-fist fixture must use idle melee chase"
        );

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().follow_target.is_none()),
            "NOWAY must clear follow target"
        );
        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(
            todo.has_go(),
            "NOWAY must enqueue roam Go on same idle tick"
        );
        assert!(
            todo.has_wait(),
            "NOWAY roam must enqueue trailing Wait(1000)"
        );
    }

    /// A4 / X4 — 772 `getNextStep` must not inline flee when queue is empty.
    #[test]
    fn test_772_get_next_step_no_inline_flee_on_772() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.health = 10;
            m.run_away_health = 20;
            m.base.has_follow_path = true;
            m.base.walk_queue.clear();
        }

        let now = std::time::Instant::now();
        assert_eq!(
            world.monster_next_walk_step(monster, now),
            None,
            "772 getNextStep must defer flee to idle drain"
        );
    }

    /// A4 — dist_dance at keep band via idle only, not `getNextStep`.
    #[test]
    fn test_772_dist_dance_via_idle() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(99, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(101, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(100, 99, 7), TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(100, 101, 7), TEST_SYNTHETIC_GROUND_WP);

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            dist_idle_monster_config(4),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        for _ in 0..50 {
            if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
                m.base.walk_queue.clear();
                m.base.has_follow_path = false;
            }
            world.monster_idle_stimulus(monster);
            if let Some(dir) = world
                .creatures
                .get(monster)
                .and_then(|k| k.base().walk_queue.back().copied())
            {
                assert!(
                    matches!(dir, Direction::North | Direction::South),
                    "only North or South maintain target distance 4 from East-aligned target, got {:?}",
                    dir
                );
            }
        }
    }

    /// A5 / B2 — master follow Manhattan 2 enqueues Wait only (no Go).
    #[test]
    fn test_772_master_follow_manhattan_2_hold() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let master = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.master = Some(master);
            m.base.follow_target = Some(master);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .is_empty(),
            "Manhattan 2 must hold without chase path"
        );
        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go()),
            "Manhattan 2 must not enqueue Go"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_wait()),
            "Manhattan 2 must enqueue Wait(1000)"
        );
    }

    /// A5 / B2 — master follow Manhattan 3 enqueues Wait only.
    #[test]
    fn test_772_master_follow_manhattan_3_hold() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(103, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let master = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.master = Some(master);
            m.base.follow_target = Some(master);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .is_empty(),
            "Manhattan 3 must hold without chase path"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_wait()),
            "Manhattan 3 must enqueue Wait(1000)"
        );
    }

    /// A5 — master follow beyond wait band queues up to 3 steps.
    #[test]
    fn test_772_master_follow_manhattan_5_chases() {
        use crate::monster_ai::MonsterIdleChaseRepathOutcome;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        for x in 100..=105u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let master = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        world.map.register_creature_at(ppos, master);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.master = Some(master);
            m.base.follow_target = Some(master);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        let outcome = world.monster_idle_master_follow(monster, Some("idle_drain"));
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .len()
                <= 3,
            "master follow must cap at 3 steps"
        );
    }

    #[test]
    fn test_772_wait_schedules_1000ms_wakeup() {
        let mut world = beat_driven_test_world();
        world.server_ms = 200;
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Rat", pos, 200);

        world.idle_enqueue_wait_and_start(monster, MONSTER_IDLE_WAIT_MS);
        world.run_monster_todo_execute(monster);

        assert!(world.creatures.get(monster).unwrap().base().todo.is_empty());
        assert_eq!(
            world.creatures.get(monster).unwrap().base().next_wakeup,
            Some(200 + MONSTER_IDLE_WAIT_MS)
        );
    }

    /// Regression: multi-step chase must drain the full `walk_queue`, not freeze after one Go.
    #[test]
    fn test_772_multi_step_chase_continues_after_first_go() {
        use crate::monster_ai::{monster_idle_chase_step_budget, MonsterIdleChaseRepathOutcome};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        for x in 100..=104u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        let (max_steps, must_reach) = monster_idle_chase_step_budget(true, false, 4, 1);
        assert_eq!((max_steps, must_reach), (3, false));
        let outcome =
            world.monster_idle_chase_repath(monster, Some("idle_drain"), max_steps, must_reach);
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert_eq!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .len(),
            3
        );

        world.enqueue_creature_go(monster);
        world.schedule_immediate_todo_wakeup(monster);
        world.process_creature_todo(monster);

        let pos_after_one = world.creatures.get(monster).unwrap().position();
        assert!(
            pos_after_one.x > mpos.x,
            "first Go must move monster east from {:?}, got {:?}",
            mpos,
            pos_after_one
        );

        let wq_after_one = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .len();
        assert!(
            wq_after_one >= 1,
            "after first step walk_queue should still have pending steps, got {wq_after_one}"
        );

        // Drain all scheduled wakeups until monster reaches player column or stalls.
        for _ in 0..20 {
            let wakeup = world
                .creatures
                .get(monster)
                .and_then(|k| k.base().next_wakeup);
            let Some(wu) = wakeup else {
                break;
            };
            world.server_ms = wu;
            while world
                .todo_queue
                .peek()
                .is_some_and(|e| e.execution_time <= world.server_ms)
            {
                world.drain_todo_queue();
            }
        }

        let final_pos = world.creatures.get(monster).unwrap().position();
        assert!(
            final_pos.x > pos_after_one.x,
            "multi-step chase must continue past first tile (after one={:?}, final={:?}, wq={})",
            pos_after_one,
            final_pos,
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .len()
        );
    }

    #[test]
    fn test_772_roam_pacing_via_wait_not_last_step() {
        let mut world = beat_driven_test_world();
        world.server_ms = 0;
        let pos = Position::new(100, 100, 7);
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new((100 + dx) as u16, (100 + dy) as u16, 7),
                    TEST_SYNTHETIC_GROUND_WP,
                );
            }
        }
        let monster = insert_monster(&mut world, "Wolf", pos, 200);
        register_distant_floor_spectator(&mut world, pos);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }

        world.monster_idle_stimulus(monster);
        assert!(world.creatures.get(monster).unwrap().base().todo.has_go());

        world.run_monster_todo_execute(monster);
        assert!(
            world.creatures.get(monster).unwrap().base().todo.is_empty(),
            "Go then Wait chain must drain Go and schedule Wait"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .next_wakeup
                .unwrap()
                >= MONSTER_IDLE_WAIT_MS
        );

        world.monster_idle_stimulus(monster);
        assert!(
            !world.creatures.get(monster).unwrap().base().todo.has_go(),
            "Wait in flight must block immediate re-roam"
        );
    }

    #[test]
    fn test_772_dist_flee_fail_enqueues_wait() {
        use tfs_rust_common::enums::ZoneType;

        use crate::tile::{flags as tilestate, Tile, TileBody};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        for (x, y) in [(99, 100), (100, 99), (100, 101)] {
            world.map.insert_tile(
                Position::new(x, y, 7),
                Tile::Normal(TileBody {
                    ground: Some(TEST_SYNTHETIC_GROUND_WP),
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            dist_idle_monster_config(4),
        );

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::DistFlee
        );

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_wait() && !k.base().todo.has_go()),
            "dist_flee fail must enqueue Wait only"
        );
    }

    #[test]
    fn test_772_dist_dance_enqueues_go_and_wait() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        for x in 100..=104 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        ensure_walkable_tile(&mut world.map, Position::new(99, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(100, 99, 7), TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(100, 101, 7), TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            dist_idle_monster_config(4),
        );

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::DistDance
        );

        let mut got_go = false;
        for _ in 0..50 {
            if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
                m.base.walk_queue.clear();
                m.base.todo.queue.clear();
                m.base.has_follow_path = false;
                m.base.next_wakeup = None;
            }
            world.monster_idle_stimulus(monster);
            if world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go())
            {
                got_go = true;
                break;
            }
        }

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(got_go, "dist_dance must enqueue Go");
        assert!(todo.has_wait(), "dist_dance must enqueue Wait after Go");
    }

    #[test]
    fn test_772_get_next_step_no_roam_on_beat_loop() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new((100 + dx) as u16, (100 + dy) as u16, 7),
                    TEST_SYNTHETIC_GROUND_WP,
                );
            }
        }
        let monster = insert_monster(&mut world, "Wolf", pos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }

        let now = std::time::Instant::now();
        assert_eq!(
            world.monster_next_walk_step(monster, now),
            None,
            "772 getNextStep must not pick roam step inline"
        );
    }

    #[test]
    fn test_772_attack_from_idle_queue() {
        use tfs_rust_common::enums::ZoneType;

        use crate::tile::{flags as tilestate, Tile, TileBody};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        for (x, y) in [(99, 100), (100, 99), (100, 101)] {
            world.map.insert_tile(
                Position::new(x, y, 7),
                Tile::Normal(TileBody {
                    ground: Some(TEST_SYNTHETIC_GROUND_WP),
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
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
                .is_some_and(|k| k.base().todo.has_attack()),
            "hostile melee at cheb==1 must enqueue Attack without spell-range canUseAttack"
        );
    }

    /// P0-2 — change-target ticks advance on `ProcessCreatures` only, not each idle drain.
    #[test]
    fn test_772_change_target_only_on_process_creatures() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        for x in 100..=105u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let config = MonsterAiConfig {
            change_target_speed: 4_000,
            change_target_chance: 100,
            ..Default::default()
        };
        let monster = insert_monster_with_config(&mut world, "Rat", mpos, 200, config);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.target_change_ticks = 0;
            m.target_change_cooldown = 0;
        }

        for _ in 0..5 {
            world.monster_idle_stimulus(monster);
        }
        let ticks_after_idle = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m.target_change_ticks,
            _ => 0,
        };
        assert_eq!(
            ticks_after_idle, 0,
            "idle drain must not advance change-target ticks on 772"
        );

        world.add_creature_think_check(monster);
        world.process_creatures();
        let ticks_after_think = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m.target_change_ticks,
            _ => 0,
        };
        assert_eq!(
            ticks_after_think, 0,
            "772 ProcessCreatures must not run TFS change-target rolls (no `onThinkTarget` in `crnonpl.cc`)"
        );
    }

    /// P0-3 — melee stick-fight enqueues Attack without trailing 1 s Wait.
    #[test]
    fn test_772_melee_stick_fight_no_wait_after_attack() {
        use tfs_rust_common::enums::ZoneType;

        use crate::creature_todo::{CreatureAction, MONSTER_IDLE_WAIT_MS};
        use crate::tile::{flags as tilestate, Tile, TileBody};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        for (x, y) in [(99, 100), (100, 99), (100, 101)] {
            world.map.insert_tile(
                Position::new(x, y, 7),
                Tile::Normal(TileBody {
                    ground: Some(TEST_SYNTHETIC_GROUND_WP),
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        world.monster_idle_stimulus(monster);

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(todo.has_attack(), "melee stick-fight must enqueue Attack");
        assert!(
            !todo.queue.iter().any(|a| {
                matches!(a, CreatureAction::Wait { delay_ms } if *delay_ms == MONSTER_IDLE_WAIT_MS)
            }),
            "melee stick-fight must not enqueue trailing 1 s Wait after Attack"
        );
    }

    #[test]
    fn test_772_think_skips_creature_on_attacking() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.attack_target = Some(player);
            m.base.follow_target = Some(player);
        }
        world.add_creature_think_check(monster);

        world.process_creatures();

        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_attack()),
            "772 ~1 Hz think must not enqueue Attack — idle todo path owns combat tail"
        );
    }

    fn e1_melee_target_setup(world: &mut GameWorld, melee_skill: i32) -> (CreatureId, CreatureId) {
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        let mut player = test_player("Hero", ppos);
        player.base.health = 500;
        player.base.max_health = 500;
        let player = insert_player(world, player);
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = melee_skill;
            m.is_hostile = true;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }
        (monster, player)
    }

    #[test]
    fn test_e1_melee_monster_enters_attacking_on_idle() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e1_melee_target_setup(&mut world, 15);

        world.monster_idle_stimulus(monster);

        assert_eq!(
            world.creatures.get(monster).and_then(|k| match k {
                CreatureKind::Monster(m) => Some(m.state),
                _ => None,
            }),
            Some(MonsterState::Attacking),
            "hostile melee with target must enter Attacking on idle drain"
        );
    }

    #[test]
    fn test_e1_idle_reset_reasserts_attacking_each_tick() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e1_melee_target_setup(&mut world, 15);

        for tick in 0..2 {
            if tick > 0 {
                if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
                    m.base.todo.queue.clear();
                    m.base.walk_queue.clear();
                    m.base.next_wakeup = None;
                }
            }
            world.monster_idle_stimulus(monster);
            assert_eq!(
                world.creatures.get(monster).and_then(|k| match k {
                    CreatureKind::Monster(m) => Some(m.state),
                    _ => None,
                }),
                Some(MonsterState::Attacking),
                "reset→Idle then walk must re-set Attacking when walk section runs"
            );
        }
    }

    #[test]
    fn test_e1_under_attack_promoted_to_attacking_in_walk_section() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e1_melee_target_setup(&mut world, 15);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::UnderAttack;
        }

        world.monster_idle_stimulus(monster);

        assert_eq!(
            world.creatures.get(monster).and_then(|k| match k {
                CreatureKind::Monster(m) => Some(m.state),
                _ => None,
            }),
            Some(MonsterState::Attacking),
            "top reset preserves UnderAttack; walk prelude promotes to Attacking — crnonpl.cc:2705"
        );
    }

    #[test]
    fn test_e1_no_attacking_without_melee_skill() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e1_melee_target_setup(&mut world, 0);

        world.monster_idle_stimulus(monster);

        assert_eq!(
            world.creatures.get(monster).and_then(|k| match k {
                CreatureKind::Monster(m) => Some(m.state),
                _ => None,
            }),
            Some(MonsterState::Idle),
            "melee_skill==0 must not enter Attacking"
        );
    }

    #[test]
    fn test_e1_panic_blocks_attacking_set() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = 15;
            m.is_hostile = true;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
            m.state = MonsterState::Panic;
        }

        world.monster_idle_stimulus(monster);

        assert_eq!(
            world.creatures.get(monster).and_then(|k| match k {
                CreatureKind::Monster(m) => Some(m.state),
                _ => None,
            }),
            Some(MonsterState::Panic),
            "PANIC must block Attacking transition"
        );
    }

    fn e5_apply_player_hit(
        world: &mut GameWorld,
        monster: CreatureId,
        player: CreatureId,
        damage: i32,
    ) {
        let applied = world.combat_execute_with_stimulus(
            Some(player),
            monster,
            &CombatDamage {
                primary: (CombatType::Physical, -damage),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams::default(),
        );
        assert!(applied, "combat_execute_with_stimulus must apply HP loss");
    }

    #[test]
    fn test_e5_idle_with_target_hit_becomes_under_attack() {
        let mut world = beat_driven_test_world();
        let (monster, player) = e1_melee_target_setup(&mut world, 15);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::Idle;
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
        }

        e5_apply_player_hit(&mut world, monster, player, 5);

        let m = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m,
            _ => panic!("expected monster"),
        };
        assert_eq!(
            m.state,
            MonsterState::UnderAttack,
            "idle rat with target must flip to UnderAttack on hit"
        );
        assert!(
            m.base
                .todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Wait { delay_ms: 0 })),
            "DamageStimulus must ToDoYield (Wait(0)) — cract.cc:1001"
        );
        assert!(
            m.base.next_wakeup.is_some(),
            "yield must schedule immediate todo wakeup"
        );
    }

    #[test]
    fn test_e5_sleeping_no_target_hit_becomes_panic_and_yields() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::Sleeping;
            m.is_idle = true;
            m.base.clear_targets();
            m.opponent_ids.clear();
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
        }

        e5_apply_player_hit(&mut world, monster, player, 3);

        let m = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m,
            _ => panic!("expected monster"),
        };
        assert_eq!(
            m.state,
            MonsterState::Panic,
            "sleeping rat without target → PANIC"
        );
        assert!(!m.is_idle, "PANIC must wake monster from idle posture");
        assert!(
            m.opponent_ids.contains(&player),
            "attacker must be recorded in opponent_ids"
        );
        assert!(
            m.base
                .todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Wait { delay_ms: 0 })),
            "sleeping hit must ToDoYield"
        );
    }

    #[test]
    fn test_e5_panic_dances_without_low_health() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::Sleeping;
            m.is_idle = true;
            m.run_away_health = 0;
            m.base.health = 200;
            m.base.clear_targets();
            m.opponent_ids.clear();
        }

        e5_apply_player_hit(&mut world, monster, player, 1);

        assert!(
            world.creatures.get(monster).is_some_and(|k| match k {
                CreatureKind::Monster(m) => {
                    m.state == MonsterState::Panic && !m.is_fleeing()
                }
                _ => false,
            }),
            "PANIC must not gate IsFleeing — crnonpl.cc:3136"
        );
        // C++ `DamageStimulus` does not set `Target`; idle `Strategy[]` picks on next drain.
        world.monster_idle_stimulus(monster);
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeDance
        );
    }

    /// C++ `%5` case 2/3 map to North/South dest tiles — `crnonpl.cc:2817-2818`.
    #[test]
    fn test_772_dance_dir_order_matches_cpp() {
        use crate::sim_glibc_rand::DANCE_DIR_ORDER;
        use tfs_rust_common::Position;

        let pos = Position::new(32361, 32290, 7);
        assert_eq!(
            pos.offset(DANCE_DIR_ORDER[2].unwrap()),
            Position::new(32361, 32289, 7),
            "case 2 must step north (DestY-=1)"
        );
        assert_eq!(
            pos.offset(DANCE_DIR_ORDER[3].unwrap()),
            Position::new(32361, 32291, 7),
            "case 3 must step south (DestY+=1)"
        );
    }

    #[test]
    fn test_e5_rehit_attacking_no_redundant_yield() {
        let mut world = beat_driven_test_world();
        let (monster, player) = e1_melee_target_setup(&mut world, 15);

        world.monster_idle_stimulus(monster);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            assert_eq!(m.state, MonsterState::Attacking);
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
        }

        e5_apply_player_hit(&mut world, monster, player, 2);
        e5_apply_player_hit(&mut world, monster, player, 2);

        let m = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m,
            _ => panic!("expected monster"),
        };
        assert_eq!(
            m.state,
            MonsterState::Attacking,
            "re-hit while Attacking must keep Attacking"
        );
        assert!(
            m.base.todo.queue.is_empty(),
            "re-hit with unchanged state must not storm ToDoYield"
        );
        assert!(
            m.base.next_wakeup.is_none(),
            "no redundant yield wakeup when state unchanged"
        );
    }

    fn e3_melee_target_at_cheb2(
        world: &mut GameWorld,
        melee_skill: i32,
    ) -> (CreatureId, CreatureId) {
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }
        let player = insert_player(world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = melee_skill;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
            m.base.todo.queue.clear();
        }
        (monster, player)
    }

    #[test]
    fn test_e3_attacking_skips_idle_melee_chase() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e3_melee_target_at_cheb2(&mut world, 15);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::Attacking;
            m.base.chase_mode = ChaseMode::Close;
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::Hold,
            "ATTACKING at cheb==2 must not use idle MeleeChase"
        );
    }

    #[test]
    fn test_e3_attack_path_enqueues_close_chase_at_cheb2() {
        let mut world = beat_driven_test_world();
        world.server_ms = 1000;
        let (monster, _player) = e3_melee_target_at_cheb2(&mut world, 15);

        world.monster_idle_stimulus(monster);

        let m = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m,
            _ => panic!("expected monster"),
        };
        assert_eq!(
            m.base.chase_mode,
            ChaseMode::Close,
            "melee ATTACKING must set CHASE_MODE_CLOSE"
        );
        assert!(
            !m.base.walk_queue.is_empty(),
            "attack-path CanToDoAttack must populate walk_queue at cheb==2"
        );
        let todo = &m.base.todo;
        assert!(todo.has_go(), "attack tail must enqueue Go before Attack");
        assert!(todo.has_attack(), "attack tail must enqueue Attack");
        assert!(
            !todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Wait { delay_ms: 100 })),
            "fist ToDoAttack skips Wait(100) when GetDistance()==1 (cract.cc:1327)"
        );
        let go_idx = todo
            .queue
            .iter()
            .position(|a| matches!(a, CreatureAction::Go))
            .expect("Go in queue");
        let attack_idx = todo
            .queue
            .iter()
            .position(|a| matches!(a, CreatureAction::Attack))
            .expect("Attack in queue");
        assert!(
            go_idx < attack_idx,
            "ToDoAttack order: Go before Attack (cract.cc:1325-1334)"
        );
    }

    /// Regression: ATTACKING melee monster at dist>1 must arm `next_wakeup` after idle stimulus.
    ///
    /// The walk branch is `Hold` (ATTACKING skips idle melee chase — `crnonpl.cc:2808`), so the Go
    /// is enqueued by `monster_combat_enqueue_close_chase_go` inside `monster_enqueue_todo_attack_actions`.
    /// C++ `ToDoStart` (`cract.cc:1010-1023`) always arms `NextWakeup` for the head todo entry; the
    /// Rust `needs_wakeup` gate previously skipped scheduling when `has_go()` was true, leaving the
    /// monster parked with `[Go, Attack]` and no heap entry until the ~1 Hz think tick rescued it
    /// — a visible ~1 s stall after every chase-batch drain while the target was kiting.
    #[test]
    fn test_e3_attacking_close_chase_arms_wakeup_after_idle() {
        let mut world = beat_driven_test_world();
        world.server_ms = 1000;
        let (monster, _player) = e3_melee_target_at_cheb2(&mut world, 15);

        world.monster_idle_stimulus(monster);

        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            base.todo.has_go(),
            "close-chase Go must be enqueued"
        );
        assert!(
            base.next_wakeup.is_some(),
            "ATTACKING close-chase Go must arm next_wakeup — \
             C++ ToDoStart always arms NextWakeup (cract.cc:1010-1023)"
        );
    }

    fn e2_adjacent_combat_setup(
        world: &mut GameWorld,
        melee_skill: i32,
        melee_attack: i32,
    ) -> (CreatureId, CreatureId) {
        let (monster, player) = e1_melee_target_setup(world, melee_skill);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.melee_attack = melee_attack;
        }
        (monster, player)
    }

    fn e2_run_attack_todo(world: &mut GameWorld, monster: CreatureId) {
        world.enqueue_creature_attack(monster);
        world.schedule_immediate_todo_wakeup(monster);
        world.run_monster_todo_execute(monster);
    }

    fn e2_drain_until_idle(world: &mut GameWorld, monster: CreatureId) {
        for _ in 0..30 {
            let wakeup = world
                .creatures
                .get(monster)
                .and_then(|k| k.base().next_wakeup);
            let Some(wu) = wakeup else {
                break;
            };
            world.server_ms = wu;
            while world
                .todo_queue
                .peek()
                .is_some_and(|e| e.execution_time <= world.server_ms)
            {
                world.drain_todo_queue();
            }
        }
    }

    #[test]
    fn test_e2_melee_damage_and_damage_map() {
        use crate::max_melee_damage_monster;

        let mut world = beat_driven_test_world();
        world.server_ms = 1000;
        let (monster, player) = e2_adjacent_combat_setup(&mut world, 15, 7);
        let hp_before = world.creatures.get(player).unwrap().base().health;

        e2_run_attack_todo(&mut world, monster);

        let hp_after = world.creatures.get(player).unwrap().base().health;
        assert!(hp_after < hp_before, "adjacent melee must reduce target HP");
        let dealt = (hp_before - hp_after) as u64;
        assert!(
            dealt <= max_melee_damage_monster(15, 7) as u64,
            "damage must not exceed max roll"
        );
        assert_eq!(
            world
                .creatures
                .get(player)
                .unwrap()
                .base()
                .damage_map
                .get(&monster)
                .copied(),
            Some(dealt),
            "damage_map must attribute dealt HP to attacker"
        );
    }

    #[test]
    fn test_e2_attack_cadence_2000ms() {
        let mut world = beat_driven_test_world();
        world.server_ms = 5000;
        let (monster, player) = e2_adjacent_combat_setup(&mut world, 15, 7);

        e2_run_attack_todo(&mut world, monster);
        let hp_after_first = world.creatures.get(player).unwrap().base().health;
        let earliest = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .earliest_attack_ms;
        assert_eq!(earliest, 5000 + 2000, "CloseAttack must DelayAttack(2000)");

        world.server_ms = earliest - 1;
        e2_run_attack_todo(&mut world, monster);
        assert_eq!(
            world.creatures.get(player).unwrap().base().health,
            hp_after_first,
            "attack must not land before cadence elapses"
        );

        world.server_ms = earliest;
        e2_drain_until_idle(&mut world, monster);
        let hp_second = world
            .creatures
            .get(player)
            .map(|k| k.base().health)
            .expect("player must remain in world");
        assert!(
            hp_second < hp_after_first,
            "second hit must land after 2000 ms cadence"
        );
    }

    #[test]
    fn test_e2_melee_adjacent_enqueues_attack_without_wait() {
        use crate::creature::monster_weapon_attack_distance;

        let mut world = beat_driven_test_world();
        let (monster, _player) = e2_adjacent_combat_setup(&mut world, 15, 7);
        let (melee_skill, has_ranged) = world
            .creatures
            .get(monster)
            .map(|k| match k {
                CreatureKind::Monster(m) => (m.melee_skill, m.spells.iter().any(|s| s.range > 1)),
                _ => (0, false),
            })
            .unwrap();
        assert_eq!(monster_weapon_attack_distance(melee_skill, has_ranged), 1);

        assert!(world.enqueue_creature_attack(monster));

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert_eq!(todo.queue.len(), 1);
        assert!(matches!(todo.queue[0], CreatureAction::Attack));
    }

    #[test]
    fn test_e2_wait_100_before_attack_when_weapon_range_not_close() {
        use crate::creature::monster_weapon_attack_distance;

        assert_eq!(monster_weapon_attack_distance(0, true), 3);
        assert_eq!(monster_weapon_attack_distance(15, true), 1);

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        let spell = MonsterSpell {
            delay: 4,
            range: 5,
            radius: 0,
            min_cycle: 6,
            shape: SpellShape::Victim,
            impact: SpellImpact::Condition {
                condition: ConditionType::Poison,
                cycle: 20,
                min_cycle: 6,
            },
            shoot_effect: None,
            area_effect: None,
        };
        let mut cfg = MonsterAiConfig::default();
        cfg.melee_skill = 0;
        cfg.spells = vec![spell];
        let monster = insert_monster_with_config(&mut world, "Cobra", mpos, 200, cfg);

        if monster_weapon_attack_distance(0, true) != 1 {
            world.enqueue_creature_wait(monster, 100);
        }
        world.enqueue_creature_attack(monster);

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert_eq!(todo.queue.len(), 2);
        assert!(matches!(
            todo.queue[0],
            CreatureAction::Wait { delay_ms: 100 }
        ));
        assert!(matches!(todo.queue[1], CreatureAction::Attack));
    }

    // ---- Rotate direct call (audit: turn-on-spot defect) ----
    //
    // C++ `Rotate(Target)` is called directly in `IdleStimulus` (`crnonpl.cc:2872-2873`),
    // NOT enqueued as a `TDRotate` todo action. The 0x6B turn broadcast and the first `TDGo`
    // move packet land in the same beat, so the client renders the turn imperceptibly.
    // Enqueuing `Rotate` caused a visible "turn on the spot" because the 0x6B fired in a
    // separate beat from any move packet.

    /// `monster_idle_rotate_toward_attack_target` turns the monster directly (no enqueue),
    /// matching C++'s unconditional `Rotate(Target)` direct call. The direction changes
    /// immediately and no `Rotate` action is left in the todo queue.
    #[test]
    fn test_rotate_direct_call_turns_monster_immediately() {
        let mut world = beat_driven_test_world();
        world.server_ms = 1000;
        let (monster, player) = e1_melee_target_setup(&mut world, 15);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::Attacking;
        }
        // Monster starts facing North (insert_monster default); player is East at (101,100).
        assert_eq!(
            world.creatures.get(monster).unwrap().base().direction,
            Direction::North
        );

        world.monster_idle_rotate_toward_attack_target(monster);

        // Direction changed immediately — no enqueue, no deferred execute.
        assert_eq!(
            world.creatures.get(monster).unwrap().base().direction,
            Direction::East,
            "Rotate must turn the monster toward the target immediately (direct call)"
        );
        // No Rotate action in the queue — it was a direct call, not an enqueue.
        // (CreatureAction no longer has a Rotate variant; the queue can only hold
        // Go/Wait/Attack, so this is structurally guaranteed.)
        let _ = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .todo
            .queue
            .iter()
            .count();
    }

    /// `Rotate(Target)` fires even when a walk (`Go`) is already armed — no `walk_timer_idle`
    /// gate, matching C++'s unconditional direct call. The old `monster_update_look_direction`
    /// path was gated and would SKIP the rotate when a walk timer was armed.
    #[test]
    fn test_rotate_direct_call_fires_when_walk_armed() {
        let mut world = beat_driven_test_world();
        world.server_ms = 1000;
        let (monster, player) = e1_melee_target_setup(&mut world, 15);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::Attacking;
        }

        // Arm a Go first (simulates the walk branch having queued a step).
        assert!(world.enqueue_creature_go(monster));
        // Arm a wakeup so walk_timer_idle() returns false — the old gate condition.
        world.schedule_immediate_todo_wakeup(monster);
        assert!(
            !world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_timer_idle(),
            "test precondition: walk timer must be armed (walk_timer_idle == false)"
        );

        // The idle rotate tail must still turn the monster despite the armed walk timer.
        world.monster_idle_rotate_toward_attack_target(monster);

        assert_eq!(
            world.creatures.get(monster).unwrap().base().direction,
            Direction::East,
            "Rotate must fire directly even when a walk timer is armed (no walk_timer_idle gate)"
        );
    }

    /// The idle combat tail calls `Rotate(Target)` directly (not enqueued) before
    /// `ToDoAttack` is enqueued — matching C++ `crnonpl.cc:2872-2877` order. The turn
    /// broadcast lands in the same beat as the first `Go`/`Attack`, making it imperceptible.
    #[test]
    fn test_idle_tail_rotate_direct_then_attack_enqueued() {
        let mut world = beat_driven_test_world();
        world.server_ms = 1000;
        let (monster, _player) = e1_melee_target_setup(&mut world, 15);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::Attacking;
            m.melee_attack = 7;
        }

        // Invoke the two idle combat tail calls in order (crnonpl.cc:2872-2877).
        world.monster_idle_rotate_toward_attack_target(monster);
        let attack_enqueued = world.monster_idle_maybe_enqueue_attack(monster);
        assert!(attack_enqueued);

        // Rotate was a direct call — direction already changed, no Rotate in the queue.
        assert_eq!(
            world.creatures.get(monster).unwrap().base().direction,
            Direction::East,
            "Rotate direct call must have turned the monster"
        );
        // Attack is enqueued (Rotate is not, since it was a direct call).
        // (CreatureAction no longer has a Rotate variant; the queue can only hold
        // Go/Wait/Attack, so absence of Rotate is structurally guaranteed.)
        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(
            todo.queue.iter().any(|a| matches!(a, CreatureAction::Attack)),
            "Attack must be enqueued after the direct Rotate call"
        );
    }

    fn e4_cobra_config() -> MonsterAiConfig {
        use std::path::Path;
        use tfs_rust_content::items::ItemDatabase;
        use tfs_rust_content::monsters::MonsterDatabase;

        let manifest = env!("CARGO_MANIFEST_DIR");
        let items = ItemDatabase {
            items: Default::default(),
            client_to_server: Default::default(),
        };
        let db = MonsterDatabase::load_dir(&Path::new(manifest).join("../../data/monster"), &items)
            .expect("load monsters");
        let mtype = db.monsters.get("cobra").cloned().expect("cobra type");
        MonsterAiConfig::from_monster_type(&mtype)
    }

    #[test]
    fn test_e4_cobra_poison_at_range() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(103, 100, 7);
        for x in 100..=103u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        let mut cfg = e4_cobra_config();
        cfg.spells[0].delay = 1;
        let monster = insert_monster_with_config(&mut world, "Cobra", mpos, 200, cfg);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.state = MonsterState::Idle;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }

        let mut poisoned = false;
        for attempt in 0..64 {
            if attempt > 0 {
                // Delay gate: rand() % 4 == 0 — retry until cast fires.
            }
            world.monster_idle_stimulus(monster);
            poisoned = world.creatures.get(player).is_some_and(|k| {
                k.base()
                    .active_conditions
                    .iter()
                    .any(|c| c.ctype == ConditionType::Poison)
            });
            if poisoned {
                break;
            }
        }
        assert!(
            poisoned,
            "cobra must apply poison condition to player at Chebyshev distance 3 within spell range 5"
        );
    }

    #[test]
    fn test_e4_casting_runs_after_target_acquire() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(103, 100, 7);
        for x in 100..=103u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        let mut cfg = e4_cobra_config();
        cfg.spells[0].delay = 1;
        let monster = insert_monster_with_config(&mut world, "Cobra", mpos, 200, cfg);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.state = MonsterState::Idle;
            m.strategy_nearest = 100;
            m.strategy_health = 0;
            m.strategy_damage = 0;
        }

        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .follow_target
                .is_none(),
            "precondition: no target before idle"
        );

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .follow_target
                .is_some(),
            "acquire must pick target same idle cycle"
        );
        assert!(
            world.creatures.get(player).is_some_and(|k| {
                k.base()
                    .active_conditions
                    .iter()
                    .any(|c| c.ctype == ConditionType::Poison)
            }),
            "cast must run after acquire on the same idle cycle when delay=1"
        );
    }

    #[test]
    fn test_e4_spell_delay_gate() {
        let mut world = beat_driven_test_world();
        world.seed_parity_rng(772);
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(103, 100, 7);
        for x in 100..=103u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        let mut cfg = e4_cobra_config();
        cfg.spells[0].delay = 4;
        let monster = insert_monster_with_config(&mut world, "Cobra", mpos, 200, cfg);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.state = MonsterState::Idle;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }

        let mut casts = 0u32;
        for _ in 0..40 {
            world.server_ms = world.server_ms.saturating_add(200);
            if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
                p.base.active_conditions.clear();
            }
            world.monster_idle_stimulus(monster);
            let poisoned = world.creatures.get(player).is_some_and(|k| {
                k.base()
                    .active_conditions
                    .iter()
                    .any(|c| c.ctype == ConditionType::Poison)
            });
            if poisoned {
                casts += 1;
            }
        }
        assert!(
            casts >= 4 && casts <= 16,
            "delay=4 gate should yield roughly 1-in-4 cast attempts over 40 idles, got {casts}"
        );
    }

    #[test]
    fn test_e2_attack_deferred_until_cadence() {
        let mut world = beat_driven_test_world();
        world.server_ms = 2000;
        let (monster, player) = e2_adjacent_combat_setup(&mut world, 15, 7);
        let hp_before = world.creatures.get(player).unwrap().base().health;

        e2_run_attack_todo(&mut world, monster);
        let hp_after_first = world.creatures.get(player).unwrap().base().health;
        assert!(hp_after_first < hp_before, "first attack must deal damage");

        world.enqueue_creature_attack(monster);
        world.schedule_immediate_todo_wakeup(monster);
        world.run_monster_todo_execute(monster);
        assert_eq!(
            world.creatures.get(player).unwrap().base().health,
            hp_after_first,
            "immediate re-attack must defer without damage"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .next_wakeup
                .is_some(),
            "deferred attack must schedule a wakeup"
        );
    }

    /// Regression: adjacent melee must not freeze after first hit while target stands still.
    ///
    /// C++ always enqueues `ToDoAttack` at the idle tail; `TDAttack` arms the cadence wakeup.
    #[test]
    fn test_e2_melee_adjacent_does_not_freeze_after_first_strike() {
        let mut world = beat_driven_test_world();
        world.server_ms = 5000;
        let (monster, player) = e2_adjacent_combat_setup(&mut world, 15, 7);
        let hp_before = world.creatures.get(player).unwrap().base().health;

        e2_run_attack_todo(&mut world, monster);
        let hp_after_first = world.creatures.get(player).unwrap().base().health;
        assert!(hp_after_first < hp_before, "first attack must deal damage");

        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            base.todo.has_attack() || base.next_wakeup.is_some(),
            "adjacent melee on cooldown must keep Attack or cadence wakeup armed (not freeze)"
        );

        let earliest = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .earliest_attack_ms;
        e2_drain_until_idle(&mut world, monster);
        let hp_second = world
            .creatures
            .get(player)
            .map(|k| k.base().health)
            .expect("player must remain in world");
        assert!(
            hp_second < hp_after_first,
            "second hit must land after cadence without target moving"
        );
        assert_eq!(
            earliest,
            5000 + 2000,
            "cadence must remain DelayAttack(2000) after idle re-enqueue"
        );
    }

    /// Empty `walk_queue` + no `TDAttack` — follow-move must not idle-repath (`crmain.cc:919-961`;
    /// lesson 37: empty queue defers to idle segment drain).
    #[test]
    fn test_chase_empty_queue_attacking_does_not_idle_repath_on_target_kite() {
        let mut world = beat_driven_test_world();
        world.server_ms = 5000;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let ppos_kited = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.base.chase_mode = ChaseMode::Close;
            m.base.walk_queue.clear();
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
            m.base.has_follow_path = false;
            m.base.force_update_follow_path = false;
            m.base.earliest_attack_ms = world.server_ms + 2000;
        }

        world.monster_dispatch_creature_move(player, ppos, ppos_kited);

        let base = world.creatures.get(monster).unwrap().base();
        assert_eq!(
            base.next_wakeup, None,
            "empty queue must not schedule idle repath on kite"
        );
        assert!(base.walk_queue.is_empty());
        assert!(base.todo.is_empty());
        assert!(!base.force_update_follow_path);
        assert_eq!(
            world.creatures.get(monster).unwrap().position(),
            mpos,
            "no idle repath — position unchanged until idle drain"
        );
        assert_eq!(
            base.follow_target,
            Some(player),
            "kite must not drop follow"
        );
    }

    /// `TDAttack` armed close-chase — `CreatureMoveStimulus` re-queues Wait+Attack (`crmain.cc:946-961`).
    #[test]
    fn test_chase_combat_move_stimulus_rearms_attack_on_target_kite() {
        use crate::creature_todo::CreatureAction;

        let mut world = beat_driven_test_world();
        world.server_ms = 5000;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let ppos_kited = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.base.chase_mode = ChaseMode::Close;
            m.base.walk_queue.clear();
            m.base.todo.queue.push_back(CreatureAction::Attack);
            m.base.todo.locked = false;
            m.base.next_wakeup = None;
            m.base.earliest_attack_ms = world.server_ms;
        }

        world.monster_dispatch_creature_move(player, ppos, ppos_kited);

        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            base.todo.has_attack(),
            "combat move stimulus must re-arm TDAttack after target kites away"
        );
        assert!(
            !base.todo.is_empty(),
            "combat re-arm must enqueue Wait+Attack actions"
        );
        assert_eq!(
            base.follow_target,
            Some(player),
            "combat re-arm must keep follow"
        );
    }

    /// Dist at keep-band: target flee must inline-chase, not sit in goal `ToDoWait(1000)`.
    #[test]
    fn test_772_dist_target_flee_inline_chase_after_goal_wait() {
        use crate::creature_todo::CreatureAction;
        use crate::test_world::support::dist_idle_monster_config;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        let ppos_fled = Position::new(105, 100, 7);
        for x in 100..=105 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster_with_config(
            &mut world,
            "Hunter",
            mpos,
            200,
            dist_idle_monster_config(4),
        );
        world.map.register_creature_at(mpos, monster);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.base.walk_queue.clear();
            m.base.todo.queue.clear();
            m.base.has_follow_path = true;
        }

        world.monster_idle_stimulus(monster);
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_wait()),
            "at dist band idle arms trailing wait"
        );

        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_fled, player);
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_fled;
        }
        world.monster_dispatch_creature_move(player, ppos, ppos_fled);

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(
            todo.has_go(),
            "target leaving dist band must arm chase Go immediately"
        );
        assert!(
            !todo.queue.iter().any(|a| matches!(a, CreatureAction::Wait { delay_ms: 1000 })),
            "goal wait must be preempted when target flees"
        );
    }

    /// Close chase: pending `ToDoGo` must not block restep when target leaves cheb 1.
    #[test]
    fn test_772_close_chase_pending_go_clears_on_target_flee() {
        use crate::creature_todo::CreatureAction;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let ppos_fled = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        world.map.register_creature_at(mpos, monster);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.base.chase_mode = ChaseMode::Close;
            m.base.walk_queue.clear();
            m.base.todo.queue.push_back(CreatureAction::Go);
            m.base.todo.locked = false;
            m.base.has_follow_path = true;
            m.base.earliest_attack_ms = world.server_ms + 2000;
        }

        world.monster_dispatch_creature_move(player, ppos, ppos_fled);

        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            base.todo.has_go(),
            "pending goal Go must be replaced with chase Go when target flees"
        );
        assert!(
            !base.todo.has_attack() || base.todo.queue.len() > 1,
            "stale single-action queue must be rebuilt for chase"
        );
    }

    /// Attack-path `TShortway` fail must NOWAY-clear target and not enqueue undeliverable Attack.
    #[test]
    fn test_chase_freeze_attack_path_noway_clears_target() {
        use crate::map::Map;
        use crate::test_world::support::{beat_driven_world, insert_monster_with_config};
        use crate::tile::{Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;

        fn sight_open_unwalkable(map: &mut Map, pos: Position) {
            map.insert_tile(
                pos,
                Tile::Normal(TileBody {
                    ground: None,
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: 0,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let mut world = beat_driven_world();
        let mpos = Position::new(100, 100, 7);
        let mid = Position::new(101, 100, 7);
        let ppos = Position::new(102, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        sight_open_unwalkable(&mut world.map, mid);
        ensure_walkable_tile(&mut world.map, ppos, 150);

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.base.chase_mode = ChaseMode::Close;
        }

        assert_eq!(
            world.monster_combat_enqueue_close_chase_go(monster),
            MonsterCombatCloseChaseEnqueue::Noway,
            "attack-path close chase must Noway when TShortway fails (C++ catch clears Target)"
        );
        let base = world.creatures.get(monster).unwrap().base();
        assert_eq!(
            base.follow_target,
            Some(player),
            "Noway return must keep chase target — caller clears it (monster_idle_noway_clear_and_roam)"
        );
        assert_eq!(base.attack_target, Some(player));
        assert!(
            !base.todo.has_attack(),
            "Noway must not leave undeliverable Attack"
        );

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.chase_mode = ChaseMode::Close;
            m.base.todo.queue.clear();
        }
        assert!(
            matches!(
                world.monster_enqueue_todo_attack_actions(monster),
                MonsterEnqueueAttackResult::Noway,
            ),
            "blocked chase must return Noway (not enqueue Attack)"
        );
        assert!(
            !world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .todo
                .has_attack(),
            "blocked chase must not leave Attack on the todo queue"
        );
    }

    /// Regression: an ATTACKING melee monster with a visible target but NO walkable path must
    /// clear its target and roam — not park indefinitely re-failing the same pathfind.
    ///
    /// C++ `IdleStimulus` catch block (`crnonpl.cc:2890-2898`): `ToDoAttack`→`CanToDoAttack`→
    /// `ToDoGo` throws NOWAY → `Target = 0` + fall through to roam tail (`crnonpl.cc:2900-2939`).
    /// Before the fix, the Rust close-chase returned `Retry` (keep target + wait 1000 ms),
    /// causing an infinite parking loop.
    #[test]
    fn test_772_attacking_no_path_roams_not_park() {
        use crate::map::Map;
        use crate::tile::{Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;

        fn sight_open_unwalkable(map: &mut Map, pos: Position) {
            map.insert_tile(
                pos,
                Tile::Normal(TileBody {
                    ground: None,
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: 0,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let mut world = beat_driven_test_world();
        // Monster at (100,100), player at (102,100), wall at (101,100) — no path.
        let mpos = Position::new(100, 100, 7);
        let wall = Position::new(101, 100, 7);
        let ppos = Position::new(102, 100, 7);
        // Roam escape tiles around the monster (so roam can step somewhere).
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(100, 99, 7), TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(100, 101, 7), TEST_SYNTHETIC_GROUND_WP);
        sight_open_unwalkable(&mut world.map, wall);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            MonsterAiConfig::default(),
        );
        world.map.register_creature_at(mpos, monster);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.base.chase_mode = ChaseMode::Close;
            m.base.walk_queue.clear();
            m.base.todo.queue.clear();
        }

        world.monster_idle_stimulus(monster);

        let base = world.creatures.get(monster).unwrap().base();
        // C++ catch clears Target on NOWAY — the monster must not keep chasing an unreachable
        // target (which caused the infinite Retry parking loop).
        assert!(
            base.follow_target.is_none(),
            "ATTACKING monster with no path must clear follow_target (C++ NOWAY catch), \
             was {:?}",
            base.follow_target
        );
        assert!(
            base.attack_target.is_none(),
            "attack_target must also be cleared on NOWAY"
        );
        // The monster must arm a wakeup (roam Go or Wait) — not be left parked with no todo.
        assert!(
            !base.todo.is_empty() || base.next_wakeup.is_some(),
            "no-path ATTACKING monster must arm a roam Go/Wait, not park with empty todo"
        );
    }

    /// Blocked mid-batch step must idle-repath instead of re-arming stale walk_queue dirs.
    #[test]
    fn test_chase_freeze_force_update_clears_stale_walk_batch() {
        use std::collections::VecDeque;
        use tfs_rust_common::enums::Direction;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = 15;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.walk_queue = VecDeque::from([Direction::East, Direction::East]);
            m.base.force_update_follow_path = true;
            m.base.todo.queue.clear();
        }

        world.finish_creature_todo_execute(monster);

        assert!(
            world.creatures.get(monster).is_some_and(|k| {
                let base = k.base();
                base.walk_queue.is_empty() || base.todo.has_go()
            }),
            "force_update after blocked step must clear stale batch or idle-repath"
        );
    }

    #[test]
    fn test_e3_attack_enqueue_succeeds_when_close_go_already_queued() {
        use std::collections::VecDeque;
        use tfs_rust_common::enums::Direction;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = 15;
            m.state = MonsterState::Attacking;
            m.base.chase_mode = ChaseMode::Close;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.walk_queue = VecDeque::from([Direction::East]);
            m.base.todo.queue.push_back(CreatureAction::Go);
        }

        assert_eq!(
            world.monster_enqueue_todo_attack_actions(monster),
            MonsterEnqueueAttackResult::Enqueued,
            "mid-batch close Go must not fail attack enqueue"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .todo
                .has_attack(),
            "Attack must append when close Go already queued"
        );
    }

    #[test]
    fn test_772_attacking_idle_tail_label_when_close_chase_skipped() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        for x in 100..=101u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Cyclops", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = 50;
            m.state = MonsterState::Attacking;
            m.base.chase_mode = ChaseMode::Close;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.todo.queue.clear();
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_enqueue_todo_attack_actions(monster),
            MonsterEnqueueAttackResult::Enqueued,
            "ATTACKING at cheb==1 must enqueue attack without close-chase Go"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .todo
                .has_attack(),
            "idle tail must append ToDoAttack when close chase is skipped"
        );
        assert!(
            world.monster_idle_skip_idle_melee_chase(monster),
            "ATTACKING posture must skip idle melee chase"
        );
    }

    #[test]
    fn test_chase_blocked_follower_rewakes_when_blocker_moves() {
        let mut world = beat_driven_test_world();
        let bpos = Position::new(100, 100, 7);
        let apos = Position::new(101, 100, 7);
        let ppos = Position::new(103, 100, 7);
        let apos_moved = Position::new(101, 101, 7);
        for pos in [bpos, apos, apos_moved, ppos] {
            ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        }
        ensure_walkable_tile(&mut world.map, Position::new(100, 101, 7), TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(102, 100, 7), TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let blocker = insert_monster(&mut world, "Rat", apos, 200);
        let follower = insert_monster(&mut world, "Rat", bpos, 200);
        for id in [blocker, follower] {
            if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(id) {
                m.is_idle = false;
                m.is_hostile = true;
                m.melee_skill = 15;
                m.opponent_ids.push(player);
                m.base.follow_target = Some(player);
                m.base.attack_target = Some(player);
                m.state = MonsterState::Attacking;
                m.base.chase_mode = ChaseMode::Close;
            }
        }
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(follower) {
            m.base.todo.queue.clear();
            m.base.walk_queue.clear();
            m.base.next_wakeup = None;
            m.base.has_follow_path = false;
        }

        world.map.register_creature_at(apos, blocker);
        world.map.unregister_creature_at(apos, blocker);
        world.map.register_creature_at(apos_moved, blocker);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(blocker) {
            m.base.position = apos_moved;
        }
        world.monster_dispatch_creature_move(blocker, apos, apos_moved);

        let base = world.creatures.get(follower).unwrap().base();
        assert!(
            base.todo.has_go() || base.next_wakeup.is_some() || !base.walk_queue.is_empty(),
            "stalled follower must re-arm chase when a blocking monster moves"
        );
    }

    fn monster_is_parked(world: &GameWorld, cid: CreatureId) -> bool {
        world.creatures.get(cid).is_some_and(|k| {
            let base = k.base();
            base.attack_target.is_some()
                && base.todo.is_empty()
                && base.walk_queue.is_empty()
                && base.next_wakeup.is_none()
        })
    }

    /// LOS blocked at cheb>1 must still arm close-chase approach — not park on bound target.
    #[test]
    fn test_772_attacking_los_blocked_does_not_freeze() {
        use crate::tile::{flags as tilestate, Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let wall = Position::new(101, 100, 7);
        let ppos = Position::new(103, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        for pos in [(100, 101), (101, 101), (102, 100), (102, 101), (103, 100)] {
            ensure_walkable_tile(&mut world.map, Position::new(pos.0, pos.1, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        world.map.insert_tile(
            wall,
            Tile::Normal(TileBody {
                ground: Some(TEST_SYNTHETIC_GROUND_WP),
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH | tilestate::UNTHROW,
                zone: ZoneType::Normal,
            }),
        );

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.base.chase_mode = ChaseMode::Close;
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
        }

        assert!(
            !world.map.is_sight_clear(mpos, ppos),
            "test setup must block LOS between monster and player"
        );

        world.monster_idle_stimulus(monster);

        assert!(
            !monster_is_parked(&world, monster),
            "ATTACKING monster with blocked LOS must still arm chase or roam, not park"
        );
    }

    /// Diverged follow/attack dest must sync and escalate to roam — not infinite Wait(200).
    #[test]
    fn test_772_close_chase_target_divergence_no_wait_loop() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(103, 100, 7);
        let decoy = Position::new(100, 103, 7);
        for x in 100..=103u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        ensure_walkable_tile(&mut world.map, decoy, TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        let decoy_player = insert_player(&mut world, test_player("Decoy", decoy));
        world.map.register_creature_at(ppos, player);
        world.map.register_creature_at(decoy, decoy_player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(decoy_player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.base.chase_mode = ChaseMode::Close;
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
        }

        world.monster_idle_stimulus(monster);

        let base = world.creatures.get(monster).unwrap().base();
        assert_eq!(
            base.follow_target,
            Some(player),
            "Attacking idle must sync follow_target to attack_target"
        );
        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_wait() && {
                    k.base()
                        .todo
                        .queue
                        .iter()
                        .any(|a| matches!(a, CreatureAction::Wait { delay_ms: 200 }))
                }),
            "diverged dest must not loop Wait(200) when off-band close chase fails"
        );
        assert!(
            !monster_is_parked(&world, monster),
            "must arm Go/roam or clear target — not park"
        );
    }

    /// ~1 Hz think rescues monsters parked on a live target with no scheduler state.
    #[test]
    fn test_772_parked_monster_rescued_by_think() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(103, 100, 7);
        for x in 100..=103u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.base.chase_mode = ChaseMode::Close;
            m.base.todo.queue.clear();
            m.base.walk_queue.clear();
            m.base.next_wakeup = None;
            m.base.has_follow_path = false;
        }
        assert!(monster_is_parked(&world, monster));

        // 772 rescue now flows through `process_creatures` (death safety only) + the
        // IdleStimulus ToDo drain — no per-creature `monster_on_think` sweep.
        world.add_creature_think_check(monster);
        world.process_creatures();
        world.request_idle_stimulus(monster);

        assert!(
            !monster_is_parked(&world, monster),
            "IdleStimulus must re-arm idle for parked combat monster"
        );
    }

    /// ATTACKING close-chase must enqueue at engagement range (cheb>8), not only strike band.
    #[test]
    fn test_772_attacking_close_chase_at_cheb11() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(111, 100, 7);
        for x in 100..=111u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Snake", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.base.chase_mode = ChaseMode::Close;
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
        }

        world.monster_idle_stimulus(monster);

        assert!(
            !monster_is_parked(&world, monster),
            "ATTACKING at cheb=11 must close-chase via attack tail, not park"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| { k.base().todo.has_go() || k.base().next_wakeup.is_some() }),
            "cheb=11 must enqueue attack-path Go"
        );
    }

    /// Attack execute `Skipped` must not leave Attack in todo without a wakeup (dead queue).
    #[test]
    fn test_772_attack_execute_skipped_reschedules_not_parks() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = 15;
            m.state = MonsterState::Attacking;
            m.base.chase_mode = ChaseMode::None;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.todo.queue.push_back(CreatureAction::Attack);
            m.base.next_wakeup = None;
        }

        world.run_monster_todo_execute(monster);

        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            base.next_wakeup.is_some() || base.todo.has_go() || !base.todo.is_empty(),
            "Skipped close-chase must reschedule todo drain, not dead-queue park"
        );
        assert!(
            !monster_is_parked(&world, monster),
            "attack execute Skipped must not park"
        );
    }

    // --- Phase 6: summon despawn / re-bind (Finding 20, `crnonpl.cc:2359–2405`) ---

    /// Helper: insert a summon monster linked to `master_id` at `pos`.
    fn insert_summon(
        world: &mut GameWorld,
        name: &str,
        pos: Position,
        master_id: CreatureId,
    ) -> CreatureId {
        let summon = insert_monster_with_config(
            world,
            name,
            pos,
            200,
            MonsterAiConfig::default(),
        );
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(summon) {
            m.base.master = Some(master_id);
        }
        summon
    }

    /// Wake a monster so it passes the sleeping+idle early return in `IdleStimulus`.
    fn wake_monster(world: &mut GameWorld, cid: CreatureId) {
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(cid) {
            m.state = MonsterState::Idle;
            m.is_idle = false;
        }
    }

    /// Summon despawns when its master is gone (removed from world).
    #[test]
    fn summon_despawns_when_master_gone() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let master = insert_monster(&mut world, "Master", mpos, 200);
        let summon = insert_summon(&mut world, "Summon", mpos, master);
        // Bypass `remove_creature`'s summon-chain cleanup — directly remove the master from the
        // SlotMap so the summon's `master` field still points to a now-gone creature. This
        // simulates the C++ path where `GetCreature(Master)` returns NULL.
        world.map.unregister_creature_at(mpos, master);
        world.creatures.remove(master);
        assert!(world.creatures.contains_key(summon));
        world.monster_idle_stimulus(summon);
        assert!(
            !world.creatures.contains_key(summon),
            "summon must despawn when master is gone"
        );
    }

    /// Summon despawns when master changes floor (non-player master, `posz != this->posz`).
    #[test]
    fn summon_despawns_on_floor_change() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let master = insert_monster(&mut world, "Master", mpos, 200);
        // Move master to floor 6 (different z).
        if let Some(k) = world.creatures.get_mut(master) {
            k.base_mut().position = Position::new(100, 100, 6);
        }
        let summon = insert_summon(&mut world, "Summon", mpos, master);
        world.monster_idle_stimulus(summon);
        assert!(
            !world.creatures.contains_key(summon),
            "summon must despawn when monster master changes floor"
        );
    }

    /// Summon despawns when straying beyond 30 tiles from master.
    #[test]
    fn summon_despawns_beyond_30_tiles() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let master = insert_monster(&mut world, "Master", mpos, 200);
        let far = Position::new(135, 100, 7); // 35 tiles away
        ensure_walkable_tile(&mut world.map, far, TEST_SYNTHETIC_GROUND_WP);
        let summon = insert_summon(&mut world, "Summon", far, master);
        world.monster_idle_stimulus(summon);
        assert!(
            !world.creatures.contains_key(summon),
            "summon must despawn when >30 tiles from master"
        );
    }

    /// Summon stays alive when within range and re-binds to master's attack target.
    #[test]
    fn summon_rebinds_to_master_attack_target_when_not_following() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let master = insert_monster(&mut world, "Master", mpos, 200);
        // Give master an attack target but no follow target (Combat.Following = false).
        let victim = insert_monster(&mut world, "Victim", Position::new(101, 100, 7), 200);
        if let Some(k) = world.creatures.get_mut(master) {
            k.base_mut().attack_target = Some(victim);
            k.base_mut().follow_target = None;
        }
        let summon = insert_summon(&mut world, "Summon", Position::new(102, 100, 7), master);
        wake_monster(&mut world, summon);
        world.monster_idle_stimulus(summon);
        assert!(world.creatures.contains_key(summon), "summon must stay alive");
        let base = world.creatures.get(summon).unwrap().base();
        assert_eq!(
            base.attack_target,
            Some(victim),
            "summon must inherit master's attack_target when master is not following"
        );
    }

    /// Summon clears target when master is following (Combat.Following = true), then falls back
    /// to master per `if (Target == 0 || Target == self) Target = Master`.
    #[test]
    fn summon_rebinds_to_master_when_target_clears() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let master = insert_monster(&mut world, "Master", mpos, 200);
        // Master is following (follow_target set) → summon Target = 0 → fallback to master.
        let follow_target = insert_monster(&mut world, "Follow", Position::new(101, 100, 7), 200);
        if let Some(k) = world.creatures.get_mut(master) {
            k.base_mut().follow_target = Some(follow_target);
        }
        let summon = insert_summon(&mut world, "Summon", Position::new(102, 100, 7), master);
        wake_monster(&mut world, summon);
        world.monster_idle_stimulus(summon);
        assert!(world.creatures.contains_key(summon));
        let base = world.creatures.get(summon).unwrap().base();
        assert_eq!(
            base.attack_target,
            Some(master),
            "summon must fall back to master when target clears"
        );
    }

    // --- Phase 6: monster Talk packet (Finding 3, `crnonpl.cc:2442–2458`) ---

    /// Monster talk emits a `0xAA` packet to spectators when the RNG gate hits.
    #[test]
    fn monster_talk_emits_packet_on_gate_hit() {
        let mpos = Position::new(100, 100, 7);

        // Try multiple seeds until the gate hits (rand_mod(50) == 0).
        let mut found_packet = false;
        for seed in 0..200u32 {
            let mut w = beat_driven_test_world();
            w.seed_parity_rng(seed);
            let mut c = MonsterAiConfig::default();
            c.talk_texts = vec!["Zzzzzz".into()];
            c.talks = 1;
            let m = insert_monster_with_config(&mut w, "Cobra", mpos, 200, c);
            // Wake the monster so it reaches the talk path (sleeping+idle returns early).
            if let Some(CreatureKind::Monster(mon)) = w.creatures.get_mut(m) {
                mon.state = MonsterState::Idle;
                mon.is_idle = false;
            }
            let cn = tfs_rust_common::ConnId(3);
            let v = insert_spectator_player(&mut w, cn, test_player("Spec", Position::new(101, 100, 7)));
            w.known_creatures_by_conn.insert(cn, std::collections::HashSet::new());
            w.pending_outgoing.clear();
            w.monster_idle_stimulus(m);
            if let Some(pkts) = w.pending_outgoing.get(&cn) {
                if pkts.iter().any(|b| !b.is_empty() && b[0] == 0xAA) {
                    found_packet = true;
                    break;
                }
            }
        }
        assert!(
            found_packet,
            "monster talk must emit a 0xAA packet on gate hit for some seed"
        );
    }

    /// Monster with no talk texts emits no packet even when the gate would hit (Talks == 0 return).
    #[test]
    fn monster_no_talk_when_talk_texts_empty() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let monster = insert_monster(&mut world, "Rat", mpos, 200); // default: talks=0, no talk_texts
        // Wake the monster so it reaches the talk path.
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::Idle;
            m.is_idle = false;
        }
        let conn = tfs_rust_common::ConnId(3);
        let viewer = insert_spectator_player(&mut world, conn, test_player("Spec", Position::new(101, 100, 7)));
        world.known_creatures_by_conn.insert(conn, std::collections::HashSet::new());
        world.pending_outgoing.clear();
        world.monster_idle_stimulus(monster);
        let pkts = world.pending_outgoing.get(&conn);
        assert!(
            !pkts.is_some_and(|p| p.iter().any(|b| !b.is_empty() && b[0] == 0xAA)),
            "monster with no talk texts must not emit 0xAA"
        );
    }

    // AI#25: monster loses target when the target stands on a house tile — C++
    // `crnonpl.cc:2427` `IsHouse(Target->posx, …)`. Uses a non-summon monster target so
    // the acquire path (`monster_idle_acquire_target`) skips it (non-summon monsters
    // are filtered at `crnonpl.cc:2500`) and doesn't re-acquire after the lose-target clear.
    #[test]
    fn test_phase9_772_loses_target_entering_house() {
        use crate::tile::{HouseTile, Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let tpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        // Insert a house tile at the target position — C++ `IsHouse`.
        world.map.insert_tile(
            tpos,
            Tile::House(HouseTile {
                inner: TileBody {
                    ground: Some(1),
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: 0,
                    zone: ZoneType::Normal,
                },
                house_id: 1,
            }),
        );

        let target = insert_monster(&mut world, "Rat", tpos, 200);
        let monster = insert_monster(&mut world, "Cyclops", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.state = MonsterState::Idle;
            m.base.follow_target = Some(target);
            m.base.attack_target = Some(target);
        }

        world.monster_idle_stimulus(monster);

        let base = world.creatures.get(monster).unwrap().base();
        assert_eq!(
            base.follow_target, None,
            "AI#25: monster must lose target on a house tile (IsHouse)"
        );
        assert_eq!(
            base.attack_target, None,
            "AI#25: monster must lose attack target on a house tile"
        );
    }

    // AI#25: monster without `SeeInvisible` loses an invisible target — C++
    // `crnonpl.cc:2429` `(Target->IsInvisible() && !RaceData[Race].SeeInvisible)`.
    #[test]
    fn test_phase9_772_loses_invisible_target_without_see_invisible() {
        use crate::condition::{add_condition_merge, ActiveCondition, ConditionData};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let tpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, tpos, TEST_SYNTHETIC_GROUND_WP);

        let target = insert_monster(&mut world, "Rat", tpos, 200);
        // Make the target invisible.
        if let Some(k) = world.creatures.get_mut(target) {
            add_condition_merge(
                &mut k.base_mut().active_conditions,
                ActiveCondition::new(
                    0,
                    0,
                    ConditionType::Invisible,
                    ConditionData::Generic { ticks: 0 },
                    None,
                ),
            );
        }
        let monster = insert_monster(&mut world, "Cyclops", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.state = MonsterState::Idle;
            // see_invisible defaults to false.
            m.base.follow_target = Some(target);
            m.base.attack_target = Some(target);
        }

        world.monster_idle_stimulus(monster);

        let base = world.creatures.get(monster).unwrap().base();
        assert_eq!(
            base.follow_target, None,
            "AI#25: monster without SeeInvisible must lose invisible target"
        );
        assert_eq!(
            base.attack_target, None,
            "AI#25: monster without SeeInvisible must lose invisible attack target"
        );
    }

    // AI#25 counterpart: monster WITH `SeeInvisible` keeps an invisible target — the
    // `(IsInvisible && !SeeInvisible)` gate does not fire.
    #[test]
    fn test_phase9_772_keeps_invisible_target_with_see_invisible() {
        use crate::condition::{add_condition_merge, ActiveCondition, ConditionData};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let tpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, tpos, TEST_SYNTHETIC_GROUND_WP);

        let target = insert_monster(&mut world, "Rat", tpos, 200);
        if let Some(k) = world.creatures.get_mut(target) {
            add_condition_merge(
                &mut k.base_mut().active_conditions,
                ActiveCondition::new(
                    0,
                    0,
                    ConditionType::Invisible,
                    ConditionData::Generic { ticks: 0 },
                    None,
                ),
            );
        }
        let monster = insert_monster_with_config(
            &mut world,
            "Cyclops",
            mpos,
            200,
            MonsterAiConfig {
                see_invisible: true,
                ..MonsterAiConfig::default()
            },
        );
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.state = MonsterState::Idle;
            m.base.follow_target = Some(target);
            m.base.attack_target = Some(target);
        }

        world.monster_idle_stimulus(monster);

        let base = world.creatures.get(monster).unwrap().base();
        assert_eq!(
            base.follow_target,
            Some(target),
            "AI#25: monster with SeeInvisible must keep invisible target"
        );
    }

    /// C++ `VictimShapeSpell` (`magic.cc:423`) checks `Actor->posz != Victim->posz` → return,
    /// and `CircleShapeSpell` (`magic.cc:522`, used by `DestinationShapeSpell`) checks
    /// `Actor->posz != DestZ` → return. A monster must NOT cast a `Victim`/`Destination`
    /// spell at a target on a different floor, even if `chebyshev` (x/y only) says it's in range.
    #[test]
    fn test_772_spell_blocked_across_z_levels() {
        use crate::creature::{MonsterAiConfig, MonsterSpell, SpellImpact, SpellShape};
        use crate::sim_harness::{
            beat_driven_test_world, ensure_walkable_tile, insert_monster_with_config,
        };

        let mut world = beat_driven_test_world();
        world.server_ms = 1000;
        // Monster on z=7, player on z=8 — same x/y (in spell range, different floor).
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(103, 100, 8);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        // Construct a Victim-shape damage spell with range 5 and delay=1 (always casts).
        let mut cfg = MonsterAiConfig::default();
        cfg.spells.push(MonsterSpell {
            delay: 1,
            range: 5,
            radius: 0,
            min_cycle: 0,
            shape: SpellShape::Victim,
            impact: SpellImpact::Damage {
                element: CombatType::Energy,
                base: 50,
                variation: 0,
            },
            shoot_effect: None,
            area_effect: None,
        });
        let monster = insert_monster_with_config(&mut world, "Rat", mpos, 200, cfg);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.state = MonsterState::Attacking;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }

        let hp_before = world.creatures.get(player).unwrap().base().health;
        // Run idle stimulus — the spell loop will attempt to cast.
        world.monster_idle_stimulus(monster);
        let hp_after = world.creatures.get(player).unwrap().base().health;

        assert_eq!(
            hp_before, hp_after,
            "monster must not cast Victim spell at a target on a different Z-level \
             (C++ VictimShapeSpell magic.cc:423 checks Actor->posz != Victim->posz)"
        );
    }

    // ===== Phase 1.4–1.5: player ToDo / idle / combat dest tests =====

    fn setup_player_world_with_conn() -> (GameWorld, CreatureId, tfs_rust_common::ConnId) {
        let mut world = beat_driven_test_world();
        let ppos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        // Lay a small arena so the player can walk and chase.
        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new(
                        (ppos.x as i32 + dx) as u16,
                        (ppos.y as i32 + dy) as u16,
                        ppos.z,
                    ),
                    TEST_SYNTHETIC_GROUND_WP,
                );
            }
        }
        let conn = tfs_rust_common::ConnId(1);
        let player = insert_spectator_player(&mut world, conn, test_player("Hero", ppos));
        world
            .known_creatures_by_conn
            .insert(conn, std::collections::HashSet::new());
        (world, player, conn)
    }

    /// Phase 1.4: single-step player walk executes via ToDo `Go` — `cract.cc:813-815`.
    #[test]
    fn test_phase1_player_single_step_walk_via_todo() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        let now = std::time::Instant::now();
        world.player_move_request(conn, player, Direction::East, now);

        // The ToDo queue should have a `Go` action.
        let base = world.creatures.get(player).unwrap().base();
        assert!(base.todo.has_go(), "player ToDo must have Go after move request");
        assert!(!base.walk_queue.is_empty(), "walk_queue must have the step");
    }

    /// Phase 1.4 / Audit #2: `player_stop_auto_walk` sets `todo_stop` (deferred) when a walk is
    /// in progress — the in-flight step lands on the next beat, then `ToDoClear + SendSnapback`
    /// (`cract.cc:1002-1008`, `:891-897`).
    #[test]
    fn test_phase1_player_stop_auto_walk_clears_todo() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        let now = std::time::Instant::now();
        world.player_move_request(conn, player, Direction::East, now);
        assert!(world.creatures.get(player).unwrap().base().todo.has_go());

        // `ToDoStop` locked branch: walk in progress → set `Stop = true` (deferred).
        world.player_stop_auto_walk(player);
        let base = world.creatures.get(player).unwrap().base();
        assert!(
            base.todo.todo_stop,
            "player todo_stop must be set when walk is in progress (cract.cc:1003-1004)"
        );

        // Advance one beat — the in-flight step lands, then `ToDoClear + SendSnapback`.
        world.pending_outgoing.clear();
        world.advance_beat(200);
        let base = world.creatures.get(player).unwrap().base();
        assert!(
            !base.todo.has_go(),
            "player ToDo must be cleared after in-flight step lands (cract.cc:891-897)"
        );
        assert!(!base.todo.todo_stop, "todo_stop must be cleared after stop completes");
        // `SendSnapback` — `0xB5` (`encode_cancel_walk`).
        let pkts = world.pending_outgoing.get(&conn).expect("must enqueue snapback");
        assert!(
            pkts.iter().any(|b| !b.is_empty() && b[0] == 0xB5),
            "stop must send 0xB5 snapback after in-flight step (cract.cc:894)"
        );
    }

    /// Audit #2: `player_stop_auto_walk` sends an immediate snapback when no walk is in progress
    /// — C++ `ToDoStop` not-locked branch (`cract.cc:1005-1006`).
    #[test]
    fn test_audit2_stop_from_standstill_sends_immediate_snapback() {
        let (mut world, player, conn) = setup_player_world_with_conn();

        // No walk in progress — `LockToDo` is false.
        world.pending_outgoing.clear();
        world.player_stop_auto_walk(player);

        let base = world.creatures.get(player).unwrap().base();
        assert!(!base.todo.todo_stop, "todo_stop must not be set when no walk is in progress");
        // Immediate `SendSnapback` — `0xB5` (`cract.cc:1005-1006`).
        let pkts = world.pending_outgoing.get(&conn).expect("must enqueue immediate snapback");
        assert!(
            pkts.iter().any(|b| !b.is_empty() && b[0] == 0xB5),
            "stop from standstill must send 0xB5 snapback immediately (cract.cc:1006)"
        );
    }

    /// Audit #2: `player_cancel_attack_and_follow` sends snapback when a pending Go is cleared —
    /// C++ `CCancelAttack`: `if(Player->ToDoClear()) SendSnapback` (`receiving.cc:1339-1341`).
    #[test]
    fn test_audit2_cancel_attack_sends_snapback_when_walk_pending() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        let now = std::time::Instant::now();
        world.player_move_request(conn, player, Direction::East, now);
        assert!(world.creatures.get(player).unwrap().base().todo.has_go());

        // `CCancelAttack` — `ToDoClear` + `SendSnapback` if pending Go.
        world.pending_outgoing.clear();
        world.player_cancel_attack_and_follow(conn, player);

        let base = world.creatures.get(player).unwrap().base();
        assert!(!base.todo.has_go(), "cancel must clear pending Go (ToDoClear)");
        // `SendSnapback` — `0xB5` (`receiving.cc:1340`).
        let pkts = world.pending_outgoing.get(&conn).expect("must enqueue snapback");
        assert!(
            pkts.iter().any(|b| !b.is_empty() && b[0] == 0xB5),
            "cancel with pending walk must send 0xB5 snapback (receiving.cc:1340)"
        );
    }

    /// Attack override: issuing `CAttack`/`CFollow` while a walk is in progress must run the
    /// C++ `ToDoAdd` `LockToDo` preamble — `ToDoClear()` the pending walk and `SendSnapback`
    /// (`0xB5`) before enqueuing the `Attack` (`cract.cc:993-1000`, `ToDoAttack` `:1353-1365`).
    /// Regression: previously the attack was appended behind the still-armed `Go`, so the player
    /// kept auto-walking the whole path and the client never resynced.
    #[test]
    fn test_attack_override_clears_pending_walk_and_snapbacks() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        let tpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, tpos, TEST_SYNTHETIC_GROUND_WP);
        let target = insert_monster(&mut world, "Rat", tpos, 200);
        let wire_id = {
            use slotmap::Key;
            (target.data().as_ffi() & 0xFFFF_FFFF) as u32
        };

        // Player starts an autowalk — pending `Go` + queued step directions.
        let now = std::time::Instant::now();
        world.player_auto_walk_path(
            conn,
            player,
            vec![Direction::East, Direction::East, Direction::East],
            now,
        );
        assert!(world.creatures.get(player).unwrap().base().todo.has_go());
        assert!(!world.creatures.get(player).unwrap().base().walk_queue.is_empty());

        // Override with an attack mid-walk.
        world.pending_outgoing.clear();
        world.player_set_attack_dest(conn, player, wire_id, false);

        let base = world.creatures.get(player).unwrap().base();
        // The pending walk must be gone: no lingering `Go`, no queued step directions.
        assert!(
            !base.todo.has_go(),
            "attack override must clear the pending Go (C++ ToDoAdd → ToDoClear)"
        );
        assert!(
            base.walk_queue.is_empty(),
            "attack override must clear the queued walk steps"
        );
        // The new action is armed.
        assert_eq!(base.attack_target, Some(target));
        assert!(base.todo.has_attack(), "attack must be enqueued after the clear");
        // The client is resynced — `SendSnapback` (`0xB5`).
        let pkts = world.pending_outgoing.get(&conn).expect("must enqueue snapback");
        assert!(
            pkts.iter().any(|b| !b.is_empty() && b[0] == 0xB5),
            "attack override with a pending walk must send 0xB5 snapback (cract.cc:993-1000)"
        );
    }

    /// Phase 1.4: `player_set_attack_dest` (Attack) sets `attack_target` and enqueues `Attack`.
    #[test]
    fn test_phase1_player_attack_sets_target_and_enqueues() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        let tpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, tpos, TEST_SYNTHETIC_GROUND_WP);
        let target = insert_monster(&mut world, "Rat", tpos, 200);

        // Resolve the target's wire id (non-player: low 32 bits of SlotMap key).
        let wire_id = {
            use slotmap::Key;
            (target.data().as_ffi() & 0xFFFF_FFFF) as u32
        };

        world.player_set_attack_dest(conn, player, wire_id, false);

        let base = world.creatures.get(player).unwrap().base();
        assert_eq!(base.attack_target, Some(target));
        assert!(base.todo.has_attack(), "player ToDo must have Attack enqueued");
    }

    /// Phase 1.4: `player_set_attack_dest` (Follow) sets `follow_target` + `ChaseMode::Close`.
    #[test]
    fn test_phase1_player_follow_sets_follow_and_close_chase() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        let tpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, tpos, TEST_SYNTHETIC_GROUND_WP);
        let target = insert_monster(&mut world, "Rat", tpos, 200);
        let wire_id = {
            use slotmap::Key;
            (target.data().as_ffi() & 0xFFFF_FFFF) as u32
        };

        world.player_set_attack_dest(conn, player, wire_id, true);

        let base = world.creatures.get(player).unwrap().base();
        assert_eq!(base.follow_target, Some(target), "follow must set follow_target");
        assert_eq!(base.chase_mode, ChaseMode::Close, "follow must set CLOSE chase");
    }

    /// Phase 1.4: `player_cancel_attack_and_follow` clears target + sends `0xA3` + stops ToDo.
    #[test]
    fn test_phase1_player_cancel_clears_target_and_sends_clear_target() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        let tpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, tpos, TEST_SYNTHETIC_GROUND_WP);
        let target = insert_monster(&mut world, "Rat", tpos, 200);
        let wire_id = {
            use slotmap::Key;
            (target.data().as_ffi() & 0xFFFF_FFFF) as u32
        };

        world.player_set_attack_dest(conn, player, wire_id, false);
        assert!(world.creatures.get(player).unwrap().base().attack_target.is_some());

        world.pending_outgoing.clear();
        world.player_cancel_attack_and_follow(conn, player);

        let base = world.creatures.get(player).unwrap().base();
        assert_eq!(base.attack_target, None, "cancel must clear attack_target");
        assert_eq!(base.follow_target, None, "cancel must clear follow_target");
        assert!(!base.todo.has_attack(), "cancel must clear ToDo Attack");

        // `SendClearTarget` — `0xA3` (`gameserver/src/protocolgame.cpp:1485-1490`).
        let pkts = world.pending_outgoing.get(&conn).expect("must enqueue clear-target");
        assert!(
            pkts.iter().any(|b| !b.is_empty() && b[0] == 0xA3),
            "cancel must send 0xA3 clear-target packet"
        );
    }

    /// Phase 1.4: `player_can_to_do_attack_chase` arms a chase `Go` when target is > 1 tile away.
    #[test]
    fn test_phase1_player_chase_arms_go_when_target_far() {
        let (mut world, player, _conn) = setup_player_world_with_conn();
        // Place target 3 tiles east — cheb = 3 > 1.
        let tpos = Position::new(103, 100, 7);
        for x in 101..=103 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let target = insert_monster(&mut world, "Rat", tpos, 200);

        if let Some(k) = world.creatures.get_mut(player) {
            k.base_mut().attack_target = Some(target);
            k.base_mut().chase_mode = ChaseMode::Close;
        }

        let outcome = world.player_can_to_do_attack_chase(player);
        assert_eq!(
            outcome,
            crate::player_combat::PlayerChaseOutcome::ChaseArmed,
            "close chase at cheb>1 must arm a Go"
        );
        let base = world.creatures.get(player).unwrap().base();
        assert!(!base.walk_queue.is_empty(), "chase must populate walk_queue");
        assert!(base.todo.has_go(), "chase must enqueue Go");
    }

    /// Phase 1.4: `player_can_to_do_attack_chase` returns `Adjacent` when target is at cheb ≤ 1.
    #[test]
    fn test_phase1_player_chase_adjacent_when_target_close() {
        let (mut world, player, _conn) = setup_player_world_with_conn();
        let tpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, tpos, TEST_SYNTHETIC_GROUND_WP);
        let target = insert_monster(&mut world, "Rat", tpos, 200);

        if let Some(k) = world.creatures.get_mut(player) {
            k.base_mut().attack_target = Some(target);
            k.base_mut().chase_mode = ChaseMode::Close;
        }

        let outcome = world.player_can_to_do_attack_chase(player);
        assert_eq!(
            outcome,
            crate::player_combat::PlayerChaseOutcome::Adjacent,
            "cheb=1 must be Adjacent (strike deferred)"
        );
    }

    /// Phase 1.4: `player_can_to_do_attack_chase` returns `TargetLost` when target is > 8 tiles away.
    #[test]
    fn test_phase1_player_chase_target_lost_when_far() {
        let (mut world, player, _conn) = setup_player_world_with_conn();
        let tpos = Position::new(110, 100, 7);
        for x in 101..=110 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let target = insert_monster(&mut world, "Rat", tpos, 200);

        if let Some(k) = world.creatures.get_mut(player) {
            k.base_mut().attack_target = Some(target);
            k.base_mut().chase_mode = ChaseMode::Close;
        }

        let outcome = world.player_can_to_do_attack_chase(player);
        assert_eq!(
            outcome,
            crate::player_combat::PlayerChaseOutcome::TargetLost,
            "cheb>8 must be TargetLost"
        );
    }

    /// Phase 1.4: `player_stop_attack` sends `0xA3` clear-target when was attacking.
    #[test]
    fn test_phase1_player_stop_attack_sends_clear_target() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        let tpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, tpos, TEST_SYNTHETIC_GROUND_WP);
        let target = insert_monster(&mut world, "Rat", tpos, 200);
        let wire_id = {
            use slotmap::Key;
            (target.data().as_ffi() & 0xFFFF_FFFF) as u32
        };

        world.player_set_attack_dest(conn, player, wire_id, false);
        world.pending_outgoing.clear();

        world.player_stop_attack(conn, player);

        let pkts = world.pending_outgoing.get(&conn).expect("must enqueue clear-target");
        assert!(
            pkts.iter().any(|b| !b.is_empty() && b[0] == 0xA3),
            "stop_attack must send 0xA3"
        );
    }

    /// Phase 1.5: drunk stagger clears ToDo + enqueues Talk "Hicks!" — `cract.cc:405-411`.
    #[test]
    fn test_phase1_drunk_stagger_clears_todo_and_enqueues_talk() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        // Set drunk: `drunkenness = 10` → `stagger_chance = max(7-10, 1) = 1` → always stagger.
        if let Some(k) = world.creatures.get_mut(player) {
            k.base_mut().drunkenness = 10;
        }

        let now = std::time::Instant::now();
        world.player_move_request(conn, player, Direction::East, now);

        // Advance past the walk delay so `on_walk` actually pops and steps.
        if let Some(k) = world.creatures.get_mut(player) {
            k.base_mut().earliest_walk_server_ms = 0;
        }

        world.pending_outgoing.clear();

        // Execute the Go action — `on_walk` runs the drunk stagger inside `Go`.
        // `drunkenness = 10` → `stagger_chance = 1` → `rand() % 1 == 0` always true.
        // The stagger enqueues `Talk("Hicks!")`, which `run_monster_todo_execute` then drains,
        // broadcasting the 0xAA speech packet.
        world.run_monster_todo_execute(player);

        // The 0xAA speech packet for "Hicks!" should be in the outgoing buffer.
        let pkts = world.pending_outgoing.get(&conn);
        assert!(
            pkts.is_some_and(|p| p.iter().any(|b| !b.is_empty() && b[0] == 0xAA)),
            "drunk stagger must broadcast 'Hicks!' (0xAA) via ToDoTalk"
        );
        // The walk_queue must be cleared (ToDoClear).
        let base = world.creatures.get(player).unwrap().base();
        assert!(
            base.walk_queue.is_empty(),
            "drunk stagger must clear walk_queue (ToDoClear)"
        );
    }

    /// Phase 1.5: `CreatureAction::Talk` execute broadcasts via `broadcast_creature_say_viewport`.
    #[test]
    fn test_phase1_talk_action_broadcasts() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        world.pending_outgoing.clear();

        let _ = world.enqueue_creature_talk(player, "Hicks!");
        world.todo_start_from_action(player, 1);
        // Execute the Talk action.
        world.run_monster_todo_execute(player);

        // The 0xAA speech packet should be in the outgoing buffer.
        let pkts = world.pending_outgoing.get(&conn);
        assert!(
            pkts.is_some_and(|p| p.iter().any(|b| !b.is_empty() && b[0] == 0xAA)),
            "Talk execute must broadcast 0xAA speech packet"
        );
    }

    /// B3 helper: place a sleeping monster at `pos` with a cleared ToDo/queue.
    fn place_sleeping_monster(world: &mut GameWorld, pos: Position, name: &str) -> CreatureId {
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let mid = insert_monster(world, name, pos, 200);
        world.map.register_creature_at(pos, mid);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mid) {
            m.state = MonsterState::Sleeping;
            m.is_idle = true;
            m.base.clear_targets();
            m.opponent_ids.clear();
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
        }
        mid
    }

    /// B3: a Player mover wakes a sleeping monster (`crnonpl.cc:2969-2975`).
    #[test]
    fn sleep_wake_wakes_for_player() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let sleeper = place_sleeping_monster(&mut world, mpos, "Sleeper");
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        world.monster_sleep_wake_on_creature_move(sleeper, player);

        let m = match world.creatures.get(sleeper) {
            Some(CreatureKind::Monster(m)) => m,
            _ => panic!("expected monster"),
        };
        assert_eq!(m.state, MonsterState::Idle, "player mover must wake sleeper");
        assert!(!m.is_idle, "wake must clear idle posture");
    }

    /// B3: a wild monster (no master) does NOT wake a sleeping monster
    /// (`crnonpl.cc:2973-2974` — `!IsPlayerControlled()` → return).
    #[test]
    fn sleep_wake_does_not_wake_for_wild_monster() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let wpos = Position::new(101, 100, 7);
        let sleeper = place_sleeping_monster(&mut world, mpos, "Sleeper");
        ensure_walkable_tile(&mut world.map, wpos, TEST_SYNTHETIC_GROUND_WP);
        let wild = insert_monster(&mut world, "WildRat", wpos, 200);
        world.map.register_creature_at(wpos, wild);
        // Wild monster: no master, hostile, with an opponent — the old band-aid
        // (`!is_summon && opponent_ids.is_empty() && !is_hostile`) would have
        // skipped wake here only because of the opponent/hostile clauses; the
        // fix keys purely on `IsPlayerControlled()`.
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(wild) {
            m.is_hostile = false;
            m.opponent_ids.clear();
        }

        world.monster_sleep_wake_on_creature_move(sleeper, wild);

        let m = match world.creatures.get(sleeper) {
            Some(CreatureKind::Monster(m)) => m,
            _ => panic!("expected monster"),
        };
        assert_eq!(
            m.state,
            MonsterState::Sleeping,
            "wild monster mover must not wake sleeper"
        );
        assert!(m.is_idle, "sleeper stays in idle posture");
    }

    /// B3: a player-owned summon wakes a sleeping monster; an NPC-owned summon
    /// does not (`IsPlayerControlled` requires `Master->Type == PLAYER`,
    /// `crnonpl.cc:3139-3146`). Guards the `is_summon()` vs `IsPlayerControlled()`
    /// distinction — `is_summon()` alone would wrongly wake for NPC summons.
    #[test]
    fn sleep_wake_wakes_for_player_summon() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        let npos = Position::new(103, 100, 7);
        let spos = Position::new(101, 100, 7);
        let sleeper = place_sleeping_monster(&mut world, mpos, "Sleeper");

        // Player-owned summon at spos.
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        ensure_walkable_tile(&mut world.map, spos, TEST_SYNTHETIC_GROUND_WP);
        let p_summon = insert_summon(&mut world, "PSummon", spos, player);
        world.map.register_creature_at(spos, p_summon);

        world.monster_sleep_wake_on_creature_move(sleeper, p_summon);
        let m = match world.creatures.get(sleeper) {
            Some(CreatureKind::Monster(m)) => m,
            _ => panic!("expected monster"),
        };
        assert_eq!(
            m.state,
            MonsterState::Idle,
            "player-owned summon mover must wake sleeper"
        );
        assert!(!m.is_idle, "wake must clear idle posture");

        // Reset sleeper and verify an NPC-owned summon does NOT wake it.
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(sleeper) {
            m.state = MonsterState::Sleeping;
            m.is_idle = true;
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
        }
        ensure_walkable_tile(&mut world.map, npos, TEST_SYNTHETIC_GROUND_WP);
        let npc_master = insert_monster(&mut world, "NpcMaster", npos, 200);
        world.map.register_creature_at(npos, npc_master);
        let n_summon = insert_summon(&mut world, "NSummon", spos, npc_master);
        // `insert_summon` registered the player summon at spos first; re-register
        // for the new occupant so the map stays consistent.
        world.map.unregister_creature_at(spos, p_summon);
        world.map.register_creature_at(spos, n_summon);

        world.monster_sleep_wake_on_creature_move(sleeper, n_summon);
        let m = match world.creatures.get(sleeper) {
            Some(CreatureKind::Monster(m)) => m,
            _ => panic!("expected monster"),
        };
        assert_eq!(
            m.state,
            MonsterState::Sleeping,
            "NPC-owned summon mover must not wake sleeper (IsPlayerControlled is false)"
        );
        assert!(m.is_idle, "sleeper stays in idle posture for NPC summon");
    }

    /// Audit #1: a fresh walk from standstill arms the wakeup at `server_ms + 1` (C++
    /// `CalculateDelay(TDGo)` leaves `Delay = 0` when `EarliestWalkTime` has elapsed, and
    /// `ToDoStart` clamps it to `1` — `cract.cc:918-923`, `:1016-1018`), **not** a full step
    /// duration. The old Rust fallback armed `get_step_duration(...)` ms into the future,
    /// adding up to one extra step of input latency to every walk started from rest.
    #[test]
    fn test_audit1_first_step_from_standstill_arms_at_one_ms() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        // `beat_driven_test_world` starts at `server_ms = 0` with `earliest_walk_server_ms = 0`
        // — the canonical "fresh walk from standstill" state.
        assert_eq!(world.server_ms, 0);
        assert_eq!(
            world.creatures.get(player).unwrap().base().earliest_walk_server_ms,
            0
        );

        let now = std::time::Instant::now();
        world.player_move_request(conn, player, Direction::East, now);

        let wakeup = world.creatures.get(player).unwrap().base().next_wakeup;
        assert!(
            wakeup.is_some(),
            "fresh walk must arm a wakeup in the ToDo heap"
        );
        // C++ clamp: `NextWakeup = ServerMilliseconds + 1` (`cract.cc:1020`).
        assert_eq!(
            wakeup,
            Some(world.server_ms + 1),
            "first step from standstill must arm at server_ms + 1 (C++ ToDoStart clamp), \
             not a full step duration"
        );
    }

    /// Audit #1 (cooldown-active case): when `EarliestWalkTime` is still in the future
    /// (set by `NotifyGo` after a prior step), the wakeup must arm at
    /// `EarliestWalkTime` — i.e. `earliest - server_ms` ms into the future
    /// (`cract.cc:919-920`). This confirms the fix did not regress the cooldown path.
    #[test]
    fn test_audit1_cooldown_active_arms_at_earliest_walk_time() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        // Simulate a recent step's `NotifyGo` (`cract.cc:1515-1525`): cooldown 400 ms out.
        const COOLDOWN_MS: u64 = 400;
        if let Some(k) = world.creatures.get_mut(player) {
            k.base_mut().earliest_walk_server_ms = world.server_ms + COOLDOWN_MS;
        }

        let now = std::time::Instant::now();
        world.player_move_request(conn, player, Direction::East, now);

        let wakeup = world.creatures.get(player).unwrap().base().next_wakeup;
        assert_eq!(
            wakeup,
            Some(world.server_ms + COOLDOWN_MS),
            "cooldown-active walk must arm at EarliestWalkTime, not server_ms + 1"
        );
    }

    // ===== Audit #3: stale walk_action cleared by new auto-walk / stop =====

    /// Helper: plant a stale `walk_action` on the player (simulates a prior walk-to-use
    /// whose action hasn't fired yet).
    fn plant_stale_walk_action(world: &mut GameWorld, player: CreatureId) {
        use crate::creature::PlayerWalkAction;
        use tfs_rust_common::game_packet::UseItemPayload;
        let action = PlayerWalkAction::UseItem(UseItemPayload {
            pos: Position::new(100, 100, 7),
            sprite_id: 100,
            stack_pos: 0,
            index: 0,
        });
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.walk_action = Some(action);
        }
    }

    /// Helper: read the player's `walk_action` (pattern-matches `CreatureKind::Player`).
    fn player_walk_action(world: &GameWorld, player: CreatureId) -> Option<crate::creature::PlayerWalkAction> {
        match world.creatures.get(player) {
            Some(CreatureKind::Player(p)) => p.walk_action.clone(),
            _ => None,
        }
    }

    /// Audit #3: `player_auto_walk_path` (client `CGoPath`) must clear a stale `walk_action`
    /// — C++ `ToDoClear()` wipes all pending entries including a queued `TDUse`/`TDMove`
    /// (`receiving.cc:120-199`, `cract.cc:953-989`). Without the clear, a prior walk-to-use
    /// fires from the wrong position after the new walk completes.
    #[test]
    fn test_audit3_auto_walk_clears_stale_walk_action() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        plant_stale_walk_action(&mut world, player);
        assert!(player_walk_action(&world, player).is_some());

        let now = std::time::Instant::now();
        world.player_auto_walk_path(conn, player, vec![Direction::East], now);

        assert!(
            player_walk_action(&world, player).is_none(),
            "player_auto_walk_path must clear stale walk_action (C++ ToDoClear, cract.cc:953-989)"
        );
    }

    /// Audit #3: `player_stop_auto_walk` (`CGoStop` → `ToDoStop`) must clear a stale
    /// `walk_action` — C++ `ToDoStop` ends in `ToDoClear` (`cract.cc:1002-1008`).
    #[test]
    fn test_audit3_stop_clears_stale_walk_action() {
        let (mut world, player, _conn) = setup_player_world_with_conn();
        plant_stale_walk_action(&mut world, player);
        assert!(player_walk_action(&world, player).is_some());

        // From standstill — not-locked branch → immediate `player_todo_clear`.
        world.player_stop_auto_walk(player);

        assert!(
            player_walk_action(&world, player).is_none(),
            "player_stop_auto_walk must clear stale walk_action (C++ ToDoStop→ToDoClear)"
        );
    }

    /// Audit #3: `try_walk_to_and_action` must preserve the newly-set `walk_action` —
    /// `player_auto_walk_path` clears stale state first, then `set_next_walk_action_task`
    /// sets the new action **after** the clear.
    #[test]
    fn test_audit3_walk_to_use_preserves_walk_action() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        // Target 2 tiles east — needs (101,100,7) + (102,100,7) walkable.
        let target = Position::new(102, 100, 7);
        ensure_walkable_tile(&mut world.map, target, TEST_SYNTHETIC_GROUND_WP);
        // Plant a stale action from a *prior* walk-to-use.
        plant_stale_walk_action(&mut world, player);

        use crate::creature::PlayerWalkAction;
        use tfs_rust_common::game_packet::UseItemPayload;
        let new_action = PlayerWalkAction::UseItem(UseItemPayload {
            pos: target,
            sprite_id: 200,
            stack_pos: 0,
            index: 0,
        });
        let now = std::time::Instant::now();
        let ok = world.try_walk_to_and_action(conn, player, target, new_action.clone(), now);
        assert!(ok, "try_walk_to_and_action must find a path to the target");

        let preserved = player_walk_action(&world, player);
        assert!(
            preserved.is_some(),
            "walk_to_use must preserve the new walk_action after the internal ToDoClear"
        );
        // The preserved action must be the NEW one, not the stale one.
        match preserved {
            Some(PlayerWalkAction::UseItem(u)) => {
                assert_eq!(u.pos, target, "preserved walk_action must be the new one");
                assert_eq!(u.sprite_id, 200);
            }
            other => panic!("expected UseItem walk_action, got {other:?}"),
        }
    }

    // ===== F8 S7: walk_action deferral branch removed for 772 =====

    // ===== Phase 5: walk_action_due + on_player_walk_complete deleted =====
    // The F8 S7 tests (`test_f8_s7_on_walk_complete_noop_for_beat_driven`,
    // `test_f8_s7_process_creature_todo_ignores_walk_action_for_beat_driven`) verified the
    // no-op transition; both the field and the function are now deleted.

    // ===== Audit #6: on_walk gate uses earliest_walk_server_ms (single source of truth) =====

    /// Audit #6: the `on_walk` cooldown gate on the beat path must derive from
    /// `earliest_walk_server_ms` (C++ `EarliestWalkTime`, fixed by `NotifyGo` at
    /// step-completion — `cract.cc:1515-1525`), NOT from a recomputation that reads
    /// current speed/conditions. Repro: take a step, then paralyze the player (halve
    /// speed) between steps. The recomputed `get_walk_delay_logical` would block the
    /// next step even though `EarliestWalkTime` has elapsed; the single-source gate
    /// lets it fire on time.
    #[test]
    fn test_audit6_on_walk_gate_uses_earliest_walk_time() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        // Extend the arena east so a 2-step walk can land.
        ensure_walkable_tile(
            &mut world.map,
            Position::new(102, 100, 7),
            TEST_SYNTHETIC_GROUND_WP,
        );
        // Player starts at speed 220 (GoStrength) → effective 520 → cardinal step on
        // 150-waypoint ground = 288 ms → ceil to beat 200 = 400 ms.
        assert_eq!(world.creatures.get(player).unwrap().base().speed, 220);

        let now = std::time::Instant::now();
        world.player_auto_walk_path(conn, player, vec![Direction::East, Direction::East], now);

        // First step fires at server_ms = 1 (C++ ToDoStart clamp). After it lands,
        // `earliest_walk_server_ms = 1 + 400 = 401`.
        world.advance_beat(1);
        assert_eq!(
            world.creatures.get(player).map(|k| k.position()),
            Some(Position::new(101, 100, 7)),
            "first step must land at server_ms = 1"
        );
        let earliest = world.creatures.get(player).unwrap().base().earliest_walk_server_ms;
        assert_eq!(earliest, 401, "NotifyGo sets earliest_walk_server_ms = 1 + 400");

        // Paralyze the player mid-cooldown (speed 42 → effective 164). The old
        // recomputation would yield a 1000 ms completed-step duration, blocking the
        // second step at server_ms = 401 (delay = 1000 - 400 = 600 > 0).
        if let Some(k) = world.creatures.get_mut(player) {
            k.base_mut().speed = 42;
        }

        // Advance to `earliest_walk_server_ms` — the second step must fire.
        world.advance_beat(400);
        assert_eq!(
            world.server_ms, 401,
            "server_ms must reach earliest_walk_server_ms"
        );
        assert_eq!(
            world.creatures.get(player).map(|k| k.position()),
            Some(Position::new(102, 100, 7)),
            "second step must fire at earliest_walk_server_ms even after mid-cooldown \
             speed change (C++ EarliestWalkTime is fixed at step-completion, cract.cc:1515-1525)"
        );
    }

    // ===== Audit #4: absolute-destination walk queue — push mid-walk aborts remaining path =====

    /// Audit #4: C++ `TDGo` stores absolute coordinates (`receiving.cc:141-160`); if the player
    /// is pushed mid-auto-walk, the next `TDGo`'s stored dest is no longer adjacent →
    /// `Go` throws `NOTACCESSIBLE` (`cract.cc:386-389`) → `Execute` catch sends
    /// `SendResult("Sorry, not possible.")` + `SendSnapback` + `ToDoClear` + `ToDoYield`
    /// (`cract.cc:870-889`). Rust stores `Direction`s; the `walk_destinations` overlay lets
    /// `on_walk` detect the divergence and abort instead of silently replaying the path
    /// offset by the push delta.
    #[test]
    fn test_audit4_push_mid_auto_walk_aborts_remaining_path() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        // Extend the arena east: (102,100,7) and (103,100,7).
        ensure_walkable_tile(&mut world.map, Position::new(102, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(103, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        // Also make (101,102,7) walkable — the push landing tile.
        ensure_walkable_tile(&mut world.map, Position::new(101, 102, 7), TEST_SYNTHETIC_GROUND_WP);

        // Queue a 3-step auto-walk east from (100,100,7).
        // Destinations: (101,100,7), (102,100,7), (103,100,7).
        let now = std::time::Instant::now();
        world.player_auto_walk_path(
            conn,
            player,
            vec![Direction::East, Direction::East, Direction::East],
            now,
        );
        // Verify destinations were populated — in pop_back order (execution order):
        // first step's dest is at the back of the queue.
        let dests: Vec<_> = world
            .creatures
            .get(player)
            .unwrap()
            .base()
            .walk_destinations
            .iter()
            .rev()
            .copied()
            .collect();
        assert_eq!(
            dests,
            vec![
                Position::new(101, 100, 7),
                Position::new(102, 100, 7),
                Position::new(103, 100, 7),
            ],
            "walk_destinations pop_back order must match execution order (receiving.cc:141-160)"
        );

        // First step fires at server_ms = 1 (C++ ToDoStart clamp) → lands at (101,100,7).
        world.advance_beat(1);
        assert_eq!(
            world.creatures.get(player).map(|k| k.position()),
            Some(Position::new(101, 100, 7)),
            "first step must land"
        );

        // Simulate a push: teleport the player 2 tiles south to (101,102,7).
        // The next stored dest (102,100,7) is now 2 tiles away → NOTACCESSIBLE.
        if let Some(k) = world.creatures.get_mut(player) {
            k.set_position(Position::new(101, 102, 7));
        }

        // Advance to the earliest_walk_server_ms — the second step attempts to fire.
        let earliest = world.creatures.get(player).unwrap().base().earliest_walk_server_ms;
        let advance = earliest.saturating_sub(world.server_ms);
        world.pending_outgoing.clear();
        world.advance_beat(advance);

        // The adjacency check must abort: player stays at the pushed position.
        assert_eq!(
            world.creatures.get(player).map(|k| k.position()),
            Some(Position::new(101, 102, 7)),
            "pushed player must NOT step toward the stale destination (cract.cc:386-389)"
        );
        // Queue must be cleared (ToDoClear — cract.cc:871).
        let base = world.creatures.get(player).unwrap().base();
        assert!(
            base.walk_queue.is_empty(),
            "walk_queue must be cleared after NOTACCESSIBLE abort"
        );
        assert!(
            base.walk_destinations.is_empty(),
            "walk_destinations must be cleared after NOTACCESSIBLE abort"
        );
        assert!(!base.todo.has_go(), "ToDo Go must be cleared after abort");
        // SendSnapback (0xB5) must be sent (Execute catch — cract.cc:881-886).
        let pkts = world
            .pending_outgoing
            .get(&conn)
            .expect("must enqueue snapback + message");
        assert!(
            pkts.iter().any(|b| !b.is_empty() && b[0] == 0xB5),
            "NOTACCESSIBLE abort must send 0xB5 snapback (cract.cc:885)"
        );
    }

    /// Audit #4: a normal (un-pushed) auto-walk must still complete — the adjacency check
    /// passes when the player is at the expected origin for each step.
    #[test]
    fn test_audit4_unpushed_auto_walk_completes_normally() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        ensure_walkable_tile(&mut world.map, Position::new(102, 100, 7), TEST_SYNTHETIC_GROUND_WP);

        let now = std::time::Instant::now();
        world.player_auto_walk_path(conn, player, vec![Direction::East, Direction::East], now);

        // First step at server_ms = 1 → (101,100,7).
        world.advance_beat(1);
        assert_eq!(
            world.creatures.get(player).map(|k| k.position()),
            Some(Position::new(101, 100, 7)),
        );

        // Second step at earliest_walk_server_ms → (102,100,7).
        let earliest = world.creatures.get(player).unwrap().base().earliest_walk_server_ms;
        let advance = earliest.saturating_sub(world.server_ms);
        world.advance_beat(advance);
        assert_eq!(
            world.creatures.get(player).map(|k| k.position()),
            Some(Position::new(102, 100, 7)),
            "un-pushed auto-walk must complete normally (adjacency check passes)"
        );
        let base = world.creatures.get(player).unwrap().base();
        assert!(base.walk_queue.is_empty(), "walk_queue must drain on completion");
        assert!(
            base.walk_destinations.is_empty(),
            "walk_destinations must drain on completion"
        );
    }

    /// Reproduction: auto-walk → arrow key override → new auto-walk must complete
    /// all steps (not just 1 tile). Tests that `player_todo_clear` properly resets
    /// all state that `finish_creature_todo_execute` checks for step chaining.
    #[test]
    fn test_arrow_override_does_not_break_subsequent_auto_walk() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        // Extend arena east: (102,100,7) through (105,100,7) and south (100,101,7).
        for x in 102..=105 {
            ensure_walkable_tile(
                &mut world.map,
                Position::new(x, 100, 7),
                TEST_SYNTHETIC_GROUND_WP,
            );
        }
        ensure_walkable_tile(
            &mut world.map,
            Position::new(101, 101, 7),
            TEST_SYNTHETIC_GROUND_WP,
        );

        let now = std::time::Instant::now();

        // 1) Start a 3-step auto-walk east from (100,100,7).
        world.player_auto_walk_path(
            conn,
            player,
            vec![Direction::East, Direction::East, Direction::East],
            now,
        );

        // 2) First step fires at server_ms = 1 → (101,100,7).
        world.advance_beat(1);
        assert_eq!(
            world.creatures.get(player).map(|k| k.position()),
            Some(Position::new(101, 100, 7)),
            "auto-walk first step must land"
        );

        // 3) Override with an arrow key (south) while auto-walk is in progress.
        world.player_move_request(conn, player, Direction::South, now);

        // 4) Advance to let the arrow step fire.
        let earliest = world.creatures.get(player).unwrap().base().earliest_walk_server_ms;
        let advance = earliest.saturating_sub(world.server_ms).max(1);
        world.advance_beat(advance);
        assert_eq!(
            world.creatures.get(player).map(|k| k.position()),
            Some(Position::new(101, 101, 7)),
            "arrow key override step must land"
        );

        // 5) Start a NEW 3-step auto-walk east from (101,101,7).
        //    Destinations: (102,101,7), (103,101,7), (104,101,7).
        //    Need walkable tiles south of the east corridor.
        for x in 102..=104 {
            ensure_walkable_tile(
                &mut world.map,
                Position::new(x, 101, 7),
                TEST_SYNTHETIC_GROUND_WP,
            );
        }
        world.player_auto_walk_path(
            conn,
            player,
            vec![Direction::East, Direction::East, Direction::East],
            now,
        );

        // 6) Advance beats — ALL 3 steps must land, not just 1.
        let mut expected_x = 102;
        for step in 1..=3u32 {
            let earliest = world.creatures.get(player).unwrap().base().earliest_walk_server_ms;
            let advance = earliest.saturating_sub(world.server_ms).max(1);
            world.advance_beat(advance);
            let pos = world.creatures.get(player).map(|k| k.position()).unwrap();
            assert_eq!(
                pos,
                Position::new(expected_x, 101, 7),
                "new auto-walk step {} must land at ({},101,7), got {:?}",
                step,
                expected_x,
                pos,
            );
            expected_x += 1;
        }

        // Verify no stuck state.
        let base = world.creatures.get(player).unwrap().base();
        assert!(
            base.walk_queue.is_empty(),
            "walk_queue must drain after auto-walk completes"
        );
        assert!(!base.todo.has_go(), "todo Go must be drained after completion");
    }

    /// Regression: a rejected step (blocked tile) must NOT strand subsequent
    /// auto-walks at 1 tile per move. The bug was that `on_walk_step_rejected`
    /// set `force_update_follow_path = true` for ALL ToDo creatures including
    /// players, but that flag is a monster chase-repath concept —
    /// `finish_creature_todo_execute` clears `walk_queue` when it's set, and
    /// `monster_idle_stimulus` (the only clearer) is a no-op for players.
    /// C++ `Execute` catch (`cract.cc:870-889`) only calls `ToDoClear + ToDoYield`
    /// — it does NOT set any follow-path flag.
    #[test]
    fn test_rejected_step_does_not_strand_subsequent_auto_walk() {
        let (mut world, player, conn) = setup_player_world_with_conn();
        // Extend arena east: (102,100,7) through (105,100,7).
        for x in 102..=105 {
            ensure_walkable_tile(
                &mut world.map,
                Position::new(x, 100, 7),
                TEST_SYNTHETIC_GROUND_WP,
            );
        }

        let now = std::time::Instant::now();

        // 1) Place a monster on (102,100,7) to block the first east step.
        let _blocker = insert_monster(
            &mut world,
            "Rat",
            Position::new(102, 100, 7),
            200,
        );

        // 2) Start a 3-step auto-walk east from (100,100,7).
        world.player_auto_walk_path(
            conn,
            player,
            vec![Direction::East, Direction::East, Direction::East],
            now,
        );

        // 3) First step fires at server_ms = 1 → tries to step to (101,100,7).
        //    This step succeeds (101 is walkable). The second step would try
        //    (102,100,7) which is blocked by the monster.
        world.advance_beat(1);
        assert_eq!(
            world.creatures.get(player).map(|k| k.position()),
            Some(Position::new(101, 100, 7)),
            "first step must land"
        );

        // 4) Advance to the second step — it should be REJECTED (blocked by monster).
        let earliest = world.creatures.get(player).unwrap().base().earliest_walk_server_ms;
        let advance = earliest.saturating_sub(world.server_ms).max(1);
        world.advance_beat(advance);
        // Player stays at (101,100,7) — the step was rejected.
        assert_eq!(
            world.creatures.get(player).map(|k| k.position()),
            Some(Position::new(101, 100, 7)),
            "second step must be rejected (blocked by monster)"
        );

        // 5) Verify `force_update_follow_path` is NOT set for a player.
        let base = world.creatures.get(player).unwrap().base();
        assert!(
            !base.force_update_follow_path,
            "force_update_follow_path must NOT be set for players after rejected step \
             (cract.cc:870-889 does not set any follow-path flag)"
        );

        // 6) Remove the blocker and start a NEW 3-step auto-walk east.
        //    ALL 3 steps must land — the prior rejection must not strand the walk.
        //    Remove the monster by killing it.
        if let Some(k) = world.creatures.get_mut(_blocker) {
            k.base_mut().health = 0;
        }
        world.apply_creature_death(_blocker);

        world.player_auto_walk_path(
            conn,
            player,
            vec![Direction::East, Direction::East, Direction::East],
            now,
        );

        let mut expected_x = 102u16;
        for step in 1..=3u32 {
            let earliest = world.creatures.get(player).unwrap().base().earliest_walk_server_ms;
            let advance = earliest.saturating_sub(world.server_ms).max(1);
            world.advance_beat(advance);
            let pos = world.creatures.get(player).map(|k| k.position()).unwrap();
            assert_eq!(
                pos,
                Position::new(expected_x, 100, 7),
                "new auto-walk step {} must land at ({},100,7), got {:?} — \
                 rejected step must not strand subsequent walks",
                step,
                expected_x,
                pos,
            );
            expected_x += 1;
        }

        let base = world.creatures.get(player).unwrap().base();
        assert!(
            base.walk_queue.is_empty(),
            "walk_queue must drain after auto-walk completes"
        );
        assert!(!base.force_update_follow_path, "flag must remain clear for players");
    }

    // === F8 S3 — CalculateDelay multiuse gate integration tests ===
    // C++ ref: `cract.cc:901-960` `CalculateDelay`, `cract.cc:795-801` `Execute` drain
    // "Delay > 0 → schedule + break". Two-object `Use` defers on `EarliestMultiuseTime`;
    // single-object `Use` is ungated (`cract.cc:925-932`).

    /// Place a bag (container, id 1987) on a tile and return its `ActionObjectRef`.
    /// Mirrors `creature_todo.rs::place_bag_on_tile` — `sprite_id=0` matches the
    /// default `client_id=0` in the test items_db.
    fn place_bag_on_tile_772(
        world: &mut GameWorld,
        pos: Position,
    ) -> crate::creature_todo::ActionObjectRef {
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let item_id = world.items.insert(crate::item::Item::new_single(1987));
        world
            .map
            .get_tile_mut(pos)
            .expect("tile just inserted")
            .add_item(item_id);
        crate::creature_todo::ActionObjectRef {
            pos,
            stack_pos: 0,
            sprite_id: 0,
        }
    }

    /// Two-object `Use` within the multiuse gate defers: the `Use` action is pushed
    /// back to the front and a wakeup is armed at `earliest_multiuse_server_ms`.
    #[test]
    fn two_object_use_within_multiuse_gate_defers() {
        let mut world = beat_driven_test_world();
        world.server_ms = 1000;
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let item_pos2 = Position::new(102, 100, 7);
        ensure_walkable_tile(&mut world.map, player_pos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", player_pos));
        world.map.register_creature_at(player_pos, player);
        let obj1 = place_bag_on_tile_772(&mut world, item_pos);
        let obj2 = place_bag_on_tile_772(&mut world, item_pos2);

        world
            .enqueue_player_use(player, obj1, Some(obj2), 0)
            .expect("both bags resolve");
        // Queue: [Wait{100}, Use{obj2:Some}]. Arm multiuse exhaustion 1000 ms ahead.
        if let Some(k) = world.creatures.get_mut(player) {
            k.base_mut().earliest_multiuse_server_ms = 2000;
        }

        // 1) Execute the Wait{100} — schedules wakeup at server_ms+100=1100.
        let kind = world.execute_creature_todo_action(player);
        assert!(
            matches!(kind, Some(crate::idle_stimulus::TodoExecuteKind::Wait)),
            "Wait{{100}} executes first"
        );
        // Queue is now [Use{obj2:Some}].
        assert_eq!(
            world
                .creatures
                .get(player)
                .unwrap()
                .base()
                .todo
                .queue
                .len(),
            1,
            "Wait consumed, Use remains"
        );

        // 2) Advance past the Wait floor to the multiuse-gated execute.
        world.server_ms = 1100;
        // Clear the wakeup armed by the Wait so the next execute isn't blocked by
        // process_creature_todo's `next_wakeup` gate (we call execute directly here).
        if let Some(k) = world.creatures.get_mut(player) {
            k.base_mut().next_wakeup = None;
        }

        let kind = world.execute_creature_todo_action(player);
        assert!(
            matches!(kind, Some(crate::idle_stimulus::TodoExecuteKind::Deferred)),
            "two-object Use within gate must defer (CalculateDelay > 0)"
        );

        // Use was pushed back to the front; wakeup armed at earliest_multiuse_server_ms.
        let base = world.creatures.get(player).unwrap().base();
        assert_eq!(
            base.todo.queue.len(),
            1,
            "deferred Use must be pushed back to the front"
        );
        assert!(
            matches!(base.todo.queue.front(), Some(CreatureAction::Use { .. })),
            "front must still be the Use action"
        );
        assert_eq!(
            base.next_wakeup,
            Some(2000),
            "wakeup armed at earliest_multiuse_server_ms (server_ms + delay = 1100 + 900)"
        );
    }

    /// Single-object `Use` does not defer — the gate only applies to `Obj2 != 0`
    /// (`cract.cc:926`). The stub executor fires (queue drains, no deferral).
    #[test]
    fn single_object_use_does_not_defer() {
        let mut world = beat_driven_test_world();
        world.server_ms = 1000;
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, player_pos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", player_pos));
        world.map.register_creature_at(player_pos, player);
        let obj1 = place_bag_on_tile_772(&mut world, item_pos);

        world
            .enqueue_player_use(player, obj1, None, 0)
            .expect("bag resolves");
        // Queue: [Wait{100}, Use{obj2:None}]. Arm multiuse exhaustion far in the future
        // — single-object Use must still be ungated.
        if let Some(k) = world.creatures.get_mut(player) {
            k.base_mut().earliest_multiuse_server_ms = 5000;
        }

        // 1) Execute the Wait{100}.
        let kind = world.execute_creature_todo_action(player);
        assert!(
            matches!(kind, Some(crate::idle_stimulus::TodoExecuteKind::Wait)),
            "Wait{{100}} executes first"
        );

        // 2) Advance past the Wait floor.
        world.server_ms = 1100;
        if let Some(k) = world.creatures.get_mut(player) {
            k.base_mut().next_wakeup = None;
        }

        let kind = world.execute_creature_todo_action(player);
        // Single-object Use is ungated → stub fires (S4 replaces with real executor).
        assert!(
            matches!(kind, Some(crate::idle_stimulus::TodoExecuteKind::Wait)),
            "single-object Use does not defer (stub fires, S4 wires executor)"
        );

        // Queue drained — Use was consumed, not pushed back.
        let base = world.creatures.get(player).unwrap().base();
        assert!(
            base.todo.queue.is_empty(),
            "single-object Use must not be pushed back (no deferral)"
        );
        assert_eq!(
            base.next_wakeup,
            None,
            "no wakeup armed — single-object Use is ungated"
        );
    }

    // ===== OTClient-on-772 floor-change dispatch tests =====
    //
    // OTClient tracks the local player as a tile creature and cannot reconcile the
    // decompile `NotifyGo` incremental `SendFloors`/`SendRow` stream after the leading
    // `0x6D` pre-jumps the self to the final tile. The fix routes OTClient-on-772
    // floor changes through TVP's per-segment teleport path (remove + `0x64`), while
    // the real 7.72 client keeps the decompile `NotifyGo` incremental path.
    // See `docs/772_FLOOR_CHANGE_CLIENT_TARGETS.md` §6.

    use tfs_rust_common::CLIENTOS_OTCLIENT_LINUX;

    /// Set up a south-facing stair at (100,100,8) — queryDestination sends the
    /// climber to (100,101,7). Returns (world, player_id, conn_id).
    fn setup_south_stair_world(
        player_start: Position,
        operating_system: u16,
    ) -> (GameWorld, CreatureId, tfs_rust_common::ConnId) {
        let mut world = beat_driven_test_world();
        // Stair tile at (100,100,8) with FLOORCHANGE_SOUTH.
        use crate::tile::{flags as tilestate, Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;
        world.map.insert_tile(
            Position::new(100, 100, 8),
            Tile::Normal(TileBody {
                ground: Some(TEST_SYNTHETIC_GROUND_WP),
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: tilestate::FLOORCHANGE_SOUTH,
                zone: ZoneType::Normal,
            }),
        );
        // Destination tile after queryDestination chain: (100,101,7).
        ensure_walkable_tile(&mut world.map, Position::new(100, 101, 7), TEST_SYNTHETIC_GROUND_WP);
        // Player start tile.
        ensure_walkable_tile(&mut world.map, player_start, TEST_SYNTHETIC_GROUND_WP);

        let conn = tfs_rust_common::ConnId(1);
        let mut player = test_player("Hero", player_start);
        player.operating_system = operating_system;
        let player = insert_spectator_player(&mut world, conn, player);
        world
            .known_creatures_by_conn
            .insert(conn, std::collections::HashSet::new());
        (world, player, conn)
    }

    /// OTClient-on-772: walking west onto south-facing stairs must emit the TVP
    /// teleport path (`0x6C` remove + `0x64` full screen) for the z-change segment,
    /// NOT the decompile `NotifyGo` incremental path (`0x6D` + `0xBE` + rows).
    /// This is the perpendicular-approach repro from the bug report.
    #[test]
    fn otclient_772_west_onto_south_stairs_uses_teleport_path() {
        let (mut world, player, conn) =
            setup_south_stair_world(Position::new(101, 100, 8), CLIENTOS_OTCLIENT_LINUX);
        let now = std::time::Instant::now();
        world.player_move_request(conn, player, Direction::West, now);

        // Advance beats until the player reaches (100,101,7) or timeout.
        for _ in 0..10 {
            if world.creatures.get(player).map(|k| k.position())
                == Some(Position::new(100, 101, 7))
            {
                break;
            }
            let earliest = world
                .creatures
                .get(player)
                .unwrap()
                .base()
                .earliest_walk_server_ms;
            let advance = earliest.saturating_sub(world.server_ms).max(1);
            world.advance_beat(advance);
        }
        assert_eq!(
            world.creatures.get(player).map(|k| k.position()),
            Some(Position::new(100, 101, 7)),
            "OTClient player must reach (100,101,7) via south stair"
        );

        let packets = world
            .pending_outgoing
            .get(&conn)
            .cloned()
            .unwrap_or_default();
        // The z-change segment must go through emit_teleport_move_packet:
        //   0x6C (remove) + 0x64 (full screen map description).
        assert!(
            packets.iter().any(|p| !p.is_empty() && p[0] == 0x64),
            "OTClient-on-772 z-change must emit 0x64 full screen (teleport path), \
             not NotifyGo incremental"
        );
        assert!(
            packets.iter().any(|p| !p.is_empty() && p[0] == 0x6C),
            "OTClient-on-772 z-change must emit 0x6C remove (teleport path)"
        );
        // Must NOT emit the NotifyGo incremental floor-change stream. The 0x6D
        // self-packet from segment 1 (same-z step onto the stair) is expected,
        // but no 0xBE/0xBF floor-change opcodes anywhere (those only appear in
        // the NotifyGo path, which is suppressed for OTClient).
        let has_floor_change_opcode = packets.iter().any(|p| {
            p.iter().any(|&b| b == 0xBE || b == 0xBF)
        });
        assert!(
            !has_floor_change_opcode,
            "OTClient-on-772 must NOT emit 0xBE/0BF (NotifyGo incremental path) \
             — the z-change must be a full-screen 0x64 teleport"
        );
    }

    /// Real 7.72 client (non-OTClient): walking west onto south-facing stairs must
    /// emit the decompile `NotifyGo` incremental path (`0x6D` + `0xBE` + rows),
    /// NOT the TVP teleport path (`0x6C` + `0x64`). This is the regression guard
    /// for the real-client contract after the OTClient dispatch fix.
    #[test]
    fn real_772_client_west_onto_south_stairs_uses_notify_go() {
        let (mut world, player, conn) =
            setup_south_stair_world(Position::new(101, 100, 8), 0);
        let now = std::time::Instant::now();
        world.player_move_request(conn, player, Direction::West, now);

        for _ in 0..10 {
            if world.creatures.get(player).map(|k| k.position())
                == Some(Position::new(100, 101, 7))
            {
                break;
            }
            let earliest = world
                .creatures
                .get(player)
                .unwrap()
                .base()
                .earliest_walk_server_ms;
            let advance = earliest.saturating_sub(world.server_ms).max(1);
            world.advance_beat(advance);
        }
        assert_eq!(
            world.creatures.get(player).map(|k| k.position()),
            Some(Position::new(100, 101, 7)),
            "real 772 client must reach (100,101,7) via south stair"
        );

        let packets = world
            .pending_outgoing
            .get(&conn)
            .cloned()
            .unwrap_or_default();
        // The real 772 client uses NotifyGo: a single packet with 0x6D self-packet
        // followed by 0xBE/0xBF + rows. The 0xBE/0xBF is embedded inside the
        // packet (after the 12-byte self-packet), not at position 0.
        let has_floor_change = packets.iter().any(|p| {
            p.iter().any(|&b| b == 0xBE || b == 0xBF)
        });
        assert!(
            has_floor_change,
            "real 772 client must emit 0xBE/0xBF (NotifyGo incremental floor change)"
        );
        // Must NOT emit 0x64 full screen (that's the OTClient/teleport path).
        assert!(
            !packets.iter().any(|p| !p.is_empty() && p[0] == 0x64),
            "real 772 client must NOT emit 0x64 full screen (that's the teleport path)"
        );
    }

    /// TVP `skip_remove` parity: when leaving the surface (oldPos.z == 7) in ANY
    /// direction — up to z=6 OR down to z=8 — TVP skips the `0x6C` remove and
    /// emits only `0x64` full screen. The Rust code previously used `&&` (only
    /// skip for z=7→z=8), sending a spurious remove on z=7→z=6 that caused
    /// OTClient errors. This test locks the `||` condition.
    /// TVP ref: `protocolgame.cpp:1770` — `if (newPos.z != 8 && oldPos.z != 7)`.
    #[test]
    fn otclient_772_up_from_surface_skips_remove() {
        let (mut world, player, conn) =
            setup_south_stair_world(Position::new(101, 100, 8), CLIENTOS_OTCLIENT_LINUX);
        // Modify the stair to go UP instead: place a north-facing stair at z=7
        // that chains to z=6. We need tiles at z=6 for the destination.
        use crate::tile::{flags as tilestate, Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;
        // Replace the z=8 stair with a z=7 stair going north (up to z=6).
        world.map.insert_tile(
            Position::new(100, 100, 7),
            Tile::Normal(TileBody {
                ground: Some(TEST_SYNTHETIC_GROUND_WP),
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: tilestate::FLOORCHANGE_NORTH,
                zone: ZoneType::Normal,
            }),
        );
        // Destination after queryDestination: (100, 99, 6).
        ensure_walkable_tile(&mut world.map, Position::new(100, 99, 6), TEST_SYNTHETIC_GROUND_WP);
        // Move player to (101, 100, 7) — east of the stair, same z.
        ensure_walkable_tile(&mut world.map, Position::new(101, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        if let Some(k) = world.creatures.get_mut(player) {
            k.set_position(Position::new(101, 100, 7));
        }
        world.map.unregister_creature_at(Position::new(101, 100, 8), player);
        world.map.register_creature_at(Position::new(101, 100, 7), player);

        let now = std::time::Instant::now();
        world.player_move_request(conn, player, Direction::West, now);

        for _ in 0..10 {
            if world.creatures.get(player).map(|k| k.position())
                == Some(Position::new(100, 99, 6))
            {
                break;
            }
            let earliest = world
                .creatures
                .get(player)
                .unwrap()
                .base()
                .earliest_walk_server_ms;
            let advance = earliest.saturating_sub(world.server_ms).max(1);
            world.advance_beat(advance);
        }
        assert_eq!(
            world.creatures.get(player).map(|k| k.position()),
            Some(Position::new(100, 99, 6)),
            "OTClient player must reach (100,99,6) via north stair up from surface"
        );

        let packets = world
            .pending_outgoing
            .get(&conn)
            .cloned()
            .unwrap_or_default();
        // Must emit 0x64 full screen (teleport path).
        assert!(
            packets.iter().any(|p| !p.is_empty() && p[0] == 0x64),
            "OTClient z-change must emit 0x64 full screen"
        );
        // Must NOT emit 0x6C remove — TVP skips when oldPos.z == 7 (leaving surface
        // in any direction). The 0x64 full screen redraw handles the relocation.
        assert!(
            !packets.iter().any(|p| !p.is_empty() && p[0] == 0x6C),
            "OTClient z=7→z=6 must NOT emit 0x6C remove — TVP skips when oldPos.z == 7"
        );
    }

    /// Wire order: the `0x6B` chain turn must be emitted AFTER the move packets
    /// (`0x64`/`0x6D`/`0x6C`), not before. C++ order: `Map::moveCreature` sends
    /// `sendMoveCreature` during the move loop (`map.cpp:316`), THEN
    /// `internalCreatureTurn` sends `0x6B` after the loop (`game.cpp:888`).
    /// Rust previously emitted `0x6B` inside `internal_move_creature_step` (before
    /// move packets in `on_walk`), causing the client to receive `0x6B` for a
    /// position it hasn't seen the creature move to yet.
    #[test]
    fn otclient_772_chain_turn_emitted_after_move_packets() {
        let (mut world, player, conn) =
            setup_south_stair_world(Position::new(101, 100, 8), CLIENTOS_OTCLIENT_LINUX);
        let now = std::time::Instant::now();
        world.player_move_request(conn, player, Direction::West, now);

        for _ in 0..10 {
            if world.creatures.get(player).map(|k| k.position())
                == Some(Position::new(100, 101, 7))
            {
                break;
            }
            let earliest = world
                .creatures
                .get(player)
                .unwrap()
                .base()
                .earliest_walk_server_ms;
            let advance = earliest.saturating_sub(world.server_ms).max(1);
            world.advance_beat(advance);
        }
        assert_eq!(
            world.creatures.get(player).map(|k| k.position()),
            Some(Position::new(100, 101, 7)),
            "player must reach (100,101,7) via south stair"
        );

        let packets = world
            .pending_outgoing
            .get(&conn)
            .cloned()
            .unwrap_or_default();

        // Find the index of the first 0x6B (turn) packet and the first 0x64
        // (full screen map description = teleport move) packet.
        let turn_idx = packets.iter().position(|p| !p.is_empty() && p[0] == 0x6B);
        let move_idx = packets.iter().position(|p| !p.is_empty() && p[0] == 0x64);

        assert!(move_idx.is_some(), "must emit 0x64 move packet");
        assert!(turn_idx.is_some(), "must emit 0x6B chain turn");
        assert!(
            turn_idx.unwrap() > move_idx.unwrap(),
            "0x6B turn must come AFTER 0x64 move packet (C++ wire order: \
             sendMoveCreature then sendCreatureTurn), got turn at {} vs move at {}",
            turn_idx.unwrap(),
            move_idx.unwrap()
        );
    }
