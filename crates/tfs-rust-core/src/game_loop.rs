//! Tokio-driven game loop: command drain + `GameWorld::tick`.
//!
//! - **Single engine, both eras:** beat-driven loop + ToDoQueue — [`run_game_loop`].
//!   Phase 5 deleted the 1098 reactive loop (`run_game_loop_1098`); Phase 7 collapsed the
//!   last `*_772` loop-entry alias into the canonical `run_game_loop`. Per-era differences
//!   live in `MechanicsProfile` / `ProtocolCodec` only.
//!
// C++ reference: `Game::gameLoop`, `ServiceManager::threadFunc` (1098);
// `tibia-game-master/src/main.cc` `LaunchGame` / `AdvanceGame` (772).

use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::ControlFlow;
use std::time::{Duration, Instant};

use tokio::signal;
use tokio::sync::mpsc::{Receiver, UnboundedReceiver};
use tokio::task::JoinSet;
use tokio::time::{MissedTickBehavior, interval_at};

use tfs_rust_common::{ConnId, GameCommand, GamePacket, OwnedPlayerLoad};
use tokio::sync::mpsc::error::TryRecvError;
use tracing::{error, info, trace, warn};

use crate::creature_todo::ActionObjectRef;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::login::{self, MAX_CONCURRENT_LOGIN_LOADS};
use crate::return_value::ReturnValue;
use tfs_rust_db::player::{LoadedPlayerData, PlayerStore};
use tfs_rust_net::{
    GameCmdTx, MAX_GAME_COMMANDS_PER_TURN, OutRegistry, OutboundSendError, OutboundTx,
};

/// Game-thread-owned outbound writers — lock-free flush path (GL-3).
type OutputSinkMap = HashMap<ConnId, OutboundTx>;

/// Game-thread pending commands with receive Instant (OBS-1 command age).
type PendingQueue = VecDeque<(Instant, GameCommand)>;

#[inline]
fn pending_push(pending: &mut PendingQueue, cmd: GameCommand) {
    pending.push_back((Instant::now(), cmd));
}

/// Persist every player still tied to a live game connection. Used for SIGINT / graceful shutdown
/// (awaited; not fire-and-forget).
///
/// Spawns saves onto the multi-thread pool and `.await`s them. Do **not** use
/// `block_in_place` here — the game loop runs on a `LocalSet`, which panics on blocking.
/// Fire-and-forget `spawn` + await of `JoinHandle` is fine: other runtime workers poll DB I/O
/// while this LocalSet task yields.
// C++ ref: `src/game.cpp` `Game::saveGameState`
async fn flush_online_players_to_db(world: &GameWorld) -> anyhow::Result<()> {
    let cids: Vec<CreatureId> = world.conn_to_creature.values().copied().collect();
    let mut datas = Vec::with_capacity(cids.len());
    for cid in cids {
        match world.build_player_save_data(cid) {
            Ok(d) => datas.push(d),
            Err(e) => {
                warn!(
                    ?e,
                    ?cid,
                    "build_player_save_data failed during shutdown flush"
                );
            }
        }
    }
    if datas.is_empty() {
        info!("shutdown: no online players to flush");
        return Ok(());
    }
    let n = datas.len();
    info!(saved = n, "shutdown: flushing online players to DB");
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

async fn handle_pending_save_tick(world: &mut GameWorld) -> anyhow::Result<bool> {
    match world.take_save_tick() {
        crate::server_save::ServerSaveTick::None => Ok(false),
        crate::server_save::ServerSaveTick::FlushStay => {
            if let Err(e) = world.process_and_persist_houses().await {
                tracing::warn!(error = %e, "house process/save on daily save failed");
            }
            flush_online_players_to_db(world).await?;
            Ok(false)
        }
        crate::server_save::ServerSaveTick::FlushShutdown => {
            crate::lua_scope::fire_on_shutdown(world);
            if let Err(e) = world.process_and_persist_houses().await {
                tracing::warn!(error = %e, "house process/save on shutdown save failed");
            }
            flush_online_players_to_db(world).await?;
            Ok(true)
        }
    }
}

fn register_output_sink(
    conn_id: ConnId,
    output_sinks: &mut OutputSinkMap,
    out_registry: &Option<OutRegistry>,
) {
    let Some(reg) = out_registry.as_ref() else {
        return;
    };
    if let Ok(g) = reg.lock()
        && let Some(tx) = g.get(&conn_id)
    {
        output_sinks.insert(conn_id, tx.clone());
    }
}

fn unregister_output_sink(conn_id: ConnId, output_sinks: &mut OutputSinkMap) {
    output_sinks.remove(&conn_id);
}

/// Ensure `output_sinks` has a live `OutboundTx`, mirroring from `OutRegistry` if needed.
fn ensure_output_sink<'a>(
    conn: ConnId,
    output_sinks: &'a mut OutputSinkMap,
    out_registry: &Option<OutRegistry>,
) -> Option<&'a OutboundTx> {
    if !output_sinks.contains_key(&conn)
        && let Some(reg) = out_registry
        && let Ok(g) = reg.lock()
        && let Some(tx) = g.get(&conn)
    {
        output_sinks.insert(conn, tx.clone());
    }
    output_sinks.get(&conn)
}

fn flush_pending_outgoing(
    world: &mut GameWorld,
    output_sinks: &mut OutputSinkMap,
    out_registry: &Option<OutRegistry>,
    pending_output_shed: &mut Vec<ConnId>,
) {
    let flushed = world.flush_output_buffers();
    if out_registry.is_none() && output_sinks.is_empty() {
        trace!(
            batches = flushed.len(),
            "flushed outgoing (no sinks — packets dropped)"
        );
        return;
    }
    for (conn, blobs) in flushed {
        let Some(tx) = ensure_output_sink(conn, output_sinks, out_registry) else {
            warn!(
                conn_id = conn.0,
                packets = blobs.len(),
                "no output sink — re-queuing pending batch"
            );
            world
                .pending_outgoing
                .entry(conn)
                .or_default()
                .extend(blobs);
            continue;
        };
        world.obs.note_output_queued_bytes(tx.queued_bytes());
        match tx.try_send(blobs) {
            Ok(()) => {}
            Err((OutboundSendError::Closed, batch)) => {
                // Writer gone — re-queue then shed so disconnect can close cleanly.
                warn!(
                    conn_id = conn.0,
                    packets = batch.len(),
                    "outbound closed — re-queuing and shedding connection"
                );
                world
                    .pending_outgoing
                    .entry(conn)
                    .or_default()
                    .extend(batch);
                output_sinks.remove(&conn);
                pending_output_shed.push(conn);
            }
            Err((OutboundSendError::Full, batch)) => {
                // Soft backpressure: do not disconnect — dropping a floor-change `0x64`
                // desyncs OTClient. Re-queue for the next flush; shed only on SlowClient.
                warn!(
                    conn_id = conn.0,
                    queued_bytes = tx.queued_bytes(),
                    "output batch channel full — re-queuing (not shedding)"
                );
                world.obs.record_output_full();
                world
                    .pending_outgoing
                    .entry(conn)
                    .or_default()
                    .extend(batch);
            }
            Err((OutboundSendError::SlowClient { queued, batch }, returned)) => {
                warn!(
                    conn_id = conn.0,
                    queued,
                    batch,
                    returned_packets = returned.len(),
                    "output hard byte cap exceeded — shedding slow client"
                );
                world.obs.record_output_slow_shed();
                world
                    .pending_outgoing
                    .entry(conn)
                    .or_default()
                    .extend(returned);
                pending_output_shed.push(conn);
            }
        }
    }
}

/// Push one connection's pending packets onto its outbound writer.
/// On failure, re-queues the batch (caller may still close the connection afterward).
fn flush_conn_outgoing(
    world: &mut GameWorld,
    conn_id: ConnId,
    output_sinks: &mut OutputSinkMap,
    out_registry: &Option<OutRegistry>,
) {
    let Some(blobs) = world.pending_outgoing.remove(&conn_id) else {
        return;
    };
    if blobs.is_empty() {
        return;
    }
    let Some(tx) = ensure_output_sink(conn_id, output_sinks, out_registry) else {
        warn!(
            conn_id = conn_id.0,
            packets = blobs.len(),
            "disconnect flush: no output sink — re-queuing"
        );
        world
            .pending_outgoing
            .entry(conn_id)
            .or_default()
            .extend(blobs);
        return;
    };
    if let Err((err, batch)) = tx.try_send(blobs) {
        warn!(
            conn_id = conn_id.0,
            ?err,
            returned = batch.len(),
            "disconnect flush failed — re-queuing"
        );
        world
            .pending_outgoing
            .entry(conn_id)
            .or_default()
            .extend(batch);
    }
}

/// Drop game-thread + registry `OutboundTx` so the writer task exits and TCP shuts down
/// (TFS `ProtocolGame::disconnect` / 772 `logout` → connection close).
fn close_output_connection(
    conn_id: ConnId,
    output_sinks: &mut OutputSinkMap,
    out_registry: &Option<OutRegistry>,
) {
    output_sinks.remove(&conn_id);
    if let Some(reg) = out_registry
        && let Ok(mut g) = reg.lock()
    {
        g.remove(&conn_id);
    }
}

fn drain_output_shed(
    world: &mut GameWorld,
    pending_login_conns: &mut HashSet<ConnId>,
    output_sinks: &mut OutputSinkMap,
    out_registry: &Option<OutRegistry>,
    pending_output_shed: &mut Vec<ConnId>,
) {
    for conn in pending_output_shed.drain(..) {
        handle_player_disconnect(
            world,
            pending_login_conns,
            conn,
            false,
            false, // dead connection / shed — StopFight=false
            output_sinks,
            out_registry,
        );
    }
}

