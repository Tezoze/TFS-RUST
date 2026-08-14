//! Monster chase / combat JSONL trace — mirror 772 `chase_ai.jsonl` for parity diffs.
//!
//! Movement: `cract.cc` `TShortway`, `ToDoGo`, `Go`; `crnonpl.cc` `TMonster::IdleStimulus`.
//! Combat (E0–E6): `crcombat.cc` `Attack`/`CloseAttack`; `cract.cc` `ToDoAttack`;
//! `crnonpl.cc` CASTING / `DamageStimulus`; `crmain.cc` death drop.
//!
//! Full event catalog (lockstep compare via `scripts/compare_chase_live_logs.py`):
//! `branch`, `todo_go`, `shortway`, `go_exec`, `idle_stimulus`, `todo_wait`, `rotate`,
//! `creature_move_stimulus`, `todo_label`, `parked`, `combat_state`, `attack_enqueue`,
//! `melee_hit`, `ranged_hit`, `spell_cast`, `damage_stimulus`, `creature_death`,
//! `harness_player_step`, `fill_map`, `rng_trace`, `rng_resync`.
//!
//! Enable: env `TFS_CHASE_PATH_DEBUG=1` (optional `TFS_CHASE_PATH_LOG=/path/to/chase_ai.jsonl`).
//!
//! Phase 2: real implementation is `#[cfg(any(test, feature = "sim"))]`; production builds
//! compile no-op stubs below to keep call sites valid without shipping diagnostic code.

use tfs_rust_common::Position;

use crate::ids::CreatureId;

// ---------------------------------------------------------------------------
// Production stubs — no-op when `sim` feature is off and not in test mode.
// ---------------------------------------------------------------------------

#[cfg(not(any(test, feature = "sim")))]
#[allow(unused_variables, dead_code, clippy::too_many_arguments)]
mod stubs {
    use super::*;

    pub fn chase_path_debug_enabled() -> bool {
        false
    }

    pub fn chase_path_reset_log() {}

    pub fn log_branch(
        tick: u64,
        cid: CreatureId,
        name: &str,
        branch: &str,
        from: Position,
        dest: Position,
        must_reach: bool,
        max_steps: i32,
        reason: Option<&str>,
    ) {
    }

