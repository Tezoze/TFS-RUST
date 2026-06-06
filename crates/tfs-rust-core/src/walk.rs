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
//! `next_walk_check` stores the **logical** deadline. Initial arms from a new move use `walk_sched_base`;
//! reschedules after a step match C++ `addEventWalk` by anchoring to `Instant::now()` at reschedule time
//! (`tasks/walk-audit.md` Issue 3).
//!
//! **Scheduling:** When the world has `walk_wake_tx` set, each
//! `next_walk_check` arms a one-shot `tokio::time::sleep_until` (`src/scheduler.cpp` `steady_timer` +
//! `async_wait` → `g_dispatcher.addTask`). Without it, [`Self::process_walk_deadlines`] polls deadlines
//! (tests / fallback).

use std::time::{Duration, Instant};

/// TFS has no grace: timer fires → `onWalk` runs. Tokio may wake a hair early; 0ms avoids re-queue loops
/// (`tasks/walk-audit.md` Issue 4).
const WALK_DEADLINE_GRACE: Duration = Duration::ZERO;

use rand::thread_rng;
use tfs_rust_common::enums::{ConditionType, Direction, SpeakType};
use tfs_rust_common::Position;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_net::map_description::{send_map_description_packet, send_move_creature_player, send_move_creature_spectator, TileContent};
use tfs_rust_net::outgoing_extra::send_text_message_simple;

use crate::combat::uniform_random;
use crate::condition::ConditionData;
use crate::return_value::ReturnValue;
use crate::creature::CreatureKind;
use crate::game_world::{DeferredTurnBroadcast, GameWorld};
use crate::ids::CreatureId;
use crate::login_out::{creature_wire_id, map_tile_content};
use crate::map::Map;
use crate::tile::client_creature_stack_pos;
use tfs_rust_common::ConnId;

/// C++ `cylinder.h` — `Tile::queryAdd` / `internalMoveCreature` flags.
const FLAG_NOLIMIT: u32 = 1 << 0;
pub(crate) const FLAG_IGNOREBLOCKITEM: u32 = 1 << 1;
const FLAG_IGNOREBLOCKCREATURE: u32 = 1 << 2;
const FLAG_PATHFINDING: u32 = 1 << 4;
const FLAG_IGNOREFIELDDAMAGE: u32 = 1 << 5;

/// Pathfinding query flags — `Map::canWalkTo` (`map.cpp` ~638).
pub(crate) const PATHFIND_WALK_FLAGS: u32 = FLAG_PATHFINDING | FLAG_IGNOREFIELDDAMAGE;

/// One movement segment emitted by `internal_move_creature_step`.
/// C++ `map.moveCreature` emits a packet per call; we collect segments and emit afterwards.
struct MoveSegment {
    from: Position,
    to: Position,
    old_stack: i32,
    /// C++ `Map::moveCreature`: `teleport = forceTeleport || !ground || !areInRange<1,1,0>`
    teleport: bool,
}

/// C++ `Position::areInRange<1,1,0>` — dx<=1, dy<=1, dz==0.
fn are_in_range_1_1_0(a: Position, b: Position) -> bool {
    let dx = (a.x as i32 - b.x as i32).unsigned_abs();
    let dy = (a.y as i32 - b.y as i32).unsigned_abs();
    let dz = (a.z as i32 - b.z as i32).unsigned_abs();
    dx <= 1 && dy <= 1 && dz == 0
}

use crate::tile::flags as tilestate;

const SPEED_A: f64 = 857.36;
const SPEED_B: f64 = 261.29;
const SPEED_C: f64 = -4795.01;

/// TFS `Player::getStepSpeed` clamp (`player.h` `PLAYER_MIN_SPEED` / `PLAYER_MAX_SPEED`).
const PLAYER_MIN_SPEED: i32 = 10;
const PLAYER_MAX_SPEED: i32 = 1500;


/// TFS `Creature::getSpeed` — `baseSpeed + varSpeed` from conditions (`creature.h`); step uses `getStepSpeed` clamp.
fn creature_effective_speed_for_step(base: &crate::creature::CreatureBase) -> i32 {
    let mut s = base.speed;
    for c in &base.active_conditions {
        if let ConditionData::Speed { flat_delta } = c.data {
            s += flat_delta;
        }
    }
    s
}

/// TFS `getStepSpeed` — `Player::getStepSpeed` clamp vs base `Creature::getStepSpeed` (`creature.h`, `player.h`).
fn step_speed_for_walk(
    kind: &CreatureKind,
    base: &crate::creature::CreatureBase,
    mech: &crate::formulas::Mechanics,
) -> i32 {
    let role = match kind {
        CreatureKind::Player(_) => WalkSpeedRole::Player,
        CreatureKind::Monster(_) | CreatureKind::Npc(_) => WalkSpeedRole::MonsterOrNpc,
    };
    go_strength_for_walk(role, base, mech)
}

/// Player vs non-player for walk speed (mirrors `step_speed_for_walk` without a full `CreatureKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WalkSpeedRole {
    Player,
    MonsterOrNpc,
}

fn go_strength_for_walk(
    role: WalkSpeedRole,
    base: &crate::creature::CreatureBase,
    mech: &crate::formulas::Mechanics,
) -> i32 {
    let raw = creature_effective_speed_for_step(base);
    match role {
        WalkSpeedRole::Player => match mech.profile.step_speed {
            // TFS `Player::getStepSpeed` clamps linear speed (`src/player.h`).
            crate::formulas::StepSpeedModel::TfsLog => raw.clamp(PLAYER_MIN_SPEED, PLAYER_MAX_SPEED),
            // 772 wire + walk GoStrength (`baseSpeed`); effective `GetSpeed` is for server timers only.
            crate::formulas::StepSpeedModel::CipSoft => raw.max(0),
        },
        WalkSpeedRole::MonsterOrNpc => raw,
    }
}

/// Speed for walk timers and protocol `step_speed` (GoStrength vs CipSoft `GetSpeed`).
fn walk_timing_speed(
    role: WalkSpeedRole,
    base: &crate::creature::CreatureBase,
    mech: &crate::formulas::Mechanics,
) -> i32 {
    let go = go_strength_for_walk(role, base, mech);
    match mech.profile.step_speed {
        crate::formulas::StepSpeedModel::CipSoft => cipsoft_speed_from_profile(go, mech),
        crate::formulas::StepSpeedModel::TfsLog => go,
    }
}

fn tfs_retail_log_speed(go: i32) -> i32 {
    if (go as f64) <= -SPEED_B {
        return 1;
    }
    let half = (go / 2) as f64;
    let raw = SPEED_A * (half + SPEED_B).ln() + SPEED_C;
    (raw + 0.5).floor() as i32
}

fn balanced_softened_go(go: i32) -> i32 {
    let anchor = 320;
    if go <= anchor {
        return go.max(1);
    }
    let scale = 100.0;
    let divisor = 120.0;
    let x = (go - anchor) as f64;
    (anchor as f64 + scale * (1.0 + x / divisor).ln()).floor() as i32
}

fn cipsoft_speed_from_profile(go: i32, mech: &crate::formulas::Mechanics) -> i32 {
    use crate::formulas::PlayerSpeedModel;
    match mech.profile.player_speed_model {
        PlayerSpeedModel::EraDefault | PlayerSpeedModel::Classic772 => {
            crate::formulas::cipsoft_effective_speed(go)
        }
        PlayerSpeedModel::Retail1098 => tfs_retail_log_speed(go).max(1),
        PlayerSpeedModel::BalancedLog => crate::formulas::cipsoft_effective_speed(balanced_softened_go(go)),
    }
}

fn walk_timing_speed_kind(
    kind: &CreatureKind,
    base: &crate::creature::CreatureBase,
    mech: &crate::formulas::Mechanics,
) -> i32 {
    let role = match kind {
        CreatureKind::Player(_) => WalkSpeedRole::Player,
        CreatureKind::Monster(_) | CreatureKind::Npc(_) => WalkSpeedRole::MonsterOrNpc,
    };
    walk_timing_speed(role, base, mech)
}

