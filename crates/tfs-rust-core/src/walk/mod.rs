//! TFS 1.4.2 walking (1:1 targets in this repo’s `src/` tree):
//!
//! - `Game::playerMove` / `playerAutoWalk` / `playerStopAutoWalk` — `game.cpp` (~1880, ~2075, ~2087).
//! - `Creature::startAutoWalk`, `addEventWalk`, `onWalk`, `getNextStep`, `getEventStepTicks`,
//!   `getWalkDelay`, `getStepDuration` — `creature.cpp` (~200–322, ~1485–1547).
//! - `Player::onWalk(Direction&)` (`nextAction` / `getStepDuration(dir)` **before** move) — `player.cpp` (~1339–1343).
//! - `Creature::onCreatureMove` (`lastStep` / `lastStepCost`) — `creature.cpp` (~485–499).
//! - `Map::moveCreature` (facing from dx/dy) — `map.cpp` (~295–306).
//! - `Game::checkCreatureWalk` — `game.cpp` (~3773–3779).
//!
//! **Partial:** cardinal **floor change** before `queryAdd` (`game.cpp` ~804–834); `queryDestination`
//! chaining (`game.cpp` ~863–880), full PZ / `Tile::queryAdd`, Lua — not ported.
//!
//! **Timing:** `get_walk_delay` uses `last_step_ground_speed` (**destination** tile of the completed step,
//! OTCv8 / TFS `getWalkDelay`). When `walk_delay <= 0`, `get_event_step_ticks` uses the **current** tile for
//! the *next* step. Wall `Instant::now()` samples (C++ `OTSYS_TIME()`).
//! 772: per-step waypoints include diagonal `×3` **before** Beat ceil (`cract.cc:1454–1462`); TFS 1098
//! keeps cardinal duration × `last_step_cost` after ceil (`creature.cpp`).
//! `next_walk_check` stores the **logical** deadline. Initial arms from a new move use `walk_sched_base`;
//! reschedules after a step match C++ `addEventWalk` by anchoring to `Instant::now()` at reschedule time
//! (`tasks/walk-audit.md` Issue 3).
//!
//! **Scheduling (Phase 5):** the 1098 reactive walk-wake machinery (`walk_wake_tx`,
//! `tokio::time::sleep_until` one-shots, `process_walk_deadlines` polling fallback) is deleted.
//! Both eras now schedule steps through the 772 ToDo queue (`schedule_creature_wakeup` +
//! `next_wakeup`). The `Instant`-based `next_walk_check` / `walk_timer` fields are gone.
//!
//! Speed/timing: [`walk_timing`]. Tile traversal: [`walk_tile`].

use std::time::Instant;

use rand::thread_rng;
use tfs_rust_common::enums::{ConditionType, Direction};
use tfs_rust_common::Position;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_net::map_description::{
    send_map_description_packet, send_move_creature_player, send_move_creature_spectator,
    send_notify_go, TileContent,
};
use tfs_rust_net::outgoing_extra::send_text_message_simple;

use crate::chase_debug;
use crate::combat::uniform_random;
use crate::creature::CreatureKind;
use crate::creature_todo::{trace_creature_todo, CreatureAction};
use crate::game_world::{DeferredTurnBroadcast, GameWorld};
use crate::ids::CreatureId;
use crate::login_out::{creature_wire_id, map_tile_content};
use crate::return_value::ReturnValue;
use crate::tile::{
    client_creature_stack_pos, client_creature_stack_pos_cip, creature_stack_pos_for_viewer,
};
use tfs_rust_common::ConnId;

/// C++ `cylinder.h` — `Tile::queryAdd` / `internalMoveCreature` flags.
const FLAG_NOLIMIT: u32 = 1 << 0;
pub(crate) const FLAG_IGNOREBLOCKITEM: u32 = 1 << 1;
const FLAG_IGNOREBLOCKCREATURE: u32 = 1 << 2;
const FLAG_PATHFINDING: u32 = 1 << 4;
const FLAG_IGNOREFIELDDAMAGE: u32 = 1 << 5;

/// Pathfinding query flags — `Map::canWalkTo` (`map.cpp` ~638).
///
/// Does NOT include `FLAG_IGNOREFIELDDAMAGE` — the 772 `MovePossible(Execute=false)`
/// (`crmain.cc:893`) blocks on `AVOID` (magic fields) during pathfinding. Actual walk
/// execution (line ~1713) passes `FLAG_IGNOREFIELDDAMAGE` directly so the player can
/// walk through fields but takes damage, matching `MovePossible(Execute=true)` which
/// skips the `AVOID` check.
pub(crate) const PATHFIND_WALK_FLAGS: u32 = FLAG_PATHFINDING;

/// Self-move / segment `oldStackPos` for the moving creature's own connection.
fn self_move_stack_pos(world: &GameWorld, cid: CreatureId, body: &crate::tile::TileBody) -> i32 {
    let is_772 = !world.codec.caps().move_creature_self_packet;
    let is_otc = world
        .creatures
        .get(cid)
        .is_some_and(|k| matches!(k, CreatureKind::Player(p) if p.is_otclient()));
    if is_772 && !is_otc {
        client_creature_stack_pos_cip(body, cid)
    } else {
        client_creature_stack_pos(body, cid)
    }
}

/// One movement segment emitted by `internal_move_creature_step`.
/// C++ `map.moveCreature` emits a packet per call; we collect segments and emit afterwards.
struct MoveSegment {
    from: Position,
    to: Position,
    old_stack: i32,
    /// C++ `Map::moveCreature`: `teleport = forceTeleport || !ground || !areInRange<1,1,0>`
    teleport: bool,
}

/// Pending chain turn deferred from `internal_move_creature_step` — the direction
/// is set immediately (matching C++ `internalCreatureTurn` state mutation), but the
/// `0x6B` broadcast is deferred until AFTER move packets are emitted in `on_walk`.
/// C++ order: `Map::moveCreature` sends `sendMoveCreature` during the move loop
/// (`map.cpp:316`), THEN `Game::internalMoveCreature` calls `internalCreatureTurn`
/// → `sendCreatureTurn` (`0x6B`) after the loop (`game.cpp:888`). Rust previously
/// emitted `0x6B` inside `internal_move_creature_step` (before move packets in
/// `on_walk`), causing the client to receive `0x6B` for a position it hasn't seen
/// the creature move to yet → "no thing at pos" errors.
struct PendingChainTurn {
    cid: CreatureId,
    dir: Direction,
}

/// C++ `Position::areInRange<1,1,0>` — dx<=1, dy<=1, dz==0.
pub(crate) fn are_in_range_1_1_0(a: Position, b: Position) -> bool {
    let dx = (a.x as i32 - b.x as i32).unsigned_abs();
    let dy = (a.y as i32 - b.y as i32).unsigned_abs();
    let dz = (a.z as i32 - b.z as i32).unsigned_abs();
    dx <= 1 && dy <= 1 && dz == 0
}

/// 772 `NotifyGo` adjacent condition: `DistanceX <= 1 && DistanceY <= 1 && DistanceZ <= 1`
/// (`cract.cc:1421`). 772 uses `SendFloors` (0xBE/0xBF) + `SendRow` (0x65-0x68) for
/// adjacent z-changes — an incremental floor update. Only `DistanceZ > 1` (or dx/dy > 1)
/// triggers `SendFullScreen` (0x64). 1098 uses `areInRange<1,1,0>` (dz==0) — z-changes
/// are always teleports (full screen 0x64). See `docs/772_FLOOR_CHANGE_DESYNC.md`.
fn are_in_range_1_1_1(a: Position, b: Position) -> bool {
    let dx = (a.x as i32 - b.x as i32).unsigned_abs();
    let dy = (a.y as i32 - b.y as i32).unsigned_abs();
    let dz = (a.z as i32 - b.z as i32).unsigned_abs();
    dx <= 1 && dy <= 1 && dz <= 1
}

/// Era- and client-aware teleport range check.
///
/// - **1098** (`move_creature_self_packet == true`): TVP `areInRange<1,1,0>` — `dz == 0`
///   required, so any z-change is a teleport (full-screen `0x64`).
/// - **772 real client** (`!otclient`): decompile `NotifyGo` adjacent condition —
///   `DistanceZ <= 1` uses incremental `SendFloors`/`SendRow`; only `dz > 1` (or
///   dx/dy > 1) is a teleport.
/// - **772 OTClient** (`otclient`): TVP contract — OTClient tracks the local player
///   as a tile creature and cannot reconcile the decompile's incremental floor/row
///   stream, so it gets the same `dz == 0` rule as 1098 (z-changes → `0x64`).
///   See `docs/772_FLOOR_CHANGE_CLIENT_TARGETS.md` §6.
///
/// `otclient` is the connection's OTClient flag (`Player::is_otclient`); the dispatch
/// site owns the policy decision and threads the bool in here rather than reaching
/// into world state from a free function.
fn is_adjacent_move(
    codec: &tfs_rust_net::codec::Codec,
    otclient: bool,
    a: Position,
    b: Position,
) -> bool {
    if codec.caps().move_creature_self_packet {
        // 1098: `areInRange<1,1,0>` — z-changes are teleports.
        are_in_range_1_1_0(a, b)
    } else if otclient {
        // 772 OTClient: TVP contract — z-changes are teleports (full-screen 0x64).
        are_in_range_1_1_0(a, b)
    } else {
        // 772 real client: `NotifyGo` adjacent condition — `DistanceZ <= 1` uses
        // SendFloors/SendRow.
        are_in_range_1_1_1(a, b)
    }
}

mod walk_tile;
mod walk_timing;

use walk_tile::{
    query_destination, resolve_player_move_destination, tile_query_add_monster, tile_query_add_npc,
    tile_query_add_player,
};
use walk_timing::{
    get_event_step_ticks, last_step_cost_for_move, peek_next_walk_direction,
    walk_timing_speed_kind,
};
pub(crate) use walk_timing::{get_step_duration_ms_with_direction, wire_step_speed, WalkSpeedRole};

#[inline]
fn is_diagonal(direction: Direction) -> bool {
    matches!(
        direction,
        Direction::NorthEast | Direction::NorthWest | Direction::SouthEast | Direction::SouthWest
    )
}

fn has_drunk_condition(base: &crate::creature::CreatureBase) -> bool {
    base.active_conditions
        .iter()
        .any(|c| c.ctype == ConditionType::Drunk)
        || base.drunkenness > 0
}

/// 772 drunk stagger — `cract.cc:392-413`: `DrunkLevel = Skills[SKILL_DRUNKEN]->TimerValue()`,
/// `StaggerChance = max(7 - DrunkLevel, 1)`, `rand() % StaggerChance == 0` → random cardinal.
///
/// `DrunkLevel` maps to `base.drunkenness` (set by `SpellImpact::Drunk`). The CipSoft
/// `Get() == 0` skill-level check is implicitly true (no CipSoft skill system in Rust).
/// Returns `Some(dir)` when the step should be replaced with a random cardinal stagger.
fn try_drunk_walk_direction(base: &crate::creature::CreatureBase) -> Option<Direction> {
    if !has_drunk_condition(base) {
        return None;
    }
    let drunk_level = base.drunkenness as i32;
    let stagger_chance = (7 - drunk_level).max(1) as u32;
    let r = uniform_random(
        &mut thread_rng(),
        0,
        (stagger_chance as i32).saturating_sub(1),
    ) as u32;
    if r != 0 {
        return None;
    }
    let dir_roll = uniform_random(&mut thread_rng(), 0, 3) as u32;
    Some(match dir_roll {
        0 => Direction::North,
        1 => Direction::East,
        2 => Direction::South,
        _ => Direction::West,
    })
}

pub(crate) fn ground_speed_for_tile_body(
    body: &crate::tile::TileBody,
    items_db: &ItemDatabase,
) -> u32 {
    let Some(gid) = body.ground else {
        return 150;
    };
    items_db.ground_speed_for_item(gid)
}

/// TFS `Position::getDirectionTo` — cardinal/diagonal direction between two positions.
/// C++ ref: src/position.h getDirectionTo
fn direction_from_positions(from: Position, to: Position) -> Direction {
    let dx = to.x as i32 - from.x as i32;
    let dy = to.y as i32 - from.y as i32;
    match (dx.signum(), dy.signum()) {
        (0, -1) => Direction::North,
        (0, 1) => Direction::South,
        (1, 0) => Direction::East,
        (-1, 0) => Direction::West,
        (1, -1) => Direction::NorthEast,
        (-1, -1) => Direction::NorthWest,
        (1, 1) => Direction::SouthEast,
        (-1, 1) => Direction::SouthWest,
        _ => Direction::South, // fallback
    }
}

/// Whether `cid` can stand on `pos` (non-pathfinding `Tile::queryAdd`).
pub(crate) fn player_can_stand_at(world: &GameWorld, cid: CreatureId, pos: Position) -> bool {
    let Some(tile) = world.map.get_tile(pos) else {
        return false;
    };
    tile_query_add_player(world, tile, cid, 0) == ReturnValue::NoError
}

/// TFS `Game::internalTeleport` for players — `game.cpp` ~1784–1804.
pub(crate) fn internal_teleport_player(
    world: &mut GameWorld,
    conn_id: ConnId,
    cid: CreatureId,
    new_pos: Position,
) -> ReturnValue {
    let old_pos = match world.creatures.get(cid) {
        Some(k) => k.position(),
        None => return ReturnValue::NotPossible,
    };
    if old_pos == new_pos {
        return ReturnValue::NoError;
    }
    let Some(to_tile) = world.map.get_tile(new_pos) else {
        return ReturnValue::NotPossible;
    };
    if tile_query_add_player(world, to_tile, cid, FLAG_NOLIMIT) != ReturnValue::NoError {
        return ReturnValue::NotPossible;
    }

    let old_stack = world
        .map
        .get_tile(old_pos)
        .map(|t| self_move_stack_pos(world, cid, t.body()))
        .filter(|s| *s >= 0)
        .unwrap_or(1);

    world.move_creature_on_map(cid, old_pos, new_pos);
    if let Some(k) = world.creatures.get_mut(cid) {
        k.set_position(new_pos);
    }

    world.emit_teleport_move_packet(cid, conn_id, old_pos, new_pos, old_stack);
    ReturnValue::NoError
}

/// TFS `Tile::queryAdd` dispatch for creatures (`tile.cpp` ~484–628).
pub(crate) fn tile_query_add_creature(
    world: &GameWorld,
    tile: &crate::tile::Tile,
    mover: CreatureId,
    flags: u32,
) -> ReturnValue {
    match world.creatures.get(mover) {
        Some(CreatureKind::Player(_)) => tile_query_add_player(world, tile, mover, flags),
        Some(CreatureKind::Monster(_)) => tile_query_add_monster(world, tile, mover, flags),
        Some(CreatureKind::Npc(_)) => tile_query_add_npc(world, tile, mover, flags),
        None => ReturnValue::NotPossible,
    }
}

fn set_direction_from_step(old_pos: Position, new_pos: Position, creature: &mut CreatureKind) {
    let teleport = old_pos.z != new_pos.z
        || (old_pos.x as i32 - new_pos.x as i32).abs() > 1
        || (old_pos.y as i32 - new_pos.y as i32).abs() > 1;
    if teleport {
        return;
    }
    let mut d = None;
    if old_pos.y > new_pos.y {
        d = Some(Direction::North);
    } else if old_pos.y < new_pos.y {
        d = Some(Direction::South);
    }
    if old_pos.x < new_pos.x {
        d = Some(Direction::East);
    } else if old_pos.x > new_pos.x {
        d = Some(Direction::West);
    }
    if let Some(dir) = d {
        creature.base_mut().direction = dir;
    }
}

/// 772 `NotifyTurn` facing update for KickCreature (`cract.cc:1566–1581`) — state only.
pub(crate) fn set_direction_from_step_for_kick(
    old_pos: Position,
    new_pos: Position,
    creature: &mut CreatureKind,
) {
    set_direction_from_step(old_pos, new_pos, creature);
}

