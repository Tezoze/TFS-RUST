//! Headless kite scenario executor — drives player kiting scripts for chase parity logs.
//!
//! C++ reference: `tibia-game-master` `chase_kite_scenario.cc`; `crmain.cc` `MoveCreatures`;
//! `operate.cc` `NotifyAllCreatures`.

use std::env;
use std::fs;
use std::path::PathBuf;

use tfs_rust_common::Position;
use tfs_rust_core::creature::{CreatureKind, MonsterAiConfig, MonsterState};
use tfs_rust_core::sim_harness::{
    audit_otbm_route_tiles, beat_driven_world_for_kite_synthetic, beat_driven_world_from_map,
    default_sim_map_config,     insert_monster_from_type, insert_monster_with_config, insert_player,
    clear_harness_appear_idle_defer, harness_place_creature_login, kite_monsters_appear_batch, log_harness_player_step, drain_todo_queue_once, move_creatures_explicit, run_sim_tick,
    set_harness_real_map, set_sim_harness_segment_ms, set_sim_harness_wall_ms, sim_hero_player, sim_player_damage_monster,
    teleport_player, validate_positions_walkable, walk_player_adjacent, write_audit_route_json,
    SimMapConfig,
};

#[derive(Debug, Clone)]
struct MonsterSpawn {
    label: String,
    pos: (u16, u16),
}

#[derive(Debug, Default)]
struct KiteScenario {
    name: String,
    z: u8,
    default_wp: u16,
    arena_center: (u16, u16),
    arena_radius: u16,
    player_start: (u16, u16),
    player_name: String,
    player_health: i32,
    player_health_from_scenario: bool,
    monsters: Vec<MonsterSpawn>,
    monster_speed: i32,
    monster_speed_from_scenario: bool,
    monster_hostile: bool,
    monster_melee_skill: i32,
    monster_melee_skill_from_scenario: bool,
    monster_melee_attack: i32,
    monster_melee_attack_from_scenario: bool,
    monster_armor: i32,
    monster_armor_from_scenario: bool,
    monster_defense: i32,
    monster_defense_from_scenario: bool,
    monster_target_distance: i32,
    monster_target_distance_from_scenario: bool,
    monster_talks: u8,
    monster_talks_from_scenario: bool,
    monster_load_type: bool,
    monster_initial_state: MonsterState,
    monster_state_explicit: bool,
    arena_synthetic: bool,
    steps: Vec<ScenarioStep>,
}

#[derive(Debug, Clone)]
enum ScenarioStep {
    AdvanceMs(u64),
    MonsterAppear,
    PlayerPos(u16, u16),
    PlayerWalk(u16, u16, u64),
    SimTick,
    PlayerDamage(i32),
    PlayerDamageMonster(usize, i32),
}

fn parse_monster_state(raw: &str) -> Result<MonsterState, String> {
    match raw.to_ascii_lowercase().as_str() {
        "sleeping" => Ok(MonsterState::Sleeping),
        "idle" => Ok(MonsterState::Idle),
        "under_attack" | "underattack" => Ok(MonsterState::UnderAttack),
        "attacking" => Ok(MonsterState::Attacking),
        "panic" => Ok(MonsterState::Panic),
        other => Err(format!("unknown monster_state: {other}")),
    }
}

