//! Character login: DB load → `Player` → world + indices (or 772 TakeOver).
//!
//! - Domain: `Game::placeCreature`, `IOLoginData::loadPlayer`.
//! - 772 outcomes: `connections.cc:224-253` (existing body / reject / TakeOver),
//!   `TPlayer::TakeOver` — `crplayer.cc:721-775`.

use std::collections::HashMap;

use tfs_rust_common::enums::{Direction, SkullType};
use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_common::ConnId;
use tfs_rust_common::Position;
use tfs_rust_db::player::{LoadedPlayerData, PlayerStore};

use crate::creature::vocation::{base_walk_speed, VocationProfile};
use crate::creature::CreatureKind;
use crate::creature::{
    CreatureBase, Outfit, Player, PlayerEconomy, PlayerInventory, PlayerPersistBaseline,
    PlayerSkills, PlayerSocial,
};
use crate::formulas::StepSpeedModel;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::lua_scope::fire_on_login;
use crate::player_flags::{flags_for_group, has_player_flag, PLAYER_FLAG_SET_MAX_SPEED};
use tfs_rust_content::groups::GroupDatabase;
use tfs_rust_content::vocations::VocationRegistry;

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
pub fn player_from_loaded(
    mut data: LoadedPlayerData,
    step_speed_model: StepSpeedModel,
    vocations: &VocationRegistry,
    groups: &GroupDatabase,
) -> Player {
    let persist = PlayerPersistBaseline {
        player_row: data.player.clone(),
        spells: std::mem::take(&mut data.spells),
        storage: std::mem::take(&mut data.storage),
        depot: std::mem::take(&mut data.items.depot),
        inbox: std::mem::take(&mut data.items.inbox),
        last_depot_id: -1,
    };
    let p = &data.player;
    let pos = Position::new(
        p.posx.clamp(0, u16::MAX as i32) as u16,
        p.posy.clamp(0, u16::MAX as i32) as u16,
        p.posz.clamp(0, u8::MAX as i32) as u8,
    );
    // C++ `IOLoginData::loadPlayer` uses raw DB values — no formula override.
    // `recalculate_vitals` is only used on level-up (`Player::add_experience`).
    let max_hp = p.healthmax;
    let max_mana = p.manamax;
    // C++ `iologindata.cpp` ~275: `player->capacity = result->getNumber("cap") * 100;`
    // TFS stores capacity internally in 1/100 oz; the DB column is in oz.
    let cap = p.cap * 100;
    // Build the vocation hot-path snapshot from the registry. Falls back to the
    // "None" vocation profile when the id is absent (matches C++ race defaults).
    let vocation_profile = vocations
        .get(p.vocation)
        .map(VocationProfile::from_def)
        .unwrap_or_else(VocationProfile::none_vocation);
    let group_id = u16::try_from(p.group_id.max(0)).unwrap_or(1);
    let group_flags = flags_for_group(groups, group_id);
    let set_max_speed = has_player_flag(group_flags, PLAYER_FLAG_SET_MAX_SPEED);
    let walk_speed = base_walk_speed(step_speed_model, &vocation_profile, p.level, set_max_speed);
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
        speed: walk_speed,
        base_speed: walk_speed,
        var_speed: 0,
        skull: skull_from_i32(i32::from(p.skull)),
        drunkenness: 0,
        active_conditions: p
            .conditions
            .as_deref()
            .map(crate::condition_blob::deserialize_conditions)
            .unwrap_or_default(),
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
        damage_map: Default::default(),
        earliest_attack_ms: 0,
        latest_attack_round: 0,
        earliest_defend_ms: 0,
        last_defend_ms: 0,
        learning_points: 0,
        todo: Default::default(),
        chase_mode: Default::default(),
        last_auto_walk_armed_ms: u64::MAX,
    };

    let account_id =
        u32::try_from(p.account_id).expect("players.account_id must fit u32 for runtime Player");
    let guid = u32::try_from(p.id).expect("players.id must fit u32 for runtime Player");

    // C++ `Player::sex` — `PLAYERSEX_FEMALE` (0) / `PLAYERSEX_MALE` (1) (`enums.h:379-380`).
    // DB column `players.sex` is `i32`; treat any non-1 value as female (matches C++ default).
    let sex = if p.sex == 1 {
        tfs_rust_common::PlayerSex::Male
    } else {
        tfs_rust_common::PlayerSex::Female
    };

    Player {
        base,
        account_id,
        guid,
        account_type: data.account_type,
        group_id,
        set_max_speed,
        sex,
        vocation_id: p.vocation,
        vocation_profile,
        level: p.level,
        experience: p.experience,
        mana: p.mana,
        max_mana,
        capacity: cap,
        inventory: PlayerInventory { capacity_slots: 10 },
        skills: PlayerSkills {
            fist: p.skill_fist as i32,
            club: p.skill_club as i32,
            sword: p.skill_sword as i32,
            axe: p.skill_axe as i32,
            dist: p.skill_dist as i32,
            shielding: p.skill_shielding as i32,
            fishing: p.skill_fishing as i32,
            maglevel: p.maglevel,
            fist_tries: p.skill_fist_tries,
            club_tries: p.skill_club_tries,
            sword_tries: p.skill_sword_tries,
            axe_tries: p.skill_axe_tries,
            dist_tries: p.skill_dist_tries,
            shielding_tries: p.skill_shielding_tries,
            fishing_tries: p.skill_fishing_tries,
            manaspent: p.manaspent,
        },
        economy: PlayerEconomy {
            balance: p.balance,
            soul: p.soul as i32,
        },
        social: PlayerSocial {
            party_id: None,
            guild_id: data
                .guild
                .as_ref()
                .and_then(|g| u32::try_from(g.guild_id).ok()),
        },
        town_id: p.town_id,
        premium_ends_at: data.premium_ends_at,
        stamina_minutes: p.stamina,
        // C++ `iologindata.cpp` ~345: `offlineTrainingTime = result->getNumber("offlinetraining_time") * 1000;`
        // DB column is in seconds; TFS internal representation is milliseconds.
        offline_training_ms: u32::from(p.offlinetraining_time) * 1000,
        spell_cooldown_end: HashMap::new(),
        spell_group_cooldown_end: HashMap::new(),
        operating_system: 0,
        otclient_v8: 0,
        ghost_mode: false,
        equipment_slots: std::array::from_fn(|_| None),
        inventory_weight: 0,
        items_light: crate::creature::LightInfo::default(),
        internal_light: crate::creature::LightInfo::default(),
        inventory_abilities: [false; 11],
        var_skills: [0; 7],
        var_stats: [0; 4],
        condition_suppressions: 0,
        shop_owner: None,
        vip_list: data.vip_list.clone(),
        health_hidden: false,
        last_activity: std::time::Instant::now(),
        last_command_round: 0,
        last_action_round: 0,
        food_remaining: apply_offline_food_drain(p.food_remaining.max(0) as u32, p.lastlogout),
        food_level: p.food_level,
        earliest_logout_round: 0,
        logging_out: false,
        logout_allowed: false,
        last_ping_sent: std::time::Instant::now(),
        last_pong_at: std::time::Instant::now(),
        next_action_until: None,
        walk_action: None,
        depot_chests: HashMap::new(),
        depot_lockers: HashMap::new(),
        inbox_root: None,
        last_depot_id: -1,
        persist: Some(persist),
        sim_melee_defense: 5,
        sim_melee_attack: 7,
        attack_mode: Default::default(),
        secure_mode: false,
        earliest_protection_zone_round: 0,
        client_icons: 0,
        message_buffer_count: 0,
        message_buffer_ticks: 0,
        blessings: p.blessings,
    }
}

