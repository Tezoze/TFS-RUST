//! Unit-test `GameWorld` fixtures (never touches the database).
//!
//! C++ reference: `tibia-game-master` test patterns; `GameWorld` tick — `game.cpp`, `crmain.cc`.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use slotmap::SlotMap;
use tfs_rust_common::Position;
use tfs_rust_common::ProtocolVersion;
use tfs_rust_common::enums::CombatType;
use tfs_rust_common::enums::ZoneType;
use tfs_rust_common::enums::{Direction, SkullType};
use tfs_rust_content::groups::GroupDatabase;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::monsters::MonsterDatabase;
use tfs_rust_content::npcs::NpcDatabase;
use tfs_rust_content::otb::ItemType;
use tfs_rust_content::otbm::TownData;
use tfs_rust_content::vocations::VocationRegistry;
use tfs_rust_db::DbPool;
use tfs_rust_db::player::PlayerRecord;

use crate::config::ConfigManager;
use crate::creature::{
    CreatureBase, CreatureKind, Monster, MonsterAiConfig, Outfit, Player, PlayerEconomy,
    PlayerInventory, PlayerPersistBaseline, PlayerSkills, PlayerSocial,
};
use crate::event_dispatcher::NullEventDispatcher;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::inventory::SLOTP_BACKPACK;
use crate::map::{Map, SparseGrid};
use crate::spawn::SpawnManager;
use crate::tile::{Tile, TileBody};

#[cfg(test)]
use crate::creature::Npc;
#[cfg(test)]
use tfs_rust_common::ConnId;

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

#[cfg(test)]
pub fn test_player(name: &str, pos: Position) -> Player {
    test_player_base(name, pos)
}

/// 772 human hero for chase parity sim — matches C++ `TKiteSimPlayer` + `human.mon` race data.
/// C++ reference: `chase_kite_scenario.cc` `TKiteSimPlayer`; `runtime/mon/human.mon` `Defend=5`.
pub fn sim_hero_player(name: &str, pos: Position) -> Player {
    let mut p = test_player_base(name, pos);
    p.base.health = 150;
    p.base.max_health = 150;
    p.fist_defense = 5;
    p.fist_attack = 7;
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
            var_speed: 0,
            skull: SkullType::None,
            drunkenness: 0,
            active_conditions: Vec::new(),
            walk_queue: Default::default(),
            walk_destinations: Default::default(),
            last_step: None,
            last_step_cost: 1,
            last_step_ground_speed: 150,
            next_wakeup: None,
            last_step_server_ms: None,
            earliest_walk_server_ms: 0,
            earliest_spell_server_ms: 0,
            earliest_multiuse_server_ms: 0,
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
            master_is_player: false,
            summoned_creatures: 0,
            damage_map: Default::default(),
            last_hit_by: None,
            last_damage_type: CombatType::Physical,
            poison_damage_origin: None,
            fire_damage_origin: None,
            energy_damage_origin: None,
            earliest_attack_ms: 0,
            latest_attack_round: 0,
            earliest_defend_ms: 0,
            last_defend_ms: 0,
            learning_points: 0,
            todo: Default::default(),
            chase_mode: Default::default(),
            last_auto_walk_armed_ms: u64::MAX,
            drop_loot: true,
            skill_loss: true,
            is_dead: false,
            logging_out: false,
            logout_allowed: false,
        },
        account_id: 1,
        guid: 1,
        account_type: 1,
        group_id: 1,
        set_max_speed: false,
        sex: crate::creature::PlayerSex::Male,
        vocation_id: 0,
        vocation_profile: crate::creature::vocation::VocationProfile::none_vocation(),
        level: 8,
        experience: 0,
        mana: 50,
        max_mana: 50,
        capacity: 40000,
        inventory: PlayerInventory::default(),
        skills: PlayerSkills::with_levels(10, 10, 10, 10, 10, 10, 10, 0),
        economy: PlayerEconomy {
            balance: 0,
            soul: 100,
        },
        social: PlayerSocial::default(),
        town_id: 1,
        premium_ends_at: 0,
        stamina_minutes: 2520,
        stamina_rest_ms: 0,
        stamina_hunt_ms: 0,
        offline_training_ms: 0,
        spell_cooldown_end: HashMap::new(),
        spell_group_cooldown_end: HashMap::new(),
        operating_system: 0,
        otclient_v8: 0,
        ghost_mode: false,
        lastip: 0,
        equipment_slots: std::array::from_fn(|_| None),
        inventory_weight: 0,
        items_light: Default::default(),
        internal_light: Default::default(),
        inventory_abilities: [false; 11],
        dact_skills: [0; 7],
        mdact_skills: [0; 7],
        last_combat_weapons: Default::default(),
        var_stats: [0; 4],
        condition_suppressions: 0,
        shop_owner: None,
        shop_items: Vec::new(),
        vip_list: Vec::new(),
        outfits: Vec::new(),
        health_hidden: false,
        last_activity: Instant::now(),
        last_command_round: 0,
        last_action_round: 0,
        food_remaining: 0,
        item_regen_interval: 0,
        soul_cycle: 0,
        soul_count: 0,
        soul_max_count: 0,
        earliest_logout_round: 0,
        attacked_players: Vec::new(),
        former_attacked_players: Vec::new(),
        aggressor: false,
        former_aggressor: false,
        former_logout_round: 0,
        playerkiller_end: 0,
        murder_timestamps: [0; 20],
        last_ping_sent: Instant::now(),
        last_pong_at: Instant::now(),
        next_action_until: None,
        walk_action: None,
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
        fist_defense: 0,
        fist_attack: 0,
        attack_mode: Default::default(),
        secure_mode: false,
        earliest_protection_zone_round: 0,
        client_icons: 0,
        talk_guard: Default::default(),
        message_buffer_count: 0,
        message_buffer_ticks: 0,
        blessings: 0,
        exact_lethal_blow: false,
        amulet_of_loss_saved: false,
        registered_creature_events: HashSet::new(),
    }
}