fn parse_scenario(input: &str) -> Result<KiteScenario, String> {
    let mut s = KiteScenario {
        z: 7,
        default_wp: 150,
        arena_center: (32360, 32290),
        arena_radius: 5,
        player_start: (32360, 32290),
        player_name: "Hero".into(),
        player_health: 150,
        monsters: Vec::new(),
        monster_speed: 200,
        monster_speed_from_scenario: false,
        monster_hostile: true,
        monster_melee_skill: 0,
        monster_melee_attack: 7,
        monster_armor: 1,
        monster_defense: 3,
        monster_target_distance: 1,
        monster_talks: 0,
        monster_load_type: true,
        monster_initial_state: MonsterState::Sleeping,
        arena_synthetic: false,
        ..Default::default()
    };

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        let Some(key) = parts.first() else {
            continue;
        };
        match *key {
            "name" if parts.len() >= 2 => s.name = parts[1].to_string(),
            "z" if parts.len() >= 2 => s.z = parts[1].parse().map_err(|_| "bad z")?,
            "default_wp" if parts.len() >= 2 => {
                s.default_wp = parts[1].parse().map_err(|_| "bad default_wp")?;
            }
            "arena" if parts.len() >= 4 => {
                s.arena_center = (
                    parts[1].parse().map_err(|_| "bad arena x")?,
                    parts[2].parse().map_err(|_| "bad arena y")?,
                );
                s.arena_radius = parts[3].parse().map_err(|_| "bad arena radius")?;
            }
            "arena_synthetic" if parts.len() >= 2 => {
                s.arena_synthetic = parts[1] != "0";
            }
            "player_start" if parts.len() >= 3 => {
                s.player_start = (
                    parts[1].parse().map_err(|_| "bad player_start x")?,
                    parts[2].parse().map_err(|_| "bad player_start y")?,
                );
            }
            "player_name" if parts.len() >= 2 => s.player_name = parts[1].to_string(),
            "player_health" if parts.len() >= 2 => {
                s.player_health = parts[1].parse().map_err(|_| "bad player_health")?;
                s.player_health_from_scenario = true;
            }
            "monster" if parts.len() >= 4 => {
                s.monsters.push(MonsterSpawn {
                    label: parts[1].to_string(),
                    pos: (
                        parts[2].parse().map_err(|_| "bad monster x")?,
                        parts[3].parse().map_err(|_| "bad monster y")?,
                    ),
                });
            }
            "monster_hostile" if parts.len() >= 2 => {
                s.monster_hostile = parts[1] != "0";
            }
            "monster_melee_skill" if parts.len() >= 2 => {
                s.monster_melee_skill = parts[1].parse().map_err(|_| "bad monster_melee_skill")?;
                s.monster_melee_skill_from_scenario = true;
            }
            "monster_melee_attack" if parts.len() >= 2 => {
                s.monster_melee_attack =
                    parts[1].parse().map_err(|_| "bad monster_melee_attack")?;
                s.monster_melee_attack_from_scenario = true;
            }
            "monster_armor" if parts.len() >= 2 => {
                s.monster_armor = parts[1].parse().map_err(|_| "bad monster_armor")?;
                s.monster_armor_from_scenario = true;
            }
            "monster_defense" if parts.len() >= 2 => {
                s.monster_defense = parts[1].parse().map_err(|_| "bad monster_defense")?;
                s.monster_defense_from_scenario = true;
            }
            "monster_target_distance" if parts.len() >= 2 => {
                s.monster_target_distance = parts[1].parse().map_err(|_| "bad target_distance")?;
                s.monster_target_distance_from_scenario = true;
            }
            "monster_speed" if parts.len() >= 2 => {
                s.monster_speed = parts[1].parse().map_err(|_| "bad monster_speed")?;
                s.monster_speed_from_scenario = true;
            }
            "monster_talks" if parts.len() >= 2 => {
                s.monster_talks = parts[1].parse().map_err(|_| "bad monster_talks")?;
                s.monster_talks_from_scenario = true;
            }
            "monster_load_type" if parts.len() >= 2 => {
                s.monster_load_type = parts[1] != "0";
            }
            "monster_state" if parts.len() >= 2 => {
                s.monster_initial_state = parse_monster_state(parts[1])?;
                s.monster_state_explicit = true;
            }
            "advance_ms" if parts.len() >= 2 => {
                let ms: u64 = parts[1].parse().map_err(|_| "bad advance_ms")?;
                s.steps.push(ScenarioStep::AdvanceMs(ms));
            }
            "monster_appear" => s.steps.push(ScenarioStep::MonsterAppear),
            "player_pos" if parts.len() >= 3 => {
                let x: u16 = parts[1].parse().map_err(|_| "bad player_pos x")?;
                let y: u16 = parts[2].parse().map_err(|_| "bad player_pos y")?;
                s.steps.push(ScenarioStep::PlayerPos(x, y));
            }
            "player_walk" if parts.len() >= 4 => {
                let x: u16 = parts[1].parse().map_err(|_| "bad player_walk x")?;
                let y: u16 = parts[2].parse().map_err(|_| "bad player_walk y")?;
                let ms: u64 = parts[3].parse().map_err(|_| "bad player_walk ms")?;
                s.steps.push(ScenarioStep::PlayerWalk(x, y, ms));
            }
            "sim_tick" => s.steps.push(ScenarioStep::SimTick),
            "player_damage" if parts.len() >= 2 => {
                let amount: i32 = parts[1].parse().map_err(|_| "bad player_damage")?;
                s.steps.push(ScenarioStep::PlayerDamage(amount));
            }
            "player_damage_monster" if parts.len() >= 3 => {
                let idx: usize = parts[1]
                    .parse()
                    .map_err(|_| "bad player_damage_monster idx")?;
                let amount: i32 = parts[2]
                    .parse()
                    .map_err(|_| "bad player_damage_monster amount")?;
                s.steps.push(ScenarioStep::PlayerDamageMonster(idx, amount));
            }
            other => return Err(format!("unknown scenario verb: {other}")),
        }
    }

    if s.name.is_empty() {
        return Err("missing scenario name".into());
    }
    if s.monsters.is_empty() {
        return Err(
            "scenario has no monster spawn(s) — add one or more `monster <label> x y` lines".into(),
        );
    }
    if s.steps.is_empty() {
        return Err("scenario has no script steps".into());
    }
    Ok(s)
}