/// TFS `Game::internalCreatureTurn` (`game.cpp` ~3703–3721).
///
/// Sets the creature's direction **and** broadcasts a `0x6B` creature-turn packet to every
/// player-spectator that can see the position.  No-op when direction is already equal
/// (mirrors the C++ `if (creature->getDirection() == dir) return false;` guard).
///
/// Called exclusively from the post-`queryDestination` chain step in
/// `internal_move_creature_step` — post-`queryDestination` chain turn (`game.cpp` ~882–891).
/// Broadcast creature turn (`0x6B`) — used by walk chain and monster look-at-target.
pub(crate) fn creature_turn_with_broadcast(world: &mut GameWorld, cid: CreatureId, dir: Direction) {
    internal_creature_turn_with_broadcast(world, cid, dir);
}

fn internal_creature_turn_with_broadcast(world: &mut GameWorld, cid: CreatureId, dir: Direction) {
    // Guard: no-op when direction unchanged — matches C++ early-return.
    let old_dir = match world.creatures.get(cid) {
        Some(k) => k.base().direction,
        None => return,
    };
    if old_dir == dir {
        return;
    }

    // Mutate direction in creature state.
    if let Some(k) = world.creatures.get_mut(cid) {
        k.base_mut().direction = dir;
    }

    internal_creature_turn_broadcast_only(world, cid, dir);
}

/// Emit the `0x6B` turn broadcast WITHOUT mutating direction state. Used by
/// `on_walk` to emit the deferred chain turn AFTER move packets — the direction
/// was already set in `internal_move_creature_step` (matching C++ state mutation
/// order); this only sends the wire packet.
fn internal_creature_turn_broadcast_only(world: &mut GameWorld, cid: CreatureId, dir: Direction) {
    // Gather wire id, position, stack position (needed for the 0x6B wire format).
    let (wire_id, pos) = match world.creatures.get(cid) {
        Some(k) => (creature_wire_id(cid, k), k.position()),
        None => return,
    };
    let stack_u8 = world
        .map
        .get_tile(pos)
        .map(|t| {
            let raw = client_creature_stack_pos(t.body(), cid);
            if !(0..10).contains(&raw) {
                10u8
            } else {
                raw as u8
            }
        })
        .unwrap_or(10);

    // Broadcast `0x6B` to ALL spectators (inc. the mover) that can see the position.
    // C++ `map.getSpectators(spectators, pos, true, true)` → players only.
    // Grid-based fan-out (audit #4) — `spectator_conns_via_grid` applies `can_see_position`.
    let spectators: Vec<ConnId> = world.spectator_conns_via_grid(pos);

    let packet = world
        .codec
        .encode_creature_turn(wire_id, stack_u8, pos, dir as u8, false)
        .into_bytes();
    for conn in spectators {
        if world.is_creature_fully_sent_to_conn(conn, wire_id) {
            world.enqueue_outgoing(conn, packet.clone());
        }
    }
}

impl GameWorld {
    /// Drain the global ToDoQueue for entries due at or before [`Self::server_ms`].
    ///
    /// C++ `MoveCreatures` (`crmain.cc:1144`) drains unconditionally
    /// (`while ToDoQueue.Entries > 0 && top.Key <= ServerMilliseconds`). The `+1` re-insertion clamp
    /// (`ToDoStart`, audit Finding 17) guarantees a re-armed creature lands strictly in the future,
    /// so this cannot spin within a beat — no per-beat cap is needed (audit Finding 10).
    pub fn drain_todo_queue(&mut self) {
        // Safety valve only: the `+1` clamp makes same-beat re-entry impossible, so this bound is
        // never reached in correct operation. If it ever trips it indicates a re-arm at
        // `<= server_ms` (a real bug) — log loudly rather than silently deferring work.
        const RUNAWAY_GUARD: usize = 1_000_000;
        let heap_before = self.todo_queue.len();
        let mut drained = 0usize;
        let mut executed = 0u64;
        let mut stale = 0u64;
        while let Some(entry) = self.todo_queue.peek() {
            if entry.execution_time > self.server_ms {
                break;
            }
            if drained >= RUNAWAY_GUARD {
                tracing::error!(
                    server_ms = self.server_ms,
                    "drain_todo_queue runaway guard tripped — a creature is re-arming at <= server_ms (ToDoStart +1 clamp violated)"
                );
                break;
            }
            let entry = self.todo_queue.pop().expect("peek implied non-empty heap");
            drained += 1;
            // C++ `Execute` runs the creature iff its current `NextWakeup <= ServerMilliseconds`
            // (`cract.cc:785`), regardless of whether this popped entry is its *latest* schedule —
            // not an exact-key match (audit Finding 9).
            let next_wakeup_snap = self
                .creatures
                .get(entry.creature_id)
                .and_then(|k| k.base().next_wakeup);
            let due = next_wakeup_snap.is_some_and(|w| w <= self.server_ms);
            tracing::debug!(
                ?entry.creature_id,
                entry_time = entry.execution_time,
                server_ms = self.server_ms,
                next_wakeup = ?next_wakeup_snap,
                due,
            );
            if due {
                let lateness = self.server_ms.saturating_sub(entry.execution_time);
                self.obs.record_todo_lateness_ms(lateness);
                self.process_creature_todo(entry.creature_id);
                executed = executed.saturating_add(1);
            } else {
                stale = stale.saturating_add(1);
            }
        }
        self.obs
            .record_todo_drain(heap_before, drained as u64, executed, stale);
        // Phase 3 (audit Finding 8): the per-beat `rescue_stalled_chase_monsters_772` band-aid is
        // removed — with the verbatim heap (Phase 1), the `+1` clamp and `<= server_ms` drain
        // (Phase 2), a creature must always either re-insert or idle, so none can strand.
    }

    /// 772 `TCreature::Execute` walk path — one due heap entry (`cract.cc:728`).
    pub fn process_creature_todo(&mut self, cid: CreatureId) {
        let health_ok = self.creatures.get(cid).is_some_and(|k| k.base().health > 0);
        if !health_ok {
            return;
        }
        let had_wakeup = self
            .creatures
            .get_mut(cid)
            .and_then(|k| k.base_mut().next_wakeup.take());
        if had_wakeup.is_none() {
            return;
        }
        let now = Instant::now();
        // F8 S7 / Phase 5 — the `walk_action` deferral branch is removed. After S6, all 772
        // player actions (Use/Move/Turn) route through ToDo builders at packet receipt;
        // `walk_action` is never set for 772 players. Phase 5 deleted the 1098 reactive
        // `process_walk_action_tasks` drain — both eras use the ToDoQueue.
        if self.creature_uses_todo_execute(cid) {
            tracing::debug!(
                ?cid,
                server_ms = self.server_ms,
                todo_queue_len = self
                    .creatures
                    .get(cid)
                    .map(|k| k.base().todo.queue.len())
                    .unwrap_or(0),
                walk_queue_len = self
                    .creatures
                    .get(cid)
                    .map(|k| k.base().walk_queue.len())
                    .unwrap_or(0),
            );
            trace_creature_todo(self, cid, "process_creature_todo");
            let mut ran_idle = false;
            if self.creature_todo_queue_empty(cid) {
                self.creature_todo_release_lock_if_drained(cid);
                self.maybe_idle_stimulus_after_go_complete(cid);
                ran_idle = true;
            }
            if !self.creature_todo_queue_empty(cid) {
                if ran_idle {
                    let front_is_go = self
                        .creatures
                        .get(cid)
                        .is_some_and(|k| {
                            matches!(k.base().todo.queue.front(), Some(CreatureAction::Go))
                        });
                    if front_is_go {
                        if self
                            .creatures
                            .get(cid)
                            .and_then(|k| k.base().next_wakeup)
                            .is_none()
                        {
                            // C++ `IdleStimulus` queues `ToDoGo` then `TDAttack`; `ToDoStart` arms
                            // `NextWakeup` — no synchronous `Go` on the idle drain tick (`cract.cc:1461`).
                            let _ = self.todo_start_go_delay(cid, true);
                        }
                    }
                    // C++ `Execute` breaks after `IdleStimulus` — fresh batch runs at `ToDoStart`
                    // wakeup (`cract.cc:789-793`), not only when front is `Go`.
                    match self.creatures.get(cid).and_then(|k| k.base().next_wakeup) {
                        Some(wakeup) if wakeup > self.server_ms => {
                            self.cleanup();
                            return;
                        }
                        None => {
                            self.cleanup();
                            return;
                        }
                        _ => {}
                    }
                }
                self.run_monster_todo_execute(cid);
            }
            self.cleanup();
            return;
        }
        self.on_walk(cid, true, now, None);
        self.cleanup();
    }

