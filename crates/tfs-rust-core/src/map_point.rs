//! One-pass `SendMapPoint` into the outbound message.
//!
//! Corpus: `sending.cc` `SendFullScreen` / `SendMapPoint` — walk the tile, skip-run empties,
//! write objects. Creature names go into one reused [`AddCreatureWire`].
//! Pack surface: `GetTileDescription` order (ground, Cip bottom/top/creature/low, 10-thing cap).

use tfs_rust_common::Position;
use tfs_rust_common::enums::SkullType;
use tfs_rust_net::NetworkMessage;
use tfs_rust_net::ProtocolCodec;
use tfs_rust_net::creature_encode::AddCreatureWire;
use tfs_rust_net::creature_known::{KnownCreatureTable, check_creature_known};
use tfs_rust_net::map_description::{
    ItemStack, MapTilePass, TileContent, send_map_description_packet_direct,
    send_map_description_packet_fill,
};

use crate::creature::CreatureKind;
use crate::creature::LightInfo;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::login_out::{
    MapDescribeCtx, creature_hidden_from_map, fill_monster_wire, fill_npc_wire, fill_player_wire,
    item_stack_from_server_id, map_tile_content_into,
};

struct MapPointBuf {
    ground: Option<ItemStack>,
    top: Vec<ItemStack>,
    bottom: Vec<ItemStack>,
    low: Vec<ItemStack>,
    creatures: Vec<CreatureId>,
    inject_self: bool,
    wire: AddCreatureWire,
}

impl MapPointBuf {
    fn clear_lists(&mut self) {
        self.ground = None;
        self.top.clear();
        self.bottom.clear();
        self.low.clear();
        self.creatures.clear();
        self.inject_self = false;
    }

    fn is_blank(&self) -> bool {
        self.ground.is_none()
            && self.top.is_empty()
            && self.bottom.is_empty()
            && self.low.is_empty()
            && self.creatures.is_empty()
            && !self.inject_self
    }
}

impl Default for MapPointBuf {
    fn default() -> Self {
        Self {
            ground: None,
            top: Vec::new(),
            bottom: Vec::new(),
            low: Vec::new(),
            creatures: Vec::new(),
            inject_self: false,
            wire: AddCreatureWire::default(),
        }
    }
}

/// Full `0x64` for `ctx.player_pos`. Bytes match [`send_map_description_packet_fill`].
pub(crate) fn encode_fullscreen(
    world: &GameWorld,
    ctx: &MapDescribeCtx,
    known: &mut KnownCreatureTable,
    with_description: bool,
) -> Vec<u8> {
    #[cfg(debug_assertions)]
    let known_before = known.clone();

    let mut buf = MapPointBuf::default();
    let mut can_see = |id: u32| world.can_see_creature_for_known_set(ctx.self_cid, id);
    let mut tile =
        |tx: i32, ty: i32, tz: i32, pass: MapTilePass, msg: &mut NetworkMessage| match pass {
            MapTilePass::Occupied => gather_map_point(world, ctx, &mut buf, tx, ty, tz),
            MapTilePass::Write => {
                write_map_point(world, ctx, &mut buf, msg, known, &mut can_see);
                true
            }
        };
    let msg = send_map_description_packet_direct(ctx.player_pos, ctx.player_pos, &mut tile);

    #[cfg(debug_assertions)]
    {
        let mut known_fill = known_before;
        let mut can_see_fill = |id: u32| world.can_see_creature_for_known_set(ctx.self_cid, id);
        let mut get_tile = |tx: i32, ty: i32, tz: i32, out: &mut TileContent| {
            map_tile_content_into(world, ctx, tx, ty, tz, out)
        };
        let fill = send_map_description_packet_fill(
            &world.codec,
            ctx.player_pos,
            ctx.player_pos,
            &mut get_tile,
            &mut known_fill,
            &mut can_see_fill,
            with_description,
        );
        debug_assert_eq!(
            msg.as_bytes(),
            fill.as_bytes(),
            "direct 0x64 drifted from fill encode"
        );
        debug_assert_eq!(known, &known_fill, "direct 0x64 known-set drifted");
    }
    #[cfg(not(debug_assertions))]
    let _ = with_description;

    msg.into_bytes()
}

