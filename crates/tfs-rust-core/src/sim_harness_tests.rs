use super::*;
use tfs_rust_common::Position;

#[test]
fn synthetic_arena_min_wp_matches_default_wp() {
    let mut world = beat_driven_world_with_synthetic_ground(Some(150));
    let min_wp = lay_synthetic_arena(&mut world.map, 100, 100, 3, 7, 150);
    assert_eq!(min_wp, 150);
    let pos = Position::new(100, 100, 7);
    assert!(world.map.is_walkable(pos));
    assert_eq!(world.map.get_tile(pos).unwrap().body().ground, Some(102));
    assert_eq!(
        world.tile_ground_speed(world.map.get_tile(pos).unwrap().body()),
        150
    );
}

#[test]
fn move_creatures_clamps_to_harness_wall() {
    let mut world = beat_driven_world();
    set_sim_harness_wall_ms(Some(2_000));
    world.server_ms = 500;
    move_creatures(&mut world, 5_000);
    assert_eq!(world.server_ms, 2_000);
}

#[test]
fn move_creatures_explicit_ignores_wall() {
    let mut world = beat_driven_world();
    set_sim_harness_wall_ms(Some(2_000));
    world.server_ms = 0;
    move_creatures_explicit(&mut world, 2_000);
    assert_eq!(world.server_ms, 2_000);
}

#[test]
fn run_sim_tick_stops_at_harness_wall() {
    let mut world = beat_driven_world();
    let pos = Position::new(100, 100, 7);
    let cid = insert_monster(&mut world, "Rat", pos, 200);
    set_sim_harness_wall_ms(Some(6_000));
    world.schedule_creature_wakeup(cid, 20_000);
    run_sim_tick(&mut world);
    assert!(world.server_ms <= 6_000);
    let _ = cid;
}

#[test]
fn batch_appear_defers_idle_then_yields_once() {
    use crate::creature::MonsterState;
    use crate::test_world::support::{ensure_walkable_tile, test_player};

    let mut world = beat_driven_world();
    let ppos = Position::new(100, 100, 7);
    let mpos = Position::new(101, 100, 7);
    ensure_walkable_tile(&mut world.map, ppos, 150);
    ensure_walkable_tile(&mut world.map, mpos, 150);
    let player = insert_player(&mut world, test_player("Hero", ppos));
    world.map.register_creature_at(ppos, player);
    let monster = insert_monster(&mut world, "Rat", mpos, 200);
    if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
        m.is_hostile = true;
        m.state = MonsterState::Sleeping;
        m.is_idle = true;
    }
    appear_monster_without_idle(&mut world, monster);
    assert!(
        world.creature_todo_queue_empty(monster),
        "appear without batch yield must not enqueue ToDoWait yet"
    );
    world.creature_todo_yield(monster);
    assert!(
        !world.creature_todo_queue_empty(monster),
        "batch yield must enqueue Wait(0)"
    );
}

/// Quad cyclops — appear batch yields at server_ms+1; no inline idle on appear beat.
#[test]
fn batch_appear_quad_yields_next_beat_not_inline_idle() {
    use crate::creature::MonsterState;
    use crate::test_world::support::{ensure_walkable_tile, test_player};

    let mut world = beat_driven_world();
    let center = Position::new(32360, 32290, 7);
    let spawns = [
        Position::new(32360, 32289, 7),
        Position::new(32361, 32290, 7),
        Position::new(32360, 32291, 7),
        Position::new(32359, 32290, 7),
    ];
    for pos in [center].into_iter().chain(spawns) {
        ensure_walkable_tile(&mut world.map, pos, 150);
    }
    let player = insert_player(&mut world, test_player("Hero", center));
    world.map.register_creature_at(center, player);
    let mut monster_ids = Vec::new();
    for (i, &mpos) in spawns.iter().enumerate() {
        let mid = insert_monster(&mut world, "Cyclops", mpos, 55);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mid) {
            m.is_hostile = true;
            m.state = MonsterState::Sleeping;
            m.is_idle = true;
            m.base.name = format!("Cyclops {}", i + 1);
        }
        monster_ids.push(mid);
    }
    kite_monsters_appear_batch(&mut world, &monster_ids);
    for &mid in &monster_ids {
        assert_eq!(
            world.creatures.get(mid).and_then(|k| k.base().next_wakeup),
            Some(1),
            "ToDoYield + ToDoStart +1 clamp arms first drain at server_ms+1"
        );
    }
    set_sim_harness_wall_ms(Some(0));
    run_sim_tick(&mut world);
    for &mid in &monster_ids {
        assert!(
            world
                .creatures
                .get(mid)
                .and_then(|k| match k {
                    CreatureKind::Monster(m) => m.idle_stimulus_last_ms,
                    _ => None,
                })
                .is_none(),
            "appear-step drain must not run idle before ms+1 wakeup"
        );
    }
}

