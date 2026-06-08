//! Minimal `GameWorld` builder for unit tests (never touches the database).
#[cfg(test)]
pub mod support {
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::{Arc, OnceLock};
    use std::time::Instant;

    use slotmap::SlotMap;
    use tfs_rust_common::enums::{Direction, SkullType};
    use tfs_rust_common::Position;
    use tfs_rust_content::groups::GroupDatabase;
    use tfs_rust_content::items::ItemDatabase;
    use tfs_rust_content::otb::ItemType;
    use tfs_rust_content::otbm::TownData;
    use tfs_rust_content::vocations::VocationDatabase;
    use tfs_rust_db::player::PlayerRecord;
    use tfs_rust_db::DbPool;

    use crate::config::ConfigManager;
    use crate::creature::{
        CreatureBase, CreatureKind, Monster, MonsterAiConfig, Npc, Outfit, Player, PlayerEconomy, PlayerInventory,
        PlayerPersistBaseline, PlayerSkills, PlayerSocial,
    };
    use crate::event_dispatcher::{EventDispatcher, NullEventDispatcher};
    use crate::game_world::GameWorld;
    use crate::ids::CreatureId;
    use crate::map::{Map, SparseGrid};
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

    fn test_runtime() -> &'static tokio::runtime::Runtime {
        static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
        RT.get_or_init(|| tokio::runtime::Runtime::new().expect("tokio runtime for tests"))
    }

    pub fn minimal_world() -> GameWorld {
        let _guard = test_runtime().enter();
        let mut items_map = HashMap::new();
        items_map.insert(1987u16, bag_item_type(1987)); // backpack
        items_map.insert(2148u16, pickup_item_type(2148)); // gold coin
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

    /// 772 beat-driven profile (`LinearGo` + reverse terrain path) for idle/todo/monster tests.
    pub fn beat_driven_world() -> GameWorld {
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
            tfs_rust_net::Codec::from_version(tfs_rust_common::ProtocolVersion::V772)
                .expect("772 codec"),
            crate::formulas::Mechanics::for_version(tfs_rust_common::ProtocolVersion::V772),
        )
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
            todo: Default::default(),
        };
        let cid = world
            .creatures
            .insert(CreatureKind::Monster(Monster::with_config(base, pos, config)));
        world.map.register_creature_at(pos, cid);
        cid
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

    /// Test helper — counts `EventDispatcher::on_think` calls per creature.
    #[derive(Debug, Default)]
    pub struct CountingEventDispatcher {
        think_calls: std::sync::Mutex<HashMap<CreatureId, u32>>,
        intervals: std::sync::Mutex<Vec<u32>>,
    }

    impl CountingEventDispatcher {
        pub fn total_think_calls(&self) -> u32 {
            self.think_calls
                .lock()
                .expect("lock")
                .values()
                .sum()
        }

        pub fn intervals(&self) -> Vec<u32> {
            self.intervals.lock().expect("lock").clone()
        }
    }

    impl EventDispatcher for CountingEventDispatcher {
        fn on_think(&self, creature: CreatureId, interval_ms: u32) {
            *self
                .think_calls
                .lock()
                .expect("lock")
                .entry(creature)
                .or_insert(0) += 1;
            self.intervals.lock().expect("lock").push(interval_ms);
        }
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
}
