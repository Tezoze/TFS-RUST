use anyhow::Context;

/// Default I/O worker count. TVP uses one Asio `io_service` thread; decompile is
/// one comm thread per client (do not copy). `#[tokio::main]` defaults to SMT
/// count (16 on 7800X3D) and the 1000-bot cell spent ~81 CPU-s there vs ~17 on
/// `game` (`results/20260920T101600Z`). Not a bcrypt-worker cap (lesson 479).
const DEFAULT_IO_WORKER_THREADS: usize = 2;
/// Bound `spawn_blocking` (bcrypt upgrade in production). Default pool is 512.
const IO_MAX_BLOCKING_THREADS: usize = 16;

fn io_worker_threads() -> usize {
    std::env::var("TFS_IO_WORKER_THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n| n >= 1)
        .unwrap_or(DEFAULT_IO_WORKER_THREADS)
}

fn main() -> anyhow::Result<()> {
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

    let workers = io_worker_threads();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .max_blocking_threads(IO_MAX_BLOCKING_THREADS)
        .thread_name("io")
        .enable_all()
        .build()
        .context("io runtime")?;
    rt.block_on(tfs_rust_core::run())
}
