//! Chase JSONL subscriber — C++ `chase_ai.jsonl` schema for Python lockstep diffs.
//!
//! C++ reference: `chase_path_debug.cc` writers; `chase_kite_scenario.cc` `ChasePathResetLog`,
//! `ChasePathLogHarnessPlayerStep`. `via` from `cract.cc:1054` (`ToDoGo` manhattan==1 → `single`).

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// `--log` wins; else `TFS_CHASE_PATH_LOG`; else `log/chase_ai.jsonl`.
pub fn resolve_log_path(cli: Option<PathBuf>) -> PathBuf {
    cli.or_else(|| std::env::var("TFS_CHASE_PATH_LOG").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("log/chase_ai.jsonl"))
}

/// Truncate chase JSONL at scenario start — C++ `ChasePathResetLog`.
pub fn truncate(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    std::fs::write(path, "").map_err(|e| format!("truncate {}: {e}", path.display()))
}

/// Install the chase JSONL layer as the process subscriber and truncate the file.
pub fn install(cli: Option<PathBuf>) -> Result<(), String> {
    let path = resolve_log_path(cli);
    truncate(&path)?;
    let layer = ChaseJsonlLayer::new(path);
    tracing_subscriber::registry()
        .with(layer)
        .try_init()
        .map_err(|e| format!("chase jsonl subscriber: {e}"))
}

pub struct ChaseJsonlLayer {
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl ChaseJsonlLayer {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            write_lock: Mutex::new(()),
        }
    }

    fn write_line(&self, line: &str) {
        let Ok(_guard) = self.write_lock.lock() else {
            return;
        };
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(file, "{line}");
            let _ = file.flush();
        }
    }
}

impl<S: Subscriber> Layer<S> for ChaseJsonlLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if event.metadata().target() != "chase" {
            return;
        }
        let mut fields = Collected::default();
        event.record(&mut fields);
        if let Some(line) = serialize(&fields) {
            self.write_line(&line);
        }
    }
}

#[derive(Default)]
struct Collected {
    event: Option<String>,
    tick: Option<u64>,
    id: Option<u64>,
    name: Option<String>,
    from_x: Option<u16>,
    from_y: Option<u16>,
    from_z: Option<u8>,
    dest_x: Option<u16>,
    dest_y: Option<u16>,
    dest_z: Option<u8>,
    to_x: Option<u16>,
    to_y: Option<u16>,
    to_z: Option<u8>,
    start_x: Option<u16>,
    start_y: Option<u16>,
    start_z: Option<u8>,
    pos_x: Option<u16>,
    pos_y: Option<u16>,
    pos_z: Option<u8>,
    branch: Option<String>,
    via: Option<String>,
    arm: Option<String>,
    reason: Option<String>,
    must: Option<bool>,
    max: Option<i32>,
    monster_state: Option<String>,
    chase_mode: Option<String>,
    attack_target: Option<u64>,
    wait_ms: Option<u32>,
    needs_close_step: Option<bool>,
    close_chase: Option<String>,
    old_state: Option<String>,
    new_state: Option<String>,
    attacker_id: Option<u64>,
    damage: Option<i32>,
    had_target: Option<bool>,
    spell: Option<String>,
    target_id: Option<u64>,
    shape: Option<String>,
    range: Option<i32>,
    delay_ms: Option<u64>,
    phase: Option<String>,
    dir: Option<u8>,
    mover_id: Option<u64>,
    kind: Option<String>,
    cheb: Option<i32>,
    label: Option<String>,
    queue_len: Option<usize>,
    locked: Option<bool>,
    walk_queue_len: Option<usize>,
    attack: Option<i32>,
    defense: Option<i32>,
    armor: Option<i32>,
    hp_before: Option<i32>,
    hp_after: Option<i32>,
    earliest_attack_ms: Option<u64>,
    killer_id: Option<u64>,
    experience: Option<u32>,
    corpse_id: Option<u16>,
    visible: Option<i32>,
    min_wp: Option<u32>,
    ok: Option<bool>,
    steps: Option<String>,
    state: Option<String>,
    follow_target: Option<u64>,
    los_clear: Option<bool>,
    step: Option<u32>,
    call_index: Option<u64>,
    value: Option<i32>,
    site: Option<String>,
    seed: Option<u64>,
}

