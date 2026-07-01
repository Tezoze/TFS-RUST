    use super::*;
    use crate::creature::{CreatureKind, MonsterAiConfig, MonsterState};
    use crate::creature_todo::{CreatureAction, MONSTER_IDLE_WAIT_MS};
    use crate::sim_harness::{
        beat_driven_world, ensure_walkable_tile, insert_monster_with_config, insert_player,
        test_player,
    };

    fn kicker_config() -> MonsterAiConfig {
        MonsterAiConfig {
            can_push_creatures: true,
            target_distance: 1,
            ..MonsterAiConfig::default()
        }
    }

    /// 772 `MovePossible` creature branch: a `KickCreatures` attacker stepping onto a **player**
    /// tile (not its target) is the `EXHAUSTED` case — `crnonpl.cc:2236-2238`. F3: this is
    /// `ExhaustedDropTarget` (Target cleared), distinct from kick-kill `Exhausted`.
    #[test]
    fn kicker_onto_player_tile_is_exhausted() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7); // far-away attack target
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, ppos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        // Some other creature is the attack target so the player on the dest tile is *not* it.
        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
        }

        let outcome = world.monster_push_before_step(mover, ppos, now);
        assert_eq!(outcome, MonsterKickOutcome::ExhaustedDropTarget);
        // Player is untouched — never kicked.
        assert_eq!(world.creatures.get(player).map(|k| k.position()), Some(ppos));
    }

    /// A non-`KickCreatures` monster never kicks — a player tile is a hard block, not `EXHAUSTED`.
    #[test]
    fn non_kicker_onto_player_tile_proceeds() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, ppos, 1);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let cfg = MonsterAiConfig {
            can_push_creatures: false,
            target_distance: 1,
            ..MonsterAiConfig::default()
        };
        let mover = insert_monster_with_config(&mut world, "Rat", mpos, 200, cfg);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(player);
        }

        assert_eq!(
            world.monster_push_before_step(mover, ppos, now),
            MonsterKickOutcome::Proceed
        );
    }

    /// The mover's own target tile is never kicked (`crnonpl.cc:2207-2210`) — Proceed, not Exhausted.
    #[test]
    fn kicker_onto_own_target_tile_proceeds() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, ppos, 1);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(player);
        }

        assert_eq!(
            world.monster_push_before_step(mover, ppos, now),
            MonsterKickOutcome::Proceed
        );
    }

    /// `EXHAUSTED` recovery with `clear_target=true` (player-tile case) clears the target and
    /// arms a 1000 ms wait (`cract.cc:870-877` + `crnonpl.cc:2236-2238`). F3: the kick-kill case
    /// passes `clear_target=false` and preserves the target — see `f3_kick_kill_preserves_target`.
    #[test]
    fn exhausted_wait_clears_target_and_waits_1000() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        world.server_ms = 0;

        let mpos = Position::new(100, 100, 7);
        let tpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);
        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
            m.base.follow_target = Some(target);
            m.base.todo.queue.push_back(CreatureAction::Go);
        }

        world.monster_exhausted_wait_772(mover, true);

        let base = world.creatures.get(mover).unwrap().base();
        assert_eq!(base.attack_target, None);
        assert_eq!(base.follow_target, None);
        assert!(
            base.todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Wait { delay_ms } if *delay_ms == MONSTER_IDLE_WAIT_MS)),
            "EXHAUSTED must enqueue a {MONSTER_IDLE_WAIT_MS} ms Wait"
        );
        assert!(
            !base.todo.queue.iter().any(|a| matches!(a, CreatureAction::Go)),
            "ToDoClear must drop the queued Go"
        );
    }

    /// 772 `CanKickBoxes()` — race flag, or inherited from a monster master (`crnonpl.cc:2984-2992`).
    #[test]
    fn can_kick_boxes_inherits_from_master() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;

        let p = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, p, 1);

        let mut boxer = MonsterAiConfig::default();
        boxer.can_push_items = true;
        let master = insert_monster_with_config(&mut world, "Boxer", p, 200, boxer);

        // Direct race flag.
        assert!(world.monster_can_kick_boxes_772(master));

        // No flag, no master → false.
        let lone = insert_monster_with_config(
            &mut world,
            "Lone",
            Position::new(101, 100, 7),
            200,
            MonsterAiConfig::default(),
        );
        assert!(!world.monster_can_kick_boxes_772(lone));

        // No flag, but master can kick → inherits true.
        let summon = insert_monster_with_config(
            &mut world,
            "Summon",
            Position::new(102, 100, 7),
            200,
            MonsterAiConfig::default(),
        );
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(summon) {
            m.base.master = Some(master);
        }
        assert!(world.monster_can_kick_boxes_772(summon));
    }

    /// 772 `KickCreature` kill — a boxed-in pushable monster (no free adjacent tile) is killed by
    /// the kicker and the step reports `EXHAUSTED` (`crnonpl.cc:3074-3080`).
    #[test]
    fn boxed_in_blocker_is_killed_and_step_exhausted() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        world.server_ms = 0;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        // Only the kicker, blocker, and far-target tiles exist — the blocker's other neighbours are
        // absent (non-walkable), so `KickCreature` cannot relocate it and must kill.
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, bpos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let blocker =
            insert_monster_with_config(&mut world, "Rat", bpos, 200, MonsterAiConfig::default());
        let kicker = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(kicker) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
        }

        let outcome = world.monster_push_before_step(kicker, bpos, now);
        assert_eq!(outcome, MonsterKickOutcome::Exhausted);
        assert!(
            !world.creatures.contains_key(blocker),
            "boxed-in blocker must be killed by the kick"
        );
    }

    // ─────────── Pass 8 re-audit tests (P1-A1, P1-B2, P1-B3, AI#23) ───────────

    /// P1-A1: a summon with `KickCreatures` CAN kick a blocking pushable monster — C++ `MovePossible`
    /// (`crnonpl.cc:2202`) has no summon gate. The old Rust `!is_summon` gate is dropped.
    #[test]
    fn summon_kicks_blocking_monster() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        let escape = Position::new(101, 101, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, bpos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);
        ensure_walkable_tile(&mut world.map, escape, 1);

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let blocker =
            insert_monster_with_config(&mut world, "Rat", bpos, 200, MonsterAiConfig::default());
        world.map.register_creature_at(bpos, blocker);
        // Summon with KickCreatures — master is a far-away monster.
        let master = insert_monster_with_config(&mut world, "Master", tpos, 200, kicker_config());
        let summon = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        world.map.register_creature_at(mpos, summon);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(summon) {
            m.base.master = Some(master);
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
        }

        let outcome = world.monster_push_before_step(summon, bpos, now);
        assert_eq!(
            outcome,
            MonsterKickOutcome::Proceed,
            "summon with KickCreatures must kick the blocker, not stall"
        );
        assert_ne!(
            world.creatures.get(blocker).map(|k| k.position()),
            Some(bpos),
            "blocker must have been relocated by the kick"
        );
    }

    /// P1-B2: a player with `IGNORED_BY_MONSTERS` on the destination tile is a hard block
    /// (Proceed), not `EXHAUSTED` — C++ `crnonpl.cc:2230`. This test verifies the baseline
    /// (non-ignored player → EXHAUSTED); the IGNORED case requires group DB setup and is
    /// verified by the code path in `monster_kick_before_step_772`.
    #[test]
    fn ignored_player_tile_is_hard_block_not_exhausted() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, ppos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
        }

        // Without IGNORED_BY_MONSTERS flag, player tile is ExhaustedDropTarget (baseline).
        assert_eq!(
            world.monster_push_before_step(mover, ppos, now),
            MonsterKickOutcome::ExhaustedDropTarget,
            "non-ignored player tile must be ExhaustedDropTarget (baseline)"
        );
    }

    /// P1-B3: an invisible blocker (when the mover lacks SeeInvisible) is a hard block in the
    /// planning gate — `monster_move_possible_planning_772` returns false for invisible creatures.
    #[test]
    fn invisible_blocker_is_hard_block_in_planning() {
        use crate::condition::{add_condition_merge, ActiveCondition, ConditionData};
        use tfs_rust_common::enums::ConditionType as CondType;

        let mut world = beat_driven_world();
        world.walk_wake_tx = None;

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, bpos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        // A separate target (not the blocker) so the blocker is not the chase target.
        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let blocker =
            insert_monster_with_config(&mut world, "Rat", bpos, 200, MonsterAiConfig::default());
        world.map.register_creature_at(bpos, blocker);
        // Make the blocker invisible.
        if let Some(k) = world.creatures.get_mut(blocker) {
            add_condition_merge(
                &mut k.base_mut().active_conditions,
                ActiveCondition::new(0, 0, CondType::Invisible, ConditionData::Generic { ticks: 0 }, None),
            );
        }

        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
            // SeeInvisible is false by default.
        }

        // Planning gate: invisible blocker is a hard block (no SeeInvisible).
        assert!(
            !world.monster_move_possible_planning_772(mover, bpos),
            "invisible blocker must be a hard block when mover lacks SeeInvisible"
        );

        // With SeeInvisible, the blocker is plannable-through.
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.see_invisible = true;
        }
        assert!(
            world.monster_move_possible_planning_772(mover, bpos),
            "invisible blocker is plannable when mover has SeeInvisible"
        );
    }

    /// AI#23: the kick-and-retry loop clears a two-deep creature wall on the same beat.
    /// Two blockers on the destination tile; the first kick relocates one, the second kick
    /// relocates the other, then the destination is clear and the step proceeds.
    #[test]
    fn kick_and_retry_clears_two_deep_blockers() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let dest = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        let escape1 = Position::new(101, 101, 7);
        let escape2 = Position::new(101, 99, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, dest, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);
        ensure_walkable_tile(&mut world.map, escape1, 1);
        ensure_walkable_tile(&mut world.map, escape2, 1);

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        // Two blockers on the destination tile.
        let b1 = insert_monster_with_config(&mut world, "Rat1", dest, 200, MonsterAiConfig::default());
        let b2 = insert_monster_with_config(&mut world, "Rat2", dest, 200, MonsterAiConfig::default());
        world.map.register_creature_at(dest, b1);
        world.map.register_creature_at(dest, b2);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
        }

        let outcome = world.monster_push_before_step(mover, dest, now);
        assert_eq!(
            outcome,
            MonsterKickOutcome::Proceed,
            "kick-and-retry must clear both blockers and proceed"
        );
        // Both blockers should have been relocated off the destination.
        assert_ne!(
            world.creatures.get(b1).map(|k| k.position()),
            Some(dest),
            "first blocker must be relocated"
        );
        assert_ne!(
            world.creatures.get(b2).map(|k| k.position()),
            Some(dest),
            "second blocker must be relocated"
        );
    }

    /// P1-A2: a player tile is plannable-through in the 772 `MovePossible` planning gate
    /// (non-summon, non-IGNORED) — C++ `crnonpl.cc:2229-2233`.
    #[test]
    fn player_tile_is_plannable_through_in_move_possible() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, ppos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        // A separate target (not the player on the dest tile) so the player is not the chase target.
        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
        }

        // Player tile is plannable-through (non-summon, non-IGNORED, has KickCreatures + target).
        assert!(
            world.monster_move_possible_planning_772(mover, ppos),
            "player tile must be plannable-through for non-summon kicker with target"
        );
    }

    /// P1-B5: a house tile is a hard block in the 772 `MovePossible` planning gate —
    /// C++ `crnonpl.cc:2168` `IsHouse(x,y,z)`.
    #[test]
    fn house_tile_is_hard_block_in_move_possible() {
        use crate::tile::{HouseTile, Tile, TileBody};

        let mut world = beat_driven_world();
        world.walk_wake_tx = None;

        let mpos = Position::new(100, 100, 7);
        let hpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        // Insert a house tile.
        world.map.insert_tile(
            hpos,
            Tile::House(HouseTile {
                inner: TileBody {
                    ground: Some(1),
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: 0,
                    zone: tfs_rust_common::enums::ZoneType::Normal,
                },
                house_id: 1,
            }),
        );

        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
        }

        assert!(
            !world.monster_move_possible_planning_772(mover, hpos),
            "house tile must be a hard block in MovePossible planning"
        );
    }

    // ─────────── F3: split EXHAUSTED target semantics (`cract.cc:870-877`) ───────────

    /// F3: a kick-kill (`Exhausted`) preserves the target — C++ `Execute` catch
    /// (`cract.cc:870-877`) does NOT clear `Target`; the kick-kill throw site
    /// (`crnonpl.cc:2241-2242`) doesn't clear it either. Was: unconditionally cleared.
    #[test]
    fn f3_kick_kill_preserves_target() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        world.server_ms = 0;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        // Only the kicker, blocker, and far-target tiles exist — the blocker is boxed in
        // (no escape tiles), so `KickCreature` kills it and returns false → `Exhausted`.
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, bpos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let blocker =
            insert_monster_with_config(&mut world, "Rat", bpos, 200, MonsterAiConfig::default());
        world.map.register_creature_at(bpos, blocker);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
            m.base.follow_target = Some(target);
        }

        let outcome = world.monster_push_before_step(mover, bpos, now);
        assert_eq!(
            outcome,
            MonsterKickOutcome::Exhausted,
            "kick-kill must return Exhausted (target preserved)"
        );

        // F3: kick-kill recovery preserves the target (`clear_target = false`).
        world.monster_exhausted_wait_772(mover, false);

        let base = world.creatures.get(mover).unwrap().base();
        assert_eq!(
            base.attack_target,
            Some(target),
            "kick-kill must preserve attack_target (C++ Execute catch cract.cc:870-877)"
        );
        assert_eq!(
            base.follow_target,
            Some(target),
            "kick-kill must preserve follow_target (C++ Execute catch cract.cc:870-877)"
        );
        // Blocker was killed.
        assert!(!world.creatures.contains_key(blocker));
        // Wait armed.
        assert!(
            base.todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Wait { delay_ms } if *delay_ms == MONSTER_IDLE_WAIT_MS)),
            "EXHAUSTED must enqueue a {MONSTER_IDLE_WAIT_MS} ms Wait"
        );
    }

    /// F3: a player-tile `ExhaustedDropTarget` clears the target — C++ `crnonpl.cc:2236-2238`
    /// clears `Target` before `throw EXHAUSTED`. Regression of the original behavior.
    #[test]
    fn f3_player_tile_clears_target() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        world.server_ms = 0;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, ppos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        // A separate target so the player on the dest tile is *not* the attack target.
        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
            m.base.follow_target = Some(target);
        }

        let outcome = world.monster_push_before_step(mover, ppos, now);
        assert_eq!(
            outcome,
            MonsterKickOutcome::ExhaustedDropTarget,
            "player-tile must return ExhaustedDropTarget (target cleared)"
        );

        // F3: player-tile recovery clears the target (`clear_target = true`).
        world.monster_exhausted_wait_772(mover, true);

        let base = world.creatures.get(mover).unwrap().base();
        assert_eq!(
            base.attack_target, None,
            "player-tile must clear attack_target (C++ crnonpl.cc:2237)"
        );
        assert_eq!(
            base.follow_target, None,
            "player-tile must clear follow_target (C++ crnonpl.cc:2237)"
        );
        assert!(
            base.todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Wait { delay_ms } if *delay_ms == MONSTER_IDLE_WAIT_MS)),
            "EXHAUSTED must enqueue a {MONSTER_IDLE_WAIT_MS} ms Wait"
        );
    }

    /// F3: after a kick-kill + 1 s wait, the monster re-engages the **same** target — the
    /// target was preserved, so `IdleStimulus`'s `lose_existing_target` keeps it (close, valid)
    /// and `acquire_target` skips (already has a target). Was: target dropped → re-acquire
    /// might pick a different target or sleep.
    #[test]
    fn f3_kick_kill_reengages_same_target() {
        use crate::sim_harness::{beat_driven_test_world, TEST_SYNTHETIC_GROUND_WP};

        let mut world = beat_driven_test_world();
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let ppos = Position::new(102, 100, 7);
        // Walkable corridor: mover → blocker → player. Blocker is boxed in (only corridor tiles
        // exist; no perpendicular escape), so KickCreature kills it.
        for x in 100..=103u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let blocker =
            insert_monster_with_config(&mut world, "Rat", bpos, 200, MonsterAiConfig::default());
        world.map.register_creature_at(bpos, blocker);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        world.map.register_creature_at(mpos, mover);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(player);
            m.base.follow_target = Some(player);
        }

        // Kick-kill the blocker → Exhausted (target preserved).
        let outcome = world.monster_push_before_step(mover, bpos, now);
        assert_eq!(outcome, MonsterKickOutcome::Exhausted);
        world.monster_exhausted_wait_772(mover, false);

        // Target preserved after the exhausted wait.
        let base = world.creatures.get(mover).unwrap().base();
        assert_eq!(base.attack_target, Some(player));
        assert_eq!(base.follow_target, Some(player));

        // Advance past the 1000 ms wait and run IdleStimulus — the monster should still
        // target the same player (close, same floor, not in PZ/house, not invisible).
        world.server_ms += MONSTER_IDLE_WAIT_MS as u64 + 1;
        world.monster_idle_stimulus(mover);

        let base = world.creatures.get(mover).unwrap().base();
        assert_eq!(
            base.attack_target,
            Some(player),
            "monster must re-engage the same target after kick-kill + 1s wait"
        );
        assert_eq!(
            base.follow_target,
            Some(player),
            "monster must still follow the same target after kick-kill + 1s wait"
        );
    }

    // ─────────── F2: recursive chain-push (`crnonpl.cc:3066`) ───────────

    /// Helper: set up an ATTACKING pushable monster with a far-away target.
    fn insert_chain_monster(
        world: &mut GameWorld,
        name: &str,
        pos: Position,
        target: CreatureId,
    ) -> CreatureId {
        let cid = insert_monster_with_config(world, name, pos, 200, kicker_config());
        world.map.register_creature_at(pos, cid);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(cid) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
            m.base.follow_target = Some(target);
        }
        cid
    }

    /// F2: A→B→C chain-push — A kicks B, B's escape tile has C, B kicks C (chain-push),
    /// C relocates to a free tile, B relocates to C's old spot, A's dest is clear.
    /// All in one beat, no stacking (`crnonpl.cc:3066`).
    #[test]
    fn f2_chain_push_three_monsters() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let cpos = Position::new(101, 101, 7);
        let escape = Position::new(101, 102, 7);
        let tpos = Position::new(105, 105, 7);
        // Only the corridor tiles + far target exist; N(101,99) is absent so B tries S first.
        for &p in &[mpos, bpos, cpos, escape, tpos] {
            ensure_walkable_tile(&mut world.map, p, 1);
        }

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let c = insert_chain_monster(&mut world, "RatC", cpos, target);
        let b = insert_chain_monster(&mut world, "RatB", bpos, target);
        let a = insert_chain_monster(&mut world, "Cyclops", mpos, target);

        let outcome = world.monster_push_before_step(a, bpos, now);
        assert_eq!(
            outcome,
            MonsterKickOutcome::Proceed,
            "chain-push must clear the dest tile and let A proceed"
        );
        // B relocated to C's old spot.
        assert_eq!(
            world.creatures.get(b).map(|k| k.position()),
            Some(cpos),
            "B must relocate to C's old spot (chain-push)"
        );
        // C relocated to the free escape tile.
        assert_eq!(
            world.creatures.get(c).map(|k| k.position()),
            Some(escape),
            "C must relocate to the free escape tile"
        );
        // No stacking: B and C on different tiles.
        assert_ne!(
            world.creatures.get(b).map(|k| k.position()),
            world.creatures.get(c).map(|k| k.position()),
            "B and C must not share a tile (no stacking)"
        );
    }

    /// F2: A→B where B's only escape has a pushable C → B and C do **not** share a tile.
    /// Before F2, B was forcibly relocated onto C's tile (stacking). After F2, B kicks C first.
    #[test]
    fn f2_chain_push_no_stacking() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let cpos = Position::new(101, 101, 7);
        let escape = Position::new(101, 102, 7);
        let tpos = Position::new(105, 105, 7);
        for &p in &[mpos, bpos, cpos, escape, tpos] {
            ensure_walkable_tile(&mut world.map, p, 1);
        }

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let c = insert_chain_monster(&mut world, "RatC", cpos, target);
        let b = insert_chain_monster(&mut world, "RatB", bpos, target);
        let a = insert_chain_monster(&mut world, "Cyclops", mpos, target);

        let _ = world.monster_push_before_step(a, bpos, now);

        let b_pos = world.creatures.get(b).map(|k| k.position());
        let c_pos = world.creatures.get(c).map(|k| k.position());
        assert_ne!(
            b_pos, c_pos,
            "B and C must not share a tile — F2 chain-push prevents stacking"
        );
        // B moved off its original tile.
        assert_ne!(b_pos, Some(bpos), "B must have been relocated");
        // C moved off its original tile.
        assert_ne!(c_pos, Some(cpos), "C must have been relocated by chain-push");
    }

    /// F2: a boxed-in blocker (no escape tiles at all) is still killed — regression of the
    /// existing `boxed_in_blocker_is_killed_and_step_exhausted` behavior with the F2 changes.
    #[test]
    fn f2_chain_push_boxed_in_kills() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        // Only the kicker, blocker, and far-target tiles exist — the blocker's other neighbours
        // are absent (non-walkable), so `KickCreature` cannot relocate it and must kill.
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, bpos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let blocker = insert_chain_monster(&mut world, "Rat", bpos, target);
        let kicker = insert_chain_monster(&mut world, "Cyclops", mpos, target);

        let outcome = world.monster_push_before_step(kicker, bpos, now);
        assert_eq!(
            outcome,
            MonsterKickOutcome::Exhausted,
            "boxed-in blocker must be killed → Exhausted (kick-kill)"
        );
        assert!(
            !world.creatures.contains_key(blocker),
            "boxed-in blocker must be killed by the kick"
        );
    }

    /// F2: cycle guard — a 4-monster cycle (B→C→D→A→B) must terminate via `MAX_KICK_DEPTH`
    /// instead of infinite recursion. Each monster's only escape is the next one's tile.
    #[test]
    fn f2_chain_push_cycle_guard() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        // 2×2 cluster: A(100,100), B(101,100), C(101,101), D(100,101).
        // Only these 4 tiles + far target exist — each monster's only escape is the next one's tile.
        let apos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let cpos = Position::new(101, 101, 7);
        let dpos = Position::new(100, 101, 7);
        let tpos = Position::new(105, 105, 7);
        for &p in &[apos, bpos, cpos, dpos, tpos] {
            ensure_walkable_tile(&mut world.map, p, 1);
        }

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let _d = insert_chain_monster(&mut world, "RatD", dpos, target);
        let _c = insert_chain_monster(&mut world, "RatC", cpos, target);
        let b = insert_chain_monster(&mut world, "RatB", bpos, target);
        let a = insert_chain_monster(&mut world, "Cyclops", apos, target);

        // A kicks B. B's only escape is S(101,101)=C. C's only escape is W(100,101)=D.
        // D's only escape is N(100,100)=A. A's only escape is E(101,100)=B. → 4-cycle.
        // The depth guard must terminate the recursion. Eventually B has no passable escape
        // and is killed. The test passing (not hanging) proves the cycle guard works.
        let outcome = world.monster_push_before_step(a, bpos, now);
        // The cycle causes all chain-kicks attempts to fail at MAX_KICK_DEPTH → B has no
        // passable escape → B is killed → Exhausted (kick-kill).
        assert_eq!(
            outcome,
            MonsterKickOutcome::Exhausted,
            "cycle must terminate via depth guard → blocker killed → Exhausted"
        );
        assert!(
            !world.creatures.contains_key(b),
            "blocker must be killed after cycle guard terminates recursion"
        );
    }

    /// F2: a 5-monster chain-push (A→B→C→D→E) in a 1-wide corridor — all relocate one tile
    /// in a single beat. This is the "dense convoy" scenario from the audit.
    #[test]
    fn f2_dense_convoy_fluid() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        // Corridor: A(100,100)→B(101,100)→C(101,101)→D(101,102)→E(101,103)→escape(101,104).
        // The chain goes South: each blocker's escape is the next one's tile.
        let positions: [Position; 6] = [
            Position::new(100, 100, 7), // A (mover)
            Position::new(101, 100, 7), // B
            Position::new(101, 101, 7), // C
            Position::new(101, 102, 7), // D
            Position::new(101, 103, 7), // E
            Position::new(101, 104, 7), // escape (free)
        ];
        let tpos = Position::new(105, 105, 7);
        for &p in positions.iter().chain(std::iter::once(&tpos)) {
            ensure_walkable_tile(&mut world.map, p, 1);
        }

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        // Insert in reverse order so chain-push goes A→B→C→D→E.
        let e = insert_chain_monster(&mut world, "RatE", positions[4], target);
        let d = insert_chain_monster(&mut world, "RatD", positions[3], target);
        let c = insert_chain_monster(&mut world, "RatC", positions[2], target);
        let b = insert_chain_monster(&mut world, "RatB", positions[1], target);
        let a = insert_chain_monster(&mut world, "Cyclops", positions[0], target);

        let outcome = world.monster_push_before_step(a, positions[1], now);
        assert_eq!(
            outcome,
            MonsterKickOutcome::Proceed,
            "5-monster chain-push must clear the dest tile and let A proceed"
        );
        // Each monster advanced one tile South.
        assert_eq!(
            world.creatures.get(b).map(|k| k.position()),
            Some(positions[2]),
            "B must advance to C's old spot"
        );
        assert_eq!(
            world.creatures.get(c).map(|k| k.position()),
            Some(positions[3]),
            "C must advance to D's old spot"
        );
        assert_eq!(
            world.creatures.get(d).map(|k| k.position()),
            Some(positions[4]),
            "D must advance to E's old spot"
        );
        assert_eq!(
            world.creatures.get(e).map(|k| k.position()),
            Some(positions[5]),
            "E must advance to the free escape tile"
        );
        // No stacking: all on distinct tiles.
        let positions_after: Vec<_> = [b, c, d, e]
            .iter()
            .map(|&id| world.creatures.get(id).map(|k| k.position()))
            .collect();
        let unique: std::collections::HashSet<_> = positions_after.iter().collect();
        assert_eq!(
            unique.len(),
            positions_after.len(),
            "all monsters must be on distinct tiles (no stacking)"
        );
    }