    /// Schedule a creature wakeup in the logical ToDoQueue (`cract.cc:968` `ToDoStart`).
    /// Insert a creature wakeup into the global `ToDoQueue` at logical `execution_time`
    /// (`cract.cc:1021` `ToDoQueue.insert(NextWakeup, ID)`). Equal-key drain order is the
    /// structural heap order — **no** per-scenario tie; see `todo_queue.rs` / audit Finding 6.
    pub(crate) fn schedule_creature_wakeup(&mut self, cid: CreatureId, execution_time: u64) {
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().next_wakeup = Some(execution_time);
        }
        self.todo_queue.insert(execution_time, cid);
        trace_creature_todo(self, cid, "schedule_wakeup");
    }

    /// Compute walk delay for the next queued Go and arm the global heap (772 monster idle path).
    /// Returns `true` when the step should run immediately (1098 `getEventStepTicks` <= 1).
    ///
    /// 772 beat path mirrors `CalculateDelay` (`TDGo`) + `ToDoStart` (`cract.cc:918–923`, `1010–1023`):
    /// `Delay = EarliestWalkTime - ServerMilliseconds` when the cooldown is still active, else `0`;
    /// `ToDoStart` clamps `Delay < 1` to `1`, so a fresh walk from standstill arms at
    /// `ServerMilliseconds + 1` and lands on the next beat drain — **not** a full step duration.
    /// Subsequent steps wait out `EarliestWalkTime` set by `NotifyGo` (`on_walk`).
    pub(crate) fn todo_start_go_delay(&mut self, cid: CreatureId, first_step: bool) -> bool {
        // Phase 4: 1098 wall-clock `get_event_step_ticks` path deleted — both eras use the
        // 772 `CalculateDelay` + `ToDoStart` beat path.
        if !self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().walk_timer_idle())
        {
            return false;
        }
        let server_ms = self.server_ms;
        let earliest = self
            .creatures
            .get(cid)
            .map(|k| k.base().earliest_walk_server_ms)
            .unwrap_or(0);
        // C++ `CalculateDelay(TDGo)`: `Delay = EarliestWalkTime - ServerMilliseconds` only when
        // the cooldown is still active; otherwise `Delay` stays `0` and `ToDoStart` clamps it to
        // `1` (`cract.cc:918–923`, `:1016–1018`). Arming a full step duration here added up to
        // one extra step of input latency to every walk started from rest (audit #1).
        let calc_delay = if earliest > server_ms {
            earliest - server_ms
        } else {
            1
        };
        let delay = calc_delay.max(1);
        tracing::debug!(
            ?cid,
            first_step,
            earliest_walk_ms = earliest,
            server_ms,
            calc_delay,
            delay,
        );
        self.todo_start_from_action(cid, delay);
        false
    }

    /// O(1) reverse lookup via `creature_to_conn` (audit #4). Replaces the previous
    /// O(players) linear scan of `conn_to_creature`.
    pub(crate) fn conn_for_creature(&self, cid: CreatureId) -> Option<ConnId> {
        self.creature_to_conn.get(&cid).copied()
    }

    /// Send a deferred `0x6B` from [`Self::player_turn_request`], if any (`walk-smoothness-audit` Bug 7).
    pub fn flush_deferred_turn_broadcast(&mut self, cid: CreatureId) {
        let Some(data) = self.deferred_turn_broadcast.remove(&cid) else {
            return;
        };
        let DeferredTurnBroadcast {
            guid,
            pos,
            stack_u8,
            dir,
        } = data;
        // Grid-based fan-out (audit #4) — `spectator_conns_via_grid` already applies
        // `can_see_position`, so every conn here can see `pos`.
        let spectators: Vec<ConnId> = self.spectator_conns_via_grid(pos);
        for conn in spectators {
            let packet = self
                .codec
                .encode_creature_turn(guid, stack_u8, pos, dir as u8, false)
                .into_bytes();
            self.enqueue_outgoing(conn, packet);
        }
    }

    /// 772 `TCreature::ToDoClear` — clear the entire ToDo queue + walk queue + cancel wakeup.
    ///
    /// Returns `true` when there was a pending `Go` (walk in progress), in which case the caller
    /// must send `SendSnapback` (`0xB5`) — matching `if (ToDoClear()) SendSnapback;`
    /// (`receiving.cc:120-199`, `cract.cc:953-989`).
    ///
    /// Phase 1 walk-engine unification: replaces the old `clear_todo_772` which only stopped the
    /// event-walk timer. The unified path clears the ToDo action queue as well, so stale `Go`
    /// entries from a prior autowalk don't bleed into the new walk.
    pub(crate) fn player_todo_clear(&mut self, cid: CreatureId) -> bool {
        let had_pending_go = self.creatures.get(cid).is_some_and(|k| {
            let b = k.base();
            b.todo.has_go() || !b.walk_queue.is_empty()
        });
        self.stop_event_walk(cid);
        // C++ `ToDoClear` wipes **all** pending entries, including a queued `TDUse` / `TDMove`
        // from a prior walk-to-use (`cract.cc:953-989`). Rust's `walk_action` is the deferred
        // walk-to-act marker — clear it here so a new `CGoPath` / `CGoStop` / `CCancelAttack`
        // doesn't leave a stale Use/Move firing after an unrelated walk (audit #3).
        self.clear_player_walk_action(cid);
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            base.walk_queue.clear();
            base.walk_destinations.clear();
            base.todo.queue.clear();
            base.todo.locked = false;
            base.todo.todo_stop = false;
            base.next_wakeup = None;
            base.has_follow_path = false;
        }
        had_pending_go
    }

    /// 772 `ToDoClear` + `SendSnapback` — the `CGoDirection` / `CGoPath` preamble.
    ///
    /// C++ ref: `receiving.cc:120-199` (`if (ToDoClear()) SendSnapback;`),
    /// `cract.cc:953-989` (`ToDoClear` clears the whole queue).
    pub(crate) fn player_todo_clear_with_snapback(&mut self, conn_id: ConnId, cid: CreatureId) {
        // Phase 4: 1098 defer deleted — both eras use ToDoClear + SendSnapback.
        let had_pending = self.player_todo_clear(cid);
        if had_pending {
            let dir_byte = self
                .creatures
                .get(cid)
                .map(|k| k.base().direction as u8)
                .unwrap_or(0);
            self.enqueue_encoded(conn_id, self.codec.encode_cancel_walk(dir_byte));
        }
    }

    /// TFS `Game::playerMove` (`game.cpp` ~1880–1895).
    ///
    /// **772** (`receiving.cc:120-199` `CGoDirection`, `cract.cc:1050-1107` `ToDoGo`/`ToDoStart`):
    /// `ToDoClear()` → `SendSnapback` if pending → `TDGo(dir)` → `ToDoStart()`. Every new move
    /// clears pending execution and reschedules from scratch via the unified ToDo engine.
    pub fn player_move_request(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        direction: Direction,
        now: Instant,
    ) {
        // `tasks/walk-direction-change-audit.md`: flush pending `0x6B` before move — do not drop it while
        // the client already applied the turn locally (cancel caused facing desync).
        self.flush_deferred_turn_broadcast(cid);
        // TFS `Game::playerMove` clears pending walk-action (`game.cpp` ~1893).
        self.clear_player_walk_action(cid);
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };
        if p.base.movement_blocked {
            self.enqueue_outgoing(
                conn_id,
                self.codec
                    .encode_cancel_walk(p.base.direction as u8)
                    .into_bytes(),
            );
            return;
        }

        // 772 unified ToDo path — `CGoDirection` (`receiving.cc:120-199`).
        self.player_todo_clear_with_snapback(conn_id, cid);
        let cur_pos = self.creatures.get(cid).map(|k| k.position());
        if let (Some(CreatureKind::Player(pl)), Some(pos)) = (self.creatures.get_mut(cid), cur_pos)
        {
            pl.last_activity = now;
            pl.base.walk_queue.push_back(direction);
            // C++ `TDGo` stores absolute coordinates (`receiving.cc:189-192`); Rust stores
            // `Direction`s. Track the absolute destination alongside so `on_walk` can verify
            // adjacency after a mid-walk push (audit #4 — `cract.cc:386-389`).
            pl.base.walk_destinations.push_back(pos.offset(direction));
        }
        // `ToDoGo` → `ToDoStart` (`cract.cc:1050-1107`, `:991-1024`).
        let _ = self.enqueue_creature_go(cid);
        if self.todo_start_go_delay(cid, true) {
            self.schedule_immediate_todo_wakeup(cid);
        }
    }

    /// TFS `Game::playerAutoWalk` (`game.cpp` ~2075–2084).
    ///
    /// **772** (`receiving.cc:120-199` `CGoPath`, `cract.cc:1050-1107`): `ToDoClear()` →
    /// `SendSnapback` if pending → enqueue N `TDGo` entries (one per step) → single `ToDoStart()`.
    /// Same clear-and-restart pattern as `playerMove`, via the unified ToDo engine.
    pub fn player_auto_walk_path(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        path: Vec<Direction>,
        now: Instant,
    ) {
        self.flush_deferred_turn_broadcast(cid);
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };
        if p.base.movement_blocked {
            self.enqueue_outgoing(
                conn_id,
                self.codec
                    .encode_cancel_walk(p.base.direction as u8)
                    .into_bytes(),
            );
            return;
        }

        // 772 unified ToDo path — `CGoPath` (`receiving.cc:120-199`).
        self.player_todo_clear_with_snapback(conn_id, cid);
        let cur_pos = self.creatures.get(cid).map(|k| k.position());
        if let (Some(CreatureKind::Player(pl)), Some(pos)) = (self.creatures.get_mut(cid), cur_pos)
        {
            pl.last_activity = now;
            // C++ `CGoPath` accumulates absolute coordinates from `Player->posx/y/z`
            // (`receiving.cc:141-160`); Rust stores `Direction`s. Track the absolute
            // destination of each step alongside so `on_walk` can verify adjacency after a
            // mid-walk push (audit #4 — `cract.cc:386-389`).
            //
            // `path` is in reverse execution order (packet parser `.rev()`), and
            // `walk_queue` uses `push_back` + `pop_back` (LIFO), so `pop_back` yields the
            // first-to-execute step. Accumulate destinations in execution order (rev of
            // `path`) and `push_front` so `pop_back` on both queues stays in sync.
            for d in &path {
                pl.base.walk_queue.push_back(*d);
            }
            let mut acc = pos;
            for d in path.iter().rev() {
                acc = acc.offset(*d);
                pl.base.walk_destinations.push_front(acc);
            }
        }
        // `CGoPath` builds N entries then a single `ToDoStart` — one `Go` action drains the
        // whole `walk_queue` via `finish_creature_todo_execute` re-arm (`cract.cc:1050-1107`).
        let _go_enqueued = self.enqueue_creature_go(cid);
        let immediate = self.todo_start_go_delay(cid, true);
        // OTClient auto-walk workaround: record the beat when this walk was armed.
        // OTClient sends `0x69` (StopAutoWalk) 2–200 ms after each `0x64` (AutoWalk) on
        // map-click; the stop is meant for the *previous* walk, not the fresh one.
        // `player_stop_auto_walk` ignores stops that arrive within 400 ms of a fresh arm.
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().last_auto_walk_armed_ms = self.server_ms;
        }
        if immediate {
            self.schedule_immediate_todo_wakeup(cid);
        }
    }

    /// TFS `Game::playerTurn` + `internalCreatureTurn` (`game.cpp` ~3354–3366, ~3703–3720).
    /// OTClient sends `0x6F–0x72` for in-place turns; ignoring them left server facing out of sync with
    /// the client during sharp direction changes (Move + Turn ordering).
    ///
    /// `tasks/walk-smoothness-audit.md` Bug 7: `Map::moveCreature`-style facing from the next step can
    /// overwrite `direction` immediately after a turn. We defer `0x6B` when standing so the game loop
    /// can drop it if `Move`/`AutoWalk` is next on the wire; we skip deferring when a walk is already
    /// queued (the next step sets facing).
    pub fn player_turn_request(&mut self, cid: CreatureId, dir: Direction, now: Instant) {
        let (already, guid, pos) = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => (p.base.direction == dir, p.guid, p.base.position),
            _ => return,
        };
        if already {
            self.flush_deferred_turn_broadcast(cid);
            return;
        }

        self.flush_deferred_turn_broadcast(cid);

        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.base.direction = dir;
            p.last_activity = now;
        }

        if self.creatures.get(cid).is_some_and(|k| match k {
            CreatureKind::Player(p) => !p.base.walk_queue.is_empty(),
            _ => false,
        }) {
            return;
        }

        let stack_u8 = self
            .map
            .get_tile(pos)
            .map(|t| {
                let raw = client_creature_stack_pos(t.body(), cid);
                if !(0..10).contains(&raw) {
                    10u8
                } else {
                    raw as u8
                }
            })
            .unwrap_or(10);

        self.deferred_turn_broadcast.insert(
            cid,
            DeferredTurnBroadcast {
                guid,
                pos,
                stack_u8,
                dir,
            },
        );
    }

    /// TFS `Player::stopWalk` (`player.cpp` ~3398); 772 `CGoStop` → `ToDoStop`
    /// (`receiving.cc:201-211`, `cract.cc:1002-1008`).
    ///
    /// 772 `ToDoStop` (`cract.cc:1002-1008`):
    /// - **Locked** (walk in progress, wakeup armed): set `Stop = true` — the in-flight step
    ///   lands on the next beat, then `Execute` checks `Stop` and does `ToDoClear + SendSnapback`
    ///   (`cract.cc:891-897`, `:797-801`). The client always gets a snapback.
    /// - **Not locked** (no walk): immediate `SendSnapback` (queue is empty, nothing to clear).
    ///
    /// 1098 sets `cancel_next_walk` which is processed in `onWalk`.
    pub fn player_stop_auto_walk(&mut self, cid: CreatureId) {
        // OTClient auto-walk workaround: OTClient sends `0x69` (StopAutoWalk) 2–200 ms
        // after each `0x64` (AutoWalk) on map-click. The stop is meant for the *previous*
        // walk, not the fresh one — `player_auto_walk_path` already cleared the previous
        // walk via `player_todo_clear_with_snapback`. If the new walk was armed within
        // a short window, ignore the stop entirely.
        let last_armed = self
            .creatures
            .get(cid)
            .map(|k| k.base().last_auto_walk_armed_ms)
            .unwrap_or(u64::MAX);
        if last_armed != u64::MAX {
            let since_armed = self.server_ms.saturating_sub(last_armed);
            if since_armed <= 400 {
                return;
            }
        }

        // C++ `LockToDo` is true from `ToDoStart` until `ToDoClear` (`cract.cc:1010-1012`).
        // Rust mirrors that with `todo.locked` for the whole batch (plus wakeup / Go /
        // walk_queue as belt-and-suspenders).
        let walk_in_progress = self.creatures.get(cid).is_some_and(|k| {
            let b = k.base();
            b.todo.locked || b.next_wakeup.is_some() || b.todo.has_go() || !b.walk_queue.is_empty()
        });
        if walk_in_progress {
            // C++ `ToDoStop` locked branch: `this->Stop = true` (`cract.cc:1003-1004`).
            // The in-flight step lands on the next beat; `finish_creature_todo_execute`
            // checks `todo_stop` and does `ToDoClear + SendSnapback` (`cract.cc:891-897`).
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().todo.todo_stop = true;
            }
        } else {
            // C++ `ToDoStop` not-locked branch: immediate `SendSnapback` (`cract.cc:1005-1006`).
            // Queue is empty; `player_todo_clear` is a harmless no-op that also resets flags.
            if let Some(conn) = self.conn_for_creature(cid) {
                let dir_byte = self
                    .creatures
                    .get(cid)
                    .map(|k| k.base().direction as u8)
                    .unwrap_or(0);
                self.enqueue_encoded(conn, self.codec.encode_cancel_walk(dir_byte));
            }
            self.player_todo_clear(cid);
        }
    }

    /// 1098 self-move (`ProtocolGame::sendMoveCreature`, `creature == player`, non-teleport).
    /// 772 self-moves use [`Self::emit_notify_go`] instead.
    fn emit_move_packet(
        &mut self,
        cid: CreatureId,
        conn_id: ConnId,
        old_pos: Position,
        new_pos: Position,
        old_stack: i32,
    ) {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };
        let with_description = p.item_with_description();
        let guid = p.guid;

        let mut known = self
            .known_creatures_by_conn
            .remove(&conn_id)
            .unwrap_or_default();
        self.reconcile_known_creatures_for_send(conn_id, &mut known);
        let packet = {
            let mut get_tile = |tx: i32, ty: i32, tz: i32| -> Option<TileContent> {
                map_tile_content(self, cid, new_pos, tx, ty, tz)
            };
            let mut can_see = |id: u32| self.can_see_creature_for_known_set(cid, id);
            send_move_creature_player(
                &self.codec,
                old_pos,
                new_pos,
                old_stack,
                guid,
                &mut get_tile,
                &mut known,
                &mut can_see,
                with_description,
            )
            .into_bytes()
        };
        self.commit_known_creatures_after_send(conn_id, &known);
        self.enqueue_outgoing(conn_id, packet);
    }

    /// 772 self-move — decompile `TCreature::NotifyGo` (`cract.cc:1400-1465`).
    ///
    /// Emits a single packet for the **overall** `old_pos → new_pos` move (never per segment):
    /// no `0x6D`/`0x6C` self-packet; adjacent moves stream `SendFloors`/`SendRow`, non-adjacent
    /// moves use `SendFullScreen` (0x64). Fixes the combined diagonal+z stair desync (§16.3):
    /// walking perpendicular onto a stair (e.g. west onto south-facing stairs) leaves a leftover
    /// delta on both axes, which per-segment emission cannot encode as a valid row sequence.
    fn emit_notify_go(
        &mut self,
        cid: CreatureId,
        conn_id: ConnId,
        old_pos: Position,
        new_pos: Position,
        old_stack: i32,
    ) {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };
        let with_description = p.item_with_description();
        let guid = p.guid;

        let mut known = self
            .known_creatures_by_conn
            .remove(&conn_id)
            .unwrap_or_default();
        self.reconcile_known_creatures_for_send(conn_id, &mut known);
        let packet = {
            let mut get_tile = |tx: i32, ty: i32, tz: i32| -> Option<TileContent> {
                map_tile_content(self, cid, new_pos, tx, ty, tz)
            };
            let mut can_see = |id: u32| self.can_see_creature_for_known_set(cid, id);
            send_notify_go(
                &self.codec,
                old_pos,
                new_pos,
                old_stack,
                guid,
                &mut get_tile,
                &mut known,
                &mut can_see,
                with_description,
            )
            .into_bytes()
        };
        self.commit_known_creatures_after_send(conn_id, &known);
        self.enqueue_outgoing(conn_id, packet);
    }

    /// C++ `sendCreatureMove` teleport path: `sendRemoveTileCreature` + `sendMapDescription`.
    /// Used for queryDestination chain steps where `areInRange` fails (>1 tile or, for 1098, any z-change).
    ///
    /// TVP (`protocolgame.cpp:1768-1790`): for self-teleport, sends `sendRemoveTileCreature`
    /// UNLESS `newPos.z == 8 && oldPos.z == 7` (surface→underground skips remove), then
    /// `sendMapDescription(newPos)` (0x64). Both eras use the same logic. §6 experiment
    /// confirmed the self-packet and remove are required for both clients.
    fn emit_teleport_move_packet(
        &mut self,
        cid: CreatureId,
        conn_id: ConnId,
        old_pos: Position,
        new_pos: Position,
        old_stack: i32,
    ) {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };
        let with_description = p.item_with_description();

        // 1) sendRemoveTileCreature — TVP: skip when leaving surface (oldPos.z == 7)
        // in either direction OR going down to underground (newPos.z == 8).
        // `protocolgame.cpp:1770`: `if (newPos.z != 8 && oldPos.z != 7)` — send remove
        // only when BOTH conditions hold; skip when EITHER is false.
        let skip_remove = old_pos.z == 7 || new_pos.z == 8;
        if !skip_remove {
            let remove_pkt = if (0..10).contains(&old_stack) {
                self.codec
                    .encode_remove_tile_thing(old_pos, old_stack as u8)
                    .into_bytes()
            } else {
                self.codec
                    .encode_remove_tile_creature_by_id(p.guid)
                    .into_bytes()
            };
            self.enqueue_outgoing(conn_id, remove_pkt);
        }

        // 2) sendMapDescription(newPos)
        let mut known = self
            .known_creatures_by_conn
            .remove(&conn_id)
            .unwrap_or_default();
        self.reconcile_known_creatures_for_send(conn_id, &mut known);
        let map_pkt = {
            let mut get_tile = |tx: i32, ty: i32, tz: i32| -> Option<TileContent> {
                map_tile_content(self, cid, new_pos, tx, ty, tz)
            };
            let mut can_see = |id: u32| self.can_see_creature_for_known_set(cid, id);
            send_map_description_packet(
                &self.codec,
                new_pos,
                new_pos,
                &mut get_tile,
                &mut known,
                &mut can_see,
                with_description,
            )
            .into_bytes()
        };
        self.commit_known_creatures_after_send(conn_id, &known);
        self.enqueue_outgoing(conn_id, map_pkt);
    }

    /// C++ `::Move` → `NotifyTurn` + `NotifyGo` (`operate.cc:1407–1431`, `cract.cc:1400–1564`).
    ///
    /// Shared by normal `on_walk` steps and `KickCreature` forced relocate. Locks
    /// `EarliestWalkTime` from the **destination** tile's BANK WAYPOINTS (`NotifyGo`).
    /// When `apply_notify_turn` is true (kick path), also sets facing (`NotifyTurn`).
    /// Walk steps pass `false` — direction was already set in `internal_move_creature_step`
    /// (including post-`queryDestination` chain turns) and must not be overwritten.
    pub(crate) fn apply_notify_go_after_relocate(
        &mut self,
        cid: CreatureId,
        old_pos: Position,
        new_pos: Position,
        step_dir: Direction,
        apply_notify_turn: bool,
    ) {
        // `NotifyTurn` — state only; C++ does not broadcast `0x6B` here (`cract.cc:1566–1581`).
        if apply_notify_turn {
            if let Some(k) = self.creatures.get_mut(cid) {
                set_direction_from_step(old_pos, new_pos, k);
            }
        }

        let gs_dest = self
            .map
            .get_tile(new_pos)
            .map(|t| ground_speed_for_tile_body(t.body(), self.items_db.as_ref()))
            .unwrap_or(150);
        let notify_go_ms = self
            .creatures
            .get(cid)
            .map(|k| {
                get_step_duration_ms_with_direction(
                    k,
                    k.base(),
                    step_dir,
                    gs_dest,
                    &self.mechanics,
                )
            })
            .unwrap_or(0);
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            base.last_step = Some(Instant::now());
            base.last_step_cost = last_step_cost_for_move(old_pos, new_pos);
            base.last_step_ground_speed = gs_dest;
            base.last_step_server_ms = Some(self.server_ms);
            if notify_go_ms > 0 {
                let new_earliest = self.server_ms.saturating_add(notify_go_ms.max(1) as u64);
                base.earliest_walk_server_ms = new_earliest;
            }
        }
    }

    /// After KickCreature `NotifyGo`, push any armed wakeup out to `EarliestWalkTime`.
    ///
    /// A pre-kick `next_wakeup` can fire a `Go` before the client finishes the push
    /// `0x6D` animation (OTC `getStepDuration` from dest ground speed). Premature steps
    /// look like dashes/skips. C++ `CalculateDelay(TDGo)` waits on `EarliestWalkTime`
    /// (`cract.cc:918–923`); this keeps the heap in sync when kick extends that time.
    pub(crate) fn reschedule_wakeup_for_earliest_walk(&mut self, cid: CreatureId) {
        let earliest = self
            .creatures
            .get(cid)
            .map(|k| k.base().earliest_walk_server_ms)
            .unwrap_or(0);
        if earliest <= self.server_ms {
            return;
        }
        let armed = self
            .creatures
            .get(cid)
            .and_then(|k| k.base().next_wakeup)
            .unwrap_or(0);
        // Only bump when a wakeup is already armed for sooner than the new walk lock,
        // or when the creature still has walk/todo work that will need a wake.
        let has_work = self.creatures.get(cid).is_some_and(|k| {
            let b = k.base();
            !b.walk_queue.is_empty() || !b.todo.is_empty()
        });
        if !has_work && armed == 0 {
            return;
        }
        if armed != 0 && armed >= earliest {
            return;
        }
        self.schedule_creature_wakeup(cid, earliest);
    }

    /// `ProtocolGame::sendMoveCreature` for other clients (`protocolgame.cpp` ~2872–2893).
    pub(crate) fn broadcast_spectator_move(
        &mut self,
        mover: CreatureId,
        old_pos: Position,
        new_pos: Position,
        old_creatures: &[CreatureId],
    ) {
        let wire_id = match self.creatures.get(mover) {
            Some(k) => creature_wire_id(mover, k),
            None => return,
        };

        // C++ spectator branch: remove+add on teleport or surface→underground (7→8+).
        let surface_to_underground = old_pos.z == 7 && new_pos.z >= 8;
        let z_changed = old_pos.z != new_pos.z;

        // Grid-based fan-out (audit #4): collect spectators from both old and new
        // position viewports, union + dedup, then apply per-viewer can_see checks.
        // C++ `Map::getSpectators` collects the union of old+new spectator sets
        // (`map.cpp` ~264–323).
        let mut spectator_conns: Vec<ConnId> = self.spectator_conns_via_grid(old_pos);
        spectator_conns.extend(self.spectator_conns_via_grid(new_pos));
        spectator_conns.sort_by_key(|c| c.0);
        spectator_conns.dedup();
        let spectators: Vec<(ConnId, CreatureId)> = spectator_conns
            .into_iter()
            .filter_map(|conn| {
                let viewer = *self.conn_to_creature.get(&conn)?;
                if viewer == mover {
                    return None;
                }
                Some((conn, viewer))
            })
            .collect();

        // C++ `Map::moveCreature` captures per-viewer `oldStackPos` BEFORE removing the
        // creature from the old tile (`map.cpp:292-301`), and `Tile::getClientIndexOfCreature`
        // only counts creatures the viewer can see (`tile.cpp:1207-1214`). The ground and
        // top_items counts don't change during a creature move, so we read them from the old
        // tile after the move (the creature has been removed, but ground/top_items are intact).
        //
        // 772 decompile `GetObjectRNum` (`info.cc:205`) counts the full map-container chain
        // (ground → down → top → creatures). TVP/OTC `getClientIndexOfCreature` skips down
        // items (they are emitted *after* creatures in `GetTileDescription`). Use RNum for
        // real-772 viewers so KickCreature `0x6D` hits the correct stack; OTC keeps TVP math.
        let (ground_present, down_item_count, top_item_count) = self
            .map
            .get_tile(old_pos)
            .map(|t| {
                (
                    t.body().ground.is_some(),
                    t.body().down_items.len(),
                    t.body().top_items.len(),
                )
            })
            .unwrap_or((true, 0, 0));

        // First pass: compute per-viewer data using only `&self` borrows.
        // C++ `map.cpp:295` — `tmpPlayer->canSeeCreature(&creature)` gates the entire
        // packet: viewers who can't see the moving creature (invisible/ghost) get no
        // move packet at all (stackpos = -1).
        let viewer_data: Vec<(ConnId, CreatureId, i32, bool, bool)> = spectators
            .into_iter()
            .filter_map(|(conn, viewer)| {
                if !self.can_see_creature(viewer, mover) {
                    return None;
                }
                let otc = self
                    .creatures
                    .get(viewer)
                    .is_some_and(|k| matches!(k, CreatureKind::Player(p) if p.is_otclient()));
                let downs = if otc { 0 } else { down_item_count };
                let viewer_stack = creature_stack_pos_for_viewer(
                    ground_present,
                    downs + top_item_count,
                    old_creatures,
                    mover,
                    |c| self.can_see_creature(viewer, c),
                );
                let can_see_old = self.can_see_position(viewer, old_pos);
                let can_see_new = self.can_see_position(viewer, new_pos);
                Some((conn, viewer, viewer_stack, can_see_old, can_see_new))
            })
            .collect();

        // Second pass: send packets (`&mut self` borrows).
        for (conn, viewer, viewer_stack, can_see_old, can_see_new) in viewer_data {
            if can_see_old && can_see_new {
                // Surface→underground still needs remove+appear (TVP `protocolgame.cpp:1831`).
                // For same-viewport adjacent moves — including KickCreature — ALWAYS send
                // `0x6D`. TVP's `oldStackPos >= 10` branch used remove+`sendAddCreature`,
                // which has **no** OTClient `allowAppearWalk` → creature teleports (looks
                // like a skip). Prefer `0x6D` with `0xFFFF+creature_id` fallback when the
                // stack index is out of `0..9` (`send_move_creature_spectator`); that path
                // still runs `allowAppearWalk` and uses the dest-tile ground-speed formula.
                if z_changed && surface_to_underground {
                    self.send_creature_remove_to_conn(conn, mover, old_pos, viewer_stack);
                    self.send_creature_appear_to_conn(conn, viewer, mover, new_pos);
                } else {
                    let pkt = send_move_creature_spectator(
                        &self.codec,
                        old_pos,
                        new_pos,
                        viewer_stack,
                        wire_id,
                    )
                    .map(|m| m.into_bytes());
                    if let Some(pkt) = pkt {
                        self.enqueue_outgoing(conn, pkt);
                    }
                }
            } else if can_see_old {
                self.send_creature_remove_to_conn(conn, mover, old_pos, viewer_stack);
            } else if can_see_new {
                self.send_creature_appear_to_conn(conn, viewer, mover, new_pos);
            }
        }
    }

    /// After synchronous `checkCreatureWalk` when `addEventWalk`'s initial `ticks == 1` and
    /// `first_step` is true — set `next_walk_check` from **post-`on_walk`** timing (`getEventStepTicks(false)`).
    ///
    /// C++ uses the **same** pre-sync `ticks` for `scheduler.addEvent(ticks)` (`creature.cpp` ~311–321),
    /// which is always `1` on that branch and adds an extra 1ms poll before the real walk delay elapses
    /// (`tasks/walk-smoothness-audit` Bug 1 / 8). Recomputing after `last_step` is set tightens rhythm.
    /// C++ `addEventWalk()` after the sync `ticks == 1` path — delay is from **now** when the callback runs
    /// (`creature.cpp`), not from the pre-`on_walk` logical instant.
    fn schedule_walk_followup_deadline(&mut self, cid: CreatureId) {
        let wall_now = Instant::now();
        let (pos, timing_speed) = {
            let Some(k) = self.creatures.get(cid) else {
                return;
            };
            (
                k.position(),
                walk_timing_speed_kind(k, k.base(), &self.mechanics),
            )
        };
        if timing_speed <= 0 {
            return;
        }
        if self
            .creatures
            .get(cid)
            .is_some_and(|k| !k.base().walk_timer_idle())
        {
            return;
        }
        let ground_speed = self
            .map
            .get_tile(pos)
            .map(|t| ground_speed_for_tile_body(t.body(), self.items_db.as_ref()))
            .unwrap_or(150);
        let server_ms_opt = Some(self.server_ms);
        let ticks = {
            let Some(k) = self.creatures.get(cid) else {
                return;
            };
            get_event_step_ticks(
                k,
                k.base(),
                false,
                ground_speed,
                peek_next_walk_direction(k.base()),
                wall_now,
                &self.mechanics,
                server_ms_opt,
            )
        };
        if ticks <= 0 {
            return;
        }
        let delay_ms = ticks.max(1) as u64;
        self.schedule_creature_wakeup(cid, self.server_ms.saturating_add(delay_ms));
    }

    /// Queue one step and arm the walk timer (monster/NPC AI and tests).
    // Parity helper for monster/NPC AI; currently exercised by tests. Retained ahead of caller.
    #[allow(dead_code)]
    pub(crate) fn creature_queue_walk_step(&mut self, cid: CreatureId, direction: Direction) {
        if let Some(k) = self.creatures.get_mut(cid) {
            let pos = k.base().position;
            let base = k.base_mut();
            base.walk_queue.clear();
            base.walk_destinations.clear();
            base.walk_queue.push_back(direction);
            base.walk_destinations.push_back(pos.offset(direction));
        }
        self.add_event_walk(cid, true);
    }

    /// TFS `Creature::startAutoWalk` + `addEventWalk` — all creature kinds (`creature.cpp` ~274–297).
    pub(crate) fn creature_start_auto_walk(&mut self, cid: CreatureId) {
        let is_772 = matches!(self.codec, tfs_rust_net::codec::Codec::V772(_));
        let first_only = is_772
            || self
                .creatures
                .get(cid)
                .is_some_and(|k| k.base().walk_queue.len() == 1);
        self.add_event_walk(cid, first_only);
    }

    /// TFS `Creature::addEventWalk` (`creature.cpp` ~299–322).
    ///
    /// Phase 5: the 1098 `scheduling_base` anchor (for the `Instant`-based initial timer) is gone;
    /// both eras schedule via `schedule_creature_wakeup` on the logical `server_ms` clock.
    fn add_event_walk(&mut self, cid: CreatureId, first_step: bool) {
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().cancel_next_walk = false;
        }
        let (pos, timing_speed) = {
            let Some(k) = self.creatures.get(cid) else {
                return;
            };
            (
                k.position(),
                walk_timing_speed_kind(k, k.base(), &self.mechanics),
            )
        };
        if timing_speed <= 0 {
            return;
        }
        if self
            .creatures
            .get(cid)
            .is_some_and(|k| !k.base().walk_timer_idle())
        {
            return;
        }

        let wall_now = Instant::now();
        let server_ms_opt = Some(self.server_ms);

        let ground_speed = self
            .map
            .get_tile(pos)
            .map(|t| ground_speed_for_tile_body(t.body(), self.items_db.as_ref()))
            .unwrap_or(150);

        let ticks = {
            let Some(k) = self.creatures.get(cid) else {
                return;
            };
            get_event_step_ticks(
                k,
                k.base(),
                first_step,
                ground_speed,
                peek_next_walk_direction(k.base()),
                wall_now,
                &self.mechanics,
                server_ms_opt,
            )
        };

        if ticks <= 0 {
            return;
        }

        if ticks == 1 {
            self.check_creature_walk_from_add_event_walk(cid, wall_now);
            if first_step {
                self.schedule_walk_followup_deadline(cid);
            } else {
                self.schedule_creature_wakeup(cid, self.server_ms.saturating_add(1));
            }
            return;
        }

        let delay_ms = ticks.max(1) as u64;
        let execution_time = self.server_ms.saturating_add(delay_ms);
        self.schedule_creature_wakeup(cid, execution_time);
    }

    pub(crate) fn stop_event_walk(&mut self, cid: CreatureId) {
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().next_wakeup = None;
        }
    }

    /// Same as the old `check_creature_walk`, but the walk was **not** triggered by a prior
    /// `next_walk_check` (sync branch inside `add_event_walk` when `ticks == 1`). Matches
    /// `eventWalk == 0` at `onWalk` exit in C++.
    ///
    /// Phase 5: the 1098 `check_creature_walk` (wake from `next_walk_check` deadline) is deleted —
    /// both eras schedule steps via the ToDo queue. This sync entry point remains for the
    /// `ticks == 1` fast path inside `add_event_walk`.
    fn check_creature_walk_from_add_event_walk(&mut self, cid: CreatureId, now: Instant) {
        let health_ok = self.creatures.get(cid).is_some_and(|k| k.base().health > 0);
        if !health_ok {
            return;
        }

        self.on_walk(cid, false, now, None);
        self.cleanup();
    }

    /// 772 `Execute` catch — `cract.cc:870-889`: on a rejected step (`NOTACCESSIBLE` /
    /// `MOVENOTPOSSIBLE` / etc.), send `SendResult` + `SendSnapback` (player), clear the
    /// ToDo/walk queue (`ToDoClear`), and request an idle stimulus (`ToDoYield`).
    ///
    /// Shared by the adjacency-abort path (audit #4 — `cract.cc:386-389` `NOTACCESSIBLE`)
    /// and the `internal_move_creature_step` `Err` path.
    ///
    /// **Snapback parity note (S3):** C++ splits the snapback across `SendResult` (sends
    /// it unconditionally for `MOVENOTPOSSIBLE`, `sending.cc:353-355`) and the `Execute`
    /// catch's explicit snapback (gated on `SnapbackNecessary` for non-exempt results like
    /// `NOTACCESSIBLE`). Both `NOTACCESSIBLE` and `MOVENOTPOSSIBLE` map to
    /// `ReturnValue::NotPossible` here, so they can't be distinguished. Always sending the
    /// snapback is the safer divergence: an extra snapback for `NOTACCESSIBLE` without
    /// remaining Gos is harmless (client resyncs to same position), while gating on
    /// `SnapbackNecessary` would risk **missing** the unconditional snapback for
    /// `MOVENOTPOSSIBLE` (client desync). The extra-snapback case only arises on a
    /// mid-walk push (adjacency fail) with no remaining steps — rare and harmless.
    fn on_walk_step_rejected(&mut self, cid: CreatureId, ret: ReturnValue) {
        if let Some(conn) = self.conn_for_creature(cid) {
            let d = self
                .creatures
                .get(cid)
                .map(|k| k.base().direction)
                .unwrap_or(Direction::North);
            let msg = ret.description();
            self.enqueue_outgoing(
                conn,
                send_text_message_simple(self.codec.failure_message_type(), msg).into_bytes(),
            );
            self.enqueue_outgoing(conn, self.codec.encode_cancel_walk(d as u8).into_bytes());
        }
        // TFS `Creature::onWalk` — `listWalkDir` is **not** cleared on failed move; step was already
        // popped in `getNextStep` (`src/creature.cpp` ~205–213).
        // 772: `ToDoClear` + `ToDoYield` on blocked step (`cract.cc:870-889`).
        // Phase 1.3: widened from monster-only to all creatures on the unified
        // ToDo path (players + monsters). `ToDoClear` is unconditional — clears
        // the whole queue regardless of attack state (`cract.cc:871`).
        if self.creature_uses_todo_execute(cid) {
            // C++ `Execute` catch: `SnapbackNecessary = ToDoClear() || Stop`
            // (`cract.cc:871`). The snapback above already fired for players.
            // If `todo_stop` was set (player `CGoStop` while locked), the stop is
            // now satisfied — clear the flag and skip idle stimulus (`cract.cc:891-897`).
            let was_stop_requested = self
                .creatures
                .get(cid)
                .is_some_and(|k| k.base().todo.todo_stop);
            let is_monster = self
                .creatures
                .get(cid)
                .is_some_and(|k| matches!(k, CreatureKind::Monster(_)));
            if let Some(k) = self.creatures.get_mut(cid) {
                let base = k.base_mut();
                base.walk_queue.clear();
                base.walk_destinations.clear();
                base.has_follow_path = false;
                // `force_update_follow_path` is a **monster chase-repath** flag
                // (`crnonpl.cc` `IdleStimulus` repath). C++ `Execute` catch
                // (`cract.cc:870-889`) only calls `ToDoClear + ToDoYield` — it does
                // NOT set any follow-path flag. Setting it for players strands them:
                // `finish_creature_todo_execute` clears `walk_queue` on every
                // subsequent step, and `monster_idle_stimulus` (the only clearer)
                // is a no-op for players. Only set it for monsters that may need
                // to repath toward a follow/attack target.
                if is_monster {
                    base.force_update_follow_path = true;
                }
                base.todo.queue.clear();
                base.todo.locked = false;
                base.todo.todo_stop = false;
            }
            if !was_stop_requested {
                self.request_idle_stimulus(cid);
            }
        } else if let Some(k) = self.creatures.get_mut(cid) {
            // 1098: TFS keeps `listWalkDir` and sets `forceUpdateFollowPath` only
            // when following (`creature.cpp` ~213).
            if k.base().follow_target.is_some() {
                k.base_mut().force_update_follow_path = true;
            }
        }
    }

    /// TFS `Creature::onWalk` (`creature.cpp` ~200–234).
    /// `reschedule_after` = C++ `eventWalk != 0` before the end block — only then does `onWalk` call `addEventWalk()`.
    ///
    /// `fired_deadline`: logical `next_walk_check` that triggered this `on_walk` (scheduler path); used to
    /// chain the next deadline without cumulative timer jitter.
    pub(crate) fn on_walk(
        &mut self,
        cid: CreatureId,
        reschedule_after: bool,
        now: Instant,
        _fired_deadline: Option<Instant>,
    ) {
        let walk_delay = self
            .creatures
            .get(cid)
            .map(|k| {
                // Phase 4: 1098 `get_walk_delay` path deleted — both eras use `EarliestWalkTime`.
                // C++ has a single source of truth: `EarliestWalkTime`, fixed by `NotifyGo`
                // at step-completion and consumed by `CalculateDelay` (`cract.cc:918-923`,
                // `:1515-1525`). Derive the gate from `earliest_walk_server_ms` directly
                // instead of recomputing `completed_step_duration_ms` from **current**
                // speed/conditions (audit #5/#6 — the recomputation applied `last_step_cost
                // = 2` on z-change and re-read speed, both diverging from C++ which fixes
                // the delay at step-completion time).
                let d = k
                    .base()
                    .earliest_walk_server_ms
                    .saturating_sub(self.server_ms) as i64;
                tracing::debug!(
                    ?cid,
                    earliest_walk_ms = k.base().earliest_walk_server_ms,
                    server_ms = self.server_ms,
                    walk_delay = d,
                    queue_len = k.base().walk_queue.len(),
                );
                d
            })
            .unwrap_or(0);

        let mut stopped_without_reschedule = false;

        if walk_delay <= 0 {
            // Pop the next step direction. For both players and monsters, also pop the
            // parallel `walk_destinations` entry (absolute destination of this step) so the
            // adjacency check below can verify it against the current position (audit #4,
            // `cract.cc:386-389`). Monsters now track absolute destinations too — matching
            // C++ TDGo absolute coordinates — so a mid-step kick displacement is detected
            // on the next `on_walk` and triggers `on_walk_step_rejected` (the decompile's
            // `Go` → `NOTACCESSIBLE` → `ToDoClear + ToDoYield` path, `cract.cc:870-877`).
            let (pop_dir, pop_dest) = if self
                .creatures
                .get(cid)
                .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
            {
                if self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| !k.base().walk_queue.is_empty())
                {
                    let mut dest = None;
                    let dir = self.creatures.get_mut(cid).and_then(|k| {
                        let base = k.base_mut();
                        let d = base.walk_queue.pop_back();
                        if d.is_some() {
                            dest = base.walk_destinations.pop_back();
                        }
                        d
                    });
                    (dir, dest)
                } else {
                    (self.monster_next_walk_step(cid, now), None)
                }
            } else {
                let mut dest = None;
                let dir = self.creatures.get_mut(cid).and_then(|k| {
                    let base = k.base_mut();
                    let d = base.walk_queue.pop_back();
                    // Phase 4: both eras pop walk_destinations (772 absolute-destination tracking).
                    if d.is_some() {
                        dest = base.walk_destinations.pop_back();
                    }
                    d
                });
                (dir, dest)
            };

            if let Some(mut dir) = pop_dir {
                tracing::debug!(?cid, ?dir, ?pop_dest, server_ms = self.server_ms,);
                // 772 absolute-destination adjacency check — `cract.cc:386-389`:
                // `Distance = max(abs(OrigX - DestX), abs(OrigY - DestY)); if(Distance > 1 || OrigZ != DestZ) throw NOTACCESSIBLE`.
                // C++ `TDGo` stores absolute coordinates; if the player was pushed mid-walk the
                // stored dest is no longer adjacent → `Execute` catch (`cract.cc:870-889`):
                // `SendResult("Sorry, not possible.")` + `SendSnapback` + `ToDoClear` + `ToDoYield`
                // (audit #4). The check runs before drunk stagger, matching C++ order.
                if let Some(dest) = pop_dest {
                    if let Some(cur_pos) = self.creatures.get(cid).map(|k| k.position()) {
                        let dx = (cur_pos.x as i32 - dest.x as i32).unsigned_abs();
                        let dy = (cur_pos.y as i32 - dest.y as i32).unsigned_abs();
                        if dx > 1 || dy > 1 || cur_pos.z != dest.z {
                            self.on_walk_step_rejected(cid, ReturnValue::NotPossible);
                            return;
                        }
                        // 772 `Go(DestX, DestY, DestZ)` moves to the EXACT destination
                        // coordinates from the TDGo entry, not the queued Direction
                        // (`cract.cc:443-446`: `Object Dest = GetMapContainer(DestX, DestY,
                        // DestZ); ::Move(this->ID, this->CrObject, Dest, -1, false, NONE)`).
                        // If the creature was pushed mid-walk, the queued Direction no longer
                        // matches the absolute destination — recompute the step direction from
                        // the current position to the stored destination. Without this, the
                        // 0x6D `new_pos` is wrong: the creature slides in the old direction
                        // from its new (kicked) position, landing on a different tile than the
                        // decompile's `Go` → `::Move` to the exact dest.
                        if cur_pos == dest {
                            // Creature was pushed to exactly its intended destination — the
                            // step is already complete. The decompile's `Go` with Distance==0
                            // calls `::Move` to the same position (a map no-op; the 0x6D from
                            // X to X is visually inert). Skip the move/packets/timing and let
                            // the walk reschedule for the next step.
                            if !self.creature_uses_todo_execute(cid) && reschedule_after {
                                self.add_event_walk(cid, false);
                            }
                            return;
                        }
                        dir = direction_from_positions(cur_pos, dest);
                    }
                }
                // 772 drunk stagger — `cract.cc:392-413`: on stagger, replace the step direction
                // with a random cardinal, `ToDoClear` + `SendSnapback` (player) + `ToDoTalk("Hicks!")`
                // + `ToDoStart`, then continue with the staggered step.
                let mut drunk_staggered = false;
                if let Some(CreatureKind::Player(p)) = self.creatures.get(cid) {
                    if let Some(new_dir) = try_drunk_walk_direction(&p.base) {
                        dir = new_dir;
                        drunk_staggered = true;
                    }
                }
                if drunk_staggered {
                    // `ToDoClear` + `SendSnapback` (player) — `cract.cc:405-407`.
                    if let Some(conn) = self.conn_for_creature(cid) {
                        self.player_todo_clear_with_snapback(conn, cid);
                    } else {
                        self.player_todo_clear(cid);
                    }
                    // `ToDoTalk("Hicks!")` + `ToDoStart()` — `cract.cc:409-411`.
                    let _ = self.enqueue_creature_talk(cid, "Hicks!");
                    self.todo_start_from_action(cid, 1);
                }
                let old_pos = match self.creatures.get(cid) {
                    Some(k) => k.position(),
                    None => return,
                };
                // 772 pre-step kick (`MovePossible(Execute=true)` side-effects). An `EXHAUSTED`
                // outcome (player tile / kick-kill) means the mover does **not** step this beat —
                // `ToDoClear + Wait(1000) + ToDoStart` (`cract.cc:870-877`). F3: the player-tile
                // case clears `Target` (`crnonpl.cc:2236-2238`); the kick-kill case preserves it
                // (`crnonpl.cc:2241-2242`).
                let kick_outcome = if self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
                {
                    self.monster_push_before_step(cid, old_pos.offset(dir), now)
                } else {
                    crate::monster_push::MonsterKickOutcome::Proceed
                };
                match kick_outcome {
                    crate::monster_push::MonsterKickOutcome::Exhausted => {
                        // Kick-kill: Target preserved (C++ Execute catch + `crnonpl.cc:2241-2242`).
                        self.monster_exhausted_wait(cid, false);
                        return;
                    }
                    crate::monster_push::MonsterKickOutcome::ExhaustedDropTarget => {
                        // Player-tile: Target cleared (C++ `crnonpl.cc:2237`).
                        self.monster_exhausted_wait(cid, true);
                        return;
                    }
                    crate::monster_push::MonsterKickOutcome::Proceed => {}
                }
                // C++ `Map::moveCreature` captures per-viewer `oldStackPos` BEFORE removing
                // the creature from the old tile (`map.cpp:292-301`). Snapshot the old tile's
                // creature list now — `broadcast_spectator_move` needs it for per-viewer
                // stack position computation (`Tile::getClientIndexOfCreature`).
                let old_creatures = self
                    .map
                    .get_tile(old_pos)
                    .map(|t| t.body().creatures.clone())
                    .unwrap_or_default();
                let result = self.internal_move_creature_step(cid, dir, now);
                match result {
                    Err(ret) => {
                        tracing::debug!(?cid, ?dir, ?ret, server_ms = self.server_ms,);
                        self.on_walk_step_rejected(cid, ret);
                    }
                    Ok((segments, pending_turn)) => {
                        let new_pos = match self.creatures.get(cid) {
                            Some(k) => k.position(),
                            None => return,
                        };

                        // Emit move packets to self.
                        // 772 real client (decompile `NotifyGo`, `cract.cc:1400-1465`): ONE
                        //   combined packet for the overall old→final move. `AnnounceMovingCreature`
                        //   sends `SendMoveCreature` (0x6D) to all players including self, THEN
                        //   `NotifyGo` sends `SendFloors` (0xBE/0xBF) + `SendRow` (0x65-0x68) to
                        //   self only. The 0x6D self-packet is REQUIRED — without it the client
                        //   doesn't update its central position, only the view shifts → desync
                        //   (§6 experiment). Per-segment emission produces an invalid row sequence
                        //   for combined diagonal+z stair moves (§16.3), so 772 must emit from the
                        //   overall delta.
                        // 772 OTClient: TVP contract — OTClient tracks the local player as a tile
                        //   creature and cannot reconcile the decompile's incremental floor/row
                        //   stream after the leading 0x6D pre-jumps the self to the final tile.
                        //   Route through the per-segment TVP path (teleport = remove + 0x64 for
                        //   z-changes), matching `protocolgame.cpp:1766-1829`. Fixes the
                        //   perpendicular-approach stair desync (west onto south-facing stairs):
                        //   `docs/772_FLOOR_CHANGE_CLIENT_TARGETS.md` §6.
                        // 1098 (TVP `sendMoveCreature`): per-segment emission — each
                        //   `map.moveCreature` call sends its own packet (teleport for z-changes).
                        let is_772 = !self.codec.caps().move_creature_self_packet;
                        let is_otclient = self.creatures.get(cid).is_some_and(
                            |k| matches!(k, CreatureKind::Player(p) if p.is_otclient()),
                        );
                        let use_notify_go = is_772 && !is_otclient;
                        let overall_old_stack = segments.first().map(|s| s.old_stack).unwrap_or(1);
                        if let Some(conn) = self.conn_for_creature(cid) {
                            if use_notify_go {
                                self.emit_notify_go(cid, conn, old_pos, new_pos, overall_old_stack);
                            } else {
                                for seg in &segments {
                                    if seg.teleport {
                                        // C++ teleport path: sendRemoveTileCreature + sendMapDescription
                                        self.emit_teleport_move_packet(
                                            cid,
                                            conn,
                                            seg.from,
                                            seg.to,
                                            seg.old_stack,
                                        );
                                    } else {
                                        self.emit_move_packet(
                                            cid,
                                            conn,
                                            seg.from,
                                            seg.to,
                                            seg.old_stack,
                                        );
                                    }
                                }
                            }
                        }
                        // Broadcast to spectators using overall old→new for now.
                        // C++ broadcasts per moveCreature call, but the initial step is most
                        // important for spectator rendering. `old_creatures` was captured before
                        // the move for per-viewer stack position computation.
                        self.broadcast_spectator_move(cid, old_pos, new_pos, &old_creatures);

                        // Emit deferred chain turn `0x6B` AFTER move packets — matches C++ wire
                        // order: `Map::moveCreature` sends `sendMoveCreature` during the move
                        // loop (`map.cpp:316`), THEN `Game::internalMoveCreature` calls
                        // `internalCreatureTurn` → `sendCreatureTurn` (`0x6B`) after the loop
                        // (`game.cpp:888`). The direction was already set in
                        // `internal_move_creature_step`; this only emits the `0x6B` broadcast.
                        if let Some(pt) = pending_turn {
                            internal_creature_turn_broadcast_only(self, pt.cid, pt.dir);
                        }

                        // Player-only walk debug — before NotifyGo mutates `earliest_walk_server_ms`.
                        // Matches decompile `NotifyGo` (`cract.cc:1518-1534`) + `GetSpeed` (`crmain.cc:477`).
                        if self
                            .creatures
                            .get(cid)
                            .is_some_and(|k| matches!(k, CreatureKind::Player(_)))
                        {
                            let gs_dest = self
                                .map
                                .get_tile(new_pos)
                                .map(|t| ground_speed_for_tile_body(t.body(), self.items_db.as_ref()))
                                .unwrap_or(150);
                            let (go_dbg, eff_dbg, wire_dbg, step_ms) = self
                                .creatures
                                .get(cid)
                                .map(|k| {
                                    let base = k.base();
                                    let go = crate::walk::walk_timing::go_strength_for_walk_pub(
                                        crate::walk::walk_timing::WalkSpeedRole::Player,
                                        base,
                                        &self.mechanics,
                                    );
                                    let eff = crate::formulas::linear_go_effective_speed(go);
                                    let wire = crate::walk::wire_step_speed(
                                        crate::walk::walk_timing::WalkSpeedRole::Player,
                                        base,
                                        &self.mechanics,
                                    );
                                    let ms = get_step_duration_ms_with_direction(
                                        k,
                                        base,
                                        dir,
                                        gs_dest,
                                        &self.mechanics,
                                    );
                                    (go, eff, wire, ms)
                                })
                                .unwrap_or((0, 0, 0, 0));
                            let ground_id = self
                                .map
                                .get_tile(new_pos)
                                .and_then(|t| t.body().ground)
                                .unwrap_or(0);
                            tracing::debug!(
                                cid = ?cid,
                                from = ?old_pos,
                                to = ?new_pos,
                                ?dir,
                                ground_item_id = ground_id,
                                ground_speed = gs_dest,
                                go_strength = go_dbg,
                                effective_speed = eff_dbg,
                                wire_speed = wire_dbg,
                                step_duration_ms = step_ms,
                                server_ms = self.server_ms,
                                earliest_walk_ms = self
                                    .creatures
                                    .get(cid)
                                    .map(|k| k.base().earliest_walk_server_ms)
                                    .unwrap_or(0),
                                "player walk step",
                            );
                        }
                        // TFS `lastStep` after `sendCreatureMove` (`map.cpp` ~309–324);
                        // 772 `NotifyGo` (`cract.cc:1515–1535`) — also used by KickCreature.
                        // Facing already set in `internal_move_creature_step` — do not NotifyTurn.
                        self.apply_notify_go_after_relocate(cid, old_pos, new_pos, dir, false);
                    }
                }
            } else {
                // TFS: `getNextStep` false → `stopEventWalk`, `onWalkComplete` if queue empty (`src/creature.cpp` ~215–219).
                self.stop_event_walk(cid);
                if self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
                {
                    // 772 idle drain owns chase repath — no TFS walk-timer poll (X5).
                    // When inside todo execute, `finish_creature_todo_execute` calls `idle_stimulus`;
                    // do not arm `schedule_walk_followup_deadline` (blocks `walk_timer_idle` gate).
                    let in_todo_execute = self
                        .creatures
                        .get(cid)
                        .is_some_and(|k| k.base().todo.locked);
                    if !in_todo_execute
                        && (self.monster_should_keep_chase_walk_alive(cid)
                            || self.monster_should_keep_dance_walk_alive(cid))
                    {
                        self.request_idle_stimulus(cid);
                    }
                } else {
                    stopped_without_reschedule = true;
                }
                self.events.on_walk_complete(cid);
                if self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
                {
                    self.monster_on_walk_complete(cid);
                }
            }
        }

        if self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Player(p) if p.base.cancel_next_walk))
        {
            let dir_byte = self.creatures.get(cid).and_then(|k| match k {
                CreatureKind::Player(p) => Some(p.base.direction as u8),
                _ => None,
            });
            let conn = self.conn_for_creature(cid);
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                p.base.walk_queue.clear();
                p.base.walk_destinations.clear();
                p.base.cancel_next_walk = false;
            }
            // TFS `Player::onWalkAborted` — `sendCancelWalk` (`player.cpp` ~3384–3387).
            if let (Some(conn), Some(db)) = (conn, dir_byte) {
                self.enqueue_encoded(conn, self.codec.encode_cancel_walk(db));
            }
            self.clear_player_walk_action(cid);
        }

        if !stopped_without_reschedule && reschedule_after {
            // Phase 4: 1098 `commit_next_walk_deadline` + `add_event_walk` reschedule deleted —
            // both eras use the ToDo queue for step chaining.
            if self.creature_uses_todo_execute(cid) {
                // Step chain owned by the per-creature action queue.
            } else {
                self.add_event_walk(cid, false);
            }
        }
    }

    /// TFS `Game::internalMoveCreature` — both overloads combined.
    /// C++ ref: src/game.cpp:797-894
    ///
    /// Returns `Ok((segments, pending_turn))` on success — each segment corresponds
    /// to one C++ `Map::moveCreature` call and needs its own move packet.
    /// `pending_turn` is `Some` when a post-chain direction change is needed
    /// (C++ `game.cpp:882-891`); the caller must emit the `0x6B` broadcast AFTER
    /// the move packets to match C++ wire order (`map.cpp:316` sends moves before
    /// `game.cpp:888` sends the turn).
    /// Returns `Err(ret)` when the move is rejected.
    fn internal_move_creature_step(
        &mut self,
        cid: CreatureId,
        direction: Direction,
        _now: Instant,
    ) -> Result<(Vec<MoveSegment>, Option<PendingChainTurn>), ReturnValue> {
        let current_pos = match self.creatures.get(cid) {
            Some(k) => k.position(),
            None => return Err(ReturnValue::NotPossible),
        };
        let flags_in = FLAG_IGNOREFIELDDAMAGE;

        let is_player = self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Player(_)));

        // OTClient flag for era-aware teleport detection. OTClient-on-772 uses TVP's
        // `areInRange<1,1,0>` (z-changes are teleports); the real 7.72 client uses the
        // decompile `NotifyGo` `DistanceZ <= 1` rule (adjacent z-changes are incremental).
        // See `is_adjacent_move` and `docs/772_FLOOR_CHANGE_CLIENT_TARGETS.md` §6.
        let otclient = self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Player(p) if p.is_otclient()));

        // Phase 1: destination — height-based floor change is player-only (`game.cpp` ~805).
        let (dest_pos, flags) = if is_player {
            resolve_player_move_destination(
                &self.map,
                self.items_db.as_ref(),
                &self.items,
                current_pos,
                direction,
                flags_in,
            )
        } else {
            (current_pos.offset(direction), flags_in)
        };
        let is_floor_change = dest_pos.z != current_pos.z;
        let Some(to_tile) = self.map.get_tile(dest_pos) else {
            tracing::warn!(
                ?cid,
                ?direction,
                from = ?current_pos,
                dest = ?dest_pos,
                "destination tile is None (not loaded)"
            );
            return Err(ReturnValue::NotPossible);
        };

        let ret = tile_query_add_creature(self, to_tile, cid, flags);
        if ret != ReturnValue::NoError {
            return Err(ret);
        }

        let old_pos = current_pos;
        // Phase 4: `gs_next_action` (source tile ground speed for 1098 `nextAction` lockout)
        // is no longer needed — both eras use `EarliestWalkTime` ToDo delay.

        let mut segments: Vec<MoveSegment> = Vec::new();

        // Collect old_stack for the initial move BEFORE moving the creature.
        let raw_initial_stack = self
            .map
            .get_tile(old_pos)
            .map(|t| self_move_stack_pos(self, cid, t.body()))
            .unwrap_or(-1);
        let initial_old_stack = if raw_initial_stack >= 0 {
            raw_initial_stack
        } else {
            1
        };

        // C++ map.cpp:262 — teleport detection for initial step.
        // 772: adjacent z-changes (dz ≤ 1) use `SendFloors`/`SendRow` (not teleport).
        // 1098: z-changes (dz != 0) are always teleports.
        let has_ground = self
            .map
            .get_tile(dest_pos)
            .map(|t| t.body().ground.is_some())
            .unwrap_or(false);
        let initial_teleport =
            !has_ground || !is_adjacent_move(&self.codec, otclient, old_pos, dest_pos);
        // Move creature to initial destination.
        self.move_creature_on_map(cid, old_pos, dest_pos);

        segments.push(MoveSegment {
            from: old_pos,
            to: dest_pos,
            old_stack: initial_old_stack,
            teleport: initial_teleport,
        });

        // Phase 2: queryDestination while-loop (game.cpp ~863-880).
        // C++ ref: src/tile.cpp:735-830 — chain floor changes up to MAP_MAX_LAYERS (16).
        const MAP_MAX_LAYERS: usize = 16;
        let mut final_pos = dest_pos;
        let mut from_pos: Option<Position> = None;
        for _ in 0..MAP_MAX_LAYERS {
            let tile_flags = match self.map.get_tile(final_pos) {
                Some(t) => t.body().flags,
                None => break,
            };
            let Some((new_pos, _new_flags)) = query_destination(&self.map, final_pos, tile_flags)
            else {
                break;
            };

            // Collect old_stack for this chain step BEFORE moving.
            let chain_old_stack = self
                .map
                .get_tile(final_pos)
                .map(|t| self_move_stack_pos(self, cid, t.body()))
                .filter(|s| *s >= 0)
                .unwrap_or(1);

            let chain_has_ground = self
                .map
                .get_tile(new_pos)
                .map(|t| t.body().ground.is_some())
                .unwrap_or(false);
            let chain_teleport =
                !chain_has_ground || !is_adjacent_move(&self.codec, otclient, final_pos, new_pos);

            // Move creature to the chained destination.
            self.move_creature_on_map(cid, final_pos, new_pos);

            segments.push(MoveSegment {
                from: final_pos,
                to: new_pos,
                old_stack: chain_old_stack,
                teleport: chain_teleport,
            });

            from_pos = Some(final_pos);
            final_pos = new_pos;
        }

        // ── Direction setting (must match C++ order) ──
        //
        // C++ `Map::moveCreature` (map.cpp ~295-306): sets direction from dx/dy of the
        // *initial* move (old_pos → dest_pos), but only when NOT a teleport (same z, ≤1 tile).
        // C++ `game.cpp:815,829`: height-based z-change → direction = walk input direction.
        // C++ `game.cpp:882-891`: after queryDestination chain → direction from chain from→to.
        if let Some(k) = self.creatures.get_mut(cid) {
            // Step 1: direction from the initial move (same as Map::moveCreature).
            // C++ ref: src/map.cpp:295-306
            set_direction_from_step(old_pos, dest_pos, k);

            // Step 2: height-based z-change overrides with walk input direction (player height walk only).
            // C++ ref: src/game.cpp:815,829
            if is_player && old_pos.z != dest_pos.z {
                k.base_mut().direction = direction;
            }
        }

        // Set the authoritative position FIRST — must happen before any broadcast so that
        // `can_see_position(viewer=self, pos=final_pos)` reads the correct z-level and
        // includes the moving player themselves in the `0x6B` spectator set.
        if let Some(k) = self.creatures.get_mut(cid) {
            k.set_position(final_pos);
            // Phase 4: 1098 `nextAction` lockout deleted (P9 dissolves) — both eras use
            // `EarliestWalkTime` ToDo delay for walk step gating.
        }

        if chase_debug::chase_path_debug_enabled()
            && self
                .creatures
                .get(cid)
                .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
        {
            if let Some(k) = self.creatures.get(cid) {
                chase_debug::log_go_exec(
                    self.chase_trace_tick(),
                    cid,
                    k.base().name.as_str(),
                    old_pos,
                    final_pos,
                );
            }
        }

        // Step 3: post-queryDestination chain turn — set direction NOW (matching C++
        // `internalCreatureTurn` state mutation at `game.cpp:888`), but DEFER the `0x6B`
        // broadcast. C++ wire order: `Map::moveCreature` sends `sendMoveCreature` during
        // the move loop (`map.cpp:316`), THEN `internalCreatureTurn` sends `0x6B`
        // (`game.cpp:888`). Rust emits move packets in `on_walk` AFTER this function
        // returns, so the `0x6B` must be deferred to the caller to avoid sending it
        // before the client knows the creature moved to the new position.
        let mut pending_turn: Option<PendingChainTurn> = None;
        if let Some(fp) = from_pos {
            if fp.z != final_pos.z && (fp.x != final_pos.x || fp.y != final_pos.y) {
                let dir = direction_from_positions(fp, final_pos);
                if !is_diagonal(dir) {
                    // Set direction immediately (state mutation only).
                    if let Some(k) = self.creatures.get_mut(cid) {
                        k.base_mut().direction = dir;
                    }
                    pending_turn = Some(PendingChainTurn { cid, dir });
                }
            }
        }

        if self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Player(_)))
        {
            self.auto_close_containers_for_player(cid);
        }

        // Ghost diagnostic: after all moves, verify the creature is ONLY on the final tile.
        // Scan the old position and any intermediate positions for stale registrations.
        let mut ghost_positions: Vec<Position> = Vec::new();
        for seg in &segments {
            if seg.from != final_pos
                && self
                    .map
                    .get_tile(seg.from)
                    .is_some_and(|t| t.body().creatures.contains(&cid))
            {
                ghost_positions.push(seg.from);
            }
        }
        if !ghost_positions.is_empty() {
            tracing::error!(
                ?cid,
                final_pos = ?final_pos,
                ?ghost_positions,
                "GHOST DETECTED: creature still registered on old/intermediate tile(s) after move"
            );
            debug_assert!(
                ghost_positions.is_empty(),
                "GHOST: creature {:?} still on tiles {:?} after move to {:?}",
                cid,
                ghost_positions,
                final_pos
            );
        }

        // Final sanity: creature must be on the final tile.
        let on_final = self
            .map
            .get_tile(final_pos)
            .is_some_and(|t| t.body().creatures.contains(&cid));
        if !on_final && is_player && (is_floor_change || segments.len() > 1) {
            tracing::error!(
                ?cid,
                final_pos = ?final_pos,
                "GHOST DETECTED: creature NOT on final tile after move!"
            );
            debug_assert!(
                on_final,
                "GHOST: creature {:?} not on final tile {:?} after move",
                cid, final_pos
            );
        }

        Ok((segments, pending_turn))
    }

    /// One walk step for push / auxiliary callers (discards segment payloads).
    /// Emits any pending chain turn `0x6B` immediately — push callers don't have
    /// a separate move-packet emission phase, so the deferred turn is flushed here.
    #[cfg(any(test, feature = "sim"))]
    pub(crate) fn try_creature_walk_step(
        &mut self,
        cid: CreatureId,
        direction: Direction,
        now: Instant,
    ) -> bool {
        match self.internal_move_creature_step(cid, direction, now) {
            Ok((_segments, pending_turn)) => {
                if let Some(pt) = pending_turn {
                    internal_creature_turn_with_broadcast(self, pt.cid, pt.dir);
                }
                true
            }
            Err(_) => false,
        }
    }

    /// Move a creature between tiles on the map (unregister from old, register at new).
    /// C++ `Map::moveCreature` — position follows the tile (`newTile.addThing`) before
    /// `onCreatureMove` fan-out (`map.cpp` ~293–324).
    pub(crate) fn move_creature_on_map(&mut self, cid: CreatureId, from: Position, to: Position) {
        if from == to {
            return;
        }
        // Ghost diagnostic: verify the creature is actually on the `from` tile.
        // If not, unregister is a silent no-op and the creature stays on its real
        // tile → ghost (creature on two tiles simultaneously).
        let on_from = self
            .map
            .get_tile(from)
            .is_some_and(|t| t.body().creatures.contains(&cid));
        if !on_from {
            let actual = self.creatures.get(cid).map(|k| k.position());
            tracing::error!(
                ?cid,
                from = ?from,
                to = ?to,
                actual_pos = ?actual,
                "move_creature_on_map: creature NOT on `from` tile — \
                 unregister will be a no-op → ghost on real tile"
            );
            debug_assert!(
                on_from,
                "move_creature_on_map: creature {:?} not on `from` tile {:?} (actual {:?})",
                cid, from, actual
            );
        }
        self.map.unregister_creature_at(from, cid);
        self.map.register_creature_at(to, cid);
        if let Some(k) = self.creatures.get_mut(cid) {
            k.set_position(to);
        }
        self.monster_dispatch_creature_move(cid, from, to);
        self.npc_dispatch_creature_move(cid, from, to, false);
    }

    /// TFS `Creature::getPathTo` / `Map::getPathMatching` for walk-to-item (`creature.cpp` ~1735).
    ///
    /// 772 player viewport is `VisibleX/Y = 7` (`cract.cc:1093-1094`), not the monster `10`.
    /// Path is trimmed via `truncate_tshortway_go_queue` — matching C++ `TShortway::Calculate`
    /// (`cract.cc:282-301`): `while(Node != NULL && MaxSteps > 0 && (MustReach || CurDistance > 1))`.
    /// `path_matching_tshortway` returns the full predecessor chain; the trim here is the
    /// equivalent of the C++ reconstruction loop's `MaxSteps` + `CurDistance` checks.
    ///
    /// `max_steps` maps to C++ `ToDoGo(..., MaxSteps)`:
    /// - Walk-to-use: `INT_MAX` → `usize::MAX` (`cract.cc:1282`)
    /// - Close chase: `3` (`crcombat.cc:499`)
    /// - Range chase: `Distance - 4` (`crcombat.cc:503`)
    ///
    /// `max_target_dist = 0` → `MustReach = true` (walk to exact target).
    /// `max_target_dist > 0` → `MustReach = false`, stop at Chebyshev ≤ 1 (C++ `CurDistance > 1`).
    pub(crate) fn get_creature_path_to(
        &self,
        cid: CreatureId,
        target: Position,
        min_target_dist: i32,
        max_target_dist: i32,
        max_steps: usize,
    ) -> Option<Vec<Direction>> {
        use crate::pathfinding::{
            get_path_matching, truncate_tshortway_go_queue, FindPathParams,
            CREATURE_ON_TILE_PATH_COST, PLAYER_PATH_VIEW_RADIUS,
        };

        let start = self.creatures.get(cid)?.position();
        let fpp = FindPathParams {
            min_target_dist,
            max_target_dist,
            clear_sight: true,
            allow_diagonal: true,
            full_path_search: true,
            max_search_dist: 0,
        };
        struct PathCtx<'a> {
            world: &'a GameWorld,
            cid: CreatureId,
        }
        let ctx = PathCtx { world: self, cid };
        let mut scratch = self.tshortway_scratch.borrow_mut();
        let path = get_path_matching(
            &self.map,
            start,
            target,
            &fpp,
            self.mechanics.profile.path_cost,
            self.mechanics.profile.path_search,
            self.mechanics.profile.path_forward_fallback,
            PLAYER_PATH_VIEW_RADIUS,
            |pos| {
                let Some(tile) = ctx.world.map.get_tile(pos) else {
                    return false;
                };
                tile_query_add_creature(ctx.world, tile, ctx.cid, PATHFIND_WALK_FLAGS)
                    == ReturnValue::NoError
            },
            |pos| {
                let Some(tile) = ctx.world.map.get_tile(pos) else {
                    return 0;
                };
                let mut cost = 0u32;
                for &c in tile.body().creatures.iter() {
                    if c != ctx.cid {
                        cost += CREATURE_ON_TILE_PATH_COST;
                    }
                }
                cost
            },
            |pos| {
                ctx.world
                    .map
                    .get_tile(pos)
                    .map(|t| ctx.world.tile_ground_speed(t.body()))
                    .unwrap_or(150)
            },
            Some(&mut *scratch),
        )?;

        let must_reach = max_target_dist == 0;
        let trimmed = truncate_tshortway_go_queue(start, target, path, max_steps, must_reach);
        Some(trimmed)
    }
}