impl Collected {
    fn set_u64(&mut self, name: &str, v: u64) {
        match name {
            "tick" => self.tick = Some(v),
            "id" => self.id = Some(v),
            "from_x" => self.from_x = Some(v as u16),
            "from_y" => self.from_y = Some(v as u16),
            "from_z" => self.from_z = Some(v as u8),
            "dest_x" => self.dest_x = Some(v as u16),
            "dest_y" => self.dest_y = Some(v as u16),
            "dest_z" => self.dest_z = Some(v as u8),
            "to_x" => self.to_x = Some(v as u16),
            "to_y" => self.to_y = Some(v as u16),
            "to_z" => self.to_z = Some(v as u8),
            "start_x" => self.start_x = Some(v as u16),
            "start_y" => self.start_y = Some(v as u16),
            "start_z" => self.start_z = Some(v as u8),
            "pos_x" => self.pos_x = Some(v as u16),
            "pos_y" => self.pos_y = Some(v as u16),
            "pos_z" => self.pos_z = Some(v as u8),
            "attack_target" => self.attack_target = Some(v),
            "wait_ms" => self.wait_ms = Some(v as u32),
            "attacker_id" => self.attacker_id = Some(v),
            "target_id" => self.target_id = Some(v),
            "delay_ms" => self.delay_ms = Some(v),
            "dir" => self.dir = Some(v as u8),
            "mover_id" => self.mover_id = Some(v),
            "queue_len" => self.queue_len = Some(v as usize),
            "walk_queue_len" => self.walk_queue_len = Some(v as usize),
            "earliest_attack_ms" => self.earliest_attack_ms = Some(v),
            "killer_id" => self.killer_id = Some(v),
            "experience" => self.experience = Some(v as u32),
            "corpse_id" => self.corpse_id = Some(v as u16),
            "visible" => self.visible = Some(v as i32),
            "min_wp" => self.min_wp = Some(v as u32),
            "follow_target" => self.follow_target = Some(v),
            "step" => self.step = Some(v as u32),
            "call_index" => self.call_index = Some(v),
            "seed" => self.seed = Some(v),
            "must" => self.must = Some(v != 0),
            "ok" => self.ok = Some(v != 0),
            "needs_close_step" => self.needs_close_step = Some(v != 0),
            "had_target" => self.had_target = Some(v != 0),
            "locked" => self.locked = Some(v != 0),
            "los_clear" => self.los_clear = Some(v != 0),
            "max" => self.max = Some(v as i32),
            "cheb" => self.cheb = Some(v as i32),
            "damage" => self.damage = Some(v as i32),
            "range" => self.range = Some(v as i32),
            "attack" => self.attack = Some(v as i32),
            "defense" => self.defense = Some(v as i32),
            "armor" => self.armor = Some(v as i32),
            "hp_before" => self.hp_before = Some(v as i32),
            "hp_after" => self.hp_after = Some(v as i32),
            "value" => self.value = Some(v as i32),
            _ => {}
        }
    }

    fn set_i64(&mut self, name: &str, v: i64) {
        if v >= 0 {
            self.set_u64(name, v as u64);
            return;
        }
        match name {
            "max" => self.max = Some(v as i32),
            "cheb" => self.cheb = Some(v as i32),
            "damage" => self.damage = Some(v as i32),
            "range" => self.range = Some(v as i32),
            "attack" => self.attack = Some(v as i32),
            "defense" => self.defense = Some(v as i32),
            "armor" => self.armor = Some(v as i32),
            "hp_before" => self.hp_before = Some(v as i32),
            "hp_after" => self.hp_after = Some(v as i32),
            "value" => self.value = Some(v as i32),
            "visible" => self.visible = Some(v as i32),
            _ => {}
        }
    }