/// Cyclops quad — sibling tiles must block `TShortway` fill (`crnonpl.cc:2216` Unpushable).
#[test]
fn cyclops_quad_sibling_tiles_block_chase_fill_walkable() {
    use crate::creature::{MonsterAiConfig, MonsterState};
    use crate::pathfinding::REVERSE_PATH_VIEW_RADIUS;

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
    let player = insert_player(
        &mut world,
        crate::test_world::support::test_player("Hero", Position::new(32360, 32294, 7)),
    );
    world
        .map
        .register_creature_at(Position::new(32360, 32294, 7), player);
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
            MonsterState::Sleeping,
        );
        ids.push(mid);
    }
    kite_monsters_appear_batch(&mut world, &ids);
    let c1 = ids[0];
    let c4_pos = spawns[3];
    assert!(
        !world.monster_tshortway_fill_walkable(c1, c4_pos, Position::new(32360, 32294, 7)),
        "far-N cyclops must not plan through NW sibling tile"
    );
    let tile = world.map.get_tile(c4_pos).expect("sibling tile");
    assert!(
        tile.body().creatures.contains(&ids[3]),
        "NW cyclops must be registered on map tile"
    );
}

/// P2.5e — NW cyclops first diagonal `go_exec` fires after idle@2000 and advance to 4000.
#[test]
fn cyclops_quad_nw_go_exec_at_tick_4000() {
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
    let Ok((nw_id, _, _)) = setup_cyclops_quad_chase_to_tick_2000(&mut world) else {
        return;
    };
    assert_eq!(world.server_ms, HARNESS_APPEAR_IDLE_DEFER_MS);

    let nw_base = world.creatures.get(nw_id).unwrap().base();
    assert!(
        nw_base.follow_target.is_some(),
        "appear batch must acquire chase target before tick 4000 (wakeup={:?})",
        nw_base.next_wakeup
    );

    set_sim_harness_wall_ms(Some(4_000));
    move_creatures_explicit(&mut world, 2_000);
    run_sim_tick(&mut world);

    let nw_pos = world.creatures.get(nw_id).map(|k| k.position());
    assert_ne!(
        nw_pos,
        Some(Position::new(32359, 32289, 7)),
        "NW cyclops must leave spawn after go_exec window through tick 4000"
    );
    assert_eq!(world.server_ms, 4_000);
}

/// P2.5g — all four cyclops `go_exec` positions @4000 (structural heap drain order).
#[test]
fn cyclops_quad_go_exec_order_at_tick_4000() {
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
    let Ok((_, _, _)) = setup_cyclops_quad_chase_to_tick_2000(&mut world) else {
        return;
    };

    set_sim_harness_wall_ms(Some(4_000));
    move_creatures_explicit(&mut world, 2_000);
    run_sim_tick(&mut world);

    let spawns = CYCLOPS_QUAD_SPAWNS;
    let mut by_label: Vec<(u8, Position)> = world
        .creatures
        .iter()
        .filter_map(|(_, k)| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            let label = m.base.name.strip_prefix("Cyclops ")?;
            let idx: u8 = label.parse().ok()?;
            Some((idx, m.base.position))
        })
        .collect();
    by_label.sort_by_key(|(idx, _)| *idx);
    let positions: Vec<Position> = by_label.into_iter().map(|(_, p)| p).collect();
    assert_eq!(positions.len(), 4);
    for (i, pos) in positions.iter().enumerate() {
        let spawn = Position::new(spawns[i].0, spawns[i].1, 7);
        assert_ne!(
            *pos,
            spawn,
            "cyclops {} must leave spawn after go_exec @4000",
            i + 1
        );
    }
    assert_eq!(
        positions[3],
        Position::new(32358, 32290, 7),
        "NW cyclops (spawn 4) diagonal go_exec @4000"
    );
    assert_eq!(world.server_ms, 4_000);
}

