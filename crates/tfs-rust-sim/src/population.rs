//! Synthetic populations for the Tier 2 scaling sweep.
//!
//! Built on existing `world.rs` helpers — do not grow that file. Appear-batch
//! targets the first player (`appear_monsters`); spectator fan-out needs
//! `register_conn_mapping` so `pending_outgoing` fills.
//!
//! C++ reference: `chase_kite_scenario.cc` `SpawnMonsterAppear`; pack surface
//! `Game::playerMove` via `player_move_request`.

use std::collections::HashSet;
use std::f64::consts::TAU;
use std::time::Instant;

use tfs_rust_common::ConnId;
use tfs_rust_common::Position;
use tfs_rust_common::enums::Direction;
use tfs_rust_core::creature::{CreatureKind, MonsterAiConfig, MonsterState};
use tfs_rust_core::game_world::GameWorld;
use tfs_rust_core::ids::CreatureId;

use crate::world::{
    ensure_walkable_tile_if_absent, insert_monster, insert_monster_from_type, insert_player,
    sim_hero_player,
};

/// Cardinal dirs for walk load (index `rng % 4`).
const CARDINALS: [Direction; 4] = [
    Direction::North,
    Direction::East,
    Direction::South,
    Direction::West,
];

/// Seeded xorshift64 — walk direction only (`parity_random` is `pub(crate)`).
#[derive(Debug, Clone)]
pub struct WalkRng {
    state: u64,
}

impl WalkRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed | 1 }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    pub fn direction(&mut self) -> Direction {
        CARDINALS[(self.next_u64() as usize) % CARDINALS.len()]
    }
}

/// Lay a Chebyshev disk of walkable tiles so chase pathfinding has a connected arena.
pub fn ensure_arena_walkable(world: &mut GameWorld, center: Position, radius: u16) {
    let r = radius as i32;
    let cx = center.x as i32;
    let cy = center.y as i32;
    for dx in -r..=r {
        for dy in -r..=r {
            let x = (cx + dx) as u16;
            let y = (cy + dy) as u16;
            ensure_walkable_tile_if_absent(&mut world.map, Position::new(x, y, center.z));
        }
    }
}

/// Hero at `center` — must exist before [`spawn_monster_ring`] (`appear_player_id`).
pub fn spawn_hero(world: &mut GameWorld, center: Position) -> CreatureId {
    ensure_walkable_tile_if_absent(&mut world.map, center);
    let cid = insert_player(world, sim_hero_player("Hero", center));
    world.map.register_creature_at(center, cid);
    cid
}

/// Hero plus connection mapping so `player_move_request` can enqueue cancel/snapback.
pub fn spawn_hero_with_conn(world: &mut GameWorld, center: Position, conn: ConnId) -> CreatureId {
    let cid = spawn_hero(world, center);
    world.register_conn_mapping(conn, cid);
    cid
}

fn occupy(set: &mut HashSet<(u16, u16)>, pos: Position) -> bool {
    set.insert((pos.x, pos.y))
}

/// Unique tile on a ring; spiral outward on collision with `occupied` or `center`.
fn ring_slot(
    occupied: &mut HashSet<(u16, u16)>,
    center: Position,
    radius: u16,
    i: usize,
    n: usize,
) -> Position {
    let n = n.max(1);
    let angle = TAU * (i as f64) / (n as f64);
    let mut r = f64::from(radius.max(1));
    for _ in 0..4096 {
        let dx = (r * angle.cos()).round() as i32;
        let dy = (r * angle.sin()).round() as i32;
        let x = (i32::from(center.x) + dx) as u16;
        let y = (i32::from(center.y) + dy) as u16;
        let pos = Position::new(x, y, center.z);
        if (x != center.x || y != center.y) && occupy(occupied, pos) {
            return pos;
        }
        r += 1.0;
    }
    Position::new(
        center.x.saturating_add(radius + i as u16 + 1),
        center.y,
        center.z,
    )
}

fn insert_named_monster(world: &mut GameWorld, name: &str, pos: Position) -> CreatureId {
    let key = name.to_ascii_lowercase();
    let mtype = world.monsters_db.monsters.get(&key).cloned();
    if let Some(mtype) = mtype {
        let speed = mtype.speed as i32;
        let config = MonsterAiConfig::from_monster_type(&mtype);
        insert_monster_from_type(world, &mtype, name, pos, speed, config, MonsterState::Idle)
    } else {
        insert_monster(world, name, pos, 220)
    }
}