fn gather_map_point(
    world: &GameWorld,
    ctx: &MapDescribeCtx,
    buf: &mut MapPointBuf,
    tx: i32,
    ty: i32,
    tz: i32,
) -> bool {
    buf.clear_lists();
    if tx < 0 || ty < 0 || !(0..=15).contains(&tz) {
        return false;
    }

    let px = i32::from(ctx.player_pos.x);
    let py = i32::from(ctx.player_pos.y);
    let pz = i32::from(ctx.player_pos.z);
    let on_self = tx == px && ty == py && tz == pz;
    let pos = Position::new(tx as u16, ty as u16, tz as u8);

    if let Some(tile) = world.map.get_tile(pos) {
        let body = tile.body();
        if let Some(gid) = body.ground
            && gid != 0
        {
            buf.ground = item_stack_from_server_id(world, gid, 1);
        }
        for &item_id in body.top_items() {
            let Some(item) = world.items.get(item_id) else {
                continue;
            };
            let Some(stack) = item_stack_from_server_id(world, item.item_type, item.client_count())
            else {
                continue;
            };
            buf.top.push(stack);
        }
        for &ocid in body.creatures() {
            let skip = match world.creatures.get(ocid) {
                Some(kind) => creature_hidden_from_map(kind, ocid, ctx.self_cid),
                None => true,
            };
            if !skip {
                buf.creatures.push(ocid);
            }
        }
        for &item_id in body.down_items() {
            let Some(item) = world.items.get(item_id) else {
                continue;
            };
            let iid = item.item_type;
            if iid == 0 {
                continue;
            }
            let Some(itype) = world.items_db.items.get(&iid) else {
                continue;
            };
            let cid = itype.client_id;
            if cid == 0 {
                continue;
            }
            let stackable = itype.stackable();
            let splash_fluid = (itype.is_splash() || itype.is_fluid_container()) && !stackable;
            let stack = ItemStack {
                client_id: cid,
                count: item.client_count(),
                stackable,
                is_splash_or_fluid: splash_fluid,
                is_animation: itype.is_animation(),
            };
            if ctx.cip_map_order && itype.is_cip_priority_bottom() {
                buf.bottom.push(stack);
            } else if ctx.cip_map_order {
                buf.low.push(stack);
            } else {
                buf.bottom.push(stack);
            }
        }
    }

    if on_self && !buf.creatures.contains(&ctx.self_cid) {
        buf.inject_self = true;
    }
    !buf.is_blank()
}

fn write_map_point(
    world: &GameWorld,
    ctx: &MapDescribeCtx,
    buf: &mut MapPointBuf,
    msg: &mut NetworkMessage,
    known: &mut KnownCreatureTable,
    can_see: &mut impl FnMut(u32) -> bool,
) {
    let codec = &world.codec;
    codec.write_tile_environment_prefix(msg);

    let mut count: i32 = if buf.ground.is_some() { 1 } else { 0 };
    if let Some(ground) = buf.ground.as_ref() {
        if ground.client_id != 0 {
            write_stack(codec, msg, ground);
        } else {
            count = 0;
        }
    }

    if ctx.cip_map_order {
        for it in buf.bottom.iter().rev() {
            if it.client_id == 0 || count == 10 {
                continue;
            }
            write_stack(codec, msg, it);
            count += 1;
        }
        for it in &buf.top {
            if it.client_id == 0 || count == 10 {
                continue;
            }
            write_stack(codec, msg, it);
            count += 1;
        }
        write_creatures(world, ctx, buf, msg, known, can_see, &mut count);
        if count < 10 {
            for it in &buf.low {
                if count == 10 {
                    return;
                }
                if it.client_id == 0 {
                    continue;
                }
                write_stack(codec, msg, it);
                count += 1;
            }
        }
    } else {
        for it in &buf.top {
            if it.client_id == 0 || count == 10 {
                continue;
            }
            write_stack(codec, msg, it);
            count += 1;
        }
        write_creatures(world, ctx, buf, msg, known, can_see, &mut count);
        if count < 10 {
            for it in buf.bottom.iter().chain(buf.low.iter()) {
                if count == 10 {
                    return;
                }
                if it.client_id == 0 {
                    continue;
                }
                write_stack(codec, msg, it);
                count += 1;
            }
        }
    }
}

