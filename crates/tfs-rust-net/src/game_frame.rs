//! TCP framing: 2-byte LE body length + payload (matches `Connection::parseHeader` body read).
// C++ reference (this repo): `src/connection.cpp`, `src/networkmessage.h` (`HEADER_LENGTH = 2`).

use std::io::ErrorKind;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::io::{AsyncRead, AsyncReadExt};

/// Read one logical packet body (plaintext tests; production uses checksum + XTEA per `Protocol`).
pub async fn read_sized_payload<R: AsyncRead + Unpin>(
    r: &mut R,
) -> std::io::Result<Option<Vec<u8>>> {
    read_sized_payload_with_recv(r, None).await
}

/// Same as [`read_sized_payload`], accounting `2 + n` recv bytes (`communication.cc` NetLoad).
pub async fn read_sized_payload_with_recv<R: AsyncRead + Unpin>(
    r: &mut R,
    recv_bytes: Option<&AtomicU64>,
) -> std::io::Result<Option<Vec<u8>>> {
    let mut h = [0u8; 2];
    if let Err(e) = r.read_exact(&mut h).await {
        if e.kind() == ErrorKind::UnexpectedEof {
            return Ok(None);
        }
        return Err(e);
    }
    let n = u16::from_le_bytes(h) as usize;
    if n == 0 || n > 24_000 {
        return Ok(None);
    }
    let mut buf = vec![0u8; n];
    r.read_exact(&mut buf).await?;
    if let Some(c) = recv_bytes {
        c.fetch_add(2 + n as u64, Ordering::Relaxed);
    }
    Ok(Some(buf))
}
