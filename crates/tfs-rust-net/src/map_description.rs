//! `GetMapDescription` / `GetFloorDescription` / `GetTileDescription`.
//!
//! 772 NotifyGo non-adjacent: `cract.cc` else → `SendFullScreen` (`sending.cc`); adjacent
//! floors/rows stay `0x6C`/`0x6D` then `0xBE`/`0xBF` + rows.
// C++ reference (this repo): `src/protocolgame.cpp`.

use std::collections::HashSet;

use tfs_rust_common::Position;
use tfs_rust_common::protocol_constants::{
    MAP_MAX_LAYERS, MAX_CLIENT_VIEWPORT_X, MAX_CLIENT_VIEWPORT_Y, client_viewport_height,
    client_viewport_width,
};

use crate::NetworkMessage;
use crate::codec::Codec;
use crate::creature_encode::AddCreatureWire;

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
    /// Cip `PRIORITY_BOTTOM` (pools/splashes). Stored newest-first (index 0 = most recent).
    /// Magic fields are LOW — see [`Self::low_items`].
    /// TVP path also parks all non-top down items here (emitted after creatures).
    pub bottom_items: Vec<ItemStack>,
    /// Cip `PRIORITY_LOW` only (ordinary down items). Emitted **after** creatures,
    /// newest-first: `PlaceObject` does not append LOW objects (`map.cc:2040`), so the
    /// most recently placed one heads the group — same rule the client uses for `0x6A`.
    /// Empty on TVP / OTClient / 1098 (those fold lows into [`Self::bottom_items`]).
    pub low_items: Vec<ItemStack>,
    /// Bottom-to-top creature order as stored; emitted in **reverse** (C++ `reverse(creatures)`).
    pub creatures: Vec<AddCreatureWire>,
    /// Real 772 client: Cip map-container order
    /// `Bank → Bottom → Top → Creature → Low` (`map.hh` PRIORITY_*, `PlaceObject`).
    /// TVP / OTClient / 1098: ground→top→creatures→bottom(+low).
    pub cip_map_order: bool,
}

/// C++ `ProtocolGame::checkCreatureAsKnown` — shared with tile appear broadcasts.
pub use crate::creature_known::check_creature_known;

