//! Transparent TCP proxy for OTClient ↔ TFS Rust — see `tasks/packet-proxy-spec.md`.
#![forbid(unsafe_code)]

mod connection;
mod decrypt;
mod handshake;
mod logger;
mod opcodes;

use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use rsa::RsaPrivateKey;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

use crate::logger::PacketLogger;

#[derive(Parser, Debug)]
#[command(name = "packet-proxy", about = "OTClient ↔ server packet logger (raw + XTEA decode on game port)")]
struct Args {
    /// Listen address for login (OTClient connects here; forward to upstream).
    #[arg(long, default_value = "127.0.0.1:7171")]
    login_listen: String,
    /// Upstream **server** login socket. Must not use the same **local** port as `--game-listen` (both would listen on one machine). Default 7272: run server with `TFS_LOGIN_ADDR=127.0.0.1:7272`.
    #[arg(long, default_value = "127.0.0.1:7272")]
    login_upstream: String,
    /// Listen address for game (OTClient game port).
    #[arg(long, default_value = "127.0.0.1:7172")]
    game_listen: String,
    /// Upstream **server** game socket. Default 7273: `TFS_GAME_ADDR=127.0.0.1:7273`.
    #[arg(long, default_value = "127.0.0.1:7273")]
    game_upstream: String,
    /// PKCS#1 RSA private key PEM (same as server `key.pem`) — required for game XTEA extraction.
    #[arg(long)]
    key: PathBuf,
    /// Append packet lines to this file (stdout always).
    #[arg(long)]
    log: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    let pem = std::fs::read_to_string(&args.key).with_context(|| {
        format!(
            "read RSA PEM `{}` (ENOENT: file missing or wrong cwd — use absolute path, e.g. `{}/key.pem` from the repo root)",
            args.key.display(),
            std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string())
        )
    })?;
    let rsa: Arc<RsaPrivateKey> = Arc::new(
        tfs_rust_net::rsa::private_key_from_pkcs1_pem(&pem)
            .map_err(|e| anyhow::anyhow!("{e}"))?,
    );

    let log = PacketLogger::new(args.log.as_deref())?;

    let login_listen = args.login_listen.clone();
    let login_upstream = args.login_upstream.clone();
    let log_login = log.clone();
    let login_task = tokio::spawn(async move {
        run_login_listener(login_listen, login_upstream, log_login).await;
    });

    let game_listen = args.game_listen.clone();
    let game_upstream = args.game_upstream.clone();
    let log_game = log.clone();
    let rsa_game = rsa.clone();
    let game_task = tokio::spawn(async move {
        run_game_listener(game_listen, game_upstream, log_game, rsa_game).await;
    });

    let _ = tokio::join!(login_task, game_task);
    Ok(())
}

fn log_bind_failure(which: &str, addr: &str, e: &std::io::Error) {
    error!(which, addr = %addr, %e, "listener bind failed");
    if e.kind() == ErrorKind::AddrInUse {
        eprintln!(
            "\
\nNote ({which}): `{addr}` is already in use — usually `tfs-rust` is still bound to 7171/7172.\n\
\n  • Stop the server first, or free ports:  fuser -k 7171/tcp 7172/tcp   (Linux, package psmisc)\n\
\n  • Proxy must bind OTClient ports (7171 login, 7172 game). Run the server on other ports, e.g.:\n\
      TFS_LOGIN_ADDR=127.0.0.1:7272 TFS_GAME_ADDR=127.0.0.1:7273 cargo run --bin tfs-rust\n\
\n  • Then:  cargo run -p packet-proxy -- --key ./key.pem\n\
    (defaults: forward 7171→7272, 7172→7273 — no duplicate listener on one host.)\n"
        );
    }
}

async fn run_login_listener(listen: String, upstream: String, log: PacketLogger) {
    let listener = match TcpListener::bind(&listen).await {
        Ok(l) => l,
        Err(e) => {
            log_bind_failure("login", &listen, &e);
            return;
        }
    };
    info!(addr = %listen, upstream = %upstream, "login proxy listening");
    loop {
        let (client, peer) = match listener.accept().await {
            Ok(x) => x,
            Err(e) => {
                error!(%e, "login accept");
                continue;
            }
        };
        let upstream = upstream.clone();
        let log = log.clone();
        tokio::spawn(async move {
            info!(%peer, "login connection");
            let up = match tokio::net::TcpStream::connect(&upstream).await {
                Ok(s) => s,
                Err(e) => {
                    warn!(upstream = %upstream, %e, "login upstream connect failed");
                    return;
                }
            };
            connection::proxy_login_port(client, up, log).await;
            info!(%peer, "login connection closed");
        });
    }
}

async fn run_game_listener(listen: String, upstream: String, log: PacketLogger, rsa: Arc<RsaPrivateKey>) {
    let listener = match TcpListener::bind(&listen).await {
        Ok(l) => l,
        Err(e) => {
            log_bind_failure("game", &listen, &e);
            return;
        }
    };
    info!(addr = %listen, upstream = %upstream, "game proxy listening");
    loop {
        let (client, peer) = match listener.accept().await {
            Ok(x) => x,
            Err(e) => {
                error!(%e, "game accept");
                continue;
            }
        };
        let upstream = upstream.clone();
        let log = log.clone();
        let rsa = rsa.clone();
        tokio::spawn(async move {
            info!(%peer, "game connection");
            let up = match tokio::net::TcpStream::connect(&upstream).await {
                Ok(s) => s,
                Err(e) => {
                    warn!(upstream = %upstream, %e, "game upstream connect failed");
                    return;
                }
            };
            connection::proxy_game_port(client, up, rsa, log).await;
            info!(%peer, "game connection closed");
        });
    }
}
