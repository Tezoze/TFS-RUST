#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // tfs_obs=off: mute 10s game_obs_summary; opt-in with RUST_LOG=…,tfs_obs=info
                tracing_subscriber::EnvFilter::new(
                    "info,tfs_rust_core=info,tfs_rust_net=info,tfs_obs=off",
                )
            }),
        )
        .init();
    tfs_rust_core::run().await
}
