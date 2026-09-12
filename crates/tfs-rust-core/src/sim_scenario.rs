//! Headless chase/kite scenario helpers for `chase_kite_sim`.
//!
//! C++ reference: `chase_kite_scenario.cc` `SpawnMonsterAppear`, `MoveCreatures`, `DrainTodoQueue`;
//! `tibia-game-master` `crmain.cc` `MoveCreatures`.
//! Unit-test world fixtures live in `test_support` (crate-private until Phase 5).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use slotmap::SlotMap;
use tfs_rust_common::Position;
use tfs_rust_common::ProtocolVersion;
use tfs_rust_common::enums::Direction;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::monsters::MonsterDatabase;
use tfs_rust_content::otbm::OtbmLoader;

use crate::creature::{CreatureKind, MonsterAiConfig, MonsterState};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::map::Map;
use crate::pathfinding::REVERSE_PATH_VIEW_RADIUS;
use crate::test_support::{
    beat_driven_world_with_synthetic_ground_data, ensure_walkable_tile,
    ensure_walkable_tile_if_absent, harness_at_wall, harness_clamp_delay, init_beat_driven_world,
    lay_synthetic_arena, load_items_db_for, seed_world_from_sim_env, set_harness_segment_ms,
    set_harness_wall_ms, synthetic_ground_type_for_waypoints, test_runtime,
};
use crate::tile::Tile;

pub use crate::test_support::{
    insert_monster_from_type, insert_monster_with_config, insert_player, sim_hero_player,
    sim_player_damage_monster,
};

/// Scenario step to first chase idle bucket in cyclops quad sim (`kite_cyclops_quad_chase.scenario`).
pub const HARNESS_APPEAR_IDLE_DEFER_MS: u64 = 2000;

/// Map source for `chase_kite_sim` — OTBM terrain (Rust) aligned with C++ `.sec` coords.
#[derive(Debug, Clone)]
pub struct SimMapConfig {
    pub data_dir: PathBuf,
    pub map_rel: String,
    /// When true, lay flat synthetic arena tiles instead of requiring OTBM walkability.
    pub synthetic_arena: bool,
}

/// Resolve data dir + OTBM path from env (defaults: repo `data/`, `world/forgotten.otbm`).
pub fn default_sim_map_config() -> SimMapConfig {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data_dir = std::env::var("TFS_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root.join("data"));
    let map_rel =
        std::env::var("TFS_MAP_OTBM").unwrap_or_else(|_| "world/forgotten.otbm".to_string());
    let synthetic_arena =
        std::env::var("TFS_KITE_SYNTHETIC_ARENA").is_ok_and(|v| !v.is_empty() && v != "0");
    SimMapConfig {
        data_dir,
        map_rel,
        synthetic_arena,
    }
}

/// Build a 772 beat-driven world from OTBM + `objects.srv` waypoint overlay (772 terrain costs).
/// C++ mirror: `.sec` map + `objects.srv` `Waypoints` — `map.cc`, `cract.cc`.
pub fn beat_driven_world_from_map(data_dir: &Path, map_rel: &str) -> Result<GameWorld, String> {
    let _guard = test_runtime().enter();
    let map_path = data_dir.join(map_rel);
    if !map_path.is_file() {
        return Err(format!(
            "OTBM not found: {} (set TFS_DATA_DIR / TFS_MAP_OTBM)",
            map_path.display()
        ));
    }

    let items_db = Arc::new(load_items_db_for(data_dir)?);
    let map_data = OtbmLoader::load_from_file(&map_path).map_err(|e| e.to_string())?;
    let mut items = SlotMap::default();
    let map = Map::from_map_data(map_data, items_db.as_ref(), &mut items);
    let mechanics = crate::formulas::load_mechanics(data_dir, ProtocolVersion::V772);
    let monsters_dir = data_dir.join("monster");
    let monsters_db = Arc::new(
        MonsterDatabase::load_dir(&monsters_dir, items_db.as_ref()).map_err(|e| e.to_string())?,
    );

    let world = init_beat_driven_world(map, items, items_db, monsters_db, mechanics);
    Ok(world)
}

/// Ensure explicit scenario tiles exist and are walkable on the loaded map.
pub fn validate_positions_walkable(
    map: &Map,
    positions: &[Position],
    label: &str,
) -> Result<(), String> {
    let mut bad = Vec::new();
    for pos in positions {
        if !map.is_walkable(*pos) {
            bad.push(format!("[{},{},{}]", pos.x, pos.y, pos.z));
        }
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{label} has {} unwalkable/missing tile(s) on OTBM map: {}",
            bad.len(),
            bad.join(", ")
        ))
    }
}

