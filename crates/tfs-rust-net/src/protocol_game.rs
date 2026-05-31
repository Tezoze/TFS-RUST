//! Decode client game payload → `GameCommand::Game` (after login / game state).
// C++ reference (this repo): `src/protocolgame.cpp` `parsePacket`, `src/connection.cpp`.

use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_common::{ConnId, GameCommand, ProtocolCaps, ProtocolVersion};

use tokio::io::AsyncRead;
use tokio::sync::mpsc;

use crate::adler::adler_checksum;
use crate::game_frame::read_sized_payload;
use crate::game_parse::parse_game_packet;
use crate::message::NetworkMessage;
use crate::xtea_tfs::{self, RoundKeys};

/// Parse decrypted game payload (first byte = opcode) and build `GameCommand::Game`.
/// `version` selects the per-era opcode dispatch table (Phase A2).
pub fn game_command_from_payload(
    conn_id: ConnId,
    payload: &[u8],
    version: ProtocolVersion,
) -> Result<GameCommand> {
    let mut msg = NetworkMessage::from_bytes(payload);
    let (_op, packet) = parse_game_packet(&mut msg, version)?;
    Ok(GameCommand::Game { conn_id, packet })
}

/// XTEA decrypt + payload trim (matches OTClient `xteaDecrypt` / TFS crypto: first `u16` `v` means
/// total plaintext size = `v + 2`, i.e. opcode payload is `v` bytes after the length prefix).
/// `body` is the TCP packet body only (after the 2-byte outer size).
///
/// Layout is capability-gated (`docs/PROTOCOL_VERSIONING.md` §2.1):
/// - **1098** (`caps.adler_checksum = true`, repo-root `src/protocol.cpp` `XTEA_decrypt`,
///   `networkmessage.h` `INITIAL_BUFFER_POSITION = 8`): `[Adler u32][XTEA ciphertext…]`.
/// - **772** (`caps.adler_checksum = false`, `gameserver/src/protocol.cpp` `XTEA_decrypt`,
///   `networkmessage.h` `INITIAL_BUFFER_POSITION = 4`): `[XTEA ciphertext…]`, no checksum.
///
/// The XTEA ciphertext region begins at `caps.initial_buffer_position - 4` (4 for 1098, 0 for 772):
/// `INITIAL_BUFFER_POSITION` = outer size (2) + checksum (4 / 0) + encrypted-size (2); the framing
/// layer already stripped the 2-byte outer size, and the encrypted-size u16 is the first decrypted
/// word — so only the checksum bytes precede the ciphertext.
pub fn decrypt_xtea_game_body<'a>(
    body: &'a mut [u8],
    keys: &RoundKeys,
    caps: &ProtocolCaps,
) -> Result<&'a [u8]> {
    let n = body.len();
    let cipher_off = caps.initial_buffer_position.saturating_sub(4) as usize;
    if n < cipher_off + 8 {
        return Err(TfsRustError::Protocol("game body too short".into()));
    }
    if caps.adler_checksum {
        let recv = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let expected = adler_checksum(&body[4..]);
        if recv != expected {
            return Err(TfsRustError::Protocol(
                "game packet checksum mismatch".into(),
            ));
        }
    }
    let cipher_len = n - cipher_off;
    if !cipher_len.is_multiple_of(8) {
        return Err(TfsRustError::Protocol(
            "xtea cipher length not multiple of 8".into(),
        ));
    }
    xtea_tfs::decrypt(&mut body[cipher_off..n], cipher_len, keys);

    // Inner length prefix sits at the start of the decrypted region.
    let v = u16::from_le_bytes([body[cipher_off], body[cipher_off + 1]]) as usize;
    let total = v.checked_add(2).ok_or_else(|| {
        TfsRustError::Protocol("inner length overflow (v + 2)".into())
    })?;
    if total > cipher_len {
        return Err(TfsRustError::Protocol(
            "inner length overflow vs cipher block".into(),
        ));
    }
    if total < 3 {
        return Err(TfsRustError::Protocol("inner length too small".into()));
    }
    // Opcode stream: plaintext bytes [2..total] (length `v`), i.e. `body[cipher_off+2 .. cipher_off+total]`.
    Ok(&body[cipher_off + 2..cipher_off + total])
}