/// TFS `Player::canDoAction` / `nextAction` — packets that must **not** run while the step lockout
/// is active (`player.cpp`). Walk, turn, ping, and most UI/look/channel packets stay ungated; the
/// default is **gated** for gameplay (use/attack/trade/etc.).  
// C++ reference: per-handler checks in `game.cpp` / `player.cpp`; refine when each opcode is ported.
fn game_packet_requires_timed_action(packet: &GamePacket) -> bool {
    !matches!(
        packet,
        GamePacket::EnterGame
            | GamePacket::Logout
            | GamePacket::Ping
            | GamePacket::PingBack
            | GamePacket::Move(_)
            | GamePacket::AutoWalk { .. }
            | GamePacket::StopAutoWalk
            // C++ `CAttack` (`receiving.cc:1133-1155`) has NO `EarliestAttackTime` gate —
            // `SetAttackDest` + `ToDoAttack` + `ToDoStart` run unconditionally at packet receipt.
            // The attack cooldown is only checked in `CanToDoAttack` (`crcombat.cc:442-511`) when
            // the `TDAttack` executes, not at packet receipt. Gating `Attack`/`Follow` here
            // silently drops target-swap packets while the player is mid-attack cooldown.
            | GamePacket::Attack { .. }
            | GamePacket::Follow { .. }
            | GamePacket::ExtendedOpcode { .. }
            | GamePacket::Turn(_)
            | GamePacket::CancelAttackAndFollow
            | GamePacket::FightModes { .. }
            | GamePacket::LookAt { .. }
            | GamePacket::LookInBattleList { .. }
            | GamePacket::BrowseField { .. }
            | GamePacket::GetObjectInfo
            | GamePacket::Say(_)
            | GamePacket::RequestChannels
            | GamePacket::OpenChannel { .. }
            | GamePacket::CloseChannel { .. }
            | GamePacket::OpenPrivateChannel { .. }
            | GamePacket::CloseNpcChannel
            | GamePacket::CloseContainer { .. }
            | GamePacket::UpArrowContainer { .. }
            | GamePacket::UpdateContainer { .. }
            | GamePacket::SeekInContainer { .. }
            | GamePacket::UseItem(_)
            | GamePacket::UseItemEx(_)
            | GamePacket::UseWithCreature { .. }
            // F8 S6 — `Throw`/`RotateItem` now route through the ToDo engine (Wait{100} +
            // CalculateDelay gate), so the `player_packet_action_ready` receipt-time gate is
            // redundant and would drop packets the ToDo engine would correctly queue.
            | GamePacket::Throw(_)
            | GamePacket::RotateItem { .. }
            | GamePacket::BugReport(_)
            | GamePacket::ThankYou
            | GamePacket::DebugAssert { .. }
            | GamePacket::QuestLog
            | GamePacket::QuestLine { .. }
            | GamePacket::VipAdd { .. }
            | GamePacket::VipRemove { .. }
            | GamePacket::VipEdit { .. }
            | GamePacket::RequestOutfit
            | GamePacket::SetOutfit(_)
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopExit {
    Shutdown,
    ChannelClosed,
}

/// Spawn Tokio DB load; never await I/O on the game thread (GL-1).
#[allow(clippy::too_many_arguments)]
fn begin_player_login_load(
    world: &mut GameWorld,
    cmd_tx: &GameCmdTx,
    pending_login_conns: &mut HashSet<tfs_rust_common::ConnId>,
    login_started: &mut HashMap<ConnId, Instant>,
    conn_id: tfs_rust_common::ConnId,
    name: String,
    operating_system: u16,
    otclient_v8: u16,
    peer_ip: u32,
) {
    if pending_login_conns.len() >= MAX_CONCURRENT_LOGIN_LOADS {
        warn!(
            conn_id = conn_id.0,
            %name,
            in_flight = pending_login_conns.len(),
            cap = MAX_CONCURRENT_LOGIN_LOADS,
            "rejecting login load — concurrent cap reached"
        );
        let _ = cmd_tx.send(GameCommand::PlayerLoadFailed {
            conn_id,
            name,
            reason: format!("too many concurrent login loads (cap {MAX_CONCURRENT_LOGIN_LOADS})"),
        });
        return;
    }
    if !pending_login_conns.insert(conn_id) {
        warn!(
            conn_id = conn_id.0,
            %name,
            "login load already in flight for connection"
        );
        return;
    }
    login_started.insert(conn_id, Instant::now());
    world.obs.note_concurrent_logins(pending_login_conns.len());

    let db = world.db.clone();
    let tx = cmd_tx.clone();
    let load_name = name.clone();
    tokio::spawn(async move {
        match login::load_player_data(&db, &load_name).await {
            Ok(data) => {
                let _ = tx.send(GameCommand::PlayerLoaded {
                    conn_id,
                    name: load_name,
                    operating_system,
                    otclient_v8,
                    peer_ip,
                    data: OwnedPlayerLoad::new(data),
                });
            }
            Err(e) => {
                let _ = tx.send(GameCommand::PlayerLoadFailed {
                    conn_id,
                    name: load_name,
                    reason: e.to_string(),
                });
            }
        }
    });
}

fn conn_still_current(
    world: &GameWorld,
    output_sinks: &OutputSinkMap,
    out_registry: &Option<OutRegistry>,
    conn_id: ConnId,
) -> bool {
    if world.conn_to_creature.contains_key(&conn_id) {
        return false;
    }
    if output_sinks.contains_key(&conn_id) {
        return true;
    }
    match out_registry {
        Some(reg) => reg
            .lock()
            .map(|g| g.contains_key(&conn_id))
            .unwrap_or(false),
        None => true,
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_player_loaded(
    world: &mut GameWorld,
    pending_login_conns: &mut HashSet<ConnId>,
    login_started: &mut HashMap<ConnId, Instant>,
    conn_id: ConnId,
    name: String,
    operating_system: u16,
    otclient_v8: u16,
    peer_ip: u32,
    data: OwnedPlayerLoad,
    output_sinks: &mut OutputSinkMap,
    out_registry: &Option<OutRegistry>,
) {
    if let Some(started) = login_started.remove(&conn_id) {
        world.obs.record_login_load(
            started.elapsed().as_micros() as u64,
            pending_login_conns.len().saturating_sub(1),
        );
    }
    pending_login_conns.remove(&conn_id);
    if !conn_still_current(world, output_sinks, out_registry, conn_id) {
        warn!(
            conn_id = conn_id.0,
            %name,
            "discarding PlayerLoaded — connection no longer current"
        );
        return;
    }
    let loaded = match data.downcast::<LoadedPlayerData>() {
        Ok(d) => d,
        Err(_) => {
            error!(
                conn_id = conn_id.0,
                %name,
                "PlayerLoaded payload was not LoadedPlayerData"
            );
            handle_player_disconnect(
                world,
                pending_login_conns,
                conn_id,
                false,
                true, // no in-game body / login fail — StopFight=true
                output_sinks,
                out_registry,
            );
            return;
        }
    };
    match login::apply_loaded_player(world, loaded, operating_system, otclient_v8, peer_ip) {
        Ok(login::ApplyPlayerOutcome::Spawned(cid)) => {
            world.register_conn_mapping(conn_id, cid);
            crate::login_out::enqueue_initial_login_packets(world, conn_id, cid);
            // 772 `TPlayer` ctor `FinishSendData`s only (`crplayer.cc:197-209`); `SendAll` is
            // `AdvanceGame` (`main.cc:455`). Beat-pending `SendAll` runs *before* dispatch.
        }
        Ok(login::ApplyPlayerOutcome::TakenOver { cid, old_conn }) => {
            // 772: `CharacterID = 0` then connection `Logout` — close old TCP without
            // `StartLogout` on the body (`connections.cc:244-252`).
            if let Some(old) = old_conn {
                pending_login_conns.remove(&old);
                flush_conn_outgoing(world, old, output_sinks, out_registry);
                close_output_connection(old, output_sinks, out_registry);
            }
            world.register_conn_mapping(conn_id, cid);
            crate::login_out::enqueue_initial_login_packets(world, conn_id, cid);
            // New conn waits for beat `SendAll`, same as spawn (`crplayer.cc:765-773`).
        }
        Err(e) => {
            warn!(?e, %name, conn_id = conn_id.0, "player login apply failed");
            handle_player_disconnect(
                world,
                pending_login_conns,
                conn_id,
                false,
                true, // no in-game body / login fail — StopFight=true
                output_sinks,
                out_registry,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_player_load_failed(
    world: &mut GameWorld,
    pending_login_conns: &mut HashSet<tfs_rust_common::ConnId>,
    login_started: &mut HashMap<ConnId, Instant>,
    conn_id: tfs_rust_common::ConnId,
    name: String,
    reason: String,
    output_sinks: &mut OutputSinkMap,
    out_registry: &Option<OutRegistry>,
) {
    if let Some(started) = login_started.remove(&conn_id) {
        world.obs.record_login_load(
            started.elapsed().as_micros() as u64,
            pending_login_conns.len().saturating_sub(1),
        );
    }
    pending_login_conns.remove(&conn_id);
    warn!(
        conn_id = conn_id.0,
        %name,
        %reason,
        "player login load failed"
    );
    // GL-1: async load failure must close the game TCP (TFS `disconnectClient`), not leave
    // a half-open session with no character mapping.
    handle_player_disconnect(
        world,
        pending_login_conns,
        conn_id,
        false,
        true, // no in-game body / login fail — StopFight=true
        output_sinks,
        out_registry,
    );
}

fn handle_player_disconnect(
    world: &mut GameWorld,
    pending_login_conns: &mut HashSet<ConnId>,
    conn_id: ConnId,
    display_effect: bool,
    stop_fight: bool,
    output_sinks: &mut OutputSinkMap,
    out_registry: &Option<OutRegistry>,
) {
    pending_login_conns.remove(&conn_id);
    world.dead_connections.remove(&conn_id);
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
                // Fire-and-forget on the multi-thread pool. Do not `block_in_place` —
                // the game loop is on a `LocalSet`, which panics on blocking calls.
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
        // Clear connection first (772 `ClearConnection` before `StartLogout`).
        world.unregister_conn_mapping(conn_id);
        world.known_creatures_by_conn.remove(&conn_id);
        world.creature_fully_sent_by_conn.remove(&conn_id);
        // 772 `StartLogout(false, StopFight)` — body may stay on map until LogoutPossible.
        world.creature_begin_logout(cid, false, stop_fight);
        if world.player_logout_possible(cid) == crate::game_world_lifecycle::LogoutPossible::Ok {
            world.remove_creature(cid);
        }
    } else {
        world.unregister_conn_mapping(conn_id);
        world.known_creatures_by_conn.remove(&conn_id);
        world.creature_fully_sent_by_conn.remove(&conn_id);
    }
    // TFS `ProtocolGame::logout`: flush then `disconnect()` so the client leaves the game
    // cleanly. Dropping only the game-thread sink left `OutboundTx` alive in the registry —
    // the writer never exited, TCP stayed open, OTClient desynced until idle timeout.
    flush_conn_outgoing(world, conn_id, output_sinks, out_registry);
    close_output_connection(conn_id, output_sinks, out_registry);
    trace!(conn_id = conn_id.0, stop_fight, "player disconnected");
}

fn handle_game_packet(
    world: &mut GameWorld,
    conn_id: tfs_rust_common::ConnId,
    packet: GamePacket,
    game_rx: &mut Receiver<GameCommand>,
    pending: &mut PendingQueue,
) {
    let now = Instant::now();
    // 772 `CommandAllowed` for CONNECTION_DEAD / CONNECTION_LOGOUT (`receiving.cc:17-21`).
    if world.dead_connections.contains(&conn_id) {
        match &packet {
            GamePacket::Logout
            | GamePacket::Ping
            | GamePacket::PingBack
            | GamePacket::BugReport(_) => {}
            _ => {
                trace!(
                    conn_id = conn_id.0,
                    ?packet,
                    "game packet ignored — connection dead (post-death)"
                );
                return;
            }
        }
    }
    if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
        // Phase 4: 1098 no longer resets rounds differently — both eras use the 772
        // `ProcessConnections` round tracking.
        world.player_reset_connection_rounds(
            cid,
            crate::connections::packet_counts_as_action(&packet),
        );
        if game_packet_requires_timed_action(&packet)
            && !world.player_packet_action_ready(cid, &packet)
        {
            trace!(
                conn_id = conn_id.0,
                ?packet,
                "game packet ignored — Earliest*Time / nextAction lockout"
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
            world
                .protocol_hooks
                .extended_opcode(conn_id, opcode, buffer);
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
                match game_rx.try_recv() {
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
                                pending_push(
                                    pending,
                                    GameCommand::Game {
                                        conn_id: next_conn,
                                        packet: other,
                                    },
                                );
                            }
                        },
                        other => {
                            world.flush_deferred_turn_broadcast(cid);
                            pending_push(pending, other);
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
        GamePacket::Attack { creature_id } => {
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                // 772 `CAttack(Follow=false)` — `receiving.cc:1133-1155`.
                world.player_set_attack_dest(conn_id, cid, creature_id, false);
            }
        }
        GamePacket::Follow { creature_id } => {
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                // 772 `CAttack(Follow=true)` — `receiving.cc:1133-1155`.
                world.player_set_attack_dest(conn_id, cid, creature_id, true);
            }
        }
        GamePacket::CancelAttackAndFollow => {
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                // 772 `CCancelAttack` — `SetAttackDest(0)` + `ToDoStop` (`receiving.cc`,
                // `cract.cc:1002-1008`). 1098 defers to Phase 2.
                world.player_cancel_attack_and_follow(conn_id, cid);
            }
        }
        GamePacket::FightModes {
            raw_fight_mode,
            raw_chase_mode,
            raw_secure_mode,
            ..
        } => {
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                // 772 `0xA7` `FIGHT_MODES` — `SetAttackMode` + `SetChaseMode` + `SetSecureMode`
                // (`crcombat.cc:325-355`). PC-4 wires all three setters; the attack-mode change
                // applies `DelayAttack(2000)` (`crcombat.cc:334`).
                world.player_set_fight_modes(cid, raw_fight_mode, raw_chase_mode, raw_secure_mode);
            }
        }
        GamePacket::Throw(payload) => {
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                // F8 S6 — 772 `CMoveObject` (`receiving.cc:233`): `ToDoMove(…)` + `ToDoStart`
                // (the *handler* adds no `ToDoWait`; the `ToDoMove` builder itself always
                // prepends `Wait{100}` — `cract.cc:1155,1165`, D1). Route through the
                // unified ToDo engine instead of the reactive executor. `TDMove` delay = 0
                // (`cract.cc:946-948` default), clamped to 1 for forward progress
                // (`cract.cc:1016`). `ToDoAdd` preamble clears any pending armed action +
                // snapback (`cract.cc:993-1000`). Phase 4: 1098 reactive `player_move_thing`
                // path deleted — both eras use the ToDo `TDMove` builder.
                let obj = ActionObjectRef {
                    pos: payload.from_pos,
                    stack_pos: payload.from_stack_pos,
                    sprite_id: payload.sprite_id,
                };
                world.player_todo_clear_with_snapback(conn_id, cid);
                if let Err(rv) = world.enqueue_player_move(cid, obj, payload.to_pos, payload.count)
                {
                    world.send_cancel_message(conn_id, rv);
                } else {
                    world.todo_start_from_action(cid, 1);
                }
            }
        }
        GamePacket::UseItem(payload) => {
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                // F8 S6 — 772 `CUseObject` (`receiving.cc:384`): `ToDoWait(100)` + `ToDoUse(1,…)`
                // + `ToDoStart`. Route through the unified ToDo engine; the `Wait{100}` entry
                // drives the 100ms floor and the execute arm applies the multiuse gate (S3,
                // single-object use is ungated). `ToDoAdd` preamble clears any pending armed
                // action + snapback (`cract.cc:993-1000`). Phase 4: 1098 reactive
                // `player_use_item` path deleted — both eras use the ToDo `TDUse` builder.
                let obj = ActionObjectRef {
                    pos: payload.pos,
                    stack_pos: payload.stack_pos,
                    sprite_id: payload.sprite_id,
                };
                world.player_todo_clear_with_snapback(conn_id, cid);
                if let Err(rv) = world.enqueue_player_use(cid, obj, None, payload.index) {
                    world.send_cancel_message(conn_id, rv);
                } else {
                    world.todo_start_from_action(cid, 1);
                }
            }
        }
        GamePacket::UseItemEx(payload) => {
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                // F8 S6 — 772 `CUseTwoObjects` (`receiving.cc:430`): `ToDoWait(100)` +
                // `ToDoUse(2,…)` + `ToDoStart`. Same ToDo routing as `UseItem`; the execute
                // arm gates two-object use on `EarliestMultiuseTime` (S3). `open_index` = 0
                // (`UseItemEx` has no index byte). Phase 4: 1098 reactive path deleted.
                let obj1 = ActionObjectRef {
                    pos: payload.from_pos,
                    stack_pos: payload.from_stack_pos,
                    sprite_id: payload.from_sprite_id,
                };
                let obj2 = ActionObjectRef {
                    pos: payload.to_pos,
                    stack_pos: payload.to_stack_pos,
                    sprite_id: payload.to_sprite_id,
                };
                world.player_todo_clear_with_snapback(conn_id, cid);
                if let Err(rv) = world.enqueue_player_use(cid, obj1, Some(obj2), 0) {
                    world.send_cancel_message(conn_id, rv);
                } else {
                    world.todo_start_from_action(cid, 1);
                }
            }
        }
        // TFS `ProtocolGame::parseUseWithCreature` → `Game::playerUseWithCreature`
        // (`protocolgame.cpp:930`, `game.cpp:2260`). needTarget runes (SD, HMM, …) send
        // this opcode with a wire creature id; was previously dropped in the catch-all.
        GamePacket::UseWithCreature {
            from_pos,
            sprite_id,
            from_stack_pos,
            creature_id: target_wire,
        } => {
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                let Some(target_cid) = world.creature_by_wire_id(target_wire) else {
                    world.send_cancel_message(conn_id, ReturnValue::NotPossible);
                    return;
                };
                let Some(target_pos) = world.creatures.get(target_cid).map(|k| k.position()) else {
                    world.send_cancel_message(conn_id, ReturnValue::NotPossible);
                    return;
                };
                // TFS `Game::playerUseWithCreature` — silent drop outside `areInRange<7,5,0>`
                // (`game.cpp:2272-2274`). Far-use runes fire from standing tile within this box.
                let Some(player_pos) = world.creatures.get(cid).map(|k| k.position()) else {
                    return;
                };
                if player_pos.z != target_pos.z
                    || (player_pos.x as i32 - target_pos.x as i32).unsigned_abs() > 7
                    || (player_pos.y as i32 - target_pos.y as i32).unsigned_abs() > 5
                {
                    return;
                }
                let obj1 = ActionObjectRef {
                    pos: from_pos,
                    stack_pos: from_stack_pos,
                    sprite_id,
                };
                // Synthetic obj2 at the creature's tile — `validate_use_ex_target_ref`
                // accepts creatures; `player_cast_rune` resolves the creature on the tile.
                let obj2 = ActionObjectRef {
                    pos: target_pos,
                    stack_pos: 0,
                    sprite_id: 0,
                };
                world.player_todo_clear_with_snapback(conn_id, cid);
                if let Err(rv) = world.enqueue_player_use(cid, obj1, Some(obj2), 0) {
                    world.send_cancel_message(conn_id, rv);
                } else {
                    world.todo_start_from_action(cid, 1);
                }
            }
        }
        // F8 S6 — 772 `CTurnObject` (`receiving.cc:549`): `ToDoWait(100)` + `ToDoTurn(…)` +
        // `ToDoStart`. Rotates a rotatable *item* (wall torch/rope) — **not** `CRotate` (player
        // facing, `receiving.cc:213`, already immediate via `GamePacket::Turn`). This arm is
        // new in S6: `RotateItem` previously fell through to the catch-all `_ => trace!`
        // (§0.1 F2). The executor (`player_rotate_item`) was built in S4. Phase 4: 1098
        // reactive path deleted — both eras use the ToDo `TDTurn` builder.
        GamePacket::RotateItem {
            pos,
            sprite_id,
            stack_pos,
        } => {
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                let obj = ActionObjectRef {
                    pos,
                    stack_pos,
                    sprite_id,
                };
                world.player_todo_clear_with_snapback(conn_id, cid);
                if let Err(rv) = world.enqueue_player_turn(cid, obj) {
                    world.send_cancel_message(conn_id, rv);
                } else {
                    // `TDTurn` delay = 0 (`cract.cc:946-948` default); the `Wait{100}`
                    // prefix drives the 100ms floor. Clamp to 1 for forward progress.
                    world.todo_start_from_action(cid, 1);
                }
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
        GamePacket::SeekInContainer {
            cid: client_cid,
            index,
        } => {
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
            // TFS / 772 `ProtocolGame::logout` — validate then disconnect (close TCP).
            // Mid-async-login (no `conn_to_creature` yet) still closes the session.
            // Post-death: mapping was cleared but `dead_connections` keeps the session
            // until OK (`CL_CMD_LOGOUT`) — 772 `CONNECTION_DEAD` (`receiving.cc:17-21`).
            if world.dead_connections.contains(&conn_id) {
                pending_push(
                    pending,
                    GameCommand::PlayerDisconnect {
                        conn_id,
                        display_effect: false,
                    },
                );
            } else if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                if world.player_logout_allowed(conn_id, cid, false) {
                    pending_push(
                        pending,
                        GameCommand::PlayerDisconnect {
                            conn_id,
                            display_effect: true,
                        },
                    );
                }
            } else {
                pending_push(
                    pending,
                    GameCommand::PlayerDisconnect {
                        conn_id,
                        display_effect: false,
                    },
                );
            }
        }
        GamePacket::Say(payload) => {
            // CH-1: `Game::playerSay` dispatch — `gameserver/src/game.cpp:3208-3281`.
            // Only `TALKTYPE_SAY` is wired (viewport broadcast); other arms stubbed
            // pending CH-2/CH-3/CH-4/CH-5. Text length is already capped at 255 bytes
            // by the wire parser (`game_parse.rs::parse_say`, mirroring
            // `protocolgame.cpp:945-947`).
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                world.player_say(
                    conn_id,
                    cid,
                    payload.speak_class,
                    payload.channel_id,
                    &payload.receiver,
                    &payload.text,
                );
            }
        }
        GamePacket::RequestChannels => {
            // CH-4: `Game::playerRequestChannels` — `game.cpp:3490-3502`.
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                world.player_request_channels(conn_id, cid);
            }
        }
        GamePacket::RequestOutfit => {
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                world.player_request_outfit(conn_id, cid);
            }
        }
        GamePacket::SetOutfit(payload) => {
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                world.player_change_outfit(conn_id, cid, payload);
            }
        }
        GamePacket::OpenChannel { channel_id } => {
            // CH-4: `Game::playerOpenChannel` — `game.cpp:3490-3502`.
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                world.player_open_channel(conn_id, cid, channel_id);
            }
        }
        GamePacket::CloseChannel { channel_id } => {
            // CH-4: `Game::playerCloseChannel` — `game.cpp:3490-3502`.
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                world.player_close_channel(cid, channel_id);
            }
        }
        GamePacket::OpenPrivateChannel { receiver } => {
            // CH-4: `Game::playerOpenPrivateChannel` — `game.cpp:3490-3502`.
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                world.player_open_private_channel(conn_id, cid, &receiver);
            }
        }
        GamePacket::CreatePrivateChannel => {
            // CH-4: `Game::playerCreatePrivateChannel` — `game.cpp:2023`.
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                world.player_create_private_channel(conn_id, cid);
            }
        }
        GamePacket::ChannelInvite { name } => {
            // CH-4: `Game::playerChannelInvite` — `chat.cpp:29-52`.
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                world.player_channel_invite(cid, &name);
            }
        }
        GamePacket::ChannelExclude { name } => {
            // CH-4: `Game::playerChannelExclude` — `chat.cpp:29-52`.
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                world.player_channel_exclude(cid, &name);
            }
        }
        GamePacket::TextWindow {
            window_text_id,
            new_text,
        } => {
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied()
                && let Err(rv) = world.player_write_item(cid, window_text_id, new_text)
            {
                world.send_cancel_message(conn_id, rv);
            }
        }
        GamePacket::HouseWindow {
            door_id,
            house_id: window_text_id,
            text,
        } => {
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                let _ = world.player_update_house_window(cid, door_id, window_text_id, text);
            }
        }
        GamePacket::BugReport(payload) => {
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                let pos = payload.position.unwrap_or_else(|| {
                    world
                        .creatures
                        .get(cid)
                        .map(|k| k.position())
                        .unwrap_or_default()
                });
                crate::lua_scope::fire_on_player_report_bug(
                    world,
                    cid,
                    &payload.message,
                    pos,
                    payload.category,
                );
            }
        }
        _ => trace!(
            conn_id = conn_id.0,
            ?packet,
            "game packet — simulation Phase 9+"
        ),
    }
    // Phase 4: 1098 `process_walk_deadlines` call deleted — both eras use the ToDo queue.
}