/// P2.5 — NW cyclops FillMap dump @ tick=2000 matches scenario posture for parity diff.
#[test]
fn cyclops_quad_nw_fill_walkable_dump_at_tick_2000() {
    use crate::creature::MonsterState;
    use crate::monster_ai::TShortwayFillTile;
    use std::path::PathBuf;

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
    let Ok((nw_id, _player_id, player_pos)) = setup_cyclops_quad_chase_to_tick_2000(&mut world)
    else {
        return;
    };
    assert_eq!(world.server_ms, HARNESS_APPEAR_IDLE_DEFER_MS);
    // Deferred appear arms `next_wakeup@2000` — clear so idle can run (FillMap moment).
    if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(nw_id) {
        m.base.next_wakeup = None;
    }
    world.monster_idle_stimulus(nw_id);
    let (state, tiles) =
        world.dump_tshortway_fill_walkable_viewport(nw_id, player_pos, REVERSE_PATH_VIEW_RADIUS);
    assert_eq!(
        state,
        MonsterState::Attacking,
        "NW cyclops must be ATTACKING before FillMap at tick=2000"
    );

    let priority = [
        Position::new(32359, 32290, 7),
        Position::new(32358, 32289, 7),
        Position::new(32360, 32289, 7),
    ];
    for pos in priority {
        let Some(TShortwayFillTile { walkable, wp, .. }) = tiles.iter().find(|t| t.pos == pos)
        else {
            panic!("priority tile {pos:?} missing from viewport dump");
        };
        eprintln!("fill_walkable {pos:?} walkable={walkable} wp={wp}");
    }

    if std::env::var("TFS_FILLMAP_DUMP").is_ok_and(|v| !v.is_empty() && v != "0") {
        let out = std::env::var("TFS_FILLMAP_DUMP_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../log/fill_walkable_rust_nw.json")
            });
        write_fill_walkable_dump_json(&world, nw_id, player_pos, &out)
            .expect("write fill_walkable dump");
        eprintln!("wrote {}", out.display());
    }
}

/// P1 — real-map cyclops bowl FillMap dump @ tick=2000 for parity diff vs C++ `.sec`.
#[test]
fn cyclops_bowl_real_fill_walkable_dump_at_tick_2000() {
    use crate::creature::MonsterState;
    use crate::monster_ai::TShortwayFillTile;
    use std::path::PathBuf;

    let cfg = default_sim_map_config();
    if !cfg.data_dir.is_dir() {
        return;
    }
    let Ok(mut world) = beat_driven_world_from_map(&cfg.data_dir, &cfg.map_rel) else {
        return;
    };
    let Ok((cyclops_id, _player_id, player_pos)) =
        setup_cyclops_bowl_real_first_shortway(&mut world)
    else {
        return;
    };
    assert_eq!(world.server_ms, 200);
    if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(cyclops_id) {
        m.base.next_wakeup = None;
    }
    world.monster_idle_stimulus(cyclops_id);
    let (state, tiles) = world.dump_tshortway_fill_walkable_viewport(
        cyclops_id,
        player_pos,
        REVERSE_PATH_VIEW_RADIUS,
    );
    assert_eq!(
        state,
        MonsterState::Attacking,
        "cyclops bowl must be ATTACKING before FillMap at tick=2000"
    );

    let priority = [
        Position::new(32451, 32065, 7),
        Position::new(32458, 32065, 7),
    ];
    for pos in priority {
        let Some(TShortwayFillTile { walkable, wp, .. }) = tiles.iter().find(|t| t.pos == pos)
        else {
            panic!("priority tile {pos:?} missing from viewport dump");
        };
        eprintln!("fill_walkable {pos:?} walkable={walkable} wp={wp}");
    }

    if std::env::var("TFS_FILLMAP_DUMP").is_ok_and(|v| !v.is_empty() && v != "0") {
        let out = std::env::var("TFS_FILLMAP_DUMP_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../log/fill_walkable_rust_cyclops_bowl.json")
            });
        write_fill_walkable_dump_json(&world, cyclops_id, player_pos, &out)
            .expect("write fill_walkable dump");
        eprintln!("wrote {}", out.display());
    }
}

