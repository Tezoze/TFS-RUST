//! Criterion microbenches for game-thread hot paths (`docs/BENCHMARK.md` Tier 1).
//!
//! Pub API only — benches compile as an external crate.
//!
//! - Pathfinding: `get_path_matching` / 772 `TShortway` (`cract.cc`).
//! - Spectators: `SparseGrid::collect_spectators` (`map.cpp` `Map::getSpectators`).
//! - Condition: `dot_tick_for_condition` fire/energy (`crskill.cc` `TSkillBurning::Event`).
//!   Poison is not this function (decays in `process_skills`; no API widen).
//! - ToDo: `ToDoQueue` insert/pop (`containers.hh` `priority_queue`).

use std::collections::HashMap;

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use slotmap::KeyData;
use tfs_rust_common::enums::{ConditionType, ZoneType};
use tfs_rust_common::{MAX_CLIENT_VIEWPORT_X, MAX_CLIENT_VIEWPORT_Y, Position, ProtocolVersion};
use tfs_rust_core::condition::dot_tick_for_condition;
use tfs_rust_core::map::{Map, SparseGrid};
use tfs_rust_core::pathfinding::{
    DEFAULT_TERRAIN_WAYPOINTS, FindPathParams, REVERSE_PATH_VIEW_RADIUS, TShortwayScratch,
    effective_terrain_waypoints, get_path_matching,
};
use tfs_rust_core::tile::{Tile, TileBody};
use tfs_rust_core::todo_queue::ToDoQueue;
use tfs_rust_core::{CreatureId, Mechanics, PathCostModel};

fn empty_map() -> Map {
    Map {
        width: 256,
        height: 256,
        grid: SparseGrid::new(),
        towns: HashMap::new(),
        waypoints: HashMap::new(),
        house_tiles: Vec::new(),
        refresh_positions: rustc_hash::FxHashSet::default(),
        refresh_snapshots: HashMap::new(),
        live_dirty: rustc_hash::FxHashSet::default(),
        live_baselines: HashMap::new(),
    }
}

fn insert_walkable(map: &mut Map, pos: Position) {
    map.insert_tile(
        pos,
        Tile::Normal(TileBody {
            ground: Some(DEFAULT_TERRAIN_WAYPOINTS as u16),
            ground_item: None,
            stacks: None,
            flags: 0,
            zone: ZoneType::Normal,
        }),
    );
}

/// Synthetic arena: filled square of radius `r` around (128,128,7).
///
/// `wall`: omit a north–south column at the center, leaving a one-tile gap at the south edge.
fn make_arena(radius: u16, wall: bool) -> (Map, Position, Position) {
    let z = 7u8;
    let cx = 128u16;
    let cy = 128u16;
    let start = Position::new(cx - radius, cy, z);
    let target = Position::new(cx + radius, cy, z);
    let pad = 2u16;
    let mut map = empty_map();
    let min_x = cx - radius - pad;
    let max_x = cx + radius + pad;
    let min_y = cy - radius - pad;
    let max_y = cy + radius + pad;
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            if wall && x == cx && y != max_y {
                continue;
            }
            insert_walkable(&mut map, Position::new(x, y, z));
        }
    }
    (map, start, target)
}

fn ground_cost(map: &Map, pos: Position) -> u32 {
    map.get_tile(pos)
        .and_then(|t| t.body().ground)
        .map_or(0, |g| effective_terrain_waypoints(u32::from(g)))
}

/// Chase-equivalent `FindPathParams` (`monster_ai.rs` `monster_path_search_params`:
/// not fleeing/summon, `target_distance=1`, `has_follow_path=false`).
fn chase_path_params() -> FindPathParams {
    FindPathParams {
        min_target_dist: 1,
        max_target_dist: 1,
        clear_sight: true,
        allow_diagonal: true,
        full_path_search: true,
        max_search_dist: 0,
    }
}

