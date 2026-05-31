//! Login server → client payload bytes (before XTEA + Adler framing). `ProtocolLogin::getCharacterList`, `disconnectClient`.
//
// C++ reference (shape is capability-gated, `docs/PROTOCOL_VERSIONING.md` §2.2):
// - 1098: repo-root `src/protocollogin.cpp` — `0x28` session key + world-table char list + `u8` flag
//   + `u32` premium timestamp; `disconnectClient` uses `0x0B` (`version >= 1076`).
// - 772:  `gameserver/src/protocollogin.cpp` `getCharacterList` — no session key; per-char
//   `name + serverName + u32 ip + u16 port`; premium as `u16` days; `disconnectClient` uses `0x0A`.

use tfs_rust_common::ProtocolCaps;

use crate::message::NetworkMessage;

const LOGIN_ERR_772: u8 = 0x0A;
const LOGIN_ERR_NEW: u8 = 0x0B;
const LOGIN_MOTD: u8 = 0x14;
const LOGIN_SESSION: u8 = 0x28;
const LOGIN_CHAR_LIST: u8 = 0x64;

/// Seconds per day, for the 772 `premiumDaysLeft` computation (`getCharacterList`).
const SECONDS_PER_DAY: i64 = 86_400;

/// `disconnectClient` error packet, capability-gated: `0x0B` (1098, `version >= 1076`) vs `0x0A` (772).
//
// C++ ref: repo-root `src/protocollogin.cpp` `disconnectClient` (`0x0B`); `gameserver/src/protocollogin.cpp`
// `disconnectClient` (`0x0A`).
pub fn build_login_error(caps: &ProtocolCaps, message: &str) -> Vec<u8> {
    let opcode = if caps.session_key_login {
        LOGIN_ERR_NEW
    } else {
        LOGIN_ERR_772
    };
    let mut m = NetworkMessage::new();
    m.write_u8(opcode);
    m.write_string(message);
    m.into_bytes()
}

/// `disconnectClient` for `version >= 1076` (`0x0B` + string). 1098-only; kept for callers that are
/// already 1098-specific.
pub fn build_login_error_new(message: &str) -> Vec<u8> {
    let mut m = NetworkMessage::new();
    m.write_u8(LOGIN_ERR_NEW);
    m.write_string(message);
    m.into_bytes()
}

/// Inputs for the character-list success packet, era-neutral. Each codec writes the subset its wire
/// format needs (`docs/PROTOCOL_VERSIONING.md` §4.2 "version-neutral input").
pub struct LoginSuccess<'a> {
    pub motd_num: u32,
    pub motd: &'a str,
    /// Account name (1098 session key only; unused for 772).
    pub account_name: &'a str,
    /// Account password (1098 session key only; unused for 772).
    pub password: &'a str,
    /// 2FA token (1098 session key only; unused for 772).
    pub token: &'a str,
    /// Session-key validity window (1098 only; unused for 772).
    pub ticks: u32,
    pub server_name: &'a str,
    pub ip: &'a str,
    pub game_port: u16,
    pub characters: &'a [String],
    pub premium_ends_at: i64,
    pub free_premium: bool,
    pub now_unix: i64,
}

/// Build the character-list success packet for the active era.
pub fn build_login_success(caps: &ProtocolCaps, s: &LoginSuccess<'_>) -> Vec<u8> {
    if caps.session_key_login {
        build_login_success_1098(s)
    } else {
        build_login_success_772(s)
    }
}

/// 1098 success blob: optional MOTD, `0x28` session key, world-table char list, `u8`+`u32` premium.
//
// C++ ref: repo-root `src/protocollogin.cpp` `getCharacterList`.
fn build_login_success_1098(s: &LoginSuccess<'_>) -> Vec<u8> {
    let mut m = NetworkMessage::new();
    if !s.motd.is_empty() {
        m.write_u8(LOGIN_MOTD);
        m.write_string(&format!("{}\n{}", s.motd_num, s.motd));
    }
    m.write_u8(LOGIN_SESSION);
    m.write_string(&format!(
        "{}\n{}\n{}\n{}",
        s.account_name, s.password, s.token, s.ticks
    ));
    m.write_u8(LOGIN_CHAR_LIST);
    m.write_u8(1);
    m.write_u8(0);
    m.write_string(s.server_name);
    m.write_string(s.ip);
    m.write_u16(s.game_port);
    m.write_u8(0);
    let n = s.characters.len().min(255);
    m.write_u8(n as u8);
    for name in s.characters.iter().take(n) {
        m.write_u8(0);
        m.write_string(name);
    }
    m.write_u8(0);
    if s.free_premium {
        m.write_u8(1);
        m.write_u32(0);
    } else {
        m.write_u8(if s.premium_ends_at > s.now_unix { 1 } else { 0 });
        m.write_u32(s.premium_ends_at as u32);
    }
    m.into_bytes()
}

/// 772 success blob: optional MOTD, `0x64` char list (per-char `name + serverName + u32 ip +
/// u16 port`), then premium as `u16` days. No session key.
//
// C++ ref: `gameserver/src/protocollogin.cpp` `getCharacterList`. The IP is the resolved
// little-endian `inet_addr` of the configured world IP; an unparseable string falls back to 0.
// PROTOCOL: verify against a live 7.72 client capture in A6.
fn build_login_success_772(s: &LoginSuccess<'_>) -> Vec<u8> {
    let mut m = NetworkMessage::new();
    if !s.motd.is_empty() {
        m.write_u8(LOGIN_MOTD);
        m.write_string(&format!("{}\n{}", s.motd_num, s.motd));
    }
    m.write_u8(LOGIN_CHAR_LIST);
    let n = s.characters.len().min(255);
    m.write_u8(n as u8);
    let ip_le = ipv4_inet_addr(s.ip);
    for name in s.characters.iter().take(n) {
        m.write_string(name);
        m.write_string(s.server_name);
        m.write_u32(ip_le);
        m.write_u16(s.game_port);
    }
    m.write_u16(premium_days_left_772(s.free_premium, s.premium_ends_at, s.now_unix));
    m.into_bytes()
}