/// P3 — final north kite @6000 must not idle-repath on empty `walk_queue` (C++ `CreatureMoveStimulus`).
#[test]
fn kite_rat_melee_no_idle_repath_on_final_kite_at_6000() {
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
    let Ok((player_id, monster_id)) = setup_kite_rat_melee_spawn(&mut world) else {
        return;
    };
    setup_kite_rat_melee_to_tick(&mut world, player_id, monster_id, 4_000)
        .expect("kite to tick 4000");

    set_sim_harness_wall_ms(Some(6_000));
    teleport_player(&mut world, player_id, Position::new(32363, 32292, 7))
        .expect("final north kite");
    run_sim_tick(&mut world);
    assert_eq!(world.server_ms, 6_000);
    assert!(
        !world
            .creatures
            .get(monster_id)
            .is_some_and(|k| k.base().todo.has_go() && k.base().force_update_follow_path),
        "final kite @6000 must not idle-repath after deferred player move"
    );
}

/// OTBM kite lab — rat/player/dance tiles must be walkable on forgotten.otbm.
#[test]
fn harness_place_cyclops_bowl_relocates_east_of_scripted_spawn() {
    use crate::creature::{MonsterAiConfig, MonsterState};

    let cfg = default_sim_map_config();
    if !cfg.data_dir.is_dir() {
        return;
    }
    let Ok(mut world) = beat_driven_world_from_map(&cfg.data_dir, &cfg.map_rel) else {
        return;
    };
    let requested = Position::new(32453, 32065, 7);
    let expected = Position::new(32454, 32065, 7);
    let mtype = match world.monsters_db.monsters.get("cyclops").cloned() {
        Some(t) => t,
        None => return,
    };
    let config = MonsterAiConfig::from_monster_type(&mtype);
    let cid = insert_monster_from_type(
        &mut world,
        &mtype,
        "Cyclops",
        requested,
        55,
        config,
        MonsterState::Sleeping,
    );
    let placed = harness_place_creature_login(&mut world, cid, requested);
    assert_eq!(placed, Some(expected));
    assert_eq!(world.creatures.get(cid).unwrap().position(), expected);
}

/// P3 — real-map first `player_walk` @200 arms chase under production scheduler (no appear-defer).
#[test]
fn cyclops_bowl_real_first_chase_on_player_walk_at_200() {
    let cfg = default_sim_map_config();
    if !cfg.data_dir.is_dir() {
        return;
    }
    let Ok(mut world) = beat_driven_world_from_map(&cfg.data_dir, &cfg.map_rel) else {
        return;
    };
    let Ok((cyclops_id, _, _)) = setup_cyclops_bowl_real_first_shortway(&mut world) else {
        return;
    };

    assert_eq!(world.server_ms, 200);
    assert!(
        world.creatures.get(cyclops_id).is_some_and(|k| {
            matches!(k, CreatureKind::Monster(m) if m.base.follow_target.is_some())
        }),
        "cyclops must acquire follow target during appear batch before first player_walk @200"
    );
}

/// P6 G1 — U-loop kite must not inline-repath on every player_walk; chase drains through tick 2000.
#[test]
fn cyclops_bowl_real_uloop_chase_drains_to_tick_2000() {
    let cfg = default_sim_map_config();
    if !cfg.data_dir.is_dir() {
        return;
    }
    let Ok(mut world) = beat_driven_world_from_map(&cfg.data_dir, &cfg.map_rel) else {
        return;
    };
    let cyclops_spawn = Position::new(32453, 32065, 7);
    let (cyclops_id, _player_id, _) =
        setup_cyclops_bowl_real_to_tick_2000(&mut world).expect("cyclops bowl U-loop");

    assert_eq!(world.server_ms, 2000);
    let Some(CreatureKind::Monster(m)) = world.creatures.get(cyclops_id) else {
        panic!("cyclops missing");
    };
    assert_eq!(m.state, MonsterState::Attacking);
    assert!(
        m.base.follow_target.is_some() && m.base.attack_target.is_some(),
        "cyclops must keep combat target through U-loop"
    );
    assert_ne!(
        m.base.position, cyclops_spawn,
        "cyclops must have executed at least one go_exec step by tick 2000"
    );
    assert!(
        !world.monster_close_chase_batch_in_flight(cyclops_id)
            || m.base.todo.has_go()
            || !m.base.walk_queue.is_empty()
            || m.base.next_wakeup.is_some(),
        "mid-U-loop repath storm must not leave monster in stale cleared state"
    );
}

