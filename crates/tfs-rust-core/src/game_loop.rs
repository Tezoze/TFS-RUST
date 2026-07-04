//! Tokio-driven game loop: command drain + `GameWorld::tick`.
//!
//! - **Both eras:** 772 beat-driven loop + ToDoQueue — [`run_game_loop_772`].
//!   Phase 5 deleted the 1098 reactive loop (`run_game_loop_1098`); 1098 now runs on the
//!   unified beat loop. Per-era differences live in `MechanicsProfile` / `ProtocolCodec`.
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

use tfs_rust_common::{GameCommand, GamePacket};
use tokio::sync::mpsc::error::TryRecvError;
use tracing::{error, info, trace, warn};

use crate::creature_todo::ActionObjectRef;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
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
                warn!(
                    ?e,
                    ?cid,
                    "build_player_save_data failed during shutdown flush"
                );
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
        trace!(
            batches = flushed.len(),
            "flushed outgoing (no registry — packets dropped)"
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
    )
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
    match crate::login::login_player(world, &name, operating_system, otclient_v8).await {
        Ok(cid) => {
            world.register_conn_mapping(conn_id, cid);
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
    world.unregister_conn_mapping(conn_id);
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
) {
    let now = Instant::now();
    if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
        // Phase 4: 1098 no longer resets rounds differently — both eras use the 772
        // `ProcessConnections` round tracking.
        world.player_reset_connection_rounds(
            cid,
            crate::connections_772::packet_counts_as_action_772(&packet),
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
            raw_chase_mode,
            ..
        } => {
            if let Some(cid) = world.conn_to_creature.get(&conn_id).copied() {
                // 772 `TCombat::SetChaseMode` — `crcombat.cc:339-345` (only NONE/CLOSE accepted).
                // Fight mode / secure mode storage is deferred to the player weapon-combat system.
                let chase = match raw_chase_mode {
                    0 => crate::creature::ChaseMode::None,
                    1 => crate::creature::ChaseMode::Close,
                    other => {
                        tracing::warn!(
                            conn_id = conn_id.0,
                            raw_chase_mode = other,
                            "FightModes: 772 SetChaseMode only accepts NONE(0)/CLOSE(1); clamping to NONE"
                        );
                        crate::creature::ChaseMode::None
                    }
                };
                if let Some(k) = world.creatures.get_mut(cid) {
                    // Do not override `Close` forced by an active follow (`Following ⇒ CLOSE`).
                    if k.base().follow_target.is_none() {
                        k.base_mut().chase_mode = chase;
                    }
                }
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
                if let Err(rv) =
                    world.enqueue_player_move(cid, obj, payload.to_pos, payload.count)
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
            pending.push_back(GameCommand::PlayerDisconnect {
                conn_id,
                display_effect: true,
            });
        }
        _ => trace!(
            conn_id = conn_id.0,
            ?packet,
            "game packet — simulation Phase 9+"
        ),
    }
    // Phase 4: 1098 `process_walk_deadlines` call deleted — both eras use the ToDo queue.
}

async fn dispatch_command(
    world: &mut GameWorld,
    cmd: Option<GameCommand>,
    cmd_rx: &mut UnboundedReceiver<GameCommand>,
    pending: &mut VecDeque<GameCommand>,
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
            handle_player_disconnect(world, conn_id, display_effect, out_registry);
            ControlFlow::Continue(())
        }
        GameCommand::Game { conn_id, packet } => {
            handle_game_packet(world, conn_id, packet, cmd_rx, pending);
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

/// 772 beat-driven loop — `LaunchGame` + `AdvanceGame` + `SendAll`.
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
                let mut beats = drain_burst_beats(&mut beat_timer);
                if beats == 0 {
                    beats = 1;
                }
                world.advance_beat_772(beat_ms * beats);
                while let Some(conn_id) = world.pending_idle_kick_772.pop() {
                    handle_player_disconnect(&mut world, conn_id, false, &out_registry);
                }
                flush_pending_outgoing(&mut world, &out_registry);
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
        assert!(!game_packet_requires_timed_action(&GamePacket::RotateItem {
            pos: tfs_rust_common::Position::new(0, 0, 7),
            sprite_id: 100,
            stack_pos: 0,
        }));
    }

    #[test]
    fn beat_driven_loop_flag_follows_linear_go_profile() {
        use crate::formulas::StepSpeedModel;
        let mut world = crate::test_world::support::minimal_world();
        assert!(!world.beat_driven_loop);
        world.mechanics.profile.step_speed = StepSpeedModel::LinearGo;
        world.beat_driven_loop = world.mechanics.profile.step_speed == StepSpeedModel::LinearGo;
        assert!(world.beat_driven_loop);
    }
}

