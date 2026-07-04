//! Headless simulation harness — shared world builders for unit tests and `chase_kite_sim`.
//!
//! C++ reference: `chase_kite_scenario.cc` `SpawnMonsterAppear`, `MoveCreatures`, `DrainTodoQueue`;
//! `tibia-game-master` test patterns; `GameWorld` tick — `game.cpp`, `crmain.cc`.

/// Scenario step to first chase idle bucket in cyclops quad sim (`kite_cyclops_quad_chase.scenario`).
pub const HARNESS_APPEAR_IDLE_DEFER_MS: u64 = 2000;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use slotmap::{Key, SlotMap};
use tfs_rust_common::enums::{Direction, SkullType};
use tfs_rust_common::Position;
use tfs_rust_common::ProtocolVersion;
use tfs_rust_content::groups::GroupDatabase;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::otb::ItemType;
use tfs_rust_content::otbm::{OtbmLoader, TownData};
use tfs_rust_content::vocations::VocationDatabase;
use tfs_rust_db::player::PlayerRecord;
use tfs_rust_db::DbPool;

use crate::combat::{CombatDamage, CombatParams};
use crate::config::ConfigManager;
use crate::creature::{
    CreatureBase, CreatureKind, Monster, MonsterAiConfig, MonsterState, Npc, Outfit, Player,
    PlayerEconomy, PlayerInventory, PlayerPersistBaseline, PlayerSkills, PlayerSocial,
};
use crate::event_dispatcher::NullEventDispatcher;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::map::{Map, SparseGrid};
use crate::pathfinding::{scan_min_terrain_waypoints, REVERSE_PATH_VIEW_RADIUS};
use crate::spawn::SpawnManager;
use crate::tile::{Tile, TileBody};
use tfs_rust_common::enums::CombatType;
use tfs_rust_common::enums::ZoneType;
use tfs_rust_common::ConnId;
use tfs_rust_content::monsters::MonsterDatabase;
use tfs_rust_content::monsters::{MonsterOutfit, MonsterType};

use crate::monster_ai::compute_look_toward_target;
use crate::walk::creature_turn_with_broadcast;

/// Headless sim scenario clock — caps `move_creatures` / `run_sim_tick` advance (`chase_kite_scenario.cc`).
/// Lives in this module only; production `GameWorld` never reads it (audit Finding 21 / Phase 5).
#[derive(Debug, Default, Clone, Copy)]
struct HarnessScenarioClock {
    wall_ms: Option<u64>,
    segment_ms: Option<u64>,
}

thread_local! {
    static HARNESS_SCENARIO_CLOCK: RefCell<HarnessScenarioClock> =
        RefCell::new(HarnessScenarioClock::default());
}

/// Reset scenario clock — call from beat-driven world builders.
pub fn reset_harness_scenario_clock() {
    HARNESS_SCENARIO_CLOCK.with(|c| *c.borrow_mut() = HarnessScenarioClock::default());
}

fn with_harness_clock<R>(f: impl FnOnce(&HarnessScenarioClock) -> R) -> R {
    HARNESS_SCENARIO_CLOCK.with(|c| f(&c.borrow()))
}

fn with_harness_clock_mut<R>(f: impl FnOnce(&mut HarnessScenarioClock) -> R) -> R {
    HARNESS_SCENARIO_CLOCK.with(|c| f(&mut c.borrow_mut()))
}

