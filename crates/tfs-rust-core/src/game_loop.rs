//! Tokio-driven game loop: command drain + `GameWorld::tick`.
// C++ reference: `Game::gameLoop`, `ServiceManager::threadFunc`.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use tokio::signal;
use tokio::sync::mpsc::{Receiver, UnboundedReceiver};
use tokio::time::{interval, MissedTickBehavior};

use crate::ids::CreatureId;

use tfs_rust_common::{GameCommand, GamePacket};
use tokio::sync::mpsc::error::TryRecvError;
use tracing::{trace, warn};

use crate::game_world::GameWorld;
use crate::stability::ErrorCategory;
use tfs_rust_net::OutRegistry;

fn flush_pending_outgoing(world: &mut GameWorld, out_registry: &Option<OutRegistry>) {
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
}

/// TFS `Player::canDoAction` / `nextAction` — packets that must **not** run while the step lockout
/// is active (`player.cpp`). Walk, turn, ping, and most UI/look/channel packets stay ungated; the
/// default is **gated** for gameplay (use/attack/trade/etc.).  
// C++ reference: per-handler checks in `game.cpp` / `player.cpp`; refine when each opcode is ported.
fn game_packet_requires_timed_action(packet: &GamePacket) -> bool {
    match packet {
        // Connection / keepalive / movement scheduler
        GamePacket::EnterGame
        | GamePacket::Logout
        | GamePacket::Ping
        | GamePacket::PingBack
        | GamePacket::Move(_)
        | GamePacket::AutoWalk { .. }
        | GamePacket::StopAutoWalk => false,

        // OTC / Lua — must not block on walk step (`ProtocolGame::parsePacket` extended).
        GamePacket::ExtendedOpcode { .. } => false,

        // Facing and combat cancel — allowed like TFS walk pipeline.
        GamePacket::Turn(_) | GamePacket::CancelAttackAndFollow => false,

        // Fight mode toggles — not tied to `nextAction` in TFS `parseFightModes`.
        GamePacket::FightModes { .. } => false,

        // Inspection / browse (client expects immediate feedback).
        GamePacket::LookAt { .. }
        | GamePacket::LookInBattleList { .. }
        | GamePacket::BrowseField { .. }
        | GamePacket::GetObjectInfo => false,

        // Chat / channel UI (`playerSay` uses level/channel rules — not `nextAction` in TFS 1.4.2).
        GamePacket::Say(_)
        | GamePacket::RequestChannels
        | GamePacket::OpenChannel { .. }
        | GamePacket::CloseChannel { .. }
        | GamePacket::OpenPrivateChannel { .. }
        | GamePacket::CloseNpcChannel => false,

        // Container navigation only (opening a container uses items — still unimplemented).
        GamePacket::CloseContainer { .. }
        | GamePacket::UpArrowContainer { .. }
        | GamePacket::UpdateContainer { .. }
        | GamePacket::SeekInContainer { .. } => false,

        // Client diagnostics / thanks — never gameplay-blocking.
        GamePacket::BugReport(_)
        | GamePacket::ThankYou
        | GamePacket::DebugAssert { .. }
        | GamePacket::QuestLog
        | GamePacket::QuestLine { .. } => false,

        // VIP list edits from client UI (not in-world actions).
        GamePacket::VipAdd { .. } | GamePacket::VipRemove { .. } | GamePacket::VipEdit { .. } => false,

        // Everything else: item use, combat, trade, party, market, etc.
        _ => true,
    }
}

