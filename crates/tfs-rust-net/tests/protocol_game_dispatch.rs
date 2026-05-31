//! `game_command_from_payload` → `GameCommand::Game`.

use tfs_rust_common::enums::Direction;
use tfs_rust_common::{ConnId, GameCommand, GamePacket, ProtocolVersion};
use tfs_rust_net::protocol_game::game_command_from_payload;

#[test]
fn parses_enter_game() {
    let payload = [0x0Fu8];
    let cmd =
        game_command_from_payload(ConnId(1), &payload, ProtocolVersion::V1098).expect("parse");
    match cmd {
        GameCommand::Game { packet, .. } => {
            assert!(matches!(packet, GamePacket::EnterGame));
        }
        _ => panic!("expected Game"),
    }
}

#[test]
fn parses_move_north() {
    let payload = [0x65u8];
    let cmd =
        game_command_from_payload(ConnId(1), &payload, ProtocolVersion::V1098).expect("parse");
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

/// 772 rejects 1098-only opcodes (e.g. market `0xF4`) — version-keyed dispatch (Phase A2).
#[test]
fn rejects_1098_only_opcode_on_772() {
    let payload = [0xF4u8]; // MARKET_LEAVE — absent in 7.72
    assert!(game_command_from_payload(ConnId(1), &payload, ProtocolVersion::V772).is_err());
    // Same opcode is valid on 1098.
    assert!(game_command_from_payload(ConnId(1), &payload, ProtocolVersion::V1098).is_ok());
}

/// 772-only rule-violation opcode `0x9B` parses on 772 but not 1098.
#[test]
fn accepts_772_rule_violation_opcode() {
    let payload = [0x9Bu8];
    // 1098 does not dispatch 0x9B (uses 0xF2 instead).
    assert!(game_command_from_payload(ConnId(1), &payload, ProtocolVersion::V1098).is_err());
    // 772 dispatches it; payload is empty so the structured parse may still fail, but the opcode is
    // recognized as supported (not the "not valid for protocol" rejection).
    let err = game_command_from_payload(ConnId(1), &payload, ProtocolVersion::V772)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(
        !err.contains("not valid for protocol"),
        "0x9B should be a supported 772 opcode, got: {err}"
    );
}
