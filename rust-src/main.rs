#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("info,tfs_rust_core=info,tfs_rust_net=info")
            }),
        )
        .init();
    tfs_rust_core::run().await
}
