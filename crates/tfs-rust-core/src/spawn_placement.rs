//! 772 spawn tile search — `SearchSpawnField` + monsterhome radius rules.
//!
//! C++ reference: `info.cc` `SearchSpawnField`, `crnonpl.cc` `LoadMonsterhomes` /
//! `ProcessMonsterhomes`. Login temple Create: `cract.cc` `SetOnMap` /
//! `operate.cc` `CheckMapPlace(0)`.

use std::cell::RefCell;

use tfs_rust_common::Position;
use tfs_rust_common::enums::ZoneType;

use crate::creature::CreatureKind;
use crate::formulas::{SpawnNearPlayer, SpawnPlacement};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::tile::{MapStackEntry, Tile, flags as tilestate};
use crate::visibility::can_see_floor;

/// Per-tile probe for classic BFS spawn search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpawnTileProbe {
    pub login_possible: bool,
    /// C++ `LoginBad` — spawn works but tile has moveable blockers.
    pub login_clean: bool,
    pub expansion_ok: bool,
}

/// C++ `LoadMonsterhomes` / `ProcessMonsterhomes` signed search distance.
pub(crate) fn classic772_signed_search_distance(home_radius: i32, act_in_zone: usize) -> i32 {
    let mut max_radius = home_radius;
    if max_radius < 0 {
        max_radius = 1;
    }
    if max_radius > 10 {
        max_radius = 10;
    }
    if act_in_zone == 0 {
        max_radius.min(1)
    } else {
        -max_radius
    }
}

/// C++ `ProcessMonsterhomes` player proximity radius shrink (`crnonpl.cc:1427-1455`).
pub(crate) fn shrink_spawn_radius_near_players(
    world: &GameWorld,
    home: Position,
    home_radius: i32,
) -> i32 {
    let mut max_radius = home_radius;
    if max_radius < 0 {
        max_radius = 1;
    }
    if max_radius > 10 {
        max_radius = 10;
    }

    let search_x = max_radius + 9;
    let search_y = max_radius + 7;

    for (_, kind) in world.creatures.iter() {
        if !matches!(kind, CreatureKind::Player(_)) {
            continue;
        }
        let pos = kind.position();
        // Corpus `TFindCreatures(FIND_PLAYERS)` + `CanSeeFloor` (`crnonpl.cc:1432-1446`).
        if !can_see_floor(pos.z, home.z) {
            continue;
        }
        let dx = (pos.x as i32 - home.x as i32).abs();
        let dy = (pos.y as i32 - home.y as i32).abs();
        if dx > search_x || dy > search_y {
            continue;
        }
        let radius = (dx - 9).max(dy - 7);
        if radius < max_radius {
            max_radius = radius;
        }
    }

    max_radius
}

fn offset_position(center: Position, dx: i32, dy: i32) -> Position {
    Position::new(
        (center.x as i32 + dx).max(0) as u16,
        (center.y as i32 + dy).max(0) as u16,
        center.z,
    )
}

/// C++ `SearchFreeField` / `SearchLoginField` east-first spiral (`info.cc:761`, `info.cc:868`).
pub(crate) fn spiral_free_field_positions(center: Position, distance: i32) -> Vec<Position> {
    let mut out = Vec::new();
    let base_x = center.x as i32;
    let base_y = center.y as i32;
    let z = center.z;

    let mut offset_x = 0i32;
    let mut offset_y = 0i32;
    let mut current_distance = 0i32;
    // EAST → NORTH → WEST → SOUTH (`enums.hh` direction order).
    let mut direction = 0u8;

    while current_distance <= distance {
        let field_x = base_x + offset_x;
        let field_y = base_y + offset_y;
        if field_x >= 0 && field_y >= 0 {
            out.push(Position::new(field_x as u16, field_y as u16, z));
        }

        match direction {
            1 => {
                offset_y -= 1;
                if offset_y <= -current_distance {
                    direction = 2;
                }
            }
            2 => {
                offset_x -= 1;
                if offset_x <= -current_distance {
                    direction = 3;
                }
            }
            3 => {
                offset_y += 1;
                if offset_y >= current_distance {
                    direction = 0;
                }
            }
            _ => {
                offset_x += 1;
                if offset_x > current_distance {
                    current_distance = offset_x;
                    direction = 1;
                }
            }
        }
    }

    out
}

