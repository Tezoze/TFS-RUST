//! Monster chase / TShortway JSONL trace — mirror 772 `chase_path.log` for parity diffs.
//!
//! C++ reference: `cract.cc` `TShortway`, `ToDoGo`, `Go`; `crnonpl.cc` `TMonster::IdleStimulus`.
//!
//! Enable: env `TFS_CHASE_PATH_DEBUG=1` (optional `TFS_CHASE_PATH_LOG=/path/to/chase_path.log`).

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use slotmap::Key;
use tfs_rust_common::enums::Direction;
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
                .unwrap_or_else(|_| PathBuf::from("log/chase_path.log"));
            let _ = LOG_PATH.set(path);
        }
    });
}

pub fn chase_path_debug_enabled() -> bool {
    ensure_init();
    ENABLED.load(Ordering::Relaxed)
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
) {
    if !chase_path_debug_enabled() {
        return;
    }
    let cheb = chebyshev(from, dest);
    let line = format!(
        "{},\"branch\":\"{branch}\",{},{},\"must\":{},\"max\":{},\"cheb\":{cheb}}}",
        header(tick, cid, name, "branch"),
        pos_json("from", from),
        pos_json("dest", dest),
        u8::from(must_reach),
        max_steps
    );
    write_line(&line);
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
) {
    if !chase_path_debug_enabled() {
        return;
    }
    let cheb = chebyshev(from, dest);
    let line = format!(
        "{},\"via\":\"{via}\",{},{},\"must\":{},\"max\":{},\"cheb\":{cheb}}}",
        header(tick, cid, name, "todo_go"),
        pos_json("from", from),
        pos_json("dest", dest),
        u8::from(must_reach),
        max_steps
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
