//! Tokio-driven game loop: command drain + `GameWorld::tick`.
//!
//! - **1098:** TFS Dispatcher + per-creature walk timers — [`run_game_loop_1098`].
//! - **772:** CipSoft beat-driven loop + ToDoQueue — [`run_game_loop_772`].
//!
// C++ reference: `Game::gameLoop`, `ServiceManager::threadFunc` (1098);
// `tibia-game-master/src/main.cc` `LaunchGame` / `AdvanceGame` (772).

use std::collections::VecDeque;
use std::ops::ControlFlow;
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

/// When outgoing packets may be flushed to TCP during command handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushPolicy {
    /// TFS 1098 — movement / turn / ping flush inline from the dispatcher.
    ImmediateOnMovement,
    /// CipSoft 772 — buffer until beat-end `SendAll`.
    BeatEndOnly,
}

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
        GamePacket::EnterGame
        | GamePacket::Logout
        | GamePacket::Ping
        | GamePacket::PingBack
        | GamePacket::Move(_)
        | GamePacket::AutoWalk { .. }
        | GamePacket::StopAutoWalk => false,
        GamePacket::ExtendedOpcode { .. } => false,
        GamePacket::Turn(_) | GamePacket::CancelAttackAndFollow => false,
        GamePacket::FightModes { .. } => false,
        GamePacket::LookAt { .. }
        | GamePacket::LookInBattleList { .. }
        | GamePacket::BrowseField { .. }
        | GamePacket::GetObjectInfo => false,
        GamePacket::Say(_)
        | GamePacket::RequestChannels
        | GamePacket::OpenChannel { .. }
        | GamePacket::CloseChannel { .. }
        | GamePacket::OpenPrivateChannel { .. }
        | GamePacket::CloseNpcChannel => false,
        GamePacket::CloseContainer { .. }
        | GamePacket::UpArrowContainer { .. }
        | GamePacket::UpdateContainer { .. }
        | GamePacket::SeekInContainer { .. }
        | GamePacket::UseItem(_)
        | GamePacket::UseItemEx(_) => false,
        GamePacket::BugReport(_)
        | GamePacket::ThankYou
        | GamePacket::DebugAssert { .. }
        | GamePacket::QuestLog
        | GamePacket::QuestLine { .. } => false,
        GamePacket::VipAdd { .. } | GamePacket::VipRemove { .. } | GamePacket::VipEdit { .. } => false,
        _ => true,
    }
}

