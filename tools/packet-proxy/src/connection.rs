//! Bidirectional TCP proxy with `read_sized_payload` framing (`tfs-rust-net::game_frame`).

use std::sync::{Arc, Mutex};

use anyhow::Context;
use rsa::RsaPrivateKey;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

use tfs_rust_net::game_frame::read_sized_payload;
use tfs_rust_net::xtea_tfs::{expand_key, RoundKeys};

use crate::decrypt::decrypt_game_body;
use crate::handshake::parse_first_game_packet;
use crate::logger::PacketLogger;
use crate::opcodes::{client_opcode_name, server_opcode_name};

async fn write_sized_body<W: AsyncWrite + Unpin>(w: &mut W, body: &[u8]) -> std::io::Result<()> {
    w.write_all(&(body.len() as u16).to_le_bytes()).await?;
    w.write_all(body).await?;
    Ok(())
}

/// Login port: forward only; log every framed body as raw hex (Phase 1).
pub async fn proxy_login_port(client: TcpStream, upstream: TcpStream, log: PacketLogger) {
    let (mut cr, mut cw) = client.into_split();
    let (mut ur, mut uw) = upstream.into_split();
    let log_a = log.clone();
    let log_b = log.clone();
    let a = tokio::spawn(async move { pipe_login_half(&mut cr, &mut uw, "C->S", log_a).await });
    let b = tokio::spawn(async move { pipe_login_half(&mut ur, &mut cw, "S->C", log_b).await });
    let _ = tokio::join!(a, b);
}

async fn pipe_login_half<R, W>(
    read: &mut R,
    write: &mut W,
    dir: &'static str,
    log: PacketLogger,
) -> anyhow::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    loop {
        let body = match read_sized_payload(read).await? {
            Some(b) => b,
            None => break,
        };
        write_sized_body(write, &body)
            .await
            .context("login port forward write")?;
        let ts = crate::logger::timestamp_rfc3339_ms();
        log.line(&format!(
            "[{ts}] [{dir}] raw {} bytes (login port, framed)",
            body.len()
        ));
        log.hex_dump(&body);
    }
    Ok(())
}

/// Game port: Phase 2–3 — capture XTEA from first client packet; decrypt subsequent frames for logs.
pub async fn proxy_game_port(
    client: TcpStream,
    upstream: TcpStream,
    rsa: Arc<RsaPrivateKey>,
    log: PacketLogger,
) {
    let keys: Arc<Mutex<Option<RoundKeys>>> = Arc::new(Mutex::new(None));
    let (cr, cw) = client.into_split();
    let (ur, uw) = upstream.into_split();

    let k1 = keys.clone();
    let rsa_c = rsa.clone();
    let log_c = log.clone();
    let h_c = tokio::spawn(async move { game_c2s(cr, uw, k1, rsa_c, log_c).await });

    let k2 = keys.clone();
    let log_s = log.clone();
    let h_s = tokio::spawn(async move { game_s2c(ur, cw, k2, log_s).await });

    let _ = tokio::join!(h_c, h_s);
}

async fn game_c2s(
    mut read: tokio::net::tcp::OwnedReadHalf,
    mut write: tokio::net::tcp::OwnedWriteHalf,
    keys: Arc<Mutex<Option<RoundKeys>>>,
    rsa: Arc<RsaPrivateKey>,
    log: PacketLogger,
) -> anyhow::Result<()> {
    let mut idx = 0usize;
    loop {
        let body = match read_sized_payload(&mut read).await? {
            Some(b) => b,
            None => break,
        };
        write_sized_body(&mut write, &body)
            .await
            .context("game C->S forward")?;
        let ts = crate::logger::timestamp_rfc3339_ms();
        if idx == 0 {
            match parse_first_game_packet(&body, &rsa) {
                Ok(g) => {
                    *keys.lock().expect("keys") = Some(expand_key(&g.xtea_key));
                    log.line(&format!(
                        "[{ts}] [C->S] first packet {} bytes — XTEA key captured (account={}, char={})",
                        body.len(),
                        g.account_name,
                        g.character_name
                    ));
                    log.hex_dump(&body);
                }
                Err(e) => {
                    log.line(&format!(
                        "[{ts}] [C->S] first packet {} bytes — parse_first_game_packet failed: {e}",
                        body.len()
                    ));
                    log.hex_dump(&body);
                }
            }
        } else {
            log_decrypted_frame(&log, &ts, "C->S", &body, &keys, true)?;
        }
        idx += 1;
    }
    Ok(())
}

async fn game_s2c(
    mut read: tokio::net::tcp::OwnedReadHalf,
    mut write: tokio::net::tcp::OwnedWriteHalf,
    keys: Arc<Mutex<Option<RoundKeys>>>,
    log: PacketLogger,
) -> anyhow::Result<()> {
    let mut idx = 0usize;
    loop {
        let body = match read_sized_payload(&mut read).await? {
            Some(b) => b,
            None => break,
        };
        write_sized_body(&mut write, &body)
            .await
            .context("game S->C forward")?;
        let ts = crate::logger::timestamp_rfc3339_ms();
        if idx == 0 {
            log.line(&format!(
                "[{ts}] [S->C] {} bytes — game challenge / prelude (not XTEA ciphertext)",
                body.len()
            ));
            log.hex_dump(&body);
        } else {
            log_decrypted_frame(&log, &ts, "S->C", &body, &keys, false)?;
        }
        idx += 1;
    }
    Ok(())
}

fn log_decrypted_frame(
    log: &PacketLogger,
    ts: &str,
    dir: &str,
    body: &[u8],
    keys: &Arc<Mutex<Option<RoundKeys>>>,
    is_c2s: bool,
) -> anyhow::Result<()> {
    let rk_opt = *keys.lock().expect("keys");
    if let Some(rk) = rk_opt {
        if let Some(plain) = decrypt_game_body(body, &rk) {
            let op = plain.first().copied().unwrap_or(0);
            let name = if is_c2s {
                client_opcode_name(op)
            } else {
                server_opcode_name(op)
            };
            log.line(&format!(
                "[{ts}] [{dir}] opcode=0x{op:02x} ({name}) plaintext={} bytes",
                plain.len()
            ));
            log.hex_dump(&plain);
            if !is_c2s && op == 0x64 && plain.len() >= 6 {
                let x = u16::from_le_bytes([plain[1], plain[2]]);
                let y = u16::from_le_bytes([plain[3], plain[4]]);
                let z = plain[5];
                log.line(&format!(
                    "  map: 0x64 position bytes: ({x},{y},{z}) (Phase 4 hint)"
                ));
            }
        } else {
            log.line(&format!(
                "[{ts}] [{dir}] {} bytes — XTEA decrypt/checksum failed (log only; bytes forwarded unchanged)",
                body.len()
            ));
            log.hex_dump(body);
        }
    } else {
        log.line(&format!(
            "[{ts}] [{dir}] {} bytes — no XTEA key yet (raw wire)",
            body.len()
        ));
        log.hex_dump(body);
    }
    Ok(())
}