fn pathfinding(c: &mut Criterion) {
    let mut group = c.benchmark_group("pathfinding");
    let fpp = chase_path_params();
    for radius in [16u16, 32, 64] {
        for (kind, wall) in [("straight", false), ("wall", true)] {
            let fixture = make_arena(radius, wall);
            group.bench_with_input(BenchmarkId::new(kind, radius), &fixture, |b, fixture| {
                let (map, start, target) = fixture;
                let mut scratch = TShortwayScratch::new();
                b.iter(|| {
                    black_box(get_path_matching(
                        map,
                        *start,
                        *target,
                        &fpp,
                        PathCostModel::TerrainWeighted,
                        REVERSE_PATH_VIEW_RADIUS,
                        |pos| map.is_walkable(pos),
                        |_pos| 0u32,
                        |pos| ground_cost(map, pos),
                        Some(&mut scratch),
                    ))
                });
            });
        }
    }
    group.finish();
}

fn sqrt_ceil(n: u16) -> u16 {
    let s = n.isqrt();
    if s.saturating_mul(s) < n {
        s.saturating_add(1)
    } else {
        s.max(1)
    }
}

fn spectator_map(n: u16) -> (Map, u16, u16) {
    let z = 7u8;
    let cx = 200u16;
    let cy = 200u16;
    let side = sqrt_ceil(n);
    let origin_x = cx.saturating_sub(side / 2);
    let origin_y = cy.saturating_sub(side / 2);
    let mut map = empty_map();
    for i in 0..n {
        let x = origin_x + (i % side);
        let y = origin_y + (i / side);
        let pos = Position::new(x, y, z);
        insert_walkable(&mut map, pos);
        let id = CreatureId::from(KeyData::from_ffi(u64::from(i) + 1));
        map.register_creature_at(pos, id);
    }
    (map, cx, cy)
}

fn spectators(c: &mut Criterion) {
    let mut group = c.benchmark_group("spectators");
    let range_x = u16::try_from(MAX_CLIENT_VIEWPORT_X).unwrap_or(8);
    let range_y = u16::try_from(MAX_CLIENT_VIEWPORT_Y).unwrap_or(6);
    for n in [1_000u16, 10_000] {
        let fixture = spectator_map(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &fixture, |b, fixture| {
            let (map, cx, cy) = fixture;
            let mut out = Vec::new();
            b.iter(|| {
                map.grid
                    .collect_spectators(*cx, *cy, 7, range_x, range_y, &mut out);
                black_box(out.len());
                out.clear();
            });
        });
    }
    group.finish();
}

fn condition_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("condition_tick");
    let mech = Mechanics::for_version(ProtocolVersion::V772);
    for ctype in [ConditionType::Fire, ConditionType::Energy] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{ctype:?}")),
            &ctype,
            |b, ctype| {
                let mut round = 0i32;
                b.iter(|| {
                    round = (round + 1) & 15;
                    black_box(dot_tick_for_condition(
                        &mech.profile,
                        &mech.hooks,
                        *ctype,
                        round,
                    ))
                });
            },
        );
    }
    group.finish();
}

fn dummy_ids(n: usize) -> Vec<CreatureId> {
    (1..=n)
        .map(|i| CreatureId::from(KeyData::from_ffi(i as u64)))
        .collect()
}

fn todo_heap(c: &mut Criterion) {
    let mut group = c.benchmark_group("todo_heap");
    for n in [1_000usize, 10_000] {
        let ids = dummy_ids(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &ids, |b, ids| {
            b.iter(|| {
                let mut q = ToDoQueue::default();
                for &id in ids {
                    q.insert(0, id);
                }
                while q.pop().is_some() {}
                black_box(q.len());
            });
        });
    }
    group.finish();
}

criterion_group!(
    hot_paths,
    pathfinding,
    spectators,
    condition_tick,
    todo_heap
);
criterion_main!(hot_paths);
