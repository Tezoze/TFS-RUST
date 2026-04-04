use crate::protocol::ConnectionState;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, trace};

pub struct Server {
    listener: TcpListener,
}

impl Server {
    pub async fn bind(addr: &str) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        info!("TFS Rust Server listening on {}", addr);
        Ok(Self { listener })
    }

    /// Wrap an already-bound listener (e.g. after choosing among candidate addresses in an example).
    pub fn from_listener(listener: TcpListener) -> Self {
        Self { listener }
    }

    pub fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.listener.local_addr()
    }

    pub async fn accept_loop(&mut self) {
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    // stderr: visible without RUST_LOG (smoke tests / OTClient debugging).
                    eprintln!("[tfs-rust-net] accepted TCP connection from {}", addr);
                    info!("New connection from {}", addr);
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream).await {
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

/// Keeps the socket open and drains inbound data until the client closes.
/// Used for manual OTClient / task 1.13 smoke tests before the full login protocol is wired to `run()`.
async fn handle_connection(mut stream: TcpStream) -> anyhow::Result<()> {
    let mut _state = ConnectionState::Handshake;
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