/// One OTBM tile from [`audit_otbm_route_tiles`] — P2 real-map route audit (`audit_realmap_route.py`).
///
/// C++ mirror: `.sec` `Content` first id + `objects.srv` flags vs OTBM ground stack (`map.cc`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtbmRouteTileAudit {
    pub x: u16,
    pub y: u16,
    pub z: u8,
    pub exists: bool,
    pub ground_id: Option<u16>,
    /// Raw OTB `ITEM_ATTR_SPEED` / 772 Waypoints; `-1` when tile or ground missing.
    pub wp: i32,
    pub walkable: bool,
}

/// Inspect OTBM ground id, terrain wp, and walkability for scripted route coordinates.
///
/// C++ reference: `TShortway::FillMap` ground check — `cract.cc`; `Map::isWalkable` — `map.cc`.
pub fn audit_otbm_route_tiles(
    map: &Map,
    items_db: &ItemDatabase,
    positions: &[Position],
) -> Vec<OtbmRouteTileAudit> {
    positions
        .iter()
        .map(|pos| {
            let tile = map.get_tile(*pos);
            let exists = tile.is_some();
            let ground_id = tile.and_then(|t| t.body().ground);
            let wp = ground_id
                .and_then(|gid| items_db.waypoints_raw_for_item(gid))
                .map(i32::from)
                .unwrap_or(-1);
            OtbmRouteTileAudit {
                x: pos.x,
                y: pos.y,
                z: pos.z,
                exists,
                ground_id,
                wp,
                walkable: map.is_walkable(*pos),
            }
        })
        .collect()
}

/// JSON lines for `chase_kite_sim --audit-route` stdout (`scripts/audit_realmap_route.py`).
pub fn write_audit_route_json(
    audits: &[OtbmRouteTileAudit],
    out: &mut impl std::io::Write,
) -> std::io::Result<()> {
    writeln!(out, "{{")?;
    writeln!(out, "  \"src\": \"rust\",")?;
    writeln!(out, "  \"tiles\": [")?;
    for (i, t) in audits.iter().enumerate() {
        let comma = if i + 1 < audits.len() { "," } else { "" };
        let gid = t
            .ground_id
            .map(|g| g.to_string())
            .unwrap_or_else(|| "null".to_string());
        writeln!(
            out,
            "    {{\"x\":{},\"y\":{},\"z\":{},\"exists\":{},\"ground_id\":{},\"wp\":{},\"walkable\":{}}}{comma}",
            t.x, t.y, t.z, t.exists, gid, t.wp, t.walkable
        )?;
    }
    writeln!(out, "  ]")?;
    writeln!(out, "}}")?;
    Ok(())
}

/// Ensure every tile in the scenario arena exists and is walkable on the loaded map.
pub fn validate_arena_walkable(
    map: &Map,
    cx: u16,
    cy: u16,
    radius: u16,
    z: u8,
) -> Result<(), String> {
    let r = radius as i32;
    let cx = cx as i32;
    let cy = cy as i32;
    let mut bad = Vec::new();
    for dx in -r..=r {
        for dy in -r..=r {
            let x = (cx + dx) as u16;
            let y = (cy + dy) as u16;
            let pos = Position::new(x, y, z);
            if !map.is_walkable(pos) {
                bad.push(format!("[{x},{y},{z}]"));
            }
        }
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "arena has {} unwalkable/missing tile(s) on OTBM map: {}",
            bad.len(),
            bad.join(", ")
        ))
    }
}

