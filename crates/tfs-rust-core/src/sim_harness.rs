//! Headless simulation harness — shared world builders for unit tests and `chase_kite_sim`.
//!
//! C++ reference: `tibia-game-master` test patterns; `GameWorld` tick — `game.cpp`, `crmain.cc`.

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

/// 772 human hero for chase parity sim — matches C++ `TKiteSimPlayer` + human race HP.
/// C++ reference: `chase_kite_scenario.cc` `TKiteSimPlayer`; human `.mon` race data.
pub fn sim_hero_player(name: &str, pos: Position) -> Player {
    let mut p = test_player_base(name, pos);
    p.base.health = 150;
    p.base.max_health = 150;
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

/// Lay synthetic arena and return the pinned `min_wp` for pathfinding parity checks.
pub fn lay_synthetic_arena(
    map: &mut Map,
    cx: u16,
    cy: u16,
    radius: u16,
    z: u8,
    waypoint: u16,
) -> u32 {
    lay_arena_tiles(map, cx, cy, radius, z, waypoint);
    let origin = Position::new(cx, cy, z);
    scan_min_terrain_waypoints(map, origin, REVERSE_PATH_VIEW_RADIUS, |p| {
        map.get_tile(p)
            .filter(|_| map.is_walkable(p))
            .and_then(|t| t.body().ground)
            .map(|gid| u32::from(gid))
            .unwrap_or(0)
    })
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
            if m.state != MonsterState::Sleeping {
                m.is_idle = false;
            }
        }
        world.monster_on_creature_appear_self(monster_id);
        world.add_creature_think_check(monster_id);
    }
    world.batch_appear_defer_idle = false;
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