fn write_item_stack(codec: &Codec, msg: &mut NetworkMessage, it: &ItemStack) {
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

fn item_stack_wire_len(codec: &Codec, it: &ItemStack) -> usize {
    codec.item_template_wire_len(
        it.client_id,
        it.count,
        it.stackable,
        it.is_splash_or_fluid,
        it.is_animation,
        false,
    )
}

fn emit_creatures_capped<F: FnMut(u32) -> bool>(
    codec: &Codec,
    msg: &mut NetworkMessage,
    tile: &TileContent,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
    count: &mut i32,
) {
    for c in tile.creatures.iter().rev() {
        // 7.72 returns early once count hits 10 inside the creature loop; 10.98 does not.
        if codec.tile_description_caps_creatures() && *count == 10 {
            return;
        }
        let id = c.id;
        let limit = codec.caps().known_creature_limit as usize;
        let (known, remove) = check_creature_known(id, known_creatures, can_see_creature, limit);
        let mut cw = c.clone();
        cw.apply_known_check(known, remove);
        codec.write_add_creature(msg, &cw);
        *count += 1;
    }
}

fn count_creatures_capped<F: FnMut(u32) -> bool>(
    codec: &Codec,
    tile: &TileContent,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
    count: &mut i32,
    n: &mut usize,
) {
    for c in tile.creatures.iter().rev() {
        if codec.tile_description_caps_creatures() && *count == 10 {
            return;
        }
        let id = c.id;
        let limit = codec.caps().known_creature_limit as usize;
        let (known, remove) = check_creature_known(id, known_creatures, can_see_creature, limit);
        let mut cw = c.clone();
        cw.apply_known_check(known, remove);
        *n += codec.add_creature_wire_len(&cw);
        *count += 1;
    }
}

/// BOTTOM is an appended priority group (`PlaceObject` forces `Append` for everything but
/// CREATURE and LOW — `map.cc:2040`), so the oldest object heads the group. The source vector
/// is stored newest-first, hence the reverse.
fn appended_group_emission_order(items: &[ItemStack]) -> impl Iterator<Item = &ItemStack> {
    items.iter().rev()
}

fn get_tile_description<F: FnMut(u32) -> bool>(
    codec: &Codec,
    msg: &mut NetworkMessage,
    tile: &TileContent,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
    _with_description: bool,
) {
    codec.write_tile_environment_prefix(msg);

    let mut count: i32 = if tile.ground.is_some() { 1 } else { 0 };

    if let Some(ref g) = tile.ground {
        if g.client_id != 0 {
            write_item_stack(codec, msg, g);
        } else {
            count = 0;
        }
    }

    if tile.cip_map_order {
        // Cip `PlaceObject` priority: Bank → Bottom → Top → Creature → Low.
        for it in appended_group_emission_order(&tile.bottom_items) {
            if it.client_id == 0 || count == 10 {
                continue;
            }
            write_item_stack(codec, msg, it);
            count += 1;
        }
        for it in &tile.top_items {
            if it.client_id == 0 || count == 10 {
                continue;
            }
            write_item_stack(codec, msg, it);
            count += 1;
        }
        emit_creatures_capped(
            codec,
            msg,
            tile,
            known_creatures,
            can_see_creature,
            &mut count,
        );
        if count < 10 {
            for it in &tile.low_items {
                if count == 10 {
                    return;
                }
                if it.client_id == 0 {
                    continue;
                }
                write_item_stack(codec, msg, it);
                count += 1;
            }
        }
    } else {
        // TVP / 1098: ground → top → creatures → bottom (+ low folded into bottom_items).
        for it in &tile.top_items {
            if it.client_id == 0 || count == 10 {
                continue;
            }
            write_item_stack(codec, msg, it);
            count += 1;
        }
        emit_creatures_capped(
            codec,
            msg,
            tile,
            known_creatures,
            can_see_creature,
            &mut count,
        );
        if count < 10 {
            for it in tile.bottom_items.iter().chain(tile.low_items.iter()) {
                if count == 10 {
                    return;
                }
                if it.client_id == 0 {
                    continue;
                }
                write_item_stack(codec, msg, it);
                count += 1;
            }
        }
    }
}

/// Independent byte count for [`get_tile_description`].
///
/// Map tiles always use `write_item_template(..., false)` — C++ `GetTileDescription` does not pass
/// OTCv8 description for template map items (`src/protocolgame.cpp`). Must match that, not the
/// outer `with_description` used elsewhere.
fn count_tile_description<F: FnMut(u32) -> bool>(
    codec: &Codec,
    tile: &TileContent,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
) -> usize {
    let mut n = codec.tile_environment_prefix_len(); // environmental effects (2 for 1098, 0 for 772)

    let mut count: i32 = if tile.ground.is_some() { 1 } else { 0 };

    if let Some(ref g) = tile.ground {
        if g.client_id != 0 {
            n += item_stack_wire_len(codec, g);
        } else {
            count = 0;
        }
    }

    if tile.cip_map_order {
        for it in appended_group_emission_order(&tile.bottom_items) {
            if it.client_id == 0 || count == 10 {
                continue;
            }
            n += item_stack_wire_len(codec, it);
            count += 1;
        }
        for it in &tile.top_items {
            if it.client_id == 0 || count == 10 {
                continue;
            }
            n += item_stack_wire_len(codec, it);
            count += 1;
        }
        count_creatures_capped(
            codec,
            tile,
            known_creatures,
            can_see_creature,
            &mut count,
            &mut n,
        );
        if count < 10 {
            for it in &tile.low_items {
                if count == 10 {
                    break;
                }
                if it.client_id == 0 {
                    continue;
                }
                n += item_stack_wire_len(codec, it);
                count += 1;
            }
        }
    } else {
        for it in &tile.top_items {
            if it.client_id == 0 || count == 10 {
                continue;
            }
            n += item_stack_wire_len(codec, it);
            count += 1;
        }
        count_creatures_capped(
            codec,
            tile,
            known_creatures,
            can_see_creature,
            &mut count,
            &mut n,
        );
        if count < 10 {
            for it in tile.bottom_items.iter().chain(tile.low_items.iter()) {
                if count == 10 {
                    break;
                }
                if it.client_id == 0 {
                    continue;
                }
                n += item_stack_wire_len(codec, it);
                count += 1;
            }
        }
    }
    n
}

#[allow(clippy::too_many_arguments)]
fn get_floor_description<F: FnMut(u32) -> bool>(
    codec: &Codec,
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
                    codec,
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
    codec: &Codec,
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
                n += count_tile_description(codec, &tile, known_creatures, can_see_creature);
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
    codec: &Codec,
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
            codec,
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
    codec: &Codec,
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
            codec,
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
    codec: &Codec,
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
            codec,
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
        codec,
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
#[allow(clippy::too_many_arguments)] // mirrors C++ `ProtocolGame::MoveUpCreature` parameters (parity)
fn append_move_up_creature<F: FnMut(u32) -> bool>(
    codec: &Codec,
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
                codec,
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
            codec,
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
        codec,
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
        codec,
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
#[allow(clippy::too_many_arguments)] // mirrors C++ `ProtocolGame::MoveDownCreature` parameters (parity)
fn append_move_down_creature<F: FnMut(u32) -> bool>(
    codec: &Codec,
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
                codec,
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
            codec,
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
        codec,
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
        codec,
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

/// `SendFloors` body — floor description data after the 0xBE/0xBF opcode.
// C++ reference: `sending.cc:517-578` `SendFloors`.
//
/// `player_z` is the CURRENT z (after adjustment). The floor description covers
/// the new floors the client needs based on direction (up/down) and current z.
#[allow(clippy::too_many_arguments)]
fn append_send_floors_body<F: FnMut(u32) -> bool>(
    codec: &Codec,
    msg: &mut NetworkMessage,
    player_x: i32,
    player_y: i32,
    player_z: i32,
    up: bool,
    get_tile: &mut impl FnMut(i32, i32, i32) -> Option<TileContent>,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
    with_description: bool,
) {
    let vw = client_viewport_width();
    let vh = client_viewport_height();
    let ox = player_x - MAX_CLIENT_VIEWPORT_X;
    let oy = player_y - MAX_CLIENT_VIEWPORT_Y;

    // Decompile `SendFloors`:
    //   Up + z==7: send floors [5, 0] (going to surface)
    //   Up + z>7:  send floor z-2 (one floor up, still underground)
    //   Down + z==8: send floors [8, 10] (going underground)
    //   Down + z>8:  send floor z+2 (one floor down, still underground)
    let (start_z, end_z, step_z): (i32, i32, i32) = if up {
        if player_z == 7 {
            (5, -1, -1) // floors 5→0, step -1 (EndZ exclusive: EndZ = 0 + (-1) = -1)
        } else if player_z > 7 {
            (player_z - 2, player_z - 3, -1) // one floor up
        } else {
            return; // z < 7 going up — no floor data
        }
    } else {
        if player_z == 8 {
            (8, 11, 1) // floors 8→10, step 1 (EndZ exclusive: EndZ = 10 + 1 = 11)
        } else if player_z > 8 && player_z + 2 <= 15 {
            (player_z + 2, player_z + 3, 1) // one floor down
        } else {
            return; // no floor data
        }
    };

    let mut skip = -1_i32;
    let mut z = start_z;
    while z != end_z {
        // Decompile uses ZOffset = player_z - z for x/y adjustment
        let z_offset = player_z - z;
        get_floor_description(
            codec,
            msg,
            ox,
            oy,
            z,
            vw,
            vh,
            z_offset,
            &mut skip,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
        z += step_z;
    }
    if skip >= 0 {
        msg.write_u8(skip as u8);
        msg.write_u8(0xFF);
    }
}

/// `SendRow` body — row description data after the direction opcode.
// C++ reference: `sending.cc:463-515` `SendRow`.
//
/// Sends a single row/column of map data at the given player position, with
/// multi-floor z-offset logic (sends multiple z-levels).
#[allow(clippy::too_many_arguments)]
fn append_send_row<F: FnMut(u32) -> bool>(
    codec: &Codec,
    msg: &mut NetworkMessage,
    player_x: i32,
    player_y: i32,
    player_z: i32,
    direction_opcode: u8,
    get_tile: &mut impl FnMut(i32, i32, i32) -> Option<TileContent>,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
    with_description: bool,
) {
    msg.write_u8(direction_opcode);

    let vw = client_viewport_width();
    let vh = client_viewport_height();
    let min_x = player_x - MAX_CLIENT_VIEWPORT_X;
    let min_y = player_y - MAX_CLIENT_VIEWPORT_Y;

    // Decompile `SendRow` z-range:
    //   z <= 7: floors 7→0, step -1
    //   z > 7:  floors z-2 → min(z+2, 15), step 1
    let (start_z, end_z, step_z): (i32, i32, i32) = if player_z <= 7 {
        (7, -1, -1) // floors 7→0, EndZ = 0 + (-1) = -1
    } else {
        let end = (player_z + 2).min(15) + 1;
        (player_z - 2, end, 1)
    };

    // Determine which row/column to send based on direction.
    // Decompile `SendRow`: NORTH → y=min_y, EAST → x=max_x, SOUTH → y=max_y, WEST → x=min_x.
    let (origin_x, origin_y, width, height) = match direction_opcode {
        0x65 => (min_x, min_y, vw, 1), // NORTH: full width, 1 row at min_y
        0x66 => (min_x + vw - 1, min_y, 1, vh), // EAST:  1 col at max_x, full height
        0x67 => (min_x, min_y + vh - 1, vw, 1), // SOUTH: full width, 1 row at max_y
        0x68 => (min_x, min_y, 1, vh), // WEST:  1 col at min_x, full height
        _ => return,
    };

    let mut skip = -1_i32;
    let mut z = start_z;
    while z != end_z {
        let z_offset = player_z - z;
        get_floor_description(
            codec,
            msg,
            origin_x,
            origin_y,
            z,
            width,
            height,
            z_offset,
            &mut skip,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
        z += step_z;
    }
    if skip >= 0 {
        msg.write_u8(skip as u8);
        msg.write_u8(0xFF);
    }
}

//
// Both eras emit the self-packet (`0x6D`/`0x6C`) — TVP works fine on the real 772 client.
// The self-packet updates the client's central position BEFORE the floor change (0xBE/0xBF)
// is processed. Without it, the client uses the OLD position when parsing the floor
// description, placing tiles at wrong coordinates → "no thing at pos" errors.
#[allow(clippy::too_many_arguments)] // mirrors C++ `ProtocolGame::sendMoveCreature` parameters (parity)
pub fn send_move_creature_player<F: FnMut(u32) -> bool>(
    codec: &Codec,
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
        // Self-packet is REQUIRED for both clients — it updates the client's central
        // position BEFORE the floor change (0xBE/0xBF) is processed. §6 experiment
        // (2026-07-04) confirmed: suppressing it desyncs both OTClient and the real
        // 772 client immediately. The `bug000017` log is a debug warning, not a desync.
        // TVP sends it on both eras and works fine.
        if old_pos.z == 7 && new_pos.z >= 8 {
            if (0..10).contains(&old_stack_pos) {
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
            if (0..10).contains(&old_stack_pos) {
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
                codec,
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
                codec,
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
                codec,
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
                codec,
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
                codec,
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
                codec,
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
    // Self-packet is REQUIRED for both clients on same-z moves too. §6 experiment
    // confirmed suppressing it desyncs immediately.
    msg.write_u8(0x6D);
    if (0..10).contains(&old_stack_pos) {
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
            codec,
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
            codec,
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
            codec,
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
            codec,
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

/// 772 `TCreature::NotifyGo` — the player's **own** move notification
/// (`reference/cipsoft-772/tibia-game-master/src/cract.cc:1400-1465`).
///
/// Computes the **overall** `orig → dest` delta and walks it in a fixed order —
/// z-steps first (each shifts x/y diagonally by ∓1), then x-steps, then y-steps —
/// emitting `SendFloors` (0xBE/0xBF) / `SendRow` (0x65-0x68) per step.
///
/// Non-adjacent (`|d| > 1` on any axis): `NotifyGo` else branch sets pos then
/// `SendFullScreen` only (`cract.cc:1455-1459`; `sending.cc` `SendFullScreen`) —
/// `0x64` + dest pos + map body. No `0x6D`/`0x6C`.
///
/// Adjacent leading self-packet (before floors/rows):
/// - surface→underground (`orig.z == 7 && dest.z >= 8`): `0x6C` remove — TVP
///   `sendMoveCreature` (`protocolgame.cpp` ~1793–1805). A `0x6D` pre-sets client z and
///   FloorDown then asserts `rz=-1` (bug0000013).
/// - otherwise: `0x6D` old+stack+new (centre update before `0xBE`/`SendRow`).
///
/// Callers pass the overall pre-move position (`orig`) and final position (`dest`); the
/// queryDestination chain must **not** be emitted per segment — that produces an invalid
/// row sequence for combined diagonal+z stair moves (`docs/772_FLOOR_CHANGE_DESYNC.md` §16.3,
/// e.g. walking west onto south-facing stairs).
#[allow(clippy::too_many_arguments)]
pub fn send_notify_go<F: FnMut(u32) -> bool>(
    codec: &Codec,
    orig: Position,
    dest: Position,
    old_stack_pos: i32,
    creature_id: u32,
    get_tile: &mut impl FnMut(i32, i32, i32) -> Option<TileContent>,
    known_creatures: &mut HashSet<u32>,
    can_see_creature: &mut F,
    with_description: bool,
) -> NetworkMessage {
    let (ox, oy, oz) = (orig.x as i32, orig.y as i32, orig.z as i32);
    let (dx, dy, dz) = (dest.x as i32, dest.y as i32, dest.z as i32);

    // Non-adjacent → SendFullScreen only (`cract.cc` NotifyGo else; `sending.cc` SendFullScreen).
    if (dx - ox).abs() > 1 || (dy - oy).abs() > 1 || (dz - oz).abs() > 1 {
        return send_map_description_packet(
            codec,
            dest,
            dest,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
    }

    let mut msg = NetworkMessage::new();

    // Leading self-packet before `SendFloors`/`SendRow`.
    //
    // Surface → underground (`z=7` → `z≥8`) must be `0x6C` remove (old pos only), matching
    // TVP `sendMoveCreature` (`protocolgame.cpp` ~1793–1805) and [`send_move_creature_player`].
    // A `0x6D` here pre-sets the client's map centre to the new z; `0xBF` FloorDown then
    // increments z again and reads the wrong floor-count / offsets → `Map.cpp` assert
    // `rz = -1` / bug0000013 (live 772 client, 2026-08-01).
    //
    // All other adjacent moves keep `0x6D` (old + stack + new) so the centre updates before
    // `0xBE`/`SendRow` (§6 experiment).
    let surface_to_underground = oz == 7 && dz >= 8;
    if surface_to_underground {
        msg.write_u8(0x6C);
        if (0..10).contains(&old_stack_pos) {
            msg.write_position(&orig);
            msg.write_u8(old_stack_pos as u8);
        } else {
            msg.write_u16(0xFFFF);
            msg.write_u32(creature_id);
        }
    } else {
        msg.write_u8(0x6D);
        if (0..10).contains(&old_stack_pos) {
            msg.write_position(&orig);
            msg.write_u8(old_stack_pos as u8);
        } else {
            msg.write_u16(0xFFFF);
            msg.write_u32(creature_id);
        }
        msg.write_position(&dest);
    }

    let (mut px, mut py, mut pz) = (ox, oy, oz);

    // z-steps first — each floor change shifts x/y diagonally (`cract.cc:1423-1436`).
    while pz < dz {
        px -= 1;
        py -= 1;
        pz += 1;
        msg.write_u8(0xBF);
        append_send_floors_body(
            codec,
            &mut msg,
            px,
            py,
            pz,
            false,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
    }
    while pz > dz {
        px += 1;
        py += 1;
        pz -= 1;
        msg.write_u8(0xBE);
        append_send_floors_body(
            codec,
            &mut msg,
            px,
            py,
            pz,
            true,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
    }

    // x-steps (`cract.cc:1438-1446`).
    while px < dx {
        px += 1;
        append_send_row(
            codec,
            &mut msg,
            px,
            py,
            pz,
            0x66,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
    }
    while px > dx {
        px -= 1;
        append_send_row(
            codec,
            &mut msg,
            px,
            py,
            pz,
            0x68,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
    }

    // y-steps (`cract.cc:1448-1456`).
    while py < dy {
        py += 1;
        append_send_row(
            codec,
            &mut msg,
            px,
            py,
            pz,
            0x67,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
    }
    while py > dy {
        py -= 1;
        append_send_row(
            codec,
            &mut msg,
            px,
            py,
            pz,
            0x65,
            get_tile,
            known_creatures,
            can_see_creature,
            with_description,
        );
    }

    msg
}

/// Other creature's walk (not the local player): `ProtocolGame::sendMoveCreature` when
/// `creature != player` and both old and new positions are visible (`protocolgame.cpp:1830-1848`).
/// No map row opcodes — client shifts the sprite from old stack to new tile.
///
/// TVP always sends `0x6D` for spectators (non-teleport, non-z7→z8, both visible),
/// using the `0xFFFF + creatureID` fallback when `oldStackPos >= 10` (line 1844-1845).
pub fn send_move_creature_spectator(
    _codec: &Codec,
    old_pos: Position,
    new_pos: Position,
    old_stack_pos: i32,
    creature_id: u32,
) -> Option<NetworkMessage> {
    let mut msg = NetworkMessage::new();
    msg.write_u8(0x6D);
    if (0..10).contains(&old_stack_pos) {
        msg.write_position(&old_pos);
        msg.write_u8(old_stack_pos as u8);
    } else {
        // 0xFFFF + creature_id fallback (`protocolgame.cpp:1844-1845`).
        msg.write_u16(0xFFFF);
        msg.write_u32(creature_id);
    }
    msg.write_position(&new_pos);
    Some(msg)
}

/// `ProtocolGame::sendUpdateTile` (`src/protocolgame.cpp` ~2683).
pub fn send_update_tile<F: FnMut(u32) -> bool>(
    codec: &Codec,
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
            codec,
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
