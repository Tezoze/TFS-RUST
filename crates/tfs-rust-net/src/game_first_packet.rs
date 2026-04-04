//! First TCP message after checksum: **`ProtocolGame::onRecvFirstMessage`** and/or **`ProtocolLogin::onRecvFirstMessage`**.
// C++ reference: `src/protocolgame.cpp`, `src/protocollogin.cpp`.

use rsa::RsaPrivateKey;
use tfs_rust_common::error::{Result, TfsRustError};

use crate::adler::adler_checksum;
use crate::rsa::decrypt as rsa_decrypt_block;

pub type XteaKey = [u32; 4];

#[derive(Debug, Clone)]
pub struct GameFirstParsed {
    pub xtea_key: XteaKey,
    pub account_name: String,
    pub password: String,
    pub token: String,
    pub token_time: u32,
    pub character_name: String,
    pub challenge_ts: u32,
    pub challenge_rand: u8,
}

/// Parsed first message: **game** (session + character + challenge) or **login** (account + password only).
#[derive(Debug, Clone)]
pub enum FirstClientPacket {
    Game(GameFirstParsed),
    Login {
        xtea_key: XteaKey,
        account: String,
        password: String,
    },
}

/// Try RSA at offsets used by OTClient / TFS (`src/protocollogin.cpp`, `src/protocolgame.cpp`).
pub fn parse_first_client_packet(
    body: &[u8],
    private_key: &RsaPrivateKey,
) -> Result<FirstClientPacket> {
    // Smallest layout: Adler checksum (4) + game prelude (u16 OS + u16 version + 7 skipped)
    // then RSA block — `ProtocolGame::onRecvFirstMessage` / `src/protocolgame.cpp`.
    const MIN_LEN: usize = 4 + 11 + 128;
    if body.len() < MIN_LEN {
        return Err(TfsRustError::Protocol(format!(
            "first packet too short: {} bytes (need {})",
            body.len(),
            MIN_LEN
        )));
    }
    let recv = u32::from_le_bytes(body[0..4].try_into().unwrap());
    let expected = adler_checksum(&body[4..]);
    if recv != expected {
        return Err(TfsRustError::Protocol(format!(
            "first packet checksum mismatch: recv=0x{recv:08x} expected=0x{expected:08x}"
        )));
    }

    // OTClient 10.98 game: Adler (4) + opcode 0x0A (1) + OS u16 + version u16 + client ver u32 +
    // content revision u16 + preview u8 = 16, then RSA. Login uses 26 (extra spr/pic u32s + padding u16).
    const PROTO_1098_GAME_RSA_OFF: usize = 16;
    const PROTO_1098_LOGIN_RSA_OFF: usize = 26;
    const GAME_RSA_OFF: usize = 15;
    const LOGIN_RSA_OFF_GE_971: usize = 25;
    const LOGIN_RSA_OFF_LT_971: usize = 20;

    for &off in &[
        PROTO_1098_GAME_RSA_OFF,
        PROTO_1098_LOGIN_RSA_OFF,
        GAME_RSA_OFF,
        LOGIN_RSA_OFF_GE_971,
        LOGIN_RSA_OFF_LT_971,
    ] {
        if body.len() < off + 128 {
            continue;
        }
        let block: &[u8; 128] = match body[off..off + 128].try_into() {
            Ok(b) => b,
            Err(_) => continue,
        };
        let rsa_plain = match rsa_decrypt_block(block, private_key) {
            Ok(p) if !p.is_empty() && p[0] == 0 => p,
            _ => continue,
        };
        let rsa_arr: &[u8; 128] = match rsa_plain.as_slice().try_into() {
            Ok(a) => a,
            Err(_) => continue,
        };

        let key = xtea_key_from_plain(rsa_arr);

        let tail = &body[off + 128..];

        let parsed = match off {
            PROTO_1098_GAME_RSA_OFF | GAME_RSA_OFF => parse_game_first(key, rsa_arr, tail),
            PROTO_1098_LOGIN_RSA_OFF | LOGIN_RSA_OFF_GE_971 | LOGIN_RSA_OFF_LT_971 => {
                parse_login_first(key, rsa_arr)
            }
            _ => unreachable!(),
        };
        if let Ok(v) = parsed {
            return Ok(v);
        }
    }

    Err(TfsRustError::Protocol(
        "no valid RSA block (game offset 16/15 or login 26/25/20)".into(),
    ))
}