#[cfg(test)]
mod step_speed_tests {
    use std::time::{Duration, Instant};

    use super::walk_timing::{
        calculated_step_speed_tfs, get_event_step_ticks, get_step_duration,
        get_step_duration_ms_with_direction, walk_timing_speed, wire_step_speed, WalkSpeedRole,
    };
    use crate::creature::CreatureKind;
    use crate::formulas::{linear_go_effective_speed, Mechanics};
    use crate::test_world::support::test_player;
    use crate::Monster;
    use tfs_rust_common::{Position, ProtocolVersion};

    /// Anchors from `src/creature.cpp` `Creature::getStepDuration` (`floor((A*log((step/2)+B)+C)+0.5)`).
    #[test]
    fn calculated_step_speed_matches_tfs_creature_cpp() {
        assert_eq!(calculated_step_speed_tfs(10), 1);
        assert_eq!(calculated_step_speed_tfs(220), 278);
        assert_eq!(calculated_step_speed_tfs(400), 464);
        assert_eq!(calculated_step_speed_tfs(1500), 1137);
    }

    /// 772 wire sends `GetSpeed()` = `2*GoStrength+80` for ALL creatures (`sending.cc:265`).
    #[test]
    fn wire_step_speed_772_player_is_effective_get_speed() {
        let p = test_player("Walker", Position::new(100, 100, 7));
        let mut base = p.base.clone();
        base.speed = 220;
        base.base_speed = 220;
        let mech = Mechanics::for_version(ProtocolVersion::V772);
        // Decompile `sending.cc:265`: SendWord(GetSpeed()) = 2*220+80 = 520.
        assert_eq!(wire_step_speed(WalkSpeedRole::Player, &base, &mech), 520);
        assert_eq!(walk_timing_speed(WalkSpeedRole::Player, &base, &mech), 520);
    }