pub fn test_config() -> ConfigManager {
    let path = std::env::temp_dir().join(format!(
        "tfs_depot_test_config_{}_{}.lua",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::write(
        &path,
        r#"
depotFreeLimit = 2000
depotPremiumLimit = 10000
freePremium = false
"#,
    )
    .expect("write temp config.lua");
    ConfigManager::load(&path).expect("load temp config.lua")
}

pub fn test_player(name: &str, pos: Position) -> Player {
    test_player_base(name, pos)
}

/// 772 human hero for chase parity sim — matches C++ `TKiteSimPlayer` + `human.mon` race data.
/// C++ reference: `chase_kite_scenario.cc` `TKiteSimPlayer`; `runtime/mon/human.mon` `Defend=5`.
pub fn sim_hero_player(name: &str, pos: Position) -> Player {
    let mut p = test_player_base(name, pos);
    p.base.health = 150;
    p.base.max_health = 150;
    p.sim_melee_defense = 5;
    p
}

fn test_player_base(name: &str, pos: Position) -> Player {
    Player {
        base: CreatureBase {
            name: name.into(),
            position: pos,
            direction: Direction::North,
            health: 100,
            max_health: 100,
            outfit: Outfit::default(),
            speed: 220,
            base_speed: 220,
            skull: SkullType::None,
            drunkenness: 0,
            active_conditions: Vec::new(),
            walk_queue: Default::default(),
            walk_destinations: Default::default(),
            last_step: None,
            last_step_cost: 1,
            last_step_ground_speed: 150,
            next_walk_check: None,
            next_wakeup: None,
            last_step_server_ms: None,
            earliest_walk_server_ms: 0,
            earliest_spell_server_ms: 0,
            earliest_multiuse_server_ms: 0,
            walk_timer: Default::default(),
            cancel_next_walk: false,
            force_update_follow_path: false,
            walk_update_ticks: 0,
            is_updating_path: false,
            has_follow_path: false,
            movement_blocked: false,
            stairhop_blocked_until: None,
            follow_target: None,
            attack_target: None,
            master: None,
            damage_map: Default::default(),
            think_check_bucket: None,
            earliest_attack_ms: 0,
            earliest_defend_ms: 0,
            last_defend_ms: 0,
            todo: Default::default(),
            chase_mode: Default::default(),
            last_auto_walk_armed_ms: u64::MAX,
        },
        account_id: 1,
        guid: 1,
        group_id: 1,
        vocation_id: 0,
        level: 8,
        experience: 0,
        mana: 50,
        max_mana: 50,
        capacity: 40000,
        inventory: PlayerInventory::default(),
        skills: PlayerSkills {
            fist: 10,
            club: 10,
            sword: 10,
            axe: 10,
            dist: 10,
            shielding: 10,
            fishing: 10,
            maglevel: 0,
        },
        economy: PlayerEconomy {
            balance: 0,
            soul: 100,
        },
        social: PlayerSocial::default(),
        town_id: 1,
        premium_ends_at: 0,
        stamina_minutes: 2520,
        offline_training_ms: 0,
        spell_cooldown_end: HashMap::new(),
        spell_group_cooldown_end: HashMap::new(),
        operating_system: 0,
        otclient_v8: 0,
        ghost_mode: false,
        equipment_slots: std::array::from_fn(|_| None),
        inventory_weight: 0,
        items_light: Default::default(),
        inventory_abilities: [false; 11],
        shop_owner: None,
        vip_list: Vec::new(),
        health_hidden: false,
        last_activity: Instant::now(),
        last_command_round: 0,
        last_action_round: 0,
        food_remaining: 0,
        food_level: 0,
        earliest_logout_round: 0,
        last_ping_sent: Instant::now(),
        last_pong_at: Instant::now(),
        next_action_until: None,
        walk_action: None,
        walk_action_due: None,
        depot_chests: HashMap::new(),
        depot_lockers: HashMap::new(),
        inbox_root: None,
        last_depot_id: -1,
        persist: Some(PlayerPersistBaseline {
            player_row: minimal_player_record(name),
            spells: Vec::new(),
            storage: Vec::new(),
            depot: Vec::new(),
            inbox: Vec::new(),
            last_depot_id: -1,
        }),
        sim_melee_defense: 0,
    }
}

fn minimal_player_record(name: &str) -> PlayerRecord {
    PlayerRecord {
        id: 1,
        name: name.into(),
        account_id: 1,
        group_id: 1,
        sex: 0,
        vocation: 0,
        experience: 0,
        level: 8,
        maglevel: 0,
        health: 100,
        healthmax: 100,
        blessings: 0,
        mana: 50,
        manamax: 50,
        manaspent: 0,
        soul: 100,
        lookbody: 0,
        lookfeet: 0,
        lookhead: 0,
        looklegs: 0,
        looktype: 128,
        lookaddons: 0,
        posx: 100,
        posy: 100,
        posz: 7,
        cap: 400,
        lastlogin: 0,
        lastlogout: 0,
        lastip: 0,
        conditions: None,
        skulltime: 0,
        skull: 0,
        town_id: 1,
        balance: 0,
        offlinetraining_time: 0,
        offlinetraining_skill: 0,
        stamina: 2520,
        skill_fist: 10,
        skill_fist_tries: 0,
        skill_club: 10,
        skill_club_tries: 0,
        skill_sword: 10,
        skill_sword_tries: 0,
        skill_axe: 10,
        skill_axe_tries: 0,
        skill_dist: 10,
        skill_dist_tries: 0,
        skill_shielding: 10,
        skill_shielding_tries: 0,
        skill_fishing: 10,
        skill_fishing_tries: 0,
        direction: 0,
        save: 1,
        onlinetime: 0,
        deletion: 0,
        food_remaining: 0,
        food_level: 0,
    }
}

pub fn bag_item_type(server_id: u16) -> ItemType {
    let mut it = ItemType {
        group: ItemType::GROUP_CONTAINER,
        allow_pickupable: true,
        server_id,
        ..Default::default()
    };
    it.xml_attributes
        .insert("containersize".into(), "20".into());
    it
}

pub fn pickup_item_type(server_id: u16) -> ItemType {
    ItemType {
        allow_pickupable: true,
        moveable_override: Some(true),
        server_id,
        ..Default::default()
    }
}

/// Walkable synthetic ground for chase parity — OTB `ITEM_ATTR_SPEED` / 772 `WAYPOINTS`.
///
/// C++ mirror: `objects.srv` TypeID 102 (`grass`, `Waypoints=150`).
pub fn synthetic_ground_item_type(server_id: u16, waypoint: u16) -> ItemType {
    ItemType {
        group: ItemType::GROUP_GROUND,
        allow_pickupable: false,
        server_id,
        speed: waypoint,
        ..Default::default()
    }
}

fn register_synthetic_ground(items: &mut HashMap<u16, ItemType>, waypoint: u16) {
    items.insert(waypoint, synthetic_ground_item_type(waypoint, waypoint));
}

fn test_runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Runtime::new().expect("tokio runtime for tests"))
}

