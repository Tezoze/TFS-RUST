//! Headless `GameWorld` builders and spawn/damage helpers for chase scenarios.
//!
//! C++ reference: `chase_kite_scenario.cc` spawn/appear; `crnonpl.cc` `TMonster::TMonster`;
//! `crmain.cc` `TCreature::Damage`. Core never reads `TFS_SIM_SEED`.

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
use tfs_rust_content::monsters::{MonsterOutfit, MonsterType};
use tfs_rust_content::npcs::NpcDatabase;
use tfs_rust_content::otb::ItemType;
use tfs_rust_content::otbm::TownData;
use tfs_rust_content::vocations::VocationRegistry;
use tfs_rust_core::combat::{CombatDamage, CombatParams};
use tfs_rust_core::config::ConfigManager;
use tfs_rust_core::creature::{
    CreatureBase, CreatureKind, Monster, MonsterAiConfig, MonsterState, Outfit, Player,
    PlayerEconomy, PlayerInventory, PlayerPersistBaseline, PlayerSkills, PlayerSocial,
};
use tfs_rust_core::event_dispatcher::NullEventDispatcher;
use tfs_rust_core::game_world::GameWorld;
use tfs_rust_core::ids::CreatureId;
use tfs_rust_core::inventory::SLOTP_BACKPACK;
use tfs_rust_core::map::{Map, SparseGrid};
use tfs_rust_core::spawn::SpawnManager;
use tfs_rust_core::tile::{Tile, TileBody};
use tfs_rust_core::{Mechanics, load_mechanics};
use tfs_rust_db::DbPool;
use tfs_rust_db::player::PlayerRecord;

use crate::clock::reset_harness_scenario_clock;

pub fn test_runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Runtime::new().expect("tokio runtime for tests"))
}

