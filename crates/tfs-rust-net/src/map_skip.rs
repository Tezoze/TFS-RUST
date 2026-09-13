//! Skip 772 map-description bodies so later opcodes in the same payload stay reachable.
//!
//! Encoder: [`crate::map_description::write_map_description_body`] / TVP
//! `ProtocolGame::GetMapDescription` (`protocolgame.cpp` ~589). Client skip-counter:
//! OTClient `getMapDescription` / `getFloorDescription` / `getTileDescription`.
//!
//! 772 item extra byte: `Codec772::write_item_template` / TVP `NetworkMessage::addItem` —
//! `u16` client id, then **one** count/liquid byte iff stackable or splash/fluid. No MARK,
//! no animation phase. Creatures: `0x61`/`0x62` (TVP `AddCreature` ~2051) and rust `0x63`.

use tfs_rust_common::protocol_constants::{
    MAP_MAX_LAYERS, client_viewport_height, client_viewport_width,
};

const CREATURE_UNKNOWN: u16 = 0x61;
const CREATURE_OUTDATED: u16 = 0x62;
const CREATURE_UPTODATE: u16 = 0x63;
const TILE_THING_CAP: u8 = 10;

/// Floors visited by `GetMapDescription` for player z (`protocolgame.cpp` ~594–606).
pub fn map_description_floor_count(player_z: u8) -> i32 {
    let z = i32::from(player_z);
    if z > 7 {
        let startz = z - 2;
        let endz = (MAP_MAX_LAYERS - 1).min(z + 2);
        endz - startz + 1
    } else {
        8
    }
}

/// Skip `GetMapDescription` body (after opcode; after position for `0x64`).
pub fn skip_772_map_description_body(
    buf: &[u8],
    i: &mut usize,
    player_z: u8,
    width: i32,
    height: i32,
    item_has_extra: impl Fn(u16) -> bool,
) -> bool {
    skip_skip_stream(
        buf,
        i,
        map_description_floor_count(player_z),
        width,
        height,
        item_has_extra,
    )
}

/// Skip TVP/rust `MoveUpCreature` body after opcode `0xBE` (`protocolgame.cpp` ~2188–2211).
///
/// Floor bytes are omitted when `player_z < 7`. Next opcode (`0x68` / `0x65`) is left unread.
pub fn skip_772_move_up_floor_body(
    buf: &[u8],
    i: &mut usize,
    player_z: u8,
    item_has_extra: impl Fn(u16) -> bool,
) -> bool {
    let floors = if player_z == 7 {
        6
    } else if player_z > 7 {
        1
    } else {
        0
    };
    skip_skip_stream(
        buf,
        i,
        floors,
        client_viewport_width(),
        client_viewport_height(),
        item_has_extra,
    )
}

/// Skip TVP/rust `MoveDownCreature` body after opcode `0xBF` (`protocolgame.cpp` ~2232–2253).
pub fn skip_772_move_down_floor_body(
    buf: &[u8],
    i: &mut usize,
    player_z: u8,
    item_has_extra: impl Fn(u16) -> bool,
) -> bool {
    let floors = if player_z == 8 {
        3
    } else if (9..14).contains(&player_z) {
        1
    } else {
        0
    };
    skip_skip_stream(
        buf,
        i,
        floors,
        client_viewport_width(),
        client_viewport_height(),
        item_has_extra,
    )
}

/// One tile: things until a skip pair (`u16 >= 0xFF00`). Returns the skip count (low byte).
pub fn skip_772_tile_description(
    buf: &[u8],
    i: &mut usize,
    item_has_extra: impl Fn(u16) -> bool,
) -> Option<i32> {
    skip_tile_description(buf, i, &item_has_extra)
}

/// One map object: creature `0x61`/`0x62`/`0x63` or a 772 item template.
pub fn skip_772_thing(buf: &[u8], i: &mut usize, item_has_extra: impl Fn(u16) -> bool) -> bool {
    skip_thing(buf, i, &item_has_extra)
}