pub fn minimal_world() -> GameWorld {
    let _guard = test_runtime().enter();
    let mut items_map = HashMap::new();
    items_map.insert(1987u16, bag_item_type(1987));
    items_map.insert(2148u16, pickup_item_type(2148));
    let items_db = Arc::new(ItemDatabase {
        items: items_map,
        client_to_server: HashMap::new(),
    });

    let mut map = Map {
        width: 256,
        height: 256,
        grid: SparseGrid::new(),
        towns: HashMap::new(),
        waypoints: HashMap::new(),
    };
    map.towns.insert(
        1,
        TownData {
            id: 1,
            name: "Thais".into(),
            temple_position: Position::new(100, 100, 7),
        },
    );

    GameWorld::new(
        map,
        SlotMap::default(),
        Box::new(NullEventDispatcher),
        Rc::new(test_config()),
        DbPool::lazy_for_tests().expect("lazy db pool"),
        SpawnManager::from_zones(Vec::new()),
        items_db,
        Arc::new(MonsterDatabase {
            monsters: HashMap::new(),
        }),
        Arc::new(GroupDatabase {
            groups: HashMap::new(),
        }),
        Arc::new(VocationDatabase {
            vocations: HashMap::new(),
        }),
        None,
        tfs_rust_net::Codec::from_version(tfs_rust_common::ProtocolVersion::V1098)
            .expect("1098 codec"),
        crate::formulas::Mechanics::for_version(tfs_rust_common::ProtocolVersion::V1098),
    )
}

fn beat_driven_items_db(synthetic_waypoint: Option<u16>) -> ItemDatabase {
    let mut items_map = HashMap::new();
    items_map.insert(1987u16, bag_item_type(1987));
    items_map.insert(2148u16, pickup_item_type(2148));
    if let Some(wp) = synthetic_waypoint {
        register_synthetic_ground(&mut items_map, wp);
    }
    ItemDatabase {
        items: items_map,
        client_to_server: HashMap::new(),
    }
}

/// 772 beat-driven profile (`LinearGo` + reverse terrain path) for idle/todo/monster sims.
pub fn beat_driven_world() -> GameWorld {
    beat_driven_world_with_synthetic_ground(None)
}

/// Synthetic chase arena — uniform walkable tiles with pinned waypoint cost.
pub fn beat_driven_world_with_synthetic_ground(waypoint: Option<u16>) -> GameWorld {
    beat_driven_world_with_synthetic_ground_data(Path::new("/nonexistent"), waypoint)
        .unwrap_or_else(|_| panic!("synthetic world without data dir failed"))
}

/// Pinned waypoint for unit-test arenas — matches kite sim synthetic grass (`chase_kite_scenario.cc`).
pub const TEST_SYNTHETIC_GROUND_WP: u16 = 150;

/// 772 beat-driven world with synthetic terrain registered for `TShortway::FillMap`.
pub fn beat_driven_test_world() -> GameWorld {
    let mut world = beat_driven_world_with_synthetic_ground(Some(TEST_SYNTHETIC_GROUND_WP));
    world.walk_wake_tx = None;
    world.server_ms = 0;
    world.seed_parity_rng(42);
    world
}

