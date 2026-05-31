//! First game-protocol burst after `Player` is placed (`ProtocolGame::sendAddCreature` self branch + map).
// C++ reference: `src/protocolgame.cpp` `ProtocolGame::login` (OTCv8 preamble), `sendAddCreature` (player), …

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use slotmap::Key;
use tracing::warn;
use tfs_rust_common::enums::{ConditionType, SkullType};
use tfs_rust_common::ConnId;
use tfs_rust_common::Position;

use crate::creature::CreatureKind;
use crate::creature::LightInfo;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::{Monster, Npc, Outfit, Player};

use tfs_rust_net::creature_encode::{AddCreatureWire, OutfitWire};
use tfs_rust_net::map_description::{
    send_map_description_packet, send_map_description_stub, ItemStack, TileContent,
};
use tfs_rust_net::outgoing::{send_extended_opcode, send_magic_effect, send_otcv8_features};
use tfs_rust_net::codec::{ItemTemplateArgs, PlayerSkillsWire};
use tfs_rust_net::outgoing_extra::{
    send_enter_world, send_fight_modes, send_icons, send_inventory_slot_empty,
    send_otc_features_raw, send_pending_state_entered, send_unjustified_stats_stub, send_vip_entry,
    send_world_light,
};

/// `GameFeature` (`src/const.h`) — `ProtocolGame::sendFeatures` (OTCv8), not `sendOTCFeatures`.
const OTC_FEATURE_EXTENDED_OPCODE: u8 = 80;
const OTC_FEATURE_ITEM_TOOLTIP: u8 = 93;

fn skull_byte(s: SkullType) -> u8 {
    match s {
        SkullType::None => 0,
        SkullType::Yellow => 1,
        SkullType::Green => 2,
        SkullType::White => 3,
        SkullType::Red => 4,
        SkullType::Black => 5,
        SkullType::Orange => 6,
    }
}

fn outfit_to_wire(o: &Outfit) -> OutfitWire {
    OutfitWire {
        look_type: o.look_type.max(0) as u16,
        look_head: o.look_head.clamp(0, 255) as u8,
        look_body: o.look_body.clamp(0, 255) as u8,
        look_legs: o.look_legs.clamp(0, 255) as u8,
        look_feet: o.look_feet.clamp(0, 255) as u8,
        look_addons: o.look_addons.clamp(0, 255) as u8,
        look_mount: 0,
        look_type_ex: 0,
    }
}

fn health_percent(cur: i32, max_hp: i32) -> u8 {
    if max_hp <= 0 {
        return 100;
    }
    ((cur.max(0) as u64 * 100) / max_hp as u64).min(100) as u8
}

fn step_speed(base_speed: i32) -> u16 {
    base_speed.max(0).min(u16::MAX as i32) as u16
}

/// Non-player creatures: use slot key index (low 32 bits of `KeyData::as_ffi`) as protocol id until a global id allocator exists.
fn non_player_wire_id(cid: CreatureId) -> u32 {
    (cid.data().as_ffi() & 0xFFFF_FFFF) as u32
}

/// Protocol creature id for move/turn packets (`protocolgame.cpp` `sendMoveCreature`).
pub(crate) fn creature_wire_id(cid: CreatureId, kind: &CreatureKind) -> u32 {
    match kind {
        CreatureKind::Player(p) => p.guid,
        CreatureKind::Monster(_) | CreatureKind::Npc(_) => non_player_wire_id(cid),
    }
}

/// C++ `ProtocolGame::AddCreature` — `protocolgame.cpp` ~3206 (`getCreatureLight`, viewer `isAccessPlayer`).
/// Build `AddCreatureWire` for map description and tile appear packets.
pub(crate) fn build_add_creature_wire(
    world: &GameWorld,
    cid: CreatureId,
    viewer: CreatureId,
) -> AddCreatureWire {
    let viewer_access = world.player_is_access_player(viewer);
    match world.creatures.get(cid) {
        Some(CreatureKind::Player(p)) => {
            let light = world.player_creature_light(cid);
            let is_self = world
                .creatures
                .get(viewer)
                .and_then(|k| match k {
                    CreatureKind::Player(vp) => Some(vp.guid == p.guid),
                    _ => None,
                })
                .unwrap_or(false);
            player_to_add_creature_wire(p, is_self, light, viewer_access)
        }
        Some(CreatureKind::Monster(m)) => monster_to_add_creature_wire(cid, m),
        Some(CreatureKind::Npc(n)) => npc_to_add_creature_wire(cid, n),
        None => AddCreatureWire::default(),
    }
}