struct SimHandles {
    player_id: tfs_rust_core::ids::CreatureId,
    monster_ids: Vec<tfs_rust_core::ids::CreatureId>,
    monsters_appeared: bool,
    player_walk_step: u32,
}

/// Cumulative `advance_ms` budget — caps drain fast-forward in `run_sim_tick`.
struct SimClock {
    wall_ms: u64,
}

impl SimClock {
    fn new() -> Self {
        Self { wall_ms: 0 }
    }

    fn apply_wall(&self, world: &mut tfs_rust_core::game_world::GameWorld) {
        set_sim_harness_wall_ms(world, Some(self.wall_ms));
    }

    fn advance(&mut self, world: &mut tfs_rust_core::game_world::GameWorld, ms: u64) {
        self.wall_ms = self.wall_ms.saturating_add(ms);
        set_sim_harness_wall_ms(world, Some(self.wall_ms));
        move_creatures_explicit(world, ms);
    }

    /// Bump wall without draining — paired with immediate `player_pos` (`chase_kite_scenario.cc`).
    fn bump_wall_only(&mut self, world: &mut tfs_rust_core::game_world::GameWorld, ms: u64) {
        self.wall_ms = self.wall_ms.saturating_add(ms);
        set_sim_harness_wall_ms(world, Some(self.wall_ms));
    }
}

fn scenario_advance_budget(steps: &[ScenarioStep]) -> u64 {
    steps
        .iter()
        .map(|step| match step {
            ScenarioStep::AdvanceMs(ms) => *ms,
            ScenarioStep::PlayerWalk(_, _, ms) => *ms,
            _ => 0,
        })
        .sum()
}