/// 772 `inet_addr` semantics: dotted-quad → 32-bit value in network byte order, which the C++ code
/// then writes with `add<uint32_t>` (host little-endian). We replicate the resulting on-wire bytes
/// by storing the octets in `a.b.c.d` order as a little-endian `u32` (= `inet_addr` on a LE host).
fn ipv4_inet_addr(ip: &str) -> u32 {
    let mut octets = [0u8; 4];
    let mut count = 0;
    for (i, part) in ip.split('.').enumerate() {
        if i >= 4 {
            return 0;
        }
        match part.parse::<u8>() {
            Ok(v) => octets[i] = v,
            Err(_) => return 0,
        }
        count = i + 1;
    }
    if count != 4 {
        return 0;
    }
    u32::from_le_bytes(octets)
}

/// 772 premium representation: `u16` days remaining (`getCharacterList`). Free-premium servers send
/// `u16::MAX`; otherwise `ceil`-ish `++premiumDaysLeft` from the C++ (floor of seconds/day, +1).
fn premium_days_left_772(free_premium: bool, premium_ends_at: i64, now_unix: i64) -> u16 {
    if free_premium {
        return u16::MAX;
    }
    if premium_ends_at > now_unix {
        let days = (premium_ends_at - now_unix).max(0) / SECONDS_PER_DAY;
        let with_lead = days.saturating_add(1);
        with_lead.min(u16::MAX as i64) as u16
    } else {
        0
    }
}

/// Legacy 1098-only entry point. Prefer [`build_login_success`] with [`LoginSuccess`].
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
    build_login_success_1098(&LoginSuccess {
        motd_num,
        motd,
        account_name,
        password,
        token,
        ticks,
        server_name,
        ip,
        game_port,
        characters,
        premium_ends_at,
        free_premium,
        now_unix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfs_rust_common::ProtocolVersion;

    #[test]
    fn inet_addr_le_bytes() {
        // 127.0.0.1 → octets in a.b.c.d order as LE u32.
        assert_eq!(ipv4_inet_addr("127.0.0.1").to_le_bytes(), [127, 0, 0, 1]);
        assert_eq!(ipv4_inet_addr("192.168.1.50").to_le_bytes(), [192, 168, 1, 50]);
        assert_eq!(ipv4_inet_addr("not-an-ip"), 0);
        assert_eq!(ipv4_inet_addr("1.2.3"), 0);
    }

    #[test]
    fn premium_days_772() {
        assert_eq!(premium_days_left_772(true, 0, 0), u16::MAX);
        assert_eq!(premium_days_left_772(false, 0, 100), 0);
        // ~2 days remaining → floor(2d/1d)+1 = 3 (matches C++ `++premiumDaysLeft`).
        let now = 1_000_000;
        let ends = now + 2 * SECONDS_PER_DAY;
        assert_eq!(premium_days_left_772(false, ends, now), 3);
    }

    #[test]
    fn login_error_opcode_is_version_keyed() {
        let c772 = ProtocolCaps::for_version(ProtocolVersion::V772);
        let c1098 = ProtocolCaps::for_version(ProtocolVersion::V1098);
        assert_eq!(build_login_error(&c772, "x")[0], LOGIN_ERR_772);
        assert_eq!(build_login_error(&c1098, "x")[0], LOGIN_ERR_NEW);
    }

    #[test]
    fn login_1098_success_byte_identical_to_legacy() {
        let chars = vec!["Alpha".to_string(), "Beta".to_string()];
        let legacy = build_login_success_packet(
            7, "motd", "acc", "pw", "tok", 99, "World", "127.0.0.1", 7172, &chars, 0, true, 0,
        );
        let caps = ProtocolCaps::for_version(ProtocolVersion::V1098);
        let neu = build_login_success(
            &caps,
            &LoginSuccess {
                motd_num: 7,
                motd: "motd",
                account_name: "acc",
                password: "pw",
                token: "tok",
                ticks: 99,
                server_name: "World",
                ip: "127.0.0.1",
                game_port: 7172,
                characters: &chars,
                premium_ends_at: 0,
                free_premium: true,
                now_unix: 0,
            },
        );
        assert_eq!(legacy, neu);
    }

    #[test]
    fn login_772_success_layout() {
        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        let chars = vec!["Hero".to_string()];
        let bytes = build_login_success(
            &caps,
            &LoginSuccess {
                motd_num: 0,
                motd: "",
                account_name: "",
                password: "",
                token: "",
                ticks: 0,
                server_name: "Aus",
                ip: "127.0.0.1",
                game_port: 7172,
                characters: &chars,
                premium_ends_at: 0,
                free_premium: true,
                now_unix: 0,
            },
        );
        // No MOTD, no 0x28; char-list opcode + count + per-char (name, server, ip, port) + u16 premium.
        let mut m = NetworkMessage::new();
        m.write_u8(LOGIN_CHAR_LIST);
        m.write_u8(1);
        m.write_string("Hero");
        m.write_string("Aus");
        m.write_u32(u32::from_le_bytes([127, 0, 0, 1]));
        m.write_u16(7172);
        m.write_u16(u16::MAX);
        assert_eq!(bytes, m.into_bytes());
    }
}
