//! `GetMapDescription` / `GetFloorDescription` / `GetTileDescription`.
// C++ reference (this repo): `src/protocolgame.cpp`.

use std::collections::HashSet;

use tfs_rust_common::protocol_constants::{
    client_viewport_height, client_viewport_width, MAP_MAX_LAYERS, MAX_CLIENT_VIEWPORT_X,
    MAX_CLIENT_VIEWPORT_Y,
};
use tfs_rust_common::Position;

use crate::creature_encode::{add_creature_wire_len, write_add_creature, AddCreatureWire};
use crate::item_encode::{item_template_wire_len, write_item_template};
use crate::NetworkMessage;

/// Stackable or single item for template encoding (`NetworkMessage::addItem`).
#[derive(Debug, Clone)]
pub struct ItemStack {
    pub client_id: u16,
    pub count: u8,
    pub stackable: bool,
    /// Splash / fluid container — `fluidMap[count & 7]` when not stackable (`src/networkmessage.cpp`).
    pub is_splash_or_fluid: bool,
    /// OTB `FLAG_ANIMATION` — `0xFE` before duration (`src/networkmessage.cpp`).
    pub is_animation: bool,
}

/// One tile’s worth of things for protocol encoding.
#[derive(Debug, Clone, Default)]
pub struct TileContent {
    pub ground: Option<ItemStack>,
    pub top_items: Vec<ItemStack>,
    pub bottom_items: Vec<ItemStack>,
    /// Bottom-to-top creature order as stored; emitted in **reverse** (C++ `reverse(creatures)`).
    pub creatures: Vec<AddCreatureWire>,
}

/// C++ `ProtocolGame::checkCreatureAsKnown` (`src/protocolgame.cpp` ~744–776).
fn check_creature_known<F: FnMut(u32) -> bool>(
    id: u32,
    known_set: &mut HashSet<u32>,
    can_see_creature: &mut F,
) -> (bool, u32) {
    if !known_set.insert(id) {
        return (true, 0);
    }

    if known_set.len() <= 1300 {
        return (false, 0);
    }

    let mut others: Vec<u32> = known_set.iter().copied().filter(|&k| k != id).collect();
    others.sort_unstable();

    for cid in &others {
        if !can_see_creature(*cid) {
            known_set.remove(cid);
            return (false, *cid);
        }
    }

    if let Some(first) = others.first() {
        let removed = *first;
        known_set.remove(&removed);
        return (false, removed);
    }

    (false, 0)
}

fn get_tile_description<F: FnMut(u32) -> bool>(
    msg: &mut NetworkMessage,
    tile: &TileContent,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
    _with_description: bool,
) {
    msg.write_u16(0);

    let mut count: i32 = if tile.ground.is_some() {
        1
    } else {
        0
    };

    if let Some(ref g) = tile.ground {
        if g.client_id != 0 {
            write_item_template(
                msg,
                g.client_id,
                g.count,
                g.stackable,
                g.is_splash_or_fluid,
                g.is_animation,
                false,
            );
        } else {
            count = 0;
        }
    }

    for it in &tile.top_items {
        if it.client_id == 0 || count == 10 {
            continue;
        }
        write_item_template(
            msg,
            it.client_id,
            it.count,
            it.stackable,
            it.is_splash_or_fluid,
            it.is_animation,
            false,
        );
        count += 1;
    }

    for c in tile.creatures.iter().rev() {
        let id = c.id;
        let (known, remove) = check_creature_known(id, known_creatures, can_see_creature);
        let mut cw = c.clone();
        cw.known = known;
        cw.remove_known = remove;
        write_add_creature(msg, &cw);
        count += 1;
    }

    if count < 10 {
        for it in &tile.bottom_items {
            if count == 10 {
                return;
            }
            if it.client_id == 0 {
                continue;
            }
            write_item_template(
                msg,
                it.client_id,
                it.count,
                it.stackable,
                it.is_splash_or_fluid,
                it.is_animation,
                false,
            );
            count += 1;
        }
    }
}