    /// 772 player GoStrength scales with level (decompile `TSkillAdd::Advance`,
    /// `crskill.cc:667` with `AddLevel=1` from `human.mon:27`): `base + (level-1)`.
    /// Wire sends `GetSpeed()` = `2*go+80` (`sending.cc:265`).
    #[test]
    fn wire_step_speed_772_player_scales_with_level() {
        let mech = Mechanics::for_version(ProtocolVersion::V772);
        // Use base_speed=220 to match the pre-PC-0 hardcoded constant; the
        // shipped vocations.lua has base_speed=70, but the level-scaling shape
        // is what this test verifies.
        let profile = crate::creature::vocation::VocationProfile {
            base_speed: 220,
            ..crate::creature::vocation::VocationProfile::none_vocation()
        };
        // Decompile shape: GoStrength = 220 + (level-1); wire = 2*go+80.
        for (level, expected_go) in [(1, 220), (2, 221), (8, 227), (50, 269)] {
            let go = crate::creature::vocation::base_walk_speed(
                crate::formulas::StepSpeedModel::LinearGo,
                &profile,
                level,
                false,
            );
            assert_eq!(go, expected_go, "level {level}");
            let mut base = test_player("Walker", Position::new(100, 100, 7)).base;
            base.speed = go;
            base.base_speed = go;
            let expected_wire = (2 * go + 80) as u16;
            assert_eq!(
                wire_step_speed(WalkSpeedRole::Player, &base, &mech),
                expected_wire,
                "level {level}"
            );
        }
    }

