//! `GetMapDescription` / floor skip logic (vs `src/protocolgame.cpp`).

use std::collections::HashSet;

use tfs_rust_common::{Position, ProtocolVersion};
use tfs_rust_net::map_description::{
    send_map_description_packet, send_move_creature_player, send_move_creature_spectator,
    TileContent,
};
use tfs_rust_net::{Codec, NetworkMessage};

fn codec_1098() -> Codec {
    Codec::from_version(ProtocolVersion::V1098).expect("1098 codec")
}

#[test]
fn full_map_description_empty_map_terminates_skip() {
    let player = Position::new(100, 200, 7);
    let center = player;
    let mut known = HashSet::new();
    let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
    let mut can_see = |_id: u32| true;
    let msg: NetworkMessage = send_map_description_packet(
        &codec_1098(),
        player,
        center,
        &mut get_tile,
        &mut known,
        &mut can_see,
        false,
    );
    let b = msg.as_bytes();
    assert_eq!(b[0], 0x64);
    // `Position`: u16 x, u16 y, u8 z (5 bytes).
    assert_eq!(&b[1..6], &[100, 0, 200, 0, 7]);
    assert!(b.len() > 6);
    // `GetMapDescription` ends with `skip` (may be large if all tiles empty) then `0xFF`.
    assert_eq!(b[b.len() - 1], 0xFF);
}

#[test]
fn move_creature_player_starts_with_6d_not_full_map_stub() {
    let old_p = Position::new(100, 200, 7);
    let new_p = Position::new(101, 200, 7);
    let mut known = HashSet::new();
    let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
    let mut can_see = |_id: u32| true;
    let msg = send_move_creature_player(
        &codec_1098(),
        old_p,
        new_p,
        1,
        1,
        &mut get_tile,
        &mut known,
        &mut can_see,
        false,
    );
    let b = msg.as_bytes();
    assert_eq!(b[0], 0x6D, "walk must use MoveCreature, not opcode 0x64 map stub");
    assert_ne!(b[0], 0x64);
}

#[test]
fn move_creature_spectator_uses_stack_when_in_range() {
    let old_p = Position::new(100, 200, 7);
    let new_p = Position::new(101, 200, 7);
    let msg = send_move_creature_spectator(old_p, new_p, 3, 0x11223344);
    let b = msg.as_bytes();
    assert_eq!(b[0], 0x6D);
    assert_eq!(&b[1..6], &[100, 0, 200, 0, 7]);
    assert_eq!(b[6], 3);
    assert_eq!(&b[7..12], &[101, 0, 200, 0, 7]);
}

#[test]
fn move_creature_spectator_falls_back_to_creature_id_when_stack_invalid() {
    let old_p = Position::new(50, 60, 3);
    let new_p = Position::new(51, 60, 3);
    let msg = send_move_creature_spectator(old_p, new_p, -1, 0xAABBCCDD);
    let b = msg.as_bytes();
    assert_eq!(b[0], 0x6D);
    assert_eq!(&b[1..3], &[0xFF, 0xFF]);
    assert_eq!(&b[3..7], &[0xDD, 0xCC, 0xBB, 0xAA]);
    assert_eq!(&b[7..12], &[51, 0, 60, 0, 3]);
}