fn sim_config() -> ConfigManager {
    let path = std::env::temp_dir().join(format!(
        "tfs_sim_config_{}_{}.lua",
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

/// 772 human hero for chase parity sim — matches C++ `TKiteSimPlayer` + `human.mon` race data.
/// C++ reference: `chase_kite_scenario.cc` `TKiteSimPlayer`; `runtime/mon/human.mon` `Defend=5`.
pub fn sim_hero_player(name: &str, pos: Position) -> Player {
    let mut p = sim_player_base(name, pos);
    p.base.health = 150;
    p.base.max_health = 150;
    p.fist_defense = 5;
    p.fist_attack = 7;
    p
}

fn sim_player_base(name: &str, pos: Position) -> Player {
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
        sex: tfs_rust_core::creature::PlayerSex::Male,
        vocation_id: 0,
        vocation_profile: tfs_rust_core::creature::vocation::VocationProfile::none_vocation(),
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

fn bag_item_type(server_id: u16) -> ItemType {
    let mut it = ItemType {
        group: ItemType::GROUP_CONTAINER,
        allow_pickupable: true,
        server_id,
        slot_position: SLOTP_BACKPACK,
        ..Default::default()
    };
    it.xml_attributes
        .insert("containersize".into(), "20".into());
    it
}

fn pickup_item_type(server_id: u16) -> ItemType {
    ItemType {
        allow_pickupable: true,
        moveable_override: Some(true),
        server_id,
        ..Default::default()
    }
}

fn synthetic_ground_item_type(server_id: u16, waypoint: u16) -> ItemType {
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

/// Headless / battery seed — `TFS_SIM_SEED` is read here, never inside `GameWorld` combat.
fn sim_seed_from_env() -> Option<u32> {
    let seed_str = std::env::var("TFS_SIM_SEED").ok()?;
    seed_str.parse::<u64>().ok().map(|s| s as u32)
}

pub fn seed_world_from_sim_env(world: &mut GameWorld) {
    if let Some(seed) = sim_seed_from_env() {
        world.seed_parity_rng(seed);
    }
}

pub fn load_items_db_for(data_dir: &Path) -> Result<ItemDatabase, String> {
    let otb = data_dir.join("items/items.otb");
    let xml = data_dir.join("items/items.xml");
    if !otb.is_file() {
        return Err(format!("items.otb not found: {}", otb.display()));
    }
    if !xml.is_file() {
        return Err(format!("items.xml not found: {}", xml.display()));
    }
    let db = ItemDatabase::load(&otb, &xml).map_err(|e| e.to_string())?;
    Ok(db)
}

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

pub fn init_beat_driven_world(
    map: Map,
    items: SlotMap<tfs_rust_core::ids::ItemId, tfs_rust_core::item::Item>,
    items_db: Arc<ItemDatabase>,
    monsters_db: Arc<MonsterDatabase>,
    mechanics: Mechanics,
) -> GameWorld {
    let mut world = GameWorld::new(
        map,
        items,
        Box::new(NullEventDispatcher),
        Rc::new(sim_config()),
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
        tfs_rust_net::Codec::from_version(ProtocolVersion::V772).expect("772 codec"),
        mechanics,
    );
    reset_harness_scenario_clock();
    seed_world_from_sim_env(&mut world);
    world
}

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
        refresh_snapshots: HashMap::new(),
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
        load_mechanics(data_dir, ProtocolVersion::V772)
    } else {
        Mechanics::for_version(ProtocolVersion::V772)
    };

    Ok(init_beat_driven_world(
        map,
        SlotMap::default(),
        items_db,
        monsters_db,
        mechanics,
    ))
}

/// Empty synthetic beat-driven world (no data-pack directory).
pub fn beat_driven_world() -> GameWorld {
    beat_driven_world_with_synthetic_ground(None)
}

/// Synthetic chase arena — uniform walkable tiles with pinned waypoint cost.
pub fn beat_driven_world_with_synthetic_ground(waypoint: Option<u16>) -> GameWorld {
    beat_driven_world_with_synthetic_ground_data(Path::new("/nonexistent"), waypoint)
        .unwrap_or_else(|_| panic!("synthetic world without data dir failed"))
}

pub fn insert_player(world: &mut GameWorld, player: Player) -> CreatureId {
    world.creatures.insert(CreatureKind::Player(player))
}

pub fn ensure_walkable_tile(map: &mut Map, pos: Position, ground_type: u16) {
    map.insert_tile(
        pos,
        Tile::Normal(TileBody {
            ground: Some(ground_type),
            ground_item: None,
            down_items: Vec::new(),
            top_items: Vec::new(),
            creatures: Vec::new(),
            flags: 0,
            zone: ZoneType::Normal,
        }),
    );
}

pub fn ensure_walkable_tile_if_absent(map: &mut Map, pos: Position) {
    if map.get_tile(pos).is_none() {
        ensure_walkable_tile(map, pos, 100);
    }
}

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
    u32::from(waypoint)
}

fn monster_base(
    name: &str,
    pos: Position,
    speed: i32,
    health: i32,
    max_health: i32,
) -> CreatureBase {
    CreatureBase {
        name: name.into(),
        position: pos,
        direction: Direction::North,
        health,
        max_health,
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
    let cid = world
        .creatures
        .insert(CreatureKind::Monster(Monster::with_config(
            monster_base(name, pos, speed, 100, 100),
            pos,
            config,
        )));
    world.assign_creature_wire_id(cid);
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
    let mut base = monster_base(
        display_name,
        pos,
        speed,
        mtype.health_now as i32,
        mtype.health_max as i32,
    );
    base.outfit = monster_outfit_to_sim(&mtype.outfit);
    let cid = world
        .creatures
        .insert(CreatureKind::Monster(Monster::with_config(
            base, pos, config,
        )));
    world.assign_creature_wire_id(cid);
    if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(cid) {
        m.experience = mtype.experience;
        m.corpse_id = mtype.outfit.corpse_id;
        m.blood = mtype.blood_type();
        m.state = initial_state;
        m.is_idle = true;
    }
    world.roll_monster_spawn_loot(cid, mtype);
    world.recompute_monster_combat_from_equipment(cid);
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
    ) > 0
}