/// Replace BANK ground only — keeps OTBM items (`chase_kite_scenario.cc` `ClearBankObjects` + `AppendObject`).
pub fn overlay_synthetic_ground_in_arena(
    map: &mut Map,
    cx: u16,
    cy: u16,
    radius: u16,
    z: u8,
    waypoint: u16,
) -> u32 {
    let ground_type = synthetic_ground_type_for_waypoints(waypoint);
    let r = radius as i32;
    let cx = cx as i32;
    let cy = cy as i32;
    for dx in -r..=r {
        for dy in -r..=r {
            let x = (cx + dx) as u16;
            let y = (cy + dy) as u16;
            let pos = Position::new(x, y, z);
            if let Some(tile) = map.get_tile_mut(pos) {
                if let Tile::Normal(body) = tile {
                    body.ground = Some(ground_type);
                }
            } else {
                ensure_walkable_tile(map, pos, ground_type);
            }
        }
    }
    u32::from(waypoint)
}

/// C++ `LaySyntheticArena` when `arena_synthetic` — OTBM base + grass overlay, else flat arena.
pub fn beat_driven_world_for_kite_synthetic(
    data_dir: &Path,
    map_rel: &str,
    arena_center: (u16, u16),
    arena_radius: u16,
    z: u8,
    default_wp: u16,
) -> Result<GameWorld, String> {
    let fill_radius = arena_radius.saturating_add(REVERSE_PATH_VIEW_RADIUS as u16);
    if data_dir.is_dir() {
        let mut world = beat_driven_world_from_map(data_dir, map_rel)?;
        let min_wp = overlay_synthetic_ground_in_arena(
            &mut world.map,
            arena_center.0,
            arena_center.1,
            fill_radius,
            z,
            default_wp,
        );
        if min_wp != u32::from(default_wp) {
            return Err(format!(
                "synthetic overlay min_wp={min_wp} != default_wp={default_wp}"
            ));
        }
        Ok(world)
    } else {
        let mut world = beat_driven_world_with_synthetic_ground_data(data_dir, Some(default_wp))?;
        let min_wp = lay_synthetic_arena(
            &mut world.map,
            arena_center.0,
            arena_center.1,
            fill_radius,
            z,
            default_wp,
        );
        if min_wp != u32::from(default_wp) {
            return Err(format!(
                "synthetic arena min_wp={min_wp} != default_wp={default_wp}"
            ));
        }
        Ok(world)
    }
}

/// Chebyshev-1 step direction for harness `player_walk` (real-map kite routes).
///
/// C++ reference: `chase_kite_scenario.cc` `MoveKitePlayer` via `Move()`.
fn direction_to_adjacent(from: Position, to: Position) -> Result<Direction, String> {
    if from.z != to.z {
        return Err(format!(
            "player_walk: floor mismatch [{},{},{}] -> [{},{},{}]",
            from.x, from.y, from.z, to.x, to.y, to.z
        ));
    }
    let dx = to.x as i32 - from.x as i32;
    let dy = to.y as i32 - from.y as i32;
    let cheb = dx.abs().max(dy.abs());
    if cheb != 1 {
        return Err(format!(
            "player_walk: destination [{},{},{}] not adjacent to [{},{},{}]",
            to.x, to.y, to.z, from.x, from.y, from.z
        ));
    }
    let dir = match (dx, dy) {
        (0, -1) => Direction::North,
        (1, 0) => Direction::East,
        (0, 1) => Direction::South,
        (-1, 0) => Direction::West,
        (1, -1) => Direction::NorthEast,
        (-1, -1) => Direction::NorthWest,
        (1, 1) => Direction::SouthEast,
        (-1, 1) => Direction::SouthWest,
        _ => {
            return Err(format!(
                "player_walk: invalid adjacent delta ({dx},{dy}) to [{},{},{}]",
                to.x, to.y, to.z
            ));
        }
    };
    Ok(dir)
}