    /// C++ `ToDoGo` via label — `cract.cc:1054` (manhattan==1 → `single`, else `enter`).
    pub fn todo_go_via_from_path(from: Position, dest: Position) -> &'static str {
        let dx = (dest.x as i32 - from.x as i32).abs();
        let dy = (dest.y as i32 - from.y as i32).abs();
        if dx + dy == 1 { "single" } else { "enter" }
    }

    pub fn log_todo_go(
        tick: u64,
        cid: CreatureId,
        name: &str,
        via: &str,
        from: Position,
        dest: Position,
        must_reach: bool,
        max_steps: i32,
        arm: Option<&str>,
    ) {
    }

    pub fn log_todo_go_aligned(
        tick: u64,
        cid: CreatureId,
        name: &str,
        from: Position,
        dest: Position,
        must_reach: bool,
        max_steps: i32,
        arm: Option<&str>,
    ) {
    }

    pub fn log_combat_state(
        tick: u64,
        cid: CreatureId,
        name: &str,
        state: &str,
        chase_mode: &str,
        attack_target: Option<u64>,
    ) {
    }

    pub fn log_attack_enqueue(
        tick: u64,
        cid: CreatureId,
        name: &str,
        wait_ms: u32,
        needs_close_step: bool,
        close_chase: &str,
    ) {
    }

    pub fn log_damage_stimulus(
        tick: u64,
        cid: CreatureId,
        name: &str,
        old_state: &str,
        new_state: &str,
        attacker_id: u64,
        damage: i32,
        had_target: bool,
    ) {
    }

    pub fn log_spell_cast(
        tick: u64,
        cid: CreatureId,
        name: &str,
        spell_name: &str,
        target_id: u64,
        shape: &str,
        range: i32,
    ) {
    }

    pub fn log_idle_stimulus(tick: u64, cid: CreatureId, name: &str) {}

    pub fn log_todo_wait(tick: u64, cid: CreatureId, name: &str, delay_ms: u64, phase: &str) {}

    pub fn log_rotate(tick: u64, cid: CreatureId, name: &str, dir: u8, target_id: Option<u64>) {}

    pub fn log_creature_move_stimulus(
        tick: u64,
        cid: CreatureId,
        name: &str,
        mover_id: u64,
        kind: &str,
        cheb: i32,
    ) {
    }

    pub fn log_todo_label(
        tick: u64,
        cid: CreatureId,
        name: &str,
        label: &str,
        queue_len: usize,
        locked: bool,
        walk_queue_len: usize,
    ) {
    }

    pub fn log_rng_trace(call_index: u64, value: i32, site: Option<&'static str>) {}

    pub fn log_rng_resync(seed: u64) {}

    pub fn log_melee_hit(
        tick: u64,
        cid: CreatureId,
        name: &str,
        target_id: u64,
        attack: i32,
        defense: i32,
        armor: i32,
        damage: i32,
        hp_before: i32,
        hp_after: i32,
        earliest_attack_ms: u64,
    ) {
    }

    pub fn log_creature_death(
        tick: u64,
        cid: CreatureId,
        name: &str,
        killer_id: u64,
        experience: u32,
        corpse_id: u16,
    ) {
    }

    pub fn log_ranged_hit(
        tick: u64,
        cid: CreatureId,
        name: &str,
        target_id: u64,
        attack: i32,
        defense: i32,
        armor: i32,
        damage: i32,
        hp_before: i32,
        hp_after: i32,
        earliest_attack_ms: u64,
    ) {
    }

    pub fn log_shortway(
        tick: u64,
        cid: CreatureId,
        name: &str,
        start: Position,
        dest: Position,
        visible: i32,
        min_wp: u32,
        must_reach: bool,
        max_steps: i32,
        ok: bool,
        steps: &[Position],
    ) {
    }

    pub fn log_parked(
        tick: u64,
        cid: CreatureId,
        name: &str,
        pos: Position,
        state: &str,
        follow_target: Option<u64>,
        attack_target: Option<u64>,
        chase_mode: &str,
        cheb: i32,
        los_clear: bool,
    ) {
    }

    pub fn log_go_exec(tick: u64, cid: CreatureId, name: &str, from: Position, to: Position) {}

    pub fn log_harness_player_step(tick: u64, step: u32, pos: Position) {}
}

#[cfg(not(any(test, feature = "sim")))]
pub use stubs::*;

// ---------------------------------------------------------------------------
// Full implementation — compiled for tests and `--features sim` builds.
// ---------------------------------------------------------------------------

#[cfg(any(test, feature = "sim"))]
use std::fs::OpenOptions;
#[cfg(any(test, feature = "sim"))]
use std::io::Write;
#[cfg(any(test, feature = "sim"))]
use std::path::PathBuf;
#[cfg(any(test, feature = "sim"))]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(any(test, feature = "sim"))]
use std::sync::{Mutex, OnceLock};

#[cfg(any(test, feature = "sim"))]
use slotmap::Key;

#[cfg(any(test, feature = "sim"))]
static ENABLED: AtomicBool = AtomicBool::new(false);
#[cfg(any(test, feature = "sim"))]
static INIT: OnceLock<()> = OnceLock::new();
#[cfg(any(test, feature = "sim"))]
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
#[cfg(any(test, feature = "sim"))]
static LOG_MUTEX: Mutex<()> = Mutex::new(());

#[cfg(any(test, feature = "sim"))]
fn ensure_init() {
    INIT.get_or_init(|| {
        let enabled = std::env::var("TFS_CHASE_PATH_DEBUG")
            .map(|v| !v.is_empty() && v != "0")
            .unwrap_or(false);
        ENABLED.store(enabled, Ordering::Relaxed);
        if enabled {
            let path = std::env::var("TFS_CHASE_PATH_LOG")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("log/chase_ai.jsonl"));
            let _ = LOG_PATH.set(path);
        }
    });
}

#[cfg(any(test, feature = "sim"))]
pub fn chase_path_debug_enabled() -> bool {
    ensure_init();
    ENABLED.load(Ordering::Relaxed)
}

