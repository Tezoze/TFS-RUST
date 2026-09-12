//! Null echo server: accept `[u16 LE][body]`, discard. Loadgen ceiling target.

use anyhow::Result;
use clap::Parser;
use tfs_rust_net::game_frame::read_sized_payload;
use tokio::net::TcpListener;

#[derive(Parser, Debug)]
#[command(name = "tfs-loadgen-echo", about = "Length-framed discard server")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:17171")]
    listen: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let listener = TcpListener::bind(&args.listen).await?;
    eprintln!("tfs-loadgen-echo listening on {}", args.listen);
    loop {
        let (mut sock, _) = listener.accept().await?;
        tokio::spawn(async move { while let Ok(Some(_)) = read_sized_payload(&mut sock).await {} });
    }
}