fn skip_skip_stream(
    buf: &[u8],
    i: &mut usize,
    floors: i32,
    width: i32,
    height: i32,
    item_has_extra: impl Fn(u16) -> bool,
) -> bool {
    if floors < 0 || width <= 0 || height <= 0 {
        return floors == 0;
    }
    if floors == 0 {
        return true;
    }
    let tiles = i64::from(floors) * i64::from(width) * i64::from(height);
    let mut skip = 0i32;
    for _ in 0..tiles {
        if skip == 0 {
            match skip_tile_description(buf, i, &item_has_extra) {
                Some(n) => skip = n,
                None => return false,
            }
        } else {
            skip -= 1;
        }
    }
    true
}

fn skip_tile_description(
    buf: &[u8],
    i: &mut usize,
    item_has_extra: &impl Fn(u16) -> bool,
) -> Option<i32> {
    let mut nthings = 0u8;
    loop {
        let peek = peek_u16(buf, *i)?;
        if peek >= 0xFF00 {
            *i += 2;
            return Some(i32::from(peek as u8));
        }
        if nthings >= TILE_THING_CAP {
            return None;
        }
        if !skip_thing(buf, i, item_has_extra) {
            return None;
        }
        nthings += 1;
    }
}

fn skip_thing(buf: &[u8], i: &mut usize, item_has_extra: &impl Fn(u16) -> bool) -> bool {
    let Some(id) = peek_u16(buf, *i) else {
        return false;
    };
    match id {
        CREATURE_UNKNOWN | CREATURE_OUTDATED | CREATURE_UPTODATE => skip_creature(buf, i, id),
        _ => {
            *i += 2;
            if item_has_extra(id) {
                if *i >= buf.len() {
                    return false;
                }
                *i += 1;
            }
            true
        }
    }
}

fn skip_creature(buf: &[u8], i: &mut usize, kind: u16) -> bool {
    *i += 2;
    match kind {
        CREATURE_UPTODATE => take(buf, i, 4 + 1), // id + direction
        CREATURE_OUTDATED => take(buf, i, 4) && skip_creature_tail(buf, i),
        CREATURE_UNKNOWN => {
            take(buf, i, 4 + 4) && skip_string(buf, i) && skip_creature_tail(buf, i)
        }
        _ => false,
    }
}

fn skip_creature_tail(buf: &[u8], i: &mut usize) -> bool {
    // health, direction, outfit, light(2), speed(2), skull, party
    if !take(buf, i, 2) {
        return false;
    }
    if !skip_outfit(buf, i) {
        return false;
    }
    take(buf, i, 1 + 1 + 2 + 1 + 1)
}

fn skip_outfit(buf: &[u8], i: &mut usize) -> bool {
    let Some(look_type) = read_u16(buf, i) else {
        return false;
    };
    if look_type != 0 {
        take(buf, i, 4)
    } else {
        take(buf, i, 2)
    }
}

fn skip_string(buf: &[u8], i: &mut usize) -> bool {
    let Some(len) = read_u16(buf, i) else {
        return false;
    };
    take(buf, i, usize::from(len))
}

fn peek_u16(buf: &[u8], i: usize) -> Option<u16> {
    let b = buf.get(i..i + 2)?;
    Some(u16::from_le_bytes([b[0], b[1]]))
}

fn read_u16(buf: &[u8], i: &mut usize) -> Option<u16> {
    let v = peek_u16(buf, *i)?;
    *i += 2;
    Some(v)
}

