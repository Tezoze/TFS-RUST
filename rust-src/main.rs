#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tfs_rust_core::run().await
}