/// Helper for PC-4 fight-mode tests — a minimal `Player` with default vitals.
#[cfg(test)]
#[allow(dead_code)] // fixture API; fight-mode tests often build Player via `test_player`
pub fn minimal_player() -> Player {
    test_player("test", Position::new(0, 0, 7))
}

/// Helper for PC-4 fight-mode tests — a minimal `CreatureBase` with default vitals.
#[cfg(test)]
pub fn minimal_creature_base() -> CreatureBase {
    CreatureBase {
        name: "test".into(),
        position: Position::new(0, 0, 7),
        direction: Direction::North,
        health: 100,
        max_health: 100,
        outfit: Outfit::default(),
        speed: 220,
        base_speed: 220,
        var_speed: 0,
        skull: SkullType::None,
        drunkenness: 0,
        active_conditions: Vec::new(),
        walk_queue: Default::default(),
        walk_destinations: Default::default(),
        last_step: None,
        last_step_cost: 1,
        last_step_ground_speed: 150,
        next_wakeup: None,
        last_step_server_ms: None,
        earliest_walk_server_ms: 0,
        earliest_spell_server_ms: 0,
        earliest_multiuse_server_ms: 0,
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
        master_is_player: false,
        summoned_creatures: 0,
        damage_map: Default::default(),
        last_hit_by: None,
        last_damage_type: CombatType::Physical,
        poison_damage_origin: None,
        fire_damage_origin: None,
        energy_damage_origin: None,
        earliest_attack_ms: 0,
        latest_attack_round: 0,
        earliest_defend_ms: 0,
        last_defend_ms: 0,
        learning_points: 0,
        todo: Default::default(),
        chase_mode: Default::default(),
        last_auto_walk_armed_ms: u64::MAX,
        drop_loot: true,
        skill_loss: true,
        is_dead: false,
        logging_out: false,
        logout_allowed: false,
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
        murder_timestamps: String::new(),
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
        soul_cycle: 0,
        soul_count: 0,
        soul_max_count: 0,
    }
}