/// F8 S6 — handler routing tests.
///
/// Verifies `handle_game_packet` routes `UseItem`/`UseItemEx`/`Throw`/`RotateItem` through
/// the ToDo builders (`enqueue_player_use`/`enqueue_player_move`/`enqueue_player_turn`) +
/// `todo_start_from_action` when `beat_driven_loop`, instead of the reactive executors.
/// C++ ref: `receiving.cc:384/430/233/549` (`CUseObject`/`CUseTwoObjects`/`CMoveObject`/
/// `CTurnObject`) → `ToDo*` builder + `ToDoStart` (`cract.cc:955-1024`).
#[cfg(test)]
mod f8_s6_handler_routing_tests {
    use std::collections::VecDeque;

    use tokio::sync::mpsc;

    use tfs_rust_common::{ConnId, GameCommand, GamePacket, Position};

    use crate::creature::{CreatureKind, Player};
    use crate::creature_todo::{ActionObjectRef, CreatureAction};
    use crate::item::Item;
    use crate::test_world::support::{
        beat_driven_test_world, ensure_walkable_tile, test_player, TEST_SYNTHETIC_GROUND_WP,
    };

    use super::handle_game_packet;

    /// Place a bag (container, client_id=0 in the test items_db) on a tile and return its
    /// `ActionObjectRef`. Mirrors `creature_todo` tests' `place_bag_on_tile`.
    fn place_bag_on_tile(world: &mut crate::game_world::GameWorld, pos: Position) -> ActionObjectRef {
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
    fn place_gold_on_tile(world: &mut crate::game_world::GameWorld, pos: Position) -> ActionObjectRef {
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

    fn insert_player(world: &mut crate::game_world::GameWorld, pos: Position) -> (ConnId, crate::ids::CreatureId) {
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let cid = world.creatures.insert(CreatureKind::Player(test_player("S6Hero", pos)));
        world.map.register_creature_at(pos, cid);
        let conn_id = ConnId(1);
        world.register_conn_mapping(conn_id, cid);
        (conn_id, cid)
    }

    /// Drive one packet through `handle_game_packet` with empty cmd/pending queues.
    fn dispatch(world: &mut crate::game_world::GameWorld, conn_id: ConnId, packet: GamePacket) {
        let (_tx, mut rx) = mpsc::unbounded_channel::<GameCommand>();
        let mut pending = VecDeque::new();
        handle_game_packet(world, conn_id, packet, &mut rx, &mut pending);
        // None of the rerouted opcodes push to pending (only Logout does).
        assert!(pending.is_empty(), "Use/Throw/RotateItem must not push commands");
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
        assert!(matches!(base.todo.queue[0], CreatureAction::Wait { delay_ms: 100 }));
        match base.todo.queue[1] {
            CreatureAction::Use { obj2, open_index, .. } => {
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
        assert_eq!(base.todo.queue.len(), 2, "Use two-object → [Wait{{100}}, Use]");
        assert!(matches!(base.todo.queue[0], CreatureAction::Wait { delay_ms: 100 }));
        match base.todo.queue[1] {
            CreatureAction::Use { obj2, open_index, .. } => {
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
            matches!(base.todo.queue[0], CreatureAction::Wait { delay_ms: 100 }),
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
        assert!(matches!(base.todo.queue[0], CreatureAction::Wait { delay_ms: 100 }));
        assert!(matches!(base.todo.queue[1], CreatureAction::Turn { .. }));
        assert!(base.next_wakeup.is_some(), "ToDoStart armed a wakeup");
    }

    /// Phase 4: `RotateItem` now always uses the ToDo `TDTurn` builder for both eras —
    /// the 1098 no-op trace arm was deleted. Verify it enqueues even when
    /// `beat_driven_loop = false` (1098 without `TFS_FORCE_BEAT_LOOP`).
    #[test]
    fn rotate_item_enqueues_even_when_not_beat_driven() {
        let mut world = beat_driven_test_world();
        world.beat_driven_loop = false;
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
        assert!(matches!(base.todo.queue[0], CreatureAction::Wait { delay_ms: 100 }));
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
        assert!(base.next_wakeup.is_none(), "failed builder must not arm a wakeup");
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

    // Suppress unused-import warning for `Player` (re-exported via test_player; kept for
    // future tests that construct a Player directly).
    #[allow(dead_code)]
    fn _suppress_player_import() -> Player {
        test_player("unused", Position::new(0, 0, 7))
    }
}
