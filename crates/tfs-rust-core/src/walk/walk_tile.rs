//! Tile traversal checks for walking — `Tile::queryAdd`, `queryDestination`, height floor changes.
//!
//! - `Tile::queryAdd` monster/player/NPC arms — `tile.cpp` (~484–628).
//! - `Tile::queryDestination` — `tile.cpp` (~735–830).
//! - `Game::internalMoveCreature` height floor change — `game.cpp` (~804–834).
//! - `Tile::hasHeight(n)` — `tile.cpp` (~62–87).

use tfs_rust_common::enums::{Direction, ZoneType};
use tfs_rust_common::Position;
use tfs_rust_content::items::ItemDatabase;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::map::Map;
use crate::player_flags::PLAYER_FLAG_IGNORE_PROTECTION_ZONE;
use crate::return_value::ReturnValue;
use crate::tile::flags as tilestate;

use super::{
    is_diagonal, FLAG_IGNOREBLOCKCREATURE, FLAG_IGNOREBLOCKITEM, FLAG_IGNOREFIELDDAMAGE,
    FLAG_NOLIMIT, FLAG_PATHFINDING,
};

/// Bank+`Waypoints==0` with OTB `blockSolid` cleared for player cliffs (lesson 171).
///
/// Monsters/NPCs must still treat these as 772 Unpass; players keep walking via cleared solid.
fn ground_is_cleared_zero_waypoint_bank(world: &GameWorld, ground: Option<u16>) -> bool {
    let Some(ground_id) = ground else {
        return false;
    };
    let Some(t) = world.items_db.items.get(&ground_id) else {
        return false;
    };
    t.is_terrain_bank() && t.waypoints_raw() == 0 && !t.block_solid()
}

/// TFS `Tile::hasHeight(n)` (`src/tile.cpp` ~62–87) — nth item with `CONST_PROP_HASHEIGHT` along stack.
pub(crate) fn tile_has_height_n(
    pos: Position,
    body: &crate::tile::TileBody,
    items_db: &ItemDatabase,
    items: &slotmap::SlotMap<crate::ids::ItemId, crate::item::Item>,
    n: u32,
) -> bool {
    let mut height = 0u32;
    tracing::debug!(
        "tile_has_height_n: checking tile at {:?}, ground: {:?}, down_items: {:?}, top_items: {:?}",
        pos,
        body.ground,
        body.down_items,
        body.top_items
    );

    if let Some(gid) = body.ground {
        let has_height = items_db.items.get(&gid).is_some_and(|t| t.has_height());
        tracing::debug!(
            "tile_has_height_n: ground item {} has_height: {} at {:?}",
            gid,
            has_height,
            pos
        );
        if has_height {
            height += 1;
            if height == n {
                return true;
            }
        }
    }
    for &item_id in &body.down_items {
        if let Some(item) = items.get(item_id) {
            let has_height = items_db
                .items
                .get(&item.item_type)
                .is_some_and(|t| t.has_height());
            tracing::debug!(
                "tile_has_height_n: down item {:?} (type {}) has_height: {} at {:?}",
                item_id,
                item.item_type,
                has_height,
                pos
            );
            if has_height {
                height += 1;
                if height == n {
                    return true;
                }
            }
        }
    }
    for &item_id in &body.top_items {
        if let Some(item) = items.get(item_id) {
            let has_height = items_db
                .items
                .get(&item.item_type)
                .is_some_and(|t| t.has_height());
            tracing::debug!(
                "tile_has_height_n: top item {:?} (type {}) has_height: {} at {:?}",
                item_id,
                item.item_type,
                has_height,
                pos
            );
            if has_height {
                height += 1;
                if height == n {
                    return true;
                }
            }
        }
    }
    tracing::debug!(
        "tile_has_height_n: total height {} at {:?}, needed {}",
        height,
        pos,
        n
    );
    false
}