fn capitalize_monster(label: &str) -> String {
    let mut chars = label.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn scenario_walk_positions(scenario: &KiteScenario) -> Vec<Position> {
    let mut out = vec![Position::new(
        scenario.player_start.0,
        scenario.player_start.1,
        scenario.z,
    )];
    for spawn in &scenario.monsters {
        out.push(Position::new(spawn.pos.0, spawn.pos.1, scenario.z));
    }
    for step in &scenario.steps {
        match step {
            ScenarioStep::PlayerPos(x, y) | ScenarioStep::PlayerWalk(x, y, _) => {
                out.push(Position::new(*x, *y, scenario.z));
            }
            _ => {}
        }
    }
    out
}

fn build_world(
    scenario: &KiteScenario,
    map_cfg: &SimMapConfig,
) -> Result<tfs_rust_core::game_world::GameWorld, String> {
    let world = if map_cfg.synthetic_arena || scenario.arena_synthetic {
        beat_driven_world_for_kite_synthetic(
            &map_cfg.data_dir,
            &map_cfg.map_rel,
            scenario.arena_center,
            scenario.arena_radius,
            scenario.z,
            scenario.default_wp,
        )?
    } else {
        let mut w = beat_driven_world_from_map(&map_cfg.data_dir, &map_cfg.map_rel)?;
        validate_positions_walkable(&w.map, &scenario_walk_positions(scenario), "scenario")?;
        set_harness_real_map(&mut w, true);
        w
    };
    Ok(world)
}

fn scenario_monster_config(scenario: &KiteScenario) -> MonsterAiConfig {
    let mut config = MonsterAiConfig::default();
    config.is_hostile = scenario.monster_hostile;
    config.target_distance = scenario.monster_target_distance;
    config.melee_skill = scenario.monster_melee_skill;
    config.melee_attack = scenario.monster_melee_attack;
    config.armor = scenario.monster_armor;
    config.defense = scenario.monster_defense;
    config.talks = scenario.monster_talks;
    config
}

fn spawn_entities(
    world: &mut tfs_rust_core::game_world::GameWorld,
    scenario: &KiteScenario,
) -> Result<SimHandles, String> {
    let z = scenario.z;
    let player_pos = Position::new(scenario.player_start.0, scenario.player_start.1, z);

    let player_id = insert_player(world, sim_hero_player(&scenario.player_name, player_pos));
    if scenario.player_health_from_scenario {
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player_id) {
            p.base.health = scenario.player_health;
            p.base.max_health = scenario.player_health;
        }
    }
    world.map.register_creature_at(player_pos, player_id);

    let config = scenario_monster_config(scenario);

    let mut monster_ids = Vec::with_capacity(scenario.monsters.len());
    for (idx, spawn) in scenario.monsters.iter().enumerate() {
        let monster_pos = Position::new(spawn.pos.0, spawn.pos.1, z);
        let monster_name = if scenario.monsters.len() == 1 {
            capitalize_monster(&spawn.label)
        } else {
            format!("{} {}", capitalize_monster(&spawn.label), idx + 1)
        };

        let type_key = spawn.label.to_ascii_lowercase();
        let mtype_owned = scenario
            .monster_load_type
            .then(|| world.monsters_db.monsters.get(&type_key).cloned())
            .flatten();

        let speed = if scenario.monster_speed_from_scenario {
            scenario.monster_speed
        } else {
            mtype_owned
                .as_ref()
                .map(|t| t.speed as i32)
                .unwrap_or(scenario.monster_speed)
        };

        let monster_id = if let Some(ref mtype) = mtype_owned {
            let mut typed_config = MonsterAiConfig::from_monster_type(mtype);
            typed_config.is_hostile = config.is_hostile;
            if scenario.monster_melee_skill_from_scenario {
                typed_config.melee_skill = config.melee_skill;
            }
            if scenario.monster_melee_attack_from_scenario {
                typed_config.melee_attack = config.melee_attack;
            }
            if scenario.monster_armor_from_scenario {
                typed_config.armor = config.armor;
            }
            if scenario.monster_defense_from_scenario {
                typed_config.defense = config.defense;
            }
            if scenario.monster_target_distance_from_scenario {
                typed_config.target_distance = config.target_distance;
            }
            if scenario.monster_talks_from_scenario {
                typed_config.talks = config.talks;
            }
            insert_monster_from_type(
                world,
                mtype,
                &monster_name,
                monster_pos,
                speed,
                typed_config,
                scenario.monster_initial_state,
            )
        } else {
            if scenario.monster_load_type {
                return Err(format!(
                    "monster_load_type: unknown type '{type_key}' in monsters db"
                ));
            }
            let monster_id = insert_monster_with_config(
                world,
                &monster_name,
                monster_pos,
                speed,
                config.clone(),
            );
            if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster_id) {
                m.state = scenario.monster_initial_state;
                m.is_idle = true;
            }
            monster_id
        };

        monster_ids.push(monster_id);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster_id) {
            m.harness_spawn_order = (idx as u16).saturating_add(1);
        }
        if harness_place_creature_login(world, monster_id, monster_pos).is_none()
        {
            return Err(format!(
                "harness spawn: cannot place monster at [{},{},{}]",
                monster_pos.x, monster_pos.y, monster_pos.z
            ));
        }
        if scenario.monster_state_explicit
            && scenario.monster_initial_state == MonsterState::Sleeping
        {
            if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster_id) {
                m.harness_preserve_sleep = true;
            }
        }
    }

    Ok(SimHandles {
        player_id,
        monster_ids,
        monsters_appeared: false,
        player_walk_step: 0,
    })
}

