//! Character login: DB load → `Player` → world + indices.
// C++ reference: `Game::placeCreature`, `IOLoginData::loadPlayer`.

use std::collections::HashMap;

use tfs_rust_common::enums::{Direction, SkullType};
use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_common::Position;
use tfs_rust_db::player::{LoadedPlayerData, PlayerStore};

use crate::creature::vocation::recalculate_vitals;
use crate::creature::CreatureKind;
use crate::creature::{
    CreatureBase, Outfit, Player, PlayerEconomy, PlayerInventory, PlayerSkills, PlayerSocial,
};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

fn direction_from_u8(d: u8) -> Direction {
    match d {
        0 => Direction::North,
        1 => Direction::East,
        2 => Direction::South,
        3 => Direction::West,
        _ => Direction::South,
    }
}

fn skull_from_i32(s: i32) -> SkullType {
    match s {
        1 => SkullType::Yellow,
        2 => SkullType::Green,
        3 => SkullType::White,
        4 => SkullType::Red,
        5 => SkullType::Black,
        6 => SkullType::Orange,
        _ => SkullType::None,
    }
}

/// Build runtime `Player` from SQL load result.
pub fn player_from_loaded(data: LoadedPlayerData) -> Player {
    let p = &data.player;
    let pos = Position::new(
        p.posx.clamp(0, u16::MAX as i32) as u16,
        p.posy.clamp(0, u16::MAX as i32) as u16,
        p.posz.clamp(0, u8::MAX as i32) as u8,
    );
    let (max_hp, max_mana, cap) = recalculate_vitals(p.vocation, p.level);
    let max_hp = max_hp.max(p.healthmax);
    let max_mana = max_mana.max(p.manamax);
    let cap = cap.max(p.cap);
    let outfit = Outfit {
        look_type: p.looktype,
        look_head: p.lookhead,
        look_body: p.lookbody,
        look_legs: p.looklegs,
        look_feet: p.lookfeet,
        look_addons: p.lookaddons,
    };
    let base = CreatureBase {
        name: p.name.clone(),
        position: pos,
        direction: direction_from_u8(p.direction),
        health: p.health.min(max_hp).max(1),
        max_health: max_hp,
        outfit,
        speed: 220,
        base_speed: 220,
        skull: skull_from_i32(p.skull),
        active_conditions: Vec::new(),
        walk_queue: Default::default(),
        follow_target: None,
        attack_target: None,
        master: None,
        damage_map: Default::default(),
    };

    Player {
        base,
        account_id: p.account_id,
        guid: p.id,
        vocation_id: p.vocation,
        level: p.level,
        experience: p.experience,
        mana: p.mana,
        max_mana: max_mana.max(p.manamax),
        capacity: cap.max(p.cap),
        inventory: PlayerInventory { capacity_slots: 10 },
        skills: PlayerSkills {
            fist: p.skill_fist,
            club: p.skill_club,
            sword: p.skill_sword,
            axe: p.skill_axe,
            dist: p.skill_dist,
            shielding: p.skill_shielding,
            fishing: p.skill_fishing,
            maglevel: p.maglevel,
        },
        economy: PlayerEconomy {
            balance: p.balance,
            soul: p.soul,
        },
        social: PlayerSocial {
            party_id: None,
            guild_id: data.guild.as_ref().map(|g| g.guild_id),
        },
        town_id: p.town_id,
        spell_cooldown_end: HashMap::new(),
        spell_group_cooldown_end: HashMap::new(),
    }
}

/// Load character by name, insert into world and indices. Returns new `CreatureId`.
pub async fn login_player(world: &mut GameWorld, name: &str) -> Result<CreatureId> {
    let store = PlayerStore::new(&world.db);
    let Some(loaded) = store.load_player_full(name).await? else {
        return Err(TfsRustError::Database(format!(
            "character `{name}` not found"
        )));
    };

    let key = loaded.player.name.clone();
    let guid = loaded.player.id;
    let pos = {
        let p = &loaded.player;
        Position::new(
            p.posx.clamp(0, u16::MAX as i32) as u16,
            p.posy.clamp(0, u16::MAX as i32) as u16,
            p.posz.clamp(0, u8::MAX as i32) as u8,
        )
    };

    let player = player_from_loaded(loaded);
    let cid = world.creatures.insert(CreatureKind::Player(player));

    // GAME THREAD ONLY
    world.player_by_name.insert(key, cid);
    world.player_by_guid.insert(guid, cid);
    world.map.register_creature_index(pos, cid);

    let guild_opt = world.creatures.get(cid).and_then(|k| match k {
        CreatureKind::Player(p) => p.social.guild_id,
        _ => None,
    });
    if let Some(gid) = guild_opt {
        world.guilds.register_online(cid, gid);
    }

    world.events.on_login(cid);
    Ok(cid)
}