    /// GM `PlayerFlag_SetMaxSpeed` pins base speed to 1500. With the shipped 772
    /// `playerSpeed = "772"` profile, wire sends `2*1500+80 = 3080`.
    #[test]
    fn wire_step_speed_772_set_max_speed() {
        let p = test_player("Walker", Position::new(100, 100, 7));
        let mut base = p.base.clone();
        base.speed = 1500;
        base.base_speed = 1500;
        let mut mech = Mechanics::for_version(ProtocolVersion::V772);
        mech.profile.player_speed_model = crate::formulas::PlayerSpeedModel::Classic772;
        assert_eq!(wire_step_speed(WalkSpeedRole::Player, &base, &mech), 3080);
    }

    /// 1098 GM `PlayerFlag_SetMaxSpeed` wire holds the clamped GoStrength (codec halves it).
    #[test]
    fn wire_step_speed_1098_set_max_speed() {
        let p = test_player("Walker", Position::new(100, 100, 7));
        let mut base = p.base.clone();
        base.speed = 1500;
        base.base_speed = 1500;
        let mech = Mechanics::for_version(ProtocolVersion::V1098);
        assert_eq!(wire_step_speed(WalkSpeedRole::Player, &base, &mech), 1500);
    }

    /// 772 monster wire matches TVP `getStepSpeed()` — wolf GoStrength 42 → 164 on wire.
    #[test]
    fn wire_step_speed_772_monster_is_effective_get_speed() {
        let mut base = test_player("Wolf", Position::new(100, 100, 7)).base;
        base.speed = 42;
        base.base_speed = 42;
        let mech = Mechanics::for_version(ProtocolVersion::V772);
        let kind = CreatureKind::Monster(Monster::new(base.clone(), Position::new(0, 0, 7)));
        assert_eq!(
            wire_step_speed(WalkSpeedRole::MonsterOrNpc, &base, &mech),
            164
        );
        assert_eq!(get_step_duration(&kind, &base, 150, &mech), 950);
    }