/// Truncate chase JSONL at scenario start — C++ `ChasePathResetLog`.
#[cfg(any(test, feature = "sim"))]
pub fn chase_path_reset_log() {
    ensure_init();
    if !chase_path_debug_enabled() {
        return;
    }
    let Some(path) = LOG_PATH.get() else {
        return;
    };
    let Ok(_guard) = LOG_MUTEX.lock() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, "");
}

#[cfg(any(test, feature = "sim"))]
fn write_line(line: &str) {
    let Some(path) = LOG_PATH.get() else {
        return;
    };
    let Ok(_guard) = LOG_MUTEX.lock() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
        let _ = file.flush();
    }
}

#[cfg(any(test, feature = "sim"))]
fn json_escape_name(name: &str) -> String {
    name.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(any(test, feature = "sim"))]
fn pos_json(key: &str, pos: Position) -> String {
    format!(
        "\"{key}\":{{\"x\":{},\"y\":{},\"z\":{}}}",
        pos.x, pos.y, pos.z
    )
}

#[cfg(any(test, feature = "sim"))]
fn header(tick: u64, cid: CreatureId, name: &str, evt: &str) -> String {
    format!(
        "{{\"src\":\"rust\",\"evt\":\"{evt}\",\"tick\":{tick},\"id\":{},\"name\":\"{}\"",
        cid.data().as_ffi(),
        json_escape_name(name)
    )
}

#[cfg(any(test, feature = "sim"))]
pub fn log_branch(
    tick: u64,
    cid: CreatureId,
    name: &str,
    branch: &str,
    from: Position,
    dest: Position,
    must_reach: bool,
    max_steps: i32,
    reason: Option<&str>,
) {
    if !chase_path_debug_enabled() {
        return;
    }
    let cheb = chebyshev(from, dest);
    let reason_json = reason
        .map(|r| format!(",\"reason\":\"{r}\""))
        .unwrap_or_default();
    let line = format!(
        "{},\"branch\":\"{branch}\",{},{},\"must\":{},\"max\":{max_steps},\"cheb\":{cheb}{reason_json}}}",
        header(tick, cid, name, "branch"),
        pos_json("from", from),
        pos_json("dest", dest),
        u8::from(must_reach),
    );
    write_line(&line);
}

/// C++ `ToDoGo` via label — `cract.cc:1054` (manhattan==1 → `single`, else `enter`).
#[cfg(any(test, feature = "sim"))]
pub fn todo_go_via_from_path(from: Position, dest: Position) -> &'static str {
    let dx = (dest.x as i32 - from.x as i32).abs();
    let dy = (dest.y as i32 - from.y as i32).abs();
    if dx + dy == 1 { "single" } else { "enter" }
}

#[cfg(any(test, feature = "sim"))]
pub fn log_todo_go(
    tick: u64,
    cid: CreatureId,
    name: &str,
    via: &str,
    from: Position,
    dest: Position,
    must_reach: bool,
    max_steps: i32,
    arm: Option<&str>,
) {
    if !chase_path_debug_enabled() {
        return;
    }
    let cheb = chebyshev(from, dest);
    let arm_json = arm
        .filter(|a| !a.is_empty())
        .map(|a| format!(",\"arm\":\"{a}\""))
        .unwrap_or_default();
    let line = format!(
        "{},\"via\":\"{via}\",{},{},\"must\":{},\"max\":{},\"cheb\":{cheb}{arm_json}}}",
        header(tick, cid, name, "todo_go"),
        pos_json("from", from),
        pos_json("dest", dest),
        u8::from(must_reach),
        max_steps
    );
    write_line(&line);
}

/// Log `todo_go` with C++-aligned `via` (`enter`/`single`) and optional idle arm name.
#[cfg(any(test, feature = "sim"))]
pub fn log_todo_go_aligned(
    tick: u64,
    cid: CreatureId,
    name: &str,
    from: Position,
    dest: Position,
    must_reach: bool,
    max_steps: i32,
    arm: Option<&str>,
) {
    let via = todo_go_via_from_path(from, dest);
    log_todo_go(tick, cid, name, via, from, dest, must_reach, max_steps, arm);
}

