//! Login server → client payload bytes (before XTEA + Adler framing). `ProtocolLogin::getCharacterList`, `disconnectClient`.
// C++ reference: `src/protocollogin.cpp`.

use crate::message::NetworkMessage;

const LOGIN_ERR_NEW: u8 = 0x0B;
const LOGIN_MOTD: u8 = 0x14;
const LOGIN_SESSION: u8 = 0x28;
const LOGIN_CHAR_LIST: u8 = 0x64;

/// `disconnectClient` for `version >= 1076` (`0x0B` + string).
pub fn build_login_error_new(message: &str) -> Vec<u8> {
    let mut m = NetworkMessage::new();
    m.write_u8(LOGIN_ERR_NEW);
    m.write_string(message);
    m.into_bytes()
}

/// Full success blob: optional MOTD, session key, character list, premium tail (`getCharacterList`).
#[allow(clippy::too_many_arguments)]
pub fn build_login_success_packet(
    motd_num: u32,
    motd: &str,
    account_name: &str,
    password: &str,
    token: &str,
    ticks: u32,
    server_name: &str,
    ip: &str,
    game_port: u16,
    characters: &[String],
    premium_ends_at: i64,
    free_premium: bool,
    now_unix: i64,
) -> Vec<u8> {
    let mut m = NetworkMessage::new();
    if !motd.is_empty() {
        m.write_u8(LOGIN_MOTD);
        m.write_string(&format!("{motd_num}\n{motd}"));
    }
    m.write_u8(LOGIN_SESSION);
    m.write_string(&format!("{account_name}\n{password}\n{token}\n{ticks}"));
    m.write_u8(LOGIN_CHAR_LIST);
    m.write_u8(1);
    m.write_u8(0);
    m.write_string(server_name);
    m.write_string(ip);
    m.write_u16(game_port);
    m.write_u8(0);
    let n = characters.len().min(255);
    m.write_u8(n as u8);
    for name in characters.iter().take(n) {
        m.write_u8(0);
        m.write_string(name);
    }
    m.write_u8(0);
    if free_premium {
        m.write_u8(1);
        m.write_u32(0);
    } else {
        m.write_u8(if premium_ends_at > now_unix { 1 } else { 0 });
        m.write_u32(premium_ends_at as u32);
    }
    m.into_bytes()
}