    /// 1098 wire payload is halved in codec; neutral struct holds full GoStrength before `/2`.
    #[test]
    fn wire_step_speed_1098_player_is_clamped_go() {
        let p = test_player("Walker", Position::new(100, 100, 7));
        let mut base = p.base.clone();
        base.speed = 220;
        base.base_speed = 220;
        let mech = Mechanics::for_version(ProtocolVersion::V1098);
        assert_eq!(wire_step_speed(WalkSpeedRole::Player, &base, &mech), 220);
    }

    /// Overdue `addEventWalk(true)` (walk_delay <= 0) returns `1` ms to trigger step immediately.
    #[test]
    fn event_step_ticks_overdue_only_delay_returns_one_ms() {
        let p = test_player("Walker", Position::new(100, 100, 7));
        let mut base = p.base.clone();
        base.speed = 220;
        base.base_speed = 220;
        base.last_step_ground_speed = 150;
        base.last_step_cost = 1;
        let mech = Mechanics::for_version(ProtocolVersion::V1098);
        let kind = CreatureKind::Player(p);
        let step_ms = get_step_duration(&kind, &base, 150, &mech);
        base.last_step = Some(Instant::now() - Duration::from_millis((step_ms + 10) as u64));
        let ticks =
            get_event_step_ticks(&kind, &base, true, 150, None, Instant::now(), &mech, None);
        assert_eq!(ticks, 1);
    }

    #[test]
    fn event_step_ticks_fresh_only_delay_returns_one_ms() {
        let p = test_player("Walker", Position::new(100, 100, 7));
        let mut base = p.base.clone();
        base.speed = 220;
        base.base_speed = 220;
        base.last_step = None;
        let mech = Mechanics::for_version(ProtocolVersion::V772);
        let kind = CreatureKind::Player(p);
        let ticks =
            get_event_step_ticks(&kind, &base, true, 150, None, Instant::now(), &mech, None);
        assert_eq!(ticks, 1);
    }

    /// 772 wolf GoStrength 42 → `GetSpeed` 164; `NotifyGo` quantizes to `Beat` (50 ms).
    #[test]
    fn linear_go_step_duration_matches_notify_go() {
        let p = test_player("Wolf", Position::new(100, 100, 7));
        let mut base = p.base.clone();
        base.speed = 42;
        base.base_speed = 42;
        let mech = Mechanics::for_version(ProtocolVersion::V772);
        assert_eq!(linear_go_effective_speed(42), 164);
        let kind = CreatureKind::Monster(Monster::new(base.clone(), Position::new(0, 0, 7)));
        assert_eq!(get_step_duration(&kind, &base, 150, &mech), 950);
        assert_eq!(get_step_duration(&kind, &base, 150, &mech) % 50, 0);
    }

    /// 772 diagonal: `×3` waypoints before step quantizer ceil — 2750 ms, not TFS-style 950×3.
    #[test]
    fn linear_go_diagonal_step_duration_quantizes_waypoints_before_beat() {
        use tfs_rust_common::enums::Direction;
        let p = test_player("Wolf", Position::new(100, 100, 7));
        let mut base = p.base.clone();
        base.speed = 42;
        base.base_speed = 42;
        let mech = Mechanics::for_version(ProtocolVersion::V772);
        let kind = CreatureKind::Monster(Monster::new(base.clone(), Position::new(0, 0, 7)));
        let cardinal =
            get_step_duration_ms_with_direction(&kind, &base, Direction::East, 150, &mech);
        let diagonal =
            get_step_duration_ms_with_direction(&kind, &base, Direction::NorthEast, 150, &mech);
        assert_eq!(cardinal, 950);
        assert_eq!(diagonal, 2750);
        assert_ne!(diagonal, cardinal * 3, "CipSoft ceils before ×3, not after");
    }

    /// 1098 — TFS log curve; durations quantize to 50 ms beat.
    #[test]
    fn tfs_log_step_duration_quantizes_to_beat() {
        let p = test_player("Stepper", Position::new(100, 100, 7));
        let mech = Mechanics::for_version(ProtocolVersion::V1098);
        let kind = CreatureKind::Player(p.clone());

        for &speed in &[120i32, 200, 220, 350, 500] {
            let mut base = p.base.clone();
            base.speed = speed;
            base.base_speed = speed;
            let d = get_step_duration(&kind, &base, 150, &mech);
            assert_eq!(
                d % 50,
                0,
                "1098 duration must be a multiple of 50 (speed {speed})"
            );
        }
    }

    /// B1.3 — a registered Tier-2 `getStepDuration` overrides the native curve entirely.
    #[test]
    fn tier2_step_duration_hook_overrides_native() {
        use crate::formulas::{FormulaHooks, MechanicsProfile};
        let lua = mlua::Lua::new();
        lua.load("function getStepDuration(speed, ground, diagonal) return 1234 end")
            .exec()
            .unwrap();
        let mech = Mechanics {
            profile: MechanicsProfile::for_version(ProtocolVersion::V1098),
            hooks: FormulaHooks::from_lua_for_test(lua),
        };
        let p = test_player("Hooked", Position::new(100, 100, 7));
        let kind = CreatureKind::Player(p.clone());
        assert_eq!(get_step_duration(&kind, &p.base, 150, &mech), 1234);
    }

    /// Audit #5: `completed_step_duration_ms` LinearGo arm must NOT apply `last_step_cost
    /// = 2` (z-change / stair-hop) as a waypoint multiplier — C++ `NotifyGo` only
    /// multiplies ×3 for diagonal **same-z**; a floor change gets ×1
    /// (`cract.cc:1526-1528`). The old code passed `last_step_cost.max(1)`, doubling the
    /// post-stair-hop cooldown (e.g. 600 ms instead of 400 ms for speed 220 / ground 150).
    #[test]
    fn linear_go_completed_step_zchange_uses_one_waypoint_cost() {
        use super::walk_timing::get_walk_delay_logical;
        let p = test_player("Walker", Position::new(100, 100, 7));
        let mut base = p.base.clone();
        base.speed = 220;
        base.base_speed = 220;
        base.last_step_ground_speed = 150;
        base.last_step_server_ms = Some(0);
        let mech = Mechanics::for_version(ProtocolVersion::V772);
        let kind = CreatureKind::Player(p);

        // Cardinal same-z (last_step_cost = 1) — the C++ NotifyGo ×1 reference.
        let mut base_cardinal = base.clone();
        base_cardinal.last_step_cost = 1;
        let delay_cardinal = get_walk_delay_logical(&kind, &base_cardinal, 0, &mech);

        // Diagonal same-z (last_step_cost = 3) — C++ NotifyGo ×3.
        let mut base_diagonal = base.clone();
        base_diagonal.last_step_cost = 3;
        let delay_diagonal = get_walk_delay_logical(&kind, &base_diagonal, 0, &mech);

        // Z-change / stair-hop (last_step_cost = 2) — C++ NotifyGo ×1, NOT ×2.
        let mut base_zchange = base.clone();
        base_zchange.last_step_cost = 2;
        let delay_zchange = get_walk_delay_logical(&kind, &base_zchange, 0, &mech);

        // Cardinal: 150×1000/520 = 288 → ceil 50 = 300 ms.
        assert_eq!(delay_cardinal, 300, "cardinal completed step = 300 ms");
        // Z-change must equal cardinal (×1), not double (×2 → 600 ms).
        assert_eq!(
            delay_zchange, delay_cardinal,
            "z-change completed step must use ×1 waypoint cost, not ×2 (cract.cc:1526-1528)"
        );
        // Diagonal: 150×3×1000/520 = 865 → ceil 50 = 900 ms (×3 before ceil).
        assert_eq!(
            delay_diagonal, 900,
            "diagonal completed step = ×3 = 900 ms"
        );
        assert_ne!(
            delay_diagonal, delay_cardinal,
            "diagonal must differ from cardinal (×3 vs ×1)"
        );
    }

    /// Phase 3 reachability guard (`docs/772_FLOOR_CHANGE_DESYNC.md` §16.3):
    /// 1098 uses `areInRange<1,1,0>` (dz==0) — z-changes are teleports.
    /// 772 uses `NotifyGo`'s adjacent condition (dz ≤ 1) — adjacent z-changes use
    /// `SendFloors`/`SendRow` via `send_move_creature_player`, NOT the teleport path.
    /// The both-axes+z diagonal case IS reachable for 772 (e.g. diagonal stair-step
    /// with dx=1, dy=1, dz=1) — it routes through `send_move_creature_player`'s
    /// z-change branch, which has a row-ordering divergence from 772 `NotifyGo`
    /// (§16.3). This is accepted because the row content is correct; only the
    /// sequence differs, and the 772 client applies rows to viewport edges
    /// sequentially so the sequence matters but the desync is minor and
    /// self-correcting (same mechanism as same-z double-shift, §1).
    #[test]
    fn era_aware_teleport_detection_routes_z_changes_correctly() {
        use super::{are_in_range_1_1_0, are_in_range_1_1_1, is_adjacent_move};
        use tfs_rust_common::Position;
        use tfs_rust_common::ProtocolVersion;
        use tfs_rust_net::codec::Codec;

        // 1098: dz==0 required — z-changes are teleports.
        assert!(are_in_range_1_1_0(
            Position::new(100, 100, 7),
            Position::new(101, 101, 7),
        ));
        assert!(!are_in_range_1_1_0(
            Position::new(100, 100, 7),
            Position::new(101, 101, 8),
        ));
        assert!(!are_in_range_1_1_0(
            Position::new(100, 100, 7),
            Position::new(100, 100, 8),
        ));

        // 772: dz ≤ 1 allowed — adjacent z-changes use SendFloors/SendRow.
        assert!(are_in_range_1_1_1(
            Position::new(100, 100, 7),
            Position::new(101, 101, 8),
        ));
        assert!(are_in_range_1_1_1(
            Position::new(100, 100, 7),
            Position::new(100, 100, 8),
        ));
        // 772: dz > 1 is still a teleport.
        assert!(!are_in_range_1_1_1(
            Position::new(100, 100, 7),
            Position::new(100, 100, 9),
        ));

        // Era- and client-aware dispatch:
        //   1098 → dz==0 (z-changes are teleports)
        //   772 real client → dz ≤ 1 (adjacent z-changes use SendFloors/SendRow)
        //   772 OTClient → dz==0 (TVP contract — z-changes are teleports)
        let codec_1098 = Codec::from_version(ProtocolVersion::V1098).unwrap();
        let codec_772 = Codec::from_version(ProtocolVersion::V772).unwrap();
        let z_change = (Position::new(100, 100, 7), Position::new(100, 100, 8));
        assert!(
            !is_adjacent_move(&codec_1098, false, z_change.0, z_change.1),
            "1098: z-change is teleport"
        );
        assert!(
            is_adjacent_move(&codec_772, false, z_change.0, z_change.1),
            "772 real client: adjacent z-change is NOT teleport"
        );
        assert!(
            !is_adjacent_move(&codec_772, true, z_change.0, z_change.1),
            "772 OTClient: z-change is teleport (TVP contract)"
        );
    }
}

#[cfg(test)]
mod monster_walk_tests {
    use crate::creature::CreatureKind;
    use crate::login_out::creature_wire_id;
    use crate::test_world::support;
    use tfs_rust_common::enums::Direction;
    use tfs_rust_common::ConnId;
    use tfs_rust_common::Position;