fn take(buf: &[u8], i: &mut usize, n: usize) -> bool {
    if buf.len().saturating_sub(*i) < n {
        return false;
    }
    *i += n;
    true
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use tfs_rust_common::protocol_constants::{client_viewport_height, client_viewport_width};
    use tfs_rust_common::{Position, ProtocolVersion};

    use super::*;
    use crate::NetworkMessage;
    use crate::codec::Codec;
    use crate::map_description::{
        ItemStack, TileContent, send_map_description_packet, write_map_description_body,
    };

    fn codec_772() -> Codec {
        Codec::from_version(ProtocolVersion::V772).expect("772 codec")
    }

    fn empty_tile(_: i32, _: i32, _: i32) -> Option<TileContent> {
        None
    }

    #[test]
    fn skip_empty_full_map_then_magic_effect_byte() {
        let player = Position::new(32369, 32241, 7);
        let mut known = HashSet::new();
        let mut get_tile = empty_tile;
        let mut can_see = |_id: u32| true;
        let msg = send_map_description_packet(
            &codec_772(),
            player,
            player,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        );
        let mut payload = msg.into_bytes();
        payload.push(0x83);
        payload.extend_from_slice(&1u16.to_le_bytes());
        payload.extend_from_slice(&2u16.to_le_bytes());
        payload.push(7);
        payload.push(11);

        assert_eq!(payload[0], 0x64);
        let mut i = 1usize;
        i += 5; // position
        assert!(skip_772_map_description_body(
            &payload,
            &mut i,
            7,
            client_viewport_width(),
            client_viewport_height(),
            |_| false,
        ));
        assert_eq!(payload[i], 0x83);
        assert_eq!(i, payload.len() - 7);
    }

    #[test]
    fn skip_empty_north_strip() {
        let mut msg = NetworkMessage::new();
        let mut known = HashSet::new();
        let mut get_tile = empty_tile;
        let mut can_see = |_id: u32| true;
        write_map_description_body(
            &codec_772(),
            &mut msg,
            100,
            200,
            7,
            client_viewport_width(),
            1,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        );
        let body = msg.into_bytes();
        let mut i = 0usize;
        assert!(skip_772_map_description_body(
            &body,
            &mut i,
            7,
            client_viewport_width(),
            1,
            |_| false,
        ));
        assert_eq!(i, body.len());
    }

    #[test]
    fn skip_ground_tile_without_extra_byte() {
        let center = Position::new(100, 200, 7);
        let mut known = HashSet::new();
        let mut get_tile = |x: i32, y: i32, z: i32| -> Option<TileContent> {
            if x == i32::from(center.x) && y == i32::from(center.y) && z == i32::from(center.z) {
                Some(TileContent {
                    ground: Some(ItemStack {
                        client_id: 0x0673,
                        count: 1,
                        stackable: false,
                        is_splash_or_fluid: false,
                        is_animation: false,
                    }),
                    ..TileContent::default()
                })
            } else {
                None
            }
        };
        let mut can_see = |_id: u32| true;
        let msg = send_map_description_packet(
            &codec_772(),
            center,
            center,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        );
        let bytes = msg.into_bytes();
        let mut i = 6usize; // opcode + pos
        assert!(skip_772_map_description_body(
            &bytes,
            &mut i,
            7,
            client_viewport_width(),
            client_viewport_height(),
            |_| false,
        ));
        assert_eq!(i, bytes.len());
    }

    #[test]
    fn skip_stackable_needs_extra_byte() {
        let center = Position::new(100, 200, 7);
        let gold = 3031u16;
        let mut known = HashSet::new();
        let mut get_tile = |x: i32, y: i32, z: i32| -> Option<TileContent> {
            if x == i32::from(center.x) && y == i32::from(center.y) && z == i32::from(center.z) {
                Some(TileContent {
                    ground: Some(ItemStack {
                        client_id: gold,
                        count: 5,
                        stackable: true,
                        is_splash_or_fluid: false,
                        is_animation: false,
                    }),
                    ..TileContent::default()
                })
            } else {
                None
            }
        };
        let mut can_see = |_id: u32| true;
        let msg = send_map_description_packet(
            &codec_772(),
            center,
            center,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        );
        let bytes = msg.into_bytes();
        let mut i = 6usize;
        assert!(
            !skip_772_map_description_body(
                &bytes,
                &mut i,
                7,
                client_viewport_width(),
                client_viewport_height(),
                |_| false,
            ),
            "stackable extra byte must not be treated as skip/item"
        );
        i = 6;
        assert!(skip_772_map_description_body(
            &bytes,
            &mut i,
            7,
            client_viewport_width(),
            client_viewport_height(),
            |id| id == gold,
        ));
        assert_eq!(i, bytes.len());
    }
}
