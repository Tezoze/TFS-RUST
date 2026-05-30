use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use slotmap::SlotMap;
use tfs_rust_common::enums::{Direction, SkullType};
use tfs_rust_common::Position;
use tfs_rust_core::{
    CreatureBase, CreatureId, CreatureKind, Outfit, Player, PlayerEconomy, PlayerInventory,
    PlayerSkills, PlayerSocial,
};

fn test_player(name: &str, guid: u32, pos: Position) -> Player {
    Player {
        base: CreatureBase {
            name: name.to_string(),
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
            walk_queue: VecDeque::new(),
            last_step: None,
            last_step_cost: 1,
            last_step_ground_speed: 150,
            next_walk_check: None,
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
        },
        account_id: 0,
        guid,
        group_id: 1,
        vocation_id: 0,
        level: 1,
        experience: 0,
        mana: 0,
        max_mana: 0,
        capacity: 400,
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
            soul: 0,
        },
        social: PlayerSocial::default(),
        town_id: 0,
        premium_ends_at: 0,
        stamina_minutes: 0,
        offline_training_ms: 0,
        spell_cooldown_end: HashMap::new(),
        spell_group_cooldown_end: HashMap::new(),
        operating_system: 0,
        otclient_v8: 0,
        ghost_mode: false,
        equipment_slots: std::array::from_fn(|_| None),
        inventory_weight: 0,
        items_light: tfs_rust_core::LightInfo::default(),
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
        persist: None,
    }
}

#[test]
fn slotmap_generation_invalidation() {
    let mut sm: SlotMap<CreatureId, CreatureKind> = SlotMap::with_key();
    let id = sm.insert(CreatureKind::Player(test_player(
        "a",
        1,
        Position::new(1, 1, 7),
    )));
    sm.remove(id);
    let id2 = sm.insert(CreatureKind::Player(test_player(
        "b",
        2,
        Position::new(2, 2, 7),
    )));
    assert_ne!(id, id2);
}