fn write_creatures(
    world: &GameWorld,
    ctx: &MapDescribeCtx,
    buf: &mut MapPointBuf,
    msg: &mut NetworkMessage,
    known: &mut KnownCreatureTable,
    can_see: &mut impl FnMut(u32) -> bool,
    count: &mut i32,
) {
    let caps = world.codec.tile_description_caps_creatures();
    if buf.inject_self {
        if caps && *count == 10 {
            return;
        }
        buf.wire.copy_from(&ctx.self_wire);
        emit_wire(world, buf, msg, known, can_see);
        *count += 1;
    }
    for i in (0..buf.creatures.len()).rev() {
        let ocid = buf.creatures[i];
        if caps && *count == 10 {
            return;
        }
        if !fill_creature_wire(world, ctx, &mut buf.wire, ocid) {
            continue;
        }
        emit_wire(world, buf, msg, known, can_see);
        *count += 1;
    }
}

fn fill_creature_wire(
    world: &GameWorld,
    ctx: &MapDescribeCtx,
    wire: &mut AddCreatureWire,
    ocid: CreatureId,
) -> bool {
    if ocid == ctx.self_cid {
        wire.copy_from(&ctx.self_wire);
        return true;
    }
    let skull = match world.creatures.get(ocid) {
        Some(CreatureKind::Player(_)) => world.player_get_killing_mark(ocid, ctx.self_cid),
        _ => SkullType::None,
    };
    let party_shield = match world.creatures.get(ocid) {
        Some(CreatureKind::Player(_)) => world.player_get_party_mark(ocid, ctx.self_cid),
        _ => 0,
    };
    let light = match world.creatures.get(ocid) {
        Some(CreatureKind::Player(_)) => world.player_creature_light(ocid),
        _ => LightInfo::default(),
    };
    match world.creatures.get(ocid) {
        Some(CreatureKind::Player(p)) => fill_player_wire(
            wire,
            p,
            p.guid == ctx.self_guid,
            light,
            ctx.viewer_access,
            &world.mechanics,
            skull,
            party_shield,
        ),
        Some(CreatureKind::Monster(m)) => fill_monster_wire(wire, ocid, m, &world.mechanics),
        Some(CreatureKind::Npc(n)) => fill_npc_wire(wire, ocid, n, &world.mechanics),
        None => return false,
    }
    true
}

fn emit_wire(
    world: &GameWorld,
    buf: &mut MapPointBuf,
    msg: &mut NetworkMessage,
    known: &mut KnownCreatureTable,
    can_see: &mut impl FnMut(u32) -> bool,
) {
    let limit = world.codec.caps().known_creature_limit as usize;
    let id = buf.wire.id;
    let (already, remove) = check_creature_known(id, known, can_see, limit);
    buf.wire.apply_known_check(already, remove);
    world.codec.write_add_creature(msg, &buf.wire);
}

fn write_stack(codec: &tfs_rust_net::Codec, msg: &mut NetworkMessage, it: &ItemStack) {
    codec.write_item_template(
        msg,
        it.client_id,
        it.count,
        it.stackable,
        it.is_splash_or_fluid,
        it.is_animation,
        false,
    );
}