#[allow(clippy::too_many_arguments)]
fn dispatch_command(
    world: &mut GameWorld,
    cmd: Option<GameCommand>,
    game_rx: &mut Receiver<GameCommand>,
    cmd_tx: &GameCmdTx,
    pending: &mut PendingQueue,
    pending_login_conns: &mut HashSet<ConnId>,
    login_started: &mut HashMap<ConnId, Instant>,
    output_sinks: &mut OutputSinkMap,
    out_registry: &Option<OutRegistry>,
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
            peer_ip,
        } => {
            begin_player_login_load(
                world,
                cmd_tx,
                pending_login_conns,
                login_started,
                conn_id,
                name,
                operating_system,
                otclient_v8,
                peer_ip,
            );
            ControlFlow::Continue(())
        }
        GameCommand::PlayerLoaded {
            conn_id,
            name,
            operating_system,
            otclient_v8,
            peer_ip,
            data,
        } => {
            handle_player_loaded(
                world,
                pending_login_conns,
                login_started,
                conn_id,
                name,
                operating_system,
                otclient_v8,
                peer_ip,
                data,
                output_sinks,
                out_registry,
            );
            ControlFlow::Continue(())
        }
        GameCommand::PlayerLoadFailed {
            conn_id,
            name,
            reason,
        } => {
            handle_player_load_failed(
                world,
                pending_login_conns,
                login_started,
                conn_id,
                name,
                reason,
                output_sinks,
                out_registry,
            );
            ControlFlow::Continue(())
        }
        GameCommand::RegisterOutputSink { conn_id } => {
            register_output_sink(conn_id, output_sinks, out_registry);
            ControlFlow::Continue(())
        }
        GameCommand::UnregisterOutputSink { conn_id } => {
            unregister_output_sink(conn_id, output_sinks);
            ControlFlow::Continue(())
        }
        GameCommand::LuaCallback { event_id } => {
            // C++ reference: `LuaEnvironment::executeTimerEvent` (`luascript.cpp:18238`).
            // Dispatch the `addEvent` callback with Lua mutation scope + read context,
            // then clean up the stale abort handle in the scheduler.
            crate::lua_scope::fire_on_timer_event(world, event_id);
            if let Some(scheduler) = &world.scheduler {
                scheduler.forget(event_id);
            }
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
            handle_player_disconnect(
                world,
                pending_login_conns,
                conn_id,
                display_effect,
                true, // CQuitGame / intentional — StopFight=true
                output_sinks,
                out_registry,
            );
            ControlFlow::Continue(())
        }
        GameCommand::Game { conn_id, packet } => {
            handle_game_packet(world, conn_id, packet, game_rx, pending);
            ControlFlow::Continue(())
        }
        GameCommand::HouseNamesResolved {
            house_id,
            list_id,
            text,
            resolved,
        } => {
            world.apply_house_names_resolved(house_id, list_id, text, resolved);
            ControlFlow::Continue(())
        }
    }
}