fn player_to_add_creature_wire(
    p: &Player,
    is_self: bool,
    light: LightInfo,
    viewer_is_access: bool,
) -> AddCreatureWire {
    let hp = if !is_self && p.health_hidden {
        0
    } else {
        health_percent(p.base.health, p.base.max_health)
    };
    AddCreatureWire {
        id: p.guid,
        remove_known: 0,
        known: false,
        creature_type: 0,
        name: p.base.name.clone(),
        health_percent: hp,
        direction: p.base.direction as u8,
        outfit: outfit_to_wire(&p.base.outfit),
        light_level: light.level,
        light_color: light.color,
        step_speed: step_speed(p.base.speed),
        skull: skull_byte(p.base.skull),
        party_shield: 0,
        guild_emblem: 0,
        speech_bubble: 0,
        helpers: 0,
        walkthrough_blocked: 1,
        access_player: viewer_is_access,
    }
}

fn monster_to_add_creature_wire(cid: CreatureId, m: &Monster) -> AddCreatureWire {
    AddCreatureWire {
        id: non_player_wire_id(cid),
        remove_known: 0,
        known: false,
        creature_type: 1,
        name: m.base.name.clone(),
        health_percent: health_percent(m.base.health, m.base.max_health),
        direction: m.base.direction as u8,
        outfit: outfit_to_wire(&m.base.outfit),
        light_level: 0,
        light_color: 0,
        step_speed: step_speed(m.base.speed),
        skull: skull_byte(m.base.skull),
        party_shield: 0,
        guild_emblem: 0,
        speech_bubble: 0,
        helpers: 0,
        walkthrough_blocked: 1,
        access_player: false,
    }
}

fn npc_to_add_creature_wire(cid: CreatureId, n: &Npc) -> AddCreatureWire {
    AddCreatureWire {
        id: non_player_wire_id(cid),
        remove_known: 0,
        known: false,
        creature_type: 2,
        name: n.base.name.clone(),
        health_percent: health_percent(n.base.health, n.base.max_health),
        direction: n.base.direction as u8,
        outfit: outfit_to_wire(&n.base.outfit),
        light_level: 0,
        light_color: 0,
        step_speed: step_speed(n.base.speed),
        skull: skull_byte(n.base.skull),
        party_shield: 0,
        guild_emblem: 0,
        speech_bubble: 0,
        helpers: 0,
        walkthrough_blocked: 1,
        access_player: false,
    }
}

