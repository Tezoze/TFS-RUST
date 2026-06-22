//! Headless simulation harness — shared world builders for unit tests and `chase_kite_sim`.
//!
//! C++ reference: `chase_kite_scenario.cc` `SpawnMonsterAppear`, `MoveCreatures`, `DrainTodoQueue`;
//! `tibia-game-master` test patterns; `GameWorld` tick — `game.cpp`, `crmain.cc`.

/// First productive `IdleStimulus` after harness appear — C++ defers until first `advance_ms 2000` drain.
pub const HARNESS_APPEAR_IDLE_DEFER_MS: u64 = 2000;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use slotmap::SlotMap;
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

use crate::config::ConfigManager;
use crate::combat::{CombatDamage, CombatParams};
use crate::creature::{
    CreatureBase, CreatureKind, Monster, MonsterAiConfig, MonsterState, Npc, Outfit, Player,
    PlayerEconomy, PlayerInventory, PlayerPersistBaseline, PlayerSkills, PlayerSocial,
};
use tfs_rust_common::enums::CombatType;
use tfs_rust_content::monsters::{MonsterOutfit, MonsterType};
use crate::event_dispatcher::NullEventDispatcher;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::map::{Map, SparseGrid};
use crate::pathfinding::{scan_min_terrain_waypoints, REVERSE_PATH_VIEW_RADIUS};
use crate::spawn::SpawnManager;
use tfs_rust_content::monsters::MonsterDatabase;
use crate::tile::{Tile, TileBody};
use tfs_rust_common::ConnId;
use tfs_rust_common::enums::ZoneType;

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
            last_step: None,
            last_step_cost: 1,
            last_step_ground_speed: 150,
            next_walk_check: None,
            next_wakeup: None,
            last_step_server_ms: None,
            earliest_walk_server_ms: 0,
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
    let map_rel = std::env::var("TFS_MAP_OTBM")
        .unwrap_or_else(|_| "world/forgotten.otbm".to_string());
    let synthetic_arena = std::env::var("TFS_KITE_SYNTHETIC_ARENA")
        .is_ok_and(|v| !v.is_empty() && v != "0");
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
        last_step: None,
        last_step_cost: 1,
        last_step_ground_speed: 150,
        next_walk_check: None,
        next_wakeup: None,
        last_step_server_ms: None,
        earliest_walk_server_ms: 0,
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
    };
    let cid = world
        .creatures
        .insert(CreatureKind::Monster(Monster::with_config(base, pos, config)));
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
        last_step: None,
        last_step_cost: 1,
        last_step_ground_speed: 150,
        next_walk_check: None,
        next_wakeup: None,
        last_step_server_ms: None,
        earliest_walk_server_ms: 0,
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
    };
    let cid = world
        .creatures
        .insert(CreatureKind::Monster(Monster::with_config(base, pos, config)));
    if world.beat_driven_loop {
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(cid) {
            m.experience = mtype.experience;
            m.corpse_id = mtype.outfit.corpse_id;
            m.state = initial_state;
            m.is_idle = true;
        }
        world.roll_monster_spawn_loot(cid, mtype);
        world.recompute_monster_combat_from_equipment(cid);
    }
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
        last_step: None,
        last_step_cost: 1,
        last_step_ground_speed: 150,
        next_walk_check: None,
        next_wakeup: None,
        last_step_server_ms: None,
        earliest_walk_server_ms: 0,
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
    };
    let cid = world.creatures.insert(CreatureKind::Npc(Npc {
        base,
        npc_type_id: 0,
    }));
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
    world.conn_to_creature.insert(conn_id, cid);
    world.map.register_creature_at(pos, cid);
    cid
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
    world.map.register_creature_at(new_pos, player_id);
    world.monster_dispatch_creature_move(player_id, old_pos, new_pos);
    Ok(())
}

/// Wake monsters, acquire targets, then batch `ToDoYield` — `chase_kite_scenario.cc` `SpawnMonsterAppear`.
pub fn kite_monsters_appear_batch(world: &mut GameWorld, monster_ids: &[CreatureId]) {
    world.batch_appear_defer_idle = true;
    for &monster_id in monster_ids {
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster_id) {
            if !m.harness_preserve_sleep {
                m.is_idle = false;
            }
        }
        world.monster_on_creature_appear_self(monster_id);
        world.add_creature_think_check(monster_id);
    }
    world.batch_appear_defer_idle = false;
    for &monster_id in monster_ids {
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster_id) {
            m.harness_defer_appear_idle = true;
        }
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
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mid) {
            m.harness_spawn_order = (i as u16).saturating_add(1);
        }
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
    set_sim_harness_wall_ms(world, Some(0));
    for &dest in &kite_path {
        teleport_player(world, player_id, dest)?;
        run_sim_tick(world);
    }

    set_sim_harness_wall_ms(world, Some(HARNESS_APPEAR_IDLE_DEFER_MS));
    move_creatures_explicit(world, HARNESS_APPEAR_IDLE_DEFER_MS);
    run_sim_tick(world);
    // Caller may run further drains — first chase idle @2000 runs during `run_sim_tick` above.

    let nw_id = monster_ids[3];
    let player_pos = Position::new(32360, 32294, z);
    Ok((nw_id, player_id, player_pos))
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
    if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster_id) {
        m.harness_spawn_order = 1;
    }
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
    set_sim_harness_wall_ms(world, Some(0));
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
        set_sim_harness_wall_ms(world, Some(wall));
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
    let origin = world.creatures.get(cid).map(|k| k.position()).unwrap_or(target);
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
            pos.x,
            pos.y,
            pos.z,
            wp,
            walkable
        )?;
    }
    writeln!(out, "  ]")?;
    writeln!(out, "}}")?;
    Ok(())
}

