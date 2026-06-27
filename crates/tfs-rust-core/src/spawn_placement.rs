//! 772 spawn tile search — `SearchSpawnField` + monsterhome radius rules.
//!
//! C++ reference: `info.cc` `SearchSpawnField`, `crnonpl.cc` `LoadMonsterhomes` /
//! `ProcessMonsterhomes`.

use tfs_rust_common::enums::ZoneType;
use tfs_rust_common::Position;

use crate::creature::CreatureKind;
use crate::formulas::{SpawnNearPlayer, SpawnPlacement};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::player_flags::{flags_for_group, has_player_flag, PLAYER_FLAG_IGNORED_BY_MONSTERS};
use crate::return_value::ReturnValue;
use crate::tile::flags as tilestate;
use crate::walk::{tile_query_add_creature, FLAG_IGNOREBLOCKITEM};

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
        let CreatureKind::Player(player) = kind else {
            continue;
        };
        if player.ghost_mode {
            continue;
        }
        let flags = flags_for_group(&world.groups, player.group_id);
        if has_player_flag(flags, PLAYER_FLAG_IGNORED_BY_MONSTERS) {
            continue;
        }
        let pos = kind.position();
        if pos.z != home.z {
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
fn spiral_login_positions(center: Position, distance: i32) -> Vec<Position> {
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
    for pos in spiral_login_positions(center, distance) {
        if login_possible(pos) {
            return Some(pos);
        }
    }
    None
}

/// C++ `SearchSpawnField` (`info.cc:911`).
pub(crate) fn search_spawn_field(
    distance: i32,
    center: Position,
    mut probe_at: impl FnMut(Position) -> SpawnTileProbe,
) -> Option<Position> {
    let minimize = distance >= 0;
    let distance = distance.unsigned_abs().min(30) as i32;
    if distance == 0 {
        let probe = probe_at(center);
        return probe.login_possible.then_some(center);
    }

    let grid = (2 * distance + 1) as usize;
    let mut phases = vec![i32::MAX; grid * grid];
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
                    let tie = crate::sim_glibc_rand::parity_random(0, 99)
                        + if probe.login_clean { 100 } else { 0 };
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
}

impl GameWorld {
    pub(crate) fn probe_spawn_tile(
        &self,
        cid: CreatureId,
        pos: Position,
        place_in_pz: bool,
        forced: bool,
    ) -> SpawnTileProbe {
        let Some(tile) = self.map.get_tile(pos) else {
            return SpawnTileProbe {
                login_possible: false,
                login_clean: false,
                expansion_ok: false,
            };
        };
        let body = tile.body();
        if body.ground.is_none() {
            return SpawnTileProbe {
                login_possible: false,
                login_clean: false,
                expansion_ok: false,
            };
        }
        if place_in_pz && body.zone != ZoneType::Protection {
            return SpawnTileProbe {
                login_possible: false,
                login_clean: false,
                expansion_ok: false,
            };
        }

        let expansion_ok = (body.flags & tilestate::IMMOVABLEBLOCKSOLID) == 0;
        let flags = if forced { FLAG_IGNOREBLOCKITEM } else { 0 };
        let ret = tile_query_add_creature(self, tile, cid, flags);
        let login_possible =
            forced || ret == ReturnValue::NoError || ret == ReturnValue::PlayerIsNotInvited;
        let login_clean = login_possible && (body.flags & tilestate::BLOCKSOLID) == 0;

        SpawnTileProbe {
            login_possible,
            login_clean,
            expansion_ok,
        }
    }

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
                let Some(slot) = self.spawns.slot(slot_index) else {
                    return false;
                };
                let home = self
                    .spawns
                    .zone_center(slot.zone_index)
                    .unwrap_or(center);
                let act = self.spawns.count_occupied_in_zone(slot.zone_index);
                let mut effective_radius = home_radius;
                if !startup
                    && self.mechanics.profile.spawn_near_player == SpawnNearPlayer::RadiusShrink
                {
                    effective_radius =
                        shrink_spawn_radius_near_players(self, home, home_radius);
                    if effective_radius < 0 {
                        return false;
                    }
                }
                let signed_dist = classic772_signed_search_distance(effective_radius, act);
                let place_in_pz = self
                    .map
                    .get_tile(home)
                    .map(|t| t.body().zone == ZoneType::Protection)
                    .unwrap_or(false);
                let pos = search_spawn_field(signed_dist, home, |try_pos| {
                    self.probe_spawn_tile(cid, try_pos, place_in_pz, forced)
                });
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

    /// Harness `TCreature::SetOnMap` — `SearchLoginField(dist=1)` (`cract.cc:311`, `info.cc:861`).
    pub(crate) fn harness_place_creature_login(
        &mut self,
        cid: CreatureId,
        requested: Position,
    ) -> Option<Position> {
        const LOGIN_DISTANCE: i32 = 1;
        let pos = search_login_field(requested, LOGIN_DISTANCE, |try_pos| {
            self.probe_spawn_tile(cid, try_pos, false, false)
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

    #[test]
    fn search_spawn_field_minimize_picks_closest() {
        let center = Position::new(10, 10, 7);
        let mut walkable = std::collections::HashSet::new();
        walkable.insert((10, 10));
        walkable.insert((11, 10));
        walkable.insert((15, 15));

        let pos = search_spawn_field(2, center, |p| {
            let key = (p.x, p.y);
            let ok = walkable.contains(&key);
            SpawnTileProbe {
                login_possible: ok,
                login_clean: ok,
                expansion_ok: ok,
            }
        });
        assert_eq!(pos, Some(Position::new(10, 10, 7)));
    }

    #[test]
    fn search_spawn_field_extended_reaches_outer_ring() {
        let center = Position::new(10, 10, 7);
        let far = Position::new(13, 10, 7);

        let pos = search_spawn_field(-3, center, |p| {
            let ok = p == far;
            SpawnTileProbe {
                login_possible: ok,
                login_clean: ok,
                expansion_ok: true,
            }
        });
        assert_eq!(pos, Some(far));
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
}
