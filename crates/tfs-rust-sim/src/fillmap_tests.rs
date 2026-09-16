//! OTBM FillMap / cyclops-quad path tests — lived in `tfs-rust-core` until Phase 5.

use tfs_rust_common::Position;
use tfs_rust_common::enums::Direction;
use tfs_rust_core::creature::{CreatureKind, MonsterAiConfig, MonsterState};
use tfs_rust_core::pathfinding::{CHASE_PATH_MAX_STEPS, truncate_tshortway_go_queue};

use crate::scenario::{beat_driven_world_for_kite_synthetic, default_sim_map_config};
use crate::world::{insert_monster_from_type, insert_player, sim_hero_player};

#[test]
fn fillmap_terrain_reads_grass_bank_waypoints() {
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
        sim_hero_player("Hero", Position::new(32360, 32294, 7)),
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
fn cyclops_quad_far_n_path_avoids_nw_sibling_when_last() {
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
    let player = insert_player(&mut world, sim_hero_player("Hero", player_pos));
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
        tile.body().creatures().contains(&ids[3]),
        "NW cyclops must occupy map tile before path query"
    );
    assert!(
        world.fillmap_waypoints_at(far_n, nw_pos, player_pos) < 0,
        "fill walkable must reject NW sibling tile"
    );
    let fpp = world.monster_path_search_params(far_n, player, false, 1, false, false);
    let mut steps = world
        .get_creature_path_to_with_fpp(far_n, player_pos, &fpp)
        .expect("far-N chase path");
    steps = truncate_tshortway_go_queue(spawns[0], player_pos, steps, CHASE_PATH_MAX_STEPS, false);
    assert!(!steps.is_empty(), "path must not be empty");
    let first = spawns[0].offset(steps[0]);
    assert_ne!(
        first, nw_pos,
        "far-N first hop must not enter NW sibling tile (C++ `MovePossible` blocks unpushable)"
    );
}

/// Ignored until fresh C++ oracle — see `tasks/lessons.md` §59.
#[test]
fn cyclops_quad_nw_and_far_n_shortway_match_live_ref() {
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
        Position::new(32359, 32288, 7),
        Position::new(32361, 32290, 7),
        Position::new(32360, 32291, 7),
        Position::new(32359, 32289, 7),
    ];
    let player_pos = Position::new(32360, 32294, 7);
    let player = insert_player(&mut world, sim_hero_player("Hero", player_pos));
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

    let mut chase_path = |cid, start: Position| -> Vec<Position> {
        let fpp = world.monster_path_search_params(cid, player, false, 1, false, false);
        let raw = world
            .get_creature_path_to_with_fpp(cid, player_pos, &fpp)
            .expect("chase path");
        let steps =
            truncate_tshortway_go_queue(start, player_pos, raw, CHASE_PATH_MAX_STEPS, false);
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
