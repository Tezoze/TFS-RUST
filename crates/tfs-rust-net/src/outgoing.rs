//! Server → client game packet builders.
// C++ reference (this repo): `src/protocolgame.cpp` — `ProtocolGame::send*`.

use tfs_rust_common::protocol_opcodes::server;
use tfs_rust_common::Position;

use crate::NetworkMessage;

#[inline]
pub fn send_ping() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(server::SEND_PING);
    m
}

#[inline]
pub fn send_ping_back() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(server::SEND_PING_BACK);
    m
}

/// Magic effect at position (`sendMagicEffect`).
pub fn send_magic_effect(pos: Position, effect: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(server::MAGIC_EFFECT);
    m.write_position(&pos);
    m.write_u8(effect);
    m
}

/// Creature health bar update (`sendCreatureHealth`).
pub fn send_creature_health(creature_id: u32, health_percent: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(server::CREATURE_HEALTH);
    m.write_u32(creature_id);
    m.write_u8(health_percent);
    m
}

/// Simple text message (`sendTextMessage` default branch: type + string only).
// C++ reference: `src/protocolgame.cpp` `ProtocolGame::sendTextMessage` (lines ~1583–1618).
pub fn send_text_message(message_type: u8, text: &str) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(server::TEXT_MESSAGE);
    m.write_u8(message_type);
    m.write_string(text);
    m
}

/// OTCv8 extended opcode to client (`sendExtendedOpcode`).
pub fn send_extended_opcode(ext_opcode: u8, buffer: &str) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(server::EXTENDED_OPCODE);
    m.write_u8(ext_opcode);
    m.write_string(buffer);
    m
}

/// OTCv8 feature list (`ProtocolGame::sendFeatures` when `otclientV8`).
// C++ reference: `src/protocolgame.cpp` lines ~3475–3495 (`uint16_t` count, then pairs: feature id `u8`, enabled `u8`).
pub fn send_otcv8_features(features: &[(u8, bool)]) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(server::OTCV8_FEATURES);
    let n = features.len().min(u16::MAX as usize) as u16;
    m.write_u16(n);
    for &(id, enabled) in features.iter().take(u16::MAX as usize) {
        m.write_u8(id);
        m.write_u8(u8::from(enabled));
    }
    m
}