    fn set_bool(&mut self, name: &str, v: bool) {
        match name {
            "must" => self.must = Some(v),
            "ok" => self.ok = Some(v),
            "needs_close_step" => self.needs_close_step = Some(v),
            "had_target" => self.had_target = Some(v),
            "locked" => self.locked = Some(v),
            "los_clear" => self.los_clear = Some(v),
            _ => {}
        }
    }

    fn set_str(&mut self, name: &str, v: &str) {
        let owned = v.to_string();
        match name {
            "event" => self.event = Some(owned),
            "name" => self.name = Some(owned),
            "branch" => self.branch = Some(owned),
            "via" => self.via = Some(owned),
            "arm" => self.arm = Some(owned),
            "reason" => self.reason = Some(owned),
            "monster_state" => self.monster_state = Some(owned),
            "chase_mode" => self.chase_mode = Some(owned),
            "close_chase" => self.close_chase = Some(owned),
            "old_state" => self.old_state = Some(owned),
            "new_state" => self.new_state = Some(owned),
            "spell" => self.spell = Some(owned),
            "shape" => self.shape = Some(owned),
            "phase" => self.phase = Some(owned),
            "kind" => self.kind = Some(owned),
            "label" => self.label = Some(owned),
            "steps" => self.steps = Some(owned),
            "state" => self.state = Some(owned),
            "site" => self.site = Some(owned),
            _ => {}
        }
    }
}

impl Visit for Collected {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.set_str(field.name(), &format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.set_str(field.name(), value);
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.set_u64(field.name(), value);
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.set_i64(field.name(), value);
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.set_bool(field.name(), value);
    }
}

fn json_escape_name(name: &str) -> String {
    name.replace('\\', "\\\\").replace('"', "\\\"")
}

fn pos_json(key: &str, x: u16, y: u16, z: u8) -> String {
    format!("\"{key}\":{{\"x\":{x},\"y\":{y},\"z\":{z}}}")
}

fn header(tick: u64, id: u64, name: &str, evt: &str) -> String {
    format!(
        "{{\"src\":\"rust\",\"evt\":\"{evt}\",\"tick\":{tick},\"id\":{id},\"name\":\"{}\"",
        json_escape_name(name)
    )
}

fn flag(v: bool) -> u8 {
    u8::from(v)
}

fn chebyshev(ax: u16, ay: u16, bx: u16, by: u16) -> i32 {
    (ax as i32 - bx as i32)
        .abs()
        .max((ay as i32 - by as i32).abs())
}

/// C++ `ToDoGo` via — `cract.cc:1054` (manhattan==1 → `single`, else `enter`).
fn todo_go_via(from_x: u16, from_y: u16, dest_x: u16, dest_y: u16) -> &'static str {
    let dx = (dest_x as i32 - from_x as i32).abs();
    let dy = (dest_y as i32 - from_y as i32).abs();
    if dx + dy == 1 { "single" } else { "enter" }
}

fn creature_header(f: &Collected, evt: &str) -> Option<String> {
    Some(header(f.tick?, f.id?, f.name.as_deref()?, evt))
}