/// Inverse of [`decrypt_xtea_game_body`]: one logical game payload → TCP frame (`u16` outer + optional
/// Adler + XTEA). Header layout is capability-gated (`docs/PROTOCOL_VERSIONING.md` §2.1):
/// - **1098** (`caps.adler_checksum = true`): `[u16 outer][Adler u32][XTEA ciphertext…]`.
/// - **772** (`caps.adler_checksum = false`): `[u16 outer][XTEA ciphertext…]`, no checksum.
// Plaintext: [u16 v][payload…] zero-padded to a multiple of 8 bytes for XTEA. `v` is **`payload.len()`** (bytes
// after the 2-byte header), not the padded block size minus 2 — matches OTClient/TFS inner length semantics.
pub fn encrypt_xtea_game_frame(payload: &[u8], keys: &RoundKeys, caps: &ProtocolCaps) -> Vec<u8> {
    // Plaintext = [u16 v][payload…] padded up to the next multiple of 8 for XTEA.
    let plain_len = (2 + payload.len()).next_multiple_of(8);
    let v = payload.len();
    let mut plain = vec![0u8; plain_len];
    plain[0..2].copy_from_slice(&(v as u16).to_le_bytes());
    plain[2..2 + payload.len()].copy_from_slice(payload);
    xtea_tfs::encrypt(&mut plain, plain_len, keys);

    // 1098 prefixes a 4-byte Adler checksum over the ciphertext (`addCryptoHeader(true)`); 772 omits it.
    let checksum_len = if caps.adler_checksum { 4 } else { 0 };
    let mut body = vec![0u8; checksum_len + plain.len()];
    body[checksum_len..].copy_from_slice(&plain);
    if caps.adler_checksum {
        let c = adler_checksum(&plain);
        body[0..4].copy_from_slice(&c.to_le_bytes());
    }
    let mut frame = Vec::with_capacity(2 + body.len());
    frame.extend_from_slice(&(body.len() as u16).to_le_bytes());
    frame.extend_from_slice(&body);
    frame
}

/// Read framed payloads from the connection and forward parsed commands to the game thread.
/// Plaintext opcodes (tests / local harness).
pub async fn forward_game_packets<R: AsyncRead + Unpin>(
    mut read: R,
    conn_id: ConnId,
    cmd_tx: mpsc::UnboundedSender<GameCommand>,
    version: ProtocolVersion,
) -> std::io::Result<()> {
    while let Some(payload) = read_sized_payload(&mut read).await? {
        match game_command_from_payload(conn_id, &payload, version) {
            Ok(cmd) => {
                if cmd_tx.send(cmd).is_err() {
                    break;
                }
            }
            Err(e) => {
                tracing::warn!(?e, "game packet parse error");
            }
        }
    }
    Ok(())
}

/// Post-handshake game packets: 2-byte LE size + body with optional Adler + XTEA
/// (`connection.cpp` + `protocol.cpp`). Transport layout follows `caps` (§2.1).
pub async fn forward_game_packets_xtea<R: AsyncRead + Unpin>(
    mut read: R,
    conn_id: ConnId,
    cmd_tx: mpsc::UnboundedSender<GameCommand>,
    keys: &RoundKeys,
    version: ProtocolVersion,
    caps: &ProtocolCaps,
) -> std::io::Result<()> {
    while let Some(mut body) = read_sized_payload(&mut read).await? {
        let plain = match decrypt_xtea_game_body(&mut body, keys, caps) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(?e, "xtea game body");
                continue;
            }
        };
        match game_command_from_payload(conn_id, plain, version) {
            Ok(cmd) => {
                if cmd_tx.send(cmd).is_err() {
                    break;
                }
            }
            Err(e) => {
                tracing::warn!(?e, "game packet parse error");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod encrypt_tests {
    use super::*;

    #[test]
    fn xtea_frame_roundtrip_matches_manual_vector() {
        let keys = crate::xtea_tfs::expand_key(&[1u32, 2, 3, 4]);
        let caps = ProtocolCaps::for_version(ProtocolVersion::V1098);
        let payload = [0x65u8, 0, 0, 0, 0, 0];
        let frame = encrypt_xtea_game_frame(&payload, &keys, &caps);
        let bl = u16::from_le_bytes([frame[0], frame[1]]) as usize;
        let mut body = frame[2..2 + bl].to_vec();
        let plain = decrypt_xtea_game_body(&mut body, &keys, &caps).expect("decrypt");
        assert_eq!(plain, payload);
    }

    #[test]
    fn xtea_frame_roundtrip_772_no_checksum() {
        let keys = crate::xtea_tfs::expand_key(&[1u32, 2, 3, 4]);
        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        let payload = [0x14u8, 1, 2, 3];
        let frame = encrypt_xtea_game_frame(&payload, &keys, &caps);
        // 772 frame has no 4-byte Adler header: body == ciphertext only.
        let bl = u16::from_le_bytes([frame[0], frame[1]]) as usize;
        assert!(bl.is_multiple_of(8), "772 body is pure XTEA blocks, no checksum");
        let mut body = frame[2..2 + bl].to_vec();
        let plain = decrypt_xtea_game_body(&mut body, &keys, &caps).expect("decrypt");
        assert_eq!(plain, payload);
    }
}