/// C++ `SearchLoginField` (`info.cc:861`) — spiral login probe up to `distance`.
pub(crate) fn search_login_field(
    center: Position,
    distance: i32,
    mut login_possible: impl FnMut(Position) -> bool,
) -> Option<Position> {
    let distance = distance.max(0);
    spiral_free_field_positions(center, distance)
        .into_iter()
        .find(|&pos| login_possible(pos))
}

// Game-thread TLS for SearchSpawnField BFS phases. Placement is serial on the
// sim thread (GameWorld is not Send); keep capacity across ~23k boot searches.
thread_local! {
    static SEARCH_SPAWN_PHASES: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
}

/// C++ `SearchSpawnField` (`info.cc:911`).
pub(crate) fn search_spawn_field(
    distance: i32,
    center: Position,
    mut probe_at: impl FnMut(Position) -> SpawnTileProbe,
    mut tie_roll: impl FnMut() -> i32,
) -> Option<Position> {
    let minimize = distance >= 0;
    let distance = distance.unsigned_abs().min(30) as i32;
    if distance == 0 {
        let probe = probe_at(center);
        return probe.login_possible.then_some(center);
    }

    SEARCH_SPAWN_PHASES.with(|cell| {
        let mut phases = cell.borrow_mut();
        let grid = (2 * distance + 1) as usize;
        let need = grid * grid;
        if phases.len() < need {
            phases.resize(need, i32::MAX);
        }
        phases[..need].fill(i32::MAX);
        let idx = |ox: i32, oy: i32| -> usize {
            let row = (oy + distance) as usize;
            let col = (ox + distance) as usize;
            row * grid + col
        };

        phases[idx(0, 0)] = 0;

        let mut best_pos: Option<Position> = None;
        let mut best_tie = -1i32;
        let mut expansion_phase = 0i32;

        loop {
            let mut found = false;
            let mut expanded = false;

            for oy in -distance..=distance {
                for ox in -distance..=distance {
                    if phases[idx(ox, oy)] != expansion_phase {
                        continue;
                    }

                    let pos = offset_position(center, ox, oy);
                    let probe = probe_at(pos);

                    if probe.expansion_ok || expansion_phase == 0 {
                        for ny in -1..=1 {
                            for nx in -1..=1 {
                                if nx == 0 && ny == 0 {
                                    continue;
                                }
                                let nox = ox + nx;
                                let noy = oy + ny;
                                if nox < -distance
                                    || nox > distance
                                    || noy < -distance
                                    || noy > distance
                                {
                                    continue;
                                }
                                let step = (nox - ox).abs() + (noy - oy).abs();
                                let neighbor = phases[idx(nox, noy)];
                                if neighbor > expansion_phase + step {
                                    phases[idx(nox, noy)] = expansion_phase + step;
                                }
                            }
                        }
                        expanded = true;
                    }

                    if probe.login_possible {
                        // C++ `SearchSpawnField` tie-break `random(0, 99)` (`info.cc`) — glibc parity
                        // stream, not `thread_rng` (Finding 19).
                        let tie = tie_roll() + if probe.login_clean { 100 } else { 0 };
                        if tie > best_tie {
                            best_tie = tie;
                            best_pos = Some(pos);
                        }
                        found = true;
                    }
                }
            }

            if (found && minimize) || !expanded {
                break;
            }
            expansion_phase += 1;
        }

        best_pos
    })
}

impl GameWorld {
    /// 772 `SearchFreeField` — `info.cc:761`. East-first spiral; `HouseID == 0` (no house tiles).
    ///
    /// Used by `CreateMonster` after `SearchSummonField` (`crnonpl.cc:3169`). Return value is
    /// ignored there on failure — callers should `unwrap_or(center)`.
    pub(crate) fn search_free_field(&self, center: Position, distance: i32) -> Option<Position> {
        let distance = distance.max(0);
        spiral_free_field_positions(center, distance)
            .into_iter()
            .find(|&pos| self.free_field_tile_ok(pos))
    }

    /// `SearchFreeField` tile probe — shared by `creature_lib` closest-free search.
    pub(crate) fn is_free_field_tile(&self, pos: Position) -> bool {
        self.free_field_tile_ok(pos)
    }

