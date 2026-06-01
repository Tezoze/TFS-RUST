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

fn codec_772() -> Codec {
    Codec::from_version(ProtocolVersion::V772).expect("772 codec")
}

/// A single non-empty tile (ground item only) under each codec. 1098 prefixes every tile with a
/// `u16` environmental-effects field (`0x00 0x00`); 772 (`gameserver/src`) omits it entirely.
fn single_ground_tile_map(codec: &Codec, center: Position) -> Vec<u8> {
    use tfs_rust_net::map_description::ItemStack;
    let mut known = HashSet::new();
    let mut get_tile = move |x: i32, y: i32, z: i32| -> Option<TileContent> {
        if x == center.x as i32 && y == center.y as i32 && z == center.z as i32 {
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
    send_map_description_packet(
        codec, center, center, &mut get_tile, &mut known, &mut can_see, false,
    )
    .into_bytes()
}

#[test]
fn tile_environment_prefix_is_1098_only() {
    // Center the player so the very first described tile (top-left of the viewport at floor 7's
    // first non-empty scan) is deterministic; we only assert on the env-prefix presence/absence by
    // length difference for the same content.
    let center = Position::new(100, 200, 7);
    let b1098 = single_ground_tile_map(&codec_1098(), center);
    let b772 = single_ground_tile_map(&codec_772(), center);

    // Both start with the 0x64 map opcode + position (6 bytes).
    assert_eq!(b1098[0], 0x64);
    assert_eq!(b772[0], 0x64);
    assert_eq!(&b1098[1..6], &[100, 0, 200, 0, 7]);
    assert_eq!(&b772[1..6], &[100, 0, 200, 0, 7]);

    // Same single ground item (client id 0x0673, 2-byte item, no count for non-stackable) on both,
    // but 1098 carries exactly one extra `0x00 0x00` environmental-effects field for that tile.
    assert_eq!(
        b1098.len(),
        b772.len() + 2,
        "1098 map must be exactly 2 bytes longer (per-tile env prefix); 772 omits it"
    );
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