/// Load item + monster databases from the data pack for chase sim spawn parity.
/// C++ reference: `Monsters::loadMonster` — `monsters.cpp`.
pub fn load_sim_content_dbs(
    data_dir: &Path,
    synthetic_ground_wp: Option<u16>,
) -> Result<(Arc<ItemDatabase>, Arc<MonsterDatabase>), String> {
    let mut items_db = load_items_db_for_772(data_dir)?;
    if let Some(wp) = synthetic_ground_wp {
        register_synthetic_ground(&mut items_db.items, wp);
    }
    let items_db = Arc::new(items_db);
    let monsters_dir = data_dir.join("monster");
    let monsters_db = Arc::new(
        MonsterDatabase::load_dir(&monsters_dir, items_db.as_ref()).map_err(|e| e.to_string())?,
    );
    Ok((items_db, monsters_db))
}

fn monster_outfit_to_sim(o: &MonsterOutfit) -> Outfit {
    Outfit {
        look_type: o.look_type,
        look_head: o.look_head,
        look_body: o.look_body,
        look_legs: o.look_legs,
        look_feet: o.look_feet,
        look_addons: o.look_addons,
    }
}

fn init_beat_driven_world(
    map: Map,
    items: SlotMap<crate::ids::ItemId, crate::item::Item>,
    items_db: Arc<ItemDatabase>,
    monsters_db: Arc<MonsterDatabase>,
    mechanics: crate::formulas::Mechanics,
) -> GameWorld {
    let mut world = GameWorld::new(
        map,
        items,
        Box::new(NullEventDispatcher),
        Rc::new(test_config()),
        DbPool::lazy_for_tests().expect("lazy db pool"),
        SpawnManager::from_zones(Vec::new()),
        items_db,
        monsters_db,
        Arc::new(GroupDatabase {
            groups: HashMap::new(),
        }),
        Arc::new(VocationDatabase {
            vocations: HashMap::new(),
        }),
        None,
        tfs_rust_net::Codec::from_version(tfs_rust_common::ProtocolVersion::V772)
            .expect("772 codec"),
        mechanics,
    );
    world.beat_driven_loop = true;
    world.walk_wake_tx = None;
    world.server_ms = 0;
    reset_harness_scenario_clock();
    world.init_sim_rng_from_env();
    world
}

/// Synthetic beat-driven world with data-pack items + monsters (E0/E6 loot roll).
pub fn beat_driven_world_with_synthetic_ground_data(
    data_dir: &Path,
    waypoint: Option<u16>,
) -> Result<GameWorld, String> {
    let _guard = test_runtime().enter();
    let (items_db, monsters_db) = if data_dir.is_dir() {
        load_sim_content_dbs(data_dir, waypoint)?
    } else {
        (
            Arc::new(beat_driven_items_db(waypoint)),
            Arc::new(MonsterDatabase {
                monsters: HashMap::new(),
            }),
        )
    };

    let mut map = Map {
        width: 256,
        height: 256,
        grid: SparseGrid::new(),
        towns: HashMap::new(),
        waypoints: HashMap::new(),
    };
    map.towns.insert(
        1,
        TownData {
            id: 1,
            name: "Thais".into(),
            temple_position: Position::new(100, 100, 7),
        },
    );

    let mechanics = if data_dir.is_dir() {
        crate::formulas::load_mechanics(data_dir, ProtocolVersion::V772)
    } else {
        crate::formulas::Mechanics::for_version(ProtocolVersion::V772)
    };

    Ok(init_beat_driven_world(
        map,
        SlotMap::default(),
        items_db,
        monsters_db,
        mechanics,
    ))
}

/// C++ `SyntheticGroundType` — `chase_kite_scenario.cc:113-121` (grass TypeID 102 = wp 150).
pub fn synthetic_ground_type_for_waypoints(default_wp: u16) -> u16 {
    match default_wp {
        110 => 103,
        120 => 107,
        130 => 110,
        140 => 106,
        160 => 104,
        _ => 102,
    }
}

/// Lay synthetic arena and return the pinned `min_wp` for pathfinding parity checks.
pub fn lay_synthetic_arena(
    map: &mut Map,
    cx: u16,
    cy: u16,
    radius: u16,
    z: u8,
    waypoint: u16,
) -> u32 {
    let ground_type = synthetic_ground_type_for_waypoints(waypoint);
    lay_arena_tiles(map, cx, cy, radius, z, ground_type);
    // Uniform synthetic grass — pinned to scenario `default_wp` (`chase_kite_scenario.cc`).
    u32::from(waypoint)
}

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