/// One tile for `GetMapDescription` / move strips (`player_pos` = tile where the local player stands).
///
/// Map tiles hold **server** item ids; this resolves them with `ItemDatabase::client_id_for_server`
/// before building `TileContent` (every `ItemStack.client_id` is a client/sprite id on the wire).
pub(crate) fn map_tile_content(
    world: &GameWorld,
    self_cid: CreatureId,
    player_pos: Position,
    tx: i32,
    ty: i32,
    tz: i32,
) -> Option<TileContent> {
    if tx < 0 || ty < 0 || !(0..=15).contains(&tz) {
        return None;
    }
    let Some(CreatureKind::Player(self_player)) = world.creatures.get(self_cid) else {
        return None;
    };
    let viewer_access = world.player_is_access_player(self_cid);
    let self_light = world.player_creature_light(self_cid);
    let self_wire = player_to_add_creature_wire(self_player, true, self_light, viewer_access);
    let self_guid = self_player.guid;

    let px = player_pos.x as i32;
    let py = player_pos.y as i32;
    let pz = player_pos.z as i32;
    let pos = Position::new(tx as u16, ty as u16, tz as u8);
    let on_self = tx == px && ty == py && tz == pz;

    let mut content = TileContent::default();

    if let Some(tile) = world.map.get_tile(pos) {
        let body = tile.body();
        if let Some(gid) = body.ground {
            if gid != 0 {
                let cid = world.items_db.client_id_for_server(gid);
                if cid != 0 {
                    let stackable = world.items_db.stackable_for_server(gid);
                    let splash_fluid = world.items_db.is_splash_or_fluid_for_server(gid);
                    content.ground = Some(ItemStack {
                        client_id: cid,
                        count: 1,
                        stackable,
                        is_splash_or_fluid: splash_fluid && !stackable,
                        is_animation: world.items_db.is_animation_for_server(gid),
                    });
                }
            }
        }
        for &item_id in &body.top_items {
            // Get the actual item from world storage
            let Some(item) = world.items.get(item_id) else {
                continue;
            };
            let iid = item.item_type;
            if iid == 0 {
                continue;
            }
            let cid = world.items_db.client_id_for_server(iid);
            if cid == 0 {
                continue;
            }
            let stackable = world.items_db.stackable_for_server(iid);
            let splash_fluid = world.items_db.is_splash_or_fluid_for_server(iid);
            content.top_items.push(ItemStack {
                client_id: cid,
                count: item.client_count(),
                stackable,
                is_splash_or_fluid: splash_fluid && !stackable,
                is_animation: world.items_db.is_animation_for_server(iid),
            });
        }
        for &ocid in &body.creatures {
            if let Some(k) = world.creatures.get(ocid) {
                if let CreatureKind::Player(p) = k {
                    if p.ghost_mode && ocid != self_cid {
                        continue;
                    }
                    if p.base.active_conditions.iter().any(|c| c.ctype == ConditionType::Invisible)
                        && ocid != self_cid
                    {
                        continue;
                    }
                }
                if let CreatureKind::Monster(m) = k {
                    if m.base.active_conditions.iter().any(|c| c.ctype == ConditionType::Invisible)
                        && ocid != self_cid
                    {
                        continue;
                    }
                }
                if let CreatureKind::Npc(n) = k {
                    if n.base.active_conditions.iter().any(|c| c.ctype == ConditionType::Invisible)
                        && ocid != self_cid
                    {
                        continue;
                    }
                }
                let w = match k {
                    CreatureKind::Player(p) => {
                        let light = world.player_creature_light(ocid);
                        player_to_add_creature_wire(p, p.guid == self_guid, light, viewer_access)
                    }
                    CreatureKind::Monster(m) => monster_to_add_creature_wire(ocid, m),
                    CreatureKind::Npc(n) => npc_to_add_creature_wire(ocid, n),
                };
                content.creatures.push(w);
            }
        }
        for &item_id in &body.down_items {
            // Get the actual item from world storage
            let Some(item) = world.items.get(item_id) else {
                continue;
            };
            let iid = item.item_type;
            if iid == 0 {
                continue;
            }
            let cid = world.items_db.client_id_for_server(iid);
            if cid == 0 {
                continue;
            }
            let stackable = world.items_db.stackable_for_server(iid);
            let splash_fluid = world.items_db.is_splash_or_fluid_for_server(iid);
            content.bottom_items.push(ItemStack {
                client_id: cid,
                count: item.client_count(),
                stackable,
                is_splash_or_fluid: splash_fluid && !stackable,
                is_animation: world.items_db.is_animation_for_server(iid),
            });
        }
    }

    if on_self && !content.creatures.iter().any(|c| c.id == self_guid) {
        content.creatures.push(self_wire);
    }

    if content.ground.is_none()
        && content.top_items.is_empty()
        && content.creatures.is_empty()
        && content.bottom_items.is_empty()
    {
        return None;
    }
    Some(content)
}

/// Full `0x64` map around `center` from loaded OTBM tiles and creature indices.
fn build_initial_map_packet(
    world: &GameWorld,
    self_cid: CreatureId,
    center: Position,
    known: &mut HashSet<u32>,
) -> Vec<u8> {
    if world.creatures.get(self_cid).is_none() {
        return send_map_description_stub(center, center).into_bytes();
    }

    let with_description = matches!(
        world.creatures.get(self_cid),
        Some(CreatureKind::Player(p)) if p.item_with_description()
    );

    let mut get_tile = |tx: i32, ty: i32, tz: i32| -> Option<TileContent> {
        map_tile_content(world, self_cid, center, tx, ty, tz)
    };

    let mut can_see = |guid: u32| world.can_see_creature_for_known_set(self_cid, guid);

    send_map_description_packet(
        &world.codec,
        center,
        center,
        &mut get_tile,
        known,
        &mut can_see,
        with_description,
    )
    .into_bytes()
}

