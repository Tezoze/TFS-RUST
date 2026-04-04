//! TCP smoke listener for manual OTClient checks (task 1.13).
//!
//! Run: `cargo run -p tfs-rust-net --example tcp_smoke`
//!
//! We try several bind addresses in order so local clients can connect whether they use
//! `127.0.0.1`, `localhost` → `::1`, or need `0.0.0.0` (e.g. some firewall setups).

use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    eprintln!("Starting tcp_smoke (Ctrl+C to stop)…");

    let candidates: &[&str] = &["127.0.0.1:7171", "[::1]:7171", "0.0.0.0:7171"];

    let mut server = None;
    for addr in candidates {
        match TcpListener::bind(addr).await {
            Ok(l) => {
                let bound = l.local_addr()?;
                eprintln!(
                    "Bound successfully — local address: {} (tried {})",
                    bound, addr
                );
                eprintln!(
                    "OTClient: use host 127.0.0.1 if you bound to 127.0.0.1; use localhost or ::1 if bound to [::1]; port 7171."
                );
                server = Some(tfs_rust_net::Server::from_listener(l));
                break;
            }
            Err(e) => {
                eprintln!("Bind {} failed: {}", addr, e);
            }
        }
    }

    let mut server = server.ok_or_else(|| {
        anyhow::anyhow!(
            "could not bind port 7171 on any candidate address — is another server using 7171?"
        )
    })?;

    if let Ok(a) = server.local_addr() {
        eprintln!(
            "Listening as {:?}. Connect from OTClient; you should see a line when TCP accepts.",
            a
        );
    }

    server.accept_loop().await;
    Ok(())
}
