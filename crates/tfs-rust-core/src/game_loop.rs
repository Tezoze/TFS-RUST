//! Tokio-driven game loop: command drain + `GameWorld::tick`.
// C++ reference: `Game::gameLoop`, `ServiceManager::threadFunc`.

use std::time::{Duration, Instant};

use tokio::signal;
use tokio::sync::mpsc::Receiver;
use tokio::time::{interval, MissedTickBehavior};

use tfs_rust_common::GameCommand;
use tracing::warn;

use crate::game_world::GameWorld;
use crate::stability::ErrorCategory;

/// Process incoming commands until shutdown; runs world ticks on a fixed interval.
pub async fn run_game_loop(
    mut world: GameWorld,
    mut cmd_rx: Receiver<GameCommand>,
) -> anyhow::Result<()> {
    let mut tick_timer = interval(Duration::from_millis(50));
    tick_timer.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(GameCommand::Shutdown) | None => break,
                    Some(_other) => {
                        // Phase 5+: dispatch movement, speech, etc.
                    }
                }
            }
            _ = tick_timer.tick() => {
                let t0 = Instant::now();
                for _ in 0..256 {
                    match cmd_rx.try_recv() {
                        Ok(GameCommand::Shutdown) => return Ok(()),
                        Ok(_) => {}
                        Err(_) => break,
                    }
                }
                world.on_tick();
                let elapsed = t0.elapsed();
                if elapsed > Duration::from_millis(45) {
                    warn!(?elapsed, "game tick exceeded 45ms budget");
                }
                if elapsed > Duration::from_millis(50) {
                    world.stability.record_error(ErrorCategory::TickOverrun);
                }
            }
        }
    }
    Ok(())
}

/// Wait for Ctrl+C (SIGINT) — SIGTERM requires more setup on some platforms.
pub async fn wait_for_shutdown_signal() -> anyhow::Result<()> {
    signal::ctrl_c().await?;
    Ok(())
}

/// Stop accepting work, persist players, flush DB — full wiring in Phase 5/10.
// C++ reference: `Game::saveGameState`, `Dispatcher::shutdown`.
pub async fn graceful_shutdown(_db: &tfs_rust_db::DbPool) -> anyhow::Result<()> {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        // Phase 5+: save all online players, house data, then close pool.
    })
    .await
    .map_err(|_| anyhow::anyhow!("shutdown timed out after 30s"))?;
    Ok(())
}