pub fn bag_item_type(server_id: u16) -> ItemType {
    let mut it = ItemType {
        id: server_id,
        group: ItemType::GROUP_CONTAINER,
        allow_pickupable: true,
        server_id,
        client_id: server_id,
        slot_position: SLOTP_BACKPACK,
        ..Default::default()
    };
    it.xml_attributes
        .insert("containersize".into(), "20".into());
    it
}

pub fn pickup_item_type(server_id: u16) -> ItemType {
    ItemType {
        id: server_id,
        allow_pickupable: true,
        moveable_override: Some(true),
        server_id,
        client_id: server_id,
        ..Default::default()
    }
}

/// Walkable synthetic ground for chase parity — OTB `ITEM_ATTR_SPEED` / 772 `WAYPOINTS`.
///
/// C++ mirror: `objects.srv` TypeID 102 (`grass`, `Waypoints=150`).
pub fn synthetic_ground_item_type(server_id: u16, waypoint: u16) -> ItemType {
    ItemType {
        id: server_id,
        group: ItemType::GROUP_GROUND,
        allow_pickupable: false,
        server_id,
        client_id: server_id,
        speed: waypoint,
        ..Default::default()
    }
}

fn register_synthetic_ground(items: &mut HashMap<u16, ItemType>, waypoint: u16) {
    items.insert(waypoint, synthetic_ground_item_type(waypoint, waypoint));
}

pub(crate) fn test_runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Runtime::new().expect("tokio runtime for tests"))
}

#[cfg(test)]
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
        house_tiles: Vec::new(),
        refresh_positions: rustc_hash::FxHashSet::default(),
        refresh_snapshots: HashMap::new(),
        live_dirty: rustc_hash::FxHashSet::default(),
        live_baselines: HashMap::new(),
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
        Arc::new(NpcDatabase::new()),
        Arc::new(GroupDatabase {
            groups: HashMap::new(),
        }),
        Arc::new(VocationRegistry {
            vocations: HashMap::new(),
        }),
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
#[cfg(test)]
pub fn beat_driven_world() -> GameWorld {
    beat_driven_world_with_synthetic_ground(None)
}

/// Synthetic chase arena — uniform walkable tiles with pinned waypoint cost.
#[cfg(test)]
pub fn beat_driven_world_with_synthetic_ground(waypoint: Option<u16>) -> GameWorld {
    beat_driven_world_with_synthetic_ground_data(Path::new("/nonexistent"), waypoint)
        .unwrap_or_else(|_| panic!("synthetic world without data dir failed"))
}

/// Pinned waypoint for unit-test arenas — matches kite sim synthetic grass (`chase_kite_scenario.cc`).
#[cfg(test)]
pub const TEST_SYNTHETIC_GROUND_WP: u16 = 150;