/// One legal harness step to an adjacent walkable tile — `MoveKitePlayer` / `Move()`, not teleport.
pub fn walk_player_adjacent(
    world: &mut GameWorld,
    player_id: CreatureId,
    dest: Position,
) -> Result<(), String> {
    let old_pos = world
        .creatures
        .get(player_id)
        .map(|k| k.position())
        .ok_or_else(|| "player not found".to_string())?;
    if old_pos == dest {
        return Ok(());
    }
    if !world.map.is_walkable(dest) {
        return Err(format!(
            "player_walk: destination [{},{},{}] not walkable on map",
            dest.x, dest.y, dest.z
        ));
    }
    let dir = direction_to_adjacent(old_pos, dest)?;
    if !world.try_walk(player_id, dir) {
        return Err(format!(
            "player_walk: move blocked to [{},{},{}]",
            dest.x, dest.y, dest.z
        ));
    }
    Ok(())
}

/// C++ `TCreature::SetOnMap` — relocate creature via `SearchLoginField(dist=1)`.
pub fn place_creature_login(
    world: &mut GameWorld,
    cid: CreatureId,
    requested: Position,
) -> Option<Position> {
    world.place_creature_login(cid, requested)
}

/// Wake monsters, acquire targets, then batch `ToDoYield` — `chase_kite_scenario.cc` `SpawnMonsterAppear`.
pub fn kite_monsters_appear_batch(world: &mut GameWorld, monster_ids: &[CreatureId]) {
    // C++ `EnsureMonstersSpawned` → `ResyncHarnessRng()` after spawn loot (`chase_kite_scenario.cc:537`).
    seed_world_from_sim_env(world);
    world.appear_monsters(monster_ids);
}

/// Wake monster and run appear/target acquisition — `monster_appear` scenario step.
pub fn kite_monster_appear(world: &mut GameWorld, monster_id: CreatureId) {
    world.appear_monster(monster_id);
}

/// Teleport player and fan out `CreatureMoveStimulus` — `operate.cc` `NotifyAllCreatures`.
pub fn teleport_player(
    world: &mut GameWorld,
    player_id: CreatureId,
    new_pos: Position,
) -> Result<(), String> {
    let old_pos = world
        .creatures
        .get(player_id)
        .map(|k| k.position())
        .ok_or_else(|| "player not found".to_string())?;
    if old_pos == new_pos {
        return Ok(());
    }
    if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player_id) {
        p.base.position = new_pos;
    }
    world.map.unregister_creature_at(old_pos, player_id);
    ensure_walkable_tile_if_absent(&mut world.map, new_pos);
    world.map.register_creature_at(new_pos, player_id);
    world.monster_dispatch_creature_move(player_id, old_pos, new_pos);
    Ok(())
}

/// Cyclops quad spawn layout — `kite_cyclops_quad_chase.scenario` (spawn order = idle drain order).
pub const CYCLOPS_QUAD_SPAWNS: [(u16, u16); 4] = [
    (32359, 32288), // far-N
    (32361, 32290), // east
    (32360, 32291), // south
    (32359, 32289), // NW
];

