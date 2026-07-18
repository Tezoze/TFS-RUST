use super::*;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::creature::Player;
use crate::creature::PlayerEconomy;
use crate::creature::PlayerInventory;
use crate::creature::PlayerSkills;
use crate::creature::PlayerSocial;
use crate::creature::{CreatureBase, Outfit};
use tfs_rust_common::enums::{Direction, SkullType};
use tfs_rust_common::Position;

fn minimal_player(next_action_until: Option<u64>) -> Player {
    Player {
        base: CreatureBase {
            name: "t".into(),
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
            walk_queue: VecDeque::new(),
            walk_destinations: VecDeque::new(),
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
            damage_map: Default::default(),
            earliest_attack_ms: 0,
            earliest_defend_ms: 0,
            last_defend_ms: 0,
            learning_points: 0,
            todo: Default::default(),
            chase_mode: Default::default(),
            last_auto_walk_armed_ms: u64::MAX,
        },
        account_id: 1,
        guid: 1,
        account_type: 1,
        group_id: 1,
        set_max_speed: false,
        sex: crate::creature::PlayerSex::Male,
        vocation_id: 1,
        vocation_profile: crate::creature::vocation::VocationProfile::none_vocation(),
        level: 50,
        experience: 0,
        mana: 100,
        max_mana: 100,
        capacity: 100,
        inventory: PlayerInventory::default(),
        skills: PlayerSkills::with_levels(10, 10, 10, 10, 10, 10, 10, 10),
        economy: PlayerEconomy {
            balance: 0,
            soul: 100,
        },
        social: PlayerSocial::default(),
        town_id: 1,
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
        items_light: crate::creature::LightInfo::default(),
        inventory_abilities: [false; 11],
        var_skills: [0; 7],
        var_stats: [0; 4],
        condition_suppressions: 0,
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
        next_action_until,
        walk_action: None,
        depot_chests: HashMap::new(),
        depot_lockers: HashMap::new(),
        inbox_root: None,
        last_depot_id: -1,
        persist: None,
        sim_melee_defense: 0,
        sim_melee_attack: 0,
        attack_mode: Default::default(),
        secure_mode: false,
        earliest_protection_zone_round: 0,
        message_buffer_count: 0,
        message_buffer_ticks: 0,
        blessings: 0,
    }
}

// Phase 4: `can_cast_instant_blocks_while_next_action_in_future` deleted — the 1098
// `nextAction` gate was removed; both eras use `EarliestSpellTime` (covered by the 772
// test below).

#[test]
fn can_cast_instant_772_blocks_on_earliest_spell_time() {
    let spell = SpellDefinition {
        id: 1,
        level: 1,
        mana: 0,
        soul: 0,
        cooldown_ticks: 0,
        group_id: 0,
        group_cooldown_ticks: 0,
        vocation_mask: 0xFFFF_FFFF,
    };
    let now_tick: u64 = 1_000;
    let mut p = minimal_player(None);
    p.base.earliest_spell_server_ms = now_tick + 500;
    assert_eq!(
        can_cast_instant(&p, &spell, now_tick),
        Err(SpellFailReason::NextAction)
    );
    p.base.earliest_spell_server_ms = now_tick;
    assert!(can_cast_instant(&p, &spell, now_tick).is_ok());
}