/// 772 `GetHeight` (`info.cc:689`) — sums the `ELEVATION` attribute of every `HEIGHT`-flagged
/// object on the tile stack. Distinct from `tile_has_height_n` (a hasHeight **count**);
/// this is an **elevation sum** used by the `CheckMapDestination` floor-change gate (P5/C1).
pub(crate) fn tile_elevation_sum(
    body: &crate::tile::TileBody,
    items_db: &ItemDatabase,
    items: &slotmap::SlotMap<crate::ids::ItemId, crate::item::Item>,
) -> i32 {
    let mut sum = 0i32;
    if let Some(gid) = body.ground {
        if items_db.items.get(&gid).is_some_and(|t| t.has_height()) {
            sum += items_db.items.get(&gid).map(|t| t.elevation()).unwrap_or(0);
        }
    }
    for &item_id in &body.down_items {
        if let Some(item) = items.get(item_id) {
            if let Some(it) = items_db.items.get(&item.item_type) {
                if it.has_height() {
                    sum += it.elevation();
                }
            }
        }
    }
    for &item_id in &body.top_items {
        if let Some(item) = items.get(item_id) {
            if let Some(it) = items_db.items.get(&item.item_type) {
                if it.has_height() {
                    sum += it.elevation();
                }
            }
        }
    }
    sum
}

#[inline]
fn tile_is_hole_like(body: &crate::tile::TileBody) -> bool {
    body.ground.is_none() && (body.flags & tilestate::BLOCKSOLID) == 0
}

/// TFS `Game::internalMoveCreature(Creature*, Direction, flags)` — height-based floor change
/// (`game.cpp` ~804–834). Only runs for cardinal (non-diagonal) player moves.
/// C++ ref: src/game.cpp:797-841
pub(crate) fn resolve_player_move_destination(
    map: &Map,
    items_db: &ItemDatabase,
    items: &slotmap::SlotMap<crate::ids::ItemId, crate::item::Item>,
    current_pos: Position,
    direction: Direction,
    mut flags: u32,
) -> (Position, u32) {
    let mut dest_pos = current_pos.offset(direction);
    if is_diagonal(direction) {
        return (dest_pos, flags);
    }

    // C++ ref: src/game.cpp:807-820 — try to go up
    if current_pos.z != 8 {
        if let Some(cur_tile) = map.get_tile(current_pos) {
            let has_h3 = tile_has_height_n(current_pos, cur_tile.body(), items_db, items, 3);
            if has_h3 {
                let z_above = current_pos.z.wrapping_sub(1);
                let tmp = map.get_tile(Position {
                    x: current_pos.x,
                    y: current_pos.y,
                    z: z_above,
                });
                let open = tmp.map(|t| tile_is_hole_like(t.body())).unwrap_or(true);
                if open {
                    let tmp2 = map.get_tile(Position {
                        x: dest_pos.x,
                        y: dest_pos.y,
                        z: z_above,
                    });
                    if let Some(tt) = tmp2 {
                        let tb = tt.body();
                        if tb.ground.is_some() && (tb.flags & tilestate::IMMOVABLEBLOCKSOLID) == 0 {
                            flags |= FLAG_IGNOREBLOCKITEM | FLAG_IGNOREBLOCKCREATURE;
                            if (tb.flags & tilestate::FLOORCHANGE) == 0 {
                                dest_pos.z = z_above;
                            }
                        }
                    }
                }
            }
        }
    }

    // C++ ref: src/game.cpp:823-833 — try to go down
    if current_pos.z != 7 && current_pos.z == dest_pos.z {
        let tmp = map.get_tile(dest_pos);
        let open = tmp.map(|t| tile_is_hole_like(t.body())).unwrap_or(true);
        if open {
            let z_below = dest_pos.z.wrapping_add(1);
            if let Some(tt) = map.get_tile(Position {
                x: dest_pos.x,
                y: dest_pos.y,
                z: z_below,
            }) {
                let tb = tt.body();
                if tile_has_height_n(
                    Position {
                        x: dest_pos.x,
                        y: dest_pos.y,
                        z: z_below,
                    },
                    tb,
                    items_db,
                    items,
                    3,
                ) && (tb.flags & tilestate::IMMOVABLEBLOCKSOLID) == 0
                {
                    flags |= FLAG_IGNOREBLOCKITEM | FLAG_IGNOREBLOCKCREATURE;
                    dest_pos.z = z_below;
                }
            }
        }
    }

    (dest_pos, flags)
}