/// Independent byte count for [`get_tile_description`].
///
/// Map tiles always use `write_item_template(..., false)` — C++ `GetTileDescription` does not pass
/// OTCv8 description for template map items (`src/protocolgame.cpp`). Must match that, not the
/// outer `with_description` used elsewhere.
fn count_tile_description<F: FnMut(u32) -> bool>(
    tile: &TileContent,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
) -> usize {
    let mut n = 2; // environmental effects u16

    let mut count: i32 = if tile.ground.is_some() {
        1
    } else {
        0
    };

    if let Some(ref g) = tile.ground {
        if g.client_id != 0 {
            n += item_template_wire_len(
                g.client_id,
                g.count,
                g.stackable,
                g.is_splash_or_fluid,
                g.is_animation,
                false,
            );
        } else {
            count = 0;
        }
    }

    for it in &tile.top_items {
        if it.client_id == 0 || count == 10 {
            continue;
        }
        n += item_template_wire_len(
            it.client_id,
            it.count,
            it.stackable,
            it.is_splash_or_fluid,
            it.is_animation,
            false,
        );
        count += 1;
    }

    for c in tile.creatures.iter().rev() {
        let id = c.id;
        let (known, remove) = check_creature_known(id, known_creatures, can_see_creature);
        let mut cw = c.clone();
        cw.known = known;
        cw.remove_known = remove;
        n += add_creature_wire_len(&cw);
        count += 1;
    }

    if count < 10 {
        for it in &tile.bottom_items {
            if count == 10 {
                break;
            }
            if it.client_id == 0 {
                continue;
            }
            n += item_template_wire_len(
                it.client_id,
                it.count,
                it.stackable,
                it.is_splash_or_fluid,
                it.is_animation,
                false,
            );
            count += 1;
        }
    }
    n
}

#[allow(clippy::too_many_arguments)]
fn get_floor_description<F: FnMut(u32) -> bool>(
    msg: &mut NetworkMessage,
    x: i32,
    y: i32,
    z: i32,
    width: i32,
    height: i32,
    offset: i32,
    skip: &mut i32,
    get_tile: &mut impl FnMut(i32, i32, i32) -> Option<TileContent>,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
    with_description: bool,
) {
    for nx in 0..width {
        for ny in 0..height {
            let tx = x + nx + offset;
            let ty = y + ny + offset;
            if let Some(tile) = get_tile(tx, ty, z) {
                if *skip >= 0 {
                    msg.write_u8(*skip as u8);
                    msg.write_u8(0xFF);
                }
                *skip = 0;
                get_tile_description(
                    msg,
                    &tile,
                    known_creatures,
                    can_see_creature,
                    with_description,
                );
            } else if *skip == 0xFE {
                msg.write_u8(0xFF);
                msg.write_u8(0xFF);
                *skip = -1;
            } else {
                *skip += 1;
            }
        }
    }
}

/// Byte count for [`get_floor_description`] (must match skip + tile bytes).
#[allow(clippy::too_many_arguments)]
fn count_floor_description<F: FnMut(u32) -> bool>(
    x: i32,
    y: i32,
    z: i32,
    width: i32,
    height: i32,
    offset: i32,
    skip: &mut i32,
    get_tile: &mut impl FnMut(i32, i32, i32) -> Option<TileContent>,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
) -> usize {
    let mut n = 0usize;
    for nx in 0..width {
        for ny in 0..height {
            let tx = x + nx + offset;
            let ty = y + ny + offset;
            if let Some(tile) = get_tile(tx, ty, z) {
                if *skip >= 0 {
                    n += 1 + 1;
                }
                *skip = 0;
                n += count_tile_description(
                    &tile,
                    known_creatures,
                    can_see_creature,
                );
            } else if *skip == 0xFE {
                n += 1 + 1;
                *skip = -1;
            } else {
                *skip += 1;
            }
        }
    }
    n
}

/// Total bytes for [`write_map_description_body`] (opcode `0x64` **not** included).
///
/// Requires the same **`get_tile`** determinism as the write pass (typically pure lookups).
#[allow(clippy::too_many_arguments)]
pub fn count_map_description_body<F: FnMut(u32) -> bool>(
    origin_x: i32,
    origin_y: i32,
    origin_z: i32,
    width: i32,
    height: i32,
    get_tile: &mut impl FnMut(i32, i32, i32) -> Option<TileContent>,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
) -> usize {
    let mut skip = -1_i32;
    let (startz, endz, zstep) = if origin_z > 7 {
        let startz = origin_z - 2;
        let endz = (MAP_MAX_LAYERS - 1).min(origin_z + 2);
        (startz, endz, 1)
    } else {
        (7_i32, 0_i32, -1)
    };

    let mut n = 0usize;
    let mut nz = startz;
    loop {
        n += count_floor_description(
            origin_x,
            origin_y,
            nz,
            width,
            height,
            origin_z - nz,
            &mut skip,
            get_tile,
            known_creatures,
            can_see_creature,
        );
        if nz == endz {
            break;
        }
        nz += zstep;
    }

    if skip >= 0 {
        n += 1 + 1;
    }
    n
}