/// P6 T1 — dual cyclops `go_exec` tie @400 (`todo_queue` unit test + movement-core gate).
#[test]
#[ignore = "end-to-end positions validated by movement-core gate; harness setup uses explicit wall ms"]
fn cyclops_bowl_real_dual_go_exec_order_at_tick_400() {
    let cfg = default_sim_map_config();
    if !cfg.data_dir.is_dir() {
        return;
    }
    let Ok(mut world) = beat_driven_world_from_map(&cfg.data_dir, &cfg.map_rel) else {
        return;
    };
    let Ok((east_id, north_id, _player_id)) = setup_cyclops_bowl_real_dual_to_tick_400(&mut world)
    else {
        return;
    };

    assert_eq!(world.server_ms, 400);
    let east_pos = world.creatures.get(east_id).map(|k| k.position());
    let north_pos = world.creatures.get(north_id).map(|k| k.position());
    assert_eq!(
        (north_pos, east_pos),
        (
            Some(Position::new(32453, 32066, 7)),
            Some(Position::new(32454, 32064, 7)),
        ),
        "dual cyclops go_exec @400 must match C++ bowl drain (north then east)"
    );
}

#[test]
fn kite_lab_tiles_walkable_on_otbm_when_data_present() {
    let cfg = default_sim_map_config();
    let Ok(world) = beat_driven_world_from_map(&cfg.data_dir, &cfg.map_rel) else {
        return;
    };
    let positions = [
        Position::new(32361, 32290, 7),
        Position::new(32363, 32290, 7),
        Position::new(32363, 32292, 7),
        Position::new(32361, 32291, 7),
    ];
    for pos in positions {
        assert!(
            world.map.is_walkable(pos),
            "kite lab tile [{},{},{}] must be walkable on OTBM",
            pos.x,
            pos.y,
            pos.z
        );
    }
}

#[test]
fn walk_player_adjacent_rejects_non_adjacent_destination() {
    let mut world = beat_driven_world_with_synthetic_ground(Some(150));
    let start = Position::new(100, 100, 7);
    let far = Position::new(102, 100, 7);
    lay_synthetic_arena(&mut world.map, 100, 100, 3, 7, 150);
    let player = insert_player(&mut world, sim_hero_player("Hero", start));
    world.map.register_creature_at(start, player);
    let err = walk_player_adjacent(&mut world, player, far).unwrap_err();
    assert!(err.contains("not adjacent"));
}

#[test]
fn walk_player_adjacent_rejects_unwalkable_destination() {
    let mut world = beat_driven_world_with_synthetic_ground(Some(150));
    let start = Position::new(100, 100, 7);
    lay_synthetic_arena(&mut world.map, 100, 100, 1, 7, 150);
    let player = insert_player(&mut world, sim_hero_player("Hero", start));
    world.map.register_creature_at(start, player);
    let blocked = Position::new(200, 200, 7);
    let err = walk_player_adjacent(&mut world, player, blocked).unwrap_err();
    assert!(err.contains("not walkable"));
}

#[test]
fn walk_player_adjacent_steps_cardinal_directions() {
    let mut world = beat_driven_world_with_synthetic_ground(Some(150));
    let start = Position::new(100, 100, 7);
    lay_synthetic_arena(&mut world.map, 100, 100, 3, 7, 150);
    let player = insert_player(&mut world, sim_hero_player("Hero", start));
    world.map.register_creature_at(start, player);

    for dest in [
        Position::new(101, 100, 7),
        Position::new(101, 101, 7),
        Position::new(100, 101, 7),
        Position::new(99, 101, 7),
        Position::new(99, 100, 7),
        Position::new(99, 99, 7),
        Position::new(100, 99, 7),
        Position::new(101, 99, 7),
        start,
    ] {
        walk_player_adjacent(&mut world, player, dest)
            .unwrap_or_else(|e| panic!("walk to {dest:?}: {e}"));
        assert_eq!(world.creatures.get(player).unwrap().position(), dest);
    }
}
