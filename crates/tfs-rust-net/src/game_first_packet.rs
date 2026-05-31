//! First TCP message after checksum: **`ProtocolGame::onRecvFirstMessage`** and/or **`ProtocolLogin::onRecvFirstMessage`**.
//
// C++ reference (login shape is capability-gated, `docs/PROTOCOL_VERSIONING.md` §2.2):
// - 1098: repo-root `src/protocolgame.cpp`, `src/protocollogin.cpp` — account **name** string + session key.
// - 772:  `gameserver/src/protocolgame.cpp` `onRecvFirstMessage` (gm flag + `u32` accountNumber +
//         char + password) and `gameserver/src/protocollogin.cpp` `onRecvFirstMessage`
//         (`u32` accountNumber + password). 772 has no Adler checksum and no session key.

use rsa::RsaPrivateKey;
use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_common::ProtocolCaps;

use crate::adler::adler_checksum;
use crate::rsa::decrypt as rsa_decrypt_block;

pub type XteaKey = [u32; 4];

/// Account identity carried by the first packet. Capability-gated (`ProtocolCaps::account_name_login`):
/// 1098 uses an account **name** string; 772 uses a numeric **account number** (`accounts.id`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginIdentity {
    /// 1098 / TFS 1.4.2 — account name string (repo-root `src/protocollogin.cpp`).
    AccountName(String),
    /// 7.72 — numeric account number (`gameserver/src/protocollogin.cpp` `accountNumber`).
    AccountNumber(u32),
}