/// Game-only first packet (fails if login layout matched).
pub fn parse_first_game_packet(
    body: &[u8],
    private_key: &RsaPrivateKey,
) -> Result<GameFirstParsed> {
    match parse_first_client_packet(body, private_key)? {
        FirstClientPacket::Game(g) => Ok(g),
        FirstClientPacket::Login { .. } => {
            Err(TfsRustError::Protocol("expected game first packet".into()))
        }
    }
}

fn xtea_key_from_plain(rsa_plain: &[u8; 128]) -> XteaKey {
    [
        u32::from_le_bytes(rsa_plain[1..5].try_into().unwrap()),
        u32::from_le_bytes(rsa_plain[5..9].try_into().unwrap()),
        u32::from_le_bytes(rsa_plain[9..13].try_into().unwrap()),
        u32::from_le_bytes(rsa_plain[13..17].try_into().unwrap()),
    ]
}

fn parse_login_first(key: XteaKey, rsa_plain: &[u8; 128]) -> Result<FirstClientPacket> {
    let mut s = &rsa_plain[17..128];
    let account = read_string(&mut s)?;
    let password = read_string(&mut s)?;
    Ok(FirstClientPacket::Login {
        xtea_key: key,
        account,
        password,
    })
}

fn parse_game_first(key: XteaKey, rsa_plain: &[u8; 128], tail: &[u8]) -> Result<FirstClientPacket> {
    let mut stream = Vec::new();
    stream.extend_from_slice(&rsa_plain[17..128]);
    stream.extend_from_slice(tail);
    let mut s = stream.as_slice();

    if s.is_empty() {
        return Err(TfsRustError::Protocol("missing gamemaster flag".into()));
    }
    let _gm = s[0];
    s = &s[1..];

    let session = read_string(&mut s)?;
    let character_name = read_string(&mut s)?;
    if s.len() < 5 {
        return Err(TfsRustError::Protocol(
            "missing login challenge fields".into(),
        ));
    }
    let challenge_ts = read_u32(&mut s)?;
    let challenge_rand = read_u8(&mut s)?;

    let parts: Vec<&str> = session.splitn(4, '\n').collect();
    if parts.len() != 4 {
        return Err(TfsRustError::Protocol(
            "session key must have 4 fields".into(),
        ));
    }
    let account_name = parts[0].to_string();
    let password = parts[1].to_string();
    let token = parts[2].to_string();
    let token_time: u32 = parts[3]
        .parse()
        .map_err(|_| TfsRustError::Protocol("invalid token time in session key".into()))?;

    Ok(FirstClientPacket::Game(GameFirstParsed {
        xtea_key: key,
        account_name,
        password,
        token,
        token_time,
        character_name,
        challenge_ts,
        challenge_rand,
    }))
}

fn read_u16(data: &mut &[u8]) -> Result<u16> {
    if data.len() < 2 {
        return Err(TfsRustError::Protocol("EOF u16".into()));
    }
    let v = u16::from_le_bytes([data[0], data[1]]);
    *data = &data[2..];
    Ok(v)
}

fn read_u32(data: &mut &[u8]) -> Result<u32> {
    if data.len() < 4 {
        return Err(TfsRustError::Protocol("EOF u32".into()));
    }
    let v = u32::from_le_bytes(data[0..4].try_into().unwrap());
    *data = &data[4..];
    Ok(v)
}

fn read_u8(data: &mut &[u8]) -> Result<u8> {
    if data.is_empty() {
        return Err(TfsRustError::Protocol("EOF u8".into()));
    }
    let b = data[0];
    *data = &data[1..];
    Ok(b)
}

fn read_string(data: &mut &[u8]) -> Result<String> {
    let len = read_u16(data)? as usize;
    if data.len() < len {
        return Err(TfsRustError::Protocol("EOF string".into()));
    }
    let out = String::from_utf8_lossy(&data[..len]).into_owned();
    *data = &data[len..];
    Ok(out)
}
