    use super::*;

    #[test]
    fn fillmap_terrain_reads_grass_bank_waypoints() {
        use crate::sim_harness::{beat_driven_world_for_kite_synthetic, default_sim_map_config};

        let cfg = default_sim_map_config();
        if !cfg.data_dir.is_dir() {
            return;
        }
        let Ok(world) = beat_driven_world_for_kite_synthetic(
            &cfg.data_dir,
            &cfg.map_rel,
            (32360, 32290),
            16,
            7,
            150,
        ) else {
            return;
        };
        let grass = Position::new(32360, 32290, 7);
        assert_eq!(
            world.fillmap_terrain_waypoints_at(grass),
            150,
            "stack-head grass BANK must expose raw OTB WAYPOINTS"
        );
    }

    #[test]
    fn fillmap_movepossible_blocks_unpass_under_grass() {
        use crate::creature::{MonsterAiConfig, MonsterState};
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
        let player = insert_player(
            &mut world,
            crate::test_world::support::test_player("Hero", Position::new(32360, 32294, 7)),
        );
        let mtype = match world.monsters_db.monsters.get("cyclops").cloned() {
            Some(t) => t,
            None => return,
        };
        let cid = insert_monster_from_type(
            &mut world,
            &mtype,
            "Cyclops",
            Position::new(32359, 32288, 7),
            mtype.speed as i32,
            MonsterAiConfig::from_monster_type(&mtype),
            MonsterState::Attacking,
        );
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(cid) {
            m.base.attack_target = Some(player);
            m.base.follow_target = Some(player);
        }
        let fir_tile = Position::new(32359, 32290, 7);
        assert_eq!(
            world.fillmap_terrain_waypoints_at(fir_tile),
            150,
            "terrain read uses BANK stack head (grass), not deeper UNPASS items"
        );
        assert!(
            world.fillmap_waypoints_at(cid, fir_tile, Position::new(32360, 32294, 7)) < 0,
            "MovePossible must clear WAYPOINTS when UNPASS fir tree is in stack"
        );
    }

    #[test]
    fn is_fleeing_gate() {
        assert!(!is_fleeing(10, 5, false));
        assert!(is_fleeing(5, 5, false));
        assert!(!is_fleeing(5, 5, true));
    }

    #[test]
    fn is_in_spawn_range_chebyshev_and_z() {
        let spawn = Position::new(100, 100, 7);
        assert!(is_in_spawn_range(Position::new(110, 110, 7), spawn, 50, 2));
        assert!(!is_in_spawn_range(Position::new(200, 100, 7), spawn, 50, 2));
        assert!(!is_in_spawn_range(
            Position::new(100, 100, 10),
            spawn,
            50,
            2
        ));
    }

    /// Finding 17/17b — an ATTACKING monster follows its target beyond the home radius (leash
    /// skipped), while a roaming (Idle) monster is bounded by its per-home `home_radius`.
    #[test]
    fn chase_leash_skipped_when_attacking_bounded_when_roaming() {
        use crate::creature::{MonsterAiConfig, MonsterState};
        use crate::sim_harness::{beat_driven_world, ensure_walkable_tile, insert_monster_with_config};

        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        // Global despawn radius is large (50); the per-home radius is small (3).
        world.monster_world_config.despawn_radius = 50;

        let spawn = Position::new(100, 100, 7);
        let far = Position::new(110, 100, 7); // chebyshev 10: > home_radius 3, < despawn 50
        ensure_walkable_tile(&mut world.map, spawn, 1);
        ensure_walkable_tile(&mut world.map, far, 1);

        let monster = insert_monster_with_config(&mut world, "Rat", spawn, 200, MonsterAiConfig::default());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.spawn_position = spawn;
            m.home_radius = 3;
            m.state = MonsterState::Idle;
        }
        assert!(
            !world.monster_can_occupy_chase_tile(monster, far),
            "roaming monster must stay within its home radius"
        );

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::Attacking;
        }
        assert!(
            world.monster_can_occupy_chase_tile(monster, far),
            "ATTACKING monster must chase past the home radius"
        );
    }

    /// Finding 17b — with no per-home radius (`home_radius == 0`) the roam leash falls back to the
    /// global despawn radius (no behavior change for synthetic/test monsters).
    #[test]
    fn roam_leash_falls_back_to_despawn_radius_when_home_unset() {
        use crate::creature::{MonsterAiConfig, MonsterState};
        use crate::sim_harness::{beat_driven_world, ensure_walkable_tile, insert_monster_with_config};

        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        world.monster_world_config.despawn_radius = 50;

        let spawn = Position::new(100, 100, 7);
        let near = Position::new(110, 100, 7); // cheb 10 ≤ despawn 50
        ensure_walkable_tile(&mut world.map, spawn, 1);
        ensure_walkable_tile(&mut world.map, near, 1);

        let monster = insert_monster_with_config(&mut world, "Rat", spawn, 200, MonsterAiConfig::default());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.spawn_position = spawn;
            m.home_radius = 0;
            m.state = MonsterState::Idle;
        }
        assert!(
            world.monster_can_occupy_chase_tile(monster, near),
            "unset home_radius roams within the global despawn radius"
        );
    }

    #[test]
    fn is_within_walk_to_spawn_range_axis_box() {
        let spawn = Position::new(100, 100, 7);
        assert!(is_within_walk_to_spawn_range(
            Position::new(110, 110, 7),
            spawn,
            15
        ));
        assert!(!is_within_walk_to_spawn_range(
            Position::new(120, 100, 7),
            spawn,
            15
        ));
        assert!(is_within_walk_to_spawn_range(
            Position::new(100, 100, 7),
            spawn,
            15
        ));
    }

    #[test]
    fn compute_look_faces_target() {
        let from = Position::new(10, 10, 7);
        assert_eq!(
            compute_look_toward_target(from, Position::new(12, 10, 7), Direction::North),
            Direction::East
        );
        assert_eq!(
            compute_look_toward_target(from, Position::new(10, 8, 7), Direction::East),
            Direction::North
        );
    }