async fn recv_next_command(
    game_rx: &mut Receiver<GameCommand>,
    ctrl_rx: &mut UnboundedReceiver<GameCommand>,
    pending: &mut PendingQueue,
) -> Option<GameCommand> {
    if let Some((_at, c)) = pending.pop_front() {
        return Some(c);
    }
    tokio::select! {
        biased;
        c = ctrl_rx.recv() => c,
        c = game_rx.recv() => c,
    }
}

/// Prefer control-lane, then pending, then one game-lane try_recv.
fn try_recv_next_command(
    game_rx: &mut Receiver<GameCommand>,
    ctrl_rx: &mut UnboundedReceiver<GameCommand>,
    pending: &mut PendingQueue,
) -> Option<GameCommand> {
    if let Some((_at, c)) = pending.pop_front() {
        return Some(c);
    }
    if let Ok(c) = ctrl_rx.try_recv() {
        return Some(c);
    }
    game_rx.try_recv().ok()
}

/// Count how many beat ticks are already ready without awaiting (may be zero).
fn drain_ready_beats(interval: &mut tokio::time::Interval) -> u64 {
    use std::future::Future;
    use std::task::Poll;

    let mut beats = 0u64;
    loop {
        let next = interval.tick();
        tokio::pin!(next);
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);
        if matches!(next.as_mut().poll(&mut cx), Poll::Ready(_)) {
            beats += 1;
        } else {
            break;
        }
    }
    beats
}

/// TFS 1098 reactive loop — Dispatcher + Scheduler walk timers.
///
/// Count additional burst ticks already pending on `interval` after one `tick().await` fired.
fn drain_burst_beats(interval: &mut tokio::time::Interval) -> u64 {
    use std::future::Future;
    use std::task::Poll;

    let mut beats = 1u64;
    loop {
        let next = interval.tick();
        tokio::pin!(next);
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);
        if matches!(next.as_mut().poll(&mut cx), Poll::Ready(_)) {
            beats += 1;
        } else {
            break;
        }
    }
    beats
}

fn obs_note_ingress(
    world: &mut GameWorld,
    game_rx: &Receiver<GameCommand>,
    pending: &PendingQueue,
) {
    let depth = game_rx.len().saturating_add(pending.len());
    world.obs.note_command_depth(depth);
    if let Some((at, _)) = pending.front() {
        world
            .obs
            .record_command_age_ms(at.elapsed().as_millis() as u64);
    }
}

#[allow(clippy::too_many_arguments)]
fn obs_advance_beats(
    world: &mut GameWorld,
    next_beat_deadline: &mut Instant,
    beat_ms: u64,
    coalesced: u64,
    pending_login_conns: &mut HashSet<ConnId>,
    output_sinks: &mut OutputSinkMap,
    out_registry: &Option<OutRegistry>,
    pending_output_shed: &mut Vec<ConnId>,
) {
    let now = Instant::now();
    let lateness_ms = now
        .saturating_duration_since(*next_beat_deadline)
        .as_millis() as u64;
    let wall_start = Instant::now();
    world.advance_beat(beat_ms.saturating_mul(coalesced.max(1)));
    let wall_ms = wall_start.elapsed().as_millis() as u64;
    world
        .obs
        .record_beat(coalesced.max(1), lateness_ms, wall_ms);
    *next_beat_deadline = next_beat_deadline
        .checked_add(Duration::from_millis(
            beat_ms.saturating_mul(coalesced.max(1)),
        ))
        .unwrap_or(now);
    // If we fell far behind, clamp so the next lateness sample is relative to "now + beat".
    if *next_beat_deadline < now {
        *next_beat_deadline = now + Duration::from_millis(beat_ms);
    }
    while let Some((conn_id, stop_fight)) = world.pending_idle_kick.pop() {
        handle_player_disconnect(
            world,
            pending_login_conns,
            conn_id,
            false,
            stop_fight,
            output_sinks,
            out_registry,
        );
    }
    flush_pending_outgoing(world, output_sinks, out_registry, pending_output_shed);
    drain_output_shed(
        world,
        pending_login_conns,
        output_sinks,
        out_registry,
        pending_output_shed,
    );
}