fn load_items_db_for_772(data_dir: &Path) -> Result<ItemDatabase, String> {
    let otb = data_dir.join("items/items.otb");
    let xml = data_dir.join("items/items.xml");
    if !otb.is_file() {
        return Err(format!("items.otb not found: {}", otb.display()));
    }
    if !xml.is_file() {
        return Err(format!("items.xml not found: {}", xml.display()));
    }
    let mut db = ItemDatabase::load(&otb, &xml).map_err(|e| e.to_string())?;
    if let Some(objects_srv) = tfs_rust_content::objects_srv::resolve_objects_srv_path() {
        let _ = tfs_rust_content::objects_srv::overlay_otb_speeds_from_objects_srv(
            &mut db.items,
            &objects_srv,
        );
    }
    Ok(db)
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

    let items_db = Arc::new(load_items_db_for_772(data_dir)?);
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

pub fn insert_player(world: &mut GameWorld, player: Player) -> CreatureId {
    world.creatures.insert(CreatureKind::Player(player))
}

/// Walkable ground tile for walk / pathfinding tests.
pub fn ensure_walkable_tile(map: &mut Map, pos: Position, ground_type: u16) {
    map.insert_tile(
        pos,
        Tile::Normal(TileBody {
            ground: Some(ground_type),
            down_items: Vec::new(),
            top_items: Vec::new(),
            creatures: Vec::new(),
            flags: 0,
            zone: ZoneType::Normal,
        }),
    );
}

/// Insert a default walkable ground tile at `pos` only if no tile is present.
///
/// Harness `insert_*` helpers call this before `register_creature_at` so the
/// "creatures stand on valid tiles" invariant (map audit #3) holds in test worlds
/// that did not pre-populate the spawn position (e.g. `minimal_world`). Does NOT
/// overwrite intentionally-placed tiles.
pub fn ensure_walkable_tile_if_absent(map: &mut Map, pos: Position) {
    if map.get_tile(pos).is_none() {
        ensure_walkable_tile(map, pos, 100);
    }
}

/// Lay a square arena of walkable tiles centered at `(cx, cy)` with inclusive radius.
pub fn lay_arena_tiles(map: &mut Map, cx: u16, cy: u16, radius: u16, z: u8, ground_type: u16) {
    let r = radius as i32;
    let cx = cx as i32;
    let cy = cy as i32;
    for dx in -r..=r {
        for dy in -r..=r {
            let x = (cx + dx) as u16;
            let y = (cy + dy) as u16;
            ensure_walkable_tile(map, Position::new(x, y, z), ground_type);
        }
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

pub fn insert_monster(world: &mut GameWorld, name: &str, pos: Position, speed: i32) -> CreatureId {
    insert_monster_with_config(world, name, pos, speed, MonsterAiConfig::default())
}

pub fn insert_monster_with_config(
    world: &mut GameWorld,
    name: &str,
    pos: Position,
    speed: i32,
    config: MonsterAiConfig,
) -> CreatureId {
    let base = CreatureBase {
        name: name.into(),
        position: pos,
        direction: Direction::North,
        health: 100,
        max_health: 100,
        outfit: Outfit::default(),
        speed,
        base_speed: speed,
        skull: SkullType::None,
        drunkenness: 0,
        active_conditions: Vec::new(),
        walk_queue: Default::default(),
        walk_destinations: Default::default(),
        last_step: None,
        last_step_cost: 1,
        last_step_ground_speed: 150,
        next_walk_check: None,
        next_wakeup: None,
        last_step_server_ms: None,
        earliest_walk_server_ms: 0,
        earliest_spell_server_ms: 0,
        earliest_multiuse_server_ms: 0,
        walk_timer: Default::default(),
        cancel_next_walk: false,
        force_update_follow_path: false,
        walk_update_ticks: 0,
        is_updating_path: false,
        has_follow_path: false,
        movement_blocked: false,
        stairhop_blocked_until: None,
        follow_target: None,
        attack_target: None,
        master: None,
        damage_map: Default::default(),
        think_check_bucket: None,
        earliest_attack_ms: 0,
        earliest_defend_ms: 0,
        last_defend_ms: 0,
        todo: Default::default(),
        chase_mode: Default::default(),
            last_auto_walk_armed_ms: u64::MAX,
    };
    let cid = world
        .creatures
        .insert(CreatureKind::Monster(Monster::with_config(
            base, pos, config,
        )));
    ensure_walkable_tile_if_absent(&mut world.map, pos);
    world.map.register_creature_at(pos, cid);
    cid
}

/// Spawn from parsed monster type — E0 combat snapshot + E6 loot roll at spawn.
/// C++ reference: `TMonster::TMonster` — `crnonpl.cc:2050`.
pub fn insert_monster_from_type(
    world: &mut GameWorld,
    mtype: &MonsterType,
    display_name: &str,
    pos: Position,
    speed: i32,
    config: MonsterAiConfig,
    initial_state: MonsterState,
) -> CreatureId {
    let base = CreatureBase {
        name: display_name.into(),
        position: pos,
        direction: Direction::North,
        health: mtype.health_now as i32,
        max_health: mtype.health_max as i32,
        outfit: monster_outfit_to_sim(&mtype.outfit),
        speed,
        base_speed: speed,
        skull: SkullType::None,
        drunkenness: 0,
        active_conditions: Vec::new(),
        walk_queue: Default::default(),
        walk_destinations: Default::default(),
        last_step: None,
        last_step_cost: 1,
        last_step_ground_speed: 150,
        next_walk_check: None,
        next_wakeup: None,
        last_step_server_ms: None,
        earliest_walk_server_ms: 0,
        earliest_spell_server_ms: 0,
        earliest_multiuse_server_ms: 0,
        walk_timer: Default::default(),
        cancel_next_walk: false,
        force_update_follow_path: false,
        walk_update_ticks: 0,
        is_updating_path: false,
        has_follow_path: false,
        movement_blocked: false,
        stairhop_blocked_until: None,
        follow_target: None,
        attack_target: None,
        master: None,
        damage_map: Default::default(),
        think_check_bucket: None,
        earliest_attack_ms: 0,
        earliest_defend_ms: 0,
        last_defend_ms: 0,
        todo: Default::default(),
        chase_mode: Default::default(),
            last_auto_walk_armed_ms: u64::MAX,
    };
    let cid = world
        .creatures
        .insert(CreatureKind::Monster(Monster::with_config(
            base, pos, config,
        )));
    if world.beat_driven_loop {
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(cid) {
            m.experience = mtype.experience;
            m.corpse_id = mtype.outfit.corpse_id;
            m.blood = mtype.blood_type();
            m.state = initial_state;
            m.is_idle = true;
        }
        world.roll_monster_spawn_loot(cid, mtype);
        world.recompute_monster_combat_from_equipment(cid);
    }
    ensure_walkable_tile_if_absent(&mut world.map, pos);
    world.map.register_creature_at(pos, cid);
    cid
}

/// Harness-only player strike — fires E5 `damage_stimulus` on monsters.
/// C++ reference: `TCreature::Damage` → `TMonster::DamageStimulus` — `crmain.cc:486`, `crnonpl.cc:2304`.
pub fn sim_player_damage_monster(
    world: &mut GameWorld,
    player_id: CreatureId,
    monster_id: CreatureId,
    amount: i32,
) -> bool {
    if amount <= 0 {
        return false;
    }
    let armor = match world.creatures.get(monster_id) {
        Some(CreatureKind::Monster(m)) => m.armor,
        _ => return false,
    };
    // C++ physical branch subtracts armor before `DamageStimulus` — `crmain.cc:623-631`.
    let damage = amount.saturating_sub(armor);
    if damage <= 0 {
        return false;
    }
    world.combat_execute_with_stimulus(
        Some(player_id),
        monster_id,
        &CombatDamage {
            primary: (CombatType::Physical, -damage),
            secondary: (CombatType::Physical, 0),
        },
        &CombatParams::default(),
    )
}

pub fn insert_npc(world: &mut GameWorld, name: &str, pos: Position, speed: i32) -> CreatureId {
    let base = CreatureBase {
        name: name.into(),
        position: pos,
        direction: Direction::North,
        health: 100,
        max_health: 100,
        outfit: Outfit::default(),
        speed,
        base_speed: speed,
        skull: SkullType::None,
        drunkenness: 0,
        active_conditions: Vec::new(),
        walk_queue: Default::default(),
        walk_destinations: Default::default(),
        last_step: None,
        last_step_cost: 1,
        last_step_ground_speed: 150,
        next_walk_check: None,
        next_wakeup: None,
        last_step_server_ms: None,
        earliest_walk_server_ms: 0,
        earliest_spell_server_ms: 0,
        earliest_multiuse_server_ms: 0,
        walk_timer: Default::default(),
        cancel_next_walk: false,
        force_update_follow_path: false,
        walk_update_ticks: 0,
        is_updating_path: false,
        has_follow_path: false,
        movement_blocked: false,
        stairhop_blocked_until: None,
        follow_target: None,
        attack_target: None,
        master: None,
        damage_map: Default::default(),
        think_check_bucket: None,
        earliest_attack_ms: 0,
        earliest_defend_ms: 0,
        last_defend_ms: 0,
        todo: Default::default(),
        chase_mode: Default::default(),
            last_auto_walk_armed_ms: u64::MAX,
    };
    let cid = world.creatures.insert(CreatureKind::Npc(Npc {
        base,
        npc_type_id: 0,
    }));
    ensure_walkable_tile_if_absent(&mut world.map, pos);
    world.map.register_creature_at(pos, cid);
    world.add_creature_think_check(cid);
    cid
}

/// Logged-in spectator with a connection mapping (for outgoing packet assertions).
pub fn insert_spectator_player(
    world: &mut GameWorld,
    conn_id: ConnId,
    player: Player,
) -> CreatureId {
    let pos = player.base.position;
    let cid = insert_player(world, player);
    world.register_conn_mapping(conn_id, cid);
    ensure_walkable_tile_if_absent(&mut world.map, pos);
    world.map.register_creature_at(pos, cid);
    cid
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
    if !world.try_creature_walk_step(player_id, dir, Instant::now()) {
        return Err(format!(
            "player_walk: move blocked to [{},{},{}]",
            dest.x, dest.y, dest.z
        ));
    }
    Ok(())
}

/// C++ `TCreature::SetOnMap` — relocate harness creature via `SearchLoginField(dist=1)`.
pub fn harness_place_creature_login(
    world: &mut GameWorld,
    cid: CreatureId,
    requested: Position,
) -> Option<Position> {
    world.harness_place_creature_login(cid, requested)
}

/// Appear step without inline `IdleStimulus` — C++ `SpawnMonsterAppear` defers yield to batch tail.
fn appear_monster_without_idle(world: &mut GameWorld, monster_id: CreatureId) {
    let keep_sleeping = world.creatures.get(monster_id).is_some_and(|k| {
        matches!(
            k,
            CreatureKind::Monster(m)
                if m.harness_preserve_sleep
                    && m.state == MonsterState::Sleeping
                    && m.is_idle
        )
    });
    if !keep_sleeping {
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster_id) {
            m.is_idle = false;
            if m.state == MonsterState::Sleeping {
                m.state = MonsterState::Idle;
            }
        }
    }
    world.monster_update_target_list(monster_id);
    if world.beat_driven_loop {
        if let Some(opponent) = world.creatures.get(monster_id).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            m.opponent_ids.first().copied()
        }) {
            harness_acquire_chase_target_without_idle(world, monster_id, opponent);
        }
        appear_face_target_for_debug(world, monster_id);
    }
}