    /// `SearchFreeField` MovePossible arm (`info.cc:775–778`) + house gate with `HouseID == 0`.
    fn free_field_tile_ok(&self, pos: Position) -> bool {
        let Some(tile) = self.map.get_tile(pos) else {
            return false;
        };
        // `HouseID == 0` → reject all house tiles (`info.cc:782–783`).
        if matches!(tile, crate::tile::Tile::House(_)) {
            return false;
        }
        let body = tile.body();
        let chain = body.map_object_chain();
        let Some(crate::tile::MapStackEntry::Ground(server_id)) = chain.first() else {
            return false;
        };
        if !self.items_db.is_terrain_bank(*server_id) {
            return false;
        }
        // Creatures are UNPASS containers — occupied tiles fail.
        if !body.creatures().is_empty() {
            return false;
        }
        for entry in &chain {
            match entry {
                crate::tile::MapStackEntry::Ground(sid) => {
                    if self.items_db.is_unpassable_for_field(*sid) {
                        return false;
                    }
                    // AVOID allowed only with BED (`info.cc:777–778`).
                    if self.items_db.is_avoid_hazard(*sid) && (body.flags & tilestate::BED) == 0 {
                        return false;
                    }
                }
                crate::tile::MapStackEntry::Item(item_id) => {
                    let Some(item) = self.items.get(*item_id) else {
                        return false;
                    };
                    let sid = item.item_type;
                    if self.items_db.is_unpassable_for_field(sid) {
                        return false;
                    }
                    if self.items_db.is_avoid_hazard(sid) && (body.flags & tilestate::BED) == 0 {
                        return false;
                    }
                }
                crate::tile::MapStackEntry::Creature(_) => return false,
            }
        }
        true
    }

