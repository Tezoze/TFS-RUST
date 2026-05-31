//! Tokio-driven game loop: command drain + `GameWorld::tick`.
// C++ reference: `Game::gameLoop`, `ServiceManager::threadFunc`.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use tokio::signal;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinSet;
use tokio::time::{interval, MissedTickBehavior};

use crate::ids::CreatureId;

use tfs_rust_common::{GameCommand, GamePacket};
use tokio::sync::mpsc::error::TryRecvError;
use tracing::{error, info, trace, warn};

use crate::game_world::GameWorld;
use crate::stability::ErrorCategory;
use tfs_rust_db::player::PlayerStore;
use tfs_rust_net::OutRegistry;

/// Persist every player still tied to a live game connection. Used for SIGINT / graceful shutdown
/// (awaited; not fire-and-forget). Bounded concurrency to limit DB load.
// C++ ref: `src/game.cpp` `Game::saveGameState`
async fn flush_online_players_to_db(world: &GameWorld) -> anyhow::Result<()> {
    let cids: Vec<CreatureId> = world.conn_to_creature.values().copied().collect();
    let mut datas = Vec::with_capacity(cids.len());
    for cid in cids {
        match world.build_player_save_data(cid) {
            Ok(d) => datas.push(d),
            Err(e) => {
                warn!(?e, ?cid, "build_player_save_data failed during shutdown flush");
            }
        }
    }
    if datas.is_empty() {
        return Ok(());
    }
    let n = datas.len();
    let db = world.db.clone();
    const MAX_IN_FLIGHT: usize = 8;
    let mut set = JoinSet::new();
    let mut any_err = false;
    for data in datas {
        while set.len() >= MAX_IN_FLIGHT {
            if let Some(j) = set.join_next().await {
                match j {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        any_err = true;
                        error!(?e, "player save on shutdown failed");
                    }
                    Err(e) => {
                        any_err = true;
                        error!(?e, "shutdown save task join error");
                    }
                }
            }
        }
        let dpool = db.clone();
        set.spawn(async move { PlayerStore::new(&dpool).save_player(&data).await });
    }
    while let Some(j) = set.join_next().await {
        match j {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                any_err = true;
                error!(?e, "player save on shutdown failed");
            }
            Err(e) => {
                any_err = true;
                error!(?e, "shutdown save task join error");
            }
        }
    }
    if any_err {
        anyhow::bail!("shutdown flush: one or more player saves failed (see error logs above)");
    }
    info!(saved = n, "shutdown: flushed online players to DB");
    Ok(())
}

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

        // Container navigation + `UseItem` to open bags (`player_use_item` / `container_ui.rs`).
        GamePacket::CloseContainer { .. }
        | GamePacket::UpArrowContainer { .. }
        | GamePacket::UpdateContainer { .. }
        | GamePacket::SeekInContainer { .. }
        // `player_use_item` defers on `nextAction` like TFS `setNextActionTask` — do not drop silently.
        | GamePacket::UseItem(_)
        | GamePacket::UseItemEx(_) => false,

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

/// Movement / facing packets that must hit the wire before the next 50ms tick.
// C++ reference: walk + turn replies are sent from the dispatcher immediately, not batched to tick end.
fn game_packet_needs_immediate_flush(packet: &GamePacket) -> bool {
    matches!(
        packet,
        GamePacket::Move(_)
            | GamePacket::AutoWalk { .. }
            | GamePacket::StopAutoWalk
            | GamePacket::Turn(_)
            | GamePacket::Ping
            | GamePacket::PingBack
            | GamePacket::UseItem(_)
            | GamePacket::UseItemEx(_)
    )
}

