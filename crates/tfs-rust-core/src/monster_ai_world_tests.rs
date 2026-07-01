    use std::time::{Duration, Instant};

    use std::collections::VecDeque;
    use tfs_rust_common::ConnId;

    use tfs_rust_common::enums::Direction;
    use tfs_rust_common::Position;

    use crate::creature::{CreatureKind, MonsterAiConfig};
    use crate::formulas::MechanicsProfile;
    use crate::login_out::creature_wire_id;
    use crate::monster_ai::MonsterIdleChaseRepathOutcome;
    use crate::pathfinding::{truncate_cipsoft_chase_queue, uses_reverse_terrain_path, CHASE_PATH_MAX_STEPS};
    use crate::test_world::support::{
        beat_driven_test_world, beat_driven_world, ensure_walkable_tile, insert_monster_with_config, insert_player,
        insert_spectator_player, minimal_world, test_player, TEST_SYNTHETIC_GROUND_WP,
    };
    use crate::{CreatureId, GameWorld};

    /// Fist monsters skip idle `MeleeChase` while ATTACKING; seed a queue for hysteresis tests.
    fn seed_idle_chase_queue_for_test(world: &mut GameWorld, monster: CreatureId) {
        if world.beat_driven_loop
            && world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().follow_target.is_none())
        {
            // 772 appear defers target pick to idle `Strategy[]` — run one drain for fixtures.
            world.monster_idle_stimulus(monster);
        }
        if world
            .creatures
            .get(monster)
            .is_some_and(|k| !k.base().walk_queue.is_empty())
        {
            return;
        }
        let outcome = world.monster_idle_chase_repath(
            monster,
            Some("test_seed"),
            CHASE_PATH_MAX_STEPS,
            false,
        );
        assert_eq!(
            outcome,
            MonsterIdleChaseRepathOutcome::PathQueued,
            "hysteresis fixture needs a non-empty chase queue"
        );
    }

    #[test]
    fn monster_acquires_target_and_steps_toward_player() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        for x in 100..=106 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        world.monster_on_creature_appear_self(monster);

        assert!(
            world.creatures.get(monster).is_some_and(
                |k| matches!(k, CreatureKind::Monster(m) if !m.opponent_ids.is_empty())
            ),
            "player should be registered as opponent"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .follow_target
                .is_some(),
            "appear-self should select target without waiting for onThink"
        );

        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().next_walk_check = Some(Instant::now());
        }
        world.process_walk_deadlines();

        let new_pos = world.creatures.get(monster).unwrap().position();
        assert!(
            new_pos.x > mpos.x,
            "monster should step toward player (was {:?}, now {:?})",
            mpos,
            new_pos
        );
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .follow_target
                .is_some(),
            "monster should acquire follow target"
        );
    }

    #[test]
    fn monster_repaths_when_follow_target_moves() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        let ppos_moved = Position::new(104, 100, 7);
        for x in 100..=106 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        world.monster_on_creature_appear_self(monster);
        assert_eq!(
            world.creatures.get(monster).unwrap().base().follow_target,
            Some(player),
            "monster should be chasing player before target moves"
        );
        let queue_before = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .clone();
        assert!(
            !queue_before.is_empty(),
            "chasing monster should have a follow path queued"
        );
        assert!(
            world.creatures.get(monster).unwrap().base().has_follow_path,
            "chasing monster should have has_follow_path set"
        );

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_moved;
        }
        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_moved, player);
        world.monster_dispatch_creature_move(player, ppos, ppos_moved);

        assert_eq!(
            world.creatures.get(monster).unwrap().base().follow_target,
            Some(player),
            "follow target should remain after target moves one tile"
        );
        let queue_after = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .clone();
        assert!(
            !queue_after.is_empty(),
            "monster should repath when follow target moves"
        );
        assert_ne!(
            queue_before, queue_after,
            "walk queue should be recomputed after target move"
        );
        assert!(
            queue_after.iter().all(|&d| d == Direction::East),
            "repath should still step east toward player at {:?}, got {:?}",
            ppos_moved,
            queue_after
        );
    }

    #[test]
    fn monster_follow_repath_without_path_is_profile_gated() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        let ppos_moved = Position::new(104, 100, 7);
        let ppos_moved_again = Position::new(103, 100, 7);
        for x in 100..=106 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        // Chase state without has_follow_path — keep-distance band / idle before pathfinding.
        if let Some(k) = world.creatures.get_mut(monster) {
            let base = k.base_mut();
            base.follow_target = Some(player);
            base.has_follow_path = false;
            base.walk_queue = VecDeque::from([Direction::North]);
        }
        assert!(!world.mechanics.profile.follow_repath_without_path);

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_moved;
        }
        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_moved, player);
        world.monster_dispatch_creature_move(player, ppos, ppos_moved);

        assert_eq!(
            world.creatures.get(monster).unwrap().base().walk_queue,
            VecDeque::from([Direction::North]),
            "1098 profile should not repath when has_follow_path is false"
        );

        // 772 / 772 profile: repath even without has_follow_path.
        world.mechanics.profile.follow_repath_without_path = true;
        if let Some(k) = world.creatures.get_mut(monster) {
            let base = k.base_mut();
            base.walk_queue = VecDeque::from([Direction::North]);
        }
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_moved_again;
        }
        world.map.unregister_creature_at(ppos_moved, player);
        world.map.register_creature_at(ppos_moved_again, player);
        world.monster_dispatch_creature_move(player, ppos_moved, ppos_moved_again);

        let queue_beat_driven = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .clone();
        assert_ne!(
            queue_beat_driven,
            VecDeque::from([Direction::North]),
            "772 profile should repath when follow target moves without has_follow_path"
        );
        assert!(
            queue_beat_driven.iter().all(|&d| d == Direction::East),
            "repath should step east toward player at {:?}, got {:?}",
            ppos_moved_again,
            queue_beat_driven
        );
    }

    #[test]
    fn monster_acquires_target_when_player_walks_into_viewport() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let far = Position::new(112, 100, 7);
        let near = Position::new(111, 100, 7);
        for x in 100..=112 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let monster =
            insert_monster_with_config(&mut world, "Wolf", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", far));
        world.map.register_creature_at(far, player);

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = near;
        }
        world.map.unregister_creature_at(far, player);
        world.map.register_creature_at(near, player);
        world.monster_dispatch_creature_move(player, far, near);

        assert!(
            world.creatures.get(monster).unwrap().base().follow_target == Some(player),
            "monster should target player as soon as they enter viewport"
        );
    }

    #[test]
    fn fleeing_monster_steps_away_from_player() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        for x in 99..=102 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let config = MonsterAiConfig {
            run_away_health: 50,
            ..Default::default()
        };
        let monster = insert_monster_with_config(&mut world, "Rat", mpos, 200, config);
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.health = 30;
            m.opponent_ids.push(player);
            m.is_idle = false;
        }
        let _ = world.monster_search_target(monster, super::TargetSearchType::Default);
        world.go_to_follow_creature(monster, None);
        world.process_walk_deadlines();

        let new_pos = world.creatures.get(monster).unwrap().position();
        assert!(
            new_pos.x < mpos.x,
            "fleeing monster should step away from player on the east"
        );
    }

    #[test]
    fn update_look_direction_broadcasts_turn() {
        let mut world = minimal_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let conn = ConnId(7);
        let player = insert_spectator_player(&mut world, conn, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.attack_target = Some(player);
            m.base.direction = Direction::North;
        }
        let wire_id = creature_wire_id(monster, world.creatures.get(monster).unwrap());
        world
            .creature_fully_sent_by_conn
            .entry(conn)
            .or_default()
            .insert(wire_id);

        world.monster_update_look_direction(monster);

        let pending = world
            .pending_outgoing
            .get(&conn)
            .cloned()
            .unwrap_or_default();
        assert!(
            pending.iter().any(|p| p.first() == Some(&0x6B)),
            "look-at-target should emit 0x6B turn packet"
        );
        assert_eq!(
            world.creatures.get(monster).unwrap().base().direction,
            Direction::East
        );
    }

    #[test]
    fn update_look_direction_ignored_when_walking() {
        let mut world = minimal_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());

        let conn = ConnId(7);
        let player = insert_spectator_player(&mut world, conn, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.attack_target = Some(player);
            m.base.direction = Direction::North;
            m.base.next_walk_check = Some(Instant::now() + std::time::Duration::from_millis(500));
        }

        world.monster_update_look_direction(monster);

        // Direction should remain North because walk is active (not idle)
        assert_eq!(
            world.creatures.get(monster).unwrap().base().direction,
            Direction::North
        );
    }

    #[test]
    fn select_target_automatically_updates_look_direction() {
        let mut world = minimal_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let conn = ConnId(7);
        let player = insert_spectator_player(&mut world, conn, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.direction = Direction::North;
            m.opponent_ids.push(player);
        }

        // monster_select_target should automatically set follow/attack target and update look direction to East (towards player).
        let selected = world.monster_select_target(monster, player);
        assert!(selected);

        let dir = world.creatures.get(monster).unwrap().base().direction;
        assert_eq!(dir, Direction::East);
    }

    #[test]
    fn monster_does_not_acquire_distant_player() {
        let mut world = minimal_world();
        let mpos = Position::new(100, 100, 8);
        let ppos = Position::new(130, 100, 8);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        world.monster_update_target_list(monster);

        assert!(
            world.creatures.get(monster).is_some_and(
                |k| matches!(k, CreatureKind::Monster(m) if m.opponent_ids.is_empty())
            ),
            "player 30 tiles away must not enter opponent list"
        );
    }

    #[test]
    fn monster_prunes_opponent_when_player_leaves_can_see_range() {
        let mut world = minimal_world();
        let mpos = Position::new(100, 100, 8);
        let near = Position::new(105, 100, 8);
        let far = Position::new(130, 100, 8);
        for p in [mpos, near, far] {
            ensure_walkable_tile(&mut world.map, p, TEST_SYNTHETIC_GROUND_WP);
        }

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", near));
        world.map.register_creature_at(near, player);
        world.monster_on_creature_appear_self(monster);
        assert!(world
            .creatures
            .get(monster)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if !m.opponent_ids.is_empty())));

        // Player teleports out of monster viewport — C++ updateTargetList prunes via canSee.
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = far;
        }
        world.map.unregister_creature_at(near, player);
        world.map.register_creature_at(far, player);

        world.monster_update_target_list(monster);

        assert!(
            world.creatures.get(monster).is_some_and(
                |k| matches!(k, CreatureKind::Monster(m) if m.opponent_ids.is_empty())
            ),
            "opponent must be pruned when outside Creature::canSee range"
        );
    }

    #[test]
    fn monster_walks_back_when_target_lost() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let spawn = Position::new(100, 100, 7);
        let far = Position::new(120, 100, 7);
        for x in 100..=120 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let monster =
            insert_monster_with_config(&mut world, "Rat", far, 200, MonsterAiConfig::default());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.spawn_position = spawn;
            m.is_idle = false;
        }
        let player = insert_player(&mut world, test_player("Hero", Position::new(121, 100, 7)));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.opponent_ids.push(player);
        }

        world.monster_remove_creature_from_lists(monster, player);

        assert!(
            world.creatures.get(monster).is_some_and(|k| {
                matches!(k, CreatureKind::Monster(m) if m.walking_to_spawn && !m.base.walk_queue.is_empty())
            }),
            "monster outside walkToSpawnRadius should path toward spawn when last opponent leaves"
        );
    }

    #[test]
    fn test_772_skips_walk_to_spawn_on_opponent_leave() {
        let mut world = beat_driven_test_world();
        world.walk_wake_tx = None;
        let spawn = Position::new(100, 100, 7);
        let far = Position::new(120, 100, 7);
        for x in 100..=120 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let monster =
            insert_monster_with_config(&mut world, "Rat", far, 200, MonsterAiConfig::default());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.spawn_position = spawn;
            m.is_idle = false;
        }
        let player = insert_player(&mut world, test_player("Hero", Position::new(121, 100, 7)));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.opponent_ids.push(player);
        }

        world.monster_remove_creature_from_lists(monster, player);

        assert!(
            world.creatures.get(monster).is_some_and(|k| {
                matches!(k, CreatureKind::Monster(m) if !m.walking_to_spawn)
            }),
            "772 must not TFS walk-to-spawn when last opponent leaves"
        );
    }

    #[test]
    fn idle_monster_does_not_random_walk() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);

        let monster =
            insert_monster_with_config(&mut world, "Rat", pos, 200, MonsterAiConfig::default());
        assert!(world
            .creatures
            .get(monster)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if m.is_idle)));

        let now = Instant::now();
        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().last_step = Some(now - Duration::from_secs(2));
            k.base_mut().next_walk_check = Some(now);
        }
        world.process_walk_deadlines();

        assert_eq!(world.creatures.get(monster).unwrap().position(), pos);
    }

    #[test]
    fn active_monster_random_roams_after_one_second() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new((100 + dx) as u16, (100 + dy) as u16, 7),
                    100,
                );
            }
        }

        let monster =
            insert_monster_with_config(&mut world, "Rat", pos, 200, MonsterAiConfig::default());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }

        let now = Instant::now();
        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().last_step = Some(now - Duration::from_secs(2));
        }
        let step = world.monster_next_walk_step(monster, now);
        assert!(
            step.is_some(),
            "active monster should pick a random roam direction after 1 s idle on tile"
        );
    }

    #[test]
    fn ranged_monster_steps_away_when_adjacent() {
        let mut world = minimal_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        for x in 99..=102 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let config = MonsterAiConfig {
            target_distance: 4,
            ..Default::default()
        };
        let monster = insert_monster_with_config(&mut world, "Rat", mpos, 200, config);
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = true;
        }

        world.go_to_follow_creature(monster, None);

        let stepped_away = world
            .creatures
            .get(monster)
            .is_some_and(|k| k.base().walk_queue.iter().any(|&d| d == Direction::West));
        if stepped_away {
            return;
        }

        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().next_walk_check = Some(Instant::now());
        }
        world.process_walk_deadlines();

        let final_pos = world.creatures.get(monster).unwrap().position();
        assert!(
            final_pos.x < mpos.x,
            "ranged monster should step west away from adjacent player (was {:?}, now {:?})",
            mpos,
            final_pos
        );
    }

    #[test]
    fn ranged_keep_distance_clamps_to_melee_when_attack_unusable() {
        let mut world = minimal_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        for x in 100..=104 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        // Rat has melee-only attack data in content, so at distance 4 it cannot use attack.
        let config = MonsterAiConfig {
            target_distance: 4,
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
            m.base.has_follow_path = false;
        }

        let fpp = world.monster_path_search_params(monster, player, false, 4, false, false);
        assert_eq!(
            fpp.max_target_dist, 4,
            "TFS keeps maxTargetDist at XML targetDistance"
        );
        assert!(
            fpp.full_path_search,
            "TFS sets fullPathSearch when attack is unusable at range"
        );
    }

    // ---- B3 mechanics-profile knobs ----

    /// B3.2 — `DistanceKeep::PerType` keeps the monster's XML `targetDistance`; `Fixed(n)` overrides.
    #[test]
    fn effective_target_distance_follows_profile() {
        use tfs_rust_common::ProtocolVersion;
        let mut world = minimal_world();

        // 1098 default: per-type passes through unchanged.
        assert_eq!(world.monster_effective_target_distance(1), 1);
        assert_eq!(world.monster_effective_target_distance(7), 7);

        // 772 default: per-type from monster file (no era-wide override).
        world.mechanics = crate::formulas::Mechanics::for_version(ProtocolVersion::V772);
        assert_eq!(world.monster_effective_target_distance(1), 1);
        assert_eq!(world.monster_effective_target_distance(7), 7);
    }

    /// B3.1 — weakest-target metric: 772 compares current HP, 1098 compares max HP. Construct two
    /// players where the lowest-current and lowest-max are different creatures.
    #[test]
    fn weakest_opponent_metric_follows_profile() {
        use tfs_rust_common::ProtocolVersion;
        let mut world = minimal_world();

        // Player A: big max pool, badly wounded (low current).
        let a = insert_player(&mut world, test_player("Tank", Position::new(100, 100, 7)));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.base.max_health = 1000;
            p.base.health = 20;
        }
        // Player B: small max pool, full health (low max, higher current than A).
        let b = insert_player(
            &mut world,
            test_player("Squire", Position::new(101, 100, 7)),
        );
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(b) {
            p.base.max_health = 100;
            p.base.health = 100;
        }
        let candidates = [a, b];

        // 1098 (max HP): B is weakest (max 100 < 1000).
        assert_eq!(world.monster_weakest_opponent(&candidates), Some(b));

        // 772 (current HP): A is weakest (current 20 < 100).
        world.mechanics = crate::formulas::Mechanics::for_version(ProtocolVersion::V772);
        assert_eq!(world.monster_weakest_opponent(&candidates), Some(a));
    }

    /// P7 — 772 beat loop: `monster_arm_event_walk` must not re-arm when `next_wakeup` is set.
    #[test]
    fn beat_driven_arm_event_walk_skips_when_wakeup_set() {
        let mut world = beat_driven_test_world();
        world.walk_wake_tx = None;
        world.server_ms = 0;

        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 2148);
        let cid =
            insert_monster_with_config(&mut world, "Rat", pos, 200, MonsterAiConfig::default());

        world.monster_arm_event_walk(cid);
        let wakeup = world
            .creatures
            .get(cid)
            .unwrap()
            .base()
            .next_wakeup
            .expect("first arm should schedule ToDoQueue wakeup");
        assert_eq!(world.todo_queue.len(), 1);

        world.monster_arm_event_walk(cid);

        assert_eq!(
            world.creatures.get(cid).unwrap().base().next_wakeup,
            Some(wakeup),
            "second arm must not reschedule walk"
        );
        assert_eq!(world.todo_queue.len(), 1, "no duplicate heap entry");
    }

    #[test]
    fn test_772_melee_dance_only_cardinal() {
        let mut world = beat_driven_test_world();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);

        // Make surrounding tiles walkable
        for dx in -1i32..=1i32 {
            for dy in -1i32..=1i32 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new((100 + dx) as u16, (100 + dy) as u16, 7),
                    2148,
                );
            }
        }

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }

        // Sample several times to verify all chosen step directions are cardinal (or None)
        let now = std::time::Instant::now();
        for _ in 0..100 {
            if let Some(dir) = world.monster_next_walk_step(monster, now) {
                assert!(
                    matches!(
                        dir,
                        Direction::North | Direction::East | Direction::South | Direction::West
                    ),
                    "772 melee dance step must be cardinal, got {:?}",
                    dir
                );
            }
        }
    }

    #[test]
    fn test_772_walk_queue_hysteresis() {
        let mut world = beat_driven_test_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        let ppos_moved = Position::new(104, 100, 7);
        for x in 100..=106 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        world.monster_on_creature_appear_self(monster);
        seed_idle_chase_queue_for_test(&mut world, monster);

        let queue_before = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .clone();
        assert!(!queue_before.is_empty());

        // Target moves 1 tile closer (105 -> 104)
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_moved;
        }
        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_moved, player);
        world.monster_dispatch_creature_move(player, ppos, ppos_moved);

        let queue_after = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .clone();

        // Hysteresis: Walk queue should NOT clear/recompute because target is still within goal range
        assert_eq!(
            queue_before, queue_after,
            "772 walk queue should be retained due to hysteresis when target moves slightly"
        );
    }

    #[test]
    fn test_772_ensure_follow_band_keeps_queue_when_not_stale() {
        let mut world = beat_driven_test_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        let ppos_moved = Position::new(104, 100, 7);
        for x in 100..=106 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        world.monster_on_creature_appear_self(monster);
        seed_idle_chase_queue_for_test(&mut world, monster);

        let queue_before = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .clone();
        assert!(!queue_before.is_empty());

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_moved;
        }
        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_moved, player);

        let repathed = world.monster_ensure_follow_band(monster, "idle");
        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            !repathed,
            "772 ensure_follow_band must not schedule repath when queue still valid"
        );
        assert_eq!(base.walk_queue, queue_before);
        assert!(!base.force_update_follow_path);
    }

    #[test]
    fn test_772_target_move_empty_queue_defers_to_idle() {
        let mut world = beat_driven_test_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        let ppos_moved = Position::new(104, 100, 7);
        for x in 100..=106 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.follow_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_moved;
        }
        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_moved, player);
        world.monster_dispatch_creature_move(player, ppos, ppos_moved);

        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            !base.force_update_follow_path,
            "772 empty queue must not force_update on every target tile"
        );
        assert!(base.walk_queue.is_empty());

        let (needs, reason) = world.monster_idle_chase_needs_repath(monster);
        assert!(needs, "idle should still repath via idle_drain or off_band");
        assert!(matches!(reason, Some("idle_drain") | Some("off_band")));
    }

    #[test]
    fn test_772_ensure_follow_band_defers_repath_via_force_update() {
        let mut world = beat_driven_test_world();
        world.walk_wake_tx = None;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(108, 100, 7);
        for x in 100..=109 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

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
            m.base.has_follow_path = true;
            m.base.walk_queue = VecDeque::from([Direction::North]);
        }

        let scheduled = world.monster_ensure_follow_band(monster, "idle");
        assert!(scheduled, "stale queue must schedule repath on 772");
        let base = world.creatures.get(monster).unwrap().base();
        assert!(base.force_update_follow_path);
        assert!(base.walk_queue.is_empty());
        assert!(!base.has_follow_path);

        world.monster_idle_stimulus(monster);
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| !k.base().walk_queue.is_empty() || k.base().todo.has_go()),
            "idle must repath after ensure_follow_band deferred force_update"
        );
    }

    #[test]
    fn test_772_path_prefers_cardinal_on_open_terrain() {
        let mut world = beat_driven_test_world();

        let mpos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        for x in 90..=110 {
            for y in 90..=110 {
                ensure_walkable_tile(&mut world.map, Position::new(x, y, 7), 150);
            }
        }

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", mpos));
        world.map.register_creature_at(mpos, player);

        for dx in -5..=5 {
            for dy in -5..=5 {
                let target_pos = Position::new((100 + dx) as u16, (100 + dy) as u16, 7);
                if target_pos == mpos {
                    continue;
                }

                if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
                    p.base.position = target_pos;
                }

                let fpp = world.monster_path_search_params(monster, player, false, 1, false, false);

                if let Some(steps) = world.get_creature_path_to_with_fpp(monster, target_pos, &fpp)
                {
                    for step in steps {
                        assert!(
                            matches!(
                                step,
                                Direction::North
                                    | Direction::East
                                    | Direction::South
                                    | Direction::West
                            ),
                            "3× diagonal cost should prefer cardinals on open uniform terrain; \
                             path to ({}, {}) used {:?}",
                            100 + dx,
                            100 + dy,
                            step,
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_772_allow_diagonal_true_stays_reverse_path_stack() {
        use tfs_rust_common::ProtocolVersion;

        let profile = MechanicsProfile::for_version(ProtocolVersion::V772);
        assert!(uses_reverse_terrain_path(
            profile.path_cost,
            profile.path_search
        ));
        assert!(!profile.path_forward_fallback);

        let mut world = beat_driven_test_world();
        assert_eq!(
            world.mechanics.profile.path_forward_fallback,
            profile.path_forward_fallback
        );

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 105, 7);
        for x in 95..=110u16 {
            for y in 95..=110u16 {
                ensure_walkable_tile(&mut world.map, Position::new(x, y, 7), TEST_SYNTHETIC_GROUND_WP);
            }
        }

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        let fpp = world.monster_path_search_params(monster, player, false, 1, false, false);
        assert!(
            fpp.allow_diagonal,
            "772 allows diagonal neighbors in 772 expansion"
        );

        let path = world
            .get_creature_path_to_with_fpp(monster, ppos, &fpp)
            .expect("reverse TShortway path");
        assert!(!path.is_empty());
        let steps = truncate_cipsoft_chase_queue(
            mpos,
            ppos,
            path,
            CHASE_PATH_MAX_STEPS,
            false,
            1,
        );
        assert!(!steps.is_empty());
        for step in &steps {
            assert!(
                matches!(
                    step,
                    Direction::North | Direction::East | Direction::South | Direction::West
                ),
                "allow_diagonal=true on 772 must still use terrain×3 (cardinals on open grass), not TFS 10/25 bias: {step:?} in {steps:?}"
            );
        }
    }

    #[test]
    fn test_772_diagonal_detour_when_cardinals_blocked() {
        use crate::tile::{flags as tilestate, Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;

        let mut world = beat_driven_test_world();

        let mpos = Position::new(10, 10, 7);
        let ppos = Position::new(12, 12, 7);
        for x in 8..=14u16 {
            for y in 8..=14u16 {
                ensure_walkable_tile(&mut world.map, Position::new(x, y, 7), 150);
            }
        }
        // Block all four cardinals around the monster — only diagonal exits remain.
        for (x, y) in [(10, 9), (10, 11), (9, 10), (11, 10)] {
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

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        let fpp = world.monster_path_search_params(monster, player, false, 1, false, false);
        let path = world
            .get_creature_path_to_with_fpp(monster, ppos, &fpp)
            .expect("reverse TShortway must diagonal out when cardinals are blocked");
        assert!(
            path.iter().any(|d| {
                matches!(
                    d,
                    Direction::NorthEast
                        | Direction::NorthWest
                        | Direction::SouthEast
                        | Direction::SouthWest
                )
            }),
            "path must use a diagonal to leave the cardinal trap: {path:?}"
        );
    }

    /// P0-1 — empty `walk_queue` during chase must idle-repath, not arm step-duration poll.
    #[test]
    fn test_772_empty_queue_triggers_idle_repath() {
        let mut world = beat_driven_test_world();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        for x in 100..=105u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

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
            m.base.walk_queue.clear();
            m.base.force_update_follow_path = true;
            m.base.last_step_server_ms = None;
        }

        let now = std::time::Instant::now();
        world.on_walk(monster, false, now, None);

        let base = world.creatures.get(monster).unwrap().base();
        let repathed = !base.walk_queue.is_empty();
        let immediate_wakeup = base
            .next_wakeup
            .is_some_and(|w| w <= world.server_ms.saturating_add(1));
        assert!(
            repathed || immediate_wakeup,
            "empty chase queue must idle-repath immediately, not poll step delay; \
             walk_queue_len={} next_wakeup={:?}",
            base.walk_queue.len(),
            base.next_wakeup
        );
        if let Some(w) = base.next_wakeup {
            let delay = w.saturating_sub(world.server_ms);
            assert!(
                delay <= 1 || repathed,
                "expected immediate wakeup (<=1 beat), got delay={delay}ms"
            );
        }
    }

    #[test]
    fn test_772_repath_first_step_delay() {
        let mut world = beat_driven_test_world();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);
        for x in 100..=105 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);

            // Set last step to now so there is a positive walk delay
            m.base.last_step_server_ms = Some(world.server_ms);
            m.base.last_step_ground_speed = 150;
            m.base.last_step_cost = 1;
        }

        // Trigger repath
        world.monster_follow_repath_now(monster, None);

        let wakeup = world.creatures.get(monster).unwrap().base().next_wakeup;

        // Wakeup should be scheduled for server_ms + walk_delay (approx 350 ms)
        assert!(wakeup.is_some());
        let delay = wakeup.unwrap() - world.server_ms;
        assert!(
            delay >= 300,
            "expected repath delay to respect walk delay, got {}",
            delay
        );
    }

    #[test]
    fn test_772_flee_steps_away() {
        let mut world = beat_driven_test_world();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);
        ensure_walkable_tile(&mut world.map, Position::new(99, 100, 7), 150);

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.health = 10;
            m.run_away_health = 20;
            m.flee_opening_melee_dance_done = true;
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        let now = std::time::Instant::now();
        assert_eq!(
            world.monster_next_walk_step(monster, now),
            None,
            "772 getNextStep must not inline flee — idle drain owns it (X4)"
        );

        world.monster_idle_stimulus(monster);

        let stepped_west = world
            .creatures
            .get(monster)
            .is_some_and(|k| k.base().walk_queue.iter().any(|&d| d == Direction::West));
        assert!(
            stepped_west,
            "idle flee arm must queue West away from player East"
        );
    }

    #[test]
    fn test_772_blocked_flee_stops() {
        let mut world = beat_driven_test_world();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7); // player to the east
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);

        // All neighbor tiles are blocked (non-walkable)
        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.health = 10;
            m.run_away_health = 20; // fleeing
        }

        // Run go_to_follow_creature. Since fleeing is true and pathing fails, it should clear follow path and stop,
        // without attempting any closer steps.
        world.go_to_follow_creature(monster, None);

        let walk_queue_empty = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .is_empty();
        assert!(
            walk_queue_empty,
            "blocked flee must not populate walk queue"
        );
    }

    #[test]
    fn test_772_at_follow_goal_keep_distance_without_spell_range() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);

        let config = MonsterAiConfig {
            target_distance: 4,
            is_hostile: true,
            ..Default::default()
        };
        let monster = insert_monster_with_config(&mut world, "Rat", mpos, 200, config);
        let player = insert_player(&mut world, test_player("Hero", ppos));

        assert!(
            !world.monster_can_use_attack(monster, mpos, player),
            "test assumes no in-range attack spells for Rat at dist 4"
        );
        assert!(
            world.monster_at_follow_goal(monster, player, mpos, ppos, false, 4),
            "772 keep-distance at cheb==target_distance is at goal without canUseAttack"
        );
    }

    #[test]
    fn test_772_full_path_search_off_band() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let at_band = Position::new(104, 100, 7);
        let off_band = Position::new(106, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, at_band, 150);
        ensure_walkable_tile(&mut world.map, off_band, 150);

        let config = MonsterAiConfig {
            target_distance: 4,
            ..Default::default()
        };
        let monster = insert_monster_with_config(&mut world, "Rat", mpos, 200, config);
        let player_at = insert_player(&mut world, test_player("HeroAt", at_band));
        let player_off = insert_player(&mut world, test_player("HeroOff", off_band));

        let fpp_at = world.monster_path_search_params(monster, player_at, false, 4, false, false);
        assert!(
            !fpp_at.full_path_search,
            "at keep band cheb==4 must use directional search box"
        );

        let fpp_off = world.monster_path_search_params(monster, player_off, false, 4, false, false);
        assert!(
            fpp_off.full_path_search,
            "off keep band cheb>4 must use full search box"
        );
    }

    #[test]
    fn test_772_melee_adjacent_chase_step_budget() {
        use super::monster_idle_chase_step_budget;

        assert_eq!(
            monster_idle_chase_step_budget(true, false, 2, 1),
            (CHASE_PATH_MAX_STEPS, false)
        );
        assert_eq!(
            monster_idle_chase_step_budget(true, false, 3, 1),
            (CHASE_PATH_MAX_STEPS, false)
        );
        assert_eq!(
            monster_idle_chase_step_budget(false, true, 7, 4),
            (3, false)
        );
        assert_eq!(
            monster_idle_chase_step_budget(false, true, 6, 3),
            (3, false)
        );
        assert_eq!(
            monster_idle_chase_step_budget(false, false, 2, 4),
            (CHASE_PATH_MAX_STEPS, false)
        );
    }

    #[test]
    fn test_772_idle_no_greedy_step_on_path_fail() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }

        let outcome = world.monster_idle_chase_repath(
            monster,
            Some("idle_drain"),
            CHASE_PATH_MAX_STEPS,
            false,
        );
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::Noway);
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .is_empty(),
            "772 path fail must not fall back to greedy closer step"
        );
    }

    /// `kite_cyclops_quad_chase` — far-N plans last; first hop must not enter NW sibling tile.
    #[test]
    fn cyclops_quad_far_n_path_avoids_nw_sibling_when_last() {
        use crate::creature::MonsterState;
        use crate::pathfinding::REVERSE_PATH_VIEW_RADIUS;
        use crate::sim_harness::{
            beat_driven_world_for_kite_synthetic, default_sim_map_config, insert_monster_from_type,
            insert_player,
        };

        let cfg = default_sim_map_config();
        if !cfg.data_dir.is_dir() {
            return;
        }
        let Ok(mut world) = beat_driven_world_for_kite_synthetic(
            &cfg.data_dir,
            &cfg.map_rel,
            (32360, 32290),
            16,
            7,
            150,
        ) else {
            return;
        };
        let spawns = [
            Position::new(32359, 32288, 7),
            Position::new(32361, 32290, 7),
            Position::new(32360, 32291, 7),
            Position::new(32359, 32289, 7),
        ];
        let player_pos = Position::new(32360, 32294, 7);
        let player = insert_player(&mut world, test_player("Hero", player_pos));
        world.map.register_creature_at(player_pos, player);
        let mtype = world.monsters_db.monsters.get("cyclops").cloned();
        let Some(mtype) = mtype else {
            return;
        };
        let mut ids = Vec::new();
        for (i, &pos) in spawns.iter().enumerate() {
            let mid = insert_monster_from_type(
                &mut world,
                &mtype,
                &format!("Cyclops {}", i + 1),
                pos,
                mtype.speed as i32,
                MonsterAiConfig::from_monster_type(&mtype),
                MonsterState::Attacking,
            );
            if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mid) {
                m.is_idle = false;
                m.opponent_ids.push(player);
                m.base.follow_target = Some(player);
                m.base.attack_target = Some(player);
            }
            ids.push(mid);
        }
        let far_n = ids[0];
        let nw_pos = spawns[3];
        let tile = world.map.get_tile(nw_pos).expect("nw tile");
        assert!(
            tile.body().creatures.contains(&ids[3]),
            "NW cyclops must occupy map tile before path query"
        );
        let walkable = world.monster_tshortway_fill_walkable(far_n, nw_pos, player_pos);
        assert!(
            !walkable,
            "fill walkable must reject NW sibling tile (got {walkable})"
        );
        let fpp = world.monster_path_search_params(far_n, player, false, 1, false, false);
        let mut steps = world
            .get_creature_path_to_with_fpp(far_n, player_pos, &fpp)
            .expect("far-N chase path");
        steps = crate::pathfinding::truncate_cipsoft_chase_queue(
            spawns[0],
            player_pos,
            steps,
            crate::pathfinding::CHASE_PATH_MAX_STEPS,
            false,
            1,
        );
        assert!(!steps.is_empty(), "path must not be empty");
        let first = spawns[0].offset(steps[0]);
        assert_ne!(
            first, nw_pos,
            "far-N first hop must not enter NW sibling tile (C++ `MovePossible` blocks unpushable)"
        );
    }

    /// `kite_cyclops_quad_chase` — NW + far-N shortway tiles match live C++ JSONL @tick=2000.
    ///
    /// Ignored until fresh C++ oracle — see `tasks/lessons.md` §59.
    #[test]
    fn cyclops_quad_nw_and_far_n_shortway_match_live_ref() {
        use crate::creature::MonsterState;
        use crate::pathfinding::{
            truncate_cipsoft_chase_queue, CHASE_PATH_MAX_STEPS, REVERSE_PATH_VIEW_RADIUS,
        };
        use crate::sim_harness::{
            beat_driven_world_for_kite_synthetic, default_sim_map_config, insert_monster_from_type,
            insert_player,
        };
        use tfs_rust_common::enums::Direction;

        fn steps_to_tiles(start: Position, steps: &[Direction]) -> Vec<Position> {
            let mut pos = start;
            steps
                .iter()
                .map(|&d| {
                    pos = pos.offset(d);
                    pos
                })
                .collect()
        }

        fn is_diagonal_step(from: Position, to: Position) -> bool {
            from.x.abs_diff(to.x) == 1 && from.y.abs_diff(to.y) == 1
        }

        let cfg = default_sim_map_config();
        if !cfg.data_dir.is_dir() {
            return;
        }
        let Ok(mut world) = beat_driven_world_for_kite_synthetic(
            &cfg.data_dir,
            &cfg.map_rel,
            (32360, 32290),
            16,
            7,
            150,
        ) else {
            return;
        };
        let spawns = [
            Position::new(32359, 32288, 7), // far-N
            Position::new(32361, 32290, 7), // east
            Position::new(32360, 32291, 7), // south
            Position::new(32359, 32289, 7), // NW
        ];
        let player_pos = Position::new(32360, 32294, 7);
        let player = insert_player(&mut world, test_player("Hero", player_pos));
        world.map.register_creature_at(player_pos, player);
        let mtype = world.monsters_db.monsters.get("cyclops").cloned();
        let Some(mtype) = mtype else {
            return;
        };
        let mut ids = Vec::new();
        for (i, &pos) in spawns.iter().enumerate() {
            let mid = insert_monster_from_type(
                &mut world,
                &mtype,
                &format!("Cyclops {}", i + 1),
                pos,
                mtype.speed as i32,
                MonsterAiConfig::from_monster_type(&mtype),
                MonsterState::Attacking,
            );
            if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mid) {
                m.is_idle = false;
                m.opponent_ids.push(player);
                m.base.follow_target = Some(player);
                m.base.attack_target = Some(player);
            }
            ids.push(mid);
        }

        let nw_id = ids[3];
        let far_n_id = ids[0];
        let nw_start = spawns[3];
        let far_n_start = spawns[0];

        let chase_path = |cid: CreatureId, start: Position| -> Vec<Position> {
            let fpp = world.monster_path_search_params(cid, player, false, 1, false, false);
            let raw = world
                .get_creature_path_to_with_fpp(cid, player_pos, &fpp)
                .expect("chase path");
            let steps = truncate_cipsoft_chase_queue(
                start,
                player_pos,
                raw,
                CHASE_PATH_MAX_STEPS,
                false,
                1,
            );
            steps_to_tiles(start, &steps)
        };

        let nw_tiles = chase_path(nw_id, nw_start);
        let want_nw = [
            Position::new(32358, 32290, 7),
            Position::new(32358, 32291, 7),
            Position::new(32359, 32291, 7),
        ];
        assert_eq!(nw_tiles, want_nw, "NW shortway must match live C++ ref");

        let far_n_tiles = chase_path(far_n_id, far_n_start);
        let want_far_n = [
            Position::new(32359, 32287, 7),
            Position::new(32359, 32286, 7),
            Position::new(32358, 32286, 7),
        ];
        assert_eq!(
            far_n_tiles, want_far_n,
            "far-N shortway must match live C++ ref"
        );

        assert!(
            is_diagonal_step(nw_start, nw_tiles[0]),
            "NW first hop must be diagonal (live ref go_exec diag=1)"
        );
    }

    /// C++ `ObjectDistance` returns `INT_MAX` when Z-levels differ (`info.cc:313`),
    /// so `TCombat::Attack` gets `Distance > 8` → `StopAttack` + `TARGETLOST`
    /// (`crcombat.cc:574-578`). A monster must NOT deal damage to a player on a
    /// different floor, even if `chebyshev` (x/y only) says they're adjacent.
    #[test]
    fn test_772_attack_blocked_across_z_levels() {
        use crate::creature::MonsterState;
        use crate::sim_harness::{
            beat_driven_test_world, ensure_walkable_tile, insert_monster, insert_player,
            test_player,
        };

        let mut world = beat_driven_test_world();
        world.server_ms = 1000;
        // Monster on z=7, player on z=8 — same x/y (adjacent in chebyshev, different floor).
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 8);
        ensure_walkable_tile(&mut world.map, mpos, 2148);
        ensure_walkable_tile(&mut world.map, ppos, 2148);

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let mut player = test_player("Hero", ppos);
        player.base.health = 500;
        player.base.max_health = 500;
        let player = insert_player(&mut world, player);
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = 15;
            m.melee_attack = 7;
            m.is_hostile = true;
            m.base.attack_target = Some(player);
            m.base.follow_target = Some(player);
            m.state = MonsterState::Attacking;
        }

        let hp_before = world.creatures.get(player).unwrap().base().health;
        world.monster_do_attacking(monster, 200);
        let hp_after = world.creatures.get(player).unwrap().base().health;

        assert_eq!(
            hp_before, hp_after,
            "monster must not deal damage to a player on a different Z-level \
             (C++ ObjectDistance returns INT_MAX for diff Z → Distance>8 → TARGETLOST)"
        );
    }