/// C++ `MoveCreatures` — `crmain.cc:1106` (harness clock + due todo drain).
///
/// When [`GameWorld::sim_harness_wall_ms`] is set, `delay_ms` is clamped so `server_ms` never
/// exceeds the scenario wall. Use [`move_creatures_explicit`] for scenario `advance_ms` steps.
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
        harness_clamp_delay(world, delay_ms)
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
pub fn set_sim_harness_wall_ms(world: &mut GameWorld, wall_ms: Option<u64>) {
    world.sim_harness_wall_ms = wall_ms;
}

/// Last scenario `advance_ms` — lower bound for `TDGo` delay at a harness wall tick.
pub fn set_sim_harness_segment_ms(world: &mut GameWorld, segment_ms: Option<u64>) {
    world.sim_harness_segment_ms = segment_ms;
}

fn harness_at_wall(world: &GameWorld) -> bool {
    world
        .sim_harness_wall_ms
        .is_some_and(|wall| world.server_ms >= wall)
}

fn harness_clamp_delay(world: &GameWorld, delay_ms: u64) -> u64 {
    let Some(wall) = world.sim_harness_wall_ms else {
        return delay_ms;
    };
    wall.saturating_sub(world.server_ms).min(delay_ms)
}

/// C++ `DrainTodoQueue` — `chase_kite_scenario.cc` (bounded `MoveCreatures` rounds).
pub fn run_sim_tick(world: &mut GameWorld) {
    const MAX_ROUNDS: usize = 64;
    for _ in 0..MAX_ROUNDS {
        let Some(entry) = world.todo_queue.peek() else {
            break;
        };
        if entry.execution_time > world.server_ms {
            if harness_at_wall(world) {
                break;
            }
            let delta = entry.execution_time - world.server_ms;
            let delta = harness_clamp_delay(world, delta);
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
mod harness_tests {
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
        assert_eq!(world.tile_ground_speed(world.map.get_tile(pos).unwrap().body()), 150);
    }

    #[test]
    fn move_creatures_clamps_to_harness_wall() {
        let mut world = beat_driven_world();
        world.sim_harness_wall_ms = Some(2_000);
        world.server_ms = 500;
        move_creatures(&mut world, 5_000);
        assert_eq!(world.server_ms, 2_000);
    }

    #[test]
    fn move_creatures_explicit_ignores_wall() {
        let mut world = beat_driven_world();
        world.sim_harness_wall_ms = Some(2_000);
        world.server_ms = 0;
        move_creatures_explicit(&mut world, 2_000);
        assert_eq!(world.server_ms, 2_000);
    }

    #[test]
    fn run_sim_tick_stops_at_harness_wall() {
        let mut world = beat_driven_world();
        let pos = Position::new(100, 100, 7);
        let cid = insert_monster(&mut world, "Rat", pos, 200);
        world.sim_harness_wall_ms = Some(6_000);
        world.schedule_creature_wakeup(cid, 20_000, crate::todo_queue::WakeupTiePolicy::Fifo);
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
        world.batch_appear_defer_idle = true;
        world.monster_on_creature_appear_self(monster);
        assert!(
            world.creature_todo_queue_empty(monster),
            "deferred appear must not ToDoYield during target acquire"
        );
        world.batch_appear_defer_idle = false;
        world.creature_todo_yield(monster);
        assert!(
            !world.creature_todo_queue_empty(monster),
            "batch yield must enqueue Wait(0)"
        );
    }

    /// Quad cyclops — move-stimulus idle must not fire during appear-defer window.
    #[test]
    fn batch_appear_quad_blocks_move_stimulus_idle_until_deferred_wakeup() {
        use crate::creature::MonsterState;
        use crate::sim_harness::HARNESS_APPEAR_IDLE_DEFER_MS;
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
        set_sim_harness_wall_ms(&mut world, Some(0));
        run_sim_tick(&mut world);
        for &mid in &monster_ids {
            assert!(
                world
                    .creatures
                    .get(mid)
                    .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if m.harness_defer_appear_idle)),
                "defer flag must stay set after appear-step drain"
            );
            assert_eq!(
                world.creatures.get(mid).and_then(|k| k.base().next_wakeup),
                Some(HARNESS_APPEAR_IDLE_DEFER_MS),
                "deferred wakeup must be scheduled at 2000ms"
            );
        }
        let kited = Position::new(32362, 32290, 7);
        teleport_player(&mut world, player, kited).expect("kite teleport");
        set_sim_harness_wall_ms(&mut world, Some(0));
        run_sim_tick(&mut world);
        for &mid in &monster_ids {
            assert!(
                world
                    .creatures
                    .get(mid)
                    .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if m.harness_defer_appear_idle)),
                "kite move must not clear defer before first idle"
            );
            assert!(
                world.creature_todo_queue_empty(mid),
                "kite move must not enqueue chase todos during defer window"
            );
        }
        set_sim_harness_wall_ms(&mut world, Some(HARNESS_APPEAR_IDLE_DEFER_MS));
        move_creatures_explicit(&mut world, HARNESS_APPEAR_IDLE_DEFER_MS);
        run_sim_tick(&mut world);
        assert!(
            monster_ids.iter().any(|&mid| {
                world
                    .creatures
                    .get(mid)
                    .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if !m.harness_defer_appear_idle))
            }),
            "at least one monster must clear defer after 2000ms idle"
        );
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
            if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mid) {
                m.harness_spawn_order = (i as u16).saturating_add(1);
            }
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

    /// P2.5e — NW cyclops first diagonal `go_exec` fires @tick=4000 (ToDoStart @2001, batch drain @4000).
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
            nw_base.todo.has_go() || !nw_base.walk_queue.is_empty(),
            "idle@2000 must enqueue chase (wakeup={:?})",
            nw_base.next_wakeup
        );

        set_sim_harness_wall_ms(&mut world, Some(4_000));
        move_creatures_explicit(&mut world, 2_000);
        run_sim_tick(&mut world);

        let nw_pos = world.creatures.get(nw_id).map(|k| k.position());
        assert_eq!(
            nw_pos,
            Some(Position::new(32358, 32290, 7)),
            "NW cyclops must execute first diagonal go_exec @4000 (todo armed @2001)"
        );
        assert_eq!(world.server_ms, 4_000);
    }

    /// P2.5g — all four cyclops `go_exec` positions @4000 match C++ oracle drain order.
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

        set_sim_harness_wall_ms(&mut world, Some(4_000));
        move_creatures_explicit(&mut world, 2_000);
        run_sim_tick(&mut world);

        let expected_after_go: [(u16, u16, u8); 4] = [
            (32359, 32287, 7), // far-N (spawn 1)
            (32361, 32291, 7), // east (spawn 2)
            (32360, 32292, 7), // south (spawn 3)
            (32358, 32290, 7), // NW (spawn 4)
        ];
        let mut by_spawn_order: Vec<(u16, Position)> = world
            .creatures
            .iter()
            .filter_map(|(id, k)| {
                let CreatureKind::Monster(m) = k else {
                    return None;
                };
                if m.harness_spawn_order == 0 {
                    return None;
                }
                Some((m.harness_spawn_order, world.creatures.get(id)?.position()))
            })
            .collect();
        by_spawn_order.sort_by_key(|(order, _)| *order);
        let positions: Vec<Position> = by_spawn_order.into_iter().map(|(_, p)| p).collect();
        assert_eq!(
            positions,
            expected_after_go
                .map(|(x, y, z)| Position::new(x, y, z))
                .to_vec(),
            "quad cyclops positions after go_exec @4000"
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
        let Ok((nw_id, _player_id, player_pos)) =
            setup_cyclops_quad_chase_to_tick_2000(&mut world)
        else {
            return;
        };
        assert_eq!(world.server_ms, HARNESS_APPEAR_IDLE_DEFER_MS);
        // Deferred appear arms `next_wakeup@2000` — clear so idle can run (FillMap moment).
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(nw_id) {
            m.base.next_wakeup = None;
        }
        world.monster_idle_stimulus(nw_id);
        let (state, tiles) = world.dump_tshortway_fill_walkable_viewport(
            nw_id,
            player_pos,
            REVERSE_PATH_VIEW_RADIUS,
        );
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
            let Some(TShortwayFillTile { walkable, wp, .. }) =
                tiles.iter().find(|t| t.pos == pos)
            else {
                panic!("priority tile {pos:?} missing from viewport dump");
            };
            eprintln!("fill_walkable {pos:?} walkable={walkable} wp={wp}");
        }

        if std::env::var("TFS_FILLMAP_DUMP")
            .is_ok_and(|v| !v.is_empty() && v != "0")
        {
            let out = std::env::var("TFS_FILLMAP_DUMP_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../log/fill_walkable_rust_nw.json")
                });
            write_fill_walkable_dump_json(&world, nw_id, player_pos, &out)
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

        set_sim_harness_wall_ms(&mut world, Some(6_000));
        teleport_player(
            &mut world,
            player_id,
            Position::new(32363, 32292, 7),
        )
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
}