fn step_followed_by_player_pos(steps: &[ScenarioStep], idx: usize) -> bool {
    matches!(steps.get(idx + 1), Some(ScenarioStep::PlayerPos(_, _)))
}

/// `advance_ms` + `player_pos` pair — drain to wall with the old tile, then teleport (`chase_kite_scenario.cc`).
fn player_pos_drains_before_teleport(steps: &[ScenarioStep], idx: usize) -> bool {
    let Some(ScenarioStep::AdvanceMs(_)) = steps.get(idx.wrapping_sub(1)) else {
        return false;
    };
    step_followed_by_player_pos(steps, idx - 1)
}

fn execute_step(
    world: &mut tfs_rust_core::game_world::GameWorld,
    handles: &mut SimHandles,
    clock: &mut SimClock,
    scenario: &KiteScenario,
    step: &ScenarioStep,
    defer_advance_drain: bool,
    player_pos_idx: Option<usize>,
) -> Result<(), String> {
    clock.apply_wall(world);
    match step {
        ScenarioStep::AdvanceMs(ms) if defer_advance_drain => {
            clock.bump_wall_only(world, *ms);
            set_sim_harness_segment_ms(world, Some(*ms));
        }
        ScenarioStep::AdvanceMs(ms) => {
            clock.advance(world, *ms);
            set_sim_harness_segment_ms(world, Some(*ms));
        }
        ScenarioStep::MonsterAppear => {
            if !handles.monsters_appeared {
                kite_monsters_appear_batch(world, &handles.monster_ids);
                handles.monsters_appeared = true;
            }
            run_sim_tick(world);
        }
        ScenarioStep::PlayerPos(x, y) => {
            let pos = Position::new(*x, *y, scenario.z);
            let drain_first = player_pos_idx
                .is_some_and(|idx| player_pos_drains_before_teleport(&scenario.steps, idx));
            if drain_first {
                run_sim_tick(world);
            }
            teleport_player(world, handles.player_id, pos)?;
            if !drain_first {
                run_sim_tick(world);
            }
        }
        ScenarioStep::PlayerWalk(x, y, ms) => {
            let pos = Position::new(*x, *y, scenario.z);
            clock.advance(world, *ms);
            set_sim_harness_segment_ms(world, Some(*ms));
            // C++ `chase_kite_scenario.cc` — `MoveCreatures` one-pass drain after defer pull,
            // then `MoveKitePlayer` + `DrainTodoQueue` post-walk (not 64-round pre-walk drain).
            clear_harness_appear_idle_defer(world, &handles.monster_ids);
            drain_todo_queue_once(world);
            walk_player_adjacent(world, handles.player_id, pos)?;
            let step = handles.player_walk_step;
            handles.player_walk_step = handles.player_walk_step.saturating_add(1);
            log_harness_player_step(world.chase_trace_tick(), step, pos);
            run_sim_tick(world);
        }
        ScenarioStep::SimTick => run_sim_tick(world),
        ScenarioStep::PlayerDamage(amount) => {
            for &monster_id in &handles.monster_ids {
                if world.creatures.contains_key(monster_id) {
                    sim_player_damage_monster(world, handles.player_id, monster_id, *amount);
                }
            }
            run_sim_tick(world);
        }
        ScenarioStep::PlayerDamageMonster(idx, amount) => {
            let monster_id = handles
                .monster_ids
                .get(*idx)
                .copied()
                .ok_or_else(|| format!("player_damage_monster: invalid index {idx}"))?;
            if world.creatures.contains_key(monster_id) {
                sim_player_damage_monster(world, handles.player_id, monster_id, *amount);
            }
            run_sim_tick(world);
        }
    }
    Ok(())
}

