//! Monster chase / combat JSONL trace — mirror 772 `chase_ai.jsonl` for parity diffs.
//!
//! Movement: `cract.cc` `TShortway`, `ToDoGo`, `Go`; `crnonpl.cc` `TMonster::IdleStimulus`.
//! Combat (E0–E3): `crcombat.cc` `Attack`/`CloseAttack`; `cract.cc` `ToDoAttack`.
//!
//! Enable: env `TFS_CHASE_PATH_DEBUG=1` (optional `TFS_CHASE_PATH_LOG=/path/to/chase_ai.jsonl`).

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use slotmap::Key;
use tfs_rust_common::Position;

use crate::ids::CreatureId;

static ENABLED: AtomicBool = AtomicBool::new(false);
static INIT: OnceLock<()> = OnceLock::new();
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static LOG_MUTEX: Mutex<()> = Mutex::new(());

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

pub fn chase_path_debug_enabled() -> bool {
    ensure_init();
    ENABLED.load(Ordering::Relaxed)
}

/// Truncate chase JSONL at scenario start — C++ `ChasePathResetLog`.
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

fn json_escape_name(name: &str) -> String {
    name.replace('\\', "\\\\").replace('"', "\\\"")
}

fn pos_json(key: &str, pos: Position) -> String {
    format!(
        "\"{key}\":{{\"x\":{},\"y\":{},\"z\":{}}}",
        pos.x, pos.y, pos.z
    )
}

fn header(tick: u64, cid: CreatureId, name: &str, evt: &str) -> String {
    format!(
        "{{\"src\":\"rust\",\"evt\":\"{evt}\",\"tick\":{tick},\"id\":{},\"name\":\"{}\"",
        cid.data().as_ffi(),
        json_escape_name(name)
    )
}

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
pub fn todo_go_via_from_path(from: Position, dest: Position) -> &'static str {
    let dx = (dest.x as i32 - from.x as i32).abs();
    let dy = (dest.y as i32 - from.y as i32).abs();
    if dx + dy == 1 {
        "single"
    } else {
        "enter"
    }
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

/// E4 prep — monster spell impact (`crnonpl.cc:2521` CASTING block).
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

/// Headless sim RNG trace — glibc draw index + raw value.
pub fn log_rng_trace(call_index: u64, value: i32) {
    if !chase_path_debug_enabled() {
        return;
    }
    let line = format!(
        "{{\"src\":\"rust\",\"evt\":\"rng_trace\",\"call_index\":{call_index},\"value\":{value}}}"
    );
    write_line(&line);
}

/// E2 — melee strike outcome (`crcombat.cc:647` `CloseAttack`).
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

fn chebyshev(a: Position, b: Position) -> i32 {
    (a.x as i32 - b.x as i32)
        .abs()
        .max((a.y as i32 - b.y as i32).abs())
}