/// Set follow/attack without `request_idle_stimulus` — harness batch appear only.
fn harness_acquire_chase_target_without_idle(
    world: &mut GameWorld,
    monster_id: CreatureId,
    target_id: CreatureId,
) {
    if !world.monster_is_target(monster_id, target_id) {
        return;
    }
    let in_list = world.creatures.get(monster_id).is_some_and(
        |k| matches!(k, CreatureKind::Monster(m) if m.opponent_ids.contains(&target_id)),
    );
    if !in_list {
        return;
    }
    if !world.can_see_creature(monster_id, target_id) {
        return;
    }
    if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster_id) {
        if m.is_hostile || m.base.is_summon() {
            m.base.attack_target = Some(target_id);
        }
        m.base.follow_target = Some(target_id);
        m.base.is_updating_path = true;
        m.base.has_follow_path = false;
        m.base.force_update_follow_path = false;
        if !m.base.walk_queue.is_empty() {
            m.base.walk_queue.clear();
        }
    }
}

/// Chase JSONL rotate @ tick 0 — harness-only; bypasses `walk_timer_idle` gate on appear.
fn appear_face_target_for_debug(world: &mut GameWorld, cid: CreatureId) {
    if !world.beat_driven_loop || !crate::chase_debug::chase_path_debug_enabled() {
        return;
    }
    let (pos, target_id, current) = match world.creatures.get(cid) {
        Some(CreatureKind::Monster(m)) => (m.base.position, m.base.attack_target, m.base.direction),
        _ => return,
    };
    let Some(target_id) = target_id else {
        return;
    };
    let target_pos = match world.creatures.get(target_id) {
        Some(k) => k.position(),
        None => return,
    };
    let new_dir = compute_look_toward_target(pos, target_pos, current);
    if new_dir != current {
        creature_turn_with_broadcast(world, cid, new_dir);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get(cid) {
            crate::chase_debug::log_rotate(
                world.chase_trace_tick(),
                cid,
                m.base.name.as_str(),
                new_dir as u8,
                Some(target_id.data().as_ffi()),
            );
        }
    }
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

/// Wake monsters, acquire targets, then batch `ToDoYield` — `chase_kite_scenario.cc` `SpawnMonsterAppear`.
pub fn kite_monsters_appear_batch(world: &mut GameWorld, monster_ids: &[CreatureId]) {
    // C++ `EnsureMonstersSpawned` → `ResyncHarnessRng()` after spawn loot (`chase_kite_scenario.cc:537`).
    world.resync_sim_glibc_rng();
    for &monster_id in monster_ids {
        appear_monster_without_idle(world, monster_id);
        world.add_creature_think_check(monster_id);
    }
    for &monster_id in monster_ids {
        world.creature_todo_yield(monster_id);
    }
}

/// Wake monster and run appear/target acquisition — `monster_appear` scenario step.
pub fn kite_monster_appear(world: &mut GameWorld, monster_id: CreatureId) {
    if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster_id) {
        if !m.harness_preserve_sleep {
            m.is_idle = false;
        }
    }
    world.monster_on_creature_appear_self(monster_id);
    world.add_creature_think_check(monster_id);
}