/// 772 beat-driven world with synthetic terrain registered for `TShortway::FillMap`.
#[cfg(test)]
pub fn beat_driven_test_world() -> GameWorld {
    let mut world = beat_driven_world_with_synthetic_ground(Some(TEST_SYNTHETIC_GROUND_WP));
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
    let mut items_db = load_items_db_for(data_dir)?;
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

pub(crate) fn init_beat_driven_world(
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
        Arc::new(NpcDatabase::new()),
        Arc::new(GroupDatabase {
            groups: HashMap::new(),
        }),
        Arc::new(VocationRegistry {
            vocations: HashMap::new(),
        }),
        tfs_rust_net::Codec::from_version(tfs_rust_common::ProtocolVersion::V772)
            .expect("772 codec"),
        mechanics,
    );
    world.server_ms = 0;
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
        house_tiles: Vec::new(),
        refresh_positions: rustc_hash::FxHashSet::default(),
        refresh_snapshots: HashMap::new(),
        live_dirty: rustc_hash::FxHashSet::default(),
        live_baselines: HashMap::new(),
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

pub(crate) fn load_items_db_for(data_dir: &Path) -> Result<ItemDatabase, String> {
    let ron = ItemDatabase::ron_path(data_dir, tfs_rust_common::ProtocolVersion::V772);
    if !ron.is_file() {
        return Err(format!("items.ron not found: {}", ron.display()));
    }
    ItemDatabase::load_ron(&ron).map_err(|e| e.to_string())
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

            ground_item: None,
            stacks: None,
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

#[cfg(test)]
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
        var_speed: 0,
        skull: SkullType::None,
        drunkenness: 0,
        active_conditions: Vec::new(),
        walk_queue: Default::default(),
        walk_destinations: Default::default(),
        last_step: None,
        last_step_cost: 1,
        last_step_ground_speed: 150,
        next_wakeup: None,
        last_step_server_ms: None,
        earliest_walk_server_ms: 0,
        earliest_spell_server_ms: 0,
        earliest_multiuse_server_ms: 0,
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
        master_is_player: false,
        summoned_creatures: 0,
        damage_map: Default::default(),
        last_hit_by: None,
        last_damage_type: CombatType::Physical,
        poison_damage_origin: None,
        fire_damage_origin: None,
        energy_damage_origin: None,
        earliest_attack_ms: 0,
        latest_attack_round: 0,
        earliest_defend_ms: 0,
        last_defend_ms: 0,
        learning_points: 0,
        todo: Default::default(),
        chase_mode: Default::default(),
        last_auto_walk_armed_ms: u64::MAX,
        drop_loot: true,
        skill_loss: true,
        is_dead: false,
        logging_out: false,
        logout_allowed: false,
    };
    let cid = world
        .creatures
        .insert(CreatureKind::Monster(Monster::with_config(
            base, pos, config,
        )));
    crate::login_out::assign_creature_wire_id(world, cid);
    ensure_walkable_tile_if_absent(&mut world.map, pos);
    world.map.register_creature_at(pos, cid);
    cid
}

#[cfg(test)]
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
        var_speed: 0,
        skull: SkullType::None,
        drunkenness: 0,
        active_conditions: Vec::new(),
        walk_queue: Default::default(),
        walk_destinations: Default::default(),
        last_step: None,
        last_step_cost: 1,
        last_step_ground_speed: 150,
        next_wakeup: None,
        last_step_server_ms: None,
        earliest_walk_server_ms: 0,
        earliest_spell_server_ms: 0,
        earliest_multiuse_server_ms: 0,
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
        master_is_player: false,
        summoned_creatures: 0,
        damage_map: Default::default(),
        last_hit_by: None,
        last_damage_type: CombatType::Physical,
        poison_damage_origin: None,
        fire_damage_origin: None,
        energy_damage_origin: None,
        earliest_attack_ms: 0,
        latest_attack_round: 0,
        earliest_defend_ms: 0,
        last_defend_ms: 0,
        learning_points: 0,
        todo: Default::default(),
        chase_mode: Default::default(),
        last_auto_walk_armed_ms: u64::MAX,
        drop_loot: true,
        skill_loss: true,
        is_dead: false,
        logging_out: false,
        logout_allowed: false,
    };
    let cid = world
        .creatures
        .insert(CreatureKind::Npc(Npc::placeholder(base)));
    crate::login_out::assign_creature_wire_id(world, cid);
    ensure_walkable_tile_if_absent(&mut world.map, pos);
    world.map.register_creature_at(pos, cid);
    cid
}

/// Logged-in spectator with a connection mapping (for outgoing packet assertions).
#[cfg(test)]
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

/// Drain due and upcoming todos without a scenario wall — NPC dialogue timing tests.
pub fn drain_todos_until_idle(world: &mut GameWorld) {
    const MAX_ROUNDS: usize = 64;
    for _ in 0..MAX_ROUNDS {
        let Some(exec) = world.next_todo_execution_ms() else {
            break;
        };
        if exec > world.server_ms() {
            world.move_creatures(exec - world.server_ms());
        } else {
            world.move_creatures(0);
        }
    }
}

#[cfg(test)]
mod tests {
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
}