fn run_scenario(scenario: &KiteScenario, map_cfg: &SimMapConfig) -> Result<(), String> {
    let mut world = build_world(scenario, map_cfg)?;
    let mut handles = spawn_entities(&mut world, scenario)?;
    let mut clock = SimClock::new();
    clock.apply_wall(&mut world);
    for (idx, step) in scenario.steps.iter().enumerate() {
        let player_pos_idx = matches!(step, ScenarioStep::PlayerPos(_, _)).then_some(idx);
        execute_step(
            &mut world,
            &mut handles,
            &mut clock,
            scenario,
            step,
            step_followed_by_player_pos(&scenario.steps, idx),
            player_pos_idx,
        )?;
    }
    Ok(())
}

fn scenario_route_positions(scenario: &KiteScenario) -> Vec<Position> {
    let z = scenario.z;
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let mut push = |x: u16, y: u16| {
        let key = (x, y, z);
        if seen.insert(key) {
            out.push(Position::new(x, y, z));
        }
    };
    push(scenario.player_start.0, scenario.player_start.1);
    for m in &scenario.monsters {
        push(m.pos.0, m.pos.1);
    }
    for step in &scenario.steps {
        if let ScenarioStep::PlayerWalk(x, y, _) = step {
            push(*x, *y);
        }
    }
    out
}

fn run_audit_route(raw: &[String]) -> Result<(), String> {
    if raw.is_empty() {
        return Err(
            "usage: chase_kite_sim --audit-route <scenario> [--data-dir DIR] [--map REL]".into(),
        );
    }
    let scenario_path = PathBuf::from(&raw[0]);
    let mut map_cfg = default_sim_map_config();
    let mut i = 1;
    while i < raw.len() {
        match raw[i].as_str() {
            "--data-dir" => {
                let path = raw
                    .get(i + 1)
                    .ok_or_else(|| "--data-dir requires a path".to_string())?;
                map_cfg.data_dir = PathBuf::from(path);
                i += 2;
            }
            "--map" => {
                let rel = raw
                    .get(i + 1)
                    .ok_or_else(|| "--map requires a relative path".to_string())?;
                map_cfg.map_rel = rel.clone();
                i += 2;
            }
            other => return Err(format!("unexpected argument: {other}")),
        }
    }

    let input = fs::read_to_string(&scenario_path)
        .map_err(|e| format!("read {}: {e}", scenario_path.display()))?;
    let scenario = parse_scenario(&input)?;
    let positions = scenario_route_positions(&scenario);
    eprintln!(
        "chase_kite_sim: audit-route '{}' ({} tiles) map={}/{}",
        scenario.name,
        positions.len(),
        map_cfg.data_dir.display(),
        map_cfg.map_rel
    );

    let world = beat_driven_world_from_map(&map_cfg.data_dir, &map_cfg.map_rel)?;
    let audits = audit_otbm_route_tiles(&world.map, world.items_db.as_ref(), &positions);
    let mut stdout = std::io::stdout().lock();
    write_audit_route_json(&audits, &mut stdout).map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    if let Err(e) = run_main() {
        eprintln!("chase_kite_sim: {e}");
        std::process::exit(1);
    }
}

