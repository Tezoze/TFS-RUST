//! Tokio-driven game loop: command drain + `GameWorld::tick`.
// C++ reference: `Game::gameLoop`, `ServiceManager::threadFunc`.

use std::time::{Duration, Instant};

use tokio::signal;
use tokio::sync::mpsc::Receiver;
use tokio::time::{interval, MissedTickBehavior};

use tfs_rust_common::enums::Direction;
use tfs_rust_common::{ConnId, GameCommand, GamePacket};
use tracing::{trace, warn};

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::stability::ErrorCategory;
use tfs_rust_net::OutRegistry;

/// Process incoming commands until shutdown; runs world ticks on a fixed interval.
///
/// `out_registry`: when set, each tick forwards `flush_output_buffers()` to per-connection writers (`server.rs`).
pub async fn run_game_loop(
    mut world: GameWorld,
    mut cmd_rx: Receiver<GameCommand>,
    out_registry: Option<OutRegistry>,
) -> anyhow::Result<()> {
    let mut tick_timer = interval(Duration::from_millis(50));
    tick_timer.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(GameCommand::Shutdown) | None => break,
                    Some(GameCommand::PlayerLogin { conn_id, name }) => {
                        match crate::login::login_player(&mut world, &name).await {
                            Ok(cid) => {
                                world.conn_to_creature.insert(conn_id, cid);
                                crate::login_out::enqueue_initial_login_packets(&mut world, conn_id, cid);
                            }
                            Err(e) => {
                                tracing::warn!(?e, %name, conn_id = conn_id.0, "player login failed");
                            }
                        }
                    }
                    Some(GameCommand::LuaCallback { event_id }) => {
                        trace!(event_id, "lua callback — scheduler / Phase 8");
                    }
                    Some(GameCommand::LuaAsyncResult {
                        conn_id,
                        request_id,
                        payload,
                        success,
                    }) => {
                        world
                            .protocol_hooks
                            .lua_async_result(conn_id, request_id, &payload, success);
                    }
                    Some(GameCommand::Game { conn_id, packet }) => {
                        match packet {
                            GamePacket::ExtendedOpcode { opcode, buffer } => {
                                world.protocol_hooks.extended_opcode(conn_id, opcode, buffer);
                            }
                            GamePacket::Move(dir) => handle_player_move(&mut world, conn_id, dir),
                            _ => trace!(conn_id = conn_id.0, ?packet, "game packet — simulation Phase 9+"),
                        }
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
                let flushed = world.flush_output_buffers();
                if let Some(reg) = out_registry.as_ref() {
                    if let Ok(g) = reg.lock() {
                        for (conn, blobs) in flushed {
                            if let Some(tx) = g.get(&conn) {
                                let _ = tx.send(blobs);
                            }
                        }
                    }
                } else {
                    trace!(batches = flushed.len(), "flushed outgoing (no registry — packets dropped)");
                }
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

fn handle_player_move(world: &mut GameWorld, conn_id: ConnId, dir: Direction) {
    let Some(cid) = world.conn_to_creature.get(&conn_id).copied() else {
        return;
    };
    let Some(k) = world.creatures.get_mut(cid) else {
        return;
    };
    let CreatureKind::Player(ref mut p) = k else {
        return;
    };
    let old_pos = p.base.position;
    let new_pos = old_pos.offset(dir);
    p.base.direction = dir;
    p.base.position = new_pos;
    world.map.unregister_creature_index(old_pos, cid);
    world.map.register_creature_index(new_pos, cid);
    world.enqueue_outgoing(
        conn_id,
        tfs_rust_net::map_description::send_map_description_stub(new_pos, new_pos).into_bytes(),
    );
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