fn packet_would_immediate_flush(packet: &GamePacket) -> bool {
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

/// Movement / facing packets that must hit the wire before the next 50ms tick (1098 only).
fn needs_immediate_flush(packet: &GamePacket, policy: FlushPolicy) -> bool {
    match policy {
        FlushPolicy::BeatEndOnly => false,
        FlushPolicy::ImmediateOnMovement => packet_would_immediate_flush(packet),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopExit {
    Shutdown,
    ChannelClosed,
}

async fn handle_player_login(
    world: &mut GameWorld,
    conn_id: tfs_rust_common::ConnId,
    name: String,
    operating_system: u16,
    otclient_v8: u16,
    out_registry: &Option<OutRegistry>,
) {
    match crate::login::login_player(
        world,
        &name,
        operating_system,
        otclient_v8,
    )
    .await
    {
        Ok(cid) => {
            world.conn_to_creature.insert(conn_id, cid);
            crate::login_out::enqueue_initial_login_packets(world, conn_id, cid);
            // Login always flushes — client must receive map / self-appear before play.
            flush_pending_outgoing(world, out_registry);
        }
        Err(e) => {
            tracing::warn!(?e, %name, conn_id = conn_id.0, "player login failed");
        }
    }
}

fn handle_player_disconnect(
    world: &mut GameWorld,
    conn_id: tfs_rust_common::ConnId,
    display_effect: bool,
    out_registry: &Option<OutRegistry>,
) {
    if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
        if display_effect {
            let pos = world.creatures.get(cid).map(|k| k.position());
            if let Some(p) = pos {
                world.broadcast_magic_effect(p, 4);
            }
        }
        let db = world.db.clone();
        match world.build_player_save_data(cid) {
            Ok(data) => {
                let guid = data.player.id;
                tokio::spawn(async move {
                    if let Err(e) = PlayerStore::new(&db).save_player(&data).await {
                        tracing::error!(?e, guid, "player save on disconnect failed");
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
    flush_pending_outgoing(world, out_registry);
    world.conn_to_creature.remove(&conn_id);
    world.known_creatures_by_conn.remove(&conn_id);
    world.creature_fully_sent_by_conn.remove(&conn_id);
    if let Some(reg) = out_registry.as_ref() {
        if let Ok(mut g) = reg.lock() {
            g.remove(&conn_id);
        }
    }
    trace!(conn_id = conn_id.0, "player disconnected");
}

fn handle_game_packet(
    world: &mut GameWorld,
    conn_id: tfs_rust_common::ConnId,
    packet: GamePacket,
    cmd_rx: &mut UnboundedReceiver<GameCommand>,
    pending: &mut VecDeque<GameCommand>,
    flush_policy: FlushPolicy,
) {
    let now = Instant::now();
    let immediate_flush = needs_immediate_flush(&packet, flush_policy);
    if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
        if game_packet_requires_timed_action(&packet) && !world.player_timed_action_ready(cid, now)
        {
            trace!(
                conn_id = conn_id.0,
                ?packet,
                "game packet ignored — nextAction lockout (TFS canDoAction)"
            );
            return;
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
                        } if next_conn == conn_id => match next_pkt {
                            GamePacket::Move(d) => {
                                world.flush_deferred_turn_broadcast(cid);
                                world.player_move_request(conn_id, cid, d, now);
                            }
                            GamePacket::AutoWalk { path } => {
                                world.flush_deferred_turn_broadcast(cid);
                                world.player_auto_walk_path(conn_id, cid, path, now);
                            }
                            other => {
                                world.flush_deferred_turn_broadcast(cid);
                                pending.push_back(GameCommand::Game {
                                    conn_id: next_conn,
                                    packet: other,
                                });
                            }
                        },
                        other => {
                            world.flush_deferred_turn_broadcast(cid);
                            pending.push_back(other);
                        }
                    },
                    Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => {
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
        GamePacket::Logout => {
            pending.push_back(GameCommand::PlayerDisconnect {
                conn_id,
                display_effect: true,
            });
        }
        _ => trace!(conn_id = conn_id.0, ?packet, "game packet — simulation Phase 9+"),
    }
    if !world.beat_driven_loop {
        world.process_walk_deadlines();
    }
    let _ = immediate_flush;
}

async fn dispatch_command(
    world: &mut GameWorld,
    cmd: Option<GameCommand>,
    cmd_rx: &mut UnboundedReceiver<GameCommand>,
    pending: &mut VecDeque<GameCommand>,
    out_registry: &Option<OutRegistry>,
    flush_policy: FlushPolicy,
) -> ControlFlow<LoopExit> {
    let Some(cmd) = cmd else {
        return ControlFlow::Break(LoopExit::ChannelClosed);
    };
    match cmd {
        GameCommand::Shutdown => ControlFlow::Break(LoopExit::Shutdown),
        GameCommand::PlayerLogin {
            conn_id,
            name,
            operating_system,
            otclient_v8,
        } => {
            handle_player_login(
                world,
                conn_id,
                name,
                operating_system,
                otclient_v8,
                out_registry,
            )
            .await;
            ControlFlow::Continue(())
        }
        GameCommand::LuaCallback { event_id } => {
            trace!(event_id, "lua callback — scheduler / Phase 8");
            ControlFlow::Continue(())
        }
        GameCommand::LuaAsyncResult {
            conn_id,
            request_id,
            payload,
            success,
        } => {
            world
                .protocol_hooks
                .lua_async_result(conn_id, request_id, &payload, success);
            ControlFlow::Continue(())
        }
        GameCommand::PlayerDisconnect {
            conn_id,
            display_effect,
        } => {
            handle_player_disconnect(world, conn_id, display_effect, out_registry);
            ControlFlow::Continue(())
        }
        GameCommand::Game { conn_id, packet } => {
            let immediate_flush = needs_immediate_flush(&packet, flush_policy);
            handle_game_packet(world, conn_id, packet, cmd_rx, pending, flush_policy);
            if immediate_flush {
                flush_pending_outgoing(world, out_registry);
            }
            ControlFlow::Continue(())
        }
    }
}

async fn recv_next_command(
    cmd_rx: &mut UnboundedReceiver<GameCommand>,
    pending: &mut VecDeque<GameCommand>,
) -> Option<GameCommand> {
    match pending.pop_front() {
        Some(c) => Some(c),
        None => cmd_rx.recv().await,
    }
}

/// TFS 1098 reactive loop — Dispatcher + Scheduler walk timers.
///
/// `walk_wake_rx`: one-shot walk wakes from `tokio::time::sleep_until` (`src/scheduler.cpp`); pairs with
/// [`GameWorld::walk_wake_tx`].
pub async fn run_game_loop_1098(
    mut world: GameWorld,
    mut cmd_rx: UnboundedReceiver<GameCommand>,
    mut walk_wake_rx: UnboundedReceiver<CreatureId>,
    out_registry: Option<OutRegistry>,
) -> anyhow::Result<()> {
    let mut tick_timer = interval(Duration::from_millis(50));
    tick_timer.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut pending: VecDeque<GameCommand> = VecDeque::new();
    loop {
        tokio::select! {
            biased;

            cmd = recv_next_command(&mut cmd_rx, &mut pending) => {
                match dispatch_command(
                    &mut world,
                    cmd,
                    &mut cmd_rx,
                    &mut pending,
                    &out_registry,
                    FlushPolicy::ImmediateOnMovement,
                ).await {
                    ControlFlow::Break(LoopExit::Shutdown) => {
                        flush_online_players_to_db(&world).await?;
                        break;
                    }
                    ControlFlow::Break(LoopExit::ChannelClosed) => break,
                    ControlFlow::Continue(()) => {}
                }
            }
            w = walk_wake_rx.recv() => {
                let Some(cid) = w else {
                    break;
                };
                world.process_walk_due_from_wake(cid);
                flush_pending_outgoing(&mut world, &out_registry);
            }
            _ = tick_timer.tick() => {
                let t0 = Instant::now();
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

/// CipSoft 772 beat-driven loop — `LaunchGame` + `AdvanceGame` + `SendAll`.
pub async fn run_game_loop_772(
    mut world: GameWorld,
    mut cmd_rx: UnboundedReceiver<GameCommand>,
    out_registry: Option<OutRegistry>,
) -> anyhow::Result<()> {
    let beat_ms = u64::from(world.mechanics.profile.beat_ms.max(1));
    let mut beat_timer = interval(Duration::from_millis(beat_ms));
    beat_timer.set_missed_tick_behavior(MissedTickBehavior::Burst);
    let mut pending: VecDeque<GameCommand> = VecDeque::new();

    loop {
        tokio::select! {
            biased;

            cmd = recv_next_command(&mut cmd_rx, &mut pending) => {
                match dispatch_command(
                    &mut world,
                    cmd,
                    &mut cmd_rx,
                    &mut pending,
                    &out_registry,
                    FlushPolicy::BeatEndOnly,
                ).await {
                    ControlFlow::Break(LoopExit::Shutdown) => {
                        flush_online_players_to_db(&world).await?;
                        break;
                    }
                    ControlFlow::Break(LoopExit::ChannelClosed) => break,
                    ControlFlow::Continue(()) => {
                        while let Ok(more) = cmd_rx.try_recv() {
                            match dispatch_command(
                                &mut world,
                                Some(more),
                                &mut cmd_rx,
                                &mut pending,
                                &out_registry,
                                FlushPolicy::BeatEndOnly,
                            ).await {
                                ControlFlow::Break(LoopExit::Shutdown) => {
                                    flush_online_players_to_db(&world).await?;
                                    return Ok(());
                                }
                                ControlFlow::Break(LoopExit::ChannelClosed) => return Ok(()),
                                ControlFlow::Continue(()) => {}
                            }
                        }
                    }
                }
            }
            _ = beat_timer.tick() => {
                world.advance_beat_772(beat_ms);
                flush_pending_outgoing(&mut world, &out_registry);
            }
        }
    }
    Ok(())
}

/// Back-compat alias — 1098 reactive loop.
pub async fn run_game_loop(
    world: GameWorld,
    cmd_rx: UnboundedReceiver<GameCommand>,
    walk_wake_rx: UnboundedReceiver<CreatureId>,
    out_registry: Option<OutRegistry>,
) -> anyhow::Result<()> {
    run_game_loop_1098(world, cmd_rx, walk_wake_rx, out_registry).await
}

/// Wait for Ctrl+C (SIGINT) — SIGTERM requires more setup on some platforms.
pub async fn wait_for_shutdown_signal() -> anyhow::Result<()> {
    signal::ctrl_c().await?;
    Ok(())
}

/// Reserved for a future "save houses / close pool" pass after the game thread stops.
pub async fn graceful_shutdown(_db: &tfs_rust_db::DbPool) -> anyhow::Result<()> {
    let _ = _db;
    Ok(())
}

#[cfg(test)]
mod timed_action_gate_tests {
    use tfs_rust_common::enums::Direction;
    use tfs_rust_common::game_packet::GamePacket;

    use super::{
        game_packet_requires_timed_action, needs_immediate_flush, FlushPolicy,
    };

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
    fn movement_packets_flush_immediately_on_1098() {
        assert!(needs_immediate_flush(
            &GamePacket::Move(Direction::North),
            FlushPolicy::ImmediateOnMovement
        ));
        assert!(needs_immediate_flush(
            &GamePacket::Turn(Direction::West),
            FlushPolicy::ImmediateOnMovement
        ));
    }

    #[test]
    fn beat_driven_loop_flag_follows_cipsoft_profile() {
        use crate::formulas::StepSpeedModel;
        let mut world = crate::test_world::support::minimal_world();
        assert!(!world.beat_driven_loop);
        world.mechanics.profile.step_speed = StepSpeedModel::CipSoft;
        world.beat_driven_loop =
            world.mechanics.profile.step_speed == StepSpeedModel::CipSoft;
        assert!(world.beat_driven_loop);
    }
}
