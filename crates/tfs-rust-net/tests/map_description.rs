//! `GetMapDescription` / floor skip logic (vs `src/protocolgame.cpp`).

use std::collections::HashSet;

use tfs_rust_common::Position;
use tfs_rust_net::map_description::{
    send_map_description_packet, send_move_creature_player, TileContent,
};
use tfs_rust_net::NetworkMessage;

#[test]
fn full_map_description_empty_map_terminates_skip() {
    let player = Position::new(100, 200, 7);
    let center = player;
    let mut known = HashSet::new();
    let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
    let mut can_see = |_id: u32| true;
    let msg: NetworkMessage = send_map_description_packet(
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