/// Protocol `AddCreature` speed byte(s).
///
/// - **1098** — clamped GoStrength; codec halves on wire (`getStepSpeed()/2`, `protocolgame.cpp`).
/// - **772 players** — GoStrength on wire (220 at level 1); OTC animates from this, not `2×go+80`.
/// - **772 monsters/NPCs** — full `getStepSpeed()` / `getSpeed()` (`gameserver` `AddCreature`), e.g. wolf
///   GoStrength 42 → wire **164**.
///
/// Server walk timers always use [`walk_timing_speed`] (effective speed on 772).
pub(crate) fn wire_step_speed(
    role: WalkSpeedRole,
    base: &crate::creature::CreatureBase,
    mech: &crate::formulas::Mechanics,
) -> u16 {
    let wire = match (mech.profile.step_speed, role) {
        (crate::formulas::StepSpeedModel::CipSoft, WalkSpeedRole::Player) => {
            go_strength_for_walk(role, base, mech)
        }
        (crate::formulas::StepSpeedModel::CipSoft, WalkSpeedRole::MonsterOrNpc) => {
            walk_timing_speed(role, base, mech)
        }
        (crate::formulas::StepSpeedModel::TfsLog, _) => go_strength_for_walk(role, base, mech),
    };
    wire.max(0).min(u16::MAX as i32) as u16
}

fn has_drunk_condition(base: &crate::creature::CreatureBase) -> bool {
    base.active_conditions
        .iter()
        .any(|c| c.ctype == ConditionType::Drunk)
}

/// TFS `Creature::onWalk(Direction&)` (`creature.cpp` ~236–248): `hasCondition(CONDITION_DRUNK)`,
/// `uniform_random(0,399)`, `rand/4 > getDrunkenness()` early out, else `dir = rand%4` cardinal;
/// caller sends `internalCreatureSay(..., "Hicks!")` when `Some`.
fn try_drunk_walk_direction(base: &crate::creature::CreatureBase) -> Option<Direction> {
    if !has_drunk_condition(base) {
        return None;
    }
    let d = base.drunkenness;
    let r = uniform_random(&mut thread_rng(), 0, 399) as u32;
    if r / 4 > d {
        return None;
    }
    Some(match r % 4 {
        0 => Direction::North,
        1 => Direction::East,
        2 => Direction::South,
        _ => Direction::West,
    })
}

/// `MESSAGE_STATUS_SMALL` (`src/const.h`).
const MESSAGE_STATUS_SMALL: u8 = 21;

/// TFS `Creature::getStepDuration()` — `creature.cpp` (uses `floor((A*log(...) + C) + 0.5)` and integer `stepSpeed/2`).
fn calculated_step_speed_tfs(step_speed: i32) -> u32 {
    if (step_speed as f64) <= -SPEED_B {
        return 1;
    }
    // C++ uses integer division: `(stepSpeed / 2) + speedB` inside `log`.
    let half = (step_speed / 2) as f64;
    let raw = SPEED_A * (half + SPEED_B).ln() + SPEED_C;
    let cs = (raw + 0.5).floor() as i32;
    cs.max(1) as u32
}

fn get_step_duration(
    kind: &CreatureKind,
    base: &crate::creature::CreatureBase,
    ground_speed: u32,
    mech: &crate::formulas::Mechanics,
) -> i64 {
    if base.health <= 0 {
        return 0;
    }
    let go = step_speed_for_walk(kind, base, mech);
    let gs = if ground_speed == 0 { 150 } else { ground_speed };

    // Tier-2 override (`getStepDuration(speed, ground)`) — returns the base per-tile duration; the
    // engine still applies the diagonal / `last_step_cost` multiplier. Native fast path when unset.
    if let Some(ms) = mech.hooks.step_duration(go, gs as i32, false) {
        return ms.max(1);
    }

    let beat = mech.profile.step_beat_ms.max(1) as i64;
    match mech.profile.step_speed {
        crate::formulas::StepSpeedModel::CipSoft => {
            // `cract.cc:1461` — `Delay = (Waypoints * 1000) / GetSpeed()`, ceil to `Beat`.
            let eff = cipsoft_speed_from_profile(go, mech);
            let delay = (gs as i64 * 1000) / i64::from(eff.max(1));
            ((delay + beat - 1) / beat) * beat
        }
        crate::formulas::StepSpeedModel::TfsLog => {
            let calculated_step_speed = calculated_step_speed_tfs(go);
            let duration = (1000.0 * gs as f64 / calculated_step_speed as f64).floor();
            let beat_f = beat as f64;
            ((duration / beat_f).ceil() * beat_f) as i64
        }
    }
}

/// Step duration for `next_action_until` — diagonal steps use `× last_step_cost` (3), matching
/// TFS `Creature::getStepDuration` / `Player::nextAction` (`creature.cpp`, `player.cpp`).
fn get_step_duration_ms_with_direction(
    kind: &CreatureKind,
    base: &crate::creature::CreatureBase,
    direction: Direction,
    ground_speed: u32,
    mech: &crate::formulas::Mechanics,
) -> i64 {
    let mut ms = get_step_duration(kind, base, ground_speed, mech);
    if matches!(
        direction,
        Direction::NorthEast | Direction::NorthWest | Direction::SouthEast | Direction::SouthWest
    ) {
        ms *= 3;
    }
    ms
}

/// TFS `Creature::onCreatureMove` — `lastStepCost` (`creature.cpp` ~489–499).
fn last_step_cost_for_move(old_pos: Position, new_pos: Position) -> u32 {
    if old_pos.z != new_pos.z {
        2
    } else if (old_pos.x as i32 - new_pos.x as i32).abs() >= 1
        && (old_pos.y as i32 - new_pos.y as i32).abs() >= 1
    {
        3
    } else {
        1
    }
}

fn get_walk_delay(
    kind: &CreatureKind,
    base: &crate::creature::CreatureBase,
    now: Instant,
    mech: &crate::formulas::Mechanics,
) -> i64 {
    let Some(last) = base.last_step else {
        return 0;
    };
    let elapsed = now.saturating_duration_since(last);
    let gs = base.last_step_ground_speed;
    let gs = if gs == 0 { 150 } else { gs };
    let step_duration =
        get_step_duration(kind, base, gs, mech).saturating_mul(base.last_step_cost as i64);
    let delay = step_duration - elapsed.as_millis() as i64;
    delay.max(0)
}

fn get_event_step_ticks(
    kind: &CreatureKind,
    base: &crate::creature::CreatureBase,
    only_delay: bool,
    ground_speed_next: u32,
    now: Instant,
    mech: &crate::formulas::Mechanics,
) -> i64 {
    let walk_delay = get_walk_delay(kind, base, now, mech);
    if walk_delay > 0 {
        return walk_delay;
    }
    let step_duration = get_step_duration(kind, base, ground_speed_next, mech);
    // TFS `getEventStepTicks(onlyDelay)` returns `1` when `getWalkDelay() <= 0` (`creature.cpp` ~1536–1546).
    if only_delay && step_duration > 0 {
        1
    } else {
        step_duration * base.last_step_cost as i64
    }
}

fn ground_speed_for_tile_body(body: &crate::tile::TileBody, items_db: &ItemDatabase) -> u32 {
    let Some(gid) = body.ground else {
        return 150;
    };
    items_db.ground_speed_for_item(gid)
}