/// `AdvanceGame` + `SendAll` when a beat is already due (`main.cc:493-497`).
///
/// Call this *before* dispatching the command that just woke the loop. Tokio
/// `Interval` is Ready the instant the deadline passes, unlike POSIX `SIGALRM`
/// which is usually still 0 during `ReceiveData`. SendAll-after-dispatch made
/// `0xA3` ride the same wakeup as the NPC-attack click, so the red square died
/// instantly instead of lasting ~Beat ms.
#[allow(clippy::too_many_arguments)]
fn send_all_if_beat_pending(
    world: &mut GameWorld,
    beat_timer: &mut tokio::time::Interval,
    next_beat_deadline: &mut Instant,
    beat_ms: u64,
    pending_login_conns: &mut HashSet<ConnId>,
    output_sinks: &mut OutputSinkMap,
    out_registry: &Option<OutRegistry>,
    pending_output_shed: &mut Vec<ConnId>,
) {
    let ready = drain_ready_beats(beat_timer);
    if ready > 0 {
        obs_advance_beats(
            world,
            next_beat_deadline,
            beat_ms,
            ready,
            pending_login_conns,
            output_sinks,
            out_registry,
            pending_output_shed,
        );
    }
}

/// Build the beat timer so the first tick waits one full beat (772 `LaunchGame` waits for
/// the first alarm — `main.cc:484–497`). Tokio's `interval` fires immediately; `interval_at`
/// matches the reference.
fn new_beat_timer(beat_ms: u64) -> (tokio::time::Interval, Instant) {
    let period = Duration::from_millis(beat_ms.max(1));
    let first_deadline = Instant::now() + period;
    let mut beat_timer = interval_at(tokio::time::Instant::from_std(first_deadline), period);
    beat_timer.set_missed_tick_behavior(MissedTickBehavior::Burst);
    (beat_timer, first_deadline)
}