impl LoginIdentity {
    /// Display form for logs (name as-is, number rendered decimal).
    pub fn as_display(&self) -> String {
        match self {
            LoginIdentity::AccountName(n) => n.clone(),
            LoginIdentity::AccountNumber(n) => n.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameFirstParsed {
    pub xtea_key: XteaKey,
    /// First u16 in game prelude (`protocolgame.cpp` `onRecvFirstMessage`); `OperatingSystem_t`.
    pub operating_system: u16,
    /// Account name (1098) or account number (772).
    pub identity: LoginIdentity,
    pub password: String,
    /// 2FA token (1098 session key only); empty for 772.
    pub token: String,
    /// Token validity window (1098 session key only); `0` for 772.
    pub token_time: u32,
    pub character_name: String,
    /// Echoed `0x1F` challenge timestamp (1098 only — `caps.prelogin_challenge`); `0` for 772.
    pub challenge_ts: u32,
    /// Echoed `0x1F` challenge random byte (1098 only); `0` for 772.
    pub challenge_rand: u8,
    /// `0` if the `"OTCv8"` probe was not present; else build number (253, 260, …).
    pub otclient_v8: u16,
}

/// Parsed first message: **game** (session + character + challenge) or **login** (account + password only).
#[derive(Debug, Clone)]
pub enum FirstClientPacket {
    Game(GameFirstParsed),
    Login {
        xtea_key: XteaKey,
        identity: LoginIdentity,
        password: String,
        operating_system: u16,
        otclient_v8: u16,
    },
}

/// Which protocol shape an RSA-offset candidate decodes into.
#[derive(Clone, Copy)]
enum FirstKind {
    Game,
    Login,
}

/// A candidate framing: where the 128-byte RSA block starts and where the `OperatingSystem_t`
/// `u16` lives, both relative to the body returned by `read_sized_payload`.
#[derive(Clone, Copy)]
struct FrameCandidate {
    rsa_off: usize,
    os_off: usize,
    kind: FirstKind,
}

/// Try RSA at offsets used by the active era. Login shape (account name vs number, session key vs
/// inline credentials) follows `caps`. 1098 candidates and checksum handling are byte-identical to
/// the pre-A4 behavior; 772 adds checksum-free candidates.
pub fn parse_first_client_packet(
    body: &[u8],
    private_key: &RsaPrivateKey,
    caps: &ProtocolCaps,
) -> Result<FirstClientPacket> {
    // 1098 prefixes a 4-byte Adler checksum; 772 omits it (`docs/PROTOCOL_VERSIONING.md` §2.1).
    if caps.adler_checksum {
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
    } else if body.len() < 5 + 128 {
        return Err(TfsRustError::Protocol(format!(
            "first packet too short: {} bytes (772, need {})",
            body.len(),
            5 + 128
        )));
    }

    for cand in frame_candidates(caps) {
        if body.len() < cand.rsa_off + 128 {
            continue;
        }
        let block: &[u8; 128] = match body[cand.rsa_off..cand.rsa_off + 128].try_into() {
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
        let operating_system = read_u16_at(body, cand.os_off);
        let tail = &body[cand.rsa_off + 128..];

        let parsed = match cand.kind {
            FirstKind::Game => parse_game_first(key, rsa_arr, tail, operating_system, caps),
            FirstKind::Login => parse_login_first(key, rsa_arr, tail, operating_system, caps),
        };
        if let Ok(v) = parsed {
            return Ok(v);
        }
    }

    Err(TfsRustError::Protocol(
        "no valid RSA block for the configured protocol version".into(),
    ))
}

/// Game-only first packet (fails if login layout matched).
pub fn parse_first_game_packet(
    body: &[u8],
    private_key: &RsaPrivateKey,
    caps: &ProtocolCaps,
) -> Result<GameFirstParsed> {
    match parse_first_client_packet(body, private_key, caps)? {
        FirstClientPacket::Game(g) => Ok(g),
        FirstClientPacket::Login { .. } => {
            Err(TfsRustError::Protocol("expected game first packet".into()))
        }
    }
}

/// RSA-offset / OS-offset candidates for the active era.
//
// 1098 (`src/protocolgame.cpp` / `src/protocollogin.cpp`): checksum(4) prefix. Game prelude
//   = opcode `0x0A`(1) + OS(2) + version(2) + clientVer(4) + revision(2) + preview(1) = 16; the
//   971/pre-971 variants (15/25/20) are kept for older OTClient builds. Login prelude = OS(2) +
//   version(2) + spr/pic(8) + padding(2) → RSA at 26 (or 25/20 historical), OS at body[4].
// 772 (`gameserver/src/`): no checksum; `make_protocol` consumes the proto-id byte, so the body
//   keeps it. Game = proto-id `0x0A`(1) + OS(2) + version(2) → RSA at 5, OS at body[1]. Login =
//   proto-id `0x01`(1) + OS(2) + version(2) + skipBytes(12) → RSA at 17, OS at body[1].
//   PROTOCOL: confirm 772 offsets against live captures in A6 before flipping `clientVersion = 772`.
fn frame_candidates(caps: &ProtocolCaps) -> Vec<FrameCandidate> {
    if caps.adler_checksum {
        vec![
            FrameCandidate { rsa_off: 16, os_off: 5, kind: FirstKind::Game },
            FrameCandidate { rsa_off: 26, os_off: 4, kind: FirstKind::Login },
            FrameCandidate { rsa_off: 15, os_off: 5, kind: FirstKind::Game },
            FrameCandidate { rsa_off: 25, os_off: 4, kind: FirstKind::Login },
            FrameCandidate { rsa_off: 20, os_off: 4, kind: FirstKind::Login },
        ]
    } else {
        vec![
            FrameCandidate { rsa_off: 5, os_off: 1, kind: FirstKind::Game },
            FrameCandidate { rsa_off: 17, os_off: 1, kind: FirstKind::Login },
        ]
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

/// Read a little-endian `u16` at `off`, or `0` if out of range.
fn read_u16_at(body: &[u8], off: usize) -> u16 {
    if body.len() >= off + 2 {
        u16::from_le_bytes([body[off], body[off + 1]])
    } else {
        0
    }
}

/// `uint16_t len` + optional `"OTCv8"` + `uint16_t` build (`protocolgame.cpp` ~468–472, `protocollogin.cpp` ~243–249).
fn parse_otcv8_string_probe(s: &[u8]) -> Result<(u16, &[u8])> {
    if s.len() < 2 {
        return Ok((0, s));
    }
    let len = u16::from_le_bytes([s[0], s[1]]) as usize;
    if s.len() < 2 + len {
        return Err(TfsRustError::Protocol(
            "truncated OTCv8 / string probe after credentials".into(),
        ));
    }
    let chunk = &s[2..2 + len];
    let tail = &s[2 + len..];
    if len == 5 && chunk == b"OTCv8" {
        if tail.len() < 2 {
            return Err(TfsRustError::Protocol("truncated OTCv8 version u16".into()));
        }
        let ver = u16::from_le_bytes([tail[0], tail[1]]);
        return Ok((ver, &tail[2..]));
    }
    Ok((0, tail))
}

/// Assemble the post-RSA credential stream: `rsa_plain[17..]` (after the leading 0 + 16-byte key)
/// followed by the unencrypted tail.
fn credential_stream(rsa_plain: &[u8; 128], tail: &[u8]) -> Vec<u8> {
    let mut stream = Vec::with_capacity(111 + tail.len());
    stream.extend_from_slice(&rsa_plain[17..128]);
    stream.extend_from_slice(tail);
    stream
}

fn parse_login_first(
    key: XteaKey,
    rsa_plain: &[u8; 128],
    tail: &[u8],
    operating_system: u16,
    caps: &ProtocolCaps,
) -> Result<FirstClientPacket> {
    let stream = credential_stream(rsa_plain, tail);
    let (identity, password, otclient_v8) = parse_login_credentials(&stream, caps)?;
    Ok(FirstClientPacket::Login {
        xtea_key: key,
        identity,
        password,
        operating_system,
        otclient_v8,
    })
}

/// Login-port credential block. 1098 = `string account` + `string password`; 772 =
/// `u32 accountNumber` + `string password` (`gameserver/src/protocollogin.cpp` `onRecvFirstMessage`).
fn parse_login_credentials(
    stream: &[u8],
    caps: &ProtocolCaps,
) -> Result<(LoginIdentity, String, u16)> {
    let mut s = stream;
    let identity = read_identity(&mut s, caps)?;
    let password = read_string(&mut s)?;
    let (otclient_v8, _) = parse_otcv8_string_probe(s)?;
    Ok((identity, password, otclient_v8))
}

fn parse_game_first(
    key: XteaKey,
    rsa_plain: &[u8; 128],
    tail: &[u8],
    operating_system: u16,
    caps: &ProtocolCaps,
) -> Result<FirstClientPacket> {
    let stream = credential_stream(rsa_plain, tail);
    let parsed = parse_game_credentials(&stream, caps)?;
    Ok(FirstClientPacket::Game(GameFirstParsed {
        xtea_key: key,
        operating_system,
        identity: parsed.identity,
        password: parsed.password,
        token: parsed.token,
        token_time: parsed.token_time,
        character_name: parsed.character_name,
        challenge_ts: parsed.challenge_ts,
        challenge_rand: parsed.challenge_rand,
        otclient_v8: parsed.otclient_v8,
    }))
}

/// Decoded game-port credential block (era-neutral result).
struct GameCredentials {
    identity: LoginIdentity,
    password: String,
    token: String,
    token_time: u32,
    character_name: String,
    challenge_ts: u32,
    challenge_rand: u8,
    otclient_v8: u16,
}

/// Game-port credential block, capability-gated.
//
// 1098 (`src/protocolgame.cpp` `onRecvFirstMessage`): `u8 gm` + `string sessionKey` + `string
//   character` + `u32 challengeTimestamp` + `u8 challengeRandom` + OTCv8 probe. The session key is
//   `account\npassword\ntoken\ntime`.
// 772 (`gameserver/src/protocolgame.cpp` `onRecvFirstMessage`): `skipBytes(1)` gm flag +
//   `u32 accountNumber` + `string character` + `string password` + OTCv8 probe. No session key, no
//   challenge echo (`caps.prelogin_challenge = false`).
fn parse_game_credentials(stream: &[u8], caps: &ProtocolCaps) -> Result<GameCredentials> {
    let mut s = stream;

    if s.is_empty() {
        return Err(TfsRustError::Protocol("missing gamemaster flag".into()));
    }
    let _gm = s[0];
    s = &s[1..];

    if caps.session_key_login {
        // 1098: session key string carries account/password/token/time.
        let session = read_string(&mut s)?;
        let character_name = read_string(&mut s)?;
        let challenge_ts = read_u32(&mut s)?;
        let challenge_rand = read_u8(&mut s)?;
        let (otclient_v8, _) = parse_otcv8_string_probe(s)?;

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

        Ok(GameCredentials {
            identity: LoginIdentity::AccountName(account_name),
            password,
            token,
            token_time,
            character_name,
            challenge_ts,
            challenge_rand,
            otclient_v8,
        })
    } else {
        // 772: inline account number + character + password, no session key / challenge.
        let identity = read_identity(&mut s, caps)?;
        let character_name = read_string(&mut s)?;
        let password = read_string(&mut s)?;
        let (otclient_v8, _) = parse_otcv8_string_probe(s)?;

        Ok(GameCredentials {
            identity,
            password,
            token: String::new(),
            token_time: 0,
            character_name,
            challenge_ts: 0,
            challenge_rand: 0,
            otclient_v8,
        })
    }
}

/// Read the account identity: a length-prefixed string (1098) or a `u32` account number (772).
fn read_identity(data: &mut &[u8], caps: &ProtocolCaps) -> Result<LoginIdentity> {
    if caps.account_name_login {
        Ok(LoginIdentity::AccountName(read_string(data)?))
    } else {
        Ok(LoginIdentity::AccountNumber(read_u32(data)?))
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use tfs_rust_common::ProtocolVersion;

    fn put_string(buf: &mut Vec<u8>, s: &str) {
        buf.extend_from_slice(&(s.len() as u16).to_le_bytes());
        buf.extend_from_slice(s.as_bytes());
    }

    #[test]
    fn game_credentials_1098_session_key() {
        let caps = ProtocolCaps::for_version(ProtocolVersion::V1098);
        let mut stream = Vec::new();
        stream.push(0); // gm flag
        put_string(&mut stream, "myacc\nsecret\ntok\n42");
        put_string(&mut stream, "Knight");
        stream.extend_from_slice(&7u32.to_le_bytes()); // challenge ts
        stream.push(9); // challenge rand
        stream.extend_from_slice(&0u16.to_le_bytes()); // no OTCv8 probe

        let c = parse_game_credentials(&stream, &caps).expect("parse 1098 game creds");
        assert_eq!(c.identity, LoginIdentity::AccountName("myacc".into()));
        assert_eq!(c.password, "secret");
        assert_eq!(c.token, "tok");
        assert_eq!(c.token_time, 42);
        assert_eq!(c.character_name, "Knight");
        assert_eq!(c.challenge_ts, 7);
        assert_eq!(c.challenge_rand, 9);
    }

    #[test]
    fn game_credentials_772_account_number() {
        // C++ ref: gameserver/src/protocolgame.cpp onRecvFirstMessage.
        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        let mut stream = Vec::new();
        stream.push(0); // gm flag (skipBytes(1))
        stream.extend_from_slice(&123_456u32.to_le_bytes()); // account number
        put_string(&mut stream, "Druid");
        put_string(&mut stream, "hunter2");
        stream.extend_from_slice(&0u16.to_le_bytes()); // no OTCv8 probe

        let c = parse_game_credentials(&stream, &caps).expect("parse 772 game creds");
        assert_eq!(c.identity, LoginIdentity::AccountNumber(123_456));
        assert_eq!(c.password, "hunter2");
        assert_eq!(c.character_name, "Druid");
        assert!(c.token.is_empty());
        assert_eq!(c.token_time, 0);
        assert_eq!(c.challenge_ts, 0);
        assert_eq!(c.challenge_rand, 0);
    }

    #[test]
    fn login_credentials_772_account_number() {
        // C++ ref: gameserver/src/protocollogin.cpp onRecvFirstMessage.
        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        let mut stream = Vec::new();
        stream.extend_from_slice(&777u32.to_le_bytes());
        put_string(&mut stream, "pw");
        stream.extend_from_slice(&0u16.to_le_bytes());

        let (identity, password, otc) =
            parse_login_credentials(&stream, &caps).expect("parse 772 login creds");
        assert_eq!(identity, LoginIdentity::AccountNumber(777));
        assert_eq!(password, "pw");
        assert_eq!(otc, 0);
    }

    #[test]
    fn login_credentials_1098_account_name() {
        let caps = ProtocolCaps::for_version(ProtocolVersion::V1098);
        let mut stream = Vec::new();
        put_string(&mut stream, "account@example");
        put_string(&mut stream, "pw");
        stream.extend_from_slice(&0u16.to_le_bytes());

        let (identity, password, _) =
            parse_login_credentials(&stream, &caps).expect("parse 1098 login creds");
        assert_eq!(identity, LoginIdentity::AccountName("account@example".into()));
        assert_eq!(password, "pw");
    }

    #[test]
    fn otcv8_probe_after_772_game_creds() {
        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        let mut stream = Vec::new();
        stream.push(0);
        stream.extend_from_slice(&1u32.to_le_bytes());
        put_string(&mut stream, "Mage");
        put_string(&mut stream, "pw");
        put_string(&mut stream, "OTCv8");
        stream.extend_from_slice(&260u16.to_le_bytes());

        let c = parse_game_credentials(&stream, &caps).expect("parse with otcv8");
        assert_eq!(c.otclient_v8, 260);
    }
}