/// Process incoming commands until shutdown; runs world ticks on a fixed interval.
///
/// `out_registry`: drained once per tick; also after walk/login/disconnect and movement packets
/// (see [`game_packet_needs_immediate_flush`]).
///
/// `walk_wake_rx`: one-shot walk wakes from `tokio::time::sleep_until` (`src/scheduler.cpp`); pairs with
/// [`GameWorld::walk_wake_tx`].
pub async fn run_game_loop(
    mut world: GameWorld,
    mut cmd_rx: UnboundedReceiver<GameCommand>,
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
                    Some(GameCommand::Shutdown) => {
                        flush_online_players_to_db(&world).await?;
                        break;
                    }
                    None => break,
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
                                flush_pending_outgoing(&mut world, &out_registry);
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
                    Some(GameCommand::PlayerDisconnect {
                        conn_id,
                        display_effect,
                    }) => {
                        // C++ ProtocolGame::disconnect() flow:
                        // 1. Send logout effect (optional)
                        // 2. Flush remaining packets
                        // 3. Remove from registry to close TCP connection
                        if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                            if display_effect {
                                let pos = world.creatures.get(cid).map(|k| k.position());
                                if let Some(p) = pos {
                                    world.broadcast_magic_effect(p, 4); // CONST_ME_POFF
                                }
                            }
                            // C++ `Game::playerLogout` → `IOLoginData::savePlayer` (async offload).
                            // C++ ref: `src/iologindata.cpp` `savePlayer`
                            let db = world.db.clone();
                            match world.build_player_save_data(cid) {
                                Ok(data) => {
                                    let guid = data.player.id;
                                    tokio::spawn(async move {
                                        if let Err(e) = PlayerStore::new(&db).save_player(&data).await
                                        {
                                            tracing::error!(
                                                ?e,
                                                guid,
                                                "player save on disconnect failed"
                                            );
                                        }
                                    });
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        ?e,
                                        ?cid,
                                        "build_player_save_data failed — disconnect continues"
                                    );
                                }
                            }
                            world.remove_creature(cid);
                        }
                        // Flush any pending packets (including the effect)
                        flush_pending_outgoing(&mut world, &out_registry);
                        // Remove from registry to close connection
                        world.conn_to_creature.remove(&conn_id);
                        world.known_creatures_by_conn.remove(&conn_id);
                        world.creature_fully_sent_by_conn.remove(&conn_id);
                        if let Some(reg) = out_registry.as_ref() {
                            if let Ok(mut g) = reg.lock() {
                                g.remove(&conn_id); // This drops batch_tx, causing writer task to close TCP
                            }
                        }
                        trace!(conn_id = conn_id.0, "player disconnected");
                    }
                    Some(GameCommand::Game { conn_id, packet }) => {
                        let now = Instant::now();
                        let immediate_flush = game_packet_needs_immediate_flush(&packet);
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
                            GamePacket::Ping => {
                                if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                                    world.player_receive_ping(conn_id, cid, now);
                                }
                            }
                            GamePacket::PingBack => {
                                if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                                    world.player_receive_ping_back(conn_id, cid);
                                }
                            }
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
                            // B.5: Throw (item/creature move)
                            GamePacket::Throw(payload) => {
                                if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                                    world.player_move_thing(
                                        conn_id,
                                        cid,
                                        payload.from_pos,
                                        payload.sprite_id,
                                        payload.from_stack_pos,
                                        payload.to_pos,
                                        payload.count,
                                        now,
                                    );
                                }
                            }
                            GamePacket::UseItem(payload) => {
                                if let Some(creature_id) = world.conn_to_creature.get(&conn_id).copied() {
                                    world.player_use_item(conn_id, creature_id, payload, now);
                                }
                            }
                            GamePacket::UseItemEx(payload) => {
                                if let Some(creature_id) = world.conn_to_creature.get(&conn_id).copied() {
                                    world.player_use_item_ex(conn_id, creature_id, payload, now);
                                }
                            }
                            // B.7: Container UI packets
                            GamePacket::CloseContainer { cid: client_cid } => {
                                if let Some(creature_id) = world.conn_to_creature.get(&conn_id).copied() {
                                    world.player_close_container(conn_id, creature_id, client_cid);
                                }
                            }
                            GamePacket::UpArrowContainer { cid: client_cid } => {
                                if let Some(creature_id) = world.conn_to_creature.get(&conn_id).copied() {
                                    world.player_up_container(conn_id, creature_id, client_cid);
                                }
                            }
                            GamePacket::UpdateContainer { cid: client_cid } => {
                                if let Some(creature_id) = world.conn_to_creature.get(&conn_id).copied() {
                                    world.player_update_container(conn_id, creature_id, client_cid);
                                }
                            }
                            GamePacket::SeekInContainer { cid: client_cid, index } => {
                                if let Some(creature_id) = world.conn_to_creature.get(&conn_id).copied() {
                                    world.player_seek_in_container(conn_id, creature_id, client_cid, index);
                                }
                            }
                            GamePacket::EquipObject { sprite_id } => {
                                if let Some(creature_id) = world.conn_to_creature.get(&conn_id).copied() {
                                    world.player_quick_equip(conn_id, creature_id, sprite_id);
                                }
                            }
                            GamePacket::LookAt { pos, stack_pos } => {
                                if let Some(creature_id) = world.conn_to_creature.get(&conn_id).copied() {
                                    world.player_look_at(conn_id, creature_id, pos, stack_pos);
                                }
                            }
                            // Logout packet (0x14) - client requests logout
                            GamePacket::Logout => {
                                // C++: ProtocolGame::logout(displayEffect=true, forced=false)
                                // Send disconnect command to properly close connection
                                pending.push_back(GameCommand::PlayerDisconnect {
                                    conn_id,
                                    display_effect: true,
                                });
                            }
                            _ => trace!(conn_id = conn_id.0, ?packet, "game packet — simulation Phase 9+"),
                        }
                        world.process_walk_deadlines();
                        if immediate_flush {
                            flush_pending_outgoing(&mut world, &out_registry);
                        }
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

