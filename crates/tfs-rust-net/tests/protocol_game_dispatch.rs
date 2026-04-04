//! `game_command_from_payload` → `GameCommand::Game`.

use tfs_rust_common::enums::Direction;
use tfs_rust_common::{ConnId, GameCommand, GamePacket};
use tfs_rust_net::protocol_game::game_command_from_payload;

#[test]
fn parses_move_north() {
    let payload = [0x65u8];
    let cmd = game_command_from_payload(ConnId(1), &payload).expect("parse");
    match cmd {
        GameCommand::Game { conn_id, packet } => {
            assert_eq!(conn_id, ConnId(1));
            match packet {
                GamePacket::Move(Direction::North) => {}
                p => panic!("expected Move North, got {:?}", p),
            }
        }
        _ => panic!("expected Game"),
    }
}
