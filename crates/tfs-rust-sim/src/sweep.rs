//! In-process scaling sweep — full `AdvanceGame` via `advance_beat`.
//!
//! Parity `.scenario` runs keep `move_creatures` (`docs/SIM_HARNESS.md` §3.3). This
//! sweep is not a parity contract: it saturates the game thread and reports
//! `GameObs` subsystem µs plus externally timed beat wall µs.
//!
//! `advance_beat` records subsystem histograms but does **not** call
//! `GameObs::record_beat` (that lives in `game_loop.rs`). Beat wall is therefore
//! timed here with `Instant`.

use std::time::Instant;

use tfs_rust_common::ConnId;
use tfs_rust_common::Position;
use tfs_rust_core::game_world::GameWorld;
use tfs_rust_core::ids::CreatureId;
use tfs_rust_core::obs::GameObs;

use crate::population::{
    WalkRng, queue_random_walks, spawn_hero, spawn_hero_with_conn, spawn_monster_ring,
    spawn_player_grid, spawn_spectator_grid,
};
use crate::scenario::{
    beat_driven_world_for_kite_synthetic, beat_driven_world_from_map, default_sim_map_config,
};

/// Kite-bowl center used with OTBM / `beat_driven_world_for_kite_synthetic`.
pub const KITE_CENTER: Position = Position {
    x: 32360,
    y: 32290,
    z: 7,
};

/// Center inside the 256×256 empty synthetic map (`beat_driven_world`).
pub const SYNTHETIC_TEST_CENTER: Position = Position {
    x: 128,
    y: 128,
    z: 7,
};

const DEFAULT_MONSTER: &str = "cyclops";
const DEFAULT_WP: u16 = 150;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepAxis {
    Monsters,
    Players,
    Spectators,
}

impl SweepAxis {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "monsters" => Ok(Self::Monsters),
            "players" => Ok(Self::Players),
            "spectators" => Ok(Self::Spectators),
            other => Err(format!(
                "unknown --axis {other} (expected monsters|players|spectators)"
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Monsters => "monsters",
            Self::Players => "players",
            Self::Spectators => "spectators",
        }
    }

