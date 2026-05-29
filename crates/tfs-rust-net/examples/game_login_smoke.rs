//! OTClient smoke: **login** (character list) on 7171 + **game** (challenge → RSA → XTEA) on 7172.
//! Matches typical `config.lua`: `loginProtocolPort = 7171`, `gameProtocolPort = 7172`.
//!
//! OTClient opens **login** first (login-shaped RSA packet). If you only bind the game handler on 7171,
//! you get: `game port: received login-shaped packet`. This example binds **both** ports.
//!
//! ```text
//! DATABASE_URL='mysql://tfs@127.0.0.1:3306/TFS' ./scripts/run_game_login_smoke.sh
//!
//! # Optional:
//! TFS_LOGIN_ADDR=127.0.0.1:7171 TFS_GAME_ADDR=127.0.0.1:7172 ...
//! ```
//!
//! In OTClient: **Login server** = host:7171, **Game server** = host:7172 (same as C++ TFS).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use tfs_rust_common::GameCommand;
use tfs_rust_net::{GameWireConfig, LoginWireConfig, OutRegistry, Server};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

/// Resolve PEM: `TFS_RSA_PEM` if set, else workspace-root `key.pem`, else `./key.pem`.
fn resolve_pem_path() -> anyhow::Result<PathBuf> {
    if let Ok(p) = std::env::var("TFS_RSA_PEM") {
        return Ok(PathBuf::from(p));
    }
    let workspace_root_key = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("key.pem");
    if workspace_root_key.is_file() {
        return Ok(workspace_root_key);
    }
    let cwd_key = PathBuf::from("key.pem");
    if cwd_key.is_file() {
        return Ok(cwd_key);
    }
    anyhow::bail!(
        "no RSA PEM found. Add key.pem at the workspace root, run from a directory containing key.pem, \
         or set TFS_RSA_PEM to a PKCS#1 private key path"
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pem_path = resolve_pem_path()?;
    eprintln!("Using RSA PEM: {}", pem_path.display());

    let pem = std::fs::read_to_string(&pem_path).with_context(|| {
        format!(
            "cannot read RSA PEM at {:?}\n\
             Set TFS_RSA_PEM to a readable PKCS#1 PEM file, or place key.pem at the workspace root.",
            pem_path
        )
    })?;
    let rsa_private_key = tfs_rust_net::rsa::private_key_from_pkcs1_pem(&pem)
        .context("load PKCS#1 RSA private key (BEGIN RSA PRIVATE KEY)")?;

    let database_url = std::env::var("DATABASE_URL")
        .context("set DATABASE_URL (MariaDB) for authentication")?;
    let db = tfs_rust_db::DbPool::connect(
        &database_url,
        &tfs_rust_db::DbPoolConnectOptions::default(),
    )
    .await
    .context("database connect")?;

    let out_registry: OutRegistry = Arc::new(Mutex::new(HashMap::new()));

    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<GameCommand>();
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            eprintln!("[game_login_smoke] {:?}", cmd);
        }
    });

    let login_addr =
        std::env::var("TFS_LOGIN_ADDR").unwrap_or_else(|_| "127.0.0.1:7171".to_string());
    let game_addr = std::env::var("TFS_GAME_ADDR").unwrap_or_else(|_| "127.0.0.1:7172".to_string());

    let game_port: u16 = std::env::var("TFS_GAME_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7172);

    let login_listener = TcpListener::bind(&login_addr)
        .await
        .with_context(|| format!("bind login server {login_addr}"))?;
    let game_listener = TcpListener::bind(&game_addr)
        .await
        .with_context(|| format!("bind game server {game_addr}"))?;

    eprintln!(
        "Login (character list): {}  — set OTClient **login** server to this address.",
        login_listener.local_addr()?
    );
    eprintln!(
        "Game (world):          {}  — set OTClient **game** server to this address.",
        game_listener.local_addr()?
    );
    eprintln!(
        "Character list will advertise game at {}:{} (see TFS_PUBLIC_IP / TFS_GAME_PORT).",
        std::env::var("TFS_PUBLIC_IP").unwrap_or_else(|_| "127.0.0.1".to_string()),
        game_port
    );
    eprintln!("Waiting for connections (Ctrl+C to stop)…");

    let server_name =
        std::env::var("TFS_SERVER_NAME").unwrap_or_else(|_| "Australis".to_string());
    let public_ip = std::env::var("TFS_PUBLIC_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
    let motd = std::env::var("TFS_MOTD").unwrap_or_default();
    let motd_num: u32 = std::env::var("TFS_MOTD_NUM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let login_cfg = LoginWireConfig {
        rsa_private_key: rsa_private_key.clone(),
        db: db.clone(),
        motd_num,
        motd: motd.clone(),
        server_name: server_name.clone(),
        public_ip: public_ip.clone(),
        game_port,
        free_premium: true,
    };

    let game_cfg = GameWireConfig {
        cmd_tx,
        rsa_private_key,
        db,
        out_registry,
        motd_num,
        motd,
        server_name,
        public_ip,
        game_port,
        free_premium: true,
    };

    tokio::spawn(async move {
        let mut login_server = Server::from_listener(login_listener);
        login_server.accept_loop_with_login(login_cfg).await;
    });

    let mut game_server = Server::from_listener(game_listener);
    game_server.accept_loop_with_game(game_cfg).await;

    Ok(())
}
