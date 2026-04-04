//! Outgoing game login challenge (`ProtocolGame::onConnect`).
// C++ reference: `src/protocolgame.cpp` `ProtocolGame::onConnect`.

use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;

use crate::adler::adler_checksum;

/// Challenge values echoed in the first game packet (`onRecvFirstMessage`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameChallenge {
    pub timestamp: u32,
    pub random: u8,
}

/// Send the 0x1F timestamp/random challenge (2-byte LE size + 12-byte body with Adler).
pub async fn send_game_challenge<W: AsyncWrite + Unpin>(
    w: &mut W,
) -> std::io::Result<GameChallenge> {
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
    Ok(GameChallenge {
        timestamp: ts,
        random: rand,
    })
}