/// Build cyclops quad chase world through first idle @2000 ms — mirrors `kite_cyclops_quad_chase.scenario`.
///
/// Caller must prepare the arena via [`beat_driven_world_for_kite_synthetic`] (OTBM + grass overlay)
/// or an equivalent map. Returns `(nw_creature_id, player_id, player_pos_at_tick_2000)`.
pub fn setup_cyclops_quad_chase_to_tick_2000(
    world: &mut GameWorld,
) -> Result<(CreatureId, CreatureId, Position), String> {
    let z = 7u8;
    let player_start = Position::new(32360, 32290, z);
    let player_id = insert_player(world, sim_hero_player("Hero", player_start));
    world.map.register_creature_at(player_start, player_id);

    let mtype = world
        .monsters_db
        .monsters
        .get("cyclops")
        .cloned()
        .ok_or_else(|| "cyclops monster type not loaded".to_string())?;

    let mut config = MonsterAiConfig::from_monster_type(&mtype);
    config.is_hostile = true;
    config.melee_skill = 50;
    config.melee_attack = 30;
    config.armor = 17;
    config.defense = 24;
    config.target_distance = 1;
    config.talks = 5;

    let mut monster_ids = Vec::with_capacity(4);
    for (i, &(x, y)) in CYCLOPS_QUAD_SPAWNS.iter().enumerate() {
        let pos = Position::new(x, y, z);
        let mid = insert_monster_from_type(
            world,
            &mtype,
            &format!("Cyclops {}", i + 1),
            pos,
            mtype.speed as i32,
            config.clone(),
            MonsterState::Sleeping,
        );
        monster_ids.push(mid);
    }

    kite_monsters_appear_batch(world, &monster_ids);

    let kite_path = [
        Position::new(32362, 32290, z),
        Position::new(32364, 32290, z),
        Position::new(32364, 32292, z),
        Position::new(32362, 32294, z),
        Position::new(32360, 32294, z),
    ];
    set_sim_harness_wall_ms(Some(0));
    for &dest in &kite_path {
        teleport_player(world, player_id, dest)?;
        run_sim_tick(world);
    }

    set_sim_harness_wall_ms(Some(HARNESS_APPEAR_IDLE_DEFER_MS));
    move_creatures_explicit(world, HARNESS_APPEAR_IDLE_DEFER_MS);
    run_sim_tick(world);
    // Caller may run further drains — first chase idle @2000 runs during `run_sim_tick` above.

    let nw_id = monster_ids[3];
    let player_pos = Position::new(32360, 32294, z);
    Ok((nw_id, player_id, player_pos))
}

/// U-loop waypoints from `kite_cyclops_one_real.scenario` — wall ms after each `player_walk`.
const CYCLOPS_BOWL_ONE_REAL_WALKS: [(u64, u16, u16); 5] = [
    (200, 32450, 32065),
    (400, 32450, 32066),
    (600, 32451, 32066),
    (800, 32452, 32066),
    (1000, 32451, 32065),
];

/// Real-map cyclops bowl — through first `shortway` @200 ms (`kite_cyclops_one_real` step 1).
///
/// Loads OTBM terrain (no synthetic overlay). Returns `(cyclops_id, player_id, player_pos)`.
pub fn setup_cyclops_bowl_real_first_shortway(
    world: &mut GameWorld,
) -> Result<(CreatureId, CreatureId, Position), String> {
    let z = 7u8;
    let player_start = Position::new(32451, 32065, z);
    let cyclops_pos = Position::new(32453, 32065, z);

    let player_id = insert_player(world, sim_hero_player("Hero", player_start));
    world.map.register_creature_at(player_start, player_id);

    let mtype = world
        .monsters_db
        .monsters
        .get("cyclops")
        .cloned()
        .ok_or_else(|| "cyclops monster type not loaded".to_string())?;

    let mut config = MonsterAiConfig::from_monster_type(&mtype);
    config.is_hostile = true;
    config.melee_skill = 50;
    config.melee_attack = 30;
    config.armor = 17;
    config.defense = 24;
    config.target_distance = 1;
    config.talks = 5;

    let cyclops_id = insert_monster_from_type(
        world,
        &mtype,
        "Cyclops",
        cyclops_pos,
        55,
        config,
        MonsterState::Sleeping,
    );
    if place_creature_login(world, cyclops_id, cyclops_pos).is_none() {
        return Err("harness spawn: cannot place cyclops on map".into());
    }
    kite_monsters_appear_batch(world, &[cyclops_id]);
    set_sim_harness_wall_ms(Some(0));
    run_sim_tick(world);

    set_sim_harness_wall_ms(Some(200));
    move_creatures_explicit(world, 200);
    run_sim_tick(world);
    walk_player_adjacent(world, player_id, Position::new(32450, 32065, z))?;
    run_sim_tick(world);

    Ok((cyclops_id, player_id, player_start))
}