/// TFS `Tile::queryDestination` — flag-based floor change after creature has landed on a tile.
/// Called in a while-loop by `internalMoveCreature(Creature&, Tile&, flags)`.
/// C++ ref: src/tile.cpp:735-830
pub(crate) fn query_destination(
    map: &Map,
    tile_pos: Position,
    tile_flags: u32,
) -> Option<(Position, u32)> {
    if tile_flags & tilestate::FLOORCHANGE_DOWN != 0 {
        // C++ ref: src/tile.cpp:740-784
        let mut dx = tile_pos.x;
        let mut dy = tile_pos.y;
        let dz = tile_pos.z.wrapping_add(1);

        // Check south-alt first
        if let Some(south_down) = map.get_tile(Position {
            x: dx,
            y: dy.wrapping_sub(1),
            z: dz,
        }) {
            if south_down.body().flags & tilestate::FLOORCHANGE_SOUTH_ALT != 0 {
                dy = dy.wrapping_sub(2);
                let dest = map.get_tile(Position {
                    x: dx,
                    y: dy,
                    z: dz,
                });
                return dest.map(|_| {
                    (
                        Position {
                            x: dx,
                            y: dy,
                            z: dz,
                        },
                        FLAG_NOLIMIT,
                    )
                });
            }
        }

        // Check east-alt
        if let Some(east_down) = map.get_tile(Position {
            x: dx.wrapping_sub(1),
            y: dy,
            z: dz,
        }) {
            if east_down.body().flags & tilestate::FLOORCHANGE_EAST_ALT != 0 {
                dx = dx.wrapping_sub(2);
                let dest = map.get_tile(Position {
                    x: dx,
                    y: dy,
                    z: dz,
                });
                return dest.map(|_| {
                    (
                        Position {
                            x: dx,
                            y: dy,
                            z: dz,
                        },
                        FLAG_NOLIMIT,
                    )
                });
            }
        }

        // Normal directional check on the tile below
        if let Some(down_tile) = map.get_tile(Position {
            x: dx,
            y: dy,
            z: dz,
        }) {
            let df = down_tile.body().flags;
            if df & tilestate::FLOORCHANGE_NORTH != 0 {
                dy = dy.wrapping_add(1);
            }
            if df & tilestate::FLOORCHANGE_SOUTH != 0 {
                dy = dy.wrapping_sub(1);
            }
            if df & tilestate::FLOORCHANGE_SOUTH_ALT != 0 {
                dy = dy.wrapping_sub(2);
            }
            if df & tilestate::FLOORCHANGE_EAST != 0 {
                dx = dx.wrapping_sub(1);
            }
            if df & tilestate::FLOORCHANGE_EAST_ALT != 0 {
                dx = dx.wrapping_sub(2);
            }
            if df & tilestate::FLOORCHANGE_WEST != 0 {
                dx = dx.wrapping_add(1);
            }
        }

        let dest = map.get_tile(Position {
            x: dx,
            y: dy,
            z: dz,
        });
        return dest.map(|_| {
            (
                Position {
                    x: dx,
                    y: dy,
                    z: dz,
                },
                FLAG_NOLIMIT,
            )
        });
    }

    // C++ ref: src/tile.cpp:785-814 — upward floor change (any non-DOWN floorchange flag)
    if tile_flags & tilestate::FLOORCHANGE != 0 {
        let mut dx = tile_pos.x;
        let mut dy = tile_pos.y;
        let dz = tile_pos.z.wrapping_sub(1);

        if tile_flags & tilestate::FLOORCHANGE_NORTH != 0 {
            dy = dy.wrapping_sub(1);
        }
        if tile_flags & tilestate::FLOORCHANGE_SOUTH != 0 {
            dy = dy.wrapping_add(1);
        }
        if tile_flags & tilestate::FLOORCHANGE_EAST != 0 {
            dx = dx.wrapping_add(1);
        }
        if tile_flags & tilestate::FLOORCHANGE_WEST != 0 {
            dx = dx.wrapping_sub(1);
        }
        if tile_flags & tilestate::FLOORCHANGE_SOUTH_ALT != 0 {
            dy = dy.wrapping_add(2);
        }
        if tile_flags & tilestate::FLOORCHANGE_EAST_ALT != 0 {
            dx = dx.wrapping_add(2);
        }

        let dest = map.get_tile(Position {
            x: dx,
            y: dy,
            z: dz,
        });
        return dest.map(|_| {
            (
                Position {
                    x: dx,
                    y: dy,
                    z: dz,
                },
                FLAG_NOLIMIT,
            )
        });
    }

    None
}