/// E1/E3 — combat state + chase mode (`crnonpl.cc:2387`, `:2705-2712`).
#[cfg(any(test, feature = "sim"))]
pub fn log_combat_state(
    tick: u64,
    cid: CreatureId,
    name: &str,
    state: &str,
    chase_mode: &str,
    attack_target: Option<u64>,
) {
    if !chase_path_debug_enabled() {
        return;
    }
    let target_json = attack_target
        .map(|id| format!(",\"attack_target\":{id}"))
        .unwrap_or_default();
    let line = format!(
        "{},\"monster_state\":\"{state}\",\"chase_mode\":\"{chase_mode}\"{target_json}}}",
        header(tick, cid, name, "combat_state"),
    );
    write_line(&line);
}

/// E2/E3 — idle `ToDoAttack` enqueue (`cract.cc:1325`, `crnonpl.cc:2800`).
#[cfg(any(test, feature = "sim"))]
pub fn log_attack_enqueue(
    tick: u64,
    cid: CreatureId,
    name: &str,
    wait_ms: u32,
    needs_close_step: bool,
    close_chase: &str,
) {
    if !chase_path_debug_enabled() {
        return;
    }
    let line = format!(
        "{},\"wait_ms\":{wait_ms},\"needs_close_step\":{},\"close_chase\":\"{close_chase}\"}}",
        header(tick, cid, name, "attack_enqueue"),
        u8::from(needs_close_step),
    );
    write_line(&line);
}

/// E5 — `TMonster::DamageStimulus` state transition (`crnonpl.cc:2278`).
#[cfg(any(test, feature = "sim"))]
pub fn log_damage_stimulus(
    tick: u64,
    cid: CreatureId,
    name: &str,
    old_state: &str,
    new_state: &str,
    attacker_id: u64,
    damage: i32,
    had_target: bool,
) {
    if !chase_path_debug_enabled() {
        return;
    }
    let line = format!(
        "{},\"old_state\":\"{old_state}\",\"new_state\":\"{new_state}\",\"attacker_id\":{attacker_id},\"damage\":{damage},\"had_target\":{}}}",
        header(tick, cid, name, "damage_stimulus"),
        u8::from(had_target),
    );
    write_line(&line);
}

/// E4 prep — monster spell impact (`crnonpl.cc:2521` CASTING block).
#[cfg(any(test, feature = "sim"))]
pub fn log_spell_cast(
    tick: u64,
    cid: CreatureId,
    name: &str,
    spell_name: &str,
    target_id: u64,
    shape: &str,
    range: i32,
) {
    if !chase_path_debug_enabled() {
        return;
    }
    let line = format!(
        "{},\"spell\":\"{}\",\"target_id\":{target_id},\"shape\":\"{shape}\",\"range\":{range}}}",
        header(tick, cid, name, "spell_cast"),
        json_escape_name(spell_name),
    );
    write_line(&line);
}

/// One `TMonster::IdleStimulus` / inline repath invocation — `crnonpl.cc:2345`.
#[cfg(any(test, feature = "sim"))]
pub fn log_idle_stimulus(tick: u64, cid: CreatureId, name: &str) {
    if !chase_path_debug_enabled() {
        return;
    }
    let line = format!("{}}}", header(tick, cid, name, "idle_stimulus"),);
    write_line(&line);
}

/// `ToDoWait` enqueue or execute — `cract.cc:1030`.
#[cfg(any(test, feature = "sim"))]
pub fn log_todo_wait(tick: u64, cid: CreatureId, name: &str, delay_ms: u64, phase: &str) {
    if !chase_path_debug_enabled() {
        return;
    }
    let line = format!(
        "{},\"delay_ms\":{delay_ms},\"phase\":\"{phase}\"}}",
        header(tick, cid, name, "todo_wait"),
    );
    write_line(&line);
}

/// `TCreature::Rotate` toward a target — `cract.cc:452`, idle tail `crnonpl.cc:2871`.
#[cfg(any(test, feature = "sim"))]
pub fn log_rotate(tick: u64, cid: CreatureId, name: &str, dir: u8, target_id: Option<u64>) {
    if !chase_path_debug_enabled() {
        return;
    }
    let target_json = target_id
        .map(|id| format!(",\"target_id\":{id}"))
        .unwrap_or_default();
    let line = format!(
        "{},\"dir\":{dir}{target_json}}}",
        header(tick, cid, name, "rotate"),
    );
    write_line(&line);
}

