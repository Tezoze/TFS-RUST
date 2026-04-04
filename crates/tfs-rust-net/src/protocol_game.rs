//! Decode client game payload → `GameCommand::Game` (after login / game state).
// C++ reference (this repo): `src/protocolgame.cpp` `parsePacket`, `src/connection.cpp`.

use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_common::{ConnId, GameCommand};

use tokio::io::AsyncRead;
use tokio::sync::mpsc;

use crate::adler::adler_checksum;
use crate::game_frame::read_sized_payload;
use crate::game_parse::parse_game_packet;
use crate::message::NetworkMessage;
use crate::xtea_tfs::{self, RoundKeys};

/// Parse decrypted game payload (first byte = opcode) and build `GameCommand::Game`.
pub fn game_command_from_payload(conn_id: ConnId, payload: &[u8]) -> Result<GameCommand> {
    let mut msg = NetworkMessage::from_bytes(payload);
    let (_op, packet) = parse_game_packet(&mut msg)?;
    Ok(GameCommand::Game { conn_id, packet })
}

/// XTEA decrypt + inner length trim (`Protocol::XTEA_decrypt` in `src/protocol.cpp`).
/// `body` is the TCP packet body only (after the 2-byte size prefix): `[checksum u32][ciphertext…]`.
pub fn decrypt_xtea_game_body<'a>(body: &'a mut [u8], keys: &RoundKeys) -> Result<&'a [u8]> {
    let n = body.len();
    if n < 4 {
        return Err(TfsRustError::Protocol("game body too short".into()));
    }
    let full_len = n + 2;
    if ((full_len - 6) & 7) != 0 {
        return Err(TfsRustError::Protocol(
            "xtea cipher length not multiple of 8".into(),
        ));
    }
    let recv = u32::from_le_bytes(body[0..4].try_into().unwrap());
    let expected = adler_checksum(&body[4..]);
    if recv != expected {
        return Err(TfsRustError::Protocol(
            "game packet checksum mismatch".into(),
        ));
    }
    let cipher_len = n - 4;
    crate::xtea_tfs::decrypt(&mut body[4..n], cipher_len, keys);

    let inner = u16::from_le_bytes([body[4], body[5]]) as usize;
    if inner + 8 > full_len {
        return Err(TfsRustError::Protocol("inner length overflow".into()));
    }
    if inner < 8 {
        return Err(TfsRustError::Protocol("inner length too small".into()));
    }
    let end = inner - 2;
    if end < 6 || end > n {
        return Err(TfsRustError::Protocol("inner length inconsistent".into()));
    }
    Ok(&body[6..end])
}

/// Inverse of [`decrypt_xtea_game_body`]: one logical game payload → TCP frame (`u16` size + Adler + XTEA).
// C++ reference: `src/protocol.cpp` `XTEA_encrypt`, `OutputMessage::addCryptoHeader`.
pub fn encrypt_xtea_game_frame(payload: &[u8], keys: &RoundKeys) -> Vec<u8> {
    // Plaintext: [u16 inner][payload…][pad to `inner` bytes] — see `decrypt_xtea_game_body` (`body[6..]` = opcodes).
    let inner_sem = (8 + payload.len()).max(10);
    let mut plain = vec![0u8; inner_sem];
    plain[0..2].copy_from_slice(&(inner_sem as u16).to_le_bytes());
    plain[2..2 + payload.len()].copy_from_slice(payload);
    while !plain.len().is_multiple_of(8) {
        plain.push(0);
    }
    let plen = plain.len();
    xtea_tfs::encrypt(&mut plain, plen, keys);
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
    cmd_tx: mpsc::Sender<GameCommand>,
) -> std::io::Result<()> {
    while let Some(payload) = read_sized_payload(&mut read).await? {
        match game_command_from_payload(conn_id, &payload) {
            Ok(cmd) => {
                if cmd_tx.send(cmd).await.is_err() {
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
    cmd_tx: mpsc::Sender<GameCommand>,
    keys: &RoundKeys,
) -> std::io::Result<()> {
    while let Some(mut body) = read_sized_payload(&mut read).await? {
        let plain = match decrypt_xtea_game_body(&mut body, keys) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(?e, "xtea game body");
                continue;
            }
        };
        match game_command_from_payload(conn_id, plain) {
            Ok(cmd) => {
                if cmd_tx.send(cmd).await.is_err() {
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