    /// 772 `SearchSpawnField` per-tile probe (`info.cc:940–1009`).
    ///
    /// No TFS `forced` / `FLAG_IGNOREBLOCKITEM` short-circuit — that incorrectly allowed
    /// UNPASS+UNMOVE wall tiles on respawn (`forced = !startup`).
    pub(crate) fn probe_spawn_tile(
        &self,
        _cid: CreatureId,
        pos: Position,
        place_in_pz: bool,
        home_house_id: u32,
    ) -> SpawnTileProbe {
        let fail = SpawnTileProbe {
            login_possible: false,
            login_clean: false,
            expansion_ok: false,
        };
        let Some(tile) = self.map.get_tile(pos) else {
            return fail;
        };

        // House gate: `HouseID == 0` rejects all houses; else only matching house.
        if let Tile::House(h) = tile
            && (home_house_id == 0 || h.house_id != home_house_id)
        {
            return fail;
        }

        let body = tile.body();
        // `Obj == NONE` → skip (`info.cc:948–950`).
        let chain = body.map_object_chain();
        if chain.is_empty() {
            return fail;
        }

        // Monsterhomes always pass `Player=false` → skip every PZ tile (`info.cc:944–946`).
        // NPC temple spawns set `place_in_pz` when home is PZ (TFS-shaped temple NPCs).
        if place_in_pz {
            if body.zone != ZoneType::Protection {
                return fail;
            }
        } else if body.zone == ZoneType::Protection {
            return fail;
        }

        let mut expansion_ok = true;
        let mut login_possible = true;
        let mut login_bad = false;

        for entry in &chain {
            let server_id = match entry {
                MapStackEntry::Creature(_) => {
                    login_possible = false;
                    continue;
                }
                MapStackEntry::Ground(sid) => *sid,
                MapStackEntry::Item(item_id) => {
                    let Some(item) = self.items.get(*item_id) else {
                        login_possible = false;
                        continue;
                    };
                    item.item_type
                }
            };

            // UNPASS + UNMOVE → hard block; UNPASS alone → LoginBad (`info.cc:962–968`).
            // Field Unpass includes Bank+wp0 after cliff clear-solid (lesson 171/255).
            if self.items_db.is_unpassable_for_field(server_id) {
                if self.items_db.is_immovable(server_id) {
                    expansion_ok = false;
                    login_possible = false;
                } else {
                    login_bad = true;
                }
            }

            // AVOID + UNMOVE: ExpansionPossible=false; LoginPossible &= !Player.
            // Monster/NPC spawn uses Player=false → login flag unchanged (`info.cc:971–976`).
            if self.items_db.is_avoid_hazard(server_id) {
                if self.items_db.is_immovable(server_id) {
                    expansion_ok = false;
                }
                login_bad = true;
            }
        }

        let login_clean = login_possible && !login_bad;
        SpawnTileProbe {
            login_possible,
            login_clean,
            expansion_ok,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn place_spawn_creature(
        &mut self,
        cid: CreatureId,
        slot_index: usize,
        center: Position,
        home_radius: i32,
        startup: bool,
        forced: bool,
        extended_pos: bool,
    ) -> bool {
        match self.mechanics.profile.spawn_placement {
            SpawnPlacement::TfsShuffle => {
                self.find_and_place_creature_tfs(cid, center, extended_pos, forced, home_radius)
            }
            SpawnPlacement::Classic772Bfs => {
                // `forced` is TFS `placeCreature` only — 772 `SearchSpawnField` has no force path.
                let _ = forced;
                let Some(slot) = self.spawns.slot(slot_index) else {
                    return false;
                };
                let home = self.spawns.zone_center(slot.zone_index).unwrap_or(center);
                let act = slot
                    .home_index
                    .map(|hi| self.spawns.count_occupied_in_home(hi))
                    .unwrap_or(0);
                let mut effective_radius = home_radius;
                if !startup
                    && self.mechanics.profile.spawn_near_player == SpawnNearPlayer::RadiusShrink
                {
                    effective_radius = shrink_spawn_radius_near_players(self, home, home_radius);
                    if effective_radius < 0 {
                        return false;
                    }
                }
                let signed_dist = classic772_signed_search_distance(effective_radius, act);
                let home_tile = self.map.get_tile(home);
                let place_in_pz = home_tile
                    .map(|t| t.body().zone == ZoneType::Protection)
                    .unwrap_or(false);
                let home_house_id = match home_tile {
                    Some(Tile::House(h)) => h.house_id,
                    _ => 0,
                };
                let pos = search_spawn_field(
                    signed_dist,
                    home,
                    |try_pos| self.probe_spawn_tile(cid, try_pos, place_in_pz, home_house_id),
                    || self.parity_random(0, 99),
                );
                let Some(pos) = pos else {
                    return false;
                };
                if let Some(kind) = self.creatures.get_mut(cid) {
                    kind.set_position(pos);
                }
                self.map.register_creature_at(pos, cid);
                true
            }
        }
    }

    /// 772 `Create` onto a map container after `SetOnMap` temple fallback (`cract.cc:334-349`).
    ///
    /// `CheckMapPlace(0, …)` skips `IsMapBlocked` (`operate.cc:601-603`) so an occupied
    /// BANK tile is legal. Requires a loaded tile with ground (BANK); do **not** use
    /// `FLAG_NOLIMIT` — that would also accept void / UNPASS walls.
    pub(crate) fn commit_set_on_map_create(&mut self, cid: CreatureId, pos: Position) -> bool {
        let has_bank = self
            .map
            .get_tile(pos)
            .is_some_and(|t| t.body().ground.is_some());
        if !has_bank {
            return false;
        }
        if let Some(kind) = self.creatures.get(cid) {
            let old = kind.position();
            let on_old = self
                .map
                .get_tile(old)
                .is_some_and(|t| t.body().creatures().contains(&cid));
            if on_old && old != pos {
                self.map.unregister_creature_at(old, cid);
            }
        }
        if let Some(kind) = self.creatures.get_mut(cid) {
            kind.set_position(pos);
        }
        self.map.register_creature_at(pos, cid);
        true
    }

    /// C++ `TCreature::SetOnMap` — `SearchLoginField(dist=1)` (`cract.cc:311`, `info.cc:861`).
    pub fn place_creature_login(
        &mut self,
        cid: CreatureId,
        requested: Position,
    ) -> Option<Position> {
        const LOGIN_DISTANCE: i32 = 1;
        let home_house_id = match self.map.get_tile(requested) {
            Some(Tile::House(h)) => h.house_id,
            _ => 0,
        };
        let pos = search_login_field(requested, LOGIN_DISTANCE, |try_pos| {
            self.probe_spawn_tile(cid, try_pos, false, home_house_id)
                .login_possible
        })?;
        let old = self.creatures.get(cid)?.position();
        if old != pos {
            self.map.unregister_creature_at(old, cid);
            if let Some(kind) = self.creatures.get_mut(cid) {
                kind.set_position(pos);
            }
            self.map.register_creature_at(pos, cid);
        }
        Some(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic772_first_slot_min_radius_one() {
        assert_eq!(classic772_signed_search_distance(50, 0), 1);
        assert_eq!(classic772_signed_search_distance(10, 0), 1);
        assert_eq!(classic772_signed_search_distance(1, 0), 1);
    }

    #[test]
    fn classic772_later_slots_extended_negative() {
        assert_eq!(classic772_signed_search_distance(50, 1), -10);
        assert_eq!(classic772_signed_search_distance(3, 2), -3);
    }

    /// Surface viewer one floor up still `CanSeeFloor` the home (`crnonpl.cc:1445`).
    #[test]
    fn player_one_floor_up_suppresses_spawn() {
        use crate::test_world::support::{
            beat_driven_test_world, ensure_walkable_tile, insert_player, test_player,
        };

        let mut world = beat_driven_test_world();
        let home = Position::new(100, 100, 7);
        let viewer = Position::new(100, 100, 6);
        ensure_walkable_tile(&mut world.map, home, 150);
        ensure_walkable_tile(&mut world.map, viewer, 150);
        insert_player(&mut world, test_player("Above", viewer));
        let shrunk = shrink_spawn_radius_near_players(&world, home, 10);
        assert!(
            shrunk < 0,
            "player on home XY one floor up shrinks radius below 0, got {shrunk}"
        );
    }

    /// Underground (`z=11`) cannot `CanSeeFloor` surface 7.
    #[test]
    fn player_underground_does_not_suppress_surface_home() {
        use crate::test_world::support::{
            beat_driven_test_world, ensure_walkable_tile, insert_player, test_player,
        };

        let mut world = beat_driven_test_world();
        let home = Position::new(100, 100, 7);
        let viewer = Position::new(100, 100, 11);
        ensure_walkable_tile(&mut world.map, home, 150);
        ensure_walkable_tile(&mut world.map, viewer, 150);
        insert_player(&mut world, test_player("Below", viewer));
        assert_eq!(shrink_spawn_radius_near_players(&world, home, 10), 10);
    }

    #[test]
    fn search_spawn_field_minimize_picks_closest() {
        let center = Position::new(10, 10, 7);
        let mut walkable = std::collections::HashSet::new();
        walkable.insert((10, 10));
        walkable.insert((11, 10));
        walkable.insert((15, 15));

        let pos = search_spawn_field(
            2,
            center,
            |p| {
                let key = (p.x, p.y);
                let ok = walkable.contains(&key);
                SpawnTileProbe {
                    login_possible: ok,
                    login_clean: ok,
                    expansion_ok: ok,
                }
            },
            || 0,
        );
        assert_eq!(pos, Some(Position::new(10, 10, 7)));
    }

    #[test]
    fn search_spawn_field_extended_reaches_outer_ring() {
        let center = Position::new(10, 10, 7);
        let far = Position::new(13, 10, 7);

        let pos = search_spawn_field(
            -3,
            center,
            |p| {
                let ok = p == far;
                SpawnTileProbe {
                    login_possible: ok,
                    login_clean: ok,
                    expansion_ok: true,
                }
            },
            || 0,
        );
        assert_eq!(pos, Some(far));
    }

    #[test]
    fn search_spawn_field_reuses_phase_buffer_across_radii() {
        let center = Position::new(10, 10, 7);
        let far = Position::new(14, 10, 7);
        let wide = search_spawn_field(
            -4,
            center,
            |p| {
                let ok = p == far;
                SpawnTileProbe {
                    login_possible: ok,
                    login_clean: ok,
                    expansion_ok: true,
                }
            },
            || 0,
        );
        assert_eq!(wide, Some(far));
        let near = search_spawn_field(
            1,
            center,
            |p| {
                let ok = p == center;
                SpawnTileProbe {
                    login_possible: ok,
                    login_clean: ok,
                    expansion_ok: ok,
                }
            },
            || 0,
        );
        assert_eq!(near, Some(center));
    }

    #[test]
    fn search_login_field_east_first_when_center_blocked() {
        let center = Position::new(10, 10, 7);
        let east = Position::new(11, 10, 7);
        let mut blocked = std::collections::HashSet::new();
        blocked.insert((10, 10));

        let pos = search_login_field(center, 1, |p| !blocked.contains(&(p.x, p.y)));
        assert_eq!(pos, Some(east));
    }

    /// Respawn used to pass `forced=true` so `login_possible = forced || …` accepted walls.
    /// 772 `SearchSpawnField` rejects UNPASS+UNMOVE (`info.cc:962–965`).
    #[test]
    fn probe_rejects_immovable_unpass_wall() {
        use crate::item::Item;
        use crate::test_support::{
            TEST_SYNTHETIC_GROUND_WP, beat_driven_test_world, ensure_walkable_tile, insert_monster,
        };
        use std::sync::Arc;
        use tfs_rust_content::otb::ItemType;

        const WALL: u16 = 9001;
        let mut world = beat_driven_test_world();
        Arc::make_mut(&mut world.items_db).items.insert(
            WALL,
            ItemType {
                server_id: WALL,
                block_solid_override: Some(true),
                moveable_override: Some(false),
                ..ItemType::default()
            },
        );

        let home = Position::new(100, 100, 7);
        let wall_pos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, home, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, wall_pos, TEST_SYNTHETIC_GROUND_WP);
        let wall_item = world.items.insert(Item::new_single(WALL));
        if let Some(tile) = world.map.get_tile_mut(wall_pos) {
            tile.body_mut().down_items_mut().push(wall_item);
        }

        let cid = insert_monster(&mut world, "Rat", home, 20);
        let wall_probe = world.probe_spawn_tile(cid, wall_pos, false, 0);
        assert!(
            !wall_probe.login_possible,
            "UNPASS+UNMOVE wall must not be a spawn login tile"
        );
        assert!(!wall_probe.expansion_ok);

        world.map.unregister_creature_at(home, cid);
        let open_probe = world.probe_spawn_tile(cid, home, false, 0);
        assert!(open_probe.login_possible);
        assert!(open_probe.expansion_ok);
        assert!(open_probe.login_clean);
    }

    /// Dirt walls / earth are Bank+Unpass+wp0 — OTB clears blockSolid for player cliffs, but
    /// `SearchSpawnField` must still block expansion or spiders leak into adjacent sewers.
    #[test]
    fn probe_rejects_bank_zero_waypoint_dirt_wall_ground() {
        use crate::test_support::{
            TEST_SYNTHETIC_GROUND_WP, beat_driven_test_world, insert_monster,
        };
        use crate::tile::{Tile, TileBody};
        use std::sync::Arc;
        use tfs_rust_common::enums::ZoneType;
        use tfs_rust_content::otb::ItemType;

        const DIRT_WALL: u16 = 9100;
        let mut world = beat_driven_test_world();
        Arc::make_mut(&mut world.items_db).items.insert(
            DIRT_WALL,
            ItemType {
                server_id: DIRT_WALL,
                group: ItemType::GROUP_GROUND,
                // Cleared blockSolid (player-walkable cliff path) — spawn must still Unpass.
                block_solid_override: Some(false),
                moveable_override: Some(false),
                speed: 0,
                ..ItemType::default()
            },
        );

        let open = Position::new(100, 100, 7);
        let wall = Position::new(101, 100, 7);
        crate::test_support::ensure_walkable_tile(&mut world.map, open, TEST_SYNTHETIC_GROUND_WP);
        world.map.insert_tile(
            wall,
            Tile::Normal(TileBody {
                ground: Some(DIRT_WALL),

                ground_item: None,
                stacks: None,
                flags: 0,
                zone: ZoneType::Normal,
            }),
        );

        let cid = insert_monster(&mut world, "Spider", open, 20);
        world.map.unregister_creature_at(open, cid);
        let probe = world.probe_spawn_tile(cid, wall, false, 0);
        assert!(
            !probe.login_possible && !probe.expansion_ok,
            "Bank+wp0 dirt wall must block spawn login and BFS expansion"
        );
        assert!(
            !world.items_db.is_unpassable(DIRT_WALL),
            "fixture models cleared blockSolid"
        );
        assert!(
            world.items_db.is_unpassable_for_field(DIRT_WALL),
            "field Unpass must include Bank+wp0"
        );
        assert!(
            !world.monster_move_possible_planning(cid, wall, false),
            "monsters must not plan steps onto Bank+wp0 dirt walls"
        );
        let wall_tile = world.map.get_tile(wall).expect("wall tile");
        assert_eq!(
            crate::walk::tile_query_add_creature(&world, wall_tile, cid, 0),
            crate::return_value::ReturnValue::NotPossible,
            "monster queryAdd must reject cleared Bank+wp0 cliffs"
        );
    }

    #[test]
    fn classic772_spawn_skips_wall_home_picks_neighbor() {
        use crate::item::Item;
        use crate::spawn::SpawnManager;
        use crate::test_support::{
            TEST_SYNTHETIC_GROUND_WP, beat_driven_test_world, ensure_walkable_tile, insert_monster,
        };
        use std::sync::Arc;
        use tfs_rust_content::otb::ItemType;
        use tfs_rust_content::spawns::{SpawnEntry, SpawnZone};

        const WALL: u16 = 9002;
        let mut world = beat_driven_test_world();
        Arc::make_mut(&mut world.items_db).items.insert(
            WALL,
            ItemType {
                server_id: WALL,
                block_solid_override: Some(true),
                moveable_override: Some(false),
                ..ItemType::default()
            },
        );

        let home = Position::new(120, 120, 7);
        ensure_walkable_tile(&mut world.map, home, TEST_SYNTHETIC_GROUND_WP);
        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new(
                        (home.x as i32 + dx) as u16,
                        (home.y as i32 + dy) as u16,
                        home.z,
                    ),
                    TEST_SYNTHETIC_GROUND_WP,
                );
            }
        }
        let wall_item = world.items.insert(Item::new_single(WALL));
        if let Some(tile) = world.map.get_tile_mut(home) {
            tile.body_mut().down_items_mut().push(wall_item);
        }

        world.spawns = SpawnManager::from_zones(vec![SpawnZone {
            center: home,
            radius: 3,
            entries: vec![SpawnEntry::Monster {
                name: "Rat".into(),
                position: home,
                spawntime_ms: 5_000,
                direction: None,
            }],
        }]);

        let cid = insert_monster(&mut world, "Rat", home, 20);
        // `spawn_monster` places before map register — detach harness registration.
        world.map.unregister_creature_at(home, cid);

        assert!(world.place_spawn_creature(cid, 0, home, 3, true, true, false));
        let placed = world.creatures.get(cid).expect("rat").position();
        assert_ne!(placed, home, "must not stay on wall home");
        assert!(
            !world
                .map
                .get_tile(placed)
                .expect("tile")
                .body()
                .down_items()
                .iter()
                .any(|&id| world.items.get(id).is_some_and(|i| i.item_type == WALL))
        );
    }