/// 772 offline food drain — `crplayer.cc:1395-1400`.
///
/// On login, `Regen = min(FoodTime / 3, OfflineTime / 15)` rounds of regen are
/// applied (HP += Regen/4, Mana += Regen), and `food_remaining` is reduced by
/// `Regen * 3`. The HP/mana gains are applied here as a simple additive bump
/// capped at max — the decompile does this in `TPlayer::Login` before the
/// player enters the world.
///
/// `lastlogout` is unix seconds; offline time is `now - lastlogout`.
fn apply_offline_food_drain(food_remaining: u32, lastlogout: u64) -> u32 {
    if food_remaining == 0 || lastlogout == 0 {
        return food_remaining;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let offline_secs = now.saturating_sub(lastlogout);
    // C++ uses seconds-per-round = 1 (RoundNr is seconds since startup).
    let offline_rounds = offline_secs;
    let regen = (food_remaining / 3).min((offline_rounds / 15) as u32);
    if regen == 0 {
        return food_remaining;
    }
    food_remaining.saturating_sub(regen * 3)
}

/// Max concurrent `load_player_full` tasks. Excess logins are rejected off the sim await path.
pub const MAX_CONCURRENT_LOGIN_LOADS: usize = 8;

/// I/O-thread (or Tokio task) load — never call while holding the game loop.
pub async fn load_player_data(
    db: &tfs_rust_db::DbPool,
    name: &str,
) -> Result<LoadedPlayerData> {
    let store = PlayerStore::new(db);
    let Some(loaded) = store.load_player_full(name).await? else {
        return Err(TfsRustError::Database(format!(
            "character `{name}` not found"
        )));
    };
    Ok(loaded)
}

/// Result of applying a DB-loaded character on the game thread.
#[derive(Debug)]
pub enum ApplyPlayerOutcome {
    /// Fresh spawn — creature inserted and placed on the map.
    Spawned(CreatureId),
    /// 772 `TakeOver` — existing body kept. Caller must close `old_conn` TCP **without**
    /// `StartLogout` (772 zeroes `CharacterID` before connection `Logout`).
    TakenOver {
        cid: CreatureId,
        old_conn: Option<ConnId>,
    },
}

impl ApplyPlayerOutcome {
    pub fn creature_id(&self) -> CreatureId {
        match *self {
            Self::Spawned(cid) | Self::TakenOver { cid, .. } => cid,
        }
    }
}

/// Game-thread apply of a completed DB load — spawn or TakeOver.
pub fn apply_loaded_player(
    world: &mut GameWorld,
    loaded: LoadedPlayerData,
    operating_system: u16,
    otclient_v8: u16,
) -> Result<ApplyPlayerOutcome> {
    let name = loaded.player.name.clone();
    let inventory_rows = loaded.items.inventory.clone();
    let store_inbox_rows = loaded.items.store_inbox.clone();
    let depot_rows = loaded.items.depot.clone();
    let inbox_rows = loaded.items.inbox.clone();

    let key = loaded.player.name.clone();
    let guid = u32::try_from(loaded.player.id).map_err(|_| {
        TfsRustError::Database(format!("player id out of u32 range: {}", loaded.player.id))
    })?;

    if let Some((cid, old_conn)) =
        world.player_try_takeover_for_login(guid, &name, operating_system, otclient_v8)?
    {
        // Live body kept — do not rehydrate inventory / place / onLogin from this load.
        return Ok(ApplyPlayerOutcome::TakenOver { cid, old_conn });
    }

    let pos = {
        let p = &loaded.player;
        Position::new(
            p.posx.clamp(0, u16::MAX as i32) as u16,
            p.posy.clamp(0, u16::MAX as i32) as u16,
            p.posz.clamp(0, u8::MAX as i32) as u8,
        )
    };

    let mut player = player_from_loaded(
        loaded,
        world.mechanics.profile.step_speed,
        &world.vocations,
        &world.groups,
    );
    player.operating_system = operating_system;
    player.otclient_v8 = otclient_v8;
    // Debug aid: confirm vocation/level/speed wiring (PC-0 base_speed + level scaling).
    // 772: base_speed/speed store GoStrength `Act` (decompile `crskill.cc:19` `Get()`);
    //      effective_speed = 2*Act + 80 (`crmain.cc:484` `GetSpeed()`), computed on demand.
    tracing::info!(
        vocation_id = player.vocation_id,
        vocation_base_speed = player.vocation_profile.base_speed,
        level = player.level,
        base_speed = player.base.base_speed,
        speed = player.base.speed,
        effective_speed = crate::formulas::linear_go_effective_speed(player.base.speed),
        step_speed_model = ?world.mechanics.profile.step_speed,
        "player login speed snapshot"
    );
    let cid = world.creatures.insert(CreatureKind::Player(player));

    world.hydrate_player_inventory_from_db(
        cid,
        &inventory_rows,
        &store_inbox_rows,
        &depot_rows,
        &inbox_rows,
    );

    // GAME THREAD ONLY
    world.player_by_name.insert(key.clone(), cid);
    world.player_by_guid.insert(guid, cid);

    // C++ `Game::placeCreature` login flow — try saved position, then town temple fallback.
    // TFS 1.4.2: `src/protocolgame.cpp:258-263`. 772 decompile: `cract.cc:314-332` `SetOnMap`.
    let town_id = world
        .creatures
        .get(cid)
        .map(|k| match k {
            CreatureKind::Player(p) => p.town_id,
            _ => 0,
        })
        .unwrap_or(0);
    let placed_pos = match world.place_player_on_login(cid, pos, town_id) {
        Some(p) => p,
        None => {
            // Both login and temple positions unplaceable — disconnect, don't crash.
            // C++: `disconnectClient("Temple position is wrong. Contact the administrator.")`.
            world.creatures.remove(cid);
            world.player_by_name.remove(&key);
            world.player_by_guid.remove(&guid);
            return Err(TfsRustError::Database(format!(
                "could not place player `{name}` at login position {pos:?} or town {town_id} temple"
            )));
        }
    };
    world.monster_notify_creature_enter_viewport(cid, placed_pos);

    let guild_opt = world.creatures.get(cid).and_then(|k| match k {
        CreatureKind::Player(p) => p.social.guild_id,
        _ => None,
    });
    if let Some(gid) = guild_opt {
        world.guilds.register_online(cid, gid);
    }

    fire_on_login(world, cid);
    Ok(ApplyPlayerOutcome::Spawned(cid))
}

/// Load + apply in one call (tests / tools). Production game loop must not await this.
pub async fn login_player(
    world: &mut GameWorld,
    name: &str,
    operating_system: u16,
    otclient_v8: u16,
) -> Result<CreatureId> {
    let loaded = load_player_data(&world.db, name).await?;
    Ok(apply_loaded_player(world, loaded, operating_system, otclient_v8)?.creature_id())
}