/// Truncate chase JSONL at scenario start — C++ `ChasePathResetLog`.
pub fn reset_chase_path_log() {
    crate::chase_debug::chase_path_reset_log();
}

/// Harness `player_walk` JSONL — C++ `ChasePathLogHarnessPlayerStep`.
pub fn log_harness_player_step(tick: u64, step: u32, pos: tfs_rust_common::Position) {
    crate::chase_debug::log_harness_player_step(tick, step, pos);
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
    if harness_place_creature_login(world, cyclops_id, cyclops_pos).is_none() {
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
    if harness_place_creature_login(world, cyclops_id, cyclops_pos).is_none() {
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
        if harness_place_creature_login(world, mid, spawn_pos).is_none() {
            return Err(format!("harness spawn: cannot place cyclops at {spawn_pos:?}"));
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
    let mut clock = 0u64;
    for &(wall, x, y) in kite_steps {
        if wall > wall_ms {
            break;
        }
        clock = wall;
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
    writeln!(out, "  \"tick\": {},", world.server_ms)?;
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
    world.advance_beat_772(delay_ms);
}

/// C++ `MoveCreatures` — `crmain.cc:1106` (harness clock + due todo drain).
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
        harness_clamp_delay(world.server_ms, delay_ms)
    } else {
        delay_ms
    };
    if respect_wall && requested > 0 && delay_ms == 0 {
        return;
    }
    world.server_ms = world.server_ms.saturating_add(delay_ms);
    world.tick_counter = world.tick_counter.saturating_add(delay_ms / 50);
    world.drain_todo_queue();
    if world.walk_wake_tx.is_none() {
        world.process_walk_deadlines();
    }
}

/// Max ms this drain round may advance — `None` means uncapped (production paths).
pub fn set_sim_harness_wall_ms(wall_ms: Option<u64>) {
    with_harness_clock_mut(|c| c.wall_ms = wall_ms);
}

/// Last scenario `advance_ms` — retained for future `chase_kite_sim` `AdvanceMs` wiring.
pub fn set_sim_harness_segment_ms(segment_ms: Option<u64>) {
    with_harness_clock_mut(|c| c.segment_ms = segment_ms);
}

fn harness_at_wall(server_ms: u64) -> bool {
    with_harness_clock(|c| c.wall_ms.is_some_and(|wall| server_ms >= wall))
}

fn harness_clamp_delay(server_ms: u64, delay_ms: u64) -> u64 {
    with_harness_clock(|c| {
        let Some(wall) = c.wall_ms else {
            return delay_ms;
        };
        wall.saturating_sub(server_ms).min(delay_ms)
    })
}

/// C++ `MoveCreatures` — single due-todo pass after advancing `ServerMilliseconds`.
pub fn drain_todo_queue_once(world: &mut GameWorld) {
    world.drain_todo_queue();
}

/// C++ `DrainTodoQueue` — `chase_kite_scenario.cc` (bounded `MoveCreatures` rounds).
pub fn run_sim_tick(world: &mut GameWorld) {
    const MAX_ROUNDS: usize = 64;
    for _ in 0..MAX_ROUNDS {
        let Some(entry) = world.todo_queue.peek() else {
            break;
        };
        if entry.execution_time > world.server_ms {
            if harness_at_wall(world.server_ms) {
                break;
            }
            let delta = entry.execution_time - world.server_ms;
            let delta = harness_clamp_delay(world.server_ms, delta);
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
#[path = "sim_harness_tests.rs"]
mod harness_tests;