/// Reserved for a future "save houses / close pool" pass after the game thread stops.
/// Online player persistence is handled inside [`run_game_loop`] on [`GameCommand::Shutdown`]
/// (bounded concurrent `PlayerStore::save_player`), with SIGINT in `run_server` sending that command.
// C++ reference: `Game::saveGameState` (player portion implemented in the game loop, not here).
pub async fn graceful_shutdown(_db: &tfs_rust_db::DbPool) -> anyhow::Result<()> {
    let _ = _db;
    Ok(())
}

#[cfg(test)]
mod timed_action_gate_tests {
    use tfs_rust_common::enums::Direction;
    use tfs_rust_common::game_packet::GamePacket;

    use super::{game_packet_needs_immediate_flush, game_packet_requires_timed_action};

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
    fn attack_is_gated() {
        assert!(game_packet_requires_timed_action(&GamePacket::Attack {
            creature_id: 1
        }));
    }

    #[test]
    fn use_item_defers_to_handler_not_game_loop_gate() {
        assert!(!game_packet_requires_timed_action(&GamePacket::UseItem(
            tfs_rust_common::game_packet::UseItemPayload {
                pos: tfs_rust_common::Position::new(0, 0, 7),
                sprite_id: 100,
                stack_pos: 0,
                index: 0,
            }
        )));
    }

    #[test]
    fn movement_packets_flush_immediately() {
        assert!(game_packet_needs_immediate_flush(&GamePacket::Move(Direction::North)));
        assert!(game_packet_needs_immediate_flush(&GamePacket::Turn(Direction::West)));
        assert!(game_packet_needs_immediate_flush(&GamePacket::UseItem(
            tfs_rust_common::game_packet::UseItemPayload {
                pos: tfs_rust_common::Position::new(0, 0, 7),
                sprite_id: 100,
                stack_pos: 0,
                index: 0,
            }
        )));
    }
}