/// Real-map cyclops bowl — `kite_cyclops_one_real.scenario` through tick 2000 ms.
///
/// Loads OTBM terrain (no synthetic overlay). Returns `(cyclops_id, player_id, player_pos)`.
pub fn setup_cyclops_bowl_real_to_tick_2000(
    world: &mut GameWorld,
) -> Result<(CreatureId, CreatureId, Position), String> {
    let z = 7u8;
    let player_start = Position::new(32451, 32065, z);
    let cyclops_pos = Position::new(32453, 32065, z);

    let player_id = insert_player(world, sim_hero_player("Hero", player_start));
    world.map.register_creature_at(player_start, player_id);

    let mtype = world
        .monsters_db
        .monsters
        .get("cyclops")
        .cloned()
        .ok_or_else(|| "cyclops monster type not loaded".to_string())?;

    let mut config = MonsterAiConfig::from_monster_type(&mtype);
    config.is_hostile = true;
    config.melee_skill = 50;
    config.melee_attack = 30;
    config.armor = 17;
    config.defense = 24;
    config.target_distance = 1;
    config.talks = 5;

    let cyclops_id = insert_monster_from_type(
        world,
        &mtype,
        "Cyclops",
        cyclops_pos,
        55,
        config,
        MonsterState::Sleeping,
    );
    if place_creature_login(world, cyclops_id, cyclops_pos).is_none() {
        return Err("harness spawn: cannot place cyclops on map".into());
    }

    kite_monsters_appear_batch(world, &[cyclops_id]);
    set_sim_harness_wall_ms(Some(0));
    run_sim_tick(world);

    let mut wall = 0u64;
    for &(target_wall, x, y) in &CYCLOPS_BOWL_ONE_REAL_WALKS {
        let delta = target_wall.saturating_sub(wall);
        set_sim_harness_wall_ms(Some(target_wall));
        move_creatures_explicit(world, delta);
        run_sim_tick(world);
        walk_player_adjacent(world, player_id, Position::new(x, y, z))?;
        run_sim_tick(world);
        wall = target_wall;
    }

    set_sim_harness_wall_ms(Some(2000));
    move_creatures_explicit(world, 1000);
    run_sim_tick(world);

    Ok((cyclops_id, player_id, player_start))
}

/// Real-map cyclops bowl — dual spawn through first `go_exec` bucket @400 ms.
///
/// Mirrors `kite_cyclops_two_real.scenario` phase A step 1. Returns
/// `(east_cyclops_id, north_cyclops_id, player_id)`.
pub fn setup_cyclops_bowl_real_dual_to_tick_400(
    world: &mut GameWorld,
) -> Result<(CreatureId, CreatureId, CreatureId), String> {
    let z = 7u8;
    let player_start = Position::new(32451, 32065, z);
    let east_spawn = Position::new(32453, 32065, z);
    let north_spawn = Position::new(32454, 32066, z);

    let player_id = insert_player(world, sim_hero_player("Hero", player_start));
    world.map.register_creature_at(player_start, player_id);

    let mtype = world
        .monsters_db
        .monsters
        .get("cyclops")
        .cloned()
        .ok_or_else(|| "cyclops monster type not loaded".to_string())?;

    let mut config = MonsterAiConfig::from_monster_type(&mtype);
    config.is_hostile = true;
    config.melee_skill = 50;
    config.melee_attack = 30;
    config.armor = 17;
    config.defense = 24;
    config.target_distance = 1;
    config.talks = 5;

    let mut monster_ids = Vec::with_capacity(2);
    for spawn_pos in [east_spawn, north_spawn] {
        let mid = insert_monster_from_type(
            world,
            &mtype,
            "Cyclops",
            spawn_pos,
            55,
            config.clone(),
            MonsterState::Sleeping,
        );
        if place_creature_login(world, mid, spawn_pos).is_none() {
            return Err(format!(
                "harness spawn: cannot place cyclops at {spawn_pos:?}"
            ));
        }
        monster_ids.push(mid);
    }

    kite_monsters_appear_batch(world, &monster_ids);
    set_sim_harness_wall_ms(Some(0));
    run_sim_tick(world);

    set_sim_harness_wall_ms(Some(200));
    move_creatures_explicit(world, 200);
    drain_todo_queue_once(world);
    walk_player_adjacent(world, player_id, Position::new(32450, 32065, z))?;
    run_sim_tick(world);

    set_sim_harness_wall_ms(Some(400));
    move_creatures_explicit(world, 200);
    drain_todo_queue_once(world);
    walk_player_adjacent(world, player_id, Position::new(32450, 32066, z))?;
    run_sim_tick(world);
    drain_todo_queue_once(world);
    run_sim_tick(world);

    Ok((monster_ids[0], monster_ids[1], player_id))
}

