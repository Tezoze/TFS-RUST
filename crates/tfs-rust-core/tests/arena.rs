use std::collections::VecDeque;

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
            condition_ids: Vec::new(),
            walk_queue: VecDeque::new(),
            follow_target: None,
            attack_target: None,
            master: None,
            damage_map: Default::default(),
        },
        account_id: 0,
        guid,
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
