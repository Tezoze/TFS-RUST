//! Pre-expand authored 772 waypoint loops to cardinal walk opcodes.
//!
//! Open-loop: no runtime pathfinder, no inbound `pos` replan. A bad CSV is a
//! data bug. Wire: client `MOVE_NORTH`..`MOVE_WEST` (`protocol_opcodes`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use tfs_rust_common::Position;
use tfs_rust_common::protocol_opcodes::client;

use crate::scenario::BotRng;

/// `# lane_clamp: N` in the CSV comment (default 1).
const DEFAULT_LANE_CLAMP: i32 = 1;

#[derive(Debug, Clone)]
pub struct WaypointLoop {
    pub tiles: Vec<Position>,
    pub lane_clamp: i32,
}

/// Resolve a repo-relative waypoint path (cwd first, then crate → repo root).
pub fn resolve_waypoint_path(file: &str) -> PathBuf {
    let p = Path::new(file);
    if p.is_file() {
        return p.to_path_buf();
    }
    let from_crate = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(file);
    if from_crate.is_file() {
        return from_crate;
    }
    p.to_path_buf()
}

pub fn load_csv(path: &Path) -> Result<WaypointLoop> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read waypoint csv {}", path.display()))?;
    parse_csv(&text).with_context(|| format!("parse {}", path.display()))
}

pub fn parse_csv(text: &str) -> Result<WaypointLoop> {
    let mut lane_clamp = DEFAULT_LANE_CLAMP;
    let mut saw_header = false;
    let mut tiles = Vec::new();
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            if let Some(n) = parse_lane_clamp_comment(line) {
                lane_clamp = n;
            }
            continue;
        }
        if !saw_header {
            let lower = line.to_ascii_lowercase();
            anyhow::ensure!(
                lower == "x,y,z",
                "line {}: expected header x,y,z, got {line}",
                lineno + 1
            );
            saw_header = true;
            continue;
        }
        let mut parts = line.split(',');
        let x = parse_coord(parts.next(), "x", lineno + 1)?;
        let y = parse_coord(parts.next(), "y", lineno + 1)?;
        let z = parse_coord(parts.next(), "z", lineno + 1)?;
        if parts.next().is_some() {
            bail!("line {}: extra CSV fields", lineno + 1);
        }
        let z = u8::try_from(z).map_err(|_| anyhow!("line {}: z out of range", lineno + 1))?;
        tiles.push(Position::new(x, y, z));
    }
    anyhow::ensure!(saw_header, "waypoint csv missing x,y,z header");
    anyhow::ensure!(
        tiles.len() >= 2,
        "waypoint csv needs at least two tiles, got {}",
        tiles.len()
    );
    Ok(WaypointLoop {
        tiles,
        lane_clamp: lane_clamp.max(0),
    })
}

/// Cardinal opcodes covering `n_steps` intended walks (phase / lane / jitter folded in).
pub fn expand(wp: &WaypointLoop, n_steps: usize, bot_index: usize, rng: &mut BotRng) -> Vec<u8> {
    if n_steps == 0 || wp.tiles.is_empty() {
        return Vec::new();
    }
    let n = wp.tiles.len();
    let start = bot_index % n;
    let lane = lane_sign(rng, wp.lane_clamp);
    let h_first = rng.next_u64().is_multiple_of(2);
    let mut out = Vec::with_capacity(n_steps);
    let mut seg = 0usize;
    while out.len() < n_steps {
        let a = wp.tiles[(start + seg) % n];
        let b = wp.tiles[(start + seg + 1) % n];
        let (from, to) = apply_lane(a, b, lane);
        let mut steps = cardinals_between(from, to, h_first);
        if steps.is_empty() {
            seg += 1;
            if seg > n * 2 {
                break;
            }
            continue;
        }
        let need = n_steps - out.len();
        if steps.len() > need {
            steps.truncate(need);
        }
        out.extend_from_slice(&steps);
        seg += 1;
    }
    out
}

fn parse_lane_clamp_comment(line: &str) -> Option<i32> {
    let rest = line.trim_start_matches('#').trim();
    let lower = rest.to_ascii_lowercase();
    let key = "lane_clamp";
    let idx = lower.find(key)?;
    let after = rest[idx + key.len()..].trim_start();
    let after = after.trim_start_matches([':', '=']).trim_start();
    after
        .split(|c: char| !c.is_ascii_digit() && c != '-')
        .next()
        .and_then(|s| s.parse().ok())
}

fn parse_coord(raw: Option<&str>, name: &str, line: usize) -> Result<u16> {
    let s = raw
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("line {line}: missing {name}"))?;
    s.parse::<u16>()
        .with_context(|| format!("line {line}: bad {name} {s}"))
}