/// Unified beat-driven game loop — `LaunchGame` + `AdvanceGame` + `SendAll`.
///
/// Both eras run on this single engine. Beat size, think cadence, condition/skill tick
/// interval, and flush policy are read from `MechanicsProfile` — no era fork here.
// C++ ref: `tibia-game-master/src/main.cc` `LaunchGame` (477-492) + `AdvanceGame` (312-449);
// 1098 observable behavior per `src/game.cpp` `Game::gameLoop` / `checkCreatures`.
pub async fn run_game_loop(
    mut world: GameWorld,
    mut game_rx: Receiver<GameCommand>,
    mut ctrl_rx: UnboundedReceiver<GameCommand>,
    cmd_tx: GameCmdTx,
    out_registry: Option<OutRegistry>,
) -> anyhow::Result<()> {
    let beat_ms = u64::from(world.mechanics.profile.beat_ms.max(1));
    let (mut beat_timer, mut next_beat_deadline) = new_beat_timer(beat_ms);
    let mut pending: PendingQueue = VecDeque::new();
    let mut pending_login_conns: HashSet<ConnId> = HashSet::new();
    let mut login_started: HashMap<ConnId, Instant> = HashMap::new();
    let mut output_sinks: OutputSinkMap = HashMap::new();
    let mut pending_output_shed: Vec<ConnId> = Vec::new();

    loop {
        tokio::select! {
            biased;

            cmd = recv_next_command(&mut game_rx, &mut ctrl_rx, &mut pending) => {
                // Flush packets queued *before* this command. SendAll after dispatch
                // raced Tokio's due Interval and dropped the red square on the click
                // (`main.cc:488-497`; POSIX alarm is usually 0 during ReceiveData).
                send_all_if_beat_pending(
                    &mut world,
                    &mut beat_timer,
                    &mut next_beat_deadline,
                    beat_ms,
                    &mut pending_login_conns,
                    &mut output_sinks,
                    &out_registry,
                    &mut pending_output_shed,
                );
                world.tick_server_save(chrono::Local::now().timestamp());
                if handle_pending_save_tick(&mut world).await? {
                    break;
                }
                obs_note_ingress(&mut world, &game_rx, &pending);
                match dispatch_command(
                    &mut world,
                    cmd,
                    &mut game_rx,
                    &cmd_tx,
                    &mut pending,
                    &mut pending_login_conns,
                    &mut login_started,
                    &mut output_sinks,
                    &out_registry,
                ) {
                    ControlFlow::Break(LoopExit::Shutdown) => {
                        crate::lua_scope::fire_on_shutdown(&mut world);
                        if let Err(e) = world.process_and_persist_houses().await {
                            tracing::warn!(error = %e, "house save on SIGINT failed");
                        }
                        flush_online_players_to_db(&world).await?;
                        break;
                    }
                    ControlFlow::Break(LoopExit::ChannelClosed) => break,
                    ControlFlow::Continue(()) => {
                        // GL-2: bounded slice — do not drain the game lane to empty.
                        let mut processed = 1usize;
                        while processed < MAX_GAME_COMMANDS_PER_TURN {
                            let Some(more) =
                                try_recv_next_command(&mut game_rx, &mut ctrl_rx, &mut pending)
                            else {
                                break;
                            };
                            match dispatch_command(
                                &mut world,
                                Some(more),
                                &mut game_rx,
                                &cmd_tx,
                                &mut pending,
                                &mut pending_login_conns,
                                &mut login_started,
                                &mut output_sinks,
                                &out_registry,
                            ) {
                                ControlFlow::Break(LoopExit::Shutdown) => {
                                    crate::lua_scope::fire_on_shutdown(&mut world);
                                    if let Err(e) = world.process_and_persist_houses().await {
                                        tracing::warn!(error = %e, "house save on SIGINT failed");
                                    }
                                    flush_online_players_to_db(&world).await?;
                                    return Ok(());
                                }
                                ControlFlow::Break(LoopExit::ChannelClosed) => return Ok(()),
                                ControlFlow::Continue(()) => {
                                    processed += 1;
                                }
                            }
                        }
                        world.obs_record_commands(processed);
                        world.obs_maybe_emit();
                    }
                }
            }
            _ = beat_timer.tick() => {
                let mut beats = drain_burst_beats(&mut beat_timer);
                if beats == 0 {
                    beats = 1;
                }
                obs_advance_beats(
                    &mut world,
                    &mut next_beat_deadline,
                    beat_ms,
                    beats,
                    &mut pending_login_conns,
                    &mut output_sinks,
                    &out_registry,
                    &mut pending_output_shed,
                );
                world.tick_server_save(chrono::Local::now().timestamp());
                if handle_pending_save_tick(&mut world).await? {
                    break;
                }
                world.obs_maybe_emit();
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

/// House items / info are saved from the game thread (`process_and_persist_houses`)
/// before the loop exits. This hook is reserved for pool teardown.
pub async fn graceful_shutdown(_db: &tfs_rust_db::DbPool) -> anyhow::Result<()> {
    let _ = _db;
    Ok(())
}

#[cfg(test)]
mod timed_action_gate_tests {
    use tfs_rust_common::enums::Direction;
    use tfs_rust_common::game_packet::GamePacket;

    use super::game_packet_requires_timed_action;

    #[test]
    fn walk_ping_and_extended_are_never_gated() {
        assert!(!game_packet_requires_timed_action(&GamePacket::Move(
            Direction::North
        )));
        assert!(!game_packet_requires_timed_action(&GamePacket::AutoWalk {
            path: Vec::new()
        }));
        assert!(!game_packet_requires_timed_action(
            &GamePacket::StopAutoWalk
        ));
        assert!(!game_packet_requires_timed_action(&GamePacket::Ping));
        assert!(!game_packet_requires_timed_action(
            &GamePacket::ExtendedOpcode {
                opcode: 1,
                buffer: String::new(),
            }
        ));
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
    fn attack_is_not_gated() {
        // C++ `CAttack` (`receiving.cc:1133-1155`) has no `EarliestAttackTime` check —
        // `SetAttackDest` runs unconditionally. The attack cooldown lives in `CanToDoAttack`
        // (`crcombat.cc:442-511`) at `TDAttack` execute time, not at packet receipt.
        assert!(!game_packet_requires_timed_action(&GamePacket::Attack {
            creature_id: 1
        }));
        assert!(!game_packet_requires_timed_action(&GamePacket::Follow {
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

    /// F8 S6 — `Throw`/`RotateItem` now route through the ToDo engine (Wait{100} +
    /// CalculateDelay gate), so the receipt-time `player_packet_action_ready` gate is
    /// redundant and must not drop them. Mirrors `use_item_defers_to_handler_not_game_loop_gate`.
    #[test]
    fn throw_and_rotate_item_defer_to_handler_not_game_loop_gate() {
        assert!(!game_packet_requires_timed_action(&GamePacket::Throw(
            tfs_rust_common::game_packet::ThrowPayload {
                from_pos: tfs_rust_common::Position::new(0, 0, 7),
                sprite_id: 100,
                from_stack_pos: 0,
                to_pos: tfs_rust_common::Position::new(1, 0, 7),
                count: 1,
            }
        )));
        assert!(!game_packet_requires_timed_action(
            &GamePacket::RotateItem {
                pos: tfs_rust_common::Position::new(0, 0, 7),
                sprite_id: 100,
                stack_pos: 0,
            }
        ));
    }

    #[test]
    fn step_speed_model_follows_profile() {
        use crate::formulas::StepSpeedModel;
        let world = crate::test_world::support::minimal_world();
        // Phase 6: `beat_driven_loop` field is removed; `step_speed` is the profile knob.
        // `minimal_world()` uses V1098 → TfsLog.
        assert_eq!(world.mechanics.profile.step_speed, StepSpeedModel::TfsLog);
    }
}

/// F8 S6 — handler routing tests.
///
/// Verifies `handle_game_packet` routes `UseItem`/`UseItemEx`/`Throw`/`RotateItem` through
/// the ToDo builders (`enqueue_player_use`/`enqueue_player_move`/`enqueue_player_turn`) +
/// `todo_start_from_action` on the unified beat engine, instead of the reactive executors.
/// C++ ref: `receiving.cc:384/430/233/549` (`CUseObject`/`CUseTwoObjects`/`CMoveObject`/
/// `CTurnObject`) → `ToDo*` builder + `ToDoStart` (`cract.cc:955-1024`).
#[cfg(test)]
mod f8_s6_handler_routing_tests {
    use std::collections::{HashMap, VecDeque};

    use tokio::sync::mpsc;

    use tfs_rust_common::{ConnId, GameCommand, GamePacket, Position};

    use crate::creature::{CreatureKind, Player};
    use crate::creature_todo::{ActionObjectRef, CreatureAction};
    use crate::item::Item;
    use crate::test_world::support::{
        TEST_SYNTHETIC_GROUND_WP, beat_driven_test_world, ensure_walkable_tile, test_player,
    };

    use super::handle_game_packet;

    /// Place a bag (container, client_id=0 in the test items_db) on a tile and return its
    /// `ActionObjectRef`. Mirrors `creature_todo` tests' `place_bag_on_tile`.
    fn place_bag_on_tile(
        world: &mut crate::game_world::GameWorld,
        pos: Position,
    ) -> ActionObjectRef {
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let item_id = world.items.insert(Item::new_single(1987));
        world
            .map
            .get_tile_mut(pos)
            .expect("tile just inserted")
            .add_item(item_id);
        ActionObjectRef {
            pos,
            stack_pos: 0,
            sprite_id: 0,
        }
    }

    /// Place a gold (pickupable + moveable) item on a tile for the Move/Throw test.
    fn place_gold_on_tile(
        world: &mut crate::game_world::GameWorld,
        pos: Position,
    ) -> ActionObjectRef {
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let item_id = world.items.insert(Item::new_single(2148));
        world
            .map
            .get_tile_mut(pos)
            .expect("tile just inserted")
            .add_item(item_id);
        ActionObjectRef {
            pos,
            stack_pos: 0,
            sprite_id: 0,
        }
    }

    fn insert_player(
        world: &mut crate::game_world::GameWorld,
        pos: Position,
    ) -> (ConnId, crate::ids::CreatureId) {
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let cid = world
            .creatures
            .insert(CreatureKind::Player(test_player("S6Hero", pos)));
        world.map.register_creature_at(pos, cid);
        let conn_id = ConnId(1);
        world.register_conn_mapping(conn_id, cid);
        (conn_id, cid)
    }

    /// Drive one packet through `handle_game_packet` with empty cmd/pending queues.
    fn dispatch(world: &mut crate::game_world::GameWorld, conn_id: ConnId, packet: GamePacket) {
        let (_tx, mut game_rx, _ctrl_rx) = tfs_rust_net::open_game_command_channels();
        let mut pending = VecDeque::new();
        handle_game_packet(world, conn_id, packet, &mut game_rx, &mut pending);
        // None of the rerouted opcodes push to pending (only Logout does).
        assert!(
            pending.is_empty(),
            "Use/Throw/RotateItem must not push commands"
        );
        let _ = _tx;
    }

    /// `UseItem` (single-object) routes to `[Wait{100}, Use{obj2:None}]` + arms a wakeup.
    #[test]
    fn use_item_routes_through_todo_builder_when_beat_driven() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let (conn_id, cid) = insert_player(&mut world, player_pos);
        let obj = place_bag_on_tile(&mut world, item_pos);

        dispatch(
            &mut world,
            conn_id,
            GamePacket::UseItem(tfs_rust_common::game_packet::UseItemPayload {
                pos: obj.pos,
                sprite_id: obj.sprite_id,
                stack_pos: obj.stack_pos,
                index: 4,
            }),
        );

        let base = world.creatures.get(cid).unwrap().base();
        assert_eq!(base.todo.queue.len(), 2, "Use single → [Wait{{100}}, Use]");
        assert!(matches!(
            base.todo.queue[0],
            CreatureAction::Wait { deadline_ms: 100 }
        ));
        match base.todo.queue[1] {
            CreatureAction::Use {
                obj2, open_index, ..
            } => {
                assert!(obj2.is_none(), "single-object use has no obj2");
                assert_eq!(open_index, 4, "open_index carries UseItemPayload.index");
            }
            ref other => panic!("expected Use, got {other:?}"),
        }
        assert!(base.next_wakeup.is_some(), "ToDoStart armed a wakeup");
    }

    /// `UseItemEx` (two-object) routes to `[Wait{100}, Use{obj2:Some}]`.
    #[test]
    fn use_item_ex_routes_through_todo_builder_with_obj2() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let item_pos2 = Position::new(102, 100, 7);
        let (conn_id, cid) = insert_player(&mut world, player_pos);
        let obj1 = place_bag_on_tile(&mut world, item_pos);
        let obj2 = place_bag_on_tile(&mut world, item_pos2);

        dispatch(
            &mut world,
            conn_id,
            GamePacket::UseItemEx(tfs_rust_common::game_packet::UseItemExPayload {
                from_pos: obj1.pos,
                from_sprite_id: obj1.sprite_id,
                from_stack_pos: obj1.stack_pos,
                to_pos: obj2.pos,
                to_sprite_id: obj2.sprite_id,
                to_stack_pos: obj2.stack_pos,
            }),
        );

        let base = world.creatures.get(cid).unwrap().base();
        assert_eq!(
            base.todo.queue.len(),
            2,
            "Use two-object → [Wait{{100}}, Use]"
        );
        assert!(matches!(
            base.todo.queue[0],
            CreatureAction::Wait { deadline_ms: 100 }
        ));
        match base.todo.queue[1] {
            CreatureAction::Use {
                obj2, open_index, ..
            } => {
                assert!(obj2.is_some(), "two-object use carries obj2");
                assert_eq!(open_index, 0, "UseItemEx has no index byte → 0");
            }
            ref other => panic!("expected Use, got {other:?}"),
        }
        assert!(base.next_wakeup.is_some(), "ToDoStart armed a wakeup");
    }

    /// `Throw` routes to `[Wait{100}, Move]` — D1: `ToDoMove` itself always calls
    /// `this->ToDoWait(100)` (`cract.cc:1155,1165`), even though the `CMoveObject`
    /// handler adds no leading `ToDoWait`. Same queue shape as `Use`/`Turn`.
    #[test]
    fn throw_routes_through_todo_builder_with_wait_floor() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let dest = Position::new(103, 100, 7);
        let (conn_id, cid) = insert_player(&mut world, player_pos);
        ensure_walkable_tile(&mut world.map, dest, TEST_SYNTHETIC_GROUND_WP);
        let obj = place_gold_on_tile(&mut world, item_pos);

        dispatch(
            &mut world,
            conn_id,
            GamePacket::Throw(tfs_rust_common::game_packet::ThrowPayload {
                from_pos: obj.pos,
                sprite_id: obj.sprite_id,
                from_stack_pos: obj.stack_pos,
                to_pos: dest,
                count: 1,
            }),
        );

        let base = world.creatures.get(cid).unwrap().base();
        assert_eq!(
            base.todo.queue.len(),
            2,
            "Move → [Wait{{100}}, Move] (D1: ToDoMove prepends Wait{{100}})"
        );
        assert!(
            matches!(
                base.todo.queue[0],
                CreatureAction::Wait { deadline_ms: 100 }
            ),
            "front = Wait{{100}}"
        );
        match base.todo.queue[1] {
            CreatureAction::Move { dest: d, count, .. } => {
                assert_eq!(d, dest);
                assert_eq!(count, 1);
            }
            ref other => panic!("expected Move, got {other:?}"),
        }
        assert!(base.next_wakeup.is_some(), "ToDoStart armed a wakeup");
    }

    /// `RotateItem` (new arm in S6) routes to `[Wait{100}, Turn]`.
    #[test]
    fn rotate_item_routes_through_todo_builder() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let (conn_id, cid) = insert_player(&mut world, player_pos);
        let obj = place_bag_on_tile(&mut world, item_pos);

        dispatch(
            &mut world,
            conn_id,
            GamePacket::RotateItem {
                pos: obj.pos,
                sprite_id: obj.sprite_id,
                stack_pos: obj.stack_pos,
            },
        );

        let base = world.creatures.get(cid).unwrap().base();
        assert_eq!(base.todo.queue.len(), 2, "Turn → [Wait{{100}}, Turn]");
        assert!(matches!(
            base.todo.queue[0],
            CreatureAction::Wait { deadline_ms: 100 }
        ));
        assert!(matches!(base.todo.queue[1], CreatureAction::Turn { .. }));
        assert!(base.next_wakeup.is_some(), "ToDoStart armed a wakeup");
    }

    /// Phase 4: `RotateItem` now always uses the ToDo `TDTurn` builder for both eras —
    /// the 1098 no-op trace arm was deleted. Phase 6: `beat_driven_loop` is collapsed;
    /// verify it enqueues on the unified beat engine.
    #[test]
    fn rotate_item_enqueues_on_beat_engine() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let item_pos = Position::new(101, 100, 7);
        let (conn_id, cid) = insert_player(&mut world, player_pos);
        let obj = place_bag_on_tile(&mut world, item_pos);

        dispatch(
            &mut world,
            conn_id,
            GamePacket::RotateItem {
                pos: obj.pos,
                sprite_id: obj.sprite_id,
                stack_pos: obj.stack_pos,
            },
        );

        let base = world.creatures.get(cid).unwrap().base();
        assert_eq!(base.todo.queue.len(), 2, "Turn → [Wait{{100}}, Turn]");
        assert!(matches!(
            base.todo.queue[0],
            CreatureAction::Wait { deadline_ms: 100 }
        ));
        assert!(matches!(base.todo.queue[1], CreatureAction::Turn { .. }));
    }

    /// Builder failure (absent object) → `send_cancel_message`, no ToDo entry, no wakeup.
    /// Mirrors C++ `GetObject` `throw RESULT` at enqueue (`receiving.cc` handler catch).
    #[test]
    fn use_item_builder_failure_sends_cancel_and_enqueues_nothing() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let (conn_id, cid) = insert_player(&mut world, player_pos);
        // No item placed at (101,100,7) — sprite 100 won't resolve.

        dispatch(
            &mut world,
            conn_id,
            GamePacket::UseItem(tfs_rust_common::game_packet::UseItemPayload {
                pos: Position::new(101, 100, 7),
                sprite_id: 100,
                stack_pos: 0,
                index: 0,
            }),
        );

        let base = world.creatures.get(cid).unwrap().base();
        assert!(base.todo.is_empty(), "failed builder must not enqueue");
        assert!(
            base.next_wakeup.is_none(),
            "failed builder must not arm a wakeup"
        );
    }

    /// `LookAt` stays reactive — no ToDo entry created (regression, F8 §1).
    #[test]
    fn look_at_stays_reactive_no_todo_entry() {
        let mut world = beat_driven_test_world();
        let player_pos = Position::new(100, 100, 7);
        let (conn_id, cid) = insert_player(&mut world, player_pos);

        dispatch(
            &mut world,
            conn_id,
            GamePacket::LookAt {
                pos: Position::new(101, 100, 7),
                stack_pos: 0,
            },
        );

        let base = world.creatures.get(cid).unwrap().base();
        assert!(base.todo.is_empty(), "LookAt must not create a ToDo entry");
        assert!(base.next_wakeup.is_none());
    }

    /// GL-1: `PlayerLogin` must not await DB — beats continue while a load is in flight.
    #[tokio::test(flavor = "current_thread")]
    async fn player_login_does_not_block_beats_while_load_pending() {
        use std::collections::HashSet;
        use std::ops::ControlFlow;
        use std::time::Duration;

        use tfs_rust_common::ConnId;

        use super::{begin_player_login_load, dispatch_command, try_recv_next_command};

        let mut world = beat_driven_test_world();
        let (tx, mut game_rx, mut ctrl_rx) = tfs_rust_net::open_game_command_channels();
        let mut pending = VecDeque::new();
        let mut pending_logins = HashSet::new();
        let mut login_started = HashMap::new();
        let out_registry = None;
        let mut output_sinks = HashMap::new();

        let login_conn = ConnId(99);
        begin_player_login_load(
            &mut world,
            &tx,
            &mut pending_logins,
            &mut login_started,
            login_conn,
            "DelayedHero".to_string(),
            0,
            0,
            0,
        );
        assert!(
            pending_logins.contains(&login_conn),
            "load must be tracked as in-flight"
        );

        // Inject a delayed failure so the load stays pending across several beats.
        let tx_delay = tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(80)).await;
            // Real load may have already failed; ignore send errors.
            let _ = tx_delay.send(GameCommand::PlayerLoadFailed {
                conn_id: login_conn,
                name: "DelayedHero".to_string(),
                reason: "injected delay".to_string(),
            });
        });

        let before_ms = world.server_ms;
        let beat_ms = u64::from(world.mechanics.profile.beat_ms.max(1));
        for _ in 0..5 {
            world.advance_beat(beat_ms);
            tokio::task::yield_now().await;
        }
        assert!(
            world.server_ms > before_ms,
            "simulation time must advance while login load is pending (before={before_ms}, after={})",
            world.server_ms
        );

        // Drain any load-result commands without blocking the test forever.
        let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
        while tokio::time::Instant::now() < deadline {
            let cmd = try_recv_next_command(&mut game_rx, &mut ctrl_rx, &mut pending);
            match cmd {
                Some(cmd) => {
                    let flow = dispatch_command(
                        &mut world,
                        Some(cmd),
                        &mut game_rx,
                        &tx,
                        &mut pending,
                        &mut pending_logins,
                        &mut login_started,
                        &mut output_sinks,
                        &out_registry,
                    );
                    assert!(matches!(flow, ControlFlow::Continue(())));
                }
                None => {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }
            if pending_logins.is_empty() {
                break;
            }
        }
    }

    /// GL-1: concurrent login load cap rejects excess attempts without awaiting.
    #[tokio::test(flavor = "current_thread")]
    async fn login_load_cap_rejects_without_blocking() {
        use std::collections::HashSet;

        use tfs_rust_common::ConnId;

        use super::begin_player_login_load;
        use crate::login::MAX_CONCURRENT_LOGIN_LOADS;

        let mut world = beat_driven_test_world();
        let (tx, _game_rx, mut ctrl_rx) = tfs_rust_net::open_game_command_channels();
        let mut pending_logins = HashSet::new();
        let mut login_started = HashMap::new();

        for i in 0..MAX_CONCURRENT_LOGIN_LOADS {
            begin_player_login_load(
                &mut world,
                &tx,
                &mut pending_logins,
                &mut login_started,
                ConnId(i as u32),
                format!("Hero{i}"),
                0,
                0,
                0,
            );
        }
        assert_eq!(pending_logins.len(), MAX_CONCURRENT_LOGIN_LOADS);

        begin_player_login_load(
            &mut world,
            &tx,
            &mut pending_logins,
            &mut login_started,
            ConnId(9000),
            "Overflow".to_string(),
            0,
            0,
            0,
        );
        assert_eq!(
            pending_logins.len(),
            MAX_CONCURRENT_LOGIN_LOADS,
            "cap must not grow past MAX"
        );

        let mut saw_reject = false;
        while let Ok(cmd) = ctrl_rx.try_recv() {
            if matches!(
                cmd,
                GameCommand::PlayerLoadFailed {
                    conn_id: ConnId(9000),
                    ..
                }
            ) {
                saw_reject = true;
            }
        }
        assert!(saw_reject, "overflow login must produce PlayerLoadFailed");
    }

    /// GL-2: sustained game-lane flood must not prevent beat advancement when budget yields.
    #[tokio::test(flavor = "current_thread")]
    async fn command_budget_allows_beats_under_game_lane_flood() {
        use std::collections::HashSet;
        use std::ops::ControlFlow;
        use std::time::Duration;

        use tfs_rust_common::{ConnId, enums::Direction};
        use tfs_rust_net::MAX_GAME_COMMANDS_PER_TURN;

        use super::{dispatch_command, drain_ready_beats, try_recv_next_command};

        let mut world = beat_driven_test_world();
        let (tx, mut game_rx, mut ctrl_rx) = tfs_rust_net::open_game_command_channels();
        let mut pending = VecDeque::new();
        let mut pending_logins = HashSet::new();
        let mut login_started = HashMap::new();
        let out_registry = None;
        let mut output_sinks = HashMap::new();

        // Fill well beyond one turn's budget.
        for _ in 0..(MAX_GAME_COMMANDS_PER_TURN * 4) {
            tx.send(GameCommand::Game {
                conn_id: ConnId(1),
                packet: GamePacket::Move(Direction::North),
            })
            .expect("game lane accepts flood in test");
        }

        let beat_ms = u64::from(world.mechanics.profile.beat_ms.max(1));
        let mut beat_timer = tokio::time::interval(Duration::from_millis(beat_ms));
        beat_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Burst);
        // Consume the immediate first tick so subsequent ready ticks are real.
        beat_timer.tick().await;

        let before = world.server_ms;
        // Simulate several command turns with budget + beat yield.
        for _ in 0..8 {
            let mut processed = 0usize;
            while processed < MAX_GAME_COMMANDS_PER_TURN {
                let Some(cmd) = try_recv_next_command(&mut game_rx, &mut ctrl_rx, &mut pending)
                else {
                    break;
                };
                let flow = dispatch_command(
                    &mut world,
                    Some(cmd),
                    &mut game_rx,
                    &tx,
                    &mut pending,
                    &mut pending_logins,
                    &mut login_started,
                    &mut output_sinks,
                    &out_registry,
                );
                assert!(matches!(flow, ControlFlow::Continue(())));
                processed += 1;
            }
            tokio::time::sleep(Duration::from_millis(beat_ms + 1)).await;
            let ready = drain_ready_beats(&mut beat_timer);
            if ready > 0 {
                world.advance_beat(beat_ms * ready);
            }
        }
        assert!(
            world.server_ms > before,
            "beats must advance under sustained game-lane flood"
        );
    }

    /// Audit #1 full-scale: multi-second flood keeps beat lateness bounded by one command budget turn.
    #[tokio::test(flavor = "current_thread")]
    async fn command_flood_bounds_beat_lateness_over_multi_second() {
        use std::collections::HashSet;
        use std::ops::ControlFlow;
        use std::time::{Duration, Instant};

        use tfs_rust_common::{ConnId, enums::Direction};
        use tfs_rust_net::MAX_GAME_COMMANDS_PER_TURN;

        use super::{dispatch_command, drain_ready_beats, new_beat_timer, try_recv_next_command};

        let mut world = beat_driven_test_world();
        let (tx, mut game_rx, mut ctrl_rx) = tfs_rust_net::open_game_command_channels();
        let mut pending = VecDeque::new();
        let mut pending_logins = HashSet::new();
        let mut login_started = HashMap::new();
        let out_registry = None;
        let mut output_sinks = HashMap::new();

        let beat_ms = u64::from(world.mechanics.profile.beat_ms.max(1));
        let (mut beat_timer, _) = new_beat_timer(beat_ms);
        // Wait for the delayed first tick so subsequent ready ticks are real.
        tokio::time::sleep(Duration::from_millis(beat_ms + 5)).await;
        let _ = drain_ready_beats(&mut beat_timer);

        let wall_start = Instant::now();
        let mut max_lateness_ms = 0u64;
        let mut total_ready = 0u64;
        // Keep the lane topped up for ~2 wall seconds.
        while wall_start.elapsed() < Duration::from_secs(2) {
            while tx
                .send(GameCommand::Game {
                    conn_id: ConnId(1),
                    packet: GamePacket::Move(Direction::North),
                })
                .is_ok()
            {
                // Fill until backpressure or enough queued for this turn.
                if pending.len() + 8 >= MAX_GAME_COMMANDS_PER_TURN * 2 {
                    break;
                }
            }

            let turn_start = Instant::now();
            let mut processed = 0usize;
            while processed < MAX_GAME_COMMANDS_PER_TURN {
                let Some(cmd) = try_recv_next_command(&mut game_rx, &mut ctrl_rx, &mut pending)
                else {
                    break;
                };
                let flow = dispatch_command(
                    &mut world,
                    Some(cmd),
                    &mut game_rx,
                    &tx,
                    &mut pending,
                    &mut pending_logins,
                    &mut login_started,
                    &mut output_sinks,
                    &out_registry,
                );
                assert!(matches!(flow, ControlFlow::Continue(())));
                processed += 1;
            }

            tokio::time::sleep(Duration::from_millis(beat_ms)).await;
            let ready = drain_ready_beats(&mut beat_timer);
            if ready > 0 {
                total_ready = total_ready.saturating_add(ready);
                world.advance_beat(beat_ms.saturating_mul(ready));
                let lateness = turn_start.elapsed().as_millis() as u64;
                max_lateness_ms = max_lateness_ms.max(lateness);
            }
        }

        assert!(
            total_ready >= 10,
            "expected multiple beats over 2s flood (got {total_ready})"
        );
        // Budget turn + one beat sleep should keep lateness well under a second.
        assert!(
            max_lateness_ms < 1_000,
            "beat lateness under flood must stay bounded (max={max_lateness_ms}ms)"
        );
    }

    /// Audit #2: outbound SlowClient hard cap sheds the connection deterministically.
    #[test]
    fn outbound_slow_client_flush_sheds_connection() {
        use tfs_rust_common::ConnId;
        use tfs_rust_net::OutboundTx;

        use super::flush_pending_outgoing;

        let mut world = beat_driven_test_world();
        let conn = ConnId(42);
        // Hard cap 100 — a 200-byte flush must SlowClient-shed.
        let (tx, _rx) = OutboundTx::pair_with_caps(8, 50, 100);
        let mut sinks = HashMap::new();
        sinks.insert(conn, tx);
        world.pending_outgoing.insert(conn, vec![vec![0u8; 200]]);
        let mut shed = Vec::new();
        flush_pending_outgoing(&mut world, &mut sinks, &None, &mut shed);
        assert!(
            shed.contains(&conn),
            "SlowClient flush must enqueue pending_output_shed"
        );
        assert!(
            world.obs.output_slow_shed >= 1,
            "OBS must record slow-client shed"
        );
    }

    /// Audit #3: decay + ToDo continue while an async login load is in flight.
    #[tokio::test(flavor = "current_thread")]
    async fn login_load_pending_allows_decay_and_todo() {
        use std::collections::HashSet;

        use tfs_rust_common::ConnId;
        use tfs_rust_content::otb::ItemType;

        use crate::creature::MonsterAiConfig;
        use crate::item::Item;
        use crate::test_world::support::{ensure_walkable_tile, insert_monster_with_config};
        use crate::tile::Tile;

        use super::begin_player_login_load;

        let mut world = beat_driven_test_world();
        world.server_ms = 1_000;

        // Decay item due after clock advance.
        let mut it = ItemType::default();
        it.id = 1490;
        it.server_id = 1490;
        it.decay_time = 1;
        it.decay_to = 0;
        let mut items = std::collections::HashMap::clone(&world.items_db.items);
        items.insert(1490, it);
        let client_to_server = std::collections::HashMap::clone(&world.items_db.client_to_server);
        world.items_db = std::sync::Arc::new(tfs_rust_content::items::ItemDatabase {
            items,
            client_to_server,
        });
        let pos = Position::new(120, 120, 7);
        world.map.insert_tile(pos, Tile::empty_normal());
        let iid = world.items.insert(Item::new_single(1490));
        world.map.get_tile_mut(pos).unwrap().add_item(iid);
        if let Some(item) = world.items.get_mut(iid) {
            item.parent = Some(crate::cylinder::Cylinder::Tile { pos });
        }
        world.start_decay(iid);

        // ToDo: due wakeup during pending login.
        let mpos = Position::new(122, 120, 7);
        ensure_walkable_tile(&mut world.map, mpos, 100);
        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 50, MonsterAiConfig::default());
        world.schedule_creature_wakeup(monster, world.server_ms);

        let (tx, _game_rx, _ctrl_rx) = tfs_rust_net::open_game_command_channels();
        let mut pending_logins = HashSet::new();
        let mut login_started = HashMap::new();
        let login_conn = ConnId(77);
        begin_player_login_load(
            &mut world,
            &tx,
            &mut pending_logins,
            &mut login_started,
            login_conn,
            "SlowHero".to_string(),
            0,
            0,
            0,
        );
        assert!(pending_logins.contains(&login_conn));

        let beat_ms = u64::from(world.mechanics.profile.beat_ms.max(1));
        let before_todo_popped = world.obs.todo_popped;
        for _ in 0..40 {
            world.advance_beat(beat_ms);
            tokio::task::yield_now().await;
        }

        // Force decay expiry if deadline units differ (round vs ms).
        if world.items.get(iid).is_some() {
            let expired = world.decay.tick(u64::MAX / 4);
            world.process_decay_expiry(&expired);
        }
        assert!(
            world.items.get(iid).is_none(),
            "decay must process while login load was/is pending"
        );
        assert!(
            world.obs.todo_popped > before_todo_popped,
            "ToDo drain must run under pending login (before={before_todo_popped}, after={})",
            world.obs.todo_popped
        );
    }

    /// Beat startup: first tick must wait one full beat (772 waits for alarm — `main.cc:484–497`).
    #[tokio::test(flavor = "current_thread")]
    async fn beat_timer_waits_one_beat_before_first_tick() {
        use std::time::{Duration, Instant};

        use super::new_beat_timer;

        let beat_ms = 40u64;
        let (mut timer, deadline) = new_beat_timer(beat_ms);
        assert!(
            deadline > Instant::now(),
            "next_beat_deadline must start one beat in the future"
        );

        // Half a period must not complete — unlike `interval()`, which fires at once.
        let early = tokio::time::timeout(Duration::from_millis(beat_ms / 2), timer.tick()).await;
        assert!(
            early.is_err(),
            "first beat tick must not fire before one full period"
        );

        tokio::time::timeout(Duration::from_millis(beat_ms + 20), timer.tick())
            .await
            .expect("first beat must fire after one period");
    }

    /// 772 `LaunchGame`: `ReceiveData` without a pending alarm does not `SendAll` (`main.cc:488-497`).
    #[tokio::test(flavor = "current_thread")]
    async fn command_only_wakeup_does_not_send_all() {
        use std::collections::HashSet;
        use std::time::Duration;

        use tfs_rust_common::ConnId;

        use super::{new_beat_timer, send_all_if_beat_pending};

        let mut world = beat_driven_test_world();
        let conn = ConnId(1);
        world.pending_outgoing.insert(conn, vec![vec![0xA3]]);
        let beat_ms = u64::from(world.mechanics.profile.beat_ms.max(1));
        let (mut beat_timer, mut deadline) = new_beat_timer(beat_ms);
        let mut logins = HashSet::new();
        let mut sinks = HashMap::new();
        let mut shed = Vec::new();

        send_all_if_beat_pending(
            &mut world,
            &mut beat_timer,
            &mut deadline,
            beat_ms,
            &mut logins,
            &mut sinks,
            &None,
            &mut shed,
        );
        assert!(
            world
                .pending_outgoing
                .get(&conn)
                .is_some_and(|pkts| pkts.iter().any(|b| b.as_slice() == [0xA3])),
            "0xA3 must stay queued until AdvanceGame SendAll, got {:?}",
            world.pending_outgoing.get(&conn)
        );

        tokio::time::sleep(Duration::from_millis(beat_ms + 5)).await;
        send_all_if_beat_pending(
            &mut world,
            &mut beat_timer,
            &mut deadline,
            beat_ms,
            &mut logins,
            &mut sinks,
            &None,
            &mut shed,
        );
        assert!(
            world
                .pending_outgoing
                .get(&conn)
                .is_none_or(|pkts| pkts.is_empty()),
            "SendAll on beat must drain queued 0xA3"
        );
    }

    /// Tokio `Interval` is Ready as soon as the deadline passes. SendAll must run
    /// *before* dispatch so a due beat does not flush this click's `0xA3`.
    #[tokio::test(flavor = "current_thread")]
    async fn due_beat_then_command_keeps_clear_target_queued() {
        use std::collections::HashSet;
        use std::time::Duration;

        use tfs_rust_common::ConnId;

        use super::{new_beat_timer, send_all_if_beat_pending};

        let mut world = beat_driven_test_world();
        let conn = ConnId(1);
        let beat_ms = u64::from(world.mechanics.profile.beat_ms.max(1));
        let (mut beat_timer, mut deadline) = new_beat_timer(beat_ms);
        let mut logins = HashSet::new();
        let mut sinks = HashMap::new();
        let mut shed = Vec::new();

        tokio::time::sleep(Duration::from_millis(beat_ms + 5)).await;
        // Start of command arm: consume the due beat (nothing queued yet).
        send_all_if_beat_pending(
            &mut world,
            &mut beat_timer,
            &mut deadline,
            beat_ms,
            &mut logins,
            &mut sinks,
            &None,
            &mut shed,
        );
        world.pending_outgoing.insert(conn, vec![vec![0xA3]]);
        // End of the same arm must not SendAll — interval was already drained.
        send_all_if_beat_pending(
            &mut world,
            &mut beat_timer,
            &mut deadline,
            beat_ms,
            &mut logins,
            &mut sinks,
            &None,
            &mut shed,
        );
        assert!(
            world
                .pending_outgoing
                .get(&conn)
                .is_some_and(|pkts| pkts.iter().any(|b| b.as_slice() == [0xA3])),
            "0xA3 from this click must wait until the next beat, got {:?}",
            world.pending_outgoing.get(&conn)
        );

        tokio::time::sleep(Duration::from_millis(beat_ms + 5)).await;
        send_all_if_beat_pending(
            &mut world,
            &mut beat_timer,
            &mut deadline,
            beat_ms,
            &mut logins,
            &mut sinks,
            &None,
            &mut shed,
        );
        assert!(
            world
                .pending_outgoing
                .get(&conn)
                .is_none_or(|pkts| pkts.is_empty()),
            "next beat SendAll must drain queued 0xA3"
        );
    }

    /// Login burst (`0x0A` self-appear) uses the same SendAll gate as commands (`crplayer.cc:199`).
    #[tokio::test(flavor = "current_thread")]
    async fn login_burst_waits_for_beat_send_all() {
        use std::collections::HashSet;
        use std::time::Duration;

        use tfs_rust_common::ConnId;

        use super::{new_beat_timer, send_all_if_beat_pending};

        let mut world = beat_driven_test_world();
        let conn = ConnId(1);
        world.pending_outgoing.insert(conn, vec![vec![0x0A]]);
        let beat_ms = u64::from(world.mechanics.profile.beat_ms.max(1));
        let (mut beat_timer, mut deadline) = new_beat_timer(beat_ms);
        let mut logins = HashSet::new();
        let mut sinks = HashMap::new();
        let mut shed = Vec::new();

        send_all_if_beat_pending(
            &mut world,
            &mut beat_timer,
            &mut deadline,
            beat_ms,
            &mut logins,
            &mut sinks,
            &None,
            &mut shed,
        );
        assert!(
            world
                .pending_outgoing
                .get(&conn)
                .is_some_and(|pkts| pkts.iter().any(|b| b.as_slice() == [0x0A])),
            "login 0x0A must stay queued until AdvanceGame SendAll, got {:?}",
            world.pending_outgoing.get(&conn)
        );

        tokio::time::sleep(Duration::from_millis(beat_ms + 5)).await;
        send_all_if_beat_pending(
            &mut world,
            &mut beat_timer,
            &mut deadline,
            beat_ms,
            &mut logins,
            &mut sinks,
            &None,
            &mut shed,
        );
        assert!(
            world
                .pending_outgoing
                .get(&conn)
                .is_none_or(|pkts| pkts.is_empty()),
            "SendAll on beat must drain queued login 0x0A"
        );
    }

    // Suppress unused-import warning for `Player` (re-exported via test_player; kept for
    // future tests that construct a Player directly).
    #[allow(dead_code)]
    fn _suppress_player_import() -> Player {
        test_player("unused", Position::new(0, 0, 7))
    }
}