/// Enqueue initial packets for a freshly loaded player (`protocolgame.cpp` `sendAddCreature` self path).
/// C++ order: `0x17` → `0x0A` → `0x43` → `0x0F` → map → magic → inventory → stats → unjustified →
/// basic → skills → world light → creature light → VIP → basic → icons (then fight modes queued here).
pub fn enqueue_initial_login_packets(
    world: &mut GameWorld,
    conn_id: ConnId,
    creature_id: CreatureId,
) {
    let (
        pid,
        pos,
        skill_levels,
        skill_bases,
        skill_percents,
        vocation_id,
        premium_ends_at,
        equipment_slots,
        vip_list,
        with_desc_inv,
    ) = {
        let Some(CreatureKind::Player(p)) = world.creatures.get(creature_id) else {
            return;
        };
        let sk = &p.skills;
        let lvl = |v: i32| v.max(0).min(u16::MAX as i32) as u16;
        let skill_levels = [
            lvl(sk.fist),
            lvl(sk.club),
            lvl(sk.sword),
            lvl(sk.axe),
            lvl(sk.dist),
            lvl(sk.shielding),
            lvl(sk.fishing),
        ];
        let skill_bases = skill_levels;
        let skill_percents = [0u8; 7];
        let with_desc_inv = p.item_with_description();
        (
            p.guid,
            p.base.position,
            skill_levels,
            skill_bases,
            skill_percents,
            p.vocation_id,
            p.premium_ends_at,
            p.equipment_slots,
            p.vip_list.clone(),
            with_desc_inv,
        )
    };
    let voc_client = world.vocations.client_id_u8(vocation_id);
    // OTClient 1098 + GameAdditionalSkills: skills 7–12 (critical / leech); not modeled in PlayerSkills yet — zeros.
    let additional_skill_levels = [0u16; 6];
    let additional_skill_bases = [0u16; 6];

    let free_premium = world
        .config
        .get_bool("freePremium")
        .unwrap_or(false);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;
    let has_premium = free_premium || premium_ends_at > now;
    let premium_packet_ends = if has_premium && !free_premium {
        premium_ends_at
    } else {
        0
    };

    let mut known = world
        .known_creatures_by_conn
        .remove(&conn_id)
        .unwrap_or_default();
    world.reconcile_known_creatures_for_send(conn_id, &mut known);
    let map_bytes = build_initial_map_packet(world, creature_id, pos, &mut known);
    let map_0x64_len = map_bytes.len();
    world.commit_known_creatures_after_send(conn_id, &known);

    // `ProtocolGame::login` — OTCv8 (`src/protocolgame.cpp` ~168–178): `sendFeatures` + extended opcode init.
    world.enqueue_outgoing(
        conn_id,
        send_otcv8_features(&[
            (OTC_FEATURE_EXTENDED_OPCODE, true),
            (OTC_FEATURE_ITEM_TOOLTIP, true),
        ])
        .into_bytes(),
    );
    world.enqueue_outgoing(conn_id, send_extended_opcode(0, "").into_bytes());

    world.enqueue_encoded(conn_id, world.codec.encode_self_appear_login(pid));
    world.enqueue_outgoing(conn_id, send_pending_state_entered().into_bytes());
    world.enqueue_outgoing(conn_id, send_otc_features_raw().into_bytes());
    world.enqueue_outgoing(conn_id, send_enter_world().into_bytes());
    world.enqueue_outgoing(conn_id, map_bytes);

    // `CONST_ME_TELEPORT` — `src/const.h`
    world.enqueue_outgoing(conn_id, send_magic_effect(pos, 11).into_bytes());

    for slot in 1u8..=11 {
        let idx = (slot - 1) as usize;
        if let Some(item_id) = equipment_slots[idx] {
            let Some(item) = world.items.get(item_id) else {
                world.enqueue_outgoing(conn_id, send_inventory_slot_empty(slot).into_bytes());
                continue;
            };
            let sid = item.item_type;
            let cid = world.items_db.client_id_for_server(sid);
            if cid == 0 {
                world.enqueue_outgoing(conn_id, send_inventory_slot_empty(slot).into_bytes());
                continue;
            }
            let cnt = item.client_count().max(1);
            let stackable = world.items_db.stackable_for_server(sid);
            let splash = world.items_db.is_splash_or_fluid_for_server(sid);
            let anim = world.items_db.is_animation_for_server(sid);
            world.enqueue_encoded(
                conn_id,
                world.codec.encode_inventory_item(
                    slot,
                    ItemTemplateArgs {
                        client_id: cid,
                        count: cnt,
                        stackable,
                        is_splash_or_fluid: splash && !stackable,
                        is_animation: anim,
                        with_description: with_desc_inv,
                    },
                ),
            );
        } else {
            world.enqueue_outgoing(conn_id, send_inventory_slot_empty(slot).into_bytes());
        }
    }

    // Use the centralized helper which correctly computes level_percent, capacity (1/100 oz),
    // offline training time (ms→minutes), etc. — matching C++ `Player::sendStats` (`player.cpp` ~882).
    world.send_player_stats(creature_id);
    world.enqueue_outgoing(conn_id, send_unjustified_stats_stub().into_bytes());
    world.enqueue_encoded(
        conn_id,
        world
            .codec
            .encode_basic_data(has_premium, premium_packet_ends, voc_client),
    );
    world.enqueue_encoded(
        conn_id,
        world.codec.encode_player_skills(&PlayerSkillsWire {
            levels: skill_levels,
            bases: skill_bases,
            percents: skill_percents,
            additional_levels: additional_skill_levels,
            additional_bases: additional_skill_bases,
        }),
    );

    let wt = crate::world_light::world_time_from_local_clock();
    let wl = crate::world_light::light_level_from_world_time(wt);
    world.enqueue_outgoing(conn_id, send_world_light(wl, 215, false).into_bytes());
    let pl = world.player_creature_light(creature_id);
    world.enqueue_encoded(
        conn_id,
        world.codec.encode_creature_light(pid, pl.level, pl.color, false),
    );
    for e in &vip_list {
        let online = world.player_by_guid.contains_key(&e.player_id);
        let status = if online { 1 } else { 0 };
        world.enqueue_outgoing(
            conn_id,
            send_vip_entry(
                e.player_id,
                &e.name,
                &e.description,
                e.icon,
                e.notify,
                status,
            )
            .into_bytes(),
        );
    }
    world.enqueue_encoded(
        conn_id,
        world
            .codec
            .encode_basic_data(has_premium, premium_packet_ends, voc_client),
    );
    world.enqueue_outgoing(conn_id, send_icons(0).into_bytes());
    world.enqueue_outgoing(conn_id, send_fight_modes(1, 0, 0, 0).into_bytes());

    world.auto_open_containers_on_login(conn_id, creature_id);

    if map_0x64_len < 32 {
        warn!(
            conn_id = conn_id.0,
            player_id = pid,
            ?pos,
            map_0x64_len,
            "initial 0x64 map packet is very small — possible stub (creature/world not ready)"
        );
    }
}

#[cfg(test)]
mod map_creature_wire_tests {
    use super::*;
    use crate::creature::LightInfo;
    use crate::test_world::support::test_player;
    use tfs_rust_common::Position;

    #[test]
    fn self_map_wire_uses_creature_light_not_is_self_access() {
        let p = test_player("Test", Position::new(100, 100, 7));
        let light = LightInfo {
            level: 7,
            color: 215,
        };
        let wire = player_to_add_creature_wire(&p, true, light, false);
        assert!(!wire.access_player);
        assert_eq!(wire.light_level, 7);
        assert_eq!(wire.light_color, 215);
    }

    #[test]
    fn gm_viewer_uses_access_player_wire_flag() {
        let p = test_player("GM", Position::new(100, 100, 7));
        let wire = player_to_add_creature_wire(&p, true, LightInfo::default(), true);
        assert!(wire.access_player);
    }
}