fn lane_sign(rng: &mut BotRng, clamp: i32) -> i32 {
    if clamp <= 0 {
        return 0;
    }
    if rng.next_u64().is_multiple_of(2) {
        clamp
    } else {
        -clamp
    }
}

/// Offset both endpoints perpendicular to the segment, then clamp by `lane`.
fn apply_lane(from: Position, to: Position, lane: i32) -> (Position, Position) {
    if lane == 0 {
        return (from, to);
    }
    let dx = i32::from(to.x) - i32::from(from.x);
    let dy = i32::from(to.y) - i32::from(from.y);
    let (ox, oy) = if dx.abs() >= dy.abs() {
        (0, lane)
    } else {
        (lane, 0)
    };
    (shift(from, ox, oy), shift(to, ox, oy))
}

fn shift(p: Position, dx: i32, dy: i32) -> Position {
    let x = (i32::from(p.x) + dx).clamp(0, i32::from(u16::MAX)) as u16;
    let y = (i32::from(p.y) + dy).clamp(0, i32::from(u16::MAX)) as u16;
    Position::new(x, y, p.z)
}

fn cardinals_between(from: Position, to: Position, h_first: bool) -> Vec<u8> {
    let dx = i32::from(to.x) - i32::from(from.x);
    let dy = i32::from(to.y) - i32::from(from.y);
    let mut out = Vec::new();
    let horiz = || {
        let op = if dx > 0 {
            client::MOVE_EAST
        } else {
            client::MOVE_WEST
        };
        std::iter::repeat_n(op, dx.unsigned_abs() as usize)
    };
    let vert = || {
        let op = if dy > 0 {
            client::MOVE_SOUTH
        } else {
            client::MOVE_NORTH
        };
        std::iter::repeat_n(op, dy.unsigned_abs() as usize)
    };
    if h_first {
        out.extend(horiz());
        out.extend(vert());
    } else {
        out.extend(vert());
        out.extend(horiz());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::BotRng;

    fn square() -> WaypointLoop {
        WaypointLoop {
            tiles: vec![
                Position::new(10, 10, 7),
                Position::new(11, 10, 7),
                Position::new(11, 11, 7),
                Position::new(10, 11, 7),
            ],
            lane_clamp: 0,
        }
    }

    #[test]
    fn four_tile_square_four_cardinals_per_loop() {
        let wp = square();
        let mut rng = BotRng::new(1);
        let ops = expand(&wp, 8, 0, &mut rng);
        assert_eq!(ops.len(), 8);
        let loop4 = &ops[..4];
        let expected = [
            client::MOVE_EAST,
            client::MOVE_SOUTH,
            client::MOVE_WEST,
            client::MOVE_NORTH,
        ];
        let expected_swap = [
            client::MOVE_SOUTH,
            client::MOVE_EAST,
            client::MOVE_NORTH,
            client::MOVE_WEST,
        ];
        assert!(loop4 == expected || loop4 == expected_swap, "got {loop4:?}");
        assert_eq!(&ops[4..8], loop4);
    }

    #[test]
    fn two_bot_seeds_differ_at_prefix() {
        let wp = WaypointLoop {
            tiles: square().tiles,
            lane_clamp: 1,
        };
        let mut a = BotRng::new(1);
        let mut b = BotRng::new(2);
        let left = expand(&wp, 16, 0, &mut a);
        let right = expand(&wp, 16, 1, &mut b);
        assert_ne!(left, right);
    }

    #[test]
    fn parse_csv_header_and_comments() {
        let text = "# thais temple ring\n# lane_clamp: 1\nx,y,z\n32369,32241,7\n32364,32241,7\n";
        let wp = parse_csv(text).expect("csv");
        assert_eq!(wp.tiles.len(), 2);
        assert_eq!(wp.lane_clamp, 1);
        assert_eq!(wp.tiles[0], Position::new(32369, 32241, 7));
    }

    #[test]
    fn parse_csv_rejects_bad_header() {
        assert!(parse_csv("a,b,c\n1,2,3\n").is_err());
    }

    #[test]
    fn load_missing_file_fails() {
        assert!(load_csv(Path::new("/no/such/waypoints.csv")).is_err());
    }

    #[test]
    fn authored_thais_depot_loop_loads() {
        let path = resolve_waypoint_path("bench/waypoints/thais_depot_loop.csv");
        let wp = load_csv(&path).expect("thais_depot_loop");
        assert!(wp.tiles.len() >= 4);
        assert_eq!(wp.tiles[0], Position::new(32369, 32241, 7));
    }
}