/// Process incoming commands until shutdown; runs world ticks on a fixed interval.
///
/// `out_registry`: when set, each tick forwards `flush_output_buffers()` to per-connection writers (`server.rs`).
///
/// `walk_wake_rx`: one-shot walk wakes from `tokio::time::sleep_until` (`src/scheduler.cpp`); pairs with
/// [`GameWorld::walk_wake_tx`].
pub async fn run_game_loop(
    mut world: GameWorld,
    mut cmd_rx: Receiver<GameCommand>,
    mut walk_wake_rx: UnboundedReceiver<CreatureId>,
    out_registry: Option<OutRegistry>,
) -> anyhow::Result<()> {
    let mut tick_timer = interval(Duration::from_millis(50));
    tick_timer.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut pending: VecDeque<GameCommand> = VecDeque::new();
    loop {
        // `biased`: check branches in order. **cmd before walk_wake** so that a pending direction
        // change (`Move(East)`) updates the walk queue *before* the walk timer fires and pops a step.
        // Without this, when a tick is processing and both a Move cmd and walk_wake become ready
        // simultaneously, walk_wake would win and execute the OLD queue direction — causing a
        // one-step-late direction change visible as stutter. C++ Dispatcher is a FIFO where
        // `playerMove` arriving first always runs before `checkCreatureWalk`.
        // When no commands are pending, `cmd_rx.recv()` awaits, so walk_wake runs immediately.
        tokio::select! {
            biased;

            cmd = async {
                match pending.pop_front() {
                    Some(c) => Some(c),
                    None => cmd_rx.recv().await,
                }
            } => {
                match cmd {
                    Some(GameCommand::Shutdown) | None => break,
                    Some(GameCommand::PlayerLogin {
                        conn_id,
                        name,
                        operating_system,
                        otclient_v8,
                    }) => {
                        match crate::login::login_player(
                            &mut world,
                            &name,
                            operating_system,
                            otclient_v8,
                        )
                        .await
                        {
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
                        let now = Instant::now();
                        if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                            if game_packet_requires_timed_action(&packet)
                                && !world.player_timed_action_ready(cid, now)
                            {
                                trace!(
                                    conn_id = conn_id.0,
                                    ?packet,
                                    "game packet ignored — nextAction lockout (TFS canDoAction)"
                                );
                                continue;
                            }
                        }
                        match packet {
                            GamePacket::EnterGame => {}
                            GamePacket::ExtendedOpcode { opcode, buffer } => {
                                world.protocol_hooks.extended_opcode(conn_id, opcode, buffer);
                            }
                            GamePacket::Move(dir) => {
                                if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                                    world.player_move_request(conn_id, cid, dir, now);
                                }
                            }
                            GamePacket::AutoWalk { path } => {
                                if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                                    world.player_auto_walk_path(conn_id, cid, path, now);
                                }
                            }
                            GamePacket::Turn(dir) => {
                                if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                                    world.player_turn_request(cid, dir, now);
                                    match cmd_rx.try_recv() {
                                        Ok(next) => match next {
                                            GameCommand::Game {
                                                conn_id: next_conn,
                                                packet: next_pkt,
                                            } if next_conn == conn_id => {
                                                match next_pkt {
                                                    GamePacket::Move(d) => {
                                                        world.flush_deferred_turn_broadcast(cid);
                                                        world.player_move_request(
                                                            conn_id, cid, d, now,
                                                        );
                                                    }
                                                    GamePacket::AutoWalk { path } => {
                                                        world.flush_deferred_turn_broadcast(cid);
                                                        world.player_auto_walk_path(
                                                            conn_id, cid, path, now,
                                                        );
                                                    }
                                                    other => {
                                                        world.flush_deferred_turn_broadcast(cid);
                                                        pending.push_back(GameCommand::Game {
                                                            conn_id: next_conn,
                                                            packet: other,
                                                        });
                                                    }
                                                }
                                            }
                                            other => {
                                                world.flush_deferred_turn_broadcast(cid);
                                                pending.push_back(other);
                                            }
                                        },
                                        Err(TryRecvError::Empty) => {
                                            world.flush_deferred_turn_broadcast(cid);
                                        }
                                        Err(TryRecvError::Disconnected) => {
                                            world.flush_deferred_turn_broadcast(cid);
                                        }
                                    }
                                }
                            }
                            GamePacket::StopAutoWalk => {
                                if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                                    world.player_stop_auto_walk(cid);
                                }
                            }
                            _ => trace!(conn_id = conn_id.0, ?packet, "game packet — simulation Phase 9+"),
                        }
                        world.process_walk_deadlines();
                        flush_pending_outgoing(&mut world, &out_registry);
                    }
                }
            }
            w = walk_wake_rx.recv() => {
                let Some(cid) = w else {
                    break;
                };
                world.process_walk_due_from_wake(cid);
                // Flush immediately so move packets hit the wire in the same iteration as `on_walk`
                // (`tasks/walk-audit.md` Issue 2).
                flush_pending_outgoing(&mut world, &out_registry);
            }
            _ = tick_timer.tick() => {
                let t0 = Instant::now();
                // Do not `try_recv`+drop here — that used to discard `Game` packets (walk, etc.) and
                // `PlayerLogin` whenever the tick branch ran first (`game_loop.rs` drain bug).
                world.on_tick(Instant::now());
                flush_pending_outgoing(&mut world, &out_registry);
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

#[cfg(test)]
mod timed_action_gate_tests {
    use tfs_rust_common::enums::Direction;
    use tfs_rust_common::game_packet::GamePacket;

    use super::game_packet_requires_timed_action;

    #[test]
    fn walk_ping_and_extended_are_never_gated() {
        assert!(!game_packet_requires_timed_action(&GamePacket::Move(Direction::North)));
        assert!(!game_packet_requires_timed_action(&GamePacket::AutoWalk {
            path: Vec::new()
        }));
        assert!(!game_packet_requires_timed_action(&GamePacket::StopAutoWalk));
        assert!(!game_packet_requires_timed_action(&GamePacket::Ping));
        assert!(!game_packet_requires_timed_action(&GamePacket::ExtendedOpcode {
            opcode: 1,
            buffer: String::new(),
        }));
        assert!(!game_packet_requires_timed_action(&GamePacket::Say(
            tfs_rust_common::game_packet::SayPayload {
                speak_class: 1,
                channel_id: 0,
                receiver: String::new(),
                text: "hi".into(),
            }
        )));
    }

    #[test]
    fn attack_and_use_item_are_gated() {
        assert!(game_packet_requires_timed_action(&GamePacket::Attack {
            creature_id: 1
        }));
        assert!(game_packet_requires_timed_action(&GamePacket::UseItem(
            tfs_rust_common::game_packet::UseItemPayload {
                pos: tfs_rust_common::Position::new(0, 0, 7),
                sprite_id: 100,
                stack_pos: 0,
                index: 0,
            }
        )));
    }
}