    /// Regression: when a creature is pushed mid-walk, `on_walk` must step toward the
    /// **absolute destination** (from `walk_destinations`), not the queued `Direction`.
    /// The decompile's `Go(DestX, DestY, DestZ)` (`cract.cc:383-446`) moves to the exact
    /// destination coordinates — it does not use a direction. After a kick, the queued
    /// Direction and the absolute destination diverge. Without the recompute, the 0x6D
    /// `new_pos` is wrong: the creature slides in the old direction from its kicked
    /// position, landing on a different tile.
    ///
    /// Scenario: monster at (101,100) walking East → destination (102,100). Kicked South
    /// to (101,101). On the next `on_walk`, the decompile's `Go(102,100,7)` moves NE to
    /// (102,100). The old Rust path stepped East from (101,101) → (102,101) — wrong tile,
    /// wrong animation direction.
    #[test]
    fn pushed_mid_walk_steps_toward_absolute_destination_not_queued_dir() {
        let mut world = support::beat_driven_test_world();
        let start = Position::new(101, 100, 7);
        let dest = Position::new(102, 100, 7);
        let kicked = Position::new(101, 101, 7);
        // All three tiles + the wrong-tile (102,101) must be walkable.
        for &p in &[start, dest, kicked, Position::new(102, 101, 7)] {
            support::ensure_walkable_tile(&mut world.map, p, support::TEST_SYNTHETIC_GROUND_WP);
        }
        let monster = support::insert_monster(&mut world, "Rat", start, 200);

        // Queue an East step with absolute destination (102,100).
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.walk_queue.push_back(Direction::East);
            m.base.walk_destinations.push_back(dest);
            m.base.has_follow_path = true;
            m.base.earliest_walk_server_ms = 0;
            m.base.next_wakeup = Some(world.server_ms + 1);
        }

        // Simulate a kick: relocate the monster South to (101,101).
        world.move_creature_on_map(monster, start, kicked);

        // Trigger the walk — the adjacency check passes (Chebyshev((101,101),(102,100))=1).
        world.on_walk(monster, false, std::time::Instant::now(), None);

        // The decompile's `Go(102,100,7)` moves to the exact destination (102,100).
        // The old Rust path stepped East from (101,101) → (102,101) — wrong.
        assert_eq!(
            world.creatures.get(monster).map(|k| k.position()),
            Some(dest),
            "pushed creature must step toward absolute destination (102,100), \
             not the queued direction from the kicked position (102,101)"
        );
    }

    /// Regression counterpart: when the creature is kicked to exactly its intended
    /// destination, the step is a no-op (the creature is already there). The decompile's
    /// `Go` with Distance==0 calls `::Move` to the same position (a map no-op).
    #[test]
    fn pushed_to_exact_destination_skips_step() {
        let mut world = support::beat_driven_test_world();
        let start = Position::new(101, 100, 7);
        let dest = Position::new(102, 100, 7);
        support::ensure_walkable_tile(&mut world.map, start, support::TEST_SYNTHETIC_GROUND_WP);
        support::ensure_walkable_tile(&mut world.map, dest, support::TEST_SYNTHETIC_GROUND_WP);
        let monster = support::insert_monster(&mut world, "Rat", start, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.walk_queue.push_back(Direction::East);
            m.base.walk_destinations.push_back(dest);
            m.base.has_follow_path = true;
            m.base.earliest_walk_server_ms = 0;
            m.base.next_wakeup = Some(world.server_ms + 1);
        }

        // Kick the creature to exactly its destination.
        world.move_creature_on_map(monster, start, dest);

        world.on_walk(monster, false, std::time::Instant::now(), None);

        // Creature stays at dest — no spurious move in the fallback direction.
        assert_eq!(
            world.creatures.get(monster).map(|k| k.position()),
            Some(dest),
            "creature kicked to its exact destination must not move further"
        );
        // walk_queue was popped; no remaining steps.
        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            base.walk_queue.is_empty(),
            "walk_queue must be popped after same-destination skip"
        );
    }

    #[test]
    fn monster_walk_step_broadcasts_spectator_move() {
        let mut world = support::minimal_world();
        let spectator_pos = Position::new(100, 100, 7);
        let monster_start = Position::new(100, 101, 7);
        let monster_end = Position::new(101, 101, 7);

        support::ensure_walkable_tile(&mut world.map, spectator_pos, 2148);
        support::ensure_walkable_tile(&mut world.map, monster_start, 2148);
        support::ensure_walkable_tile(&mut world.map, monster_end, 2148);

        let conn = ConnId(42);
        support::insert_spectator_player(
            &mut world,
            conn,
            support::test_player("Spectator", spectator_pos),
        );
        let monster = support::insert_monster(&mut world, "Rat", monster_start, 200);
        let wire_id = creature_wire_id(monster, world.creatures.get(monster).unwrap());
        world
            .creature_fully_sent_by_conn
            .entry(conn)
            .or_default()
            .insert(wire_id);

        world.creature_queue_walk_step(monster, Direction::East);

        // Phase 5: both eras schedule steps via the ToDoQueue (`schedule_creature_wakeup`).
        // Advance the logical clock + drain to fire the armed wakeup.
        for _ in 0..32 {
            if world.creatures.get(monster).map(|k| k.position()) == Some(monster_end) {
                break;
            }
            world.server_ms = world.server_ms.saturating_add(200);
            world.drain_todo_queue();
        }

        assert_eq!(
            world.creatures.get(monster).map(|k| k.position()),
            Some(monster_end),
            "monster should have stepped east"
        );

        let packets = world
            .pending_outgoing
            .get(&conn)
            .cloned()
            .unwrap_or_default();
        assert!(
            packets.iter().any(|p| !p.is_empty() && p[0] == 0x6D),
            "spectator should receive 0x6D move packet"
        );
    }

    /// Both-visible spectator move must always use 0x6D (never 0x6C+0x6A), matching
    /// C++ `sendMoveCreature` spectator branch (`protocolgame.cpp:1830-1849`). The
    /// previous `!fully_sent` branch sent remove+appear, recreating the creature
    /// sprite and making name/HP bars blink on every step.
    #[test]
    fn spectator_move_both_visible_always_uses_0x6d() {
        let mut world = support::minimal_world();
        let spectator_pos = Position::new(100, 100, 7);
        let monster_start = Position::new(100, 101, 7);
        let monster_end = Position::new(101, 101, 7);

        support::ensure_walkable_tile(&mut world.map, spectator_pos, 2148);
        support::ensure_walkable_tile(&mut world.map, monster_start, 2148);
        support::ensure_walkable_tile(&mut world.map, monster_end, 2148);

        let conn = ConnId(43);
        support::insert_spectator_player(
            &mut world,
            conn,
            support::test_player("Spectator2", spectator_pos),
        );
        let monster = support::insert_monster(&mut world, "Rat", monster_start, 200);
        // Deliberately do NOT mark fully_sent — 0x6D must still be sent (C++ has no
        // fully_sent gate; the client always knows about visible creatures).

        world.creature_queue_walk_step(monster, Direction::East);
        for _ in 0..32 {
            if world.creatures.get(monster).map(|k| k.position()) == Some(monster_end) {
                break;
            }
            world.server_ms = world.server_ms.saturating_add(200);
            world.drain_todo_queue();
        }

        let packets = world
            .pending_outgoing
            .get(&conn)
            .cloned()
            .unwrap_or_default();
        let opcodes: Vec<u8> = packets
            .iter()
            .filter_map(|p| p.first().copied())
            .collect();
        // C++ always sends 0x6D for both-visible non-teleport moves — no 0x6A appear,
        // no 0x6C remove. A 0x6A would recreate the sprite and blink the HP bar/name.
        assert!(
            opcodes.iter().any(|&o| o == 0x6D),
            "expected 0x6D move for both-visible spectator, got {opcodes:?}"
        );
        assert!(
            !opcodes.iter().any(|&o| o == 0x6A),
            "0x6A appear must not be sent for both-visible move (causes blink), got {opcodes:?}"
        );
    }

    /// Wire ids must be auto-incrementing and never reused (C++ `Monster::setID`,
    /// `monster.h:43-46`). When a monster dies and a new monster spawns at the same
    /// SlotMap slot, they must have different wire ids — otherwise the client caches
    /// the dead creature's outfit under the shared id and shows a stale sprite
    /// (the "dragon skeleton" bug: dead dragon → respawned skeleton at same slot →
    /// client shows dragon sprite with no name/HP).
    #[test]
    fn monster_wire_ids_never_reuse_after_slot_recycle() {
        use crate::login_out::creature_wire_id;
        let mut world = support::minimal_world();
        let pos = Position::new(100, 100, 7);
        support::ensure_walkable_tile(&mut world.map, pos, 2148);

        let dragon = support::insert_monster(&mut world, "Dragon", pos, 200);
        let dragon_wire = creature_wire_id(dragon, world.creatures.get(dragon).unwrap());
        assert!(
            dragon_wire >= 0x4000_0000,
            "monster wire id must start at 0x40000000 (C++ monsterAutoID), got {dragon_wire:#x}"
        );

        // Kill the dragon — frees the SlotMap slot for reuse.
        world.remove_creature(dragon);

        // Spawn a skeleton — may reuse the same SlotMap slot.
        let skeleton = support::insert_monster(&mut world, "Skeleton", pos, 200);
        let skeleton_wire = creature_wire_id(skeleton, world.creatures.get(skeleton).unwrap());

        assert_ne!(
            dragon_wire, skeleton_wire,
            "wire ids must not collide when SlotMap slots are recycled"
        );
        assert!(
            skeleton_wire >= 0x4000_0000,
            "skeleton wire id must also be in monster range, got {skeleton_wire:#x}"
        );
    }

    /// 772 beat loop: walk arms ToDoQueue + `next_wakeup`, not Tokio timers.
    #[test]
    fn beat_driven_walk_schedules_todo_queue_not_tokio() {
        let mut world = support::beat_driven_world();
        world.server_ms = 0;
        let pos = Position::new(100, 100, 7);
        support::ensure_walkable_tile(&mut world.map, pos, 2148);
        let cid = support::insert_monster(&mut world, "Rat", pos, 200);
        world.creature_queue_walk_step(cid, Direction::North);
        let entry = world.todo_queue.peek().expect("todo entry");
        assert_eq!(
            world.creatures.get(cid).unwrap().base().next_wakeup,
            Some(entry.execution_time)
        );
        assert!(
            world
                .creatures
                .get(cid)
                .unwrap()
                .base()
                .next_wakeup
                .is_some(),
            "Phase 5: walk arms next_wakeup on the ToDoQueue (no Tokio timers)"
        );
    }

    /// Stale heap entries are skipped when `next_wakeup` was cleared.
    #[test]
    fn beat_driven_stale_todo_entry_is_skipped() {
        let mut world = support::beat_driven_world();
        let pos = Position::new(100, 100, 7);
        support::ensure_walkable_tile(&mut world.map, pos, 2148);
        let cid = support::insert_monster(&mut world, "Rat", pos, 200);
        world.todo_queue.insert(200, cid);
        world.stop_event_walk(cid);
        world.advance_beat(200);
        assert!(world.todo_queue.is_empty());
        assert_eq!(world.creatures.get(cid).unwrap().position(), pos);
    }

    /// N2 / F1 regression: a hard-blocked monster (only path step lands on a missing tile) must
    /// full-clear the ToDo queue and re-arm `IdleStimulus` on the next beat via `ToDoYield`
    /// (`cract.cc:870-889`). Asserts the post-RC1 architecture: no think-sweep safety net, so the
    /// `IdleStimulus` re-arm is the sole guardian of chase continuity.
    ///
    /// Drives `on_walk` directly (the function that owns the Err arm) so the assertion observes the
    /// Err-arm + `request_idle_stimulus` contract in isolation — the full `process_creature_todo`
    /// drain would then consume the yield `Wait(0)` in `finish_creature_todo_execute`'s recursive
    /// `run_monster_todo_execute` and run `idle_stimulus`, masking the F1 contract.
    #[test]
    fn hard_block_reruns_idle_next_beat() {
        use crate::creature_todo::CreatureAction;

        let mut world = support::beat_driven_world();
        world.server_ms = 100;

        let pos = Position::new(100, 100, 7);
        // Destination tile (100, 99, 7) is intentionally NOT created → `internal_move_creature_step`
        // returns `Err(NotPossible)` (`walk/mod.rs` ~1588-1590) → hard-block Err arm.
        support::ensure_walkable_tile(&mut world.map, pos, 2148);
        let cid = support::insert_monster(&mut world, "Rat", pos, 200);

        // Mirror the state `execute_creature_todo_action` leaves for `on_walk`: the chase `Go` is
        // still queued (Err arm clears it), `locked` is set, a North step is queued, and
        // `next_wakeup` was already taken by `process_creature_todo`'s entry.
        {
            let m = world.creatures.get_mut(cid).unwrap();
            let base = m.base_mut();
            base.todo.queue.push_back(CreatureAction::Go);
            base.todo.locked = true;
            base.walk_queue.push_back(Direction::North);
            base.next_wakeup = None; // taken at `process_creature_todo` entry
            base.last_step_server_ms = None; // → `get_walk_delay_logical` returns 0 → step runs now
        }

        world.on_walk(cid, false, std::time::Instant::now(), None);

        let base = world.creatures.get(cid).unwrap().base();
        // Chase Go cleared; a yield `Wait(0)` is the only queued entry.
        assert!(
            !base
                .todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Go)),
            "hard-block Err arm must clear the chase Go (ToDoClear cract.cc:871)"
        );
        assert!(
            base.todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Wait { deadline_ms } if *deadline_ms == 100)),
            "creature_todo_yield must enqueue ToDoWait(0) at absolute deadline=server_ms (cract.cc:1001)"
        );
        // ToDoClear unlocks, then ToDoWait+ToDoStart re-locks the yield batch (`cract.cc:1012`).
        assert!(
            base.todo.locked,
            "ToDoStart after yield must set LockToDo for the Wait batch"
        );
        // IdleStimulus re-armed for the next beat.
        assert_eq!(
            base.next_wakeup,
            Some(world.server_ms + 1),
            "hard-blocked monster must re-run IdleStimulus at server_ms + 1 (ToDoStart clamps Delay<1 to 1)"
        );
        // The blocked North step was consumed (popped before the failed move).
        assert!(
            base.walk_queue.is_empty(),
            "blocked step direction must be popped"
        );
    }

    /// N2 / F1 regression (player analogue): a blocked player walk with no attack target stops
    /// cleanly — ToDo queue cleared, no walk re-arm. Phase 1.3 widened the Err arm to all
    /// todo-execute creatures (`crplayer.cc:388-405`); this test pins the player seam.
    #[test]
    fn hard_block_player_stops_cleanly() {
        use crate::creature_todo::CreatureAction;

        let mut world = support::beat_driven_world();
        world.server_ms = 100;

        let pos = Position::new(100, 100, 7);
        // Destination tile (100, 99, 7) is intentionally missing → Err(NotPossible).
        support::ensure_walkable_tile(&mut world.map, pos, 2148);
        let cid = support::insert_player(&mut world, support::test_player("Hero", pos));

        {
            let p = world.creatures.get_mut(cid).unwrap();
            let base = p.base_mut();
            base.todo.queue.push_back(CreatureAction::Go);
            base.todo.locked = true;
            base.walk_queue.push_back(Direction::North);
            base.next_wakeup = None;
            base.last_step_server_ms = None;
        }

        world.on_walk(cid, false, std::time::Instant::now(), None);

        let p = world.creatures.get(cid).unwrap();
        let base = p.base();
        assert!(
            !base
                .todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Go)),
            "blocked player walk must clear the Go (ToDoClear cract.cc:871)"
        );
        assert!(
            base.todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Wait { deadline_ms } if *deadline_ms == 100)),
            "player Err arm must yield ToDoWait(0) at absolute deadline=server_ms (cract.cc:1001)"
        );
        // ToDoClear unlocks, then ToDoWait+ToDoStart re-locks the yield batch (`cract.cc:1012`).
        assert!(
            base.todo.locked,
            "ToDoStart after yield must set LockToDo for the Wait batch"
        );
        // Player stops: no walk direction remains, no walk_action re-arm.
        assert!(
            base.walk_queue.is_empty(),
            "blocked player step must be popped"
        );
        if let crate::creature::CreatureKind::Player(pl) = world.creatures.get(cid).unwrap() {
            assert!(
                pl.walk_action.is_none(),
                "blocked player with no attack target must not re-arm a walk"
            );
        }
        // Yield armed for the next beat (IdleStimulus re-arm seam).
        assert_eq!(
            base.next_wakeup,
            Some(world.server_ms + 1),
            "player Err arm must re-arm IdleStimulus at server_ms + 1"
        );
    }
}