/// Follow-target move / combat restep — `crmain.cc:919`, dist inline repath.
#[cfg(any(test, feature = "sim"))]
pub fn log_creature_move_stimulus(
    tick: u64,
    cid: CreatureId,
    name: &str,
    mover_id: u64,
    kind: &str,
    cheb: i32,
) {
    if !chase_path_debug_enabled() {
        return;
    }
    let line = format!(
        "{},\"mover_id\":{mover_id},\"kind\":\"{kind}\",\"cheb\":{cheb}}}",
        header(tick, cid, name, "creature_move_stimulus"),
    );
    write_line(&line);
}

/// ToDo queue transition — mirrors `trace_creature_todo` labels for lockstep diffs.
#[cfg(any(test, feature = "sim"))]
pub fn log_todo_label(
    tick: u64,
    cid: CreatureId,
    name: &str,
    label: &str,
    queue_len: usize,
    locked: bool,
    walk_queue_len: usize,
) {
    if !chase_path_debug_enabled() {
        return;
    }
    let line = format!(
        "{},\"label\":\"{label}\",\"queue_len\":{queue_len},\"locked\":{},\"walk_queue_len\":{walk_queue_len}}}",
        header(tick, cid, name, "todo_label"),
        u8::from(locked),
    );
    write_line(&line);
}

/// Headless sim RNG trace — glibc draw index + raw value + optional call-site tag.
#[cfg(any(test, feature = "sim"))]
pub fn log_rng_trace(call_index: u64, value: i32, site: Option<&'static str>) {
    if !chase_path_debug_enabled() {
        return;
    }
    let site_json = site
        .map(|s| format!(",\"site\":\"{s}\""))
        .unwrap_or_default();
    let line = format!(
        "{{\"src\":\"rust\",\"evt\":\"rng_trace\",\"call_index\":{call_index},\"value\":{value}{site_json}}}"
    );
    write_line(&line);
}

/// Harness RNG stream reset — `ResyncHarnessRng` / appear-batch parity.
#[cfg(any(test, feature = "sim"))]
pub fn log_rng_resync(seed: u64) {
    if !chase_path_debug_enabled() {
        return;
    }
    let line = format!("{{\"src\":\"rust\",\"evt\":\"rng_resync\",\"seed\":{seed}}}");
    write_line(&line);
}

/// E2 — melee strike outcome (`crcombat.cc:647` `CloseAttack`).
#[cfg(any(test, feature = "sim"))]
pub fn log_melee_hit(
    tick: u64,
    cid: CreatureId,
    name: &str,
    target_id: u64,
    attack: i32,
    defense: i32,
    armor: i32,
    damage: i32,
    hp_before: i32,
    hp_after: i32,
    earliest_attack_ms: u64,
) {
    if !chase_path_debug_enabled() {
        return;
    }
    let line = format!(
        "{},\"target_id\":{target_id},\"attack\":{attack},\"defense\":{defense},\"armor\":{armor},\"damage\":{damage},\"hp_before\":{hp_before},\"hp_after\":{hp_after},\"earliest_attack_ms\":{earliest_attack_ms}}}",
        header(tick, cid, name, "melee_hit"),
    );
    write_line(&line);
}

/// E6 — monster death trace for harness compare (`~TMonster` / `DistributeExperiencePoints`).
#[cfg(any(test, feature = "sim"))]
pub fn log_creature_death(
    tick: u64,
    cid: CreatureId,
    name: &str,
    killer_id: u64,
    experience: u32,
    corpse_id: u16,
) {
    if !chase_path_debug_enabled() {
        return;
    }
    let line = format!(
        "{},\"killer_id\":{killer_id},\"experience\":{experience},\"corpse_id\":{corpse_id}}}",
        header(tick, cid, name, "creature_death"),
    );
    write_line(&line);
}