fn run_main() -> Result<(), String> {
    let raw: Vec<String> = env::args().skip(1).collect();
    if raw.is_empty() {
        return Err(
            "usage: chase_kite_sim <scenario> [--log PATH] [--data-dir DIR] [--map REL] [--synthetic]\n       chase_kite_sim --audit-route <scenario> [--data-dir DIR] [--map REL]"
                .into(),
        );
    }
    if raw[0] == "--audit-route" {
        return run_audit_route(&raw[1..]);
    }
    let scenario_path = PathBuf::from(&raw[0]);
    let mut log_path = None;
    let mut map_cfg = default_sim_map_config();
    let mut i = 1;
    while i < raw.len() {
        match raw[i].as_str() {
            "--log" => {
                let path = raw
                    .get(i + 1)
                    .ok_or_else(|| "--log requires a path".to_string())?;
                log_path = Some(PathBuf::from(path));
                i += 2;
            }
            "--data-dir" => {
                let path = raw
                    .get(i + 1)
                    .ok_or_else(|| "--data-dir requires a path".to_string())?;
                map_cfg.data_dir = PathBuf::from(path);
                i += 2;
            }
            "--map" => {
                let rel = raw
                    .get(i + 1)
                    .ok_or_else(|| "--map requires a relative path".to_string())?;
                map_cfg.map_rel = rel.clone();
                i += 2;
            }
            "--synthetic" => {
                map_cfg.synthetic_arena = true;
                i += 1;
            }
            other => return Err(format!("unexpected argument: {other}")),
        }
    }

    env::set_var("TFS_CHASE_PATH_DEBUG", "1");
    if let Some(ref path) = log_path {
        env::set_var("TFS_CHASE_PATH_LOG", path);
    }
    tfs_rust_core::sim_harness::reset_chase_path_log();

    let input = fs::read_to_string(&scenario_path)
        .map_err(|e| format!("read {}: {e}", scenario_path.display()))?;
    let scenario = parse_scenario(&input)?;
    if scenario.arena_synthetic {
        map_cfg.synthetic_arena = true;
    }
    let wall_budget = scenario_advance_budget(&scenario.steps);
    eprintln!(
        "chase_kite_sim: scenario '{}' ({} steps, {} monsters) wall_ms={} load_type={} map={}/{} synthetic={}",
        scenario.name,
        scenario.steps.len(),
        scenario.monsters.len(),
        wall_budget,
        scenario.monster_load_type,
        map_cfg.data_dir.display(),
        map_cfg.map_rel,
        map_cfg.synthetic_arena
    );
    run_scenario(&scenario, &map_cfg)?;
    eprintln!("chase_kite_sim: done");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kite_rat_melee_scenario() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/scenarios/kite_rat_melee.scenario"
        );
        let input = fs::read_to_string(path).expect("read scenario");
        let s = parse_scenario(&input).expect("parse");
        assert_eq!(s.name, "kite_rat_melee");
        assert_eq!(s.arena_center, (32360, 32290));
        assert_eq!(s.monsters.len(), 1);
        assert_eq!(s.monsters[0].label, "rat");
        assert!(s.monster_load_type);
        assert!(s
            .steps
            .iter()
            .any(|st| matches!(st, ScenarioStep::MonsterAppear)));
        assert!(
            s.steps
                .iter()
                .filter(|st| matches!(st, ScenarioStep::SimTick))
                .count()
                >= 2
        );
    }

    #[test]
    fn parses_kite_cyclops_quad_scenario() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/scenarios/kite_cyclops_quad_chase.scenario"
        );
        let input = fs::read_to_string(path).expect("read scenario");
        let s = parse_scenario(&input).expect("parse");
        assert_eq!(s.name, "kite_cyclops_quad_chase");
        assert_eq!(s.monsters.len(), 4);
        assert!(s.monsters.iter().all(|m| m.label == "cyclops"));
        assert_eq!(s.monster_melee_skill, 50);
        assert!(s.monster_melee_skill_from_scenario);
        assert_eq!(s.monster_melee_attack, 30);
        assert!(s.monster_melee_attack_from_scenario);
        assert_eq!(s.monster_talks, 5);
        assert!(s.monster_talks_from_scenario);
    }

    #[test]
    fn parses_player_damage_and_monster_state() {
        let input = r#"
name test
monster rat 1 2
monster_state sleeping
player_damage 5
monster_appear
"#;
        let s = parse_scenario(input).expect("parse");
        assert_eq!(s.monster_initial_state, MonsterState::Sleeping);
        assert!(matches!(s.steps[0], ScenarioStep::PlayerDamage(5)));
    }

    #[test]
    fn parses_player_walk_verb() {
        let input = r#"
name test_walk
monster rat 1 2
player_walk 3 2 400
sim_tick
"#;
        let s = parse_scenario(input).expect("parse");
        assert!(matches!(s.steps[0], ScenarioStep::PlayerWalk(3, 2, 400)));
    }

    #[test]
    fn scenario_advance_budget_includes_player_walk_ms() {
        let input = r#"
name budget
monster rat 1 2
advance_ms 100
player_walk 3 2 400
sim_tick
"#;
        let s = parse_scenario(input).expect("parse");
        assert_eq!(scenario_advance_budget(&s.steps), 500);
    }

    #[test]
    fn parses_hunter_dist_chase_scenario() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/scenarios/kite_hunter_dist_chase.scenario"
        );
        let input = fs::read_to_string(path).expect("read scenario");
        let s = parse_scenario(&input).expect("parse");
        assert_eq!(s.name, "kite_hunter_dist_chase");
        assert_eq!(s.monsters[0].label, "hunter");
        assert!(!s.monster_target_distance_from_scenario);
        assert!(s.arena_synthetic);
    }

    #[test]
    fn parses_kite_cyclops_one_real_scenario() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/scenarios/kite_cyclops_one_real.scenario"
        );
        let input = fs::read_to_string(path).expect("read scenario");
        let s = parse_scenario(&input).expect("parse");
        assert_eq!(s.name, "kite_cyclops_one_real");
        assert_eq!(s.monsters.len(), 1);
        assert_eq!(s.monsters[0].label, "cyclops");
        assert_eq!(s.monsters[0].pos, (32453, 32065));
        assert!(!s.arena_synthetic);
        assert_eq!(
            s.steps
                .iter()
                .filter(|st| matches!(st, ScenarioStep::PlayerWalk(_, _, _)))
                .count(),
            5
        );
    }

    #[test]
    fn parses_dragon_lowhp_flee_scenario() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/scenarios/kite_dragon_lowhp_flee.scenario"
        );
        let input = fs::read_to_string(path).expect("read scenario");
        let s = parse_scenario(&input).expect("parse");
        assert_eq!(s.name, "kite_dragon_lowhp_flee");
        assert_eq!(s.monsters[0].label, "dragon");
        assert!(s
            .steps
            .iter()
            .any(|st| matches!(st, ScenarioStep::PlayerDamage(725))));
    }

    /// P2 — OTBM route audit for real-map cyclops control scenario.
    #[test]
    fn audit_route_kite_cyclops_one_real() {
        use tfs_rust_core::sim_harness::{audit_otbm_route_tiles, default_sim_map_config};

        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/scenarios/kite_cyclops_one_real.scenario"
        );
        let input = fs::read_to_string(path).expect("read scenario");
        let s = parse_scenario(&input).expect("parse");
        let cfg = default_sim_map_config();
        if !cfg.data_dir.is_dir() {
            return;
        }
        let Ok(world) = beat_driven_world_from_map(&cfg.data_dir, &cfg.map_rel) else {
            return;
        };
        let positions = scenario_route_positions(&s);
        let audits = audit_otbm_route_tiles(&world.map, world.items_db.as_ref(), &positions);
        let start = audits
            .iter()
            .find(|t| t.x == 32451 && t.y == 32065 && t.z == 7)
            .expect("player_start tile");
        assert!(start.exists, "player_start must exist on OTBM");
        assert!(start.walkable, "player_start must be walkable");
        assert_eq!(start.wp, 150, "player_start gravel wp");
    }
}