#[inline]
fn is_diagonal(direction: Direction) -> bool {
    matches!(
        direction,
        Direction::NorthEast | Direction::NorthWest | Direction::SouthEast | Direction::SouthWest
    )
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

/// TFS `Tile::hasHeight(n)` (`src/tile.cpp` ~62–87) — nth item with `CONST_PROP_HASHEIGHT` along stack.
fn tile_has_height_n(
    pos: Position,
    body: &crate::tile::TileBody,
    items_db: &ItemDatabase,
    items: &slotmap::SlotMap<crate::ids::ItemId, crate::item::Item>,
    n: u32,
) -> bool {
    let mut height = 0u32;
    tracing::debug!(
        "tile_has_height_n: checking tile at {:?}, ground: {:?}, down_items: {:?}, top_items: {:?}",
        pos,
        body.ground,
        body.down_items,
        body.top_items
    );

    if let Some(gid) = body.ground {
        let has_height = items_db.items.get(&gid).is_some_and(|t| t.has_height());
        tracing::debug!("tile_has_height_n: ground item {} has_height: {} at {:?}", gid, has_height, pos);
        if has_height {
            height += 1;
            if height == n {
                return true;
            }
        }
    }
    for &item_id in &body.down_items {
        if let Some(item) = items.get(item_id) {
            let has_height = items_db.items.get(&item.item_type).is_some_and(|t| t.has_height());
            tracing::debug!("tile_has_height_n: down item {:?} (type {}) has_height: {} at {:?}", item_id, item.item_type, has_height, pos);
            if has_height {
                height += 1;
                if height == n {
                    return true;
                }
            }
        }
    }
    for &item_id in &body.top_items {
        if let Some(item) = items.get(item_id) {
            let has_height = items_db.items.get(&item.item_type).is_some_and(|t| t.has_height());
            tracing::debug!("tile_has_height_n: top item {:?} (type {}) has_height: {} at {:?}", item_id, item.item_type, has_height, pos);
            if has_height {
                height += 1;
                if height == n {
                    return true;
                }
            }
        }
    }
    tracing::debug!("tile_has_height_n: total height {} at {:?}, needed {}", height, pos, n);
    false
}

#[inline]
fn tile_is_hole_like(body: &crate::tile::TileBody) -> bool {
    body.ground.is_none() && (body.flags & tilestate::BLOCKSOLID) == 0
}

/// TFS `Game::internalMoveCreature(Creature*, Direction, flags)` — height-based floor change
/// (`game.cpp` ~804–834). Only runs for cardinal (non-diagonal) player moves.
/// C++ ref: src/game.cpp:797-841
fn resolve_player_move_destination(
    map: &Map,
    items_db: &ItemDatabase,
    items: &slotmap::SlotMap<crate::ids::ItemId, crate::item::Item>,
    current_pos: Position,
    direction: Direction,
    mut flags: u32,
) -> (Position, u32) {
    let mut dest_pos = current_pos.offset(direction);
    if is_diagonal(direction) {
        return (dest_pos, flags);
    }

    // C++ ref: src/game.cpp:807-820 — try to go up
    if current_pos.z != 8 {
        if let Some(cur_tile) = map.get_tile(current_pos) {
            let has_h3 = tile_has_height_n(current_pos, cur_tile.body(), items_db, items, 3);
            if has_h3 {
                let z_above = current_pos.z.wrapping_sub(1);
                let tmp = map.get_tile(Position { x: current_pos.x, y: current_pos.y, z: z_above });
                let open = tmp.map(|t| tile_is_hole_like(t.body())).unwrap_or(true);
                if open {
                    let tmp2 = map.get_tile(Position { x: dest_pos.x, y: dest_pos.y, z: z_above });
                    if let Some(tt) = tmp2 {
                        let tb = tt.body();
                        if tb.ground.is_some() && (tb.flags & tilestate::IMMOVABLEBLOCKSOLID) == 0 {
                            flags |= FLAG_IGNOREBLOCKITEM | FLAG_IGNOREBLOCKCREATURE;
                            if (tb.flags & tilestate::FLOORCHANGE) == 0 {
                                dest_pos.z = z_above;
                            }
                        }
                    }
                }
            }
        }
    }

    // C++ ref: src/game.cpp:823-833 — try to go down
    if current_pos.z != 7 && current_pos.z == dest_pos.z {
        let tmp = map.get_tile(dest_pos);
        let open = tmp.map(|t| tile_is_hole_like(t.body())).unwrap_or(true);
        if open {
            let z_below = dest_pos.z.wrapping_add(1);
            if let Some(tt) = map.get_tile(Position { x: dest_pos.x, y: dest_pos.y, z: z_below }) {
                let tb = tt.body();
                if tile_has_height_n(
                    Position { x: dest_pos.x, y: dest_pos.y, z: z_below },
                    tb,
                    items_db,
                    items,
                    3,
                ) && (tb.flags & tilestate::IMMOVABLEBLOCKSOLID) == 0
                {
                    flags |= FLAG_IGNOREBLOCKITEM | FLAG_IGNOREBLOCKCREATURE;
                    dest_pos.z = z_below;
                }
            }
        }
    }

    (dest_pos, flags)
}

/// TFS `Tile::queryDestination` — flag-based floor change after creature has landed on a tile.
/// Called in a while-loop by `internalMoveCreature(Creature&, Tile&, flags)`.
/// C++ ref: src/tile.cpp:735-830
fn query_destination(
    map: &Map,
    tile_pos: Position,
    tile_flags: u32,
) -> Option<(Position, u32)> {
    if tile_flags & tilestate::FLOORCHANGE_DOWN != 0 {
        // C++ ref: src/tile.cpp:740-784
        let mut dx = tile_pos.x;
        let mut dy = tile_pos.y;
        let dz = tile_pos.z.wrapping_add(1);

        // Check south-alt first
        if let Some(south_down) = map.get_tile(Position { x: dx, y: dy.wrapping_sub(1), z: dz }) {
            if south_down.body().flags & tilestate::FLOORCHANGE_SOUTH_ALT != 0 {
                dy = dy.wrapping_sub(2);
                let dest = map.get_tile(Position { x: dx, y: dy, z: dz });
                return dest.map(|_| (Position { x: dx, y: dy, z: dz }, FLAG_NOLIMIT));
            }
        }

        // Check east-alt
        if let Some(east_down) = map.get_tile(Position { x: dx.wrapping_sub(1), y: dy, z: dz }) {
            if east_down.body().flags & tilestate::FLOORCHANGE_EAST_ALT != 0 {
                dx = dx.wrapping_sub(2);
                let dest = map.get_tile(Position { x: dx, y: dy, z: dz });
                return dest.map(|_| (Position { x: dx, y: dy, z: dz }, FLAG_NOLIMIT));
            }
        }

        // Normal directional check on the tile below
        if let Some(down_tile) = map.get_tile(Position { x: dx, y: dy, z: dz }) {
            let df = down_tile.body().flags;
            if df & tilestate::FLOORCHANGE_NORTH != 0 { dy = dy.wrapping_add(1); }
            if df & tilestate::FLOORCHANGE_SOUTH != 0 { dy = dy.wrapping_sub(1); }
            if df & tilestate::FLOORCHANGE_SOUTH_ALT != 0 { dy = dy.wrapping_sub(2); }
            if df & tilestate::FLOORCHANGE_EAST != 0 { dx = dx.wrapping_sub(1); }
            if df & tilestate::FLOORCHANGE_EAST_ALT != 0 { dx = dx.wrapping_sub(2); }
            if df & tilestate::FLOORCHANGE_WEST != 0 { dx = dx.wrapping_add(1); }
        }

        let dest = map.get_tile(Position { x: dx, y: dy, z: dz });
        return dest.map(|_| (Position { x: dx, y: dy, z: dz }, FLAG_NOLIMIT));
    }

    // C++ ref: src/tile.cpp:785-814 — upward floor change (any non-DOWN floorchange flag)
    if tile_flags & tilestate::FLOORCHANGE != 0 {
        let mut dx = tile_pos.x;
        let mut dy = tile_pos.y;
        let dz = tile_pos.z.wrapping_sub(1);

        if tile_flags & tilestate::FLOORCHANGE_NORTH != 0 { dy = dy.wrapping_sub(1); }
        if tile_flags & tilestate::FLOORCHANGE_SOUTH != 0 { dy = dy.wrapping_add(1); }
        if tile_flags & tilestate::FLOORCHANGE_EAST != 0 { dx = dx.wrapping_add(1); }
        if tile_flags & tilestate::FLOORCHANGE_WEST != 0 { dx = dx.wrapping_sub(1); }
        if tile_flags & tilestate::FLOORCHANGE_SOUTH_ALT != 0 { dy = dy.wrapping_add(2); }
        if tile_flags & tilestate::FLOORCHANGE_EAST_ALT != 0 { dx = dx.wrapping_add(2); }

        let dest = map.get_tile(Position { x: dx, y: dy, z: dz });
        return dest.map(|_| (Position { x: dx, y: dy, z: dz }, FLAG_NOLIMIT));
    }

    None
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
        .map(|t| client_creature_stack_pos(t.body(), cid))
        .filter(|s| *s >= 0)
        .unwrap_or(1);

    world.move_creature_on_map(cid, old_pos, new_pos);
    if let Some(k) = world.creatures.get_mut(cid) {
        k.set_position(new_pos);
    }

    world.emit_teleport_move_packet(cid, conn_id, old_pos, new_pos, old_stack);
    ReturnValue::NoError
}

/// Whether `cid` can stand on `pos` during pathfinding (`Map::canWalkTo` / `Tile::queryAdd`).
pub(crate) fn creature_can_stand_for_pathfind(
    world: &GameWorld,
    cid: CreatureId,
    pos: Position,
) -> bool {
    let Some(tile) = world.map.get_tile(pos) else {
        return false;
    };
    tile_query_add_creature(world, tile, cid, PATHFIND_WALK_FLAGS) == ReturnValue::NoError
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

/// TFS `Tile::queryAdd` monster branch (`tile.cpp` ~499–563).
fn tile_query_add_monster(
    world: &GameWorld,
    tile: &crate::tile::Tile,
    mover: CreatureId,
    flags: u32,
) -> ReturnValue {
    let body = tile.body();

    if (flags & FLAG_NOLIMIT) != 0 {
        return ReturnValue::NoError;
    }

    if body.ground.is_none() {
        return ReturnValue::NotPossible;
    }

    if (flags & FLAG_PATHFINDING) != 0
        && (body.flags & (tilestate::FLOORCHANGE | tilestate::TELEPORT)) != 0
    {
        return ReturnValue::NotPossible;
    }

    if (body.flags & (tilestate::PROTECTIONZONE | tilestate::FLOORCHANGE | tilestate::TELEPORT)) != 0 {
        return ReturnValue::NotPossible;
    }

    // `canpushcreatures` / `canpushitems` from monster type at spawn.
    let (can_push_creatures, can_push_items, is_summon) = match world.creatures.get(mover) {
        Some(CreatureKind::Monster(m)) => (m.can_push_creatures, m.can_push_items, m.base.is_summon()),
        _ => (false, false, false),
    };

    if (flags & FLAG_IGNOREBLOCKCREATURE) == 0 {
        if can_push_creatures && !is_summon {
            for &tile_c in &body.creatures {
                if tile_c == mover {
                    continue;
                }
                let other_ghost = world.creatures.get(tile_c).is_some_and(|k| {
                    matches!(k, CreatureKind::Player(p) if p.ghost_mode)
                });
                if other_ghost {
                    continue;
                }
                let Some(other) = world.creatures.get(tile_c) else {
                    return ReturnValue::NotPossible;
                };
                let other_monster_pushable = matches!(other, CreatureKind::Monster(_));
                let other_summon_with_player_master = other.is_summon()
                    && other
                        .base()
                        .master
                        .and_then(|mid| world.creatures.get(mid))
                        .is_some_and(|m| matches!(m, CreatureKind::Player(_)));
                if !other_monster_pushable || other_summon_with_player_master {
                    return ReturnValue::NotPossible;
                }
            }
        } else if !body.creatures.is_empty() {
            for &tile_c in &body.creatures {
                if tile_c == mover {
                    continue;
                }
                let other_ghost = world.creatures.get(tile_c).is_some_and(|k| {
                    matches!(k, CreatureKind::Player(p) if p.ghost_mode)
                });
                if !other_ghost {
                    return ReturnValue::NotEnoughRoom;
                }
            }
        }
    }

    if (body.flags & tilestate::IMMOVABLEBLOCKSOLID) != 0 {
        return ReturnValue::NotPossible;
    }

    if (flags & FLAG_PATHFINDING) != 0 && (body.flags & tilestate::IMMOVABLENOFIELDBLOCKPATH) != 0 {
        return ReturnValue::NotPossible;
    }

    if ((body.flags & tilestate::BLOCKSOLID) != 0
        || ((flags & FLAG_PATHFINDING) != 0 && (body.flags & tilestate::NOFIELDBLOCKPATH) != 0))
        && !(can_push_items || (flags & FLAG_IGNOREBLOCKITEM) != 0) {
            return ReturnValue::NotPossible;
        }

    // Full field immunity deferred until Monster combat fields land; block damaging fields without ignore flag.
    if (body.flags & tilestate::MAGICFIELD) != 0 && (flags & FLAG_IGNOREFIELDDAMAGE) == 0 {
        return ReturnValue::NotPossible;
    }

    ReturnValue::NoError
}

/// TFS `Tile::queryAdd` NPC / generic creature branch (`tile.cpp` ~598–628); NPCs cannot enter houses or PZ.
fn tile_query_add_npc(
    world: &GameWorld,
    tile: &crate::tile::Tile,
    mover: CreatureId,
    flags: u32,
) -> ReturnValue {
    if matches!(tile, crate::tile::Tile::House(_)) {
        return ReturnValue::NotPossible;
    }

    let body = tile.body();

    if (flags & FLAG_NOLIMIT) != 0 {
        return ReturnValue::NoError;
    }

    if body.ground.is_none() {
        return ReturnValue::NotPossible;
    }

    if (flags & FLAG_PATHFINDING) != 0
        && (body.flags & (tilestate::FLOORCHANGE | tilestate::TELEPORT)) != 0
    {
        return ReturnValue::NotPossible;
    }

    if (body.flags & tilestate::PROTECTIONZONE) != 0 {
        return ReturnValue::NotPossible;
    }

    if (flags & FLAG_IGNOREBLOCKCREATURE) == 0 && !body.creatures.is_empty() {
        for &tile_c in &body.creatures {
            if tile_c == mover {
                continue;
            }
            let other_ghost = world.creatures.get(tile_c).is_some_and(|k| {
                matches!(k, CreatureKind::Player(p) if p.ghost_mode)
            });
            if !other_ghost {
                return ReturnValue::NotEnoughRoom;
            }
        }
    }

    if (flags & FLAG_IGNOREBLOCKITEM) == 0 {
        if (body.flags & tilestate::BLOCKSOLID) != 0 {
            return ReturnValue::NotEnoughRoom;
        }
        if (flags & FLAG_PATHFINDING) != 0 && (body.flags & tilestate::NOFIELDBLOCKPATH) != 0 {
            return ReturnValue::NotPossible;
        }
    } else if let Some(ground_id) = body.ground {
        if let Some(gt) = world.items_db.items.get(&ground_id) {
            if gt.block_solid() && !gt.moveable() {
                return ReturnValue::NotPossible;
            }
        }
        for &item_id in body.top_items.iter().chain(body.down_items.iter()) {
            if let Some(item) = world.items.get(item_id) {
                if let Some(it) = world.items_db.items.get(&item.item_type) {
                    if it.block_solid() && !it.moveable() {
                        return ReturnValue::NotPossible;
                    }
                }
            }
        }
    }

    ReturnValue::NoError
}

/// TFS `Tile::queryAdd` for player creatures.
/// C++ ref: src/tile.cpp:484-628
fn tile_query_add_player(world: &GameWorld, tile: &crate::tile::Tile, mover: CreatureId, flags: u32) -> ReturnValue {
    let body = tile.body();

    // C++ ref: src/tile.cpp:487-488 — FLAG_NOLIMIT bypasses all checks.
    if (flags & FLAG_NOLIMIT) != 0 {
        return ReturnValue::NoError;
    }

    if body.ground.is_none() {
        return ReturnValue::NotPossible;
    }

    // C++ ref: src/tile.cpp:491-493 — skip floor-change / teleport tiles while pathfinding.
    if (flags & FLAG_PATHFINDING) != 0
        && (body.flags & (tilestate::FLOORCHANGE | tilestate::TELEPORT)) != 0
    {
        return ReturnValue::NotPossible;
    }

    // C++ ref: src/tile.cpp:531-533 (monster); same flag checked for players on path tiles.
    if (flags & FLAG_PATHFINDING) != 0 && (body.flags & tilestate::IMMOVABLENOFIELDBLOCKPATH) != 0 {
        return ReturnValue::NotPossible;
    }

    // C++ ref: src/tile.cpp:567-573 — creature blocking (players)
    if (flags & FLAG_IGNOREBLOCKCREATURE) == 0 {
        for &tile_c in &body.creatures {
            if tile_c == mover {
                continue;
            }
            let other_ghost = world.creatures.get(tile_c).is_some_and(|k| {
                matches!(k, CreatureKind::Player(p) if p.ghost_mode)
            });
            if !other_ghost {
                return ReturnValue::NotPossible;
            }
        }
    }

    // C++ ref: src/tile.cpp:606-628 — block solid checks, respecting FLAG_IGNOREBLOCKITEM.
    if (flags & FLAG_IGNOREBLOCKITEM) == 0 {
        if (body.flags & tilestate::BLOCKSOLID) != 0 {
            return ReturnValue::NotEnoughRoom;
        }
        // C++ ref: src/tile.cpp:535 — `TILESTATE_NOFIELDBLOCKPATH` with `FLAG_PATHFINDING`.
        if (flags & FLAG_PATHFINDING) != 0 && (body.flags & tilestate::NOFIELDBLOCKPATH) != 0 {
            return ReturnValue::NotPossible;
        }
    } else {
        // FLAG_IGNOREBLOCKITEM is set — only block on *immovable* blocksolid items.
        // C++ ref: src/tile.cpp:613-627
        if let Some(ground_id) = body.ground {
            if let Some(gt) = world.items_db.items.get(&ground_id) {
                if gt.block_solid() && !gt.moveable() {
                    return ReturnValue::NotPossible;
                }
            }
        }
        for &item_id in body.top_items.iter().chain(body.down_items.iter()) {
            if let Some(item) = world.items.get(item_id) {
                if let Some(it) = world.items_db.items.get(&item.item_type) {
                    if it.block_solid() && !it.moveable() {
                        return ReturnValue::NotPossible;
                    }
                }
            }
        }
    }

    ReturnValue::NoError
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
            if !(0..10).contains(&raw) { 10u8 } else { raw as u8 }
        })
        .unwrap_or(10);

    // Broadcast `0x6B` to ALL spectators (inc. the mover) that can see the position.
    // C++ `map.getSpectators(spectators, pos, true, true)` → players only.
    let spectators: Vec<ConnId> = world
        .conn_to_creature
        .iter()
        .filter_map(|(&conn, &viewer)| {
            if world.can_see_position(viewer, pos) {
                Some(conn)
            } else {
                None
            }
        })
        .collect();

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
    /// TFS `scheduler.cpp`: `steady_timer` + `stopEvent`; wake game thread like `g_dispatcher.addTask`.
    fn commit_next_walk_deadline(&mut self, cid: CreatureId, deadline: Option<Instant>) {
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().next_walk_check = deadline;
        }
        self.sync_walk_timer_arm(cid);
    }

    /// Arm or cancel the Tokio one-shot for `next_walk_check` (no-op when `walk_wake_tx` is `None`).
    fn sync_walk_timer_arm(&mut self, cid: CreatureId) {
        let (deadline, tx_opt) = {
            let Some(k) = self.creatures.get_mut(cid) else {
                return;
            };
            if let Some(h) = k.base_mut().walk_timer.take() {
                h.abort();
            }
            (k.base().next_walk_check, self.walk_wake_tx.clone())
        };
        let Some(tx) = tx_opt else {
            return;
        };
        let Some(deadline) = deadline else {
            return;
        };
        let now = Instant::now();
        if deadline <= now {
            self.check_creature_walk(cid, Instant::now());
            return;
        }
        let handle = tokio::spawn(async move {
            tokio::time::sleep_until(deadline.into()).await;
            let _ = tx.send(cid);
        });
        if let Some(k) = self.creatures.get_mut(cid) {
            *k.base_mut().walk_timer = Some(handle);
        }
    }

    /// Wake from [`tokio::time::sleep_until`] — one `Game::checkCreatureWalk` (`game.cpp` ~3773).
    pub fn process_walk_due_from_wake(&mut self, cid: CreatureId) {
        self.check_creature_walk(cid, Instant::now());
    }

    pub(crate) fn conn_for_creature(&self, cid: CreatureId) -> Option<ConnId> {
        self.conn_to_creature
            .iter()
            .find(|(_, &c)| c == cid)
            .map(|(k, _)| *k)
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
        let spectators: Vec<ConnId> = self
            .conn_to_creature
            .iter()
            .filter_map(|(&conn, &viewer)| {
                if self.can_see_position(viewer, pos) {
                    Some(conn)
                } else {
                    None
                }
            })
            .collect();
        for conn in spectators {
            let packet = self
                .codec
                .encode_creature_turn(guid, stack_u8, pos, dir as u8, false)
                .into_bytes();
            self.enqueue_outgoing(conn, packet);
        }
    }

    /// 772 `Creature::clearToDo` — stop pending walk execution and notify the client.
    ///
    /// Called before adding new walk entries on 772, where every `playerMove` / `playerAutoWalk`
    /// clears pending ToDo entries and reschedules from scratch. TFS 1.4.2 keeps the existing
    /// `eventWalk` timer instead — the 10.98 client predicts locally so stale timers are fine.
    /// The 7.72 client doesn't predict, so stale timers cause visible stutter.
    ///
    /// C++ ref: `gameserver/src/creature.cpp` `clearToDo` (~1351–1366),
    ///          `game.cpp` `playerMove` (~1997–1999).
    fn clear_todo_772(&mut self, conn_id: ConnId, cid: CreatureId) {
        if !matches!(self.codec, tfs_rust_net::codec::Codec::V772(_)) {
            return;
        }
        let had_pending = self.creatures.get(cid).is_some_and(|k| {
            let b = k.base();
            !b.walk_queue.is_empty() || b.next_walk_check.is_some()
        });
        self.stop_event_walk(cid);
        // C++ `playerMove`: `if (player->clearToDo()) { player->sendCancelWalk(); }`
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
    /// **772** (`gameserver/src/game.cpp` ~1982–2003): `clearToDo()` → `sendCancelWalk()` →
    /// `addWalkToDo(dir)` → `startToDo()`. Every new move clears pending execution and
    /// reschedules from scratch.
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

        self.clear_todo_772(conn_id, cid);

        if let Some(CreatureKind::Player(pl)) = self.creatures.get_mut(cid) {
            pl.last_activity = now;
            // TFS 1.4.2: `addEventWalk` returns if `eventWalk != 0` (`creature.cpp` ~307–309).
            // On 772 `clear_todo_772` already stopped the timer, so `add_event_walk` proceeds.
            pl.base.walk_queue.clear();
            pl.base.walk_queue.push_back(direction);
        }
        let walk_sched_base = Instant::now();
        self.add_event_walk(cid, true, walk_sched_base);
    }

    /// TFS `Game::playerAutoWalk` (`game.cpp` ~2075–2084).
    ///
    /// **772** (`gameserver/src/game.cpp` ~2162–2173): `addWalkToDo(listDir)` (first call
    /// triggers `clearToDo()` if `isExecuting`) → `startToDo()`. Same clear-and-restart
    /// pattern as `playerMove`.
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

        self.clear_todo_772(conn_id, cid);

        let is_772 = matches!(self.codec, tfs_rust_net::codec::Codec::V772(_));
        let first_only = is_772 || path.len() == 1;
        if let Some(CreatureKind::Player(pl)) = self.creatures.get_mut(cid) {
            pl.last_activity = now;
            pl.base.walk_queue.clear();
            for d in path {
                pl.base.walk_queue.push_back(d);
            }
        }
        let walk_sched_base = Instant::now();
        self.add_event_walk(cid, first_only, walk_sched_base);
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

    /// TFS `Player::stopWalk` (`player.cpp` ~3398).
    pub fn player_stop_auto_walk(&mut self, cid: CreatureId) {
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.base.cancel_next_walk = true;
        }
    }

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

    /// C++ `sendCreatureMove` teleport path: `sendRemoveTileCreature` + `sendMapDescription`.
    /// Used for queryDestination chain steps where `areInRange<1,1,0>` fails (z-change or >1 tile).
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

        // 1) sendRemoveTileCreature(creature, oldPos, oldStackPos)
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

    /// `ProtocolGame::sendMoveCreature` for other clients (`protocolgame.cpp` ~2872–2893).
    fn broadcast_spectator_move(
        &mut self,
        mover: CreatureId,
        old_pos: Position,
        new_pos: Position,
        old_stack: i32,
    ) {
        let wire_id = match self.creatures.get(mover) {
            Some(k) => creature_wire_id(mover, k),
            None => return,
        };

        // C++ spectator branch: remove+add on teleport or surface→underground (7→8+).
        let surface_to_underground = old_pos.z == 7 && new_pos.z >= 8;
        let z_changed = old_pos.z != new_pos.z;

        let spectators: Vec<(ConnId, CreatureId)> = self
            .conn_to_creature
            .iter()
            .filter_map(|(&conn, &viewer)| {
                if viewer == mover {
                    return None;
                }
                Some((conn, viewer))
            })
            .collect();

        let move_packet =
            send_move_creature_spectator(old_pos, new_pos, old_stack, wire_id).into_bytes();

        for (conn, viewer) in spectators {
            let can_see_old = self.can_see_position(viewer, old_pos);
            let can_see_new = self.can_see_position(viewer, new_pos);

            if can_see_old && can_see_new {
                if z_changed && surface_to_underground {
                    self.send_creature_remove_to_conn(conn, mover, old_pos, old_stack);
                    self.send_creature_appear_to_conn(conn, viewer, mover, new_pos);
                } else if self.is_creature_fully_sent_to_conn(conn, wire_id) {
                    self.enqueue_outgoing(conn, move_packet.clone());
                } else {
                    self.send_creature_appear_to_conn(conn, viewer, mover, new_pos);
                }
            } else if can_see_old {
                self.send_creature_remove_to_conn(conn, mover, old_pos, old_stack);
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
            .is_some_and(|k| k.base().next_walk_check.is_some())
        {
            return;
        }
        let ground_speed = self
            .map
            .get_tile(pos)
            .map(|t| ground_speed_for_tile_body(t.body(), self.items_db.as_ref()))
            .unwrap_or(150);
        let ticks = {
            let Some(k) = self.creatures.get(cid) else {
                return;
            };
            get_event_step_ticks(k, k.base(), false, ground_speed, wall_now, &self.mechanics)
        };
        if ticks <= 0 {
            return;
        }
        let delay_ms = ticks.max(1) as u64;
        let anchor = Instant::now();
        self.commit_next_walk_deadline(
            cid,
            Some(anchor + Duration::from_millis(delay_ms)),
        );
    }

    /// Queue one step and arm the walk timer (monster/NPC AI and tests).
    // Parity helper for monster/NPC AI; currently exercised by tests. Retained ahead of caller.
    #[allow(dead_code)]
    pub(crate) fn creature_queue_walk_step(&mut self, cid: CreatureId, direction: Direction) {
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().walk_queue.clear();
            k.base_mut().walk_queue.push_back(direction);
        }
        self.add_event_walk(cid, true, Instant::now());
    }

    /// TFS `Creature::startAutoWalk` + `addEventWalk` — all creature kinds (`creature.cpp` ~274–297).
    pub(crate) fn creature_start_auto_walk(&mut self, cid: CreatureId) {
        let is_772 = matches!(self.codec, tfs_rust_net::codec::Codec::V772(_));
        let first_only = is_772 || self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().walk_queue.len() == 1);
        let walk_sched_base = Instant::now();
        self.add_event_walk(cid, first_only, walk_sched_base);
    }

    /// Monster chase — first queued step runs immediately (`addEventWalk(true)` / `ticks == 1`).
    pub(crate) fn creature_start_chase_auto_walk(&mut self, cid: CreatureId) {
        self.add_event_walk(cid, true, Instant::now());
    }

    /// TFS `Creature::addEventWalk` (`creature.cpp` ~299–322).
    ///
    /// `scheduling_base`: anchor for the **initial** timer when `first_step` is true and `ticks > 1`
    /// (new move / long first delay). Reschedules (`first_step == false`) use `Instant::now()` instead.
    fn add_event_walk(&mut self, cid: CreatureId, first_step: bool, scheduling_base: Instant) {
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
            .is_some_and(|k| k.base().next_walk_check.is_some())
        {
            return;
        }

        let wall_now = Instant::now();

        let ground_speed = self
            .map
            .get_tile(pos)
            .map(|t| ground_speed_for_tile_body(t.body(), self.items_db.as_ref()))
            .unwrap_or(150);

        let ticks = {
            let Some(k) = self.creatures.get(cid) else {
                return;
            };
            get_event_step_ticks(k, k.base(), first_step, ground_speed, wall_now, &self.mechanics)
        };

        if ticks <= 0 {
            return;
        }

        if ticks == 1 {
            // C++ ~316–321: synchronous `checkCreatureWalk`, then `scheduler.addEvent(ticks)` with the same `ticks`.
            // `onWalk` does not call `addEventWalk` when `eventWalk == 0` (~228–232). For `first_step`, `ticks` is
            // always `1` before `last_step` is updated — schedule the **follow-up** from post-step `getEventStepTicks`.
            self.check_creature_walk_from_add_event_walk(cid, wall_now);
            if first_step {
                self.schedule_walk_followup_deadline(cid);
            } else {
                let anchor = Instant::now();
                self.commit_next_walk_deadline(
                    cid,
                    Some(anchor + Duration::from_millis(1)),
                );
            }
            return;
        }

        let delay_ms = ticks.max(1) as u64;
        if first_step {
            self.commit_next_walk_deadline(
                cid,
                Some(scheduling_base + Duration::from_millis(delay_ms)),
            );
        } else {
            let anchor = Instant::now();
            self.commit_next_walk_deadline(
                cid,
                Some(anchor + Duration::from_millis(delay_ms)),
            );
        }
    }

    pub(crate) fn stop_event_walk(&mut self, cid: CreatureId) {
        if let Some(k) = self.creatures.get_mut(cid) {
            if let Some(h) = k.base_mut().walk_timer.take() {
                h.abort();
            }
            k.base_mut().next_walk_check = None;
        }
    }

    /// TFS `Game::checkCreatureWalk` (`game.cpp` ~3773–3779).
    pub fn check_creature_walk(&mut self, cid: CreatureId, now: Instant) {
        let health_ok = self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().health > 0);
        if !health_ok {
            return;
        }

        let fired_deadline = self
            .creatures
            .get_mut(cid)
            .and_then(|k| k.base_mut().next_walk_check.take());
        let Some(fired_deadline) = fired_deadline else {
            return;
        };

        // Logical deadline still in the future — re-arm (timer coalescing / ordering).
        if fired_deadline > now + WALK_DEADLINE_GRACE {
            self.commit_next_walk_deadline(cid, Some(fired_deadline));
            return;
        }

        self.on_walk(cid, true, now, Some(fired_deadline));
        self.cleanup();
    }

    /// Same as [`check_creature_walk`], but the walk was **not** triggered by a prior `next_walk_check`
    /// (sync branch inside `add_event_walk` when `ticks == 1`). Matches `eventWalk == 0` at `onWalk` exit in C++.
    fn check_creature_walk_from_add_event_walk(&mut self, cid: CreatureId, now: Instant) {
        let health_ok = self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().health > 0);
        if !health_ok {
            return;
        }

        self.commit_next_walk_deadline(cid, None);

        self.on_walk(cid, false, now, None);
        self.cleanup();
    }

    /// TFS `Creature::onWalk` (`creature.cpp` ~200–234).  
    /// `reschedule_after` = C++ `eventWalk != 0` before the end block — only then does `onWalk` call `addEventWalk()`.
    ///
    /// `fired_deadline`: logical `next_walk_check` that triggered this `on_walk` (scheduler path); used to
    /// chain the next deadline without cumulative timer jitter.
    fn on_walk(
        &mut self,
        cid: CreatureId,
        reschedule_after: bool,
        now: Instant,
        fired_deadline: Option<Instant>,
    ) {
        let walk_delay = self
            .creatures
            .get(cid)
            .map(|k| get_walk_delay(k, k.base(), now, &self.mechanics))
            .unwrap_or(0);

        let mut stopped_without_reschedule = false;

        if walk_delay <= 0 {
            let pop_dir = if self
                .creatures
                .get(cid)
                .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
            {
                if self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| !k.base().walk_queue.is_empty())
                {
                    self.creatures
                        .get_mut(cid)
                        .and_then(|k| k.base_mut().walk_queue.pop_back())
                } else {
                    self.monster_next_walk_step(cid, now)
                }
            } else {
                self.creatures
                    .get_mut(cid)
                    .and_then(|k| k.base_mut().walk_queue.pop_back())
            };

            if let Some(mut dir) = pop_dir {
                let mut drunk_hicks = false;
                if let Some(CreatureKind::Player(p)) = self.creatures.get(cid) {
                    if let Some(new_dir) = try_drunk_walk_direction(&p.base) {
                        dir = new_dir;
                        drunk_hicks = true;
                    }
                }
                if drunk_hicks {
                    self.broadcast_creature_say_viewport(cid, SpeakType::MonsterSay as u8, "Hicks!");
                }
                let old_pos = match self.creatures.get(cid) {
                    Some(k) => k.position(),
                    None => return,
                };
                let result = self.internal_move_creature_step(cid, dir, now);
                match result {
                    Err(ret) => {
                        if let Some(conn) = self.conn_for_creature(cid) {
                            let d = self
                                .creatures
                                .get(cid)
                                .map(|k| k.base().direction)
                                .unwrap_or(Direction::North);
                            let msg = ret.description();
                            self.enqueue_outgoing(
                                conn,
                                send_text_message_simple(MESSAGE_STATUS_SMALL, msg).into_bytes(),
                            );
                            self.enqueue_outgoing(
                                conn,
                                self.codec
                                    .encode_cancel_walk(d as u8)
                                    .into_bytes(),
                            );
                        }
                        // TFS `Creature::onWalk` — `listWalkDir` is **not** cleared on failed move; step was already
                        // popped in `getNextStep` (`src/creature.cpp` ~205–213).
                        // C++ sets `forceUpdateFollowPath` only — repath runs from `onThink` / follow refresh,
                        // not synchronously from `onWalk` (avoids repath→step→fail infinite recursion).
                        if let Some(k) = self.creatures.get_mut(cid) {
                            k.base_mut().force_update_follow_path = true;
                        }
                    }
                    Ok(segments) => {
                        let new_pos = match self.creatures.get(cid) {
                            Some(k) => k.position(),
                            None => return,
                        };

                        // Emit per-segment move packets — matches C++ `Map::moveCreature` which
                        // sends a packet for each call (game.cpp ~863-864 loop).
                        if let Some(conn) = self.conn_for_creature(cid) {
                            for seg in &segments {
                                if seg.teleport {
                                    // C++ teleport path: sendRemoveTileCreature + sendMapDescription
                                    self.emit_teleport_move_packet(cid, conn, seg.from, seg.to, seg.old_stack);
                                } else {
                                    self.emit_move_packet(cid, conn, seg.from, seg.to, seg.old_stack);
                                }
                            }
                        }
                        // Broadcast to spectators using overall old→new for now.
                        // C++ broadcasts per moveCreature call, but the initial step is most
                        // important for spectator rendering.
                        let overall_old_stack = segments.first().map(|s| s.old_stack).unwrap_or(1);
                        self.broadcast_spectator_move(cid, old_pos, new_pos, overall_old_stack);

                        // TFS `lastStep` is set in `onCreatureMove` **after** `sendCreatureMove` (`map.cpp` ~309–324).
                        let gs_dest = self
                            .map
                            .get_tile(new_pos)
                            .map(|t| ground_speed_for_tile_body(t.body(), self.items_db.as_ref()))
                            .unwrap_or(150);
                        if let Some(k) = self.creatures.get_mut(cid) {
                            let base = k.base_mut();
                            base.last_step = Some(Instant::now());
                            base.last_step_cost = last_step_cost_for_move(old_pos, new_pos);
                            base.last_step_ground_speed = gs_dest;
                        }
                    }
                }
            } else {
                // TFS: `getNextStep` false → `stopEventWalk`, `onWalkComplete` if queue empty (`src/creature.cpp` ~215–219).
                self.stop_event_walk(cid);
                if self.monster_should_keep_chase_walk_alive(cid)
                    || self.monster_should_keep_dance_walk_alive(cid)
                {
                    // C++ keeps polling `getNextStep` while chasing; re-arm when the queue is empty
                    // but `followCreature` is still set (including `chase_fully_blocked` repaths).
                    self.schedule_walk_followup_deadline(cid);
                } else {
                    stopped_without_reschedule = true;
                }
                self.events.on_walk_complete(cid);
                if self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| matches!(k, CreatureKind::Player(_)))
                {
                    self.on_player_walk_complete(cid, now);
                }
                if self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
                {
                    self.monster_on_walk_complete(cid);
                }
            }
        }

        if self.creatures.get(cid).is_some_and(|k| {
            matches!(k, CreatureKind::Player(p) if p.base.cancel_next_walk)
        }) {
            let dir_byte = self.creatures.get(cid).and_then(|k| match k {
                CreatureKind::Player(p) => Some(p.base.direction as u8),
                _ => None,
            });
            let conn = self.conn_for_creature(cid);
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                p.base.walk_queue.clear();
                p.base.cancel_next_walk = false;
            }
            // TFS `Player::onWalkAborted` — `sendCancelWalk` (`player.cpp` ~3384–3387).
            if let (Some(conn), Some(db)) = (conn, dir_byte) {
                self.enqueue_encoded(conn, self.codec.encode_cancel_walk(db));
            }
            self.clear_player_walk_action(cid);
        }

        if !stopped_without_reschedule && reschedule_after {
            if let Some(logical) = fired_deadline {
                self.commit_next_walk_deadline(cid, None);
                self.add_event_walk(cid, false, logical);
            }
        }
    }

    /// TFS `Game::internalMoveCreature` — both overloads combined.
    /// C++ ref: src/game.cpp:797-894
    ///
    /// Returns `Ok(segments)` on success — each segment corresponds to one C++
    /// `Map::moveCreature` call and needs its own move packet.
    /// Returns `Err(ret)` when the move is rejected.
    fn internal_move_creature_step(
        &mut self,
        cid: CreatureId,
        direction: Direction,
        now: Instant,
    ) -> Result<Vec<MoveSegment>, ReturnValue> {
        let current_pos = match self.creatures.get(cid) {
            Some(k) => k.position(),
            None => return Err(ReturnValue::NotPossible),
        };
        let flags_in = FLAG_IGNOREFIELDDAMAGE;

        let is_player = self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Player(_)));

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
        let Some(to_tile) = self.map.get_tile(dest_pos) else {
            return Err(ReturnValue::NotPossible);
        };

        let ret = tile_query_add_creature(self, to_tile, cid, flags);
        if ret != ReturnValue::NoError {
            return Err(ret);
        }

        let old_pos = current_pos;
        // `Player::onWalk(Direction&)` — `getStepDuration` reads **source** tile ground speed.
        let gs_next_action = self
            .map
            .get_tile(old_pos)
            .map(|t| ground_speed_for_tile_body(t.body(), self.items_db.as_ref()))
            .unwrap_or(150);

        let mut segments: Vec<MoveSegment> = Vec::new();

        // Collect old_stack for the initial move BEFORE moving the creature.
        let raw_initial_stack = self
            .map
            .get_tile(old_pos)
            .map(|t| client_creature_stack_pos(t.body(), cid))
            .unwrap_or(-1);
        let initial_old_stack = if raw_initial_stack >= 0 {
            raw_initial_stack
        } else {
            1
        };

        // C++ map.cpp:262 — teleport detection for initial step.
        let has_ground = self.map.get_tile(dest_pos)
            .map(|t| t.body().ground.is_some())
            .unwrap_or(false);
        let initial_teleport = !has_ground || !are_in_range_1_1_0(old_pos, dest_pos);

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
            let Some((new_pos, _new_flags)) = query_destination(&self.map, final_pos, tile_flags) else {
                break;
            };

            // Collect old_stack for this chain step BEFORE moving.
            let chain_old_stack = self
                .map
                .get_tile(final_pos)
                .map(|t| client_creature_stack_pos(t.body(), cid))
                .filter(|s| *s >= 0)
                .unwrap_or(1);

            let chain_has_ground = self.map.get_tile(new_pos)
                .map(|t| t.body().ground.is_some())
                .unwrap_or(false);
            let chain_teleport = !chain_has_ground || !are_in_range_1_1_0(final_pos, new_pos);

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
            let dur_ms =
                get_step_duration_ms_with_direction(k, k.base(), direction, gs_next_action, &self.mechanics);
            if let CreatureKind::Player(p) = k {
                p.next_action_until = Some(now + Duration::from_millis(dur_ms.max(1) as u64));
            }
        }

        // Step 3: post-queryDestination chain turn overrides everything.
        // C++ ref: src/game.cpp:882-891
        // C++ calls `internalCreatureTurn` here — which sets direction AND sends `0x6B`.
        // Now that creature.position() == final_pos, the broadcast will correctly reach
        // the moving player (previously dropped due to z-mismatch in can_see_position).
        if let Some(fp) = from_pos {
            if fp.z != final_pos.z && (fp.x != final_pos.x || fp.y != final_pos.y) {
                let dir = direction_from_positions(fp, final_pos);
                if !is_diagonal(dir) {
                    internal_creature_turn_with_broadcast(self, cid, dir);
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

        Ok(segments)
    }

    /// Move a creature between tiles on the map (unregister from old, register at new).
    /// C++ `Map::moveCreature` — position follows the tile (`newTile.addThing`) before
    /// `onCreatureMove` fan-out (`map.cpp` ~293–324).
    fn move_creature_on_map(&mut self, cid: CreatureId, from: Position, to: Position) {
        if from == to { return; }
        self.map.unregister_creature_at(from, cid);
        self.map.register_creature_at(to, cid);
        if let Some(k) = self.creatures.get_mut(cid) {
            k.set_position(to);
        }
        self.monster_dispatch_creature_move(cid, from, to);
    }

    pub fn process_walk_deadlines(&mut self) {
        if self.walk_wake_tx.is_some() {
            return;
        }
        // Chain: nested `addEventWalk` / same-deadline walks should drain in one wake (scheduler coalesces).
        // Sample `Instant::now()` each pass — do not use a snapshot from the game-loop branch (Bug 4).
        const MAX_CHAIN: usize = 64;
        for _ in 0..MAX_CHAIN {
            let now = Instant::now();
            let mut due: Vec<CreatureId> = Vec::new();
            for (cid, k) in self.creatures.iter() {
                if let Some(deadline) = k.base().next_walk_check {
                    if now >= deadline {
                        due.push(cid);
                    }
                }
            }
            if due.is_empty() {
                break;
            }
            for cid in due {
                let step_now = Instant::now();
                self.check_creature_walk(cid, step_now);
            }
        }
    }

    /// TFS `Creature::getPathTo` / `Map::getPathMatching` for walk-to-item (`creature.cpp` ~1735).
    pub(crate) fn get_creature_path_to(
        &self,
        cid: CreatureId,
        target: Position,
        min_target_dist: i32,
        max_target_dist: i32,
    ) -> Option<Vec<Direction>> {
        use crate::pathfinding::{get_path_matching, FindPathParams, CREATURE_ON_TILE_PATH_COST};

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
        get_path_matching(
            &self.map,
            start,
            target,
            &fpp,
            self.mechanics.profile.path_cost,
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
        )
    }
}

#[cfg(test)]
mod step_speed_tests {
    use std::time::{Duration, Instant};

    use super::{
        calculated_step_speed_tfs, get_event_step_ticks, get_step_duration, walk_timing_speed,
        wire_step_speed, WalkSpeedRole,
    };
    use crate::creature::CreatureKind;
    use crate::formulas::{cipsoft_effective_speed, Mechanics};
    use crate::Monster;
    use crate::test_world::support::test_player;
    use tfs_rust_common::{Position, ProtocolVersion};

    /// Anchors from `src/creature.cpp` `Creature::getStepDuration` (`floor((A*log((step/2)+B)+C)+0.5)`).
    #[test]
    fn calculated_step_speed_matches_tfs_creature_cpp() {
        assert_eq!(calculated_step_speed_tfs(10), 1);
        assert_eq!(calculated_step_speed_tfs(220), 278);
        assert_eq!(calculated_step_speed_tfs(400), 464);
        assert_eq!(calculated_step_speed_tfs(1500), 1137);
    }

    /// 772 player wire uses GoStrength (220 at level 1), not `2*go+80` (520).
    #[test]
    fn wire_step_speed_772_player_is_go_strength() {
        let p = test_player("Walker", Position::new(100, 100, 7));
        let mut base = p.base.clone();
        base.speed = 220;
        let mech = Mechanics::for_version(ProtocolVersion::V772);
        assert_eq!(
            wire_step_speed(WalkSpeedRole::Player, &base, &mech),
            220
        );
        assert_eq!(walk_timing_speed(WalkSpeedRole::Player, &base, &mech), 520);
    }

    /// 772 player GoStrength scales with level (`gameserver/src/player.h` `updateBaseSpeed`).
    #[test]
    fn wire_step_speed_772_player_scales_with_level() {
        let mech = Mechanics::for_version(ProtocolVersion::V772);
        for (level, expected_go) in [(1, 220), (2, 222), (8, 228), (50, 270)] {
            let go = crate::creature::vocation::base_walk_speed(
                crate::formulas::StepSpeedModel::CipSoft,
                1,
                level,
            );
            assert_eq!(go, expected_go, "level {level}");
            let mut base = test_player("Walker", Position::new(100, 100, 7)).base;
            base.speed = go;
            base.base_speed = go;
            assert_eq!(
                wire_step_speed(WalkSpeedRole::Player, &base, &mech),
                expected_go as u16,
                "level {level}"
            );
        }
    }

    /// 772 monster wire matches TVP `getStepSpeed()` — wolf GoStrength 42 → 164 on wire.
    #[test]
    fn wire_step_speed_772_monster_is_effective_get_speed() {
        let mut base = test_player("Wolf", Position::new(100, 100, 7)).base;
        base.speed = 42;
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
        let mech = Mechanics::for_version(ProtocolVersion::V1098);
        assert_eq!(
            wire_step_speed(WalkSpeedRole::Player, &base, &mech),
            220
        );
    }

    /// Overdue `addEventWalk(true)` (walk_delay <= 0) returns `1` ms to trigger step immediately.
    #[test]
    fn event_step_ticks_overdue_only_delay_returns_one_ms() {
        let p = test_player("Walker", Position::new(100, 100, 7));
        let mut base = p.base.clone();
        base.speed = 220;
        base.last_step = Some(Instant::now() - Duration::from_millis(315));
        base.last_step_ground_speed = 150;
        base.last_step_cost = 1;
        let mech = Mechanics::for_version(ProtocolVersion::V772);
        let kind = CreatureKind::Player(p);
        let ticks = get_event_step_ticks(&kind, &base, true, 150, Instant::now(), &mech);
        assert_eq!(ticks, 1);
    }

    #[test]
    fn event_step_ticks_fresh_only_delay_returns_one_ms() {
        let p = test_player("Walker", Position::new(100, 100, 7));
        let mut base = p.base.clone();
        base.speed = 220;
        base.last_step = None;
        let mech = Mechanics::for_version(ProtocolVersion::V772);
        let kind = CreatureKind::Player(p);
        let ticks = get_event_step_ticks(&kind, &base, true, 150, Instant::now(), &mech);
        assert_eq!(ticks, 1);
    }

    /// 772 wolf GoStrength 42 → `GetSpeed` 164; TVP walk quantizer 50 ms → 950 ms on ground 150.
    #[test]
    fn cipsoft_step_duration_matches_notify_go() {
        let p = test_player("Wolf", Position::new(100, 100, 7));
        let mut base = p.base.clone();
        base.speed = 42;
        let mech = Mechanics::for_version(ProtocolVersion::V772);
        assert_eq!(cipsoft_effective_speed(42), 164);
        let kind = CreatureKind::Monster(Monster::new(base.clone(), Position::new(0, 0, 7)));
        assert_eq!(get_step_duration(&kind, &base, 150, &mech), 950);
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
            assert_eq!(d % 50, 0, "1098 duration must be a multiple of 50 (speed {speed})");
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
}

#[cfg(test)]
mod monster_walk_tests {
    use crate::login_out::creature_wire_id;
    use crate::test_world::support;
    use tfs_rust_common::ConnId;
    use tfs_rust_common::enums::Direction;
    use tfs_rust_common::Position;

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

        for _ in 0..32 {
            if world.creatures.get(monster).map(|k| k.position()) == Some(monster_end) {
                break;
            }
            world.process_walk_deadlines();
        }

        assert_eq!(
            world.creatures.get(monster).map(|k| k.position()),
            Some(monster_end),
            "monster should have stepped east"
        );

        let packets = world.pending_outgoing.get(&conn).cloned().unwrap_or_default();
        assert!(
            packets.iter().any(|p| !p.is_empty() && p[0] == 0x6D),
            "spectator should receive 0x6D move packet"
        );
    }
}
