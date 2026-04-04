//! `GetMapDescription` / `GetFloorDescription` / `GetTileDescription`.
// C++ reference (this repo): `src/protocolgame.cpp`.

use std::collections::HashSet;

use tfs_rust_common::protocol_constants::{
    client_viewport_height, client_viewport_width, MAP_MAX_LAYERS, MAX_CLIENT_VIEWPORT_X,
    MAX_CLIENT_VIEWPORT_Y,
};
use tfs_rust_common::Position;

use crate::creature_encode::{write_add_creature, AddCreatureWire};
use crate::item_encode::write_item_template;
use crate::NetworkMessage;

/// Stackable or single item for template encoding (`addItem(id, count, false)`).
#[derive(Debug, Clone)]
pub struct ItemStack {
    pub client_id: u16,
    pub count: u8,
    pub stackable: bool,
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

fn check_creature_known(id: u32, known_set: &mut HashSet<u32>) -> (bool, u32) {
    if !known_set.insert(id) {
        return (true, 0);
    }
    if known_set.len() > 1300 {
        let removed = *known_set.iter().next().unwrap();
        known_set.remove(&removed);
        (false, removed)
    } else {
        (false, 0)
    }
}

fn get_tile_description(
    msg: &mut NetworkMessage,
    tile: &TileContent,
    known_creatures: &mut HashSet<u32>,
) {
    msg.write_u16(0);

    let mut count: i32 = if tile.ground.is_some() { 1 } else { 0 };

    if let Some(ref g) = tile.ground {
        write_item_template(msg, g.client_id, g.count, g.stackable);
    }

    for it in &tile.top_items {
        if count == 10 {
            break;
        }
        write_item_template(msg, it.client_id, it.count, it.stackable);
        count += 1;
    }

    for c in tile.creatures.iter().rev() {
        let id = c.id;
        let (known, remove) = check_creature_known(id, known_creatures);
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
            write_item_template(msg, it.client_id, it.count, it.stackable);
            count += 1;
        }
    }
}

#[allow(clippy::too_many_arguments)] // Mirrors `GetFloorDescription` parameters (`src/protocolgame.cpp`).
fn get_floor_description(
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
                get_tile_description(msg, &tile, known_creatures);
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

/// `ProtocolGame::GetMapDescription` into `msg` (does not prefix opcode — use [`send_map_description_packet`] for full packet).
#[allow(clippy::too_many_arguments)]
pub fn write_map_description_body(
    msg: &mut NetworkMessage,
    origin_x: i32,
    origin_y: i32,
    origin_z: i32,
    width: i32,
    height: i32,
    get_tile: &mut impl FnMut(i32, i32, i32) -> Option<TileContent>,
    known_creatures: &mut HashSet<u32>,
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
pub fn send_map_description_packet(
    player_pos: Position,
    center: Position,
    get_tile: &mut impl FnMut(i32, i32, i32) -> Option<TileContent>,
    known_creatures: &mut HashSet<u32>,
) -> NetworkMessage {
    let mut msg = NetworkMessage::new();
    msg.write_u8(0x64);
    msg.write_position(&player_pos);

    let origin_x = center.x as i32 - MAX_CLIENT_VIEWPORT_X;
    let origin_y = center.y as i32 - MAX_CLIENT_VIEWPORT_Y;
    let origin_z = center.z as i32;
    let w = client_viewport_width();
    let h = client_viewport_height();

    write_map_description_body(
        &mut msg,
        origin_x,
        origin_y,
        origin_z,
        w,
        h,
        get_tile,
        known_creatures,
    );
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
