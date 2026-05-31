use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rsa::RsaPrivateKey;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{error, info, trace};

use tfs_rust_common::{ConnId, GameCommand, ProtocolCaps, ProtocolVersion};

use crate::game_challenge::{send_game_challenge, GameChallenge};
use crate::game_first_packet::{parse_first_client_packet, FirstClientPacket, LoginIdentity};
use crate::game_frame::read_sized_payload;
use crate::protocol_game::{encrypt_xtea_game_frame, forward_game_packets_xtea};
use crate::protocol_login_out::{build_login_error, build_login_success, LoginSuccess};
use crate::xtea_tfs::expand_key;

/// Per-connection writer for `flush_output_buffers` batches (`src/connection.cpp`).
pub type OutRegistry = Arc<Mutex<HashMap<ConnId, mpsc::UnboundedSender<Vec<Vec<u8>>>>>>;

pub struct Server {
    listener: TcpListener,
}

/// RSA + game thread + DB + per-connection outgoing writers (`src/connection.cpp` send path).
#[derive(Clone)]
pub struct GameWireConfig {
    pub cmd_tx: mpsc::UnboundedSender<GameCommand>,
    pub rsa_private_key: RsaPrivateKey,
    pub db: tfs_rust_db::DbPool,
    pub password_hash: tfs_rust_db::PasswordHashConfig,
    pub out_registry: OutRegistry,
    pub motd_num: u32,
    pub motd: String,
    pub server_name: String,
    pub public_ip: String,
    pub game_port: u16,
    pub free_premium: bool,
    pub protocol_version: ProtocolVersion,
    pub protocol_caps: ProtocolCaps,
}

/// Login port: character list only (`src/protocollogin.cpp`).
#[derive(Clone)]
pub struct LoginWireConfig {
    pub rsa_private_key: RsaPrivateKey,
    pub db: tfs_rust_db::DbPool,
    pub password_hash: tfs_rust_db::PasswordHashConfig,
    pub motd_num: u32,
    pub motd: String,
    pub server_name: String,
    pub public_ip: String,
    pub game_port: u16,
    pub free_premium: bool,
    pub protocol_version: ProtocolVersion,
    pub protocol_caps: ProtocolCaps,
}

static NEXT_CONN_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

impl Server {
    pub async fn bind(addr: &str) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        info!("TFS Rust Server listening on {}", addr);
        Ok(Self { listener })
    }

    pub fn from_listener(listener: TcpListener) -> Self {
        Self { listener }
    }

    pub fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.listener.local_addr()
    }

    pub async fn accept_loop(&mut self) {
        self.accept_loop_inner(None, None).await;
    }

    pub async fn accept_loop_with_game(&mut self, wire: GameWireConfig) {
        self.accept_loop_inner(Some(wire), None).await;
    }

    pub async fn accept_loop_with_login(&mut self, wire: LoginWireConfig) {
        self.accept_loop_inner(None, Some(wire)).await;
    }

    async fn accept_loop_inner(
        &mut self,
        game: Option<GameWireConfig>,
        login: Option<LoginWireConfig>,
    ) {
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    eprintln!("[tfs-rust-net] accepted TCP connection from {}", addr);
                    info!("New connection from {}", addr);
                    let game = game.clone();
                    let login = login.clone();
                    let peer = addr;
                    tokio::spawn(async move {
                        let r = match (game, login) {
                            (Some(w), _) => handle_game_connection(stream, w).await,
                            (None, Some(l)) => handle_login_connection(stream, l).await,
                            _ => drain_connection(stream).await,
                        };
                        if let Err(e) = r {
                            eprintln!("[tfs-rust-net] connection error from {}: {}", peer, e);
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
            }
        }
    }
}

async fn drain_connection(mut stream: TcpStream) -> anyhow::Result<()> {
    let mut buf = [0u8; 4096];
    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            trace!("peer closed connection");
            break;
        }
        trace!("received {} bytes (protocol parse not yet wired)", n);
    }
    let _ = stream.shutdown().await;
    Ok(())
}

async fn handle_login_connection(
    mut stream: TcpStream,
    cfg: LoginWireConfig,
) -> anyhow::Result<()> {
    trace!(
        version = %cfg.protocol_version,
        "login connection accept"
    );
    let first_body = match read_sized_payload(&mut stream).await {
        Ok(Some(b)) => b,
        Ok(None) => return Ok(()),
        Err(e) => return Err(e.into()),
    };

    let parsed = parse_first_client_packet(&first_body, &cfg.rsa_private_key, &cfg.protocol_caps)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let FirstClientPacket::Login {
        xtea_key,
        identity,
        password,
        operating_system: _,
        otclient_v8: _,
    } = parsed
    else {
        let _ = stream.shutdown().await;
        return Err(anyhow::anyhow!("login port: expected login packet"));
    };

    let round_keys = expand_key(&xtea_key);
    // Account identity is capability-gated: 1098 logs in by account name, 772 by account number
    // (`docs/PROTOCOL_VERSIONING.md` §2.2). Auth is the only version-specific DB surface (§3.4).
    let auth = match &identity {
        LoginIdentity::AccountName(name) => {
            tfs_rust_db::loginserver_authentication(&cfg.db, &cfg.password_hash, name, &password)
                .await
        }
        LoginIdentity::AccountNumber(number) => {
            tfs_rust_db::loginserver_authentication_by_number(
                &cfg.db,
                &cfg.password_hash,
                *number,
                &password,
            )
            .await
        }
    }
    .map_err(|e| anyhow::anyhow!(e))?;

    let (read_half, mut write_half) = stream.into_split();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let ticks = (now / 30) as u32;

    let account_display = identity.as_display();
    let plain = match auth {
        None => build_login_error(
            &cfg.protocol_caps,
            "Account name or password is not correct.",
        ),
        Some((_, chars, prem)) => build_login_success(
            &cfg.protocol_caps,
            &LoginSuccess {
                motd_num: cfg.motd_num,
                motd: &cfg.motd,
                account_name: &account_display,
                password: &password,
                token: "",
                ticks,
                server_name: &cfg.server_name,
                ip: &cfg.public_ip,
                game_port: cfg.game_port,
                characters: &chars,
                premium_ends_at: prem,
                free_premium: cfg.free_premium,
                now_unix: now,
            },
        ),
    };

    let frame = encrypt_xtea_game_frame(&plain, &round_keys, &cfg.protocol_caps);
    write_half.write_all(&frame).await?;
    write_half.flush().await?;
    drop(write_half);
    drop(read_half);
    Ok(())
}