    fn needs_walks(self) -> bool {
        matches!(self, Self::Players | Self::Spectators)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepMap {
    Synthetic,
    Otbm,
}

impl SweepMap {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "synthetic" => Ok(Self::Synthetic),
            "otbm" => Ok(Self::Otbm),
            other => Err(format!("unknown --map {other} (expected synthetic|otbm)")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Synthetic => "synthetic",
            Self::Otbm => "otbm",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SweepPoint {
    pub n: usize,
    pub beats: u32,
    pub warmup: u32,
}

#[derive(Debug, Clone)]
pub struct SweepResult {
    pub n: usize,
    pub beat_wall_samples: u64,
    pub beat_wall_us_p50: u64,
    pub beat_wall_us_p95: u64,
    pub beat_wall_us_p99: u64,
    pub beat_wall_us_max: u64,
    pub creatures_us_p50: u64,
    pub creatures_us_p95: u64,
    pub creatures_us_p99: u64,
    pub skills_us_p50: u64,
    pub skills_us_p95: u64,
    pub skills_us_p99: u64,
    pub todo_us_p50: u64,
    pub todo_us_p95: u64,
    pub todo_us_p99: u64,
    pub path_us_p50: u64,
    pub path_us_p95: u64,
    pub path_us_p99: u64,
    pub path_searches: u64,
    pub outgoing_bytes_per_beat: u64,
}

#[derive(Debug, Clone)]
pub struct SweepReport {
    pub axis: SweepAxis,
    pub seed: u32,
    pub beat_ms: u32,
    pub warmup: u32,
    pub beats: u32,
    pub map: SweepMap,
    pub points: Vec<SweepResult>,
}

impl SweepReport {
    pub fn to_json(&self) -> String {
        let mut points = String::new();
        for (i, p) in self.points.iter().enumerate() {
            if i > 0 {
                points.push(',');
            }
            points.push_str(&p.to_json());
        }
        format!(
            "{{\"src\":\"rust\",\"evt\":\"perf_sweep\",\"axis\":\"{}\",\"seed\":{},\"beat_ms\":{},\"warmup\":{},\"beats\":{},\"map\":\"{}\",\"points\":[{}]}}",
            self.axis.as_str(),
            self.seed,
            self.beat_ms,
            self.warmup,
            self.beats,
            self.map.as_str(),
            points
        )
    }
}

impl SweepResult {
    fn to_json(&self) -> String {
        format!(
            "{{\"n\":{},\"beat_wall_samples\":{},\"beat_wall_us_p50\":{},\"beat_wall_us_p95\":{},\"beat_wall_us_p99\":{},\"beat_wall_us_max\":{},\"creatures_us_p50\":{},\"creatures_us_p95\":{},\"creatures_us_p99\":{},\"skills_us_p50\":{},\"skills_us_p95\":{},\"skills_us_p99\":{},\"todo_us_p50\":{},\"todo_us_p95\":{},\"todo_us_p99\":{},\"path_us_p50\":{},\"path_us_p95\":{},\"path_us_p99\":{},\"path_searches\":{},\"outgoing_bytes_per_beat\":{}}}",
            self.n,
            self.beat_wall_samples,
            self.beat_wall_us_p50,
            self.beat_wall_us_p95,
            self.beat_wall_us_p99,
            self.beat_wall_us_max,
            self.creatures_us_p50,
            self.creatures_us_p95,
            self.creatures_us_p99,
            self.skills_us_p50,
            self.skills_us_p95,
            self.skills_us_p99,
            self.todo_us_p50,
            self.todo_us_p95,
            self.todo_us_p99,
            self.path_us_p50,
            self.path_us_p95,
            self.path_us_p99,
            self.path_searches,
            self.outgoing_bytes_per_beat
        )
    }
}

/// Arena radius large enough for N creatures plus a pathing margin.
pub fn arena_radius_for_n(n: usize) -> u16 {
    ((n as u32) / 4).clamp(32, u32::from(u16::MAX)) as u16
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let n = sorted.len();
    let idx = ((p / 100.0) * (n.saturating_sub(1) as f64)).round() as usize;
    sorted[idx.min(n - 1)]
}

fn mean(samples: &[u64]) -> u64 {
    if samples.is_empty() {
        0
    } else {
        samples.iter().sum::<u64>() / samples.len() as u64
    }
}

fn hist_pcts(h: &tfs_rust_core::obs::FixedHistogram) -> (u64, u64, u64) {
    (h.percentile(50.0), h.percentile(95.0), h.percentile(99.0))
}

fn finish_result(n: usize, walls: &[u64], bytes: &[u64], obs: &GameObs) -> SweepResult {
    let mut walls_sorted = walls.to_vec();
    walls_sorted.sort_unstable();
    let (c50, c95, c99) = hist_pcts(&obs.creatures_us);
    let (s50, s95, s99) = hist_pcts(&obs.skills_us);
    let (t50, t95, t99) = hist_pcts(&obs.todo_us);
    let (p50, p95, p99) = hist_pcts(&obs.path_us);
    SweepResult {
        n,
        beat_wall_samples: walls.len() as u64,
        beat_wall_us_p50: percentile(&walls_sorted, 50.0),
        beat_wall_us_p95: percentile(&walls_sorted, 95.0),
        beat_wall_us_p99: percentile(&walls_sorted, 99.0),
        beat_wall_us_max: walls_sorted.last().copied().unwrap_or(0),
        creatures_us_p50: c50,
        creatures_us_p95: c95,
        creatures_us_p99: c99,
        skills_us_p50: s50,
        skills_us_p95: s95,
        skills_us_p99: s99,
        todo_us_p50: t50,
        todo_us_p95: t95,
        todo_us_p99: t99,
        path_us_p50: p50,
        path_us_p95: p95,
        path_us_p99: p99,
        path_searches: obs.path_searches,
        outgoing_bytes_per_beat: mean(bytes),
    }
}

fn populate(
    world: &mut GameWorld,
    axis: SweepAxis,
    n: usize,
    center: Position,
) -> Vec<(ConnId, CreatureId)> {
    match axis {
        SweepAxis::Monsters => {
            spawn_hero(world, center);
            let radius = (n as u16 / 8).max(4);
            spawn_monster_ring(world, DEFAULT_MONSTER, n, center, radius);
            Vec::new()
        }
        SweepAxis::Players => spawn_player_grid(world, n, center, 1),
        SweepAxis::Spectators => {
            let walker = spawn_hero_with_conn(world, center, ConnId(1));
            spawn_spectator_grid(world, n, center, 2);
            vec![(ConnId(1), walker)]
        }
    }
}

/// Run one sweep point on an already-built world (tests use `beat_driven_world`).
pub fn run_point(
    world: &mut GameWorld,
    axis: SweepAxis,
    point: SweepPoint,
    seed: u32,
    center: Position,
) -> SweepResult {
    world.seed_parity_rng(seed);
    let beat_ms = u64::from(world.mechanics.profile.beat_ms.max(1));
    let walkers = populate(world, axis, point.n, center);
    let mut walk_rng = WalkRng::new(u64::from(seed));

    for _ in 0..point.warmup {
        if axis.needs_walks() {
            queue_random_walks(world, &walkers, &mut walk_rng);
        }
        world.advance_beat(beat_ms);
        let _ = world.flush_output_buffers();
    }
    let _ = world.take_obs_window();

    let mut walls = Vec::with_capacity(point.beats as usize);
    let mut bytes = Vec::with_capacity(point.beats as usize);
    for _ in 0..point.beats {
        if axis.needs_walks() {
            queue_random_walks(world, &walkers, &mut walk_rng);
        }
        let t0 = Instant::now();
        world.advance_beat(beat_ms);
        walls.push(t0.elapsed().as_micros() as u64);
        let flushed = world.flush_output_buffers();
        let n: u64 = flushed
            .values()
            .flat_map(|pkts| pkts.iter())
            .map(|b| b.len() as u64)
            .sum();
        bytes.push(n);
    }
    let obs = world.take_obs_window();
    finish_result(point.n, &walls, &bytes, &obs)
}

pub fn build_sweep_world(map: SweepMap, n: usize, center: Position) -> Result<GameWorld, String> {
    let cfg = default_sim_map_config();
    match map {
        SweepMap::Synthetic => beat_driven_world_for_kite_synthetic(
            &cfg.data_dir,
            &cfg.map_rel,
            (center.x, center.y),
            arena_radius_for_n(n),
            center.z,
            DEFAULT_WP,
        ),
        SweepMap::Otbm => beat_driven_world_from_map(&cfg.data_dir, &cfg.map_rel),
    }
}

pub fn run_sweep(
    axis: SweepAxis,
    ns: &[usize],
    beats: u32,
    warmup: u32,
    map: SweepMap,
    seed: u32,
) -> Result<SweepReport, String> {
    let center = KITE_CENTER;
    let mut points = Vec::with_capacity(ns.len());
    let mut beat_ms = 50u32;
    for &n in ns {
        let mut world = build_sweep_world(map, n, center)?;
        beat_ms = world.mechanics.profile.beat_ms;
        let result = run_point(
            &mut world,
            axis,
            SweepPoint { n, beats, warmup },
            seed,
            center,
        );
        points.push(result);
    }
    Ok(SweepReport {
        axis,
        seed,
        beat_ms,
        warmup,
        beats,
        map,
        points,
    })
}