/// TFS `Tile::queryAdd` monster branch (`tile.cpp` ~499–563).
pub(crate) fn tile_query_add_monster(
    world: &GameWorld,
    tile: &crate::tile::Tile,
    mover: CreatureId,
    flags: u32,
) -> ReturnValue {
    let body = tile.body();

    if (flags & FLAG_NOLIMIT) != 0 {
        return ReturnValue::NoError;
    }

    if body.ground.is_none() {
        return ReturnValue::NotPossible;
    }

    // Bank+wp0 with blockSolid cleared for player cliffs (lesson 171) — monsters still Unpass.
    if ground_is_cleared_zero_waypoint_bank(world, body.ground) {
        return ReturnValue::NotPossible;
    }

    if (flags & FLAG_PATHFINDING) != 0
        && (body.flags & (tilestate::FLOORCHANGE | tilestate::TELEPORT)) != 0
    {
        return ReturnValue::NotPossible;
    }

    if (body.flags & (tilestate::PROTECTIONZONE | tilestate::FLOORCHANGE | tilestate::TELEPORT))
        != 0
    {
        return ReturnValue::NotPossible;
    }

    // `canpushcreatures` / `canpushitems` from monster type at spawn.
    let (can_push_creatures, can_push_items, is_summon) = match world.creatures.get(mover) {
        Some(CreatureKind::Monster(m)) => {
            (m.can_push_creatures, m.can_push_items, m.base.is_summon())
        }
        _ => (false, false, false),
    };

    if (flags & FLAG_IGNOREBLOCKCREATURE) == 0 {
        if can_push_creatures && !is_summon {
            for &tile_c in &body.creatures {
                if tile_c == mover {
                    continue;
                }
                let other_ghost = world
                    .creatures
                    .get(tile_c)
                    .is_some_and(|k| matches!(k, CreatureKind::Player(p) if p.ghost_mode));
                if other_ghost {
                    continue;
                }
                let Some(other) = world.creatures.get(tile_c) else {
                    return ReturnValue::NotPossible;
                };
                let other_monster_pushable = match other {
                    CreatureKind::Monster(m) => m.is_pushable(),
                    _ => false,
                };
                let other_summon_with_player_master = matches!(other, CreatureKind::Monster(_))
                    && other.is_summon()
                    && other
                        .base()
                        .master
                        .and_then(|mid| world.creatures.get(mid))
                        .is_some_and(|m| matches!(m, CreatureKind::Player(_)));
                if !other_monster_pushable || other_summon_with_player_master {
                    return ReturnValue::NotPossible;
                }
            }
        } else if !body.creatures.is_empty() {
            for &tile_c in &body.creatures {
                if tile_c == mover {
                    continue;
                }
                let other_ghost = world
                    .creatures
                    .get(tile_c)
                    .is_some_and(|k| matches!(k, CreatureKind::Player(p) if p.ghost_mode));
                if !other_ghost {
                    return ReturnValue::NotEnoughRoom;
                }
            }
        }
    }

    if (body.flags & tilestate::IMMOVABLEBLOCKSOLID) != 0 {
        return ReturnValue::NotPossible;
    }

    // 772 `AVOID` furniture/terrain — immovable `blockPathFind` (stairs, trapdoors) is
    // a hard block for monsters (`crnonpl.cc:2268` `UNMOVE || !CanKickBoxes`). Movable
    // furniture (chairs, boxes) is kickable when `can_push_items`.
    if (body.flags & tilestate::IMMOVABLEBLOCKPATH) != 0
        && !(can_push_items || (flags & FLAG_IGNOREBLOCKITEM) != 0)
    {
        return ReturnValue::NotPossible;
    }

    if (flags & FLAG_PATHFINDING) != 0 && (body.flags & tilestate::IMMOVABLENOFIELDBLOCKPATH) != 0 {
        return ReturnValue::NotPossible;
    }

    if ((body.flags & tilestate::BLOCKSOLID) != 0
        || (body.flags & tilestate::BLOCKPATH) != 0
        || ((flags & FLAG_PATHFINDING) != 0 && (body.flags & tilestate::NOFIELDBLOCKPATH) != 0))
        && !(can_push_items || (flags & FLAG_IGNOREBLOCKITEM) != 0)
    {
        return ReturnValue::NotPossible;
    }

    // Full field immunity deferred until Monster combat fields land; block damaging fields without ignore flag.
    if (body.flags & tilestate::MAGICFIELD) != 0 && (flags & FLAG_IGNOREFIELDDAMAGE) == 0 {
        return ReturnValue::NotPossible;
    }

    ReturnValue::NoError
}

