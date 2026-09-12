//! Client → server 772 first packets and game opcodes.
//!
//! C++ reference: `gameserver/src/protocollogin.cpp` `onRecvFirstMessage` (login
//! `0x01` + OS + ver + skip 12 + RSA); `protocolgame.cpp` `onRecvFirstMessage`
//! (game `0x0A` + OS + ver + RSA). Pack surface: `tfs-rust-net` `game_first_packet.rs`.

use anyhow::{Result, anyhow};
use num_bigint_dig::BigUint;
use tfs_rust_common::Position;
use tfs_rust_common::protocol_opcodes::client;
use tfs_rust_net::rsa;

/// Stock 7.72 client OS (`OperatingSystem_t` Windows), not `CLIENTOS_OTCLIENT_LINUX`.
pub const STOCK_CLIENT_OS: u16 = 1;
/// Wire protocol version advertised in the first-packet prelude.
pub const PROTOCOL_772: u16 = 772;

const LOGIN_PROTO_ID: u8 = 0x01;
const GAME_PROTO_ID: u8 = 0x0A;
const LOGIN_SKIP: usize = 12;
const RSA_LEN: usize = 128;
const XTEA_OFF: usize = 1;
const CREDS_OFF: usize = 17;

/// `[u16 LE size][body]` TCP frame (`game_frame.rs`).
pub fn wrap_tcp_frame(body: &[u8]) -> Vec<u8> {
    let n = body.len() as u16;
    let mut out = Vec::with_capacity(2 + body.len());
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(body);
    out
}

/// Login-port first body (no outer size): `0x01` + OS + 772 + 12 skip + RSA[128].
pub fn encode_login_first(
    os: u16,
    version: u16,
    xtea: &[u32; 4],
    account: u32,
    password: &str,
    n: &BigUint,
    e: &BigUint,
) -> Result<Vec<u8>> {
    let mut creds = Vec::new();
    creds.extend_from_slice(&account.to_le_bytes());
    put_string(&mut creds, password);
    let (cipher, tail) = rsa_block(xtea, &creds, n, e)?;
    let mut body = Vec::with_capacity(5 + LOGIN_SKIP + RSA_LEN + tail.len());
    body.push(LOGIN_PROTO_ID);
    body.extend_from_slice(&os.to_le_bytes());
    body.extend_from_slice(&version.to_le_bytes());
    body.extend_from_slice(&[0u8; LOGIN_SKIP]);
    body.extend_from_slice(&cipher);
    body.extend_from_slice(&tail);
    Ok(body)
}

/// Game-port first body: `0x0A` + OS + 772 + RSA[128].
pub fn encode_game_first(
    os: u16,
    version: u16,
    xtea: &[u32; 4],
    gm: u8,
    account: u32,
    character: &str,
    password: &str,
    n: &BigUint,
    e: &BigUint,
) -> Result<Vec<u8>> {
    let mut creds = Vec::new();
    creds.push(gm);
    creds.extend_from_slice(&account.to_le_bytes());
    put_string(&mut creds, character);
    put_string(&mut creds, password);
    let (cipher, tail) = rsa_block(xtea, &creds, n, e)?;
    let mut body = Vec::with_capacity(5 + RSA_LEN + tail.len());
    body.push(GAME_PROTO_ID);
    body.extend_from_slice(&os.to_le_bytes());
    body.extend_from_slice(&version.to_le_bytes());
    body.extend_from_slice(&cipher);
    body.extend_from_slice(&tail);
    Ok(body)
}

fn rsa_block(
    xtea: &[u32; 4],
    creds: &[u8],
    n: &BigUint,
    e: &BigUint,
) -> Result<([u8; RSA_LEN], Vec<u8>)> {
    let mut plain = [0xFFu8; RSA_LEN];
    plain[0] = 0x00;
    for (i, w) in xtea.iter().enumerate() {
        let off = XTEA_OFF + i * 4;
        plain[off..off + 4].copy_from_slice(&w.to_le_bytes());
    }
    let room = RSA_LEN - CREDS_OFF;
    let (head, tail) = if creds.len() <= room {
        (creds, &[][..])
    } else {
        creds.split_at(room)
    };
    plain[CREDS_OFF..CREDS_OFF + head.len()].copy_from_slice(head);
    let cipher = rsa::encrypt(&plain, n, e).map_err(|err| anyhow!("{err}"))?;
    Ok((cipher, tail.to_vec()))
}

fn put_string(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(&(s.len() as u16).to_le_bytes());
    buf.extend_from_slice(s.as_bytes());
}

fn put_position(buf: &mut Vec<u8>, pos: Position) {
    buf.extend_from_slice(&pos.x.to_le_bytes());
    buf.extend_from_slice(&pos.y.to_le_bytes());
    buf.push(pos.z);
}