/// Rat melee kite layout — `kite_rat_melee.scenario` (player + single rat).
pub fn setup_kite_rat_melee_spawn(
    world: &mut GameWorld,
) -> Result<(CreatureId, CreatureId), String> {
    let z = 7u8;
    let player_start = Position::new(32360, 32290, z);
    let rat_pos = Position::new(32361, 32290, z);
    let player_id = insert_player(world, sim_hero_player("Hero", player_start));
    world.map.register_creature_at(player_start, player_id);

    let mtype = world
        .monsters_db
        .monsters
        .get("rat")
        .cloned()
        .ok_or_else(|| "rat monster type not loaded".to_string())?;

    let mut config = MonsterAiConfig::from_monster_type(&mtype);
    config.is_hostile = true;
    config.melee_skill = 15;
    config.melee_attack = 7;
    config.armor = 1;
    config.defense = 3;
    config.target_distance = 1;

    let monster_id = insert_monster_from_type(
        world,
        &mtype,
        "Rat",
        rat_pos,
        mtype.speed as i32,
        config,
        MonsterState::Sleeping,
    );
    Ok((player_id, monster_id))
}

/// Replay `kite_rat_melee.scenario` through `wall_ms` (0 | 2000 | 4000 | 6000).
pub fn setup_kite_rat_melee_to_tick(
    world: &mut GameWorld,
    player_id: CreatureId,
    monster_id: CreatureId,
    wall_ms: u64,
) -> Result<(), String> {
    let z = 7u8;
    kite_monsters_appear_batch(world, &[monster_id]);
    set_sim_harness_wall_ms(Some(0));
    run_sim_tick(world);

    let kite_steps: &[(u64, u16, u16)] = &[
        (2_000, 32362, 32290),
        (4_000, 32363, 32290),
        (6_000, 32363, 32292),
    ];
    for &(wall, x, y) in kite_steps {
        if wall > wall_ms {
            break;
        }
        set_sim_harness_wall_ms(Some(wall));
        run_sim_tick(world);
        teleport_player(world, player_id, Position::new(x, y, z))?;
        run_sim_tick(world);
    }
    Ok(())
}