/// TFS `Tile::queryAdd` NPC / generic creature branch (`tile.cpp` ~565–628).
///
/// Unlike monsters, NPCs are **not** barred from protection zones (temple/depot
/// spawns). House tiles are rejected here; `HouseTile::queryAdd` is TFS-only for
/// invited players.
pub(crate) fn tile_query_add_npc(
    world: &GameWorld,
    tile: &crate::tile::Tile,
    mover: CreatureId,
    flags: u32,
) -> ReturnValue {
    if matches!(tile, crate::tile::Tile::House(_)) {
        return ReturnValue::NotPossible;
    }

    let body = tile.body();

    if (flags & FLAG_NOLIMIT) != 0 {
        return ReturnValue::NoError;
    }

    if body.ground.is_none() {
        return ReturnValue::NotPossible;
    }

    // Same cleared Bank+wp0 cliff Unpass as monsters — players still walk via cleared blockSolid.
    if ground_is_cleared_zero_waypoint_bank(world, body.ground) {
        return ReturnValue::NotPossible;
    }

    if (flags & FLAG_PATHFINDING) != 0
        && (body.flags & (tilestate::FLOORCHANGE | tilestate::TELEPORT)) != 0
    {
        return ReturnValue::NotPossible;
    }

    if (flags & FLAG_IGNOREBLOCKCREATURE) == 0 && !body.creatures.is_empty() {
        for &tile_c in &body.creatures {
            if tile_c == mover {
                continue;
            }
            let other_ghost = world
                .creatures
                .get(tile_c)
                .is_some_and(|k| matches!(k, CreatureKind::Player(p) if p.ghost_mode));
            if !other_ghost {
                return ReturnValue::NotEnoughRoom;
            }
        }
    }

    if (flags & FLAG_IGNOREBLOCKITEM) == 0 {
        if (body.flags & tilestate::BLOCKSOLID) != 0 {
            return ReturnValue::NotEnoughRoom;
        }
        // 772 `AVOID` furniture/terrain (OTB `blockPathFind`, `AvoidDamageTypes=0`).
        // NPCs reject all AVOID tiles (`crnonpl.cc:1675` `!CoordinateFlag(AVOID)`).
        if (body.flags & tilestate::BLOCKPATH) != 0 {
            return ReturnValue::NotEnoughRoom;
        }
        if (flags & FLAG_PATHFINDING) != 0 && (body.flags & tilestate::NOFIELDBLOCKPATH) != 0 {
            return ReturnValue::NotPossible;
        }
    } else if let Some(ground_id) = body.ground {
        if let Some(gt) = world.items_db.items.get(&ground_id) {
            if gt.block_solid() && !gt.moveable() {
                return ReturnValue::NotPossible;
            }
        }
        for &item_id in body.top_items.iter().chain(body.down_items.iter()) {
            if let Some(item) = world.items.get(item_id) {
                if let Some(it) = world.items_db.items.get(&item.item_type) {
                    if it.block_solid() && !it.moveable() {
                        return ReturnValue::NotPossible;
                    }
                }
            }
        }
    }

    ReturnValue::NoError
}

