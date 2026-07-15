//! CLI for chase path comparison — reads scenario text from stdin, prints JSON.
//!
//! Used by `scripts/compare_chase_pathfinding.py`. 772 reference: `cract.cc` `TShortway`.
//! Rust path: reverse `TShortway` + terrain waypoint costs + 3-step queue trim.

use std::collections::HashMap;
use std::io::{self, Read};

use tfs_rust_common::enums::Direction;
use tfs_rust_common::enums::ZoneType;
use tfs_rust_common::Position;
use tfs_rust_core::formulas::{PathCostModel, PathSearchModel};
use tfs_rust_core::map::{Map, SparseGrid};
use tfs_rust_core::pathfinding::{
    effective_terrain_waypoints, get_path_matching, truncate_tshortway_go_queue, FindPathParams,
    CHASE_PATH_MAX_STEPS,
};
use tfs_rust_core::tile::{Tile, TileBody};

#[derive(Debug, Default)]
struct Scenario {
    name: String,
    start: (u16, u16),
    target: (u16, u16),
    visible: i32,
    max_steps: usize,
    default_wp: u32,
    blocked: HashMap<(u16, u16), ()>,
    waypoints: HashMap<(u16, u16), u32>,
}

fn parse_scenario(input: &str) -> Scenario {
    let mut s = Scenario {
        visible: 10,
        max_steps: CHASE_PATH_MAX_STEPS,
        default_wp: 150,
        ..Default::default()
    };
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        let Some(key) = parts.first() else { continue };
        match *key {
            "name" if parts.len() >= 2 => s.name = parts[1].to_string(),
            "start" if parts.len() >= 3 => {
                s.start = (parts[1].parse().unwrap_or(0), parts[2].parse().unwrap_or(0));
            }
            "target" if parts.len() >= 3 => {
                s.target = (parts[1].parse().unwrap_or(0), parts[2].parse().unwrap_or(0));
            }
            "visible" if parts.len() >= 2 => s.visible = parts[1].parse().unwrap_or(10),
            "max_steps" if parts.len() >= 2 => s.max_steps = parts[1].parse().unwrap_or(3),
            "default_wp" if parts.len() >= 2 => s.default_wp = parts[1].parse().unwrap_or(150),
            "block" if parts.len() >= 3 => {
                let x: u16 = parts[1].parse().unwrap_or(0);
                let y: u16 = parts[2].parse().unwrap_or(0);
                s.blocked.insert((x, y), ());
            }
            "wp" if parts.len() >= 4 => {
                let x: u16 = parts[1].parse().unwrap_or(0);
                let y: u16 = parts[2].parse().unwrap_or(0);
                let wp: u32 = parts[3].parse().unwrap_or(150);
                s.waypoints.insert((x, y), wp);
            }
            _ => {}
        }
    }
    s
}

fn wp_at(s: &Scenario, x: u16, y: u16) -> Option<u32> {
    if s.blocked.contains_key(&(x, y)) {
        return None;
    }
    Some(*s.waypoints.get(&(x, y)).unwrap_or(&s.default_wp))
}

fn build_map(s: &Scenario) -> Map {
    let mut map = Map {
        width: 256,
        height: 256,
        grid: SparseGrid::new(),
        towns: HashMap::new(),
        waypoints: HashMap::new(),
    };
    let pad = s.visible.max(12) as u16 + 2;
    let min_x = s.start.0.min(s.target.0).saturating_sub(pad);
    let max_x = s.start.0.max(s.target.0).saturating_add(pad);
    let min_y = s.start.1.min(s.target.1).saturating_sub(pad);
    let max_y = s.start.1.max(s.target.1).saturating_add(pad);
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            if let Some(wp) = wp_at(s, x, y) {
                map.insert_tile(
                    Position::new(x, y, 7),
                    Tile::Normal(TileBody {
                        ground: Some(wp as u16),
                        down_items: Vec::new(),
                        top_items: Vec::new(),
                        creatures: Vec::new(),
                        flags: 0,
                        zone: ZoneType::Normal,
                    }),
                );
            }
        }
    }
    map
}

fn dir_name(d: Direction) -> &'static str {
    match d {
        Direction::North => "N",
        Direction::East => "E",
        Direction::South => "S",
        Direction::West => "W",
        Direction::NorthEast => "NE",
        Direction::NorthWest => "NW",
        Direction::SouthEast => "SE",
        Direction::SouthWest => "SW",
    }
}

fn is_diagonal(d: Direction) -> bool {
    matches!(
        d,
        Direction::NorthEast | Direction::NorthWest | Direction::SouthEast | Direction::SouthWest
    )
}

fn apply_dirs(mut pos: Position, dirs: &[Direction]) -> Vec<[u16; 2]> {
    let mut tiles = Vec::new();
    for &d in dirs {
        pos = pos.offset(d);
        tiles.push([pos.x, pos.y]);
    }
    tiles
}

fn path_total_cost(map: &Map, start: Position, dirs: &[Direction]) -> u32 {
    let mut total = 0u32;
    let mut pos = start;
    for &d in dirs {
        let wp = effective_terrain_waypoints(
            map.get_tile(pos)
                .and_then(|t| t.body().ground.map(|g| g as u32))
                .unwrap_or(0),
        );
        let step = if is_diagonal(d) { wp * 3 } else { wp };
        total = total.saturating_add(step);
        pos = pos.offset(d);
    }
    total
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{{\"ok\":false,\"error\":\"{e}\"}}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| e.to_string())?;
    let scenario = parse_scenario(&input);
    if scenario.name.is_empty() {
        return Err("missing scenario name".into());
    }

    let map = build_map(&scenario);
    let start = Position::new(scenario.start.0, scenario.start.1, 7);
    let target = Position::new(scenario.target.0, scenario.target.1, 7);

    let fpp = FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: false,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    };

    let ground = |pos: Position| -> u32 {
        if !map.is_walkable(pos) {
            return 0;
        }
        effective_terrain_waypoints(
            map.get_tile(pos)
                .and_then(|t| t.body().ground.map(|g| g as u32))
                .unwrap_or(0),
        )
    };

    let path = get_path_matching(
        &map,
        start,
        target,
        &fpp,
        PathCostModel::TerrainWeighted,
        PathSearchModel::Reverse,
        false,
        scenario.visible,
        |pos| map.is_walkable(pos),
        |_pos| 0u32,
        ground,
    );

    let Some(path) = path else {
        println!(
            "{{\"ok\":false,\"name\":\"{}\",\"error\":\"NOWAY\"}}",
            scenario.name
        );
        return Ok(());
    };

    let dirs = truncate_tshortway_go_queue(start, target, path, scenario.max_steps, false);

    let dir_names: Vec<&str> = dirs.iter().copied().map(dir_name).collect();
    let tiles = apply_dirs(start, &dirs);
    let total_cost = path_total_cost(&map, start, &dirs);
    let diagonals = dirs.iter().filter(|&&d| is_diagonal(d)).count();

    print!("{{\"ok\":true,\"name\":\"{}\",", scenario.name);
    print!("\"dirs\":[");
    for (i, d) in dir_names.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        print!("\"{d}\"");
    }
    print!("],\"tiles\":[");
    for (i, t) in tiles.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        print!("[{},{}]", t[0], t[1]);
    }
    println!("],\"total_cost\":{total_cost},\"diagonals\":{diagonals}}}");
    Ok(())
}
