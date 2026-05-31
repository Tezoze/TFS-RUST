//! Outgoing game login challenge (`ProtocolGame::onConnect`).
// C++ reference: 1098 repo-root `src/protocolgame.cpp` `ProtocolGame::onConnect` (sends `0x1F`
// timestamp/random challenge). 772 `gameserver/src/`: no `onConnect` challenge — the client sends
// the first packet directly, so 772 connections must NOT emit `0x1F`.

use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;

use tfs_rust_common::ProtocolCaps;

use crate::adler::adler_checksum;

/// Challenge values echoed in the first game packet (`onRecvFirstMessage`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameChallenge {
    pub timestamp: u32,
    pub random: u8,
}

/// Send the `0x1F` timestamp/random challenge **only when the era uses it** (`caps.prelogin_challenge`).
/// Returns `None` for 772 (no challenge); the caller then skips challenge-echo verification.
pub async fn send_game_challenge<W: AsyncWrite + Unpin>(
    w: &mut W,
    caps: &ProtocolCaps,
) -> std::io::Result<Option<GameChallenge>> {
    if !caps.prelogin_challenge {
        return Ok(None);
    }
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;
    let rand = (std::time::Instant::now().elapsed().as_nanos() & 0xFF) as u8;

    let mut body = [0u8; 12];
    body[4..6].copy_from_slice(&6u16.to_le_bytes());
    body[6] = 0x1F;
    body[7..11].copy_from_slice(&ts.to_le_bytes());
    body[11] = rand;
    let c = adler_checksum(&body[4..12]);
    body[0..4].copy_from_slice(&c.to_le_bytes());

    let mut out = [0u8; 2 + 12];
    out[0..2].copy_from_slice(&(12u16).to_le_bytes());
    out[2..].copy_from_slice(&body);
    w.write_all(&out).await?;
    Ok(Some(GameChallenge {
        timestamp: ts,
        random: rand,
    }))
}