/// TFS `Tile::queryAdd` for player creatures.
/// C++ ref: src/tile.cpp:484-628
/// 772 PZ-entry lock: `TPlayer::MovePossible` — `crplayer.cc:366-369` (`ENTERPROTECTIONZONE`).
pub(crate) fn tile_query_add_player(
    world: &GameWorld,
    tile: &crate::tile::Tile,
    mover: CreatureId,
    flags: u32,
) -> ReturnValue {
    let body = tile.body();

    // C++ ref: src/tile.cpp:487-488 — FLAG_NOLIMIT bypasses all checks.
    if (flags & FLAG_NOLIMIT) != 0 {
        return ReturnValue::NoError;
    }

    if body.ground.is_none() {
        return ReturnValue::NotPossible;
    }

    // C++ ref: src/tile.cpp:491-493 — skip floor-change / teleport tiles while pathfinding.
    if (flags & FLAG_PATHFINDING) != 0
        && (body.flags & (tilestate::FLOORCHANGE | tilestate::TELEPORT)) != 0
    {
        return ReturnValue::NotPossible;
    }

    // C++ ref: src/tile.cpp:531-533 (monster); same flag checked for players on path tiles.
    if (flags & FLAG_PATHFINDING) != 0 && (body.flags & tilestate::IMMOVABLENOFIELDBLOCKPATH) != 0 {
        return ReturnValue::NotPossible;
    }

    // 772 `EarliestProtectionZoneRound` entry gate — `crplayer.cc:366-369`.
    // TFS domain: `tile.cpp:581-596` `isPzLocked` + enter `TILESTATE_PROTECTIONZONE`.
    // Skip when already standing in PZ (movement within / out of PZ is allowed).
    if body.zone == ZoneType::Protection {
        let pz_lock_from = match world.creatures.get(mover) {
            Some(CreatureKind::Player(p))
                if p.earliest_protection_zone_round > world.round_nr =>
            {
                Some(p.base.position)
            }
            _ => None,
        };
        if let Some(cur_pos) = pz_lock_from {
            if !world.player_has_flag(mover, PLAYER_FLAG_IGNORE_PROTECTION_ZONE) {
                let currently_in_pz = world
                    .map
                    .get_tile(cur_pos)
                    .is_some_and(|t| t.body().zone == ZoneType::Protection);
                if !currently_in_pz {
                    return ReturnValue::PlayerIsPzLocked;
                }
            }
        }
    }

    // C++ ref: src/tile.cpp:567-573 — creature blocking (players)
    if (flags & FLAG_IGNOREBLOCKCREATURE) == 0 {
        for &tile_c in &body.creatures {
            if tile_c == mover {
                continue;
            }
            let other_ghost = world
                .creatures
                .get(tile_c)
                .is_some_and(|k| matches!(k, CreatureKind::Player(p) if p.ghost_mode));
            if !other_ghost {
                return ReturnValue::NotPossible;
            }
        }
    }

    // C++ ref: src/tile.cpp:606-628 — block solid checks, respecting FLAG_IGNOREBLOCKITEM.
    if (flags & FLAG_IGNOREBLOCKITEM) == 0 {
        if (body.flags & tilestate::BLOCKSOLID) != 0 {
            return ReturnValue::NotEnoughRoom;
        }
        // C++ ref: src/tile.cpp:535 — `TILESTATE_NOFIELDBLOCKPATH` with `FLAG_PATHFINDING`.
        if (flags & FLAG_PATHFINDING) != 0 && (body.flags & tilestate::NOFIELDBLOCKPATH) != 0 {
            return ReturnValue::NotPossible;
        }
    } else {
        // FLAG_IGNOREBLOCKITEM is set — only block on *immovable* blocksolid items.
        // C++ ref: src/tile.cpp:613-627
        if let Some(ground_id) = body.ground {
            if let Some(gt) = world.items_db.items.get(&ground_id) {
                if gt.block_solid() && !gt.moveable() {
                    return ReturnValue::NotPossible;
                }
            }
        }
        for &item_id in body.top_items.iter().chain(body.down_items.iter()) {
            if let Some(item) = world.items.get(item_id) {
                if let Some(it) = world.items_db.items.get(&item.item_type) {
                    if it.block_solid() && !it.moveable() {
                        return ReturnValue::NotPossible;
                    }
                }
            }
        }
    }

    // 772 `MovePossible(Execute=false)` blocks on `AVOID` (`crmain.cc:893`).
    // `AVOID` maps to `MAGICFIELD` tile-state. Gated on `!FLAG_IGNOREFIELDDAMAGE`
    // so actual walk execution (which sets that flag) can still enter fields
    // and take damage, matching `MovePossible(Execute=true)` skipping the AVOID check.
    if (body.flags & tilestate::MAGICFIELD) != 0 && (flags & FLAG_IGNOREFIELDDAMAGE) == 0 {
        return ReturnValue::NotPossible;
    }

    ReturnValue::NoError
}