/// `ProtocolGame::GetMapDescription` into `msg` (does not prefix opcode — use [`send_map_description_packet`] for full packet).
#[allow(clippy::too_many_arguments)]
pub fn write_map_description_body<F: FnMut(u32) -> bool>(
    msg: &mut NetworkMessage,
    origin_x: i32,
    origin_y: i32,
    origin_z: i32,
    width: i32,
    height: i32,
    get_tile: &mut impl FnMut(i32, i32, i32) -> Option<TileContent>,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
    with_description: bool,
) {
    let mut skip = -1_i32;
    let (startz, endz, zstep) = if origin_z > 7 {
        let startz = origin_z - 2;
        let endz = (MAP_MAX_LAYERS - 1).min(origin_z + 2);
        (startz, endz, 1)
    } else {
        (7_i32, 0_i32, -1)
    };

    let mut nz = startz;
    loop {
        get_floor_description(
            msg,
            origin_x,
            origin_y,
            nz,
            width,
            height,
            origin_z - nz,
            &mut skip,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
        if nz == endz {
            break;
        }
        nz += zstep;
    }

    if skip >= 0 {
        msg.write_u8(skip as u8);
        msg.write_u8(0xFF);
    }
}

/// Full `sendMapDescription`: opcode `0x64`, player position, then map body (`GetMapDescription`).
// C++ reference: `sendMapDescription` — `msg.addByte(0x64); msg.addPosition(player->getPosition()); GetMapDescription(...)`.
pub fn send_map_description_packet<F: FnMut(u32) -> bool>(
    player_pos: Position,
    center: Position,
    get_tile: &mut impl FnMut(i32, i32, i32) -> Option<TileContent>,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
    with_description: bool,
) -> NetworkMessage {
    let origin_x = center.x as i32 - MAX_CLIENT_VIEWPORT_X;
    let origin_y = center.y as i32 - MAX_CLIENT_VIEWPORT_Y;
    let origin_z = center.z as i32;
    let w = client_viewport_width();
    let h = client_viewport_height();

    #[cfg(debug_assertions)]
    let (expected_body, kc_after_count) = {
        let mut kc = known_creatures.clone();
        let body = count_map_description_body(
            origin_x,
            origin_y,
            origin_z,
            w,
            h,
            get_tile,
            &mut kc,
            can_see_creature,
        );
        (body, kc)
    };

    let mut msg = NetworkMessage::new();
    msg.write_u8(0x64);
    msg.write_position(&player_pos);

    write_map_description_body(
        &mut msg,
        origin_x,
        origin_y,
        origin_z,
        w,
        h,
        get_tile,
        known_creatures,
        can_see_creature,
        with_description,
    );

    #[cfg(debug_assertions)]
    {
        const MAP_HEADER: usize = 1 + 2 + 2 + 1; // opcode 0x64 + position (x,y,z)
        debug_assert_eq!(
            msg.as_bytes().len(),
            MAP_HEADER + expected_body,
            "0x64 map body: encoded length must match count_map_description_body (off-by-one / drift)"
        );
        debug_assert_eq!(
            *known_creatures, kc_after_count,
            "known_creatures after encode must match count pass (ordering / mutation drift)"
        );
    }

    msg
}

/// `ProtocolGame::MoveUpCreature` (`src/protocolgame.cpp` ~3363–3404).
fn append_move_up_creature<F: FnMut(u32) -> bool>(
    msg: &mut NetworkMessage,
    old_pos: Position,
    new_pos: Position,
    get_tile: &mut impl FnMut(i32, i32, i32) -> Option<TileContent>,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
    with_description: bool,
) {
    let ox = old_pos.x as i32 - MAX_CLIENT_VIEWPORT_X;
    let oy = old_pos.y as i32 - MAX_CLIENT_VIEWPORT_Y;
    let nz = new_pos.z as i32;
    let old_z = old_pos.z as i32;
    let vw = client_viewport_width();
    let vh = client_viewport_height();

    msg.write_u8(0xBE);

    if nz == 7 {
        let mut skip = -1_i32;
        for i in (0..=5).rev() {
            get_floor_description(
                msg,
                ox,
                oy,
                i,
                vw,
                vh,
                8 - i,
                &mut skip,
                get_tile,
                known_creatures,
                can_see_creature,
                with_description,
            );
        }
        if skip >= 0 {
            msg.write_u8(skip as u8);
            msg.write_u8(0xFF);
        }
    } else if nz > 7 {
        let mut skip = -1_i32;
        get_floor_description(
            msg,
            ox,
            oy,
            old_z - 3,
            vw,
            vh,
            3,
            &mut skip,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
        if skip >= 0 {
            msg.write_u8(skip as u8);
            msg.write_u8(0xFF);
        }
    }

    msg.write_u8(0x68);
    write_map_description_body(
        msg,
        ox,
        old_pos.y as i32 - (MAX_CLIENT_VIEWPORT_Y - 1),
        nz,
        1,
        vh,
        get_tile,
        known_creatures,
        can_see_creature,
        with_description,
    );

    msg.write_u8(0x65);
    write_map_description_body(
        msg,
        ox,
        oy,
        nz,
        vw,
        1,
        get_tile,
        known_creatures,
        can_see_creature,
        with_description,
    );
}

/// `ProtocolGame::MoveDownCreature` (`src/protocolgame.cpp` ~3406–3446).
fn append_move_down_creature<F: FnMut(u32) -> bool>(
    msg: &mut NetworkMessage,
    old_pos: Position,
    new_pos: Position,
    get_tile: &mut impl FnMut(i32, i32, i32) -> Option<TileContent>,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
    with_description: bool,
) {
    let ox = old_pos.x as i32 - MAX_CLIENT_VIEWPORT_X;
    let oy = old_pos.y as i32 - MAX_CLIENT_VIEWPORT_Y;
    let nz = new_pos.z as i32;
    let old_z = old_pos.z as i32;
    let vw = client_viewport_width();
    let vh = client_viewport_height();

    msg.write_u8(0xBF);

    if nz == 8 {
        let mut skip = -1_i32;
        for i in 0..3 {
            get_floor_description(
                msg,
                ox,
                oy,
                nz + i,
                vw,
                vh,
                -i - 1,
                &mut skip,
                get_tile,
                known_creatures,
                can_see_creature,
                with_description,
            );
        }
        if skip >= 0 {
            msg.write_u8(skip as u8);
            msg.write_u8(0xFF);
        }
    } else if nz > old_z && nz > 8 && nz < 14 {
        let mut skip = -1_i32;
        get_floor_description(
            msg,
            ox,
            oy,
            nz + 2,
            vw,
            vh,
            -3,
            &mut skip,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
        if skip >= 0 {
            msg.write_u8(skip as u8);
            msg.write_u8(0xFF);
        }
    }

    msg.write_u8(0x66);
    write_map_description_body(
        msg,
        old_pos.x as i32 + (MAX_CLIENT_VIEWPORT_X + 1),
        old_pos.y as i32 - (MAX_CLIENT_VIEWPORT_Y + 1),
        nz,
        1,
        vh,
        get_tile,
        known_creatures,
        can_see_creature,
        with_description,
    );

    msg.write_u8(0x67);
    write_map_description_body(
        msg,
        ox,
        old_pos.y as i32 + (MAX_CLIENT_VIEWPORT_Y + 1),
        nz,
        vw,
        1,
        get_tile,
        known_creatures,
        can_see_creature,
        with_description,
    );
}

/// Local player walk: `ProtocolGame::sendMoveCreature` when `creature == player` and not teleport.
// C++ reference: `src/protocolgame.cpp` `ProtocolGame::sendMoveCreature` (lines ~2827–2870).
pub fn send_move_creature_player<F: FnMut(u32) -> bool>(
    old_pos: Position,
    new_pos: Position,
    old_stack_pos: i32,
    creature_id: u32,
    get_tile: &mut impl FnMut(i32, i32, i32) -> Option<TileContent>,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
    with_description: bool,
) -> NetworkMessage {
    if old_pos.z != new_pos.z {
        let mut msg = NetworkMessage::new();
        if old_pos.z == 7 && new_pos.z >= 8 {
            if old_stack_pos >= 0 && old_stack_pos < 10 {
                msg.write_u8(0x6C);
                msg.write_position(&old_pos);
                msg.write_u8(old_stack_pos as u8);
            } else {
                msg.write_u8(0x6C);
                msg.write_u16(0xFFFF);
                msg.write_u32(creature_id);
            }
        } else {
            msg.write_u8(0x6D);
            if old_stack_pos >= 0 && old_stack_pos < 10 {
                msg.write_position(&old_pos);
                msg.write_u8(old_stack_pos as u8);
            } else {
                msg.write_u16(0xFFFF);
                msg.write_u32(creature_id);
            }
            msg.write_position(&new_pos);
        }

        if new_pos.z > old_pos.z {
            append_move_down_creature(
                &mut msg,
                old_pos,
                new_pos,
                get_tile,
                known_creatures,
                can_see_creature,
                with_description,
            );
        } else if new_pos.z < old_pos.z {
            append_move_up_creature(
                &mut msg,
                old_pos,
                new_pos,
                get_tile,
                known_creatures,
                can_see_creature,
                with_description,
            );
        }

        let w = client_viewport_width();
        let h = client_viewport_height();
        let ox = old_pos.x as i32;
        let oy = old_pos.y as i32;
        let nx = new_pos.x as i32;
        let ny = new_pos.y as i32;
        let nz = new_pos.z as i32;

        if oy > ny {
            msg.write_u8(0x65);
            write_map_description_body(
                &mut msg,
                ox - MAX_CLIENT_VIEWPORT_X,
                ny - MAX_CLIENT_VIEWPORT_Y,
                nz,
                w,
                1,
                get_tile,
                known_creatures,
                can_see_creature,
                with_description,
            );
        } else if oy < ny {
            msg.write_u8(0x67);
            write_map_description_body(
                &mut msg,
                ox - MAX_CLIENT_VIEWPORT_X,
                ny + (MAX_CLIENT_VIEWPORT_Y + 1),
                nz,
                w,
                1,
                get_tile,
                known_creatures,
                can_see_creature,
                with_description,
            );
        }

        if ox < nx {
            msg.write_u8(0x66);
            write_map_description_body(
                &mut msg,
                nx + (MAX_CLIENT_VIEWPORT_X + 1),
                ny - MAX_CLIENT_VIEWPORT_Y,
                nz,
                1,
                h,
                get_tile,
                known_creatures,
                can_see_creature,
                with_description,
            );
        } else if ox > nx {
            msg.write_u8(0x68);
            write_map_description_body(
                &mut msg,
                nx - MAX_CLIENT_VIEWPORT_X,
                ny - MAX_CLIENT_VIEWPORT_Y,
                nz,
                1,
                h,
                get_tile,
                known_creatures,
                can_see_creature,
                with_description,
            );
        }

        return msg;
    }

    let mut msg = NetworkMessage::new();
    msg.write_u8(0x6D);
    if old_stack_pos >= 0 && old_stack_pos < 10 {
        msg.write_position(&old_pos);
        msg.write_u8(old_stack_pos as u8);
    } else {
        msg.write_u16(0xFFFF);
        msg.write_u32(creature_id);
    }
    msg.write_position(&new_pos);

    let w = client_viewport_width();
    let h = client_viewport_height();
    let ox = old_pos.x as i32;
    let oy = old_pos.y as i32;
    let nx = new_pos.x as i32;
    let ny = new_pos.y as i32;
    let nz = new_pos.z as i32;

    if oy > ny {
        msg.write_u8(0x65);
        write_map_description_body(
            &mut msg,
            ox - MAX_CLIENT_VIEWPORT_X,
            ny - MAX_CLIENT_VIEWPORT_Y,
            nz,
            w,
            1,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
    } else if oy < ny {
        msg.write_u8(0x67);
        write_map_description_body(
            &mut msg,
            ox - MAX_CLIENT_VIEWPORT_X,
            ny + (MAX_CLIENT_VIEWPORT_Y + 1),
            nz,
            w,
            1,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
    }

    if ox < nx {
        msg.write_u8(0x66);
        write_map_description_body(
            &mut msg,
            nx + (MAX_CLIENT_VIEWPORT_X + 1),
            ny - MAX_CLIENT_VIEWPORT_Y,
            nz,
            1,
            h,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
    } else if ox > nx {
        msg.write_u8(0x68);
        write_map_description_body(
            &mut msg,
            nx - MAX_CLIENT_VIEWPORT_X,
            ny - MAX_CLIENT_VIEWPORT_Y,
            nz,
            1,
            h,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
    }

    msg
}

/// `ProtocolGame::sendUpdateTile` (`src/protocolgame.cpp` ~2683).
pub fn send_update_tile<F: FnMut(u32) -> bool>(
    pos: Position,
    tile: Option<&TileContent>,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
    with_description: bool,
) -> NetworkMessage {
    let mut msg = NetworkMessage::new();
    msg.write_u8(0x69);
    msg.write_position(&pos);
    if let Some(t) = tile {
        get_tile_description(
            &mut msg,
            t,
            known_creatures,
            can_see_creature,
            with_description,
        );
        msg.write_u8(0x00);
        msg.write_u8(0xFF);
    } else {
        msg.write_u8(0x01);
        msg.write_u8(0xFF);
    }
    msg
}

/// Backwards-compatible stub (empty viewport end marker only) — tests / smoke.
pub fn send_map_description_stub(player_pos: Position, _view_center: Position) -> NetworkMessage {
    let mut msg = NetworkMessage::new();
    msg.write_u8(0x64);
    msg.write_position(&player_pos);
    msg.write_u8(0xFF);
    msg.write_u8(0xFF);
    msg
}