async fn handle_game_connection(stream: TcpStream, wire: GameWireConfig) -> anyhow::Result<()> {
    // C++ `server.cpp` ~163: `acceptor->set_option(boost::asio::ip::tcp::no_delay(true))`.
    // Disables Nagle — small move packets hit the wire immediately instead of waiting for
    // delayed ACKs (~40ms). Critical for walk smoothness.
    let _ = stream.set_nodelay(true);
    let conn_id = ConnId(NEXT_CONN_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    trace!(
        conn_id = conn_id.0,
        version = %wire.protocol_version,
        "game connection accept"
    );
    let mut stream = BufWriter::new(stream);

    let challenge: Option<GameChallenge> =
        send_game_challenge(&mut stream, &wire.protocol_caps).await?;
    stream.flush().await?;

    let mut stream = stream.into_inner();

    let first_body = match read_sized_payload(&mut stream).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            let _ = stream.shutdown().await;
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    let parsed = parse_first_client_packet(&first_body, &wire.rsa_private_key, &wire.protocol_caps)
        .map_err(|e| {
            eprintln!(
                "[tfs-rust-net] conn {}: first packet parse: {}",
                conn_id.0, e
            );
            anyhow::anyhow!(e)
        })?;

    let game = match parsed {
        FirstClientPacket::Game(g) => g,
        FirstClientPacket::Login { .. } => {
            return Err(anyhow::anyhow!(
                "game port: received login-shaped packet; use the login port for account login"
            ));
        }
    };

    // 1098 echoes the `0x1F` challenge in the first game packet; 772 sends none, so only verify the
    // echo when we actually issued a challenge (`caps.prelogin_challenge`).
    if let Some(challenge) = challenge {
        if game.challenge_ts != challenge.timestamp || game.challenge_rand != challenge.random {
            return Err(anyhow::anyhow!("game login challenge mismatch"));
        }
    }

    let acc = match &game.identity {
        LoginIdentity::AccountName(name) => tfs_rust_db::gameworld_authentication(
            &wire.db,
            &wire.password_hash,
            name,
            &game.password,
            &game.character_name,
        )
        .await,
        LoginIdentity::AccountNumber(number) => tfs_rust_db::gameworld_authentication_by_number(
            &wire.db,
            &wire.password_hash,
            *number,
            &game.password,
            &game.character_name,
        )
        .await,
    }
    .map_err(|e| anyhow::anyhow!(e))?;
    if acc.is_none() {
        return Err(anyhow::anyhow!(
            "Account name or password is not correct (gameworld auth)."
        ));
    }

    info!(
        conn_id = conn_id.0,
        account = %game.identity.as_display(),
        character = %game.character_name,
        "game port: authenticated; handing session to game loop"
    );

    let xtea_key = game.xtea_key;
    let character_name = game.character_name.clone();

    let (read_half, mut write_half) = stream.into_split();
    let round_keys = expand_key(&xtea_key);
    let caps = wire.protocol_caps;

    let (batch_tx, mut batch_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<Vec<u8>>>();
    {
        let mut g = wire.out_registry.lock().expect("out_registry lock");
        g.insert(conn_id, batch_tx);
    }

    tokio::spawn(async move {
        while let Some(blobs) = batch_rx.recv().await {
            for b in blobs {
                let frame = encrypt_xtea_game_frame(&b, &round_keys, &caps);
                if write_half.write_all(&frame).await.is_err() {
                    break;
                }
            }
            let _ = write_half.flush().await;
        }
        let _ = write_half.shutdown().await;
    });

    wire.cmd_tx
        .send(GameCommand::PlayerLogin {
            conn_id,
            name: character_name,
            operating_system: game.operating_system,
            otclient_v8: game.otclient_v8,
        })
        .map_err(|_| anyhow::anyhow!("game command channel closed"))?;

    forward_game_packets_xtea(
        read_half,
        conn_id,
        wire.cmd_tx,
        &round_keys,
        wire.protocol_version,
        &caps,
    )
    .await?;
    let mut g = wire.out_registry.lock().expect("out_registry lock");
    g.remove(&conn_id);
    Ok(())
}