#[cfg(test)]
mod pz_entry_lock_tests {
    use super::*;
    use crate::creature::CreatureKind;
    use crate::sim_harness::{
        beat_driven_test_world, ensure_walkable_tile, insert_player, test_player,
        TEST_SYNTHETIC_GROUND_WP,
    };
    use crate::tile::{Tile, TileBody};
    use tfs_rust_common::enums::ZoneType;
    use tfs_rust_common::Position;

    fn ensure_pz_tile(map: &mut crate::map::Map, pos: Position) {
        map.insert_tile(
            pos,
            Tile::Normal(TileBody {
                ground: Some(TEST_SYNTHETIC_GROUND_WP),

                ground_item: None,
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: tilestate::PROTECTIONZONE,
                zone: ZoneType::Protection,
            }),
        );
    }

    /// 772 `crplayer.cc:366-369` — locked player cannot step Normal → Protection.
    #[test]
    fn pz_locked_blocks_entry_from_normal() {
        let mut world = beat_driven_test_world();
        world.round_nr = 100;
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, from, TEST_SYNTHETIC_GROUND_WP);
        ensure_pz_tile(&mut world.map, to);
        let cid = insert_player(&mut world, test_player("Locked", from));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            p.earliest_protection_zone_round = 160;
        }
        let dest = world.map.get_tile(to).expect("pz tile");
        assert_eq!(
            tile_query_add_player(&world, dest, cid, 0),
            ReturnValue::PlayerIsPzLocked
        );
    }

    /// Already standing in PZ may move to another PZ tile while locked.
    #[test]
    fn pz_locked_allows_move_within_pz() {
        let mut world = beat_driven_test_world();
        world.round_nr = 100;
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        ensure_pz_tile(&mut world.map, from);
        ensure_pz_tile(&mut world.map, to);
        let cid = insert_player(&mut world, test_player("InPz", from));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            p.earliest_protection_zone_round = 160;
        }
        let dest = world.map.get_tile(to).expect("pz tile");
        assert_eq!(
            tile_query_add_player(&world, dest, cid, 0),
            ReturnValue::NoError
        );
    }

    /// Expired lock (`earliest <= round_nr`) allows PZ entry.
    #[test]
    fn expired_pz_lock_allows_entry() {
        let mut world = beat_driven_test_world();
        world.round_nr = 160;
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, from, TEST_SYNTHETIC_GROUND_WP);
        ensure_pz_tile(&mut world.map, to);
        let cid = insert_player(&mut world, test_player("Free", from));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            p.earliest_protection_zone_round = 160;
        }
        let dest = world.map.get_tile(to).expect("pz tile");
        assert_eq!(
            tile_query_add_player(&world, dest, cid, 0),
            ReturnValue::NoError
        );
    }
}
