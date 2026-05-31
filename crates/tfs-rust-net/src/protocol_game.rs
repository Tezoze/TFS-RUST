//! Decode client game payload → `GameCommand::Game` (after login / game state).
// C++ reference (this repo): `src/protocolgame.cpp` `parsePacket`, `src/connection.cpp`.

use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_common::{ConnId, GameCommand, ProtocolVersion};

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
/// `body` is the TCP packet body only (after the 2-byte outer size): `[Adler u32][XTEA ciphertext…]`.
pub fn decrypt_xtea_game_body<'a>(body: &'a mut [u8], keys: &RoundKeys) -> Result<&'a [u8]> {
    let n = body.len();
    if n < 4 + 8 {
        return Err(TfsRustError::Protocol("game body too short".into()));
    }
    let recv = u32::from_le_bytes(body[0..4].try_into().unwrap());
    let expected = adler_checksum(&body[4..]);
    if recv != expected {
        return Err(TfsRustError::Protocol(
            "game packet checksum mismatch".into(),
        ));
    }
    let cipher_len = n - 4;
    if cipher_len % 8 != 0 {
        return Err(TfsRustError::Protocol(
            "xtea cipher length not multiple of 8".into(),
        ));
    }
    xtea_tfs::decrypt(&mut body[4..n], cipher_len, keys);

    let v = u16::from_le_bytes([body[4], body[5]]) as usize;
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
    // Opcode stream: plaintext bytes [2..total] (length `v`), same as `body[6..4+total]`.
    Ok(&body[6..4 + total])
}

/// Inverse of [`decrypt_xtea_game_body`]: one logical game payload → TCP frame (`u16` outer + Adler + XTEA).
// Plaintext: [u16 v][payload…] zero-padded to a multiple of 8 bytes for XTEA. `v` is **`payload.len()`** (bytes
// after the 2-byte header), not the padded block size minus 2 — matches OTClient/TFS inner length semantics.
pub fn encrypt_xtea_game_frame(payload: &[u8], keys: &RoundKeys) -> Vec<u8> {
    let content_len = 2 + payload.len();
    let mut plain_len = content_len;
    while plain_len % 8 != 0 {
        plain_len += 1;
    }
    let v = payload.len();
    let mut plain = vec![0u8; plain_len];
    plain[0..2].copy_from_slice(&(v as u16).to_le_bytes());
    plain[2..2 + payload.len()].copy_from_slice(payload);
    xtea_tfs::encrypt(&mut plain, plain_len, keys);
    let mut body = vec![0u8; 4 + plain.len()];
    body[4..4 + plain.len()].copy_from_slice(&plain);
    let c = adler_checksum(&body[4..]);
    body[0..4].copy_from_slice(&c.to_le_bytes());
    let mut frame = Vec::with_capacity(2 + body.len());
    frame.extend_from_slice(&(body.len() as u16).to_le_bytes());
    frame.extend_from_slice(&body);
    frame
}

#[cfg(test)]
mod encrypt_tests {
    use super::*;

    #[test]
    fn xtea_frame_roundtrip_matches_manual_vector() {
        let keys = crate::xtea_tfs::expand_key(&[1u32, 2, 3, 4]);
        let payload = [0x65u8, 0, 0, 0, 0, 0];
        let frame = encrypt_xtea_game_frame(&payload, &keys);
        let bl = u16::from_le_bytes([frame[0], frame[1]]) as usize;
        let mut body = frame[2..2 + bl].to_vec();
        let plain = decrypt_xtea_game_body(&mut body, &keys).expect("decrypt");
        assert_eq!(plain, payload);
    }
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

/// Post-handshake game packets: 2-byte LE size + body with Adler + XTEA (`connection.cpp` + `protocol.cpp`).
pub async fn forward_game_packets_xtea<R: AsyncRead + Unpin>(
    mut read: R,
    conn_id: ConnId,
    cmd_tx: mpsc::UnboundedSender<GameCommand>,
    keys: &RoundKeys,
    version: ProtocolVersion,
) -> std::io::Result<()> {
    while let Some(mut body) = read_sized_payload(&mut read).await? {
        let plain = match decrypt_xtea_game_body(&mut body, keys) {
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