/// E4 — ranged / distance attack outcome (`crcombat.cc:609` `DistanceAttack`).
#[cfg(any(test, feature = "sim"))]
pub fn log_ranged_hit(
    tick: u64,
    cid: CreatureId,
    name: &str,
    target_id: u64,
    attack: i32,
    defense: i32,
    armor: i32,
    damage: i32,
    hp_before: i32,
    hp_after: i32,
    earliest_attack_ms: u64,
) {
    if !chase_path_debug_enabled() {
        return;
    }
    let line = format!(
        "{},\"target_id\":{target_id},\"attack\":{attack},\"defense\":{defense},\"armor\":{armor},\"damage\":{damage},\"hp_before\":{hp_before},\"hp_after\":{hp_after},\"earliest_attack_ms\":{earliest_attack_ms}}}",
        header(tick, cid, name, "ranged_hit"),
    );
    write_line(&line);
}

#[cfg(any(test, feature = "sim"))]
pub fn log_shortway(
    tick: u64,
    cid: CreatureId,
    name: &str,
    start: Position,
    dest: Position,
    visible: i32,
    min_wp: u32,
    must_reach: bool,
    max_steps: i32,
    ok: bool,
    steps: &[Position],
) {
    if !chase_path_debug_enabled() {
        return;
    }
    let rel_x = dest.x as i32 - start.x as i32;
    let rel_y = dest.y as i32 - start.y as i32;
    let steps_json: String = steps
        .iter()
        .map(|p| format!("{{\"x\":{},\"y\":{},\"z\":{}}}", p.x, p.y, p.z))
        .collect::<Vec<_>>()
        .join(",");
    let line = format!(
        "{},{},{},\"rel_dest\":{{\"x\":{rel_x},\"y\":{rel_y}}},\"visible\":{visible},\"min_wp\":{min_wp},\"must\":{},\"max\":{max_steps},\"ok\":{},\"steps\":[{steps_json}]}}",
        header(tick, cid, name, "shortway"),
        pos_json("start", start),
        pos_json("dest", dest),
        u8::from(must_reach),
        u8::from(ok)
    );
    write_line(&line);
}

/// Monster ended idle with a bound target but no todo/walk/wakeup — scheduler dead-end.
#[cfg(any(test, feature = "sim"))]
pub fn log_parked(
    tick: u64,
    cid: CreatureId,
    name: &str,
    pos: Position,
    state: &str,
    follow_target: Option<u64>,
    attack_target: Option<u64>,
    chase_mode: &str,
    cheb: i32,
    los_clear: bool,
) {
    if !chase_path_debug_enabled() {
        return;
    }
    let follow_json = follow_target
        .map(|id| format!(",\"follow_target\":{id}"))
        .unwrap_or_default();
    let attack_json = attack_target
        .map(|id| format!(",\"attack_target\":{id}"))
        .unwrap_or_default();
    let los = u8::from(los_clear);
    let line = format!(
        "{},{},\"state\":\"{state}\",\"chase_mode\":\"{chase_mode}\",\"cheb\":{cheb},\"los_clear\":{los}{follow_json}{attack_json}}}",
        header(tick, cid, name, "parked"),
        pos_json("pos", pos),
    );
    write_line(&line);
}

#[cfg(any(test, feature = "sim"))]
pub fn log_go_exec(tick: u64, cid: CreatureId, name: &str, from: Position, to: Position) {
    if !chase_path_debug_enabled() {
        return;
    }
    let diag = u8::from(from.x != to.x && from.y != to.y && from.z == to.z);
    let line = format!(
        "{},{},{},\"diag\":{diag}}}",
        header(tick, cid, name, "go_exec"),
        pos_json("from", from),
        pos_json("to", to)
    );
    write_line(&line);
}

/// Harness `player_walk` step — tile after legal move, before trailing `sim_tick`.
/// C++ reference: `chase_kite_scenario.cc` `MoveKitePlayer` + `ChasePathLogHarnessPlayerStep`.
#[cfg(any(test, feature = "sim"))]
pub fn log_harness_player_step(tick: u64, step: u32, pos: Position) {
    if !chase_path_debug_enabled() {
        return;
    }
    let line = format!(
        "{{\"src\":\"rust\",\"evt\":\"harness_player_step\",\"tick\":{tick},\"step\":{step},{}}}",
        pos_json("pos", pos)
    );
    write_line(&line);
}

#[cfg(any(test, feature = "sim"))]
fn chebyshev(a: Position, b: Position) -> i32 {
    (a.x as i32 - b.x as i32)
        .abs()
        .max((a.y as i32 - b.y as i32).abs())
}