/// Single-byte walk (`MOVE_NORTH`…`MOVE_WEST`).
pub fn encode_walk(opcode: u8) -> Vec<u8> {
    vec![opcode]
}

/// `SAY` `0x96` + speak class 1 (`TALKTYPE_SAY`) + text.
pub fn encode_say(text: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(client::SAY);
    buf.push(1);
    put_string(&mut buf, text);
    buf
}

/// `ATTACK` `0xA1` + creature id.
pub fn encode_attack(creature_id: u32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5);
    buf.push(client::ATTACK);
    buf.extend_from_slice(&creature_id.to_le_bytes());
    buf
}

/// `USE_ITEM_EX` `0x83`.
pub fn encode_use_item_ex(
    from: Position,
    from_sprite: u16,
    from_stack: u8,
    to: Position,
    to_sprite: u16,
    to_stack: u8,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(client::USE_ITEM_EX);
    put_position(&mut buf, from);
    buf.extend_from_slice(&from_sprite.to_le_bytes());
    buf.push(from_stack);
    put_position(&mut buf, to);
    buf.extend_from_slice(&to_sprite.to_le_bytes());
    buf.push(to_stack);
    buf
}

/// Inventory cylinder (`x = 0xFFFF`) source for a rune in slot `slot`.
pub fn inventory_pos(slot: u16) -> Position {
    Position::new(0xFFFF, slot, 0)
}

/// `LOOK_AT` `0x8C`.
pub fn encode_look_at(pos: Position, sprite_id: u16, stack_pos: u8) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(client::LOOK_AT);
    put_position(&mut buf, pos);
    buf.extend_from_slice(&sprite_id.to_le_bytes());
    buf.push(stack_pos);
    buf
}

/// Reply to server ping: `0x1D` → client `PING_BACK`; `0x1E` → client `PING`.
pub fn encode_ping_reply(server_opcode: u8) -> Vec<u8> {
    match server_opcode {
        0x1D => vec![client::PING_BACK],
        _ => vec![client::PING],
    }
}

pub fn walk_north() -> Vec<u8> {
    encode_walk(client::MOVE_NORTH)
}

pub fn walk_south() -> Vec<u8> {
    encode_walk(client::MOVE_SOUTH)
}

pub fn walk_east() -> Vec<u8> {
    encode_walk(client::MOVE_EAST)
}

pub fn walk_west() -> Vec<u8> {
    encode_walk(client::MOVE_WEST)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfs_rust_common::{ProtocolCaps, ProtocolVersion};
    use tfs_rust_net::game_first_packet::{
        FirstClientPacket, LoginIdentity, parse_first_client_packet,
    };
    use tfs_rust_net::rsa::{private_key_from_pkcs1_pem, public_parts};

    fn workspace_key() -> ::rsa::RsaPrivateKey {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../key.pem");
        let pem = std::fs::read_to_string(&path).expect("read key.pem");
        private_key_from_pkcs1_pem(&pem).expect("pem")
    }

    #[test]
    fn login_first_parses_on_server() {
        let key = workspace_key();
        let (n, e) = public_parts(&key);
        let xtea = [0xA1A1_A1A1, 0xB2B2_B2B2, 0xC3C3_C3C3, 0xD4D4_D4D4];
        let body = encode_login_first(STOCK_CLIENT_OS, PROTOCOL_772, &xtea, 1, "1", &n, &e)
            .expect("encode");
        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        match parse_first_client_packet(&body, &key, &caps).expect("parse") {
            FirstClientPacket::Login {
                xtea_key,
                identity,
                password,
                operating_system,
                ..
            } => {
                assert_eq!(xtea_key, xtea);
                assert_eq!(identity, LoginIdentity::AccountNumber(1));
                assert_eq!(password, "1");
                assert_eq!(operating_system, STOCK_CLIENT_OS);
            }
            other => panic!("expected Login, got {other:?}"),
        }
    }

    #[test]
    fn game_first_parses_on_server() {
        let key = workspace_key();
        let (n, e) = public_parts(&key);
        let xtea = [1u32, 2, 3, 4];
        let body = encode_game_first(
            STOCK_CLIENT_OS,
            PROTOCOL_772,
            &xtea,
            0,
            1,
            "Test",
            "1",
            &n,
            &e,
        )
        .expect("encode");
        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        match parse_first_client_packet(&body, &key, &caps).expect("parse") {
            FirstClientPacket::Game(g) => {
                assert_eq!(g.xtea_key, xtea);
                assert_eq!(g.identity, LoginIdentity::AccountNumber(1));
                assert_eq!(g.character_name, "Test");
                assert_eq!(g.password, "1");
            }
            other => panic!("expected Game, got {other:?}"),
        }
    }
}