    /// Occupied saved pos + occupied temple still `Create`s onto temple (`cract.cc:327-349`).
    #[test]
    fn login_temple_create_stacks_when_query_add_would_fail() {
        use crate::test_support::{
            TEST_SYNTHETIC_GROUND_WP, beat_driven_test_world, ensure_walkable_tile, insert_monster,
            insert_player, test_player,
        };

        let mut world = beat_driven_test_world();
        let login = Position::new(130, 130, 7);
        let temple = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, temple, TEST_SYNTHETIC_GROUND_WP);
        insert_monster(&mut world, "TempleGuard", temple, 20);

        for dx in -1..=1 {
            for dy in -1..=1 {
                let pos = Position::new(
                    (login.x as i32 + dx) as u16,
                    (login.y as i32 + dy) as u16,
                    login.z,
                );
                ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
                insert_monster(&mut world, &format!("Block{dx}{dy}"), pos, 20);
            }
        }

        let joiner = insert_player(&mut world, test_player("Joiner", login));
        let placed = world
            .place_player_on_login(joiner, login, 1)
            .expect("temple Create");
        assert_eq!(placed, temple);
        let stacked = world
            .map
            .get_tile(temple)
            .expect("temple tile")
            .body()
            .creatures()
            .to_vec();
        assert!(
            stacked.contains(&joiner),
            "joiner must stack on occupied temple"
        );
        assert!(stacked.len() >= 2);
    }

    #[test]
    fn login_temple_create_fails_when_temple_tile_missing() {
        use crate::test_support::{
            TEST_SYNTHETIC_GROUND_WP, beat_driven_test_world, ensure_walkable_tile, insert_monster,
            insert_player, test_player,
        };
        use tfs_rust_content::otbm::TownData;

        let mut world = beat_driven_test_world();
        let login = Position::new(130, 130, 7);
        for dx in -1..=1 {
            for dy in -1..=1 {
                let pos = Position::new(
                    (login.x as i32 + dx) as u16,
                    (login.y as i32 + dy) as u16,
                    login.z,
                );
                ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
                insert_monster(&mut world, &format!("Block{dx}{dy}"), pos, 20);
            }
        }
        world.map.towns.insert(
            2,
            TownData {
                id: 2,
                name: "VoidTemple".into(),
                temple_position: Position::new(1, 1, 7),
            },
        );
        let joiner = insert_player(&mut world, test_player("Joiner", login));
        assert!(world.place_player_on_login(joiner, login, 2).is_none());
    }
}