/// Write Rust FillMap dump JSON when `TFS_FILLMAP_DUMP=1` — P2.5a artifact for `compare_fill_walkable.py`.
pub fn write_fill_walkable_dump_json(
    world: &GameWorld,
    cid: CreatureId,
    target: Position,
    path: &Path,
) -> std::io::Result<()> {
    use crate::monster_ai::TShortwayFillTile;
    use std::io::Write;

    let (state, tiles) =
        world.dump_tshortway_fill_walkable_viewport(cid, target, REVERSE_PATH_VIEW_RADIUS);
    let origin = world
        .creatures
        .get(cid)
        .map(|k| k.position())
        .unwrap_or(target);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = std::fs::File::create(path)?;
    writeln!(out, "{{")?;
    writeln!(out, "  \"src\": \"rust\",")?;
    writeln!(out, "  \"tick\": {},", world.server_ms())?;
    writeln!(
        out,
        "  \"monster_state\": \"{}\",",
        format!("{state:?}").to_ascii_lowercase()
    )?;
    writeln!(
        out,
        "  \"start\": {{\"x\":{},\"y\":{},\"z\":{}}},",
        origin.x, origin.y, origin.z
    )?;
    writeln!(out, "  \"visible\": {},", REVERSE_PATH_VIEW_RADIUS)?;
    writeln!(out, "  \"tiles\": [")?;
    for (i, TShortwayFillTile { pos, walkable, wp }) in tiles.iter().enumerate() {
        let comma = if i + 1 < tiles.len() { "," } else { "" };
        writeln!(
            out,
            "    {{\"x\":{},\"y\":{},\"z\":{},\"wp\":{},\"walkable\":{}}}{comma}",
            pos.x, pos.y, pos.z, wp, walkable
        )?;
    }
    writeln!(out, "  ]")?;
    writeln!(out, "}}")?;
    Ok(())
}

/// Full 772 beat advance including subsystem semantics — use for ProcessSkills/oracle tests.
pub fn advance_scenario_beat(world: &mut GameWorld, delay_ms: u64) {
    world.advance_beat(delay_ms);
}

/// C++ `MoveCreatures` wrapper — wall clamp lives here; clock+drain is [`GameWorld::move_creatures`].
///
/// When the module scenario wall is set, `delay_ms` is clamped so `server_ms` never exceeds it.
/// Use [`move_creatures_explicit`] for scenario `advance_ms` steps.
pub fn move_creatures(world: &mut GameWorld, delay_ms: u64) {
    move_creatures_impl(world, delay_ms, true);
}

/// Scenario `advance_ms` — always applies the full delay (wall is raised separately).
pub fn move_creatures_explicit(world: &mut GameWorld, delay_ms: u64) {
    move_creatures_impl(world, delay_ms, false);
}

fn move_creatures_impl(world: &mut GameWorld, delay_ms: u64, respect_wall: bool) {
    let requested = delay_ms;
    let delay_ms = if respect_wall {
        harness_clamp_delay(world.server_ms(), delay_ms)
    } else {
        delay_ms
    };
    if respect_wall && requested > 0 && delay_ms == 0 {
        return;
    }
    world.move_creatures(delay_ms);
}

/// Max ms this drain round may advance — `None` means uncapped (production paths).
pub fn set_sim_harness_wall_ms(wall_ms: Option<u64>) {
    set_harness_wall_ms(wall_ms);
}

/// Last scenario `advance_ms` — retained for future `chase_kite_sim` `AdvanceMs` wiring.
pub fn set_sim_harness_segment_ms(segment_ms: Option<u64>) {
    set_harness_segment_ms(segment_ms);
}

/// C++ `MoveCreatures(0)` — drain due todos without advancing the clock.
pub fn drain_todo_queue_once(world: &mut GameWorld) {
    world.move_creatures(0);
}

/// C++ `DrainTodoQueue` — `chase_kite_scenario.cc` (bounded `MoveCreatures` rounds).
pub fn run_sim_tick(world: &mut GameWorld) {
    const MAX_ROUNDS: usize = 64;
    for _ in 0..MAX_ROUNDS {
        let Some(exec) = world.next_todo_execution_ms() else {
            break;
        };
        if exec > world.server_ms() {
            if harness_at_wall(world.server_ms()) {
                break;
            }
            let delta = exec - world.server_ms();
            let delta = harness_clamp_delay(world.server_ms(), delta);
            if delta == 0 {
                break;
            }
            move_creatures(world, delta);
            continue;
        }
        move_creatures(world, 0);
    }
}

#[cfg(test)]
#[path = "sim_scenario_tests.rs"]
mod scenario_tests;