/// Ring of `n` monsters around `center`, then one `appear_monsters` batch (chase → `path_us`).
///
/// Hero must already be inserted. Prefers `monsters_db` type lookup; falls back to
/// [`insert_monster`] when the name is missing (empty synthetic DB in tests).
pub fn spawn_monster_ring(
    world: &mut GameWorld,
    name: &str,
    n: usize,
    center: Position,
    radius: u16,
) -> Vec<CreatureId> {
    ensure_arena_walkable(world, center, radius.saturating_add(2));
    let mut occupied = HashSet::new();
    occupy(&mut occupied, center);
    let mut ids = Vec::with_capacity(n);
    for i in 0..n {
        let pos = ring_slot(&mut occupied, center, radius, i, n);
        ids.push(insert_named_monster(world, name, pos));
    }
    if !ids.is_empty() {
        world.appear_monsters(&ids);
    }
    ids
}

/// √n grid of spectator players with `ConnId(start + i)` so broadcasts fill `pending_outgoing`.
///
/// Replicates test-only `insert_spectator_player` (insert + conn map + tile + register).
pub fn spawn_player_grid(
    world: &mut GameWorld,
    n: usize,
    center: Position,
    start_conn: u32,
) -> Vec<(ConnId, CreatureId)> {
    if n == 0 {
        return Vec::new();
    }
    let side = (n as f64).sqrt().ceil() as i32;
    let origin_x = i32::from(center.x) - side / 2;
    let origin_y = i32::from(center.y) - side / 2;
    let pad = ((side / 2) + 2).max(2) as u16;
    ensure_arena_walkable(world, center, pad);

    let mut occupied = HashSet::new();
    occupy(&mut occupied, center);
    let mut out = Vec::with_capacity(n);
    let mut placed = 0usize;
    let mut extra = 0i32;
    while placed < n {
        let idx = placed as i32 + extra;
        let gx = origin_x + idx % side;
        let gy = origin_y + idx / side;
        let pos = Position::new(gx as u16, gy as u16, center.z);
        if !occupy(&mut occupied, pos) {
            extra += 1;
            continue;
        }
        ensure_walkable_tile_if_absent(&mut world.map, pos);
        let conn = ConnId(start_conn + placed as u32);
        let mut player = sim_hero_player(&format!("P{}", start_conn + placed as u32), pos);
        player.guid = start_conn + placed as u32;
        player.account_id = start_conn + placed as u32;
        let cid = insert_player(world, player);
        world.register_conn_mapping(conn, cid);
        world.map.register_creature_at(pos, cid);
        out.push((conn, cid));
        placed += 1;
    }
    out
}

/// Pack spectators into the 18×14 view box (`range_x=8`, `range_y=6`) around `center`.
pub fn spawn_spectator_grid(
    world: &mut GameWorld,
    n: usize,
    center: Position,
    start_conn: u32,
) -> Vec<(ConnId, CreatureId)> {
    const RANGE_X: i32 = 8;
    const RANGE_Y: i32 = 6;
    ensure_arena_walkable(world, center, 10);
    let mut occupied = HashSet::new();
    occupy(&mut occupied, center);
    let mut out = Vec::with_capacity(n);
    let mut i = 0i32;
    let mut placed = 0usize;
    while placed < n {
        let dx = (i % (RANGE_X * 2 + 1)) - RANGE_X;
        let dy = (i / (RANGE_X * 2 + 1)) - RANGE_Y;
        i += 1;
        if dx == 0 && dy == 0 {
            continue;
        }
        let x = (i32::from(center.x) + dx) as u16;
        let y = (i32::from(center.y) + dy) as u16;
        let pos = Position::new(x, y, center.z);
        if !occupy(&mut occupied, pos) {
            continue;
        }
        ensure_walkable_tile_if_absent(&mut world.map, pos);
        let conn = ConnId(start_conn + placed as u32);
        let mut player = sim_hero_player(&format!("S{}", start_conn + placed as u32), pos);
        player.guid = start_conn + placed as u32;
        player.account_id = start_conn + placed as u32;
        let cid = insert_player(world, player);
        world.register_conn_mapping(conn, cid);
        world.map.register_creature_at(pos, cid);
        out.push((conn, cid));
        placed += 1;
        if i > 10_000 {
            break;
        }
    }
    out
}

/// Queue one cardinal `player_move_request` per mapped player (players / spectators axes).
pub fn queue_random_walks(
    world: &mut GameWorld,
    players: &[(ConnId, CreatureId)],
    rng: &mut WalkRng,
) {
    let now = Instant::now();
    for &(conn, cid) in players {
        if !matches!(world.creatures.get(cid), Some(CreatureKind::Player(_))) {
            continue;
        }
        let dir = rng.direction();
        world.player_move_request(conn, cid, dir, now);
    }
}
