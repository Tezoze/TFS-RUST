use crate::protocol::ConnectionState;
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};

pub struct Server {
    listener: TcpListener,
}

impl Server {
    pub async fn bind(addr: &str) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        info!("TFS Rust Server listening on {}", addr);
        Ok(Self { listener })
    }

    pub async fn accept_loop(&mut self) {
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
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

async fn handle_connection(mut _stream: TcpStream) -> anyhow::Result<()> {
    let mut _state = ConnectionState::Handshake;
    // Read loop logic for generic packet framing goes here. We will integrate NetworkMessage reading.
    Ok(())
}
