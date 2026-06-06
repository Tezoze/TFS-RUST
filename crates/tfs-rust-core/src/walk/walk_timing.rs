//! Walk speed and step timing — TFS `Creature::getStepDuration`, `getWalkDelay`, `getEventStepTicks`.
//!
//! - `Creature::getStepSpeed` / `getStepDuration` / `getWalkDelay` / `getEventStepTicks` — `creature.cpp` (~1485–1547).
//! - `Player::getStepSpeed` clamp — `player.h` `PLAYER_MIN_SPEED` / `PLAYER_MAX_SPEED`.
//! - CipSoft `NotifyGo` — `cract.cc:1454–1462`.

use std::time::Instant;

use tfs_rust_common::enums::Direction;

use crate::condition::ConditionData;
use crate::creature::CreatureKind;

use super::is_diagonal;

const SPEED_A: f64 = 857.36; // creature.h
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

/// Speed for walk timers and protocol `step_speed` (GoStrength vs CipSoft `GetSpeed`).
pub(crate) fn walk_timing_speed(
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

pub(crate) fn walk_timing_speed_kind(
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

/// TFS `Creature::getStepDuration()` — `creature.cpp` (uses `floor((A*log(...) + C) + 0.5)` and integer `stepSpeed/2`).
pub(crate) fn calculated_step_speed_tfs(step_speed: i32) -> u32 {
    if (step_speed as f64) <= -SPEED_B {
        return 1;
    }
    // C++ uses integer division: `(stepSpeed / 2) + speedB` inside `log`.
    let half = (step_speed / 2) as f64;
    let raw = SPEED_A * (half + SPEED_B).ln() + SPEED_C;
    let cs = (raw + 0.5).floor() as i32;
    cs.max(1) as u32
}

fn walk_quantizer_ms(mech: &crate::formulas::Mechanics) -> i64 {
    match mech.profile.step_speed {
        crate::formulas::StepSpeedModel::CipSoft => mech.profile.beat_ms.max(1) as i64,
        crate::formulas::StepSpeedModel::TfsLog => mech.profile.step_beat_ms.max(1) as i64,
    }
}

#[inline]
fn ceil_to_walk_quantizer(raw_ms: i64, quantizer_ms: i64) -> i64 {
    ((raw_ms + quantizer_ms - 1) / quantizer_ms) * quantizer_ms
}

/// CipSoft / TVP diagonal and floor multipliers on tile waypoints (`cract.cc:1454`, `creature.cpp`).
fn waypoint_step_cost_for_direction(dir: Direction) -> u32 {
    if is_diagonal(dir) {
        3
    } else {
        1
    }
}

/// Next queued step without popping — `walk_queue` is LIFO at the back (`creature.cpp` `listWalkDir`).
pub(crate) fn peek_next_walk_direction(base: &crate::creature::CreatureBase) -> Option<Direction> {
    base.walk_queue.back().copied()
}

/// CipSoft `NotifyGo` — `(Waypoints * 1000) / GetSpeed()`, ceil to Beat (`cract.cc:1461–1462`).
/// `waypoint_cost` is 1 (cardinal), 3 (diagonal), or 2 (floor) applied to tile waypoints before ceil.
fn cipsoft_step_duration_ms(
    kind: &CreatureKind,
    base: &crate::creature::CreatureBase,
    ground_speed: u32,
    waypoint_cost: u32,
    mech: &crate::formulas::Mechanics,
) -> i64 {
    if base.health <= 0 {
        return 0;
    }
    let go = step_speed_for_walk(kind, base, mech);
    let gs = if ground_speed == 0 { 150 } else { ground_speed };
    let waypoints = gs.saturating_mul(waypoint_cost.max(1));
    if let Some(ms) = mech.hooks.step_duration(go, gs as i32, waypoint_cost > 1) {
        return ms.max(1);
    }
    let eff = cipsoft_speed_from_profile(go, mech);
    let delay = (waypoints as i64 * 1000) / i64::from(eff.max(1));
    ceil_to_walk_quantizer(delay, walk_quantizer_ms(mech))
}

pub(crate) fn get_step_duration(
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
    // engine still applies the diagonal / `last_step_cost` multiplier on TFS. Native fast path when unset.
    if let Some(ms) = mech.hooks.step_duration(go, gs as i32, false) {
        return ms.max(1);
    }

    match mech.profile.step_speed {
        crate::formulas::StepSpeedModel::CipSoft => {
            cipsoft_step_duration_ms(kind, base, ground_speed, 1, mech)
        }
        crate::formulas::StepSpeedModel::TfsLog => {
            let calculated_step_speed = calculated_step_speed_tfs(go);
            let duration = (1000.0 * gs as f64 / calculated_step_speed as f64).floor();
            let beat_f = walk_quantizer_ms(mech) as f64;
            ((duration / beat_f).ceil() * beat_f) as i64
        }
    }
}

/// Duration of the step that just completed — era-specific cost application.
fn completed_step_duration_ms(
    kind: &CreatureKind,
    base: &crate::creature::CreatureBase,
    ground_speed: u32,
    mech: &crate::formulas::Mechanics,
) -> i64 {
    match mech.profile.step_speed {
        crate::formulas::StepSpeedModel::CipSoft => {
            cipsoft_step_duration_ms(kind, base, ground_speed, base.last_step_cost.max(1), mech)
        }
        crate::formulas::StepSpeedModel::TfsLog => get_step_duration(kind, base, ground_speed, mech)
            .saturating_mul(base.last_step_cost as i64),
    }
}

/// Duration until the next queued step may fire — uses upcoming direction on CipSoft.
fn upcoming_step_duration_ms(
    kind: &CreatureKind,
    base: &crate::creature::CreatureBase,
    ground_speed: u32,
    next_direction: Option<Direction>,
    mech: &crate::formulas::Mechanics,
) -> i64 {
    match mech.profile.step_speed {
        crate::formulas::StepSpeedModel::CipSoft => {
            let cost = next_direction
                .map(waypoint_step_cost_for_direction)
                .unwrap_or(1);
            cipsoft_step_duration_ms(kind, base, ground_speed, cost, mech)
        }
        crate::formulas::StepSpeedModel::TfsLog => get_step_duration(kind, base, ground_speed, mech)
            .saturating_mul(base.last_step_cost as i64),
    }
}

/// Step duration for `next_action_until` — era-specific diagonal handling.
pub(crate) fn get_step_duration_ms_with_direction(
    kind: &CreatureKind,
    base: &crate::creature::CreatureBase,
    direction: Direction,
    ground_speed: u32,
    mech: &crate::formulas::Mechanics,
) -> i64 {
    match mech.profile.step_speed {
        crate::formulas::StepSpeedModel::CipSoft => cipsoft_step_duration_ms(
            kind,
            base,
            ground_speed,
            waypoint_step_cost_for_direction(direction),
            mech,
        ),
        crate::formulas::StepSpeedModel::TfsLog => {
            let mut ms = get_step_duration(kind, base, ground_speed, mech);
            if is_diagonal(direction) {
                ms *= 3;
            }
            ms
        }
    }
}

/// TFS `Creature::onCreatureMove` — `lastStepCost` (`creature.cpp` ~489–499).
pub(crate) fn last_step_cost_for_move(old_pos: tfs_rust_common::Position, new_pos: tfs_rust_common::Position) -> u32 {
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

pub(crate) fn get_walk_delay(
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
    let step_duration = completed_step_duration_ms(kind, base, gs, mech);
    let delay = step_duration - elapsed.as_millis() as i64;
    delay.max(0)
}

/// CipSoft walk delay using logical `ServerMilliseconds` (`cract.cc` / `MoveCreatures`).
pub(crate) fn get_walk_delay_logical(
    kind: &CreatureKind,
    base: &crate::creature::CreatureBase,
    server_ms: u64,
    mech: &crate::formulas::Mechanics,
) -> i64 {
    let Some(last) = base.last_step_server_ms else {
        return 0;
    };
    let elapsed = server_ms.saturating_sub(last) as i64;
    let gs = base.last_step_ground_speed;
    let gs = if gs == 0 { 150 } else { gs };
    let step_duration = completed_step_duration_ms(kind, base, gs, mech);
    let delay = step_duration - elapsed;
    delay.max(0)
}

pub(crate) fn get_event_step_ticks(
    kind: &CreatureKind,
    base: &crate::creature::CreatureBase,
    only_delay: bool,
    ground_speed_next: u32,
    next_direction: Option<Direction>,
    now: Instant,
    mech: &crate::formulas::Mechanics,
    server_ms: Option<u64>,
) -> i64 {
    let walk_delay = match server_ms {
        Some(sm) => get_walk_delay_logical(kind, base, sm, mech),
        None => get_walk_delay(kind, base, now, mech),
    };
    if walk_delay > 0 {
        return walk_delay;
    }
    let step_duration =
        upcoming_step_duration_ms(kind, base, ground_speed_next, next_direction, mech);
    // TFS `getEventStepTicks(onlyDelay)` returns `1` when `getWalkDelay() <= 0` (`creature.cpp` ~1536–1546).
    if only_delay && step_duration > 0 {
        1
    } else {
        step_duration
    }
}