fn serialize(f: &Collected) -> Option<String> {
    let evt = f.event.as_deref()?;
    match evt {
        "branch" => {
            let from_x = f.from_x?;
            let from_y = f.from_y?;
            let dest_x = f.dest_x?;
            let dest_y = f.dest_y?;
            let cheb = f
                .cheb
                .unwrap_or_else(|| chebyshev(from_x, from_y, dest_x, dest_y));
            let reason_json = f
                .reason
                .as_deref()
                .filter(|r| !r.is_empty())
                .map(|r| format!(",\"reason\":\"{r}\""))
                .unwrap_or_default();
            Some(format!(
                "{},\"branch\":\"{}\",{},{},\"must\":{},\"max\":{},\"cheb\":{cheb}{reason_json}}}",
                creature_header(f, evt)?,
                f.branch.as_deref()?,
                pos_json("from", from_x, from_y, f.from_z?),
                pos_json("dest", dest_x, dest_y, f.dest_z?),
                flag(f.must?),
                f.max?,
            ))
        }
        "todo_go" => {
            let from_x = f.from_x?;
            let from_y = f.from_y?;
            let dest_x = f.dest_x?;
            let dest_y = f.dest_y?;
            let via = f
                .via
                .as_deref()
                .unwrap_or_else(|| todo_go_via(from_x, from_y, dest_x, dest_y));
            let cheb = f
                .cheb
                .unwrap_or_else(|| chebyshev(from_x, from_y, dest_x, dest_y));
            let arm_json = f
                .arm
                .as_deref()
                .filter(|a| !a.is_empty())
                .map(|a| format!(",\"arm\":\"{a}\""))
                .unwrap_or_default();
            Some(format!(
                "{},\"via\":\"{via}\",{},{},\"must\":{},\"max\":{},\"cheb\":{cheb}{arm_json}}}",
                creature_header(f, evt)?,
                pos_json("from", from_x, from_y, f.from_z?),
                pos_json("dest", dest_x, dest_y, f.dest_z?),
                flag(f.must?),
                f.max?,
            ))
        }
        "combat_state" => {
            let target_json = f
                .attack_target
                .map(|id| format!(",\"attack_target\":{id}"))
                .unwrap_or_default();
            Some(format!(
                "{},\"monster_state\":\"{}\",\"chase_mode\":\"{}\"{target_json}}}",
                creature_header(f, evt)?,
                f.monster_state.as_deref()?,
                f.chase_mode.as_deref()?,
            ))
        }
        "attack_enqueue" => Some(format!(
            "{},\"wait_ms\":{},\"needs_close_step\":{},\"close_chase\":\"{}\"}}",
            creature_header(f, evt)?,
            f.wait_ms?,
            flag(f.needs_close_step?),
            f.close_chase.as_deref()?,
        )),
        "damage_stimulus" => Some(format!(
            "{},\"old_state\":\"{}\",\"new_state\":\"{}\",\"attacker_id\":{},\"damage\":{},\"had_target\":{}}}",
            creature_header(f, evt)?,
            f.old_state.as_deref()?,
            f.new_state.as_deref()?,
            f.attacker_id?,
            f.damage?,
            flag(f.had_target?),
        )),
        "spell_cast" => Some(format!(
            "{},\"spell\":\"{}\",\"target_id\":{},\"shape\":\"{}\",\"range\":{}}}",
            creature_header(f, evt)?,
            json_escape_name(f.spell.as_deref()?),
            f.target_id?,
            f.shape.as_deref()?,
            f.range?,
        )),
        "idle_stimulus" => Some(format!("{}}}", creature_header(f, evt)?)),
        "todo_wait" => Some(format!(
            "{},\"delay_ms\":{},\"phase\":\"{}\"}}",
            creature_header(f, evt)?,
            f.delay_ms?,
            f.phase.as_deref()?,
        )),
        "rotate" => {
            let target_json = f
                .target_id
                .map(|id| format!(",\"target_id\":{id}"))
                .unwrap_or_default();
            Some(format!(
                "{},\"dir\":{}{target_json}}}",
                creature_header(f, evt)?,
                f.dir?,
            ))
        }
        "creature_move_stimulus" => Some(format!(
            "{},\"mover_id\":{},\"kind\":\"{}\",\"cheb\":{}}}",
            creature_header(f, evt)?,
            f.mover_id?,
            f.kind.as_deref()?,
            f.cheb?,
        )),
        "todo_label" => Some(format!(
            "{},\"label\":\"{}\",\"queue_len\":{},\"locked\":{},\"walk_queue_len\":{}}}",
            creature_header(f, evt)?,
            f.label.as_deref()?,
            f.queue_len?,
            flag(f.locked?),
            f.walk_queue_len?,
        )),
        "melee_hit" | "ranged_hit" => Some(format!(
            "{},\"target_id\":{},\"attack\":{},\"defense\":{},\"armor\":{},\"damage\":{},\"hp_before\":{},\"hp_after\":{},\"earliest_attack_ms\":{}}}",
            creature_header(f, evt)?,
            f.target_id?,
            f.attack?,
            f.defense?,
            f.armor?,
            f.damage?,
            f.hp_before?,
            f.hp_after?,
            f.earliest_attack_ms?,
        )),
        "creature_death" => Some(format!(
            "{},\"killer_id\":{},\"experience\":{},\"corpse_id\":{}}}",
            creature_header(f, evt)?,
            f.killer_id?,
            f.experience?,
            f.corpse_id?,
        )),
        "shortway" => {
            let start_x = f.start_x?;
            let start_y = f.start_y?;
            let dest_x = f.dest_x?;
            let dest_y = f.dest_y?;
            let rel_x = dest_x as i32 - start_x as i32;
            let rel_y = dest_y as i32 - start_y as i32;
            let steps_inner = f.steps.as_deref().unwrap_or("");
            Some(format!(
                "{},{},{},\"rel_dest\":{{\"x\":{rel_x},\"y\":{rel_y}}},\"visible\":{},\"min_wp\":{},\"must\":{},\"max\":{},\"ok\":{},\"steps\":[{steps_inner}]}}",
                creature_header(f, evt)?,
                pos_json("start", start_x, start_y, f.start_z?),
                pos_json("dest", dest_x, dest_y, f.dest_z?),
                f.visible.unwrap_or(10),
                f.min_wp?,
                flag(f.must?),
                f.max?,
                flag(f.ok?),
            ))
        }
        "parked" => {
            let follow_json = f
                .follow_target
                .map(|id| format!(",\"follow_target\":{id}"))
                .unwrap_or_default();
            let attack_json = f
                .attack_target
                .map(|id| format!(",\"attack_target\":{id}"))
                .unwrap_or_default();
            Some(format!(
                "{},{},\"state\":\"{}\",\"chase_mode\":\"{}\",\"cheb\":{},\"los_clear\":{}{follow_json}{attack_json}}}",
                creature_header(f, evt)?,
                pos_json("pos", f.pos_x?, f.pos_y?, f.pos_z?),
                f.state.as_deref()?,
                f.chase_mode.as_deref()?,
                f.cheb?,
                flag(f.los_clear?),
            ))
        }
        "go_exec" => {
            let from_x = f.from_x?;
            let from_y = f.from_y?;
            let from_z = f.from_z?;
            let to_x = f.to_x?;
            let to_y = f.to_y?;
            let to_z = f.to_z?;
            let diag = flag(from_x != to_x && from_y != to_y && from_z == to_z);
            Some(format!(
                "{},{},{},\"diag\":{diag}}}",
                creature_header(f, evt)?,
                pos_json("from", from_x, from_y, from_z),
                pos_json("to", to_x, to_y, to_z),
            ))
        }
        "harness_player_step" => Some(format!(
            "{{\"src\":\"rust\",\"evt\":\"harness_player_step\",\"tick\":{},\"step\":{},{}}}",
            f.tick?,
            f.step?,
            pos_json("pos", f.pos_x?, f.pos_y?, f.pos_z?),
        )),
        "rng_trace" => {
            let site_json = f
                .site
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| format!(",\"site\":\"{s}\""))
                .unwrap_or_default();
            Some(format!(
                "{{\"src\":\"rust\",\"evt\":\"rng_trace\",\"call_index\":{},\"value\":{}{site_json}}}",
                f.call_index?, f.value?,
            ))
        }
        "rng_resync" => Some(format!(
            "{{\"src\":\"rust\",\"evt\":\"rng_resync\",\"seed\":{}}}",
            f.seed?,
        )),
        _ => None,
    }
}
