//! Tile traversal checks for walking — `Tile::queryAdd`, `queryDestination`, height floor changes.
//!
//! - `Tile::queryAdd` monster/player/NPC arms — `tile.cpp` (~484–628).
//! - `Tile::queryDestination` — `tile.cpp` (~735–830).
//! - `Game::internalMoveCreature` height floor change — `game.cpp` (~804–834).
//! - `Tile::hasHeight(n)` — `tile.cpp` (~62–87).

use tfs_rust_common::enums::Direction;
use tfs_rust_common::Position;
use tfs_rust_content::items::ItemDatabase;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::map::Map;
use crate::return_value::ReturnValue;
use crate::tile::flags as tilestate;

use super::{
    is_diagonal, FLAG_IGNOREBLOCKCREATURE, FLAG_IGNOREBLOCKITEM, FLAG_IGNOREFIELDDAMAGE,
    FLAG_NOLIMIT, FLAG_PATHFINDING,
};

/// TFS `Tile::hasHeight(n)` (`src/tile.cpp` ~62–87) — nth item with `CONST_PROP_HASHEIGHT` along stack.
fn tile_has_height_n(
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
        tracing::debug!("tile_has_height_n: ground item {} has_height: {} at {:?}", gid, has_height, pos);
        if has_height {
            height += 1;
            if height == n {
                return true;
            }
        }
    }
    for &item_id in &body.down_items {
        if let Some(item) = items.get(item_id) {
            let has_height = items_db.items.get(&item.item_type).is_some_and(|t| t.has_height());
            tracing::debug!("tile_has_height_n: down item {:?} (type {}) has_height: {} at {:?}", item_id, item.item_type, has_height, pos);
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
            let has_height = items_db.items.get(&item.item_type).is_some_and(|t| t.has_height());
            tracing::debug!("tile_has_height_n: top item {:?} (type {}) has_height: {} at {:?}", item_id, item.item_type, has_height, pos);
            if has_height {
                height += 1;
                if height == n {
                    return true;
                }
            }
        }
    }
    tracing::debug!("tile_has_height_n: total height {} at {:?}, needed {}", height, pos, n);
    false
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
                let tmp = map.get_tile(Position { x: current_pos.x, y: current_pos.y, z: z_above });
                let open = tmp.map(|t| tile_is_hole_like(t.body())).unwrap_or(true);
                if open {
                    let tmp2 = map.get_tile(Position { x: dest_pos.x, y: dest_pos.y, z: z_above });
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
            if let Some(tt) = map.get_tile(Position { x: dest_pos.x, y: dest_pos.y, z: z_below }) {
                let tb = tt.body();
                if tile_has_height_n(
                    Position { x: dest_pos.x, y: dest_pos.y, z: z_below },
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
        if let Some(south_down) = map.get_tile(Position { x: dx, y: dy.wrapping_sub(1), z: dz }) {
            if south_down.body().flags & tilestate::FLOORCHANGE_SOUTH_ALT != 0 {
                dy = dy.wrapping_sub(2);
                let dest = map.get_tile(Position { x: dx, y: dy, z: dz });
                return dest.map(|_| (Position { x: dx, y: dy, z: dz }, FLAG_NOLIMIT));
            }
        }

        // Check east-alt
        if let Some(east_down) = map.get_tile(Position { x: dx.wrapping_sub(1), y: dy, z: dz }) {
            if east_down.body().flags & tilestate::FLOORCHANGE_EAST_ALT != 0 {
                dx = dx.wrapping_sub(2);
                let dest = map.get_tile(Position { x: dx, y: dy, z: dz });
                return dest.map(|_| (Position { x: dx, y: dy, z: dz }, FLAG_NOLIMIT));
            }
        }

        // Normal directional check on the tile below
        if let Some(down_tile) = map.get_tile(Position { x: dx, y: dy, z: dz }) {
            let df = down_tile.body().flags;
            if df & tilestate::FLOORCHANGE_NORTH != 0 { dy = dy.wrapping_add(1); }
            if df & tilestate::FLOORCHANGE_SOUTH != 0 { dy = dy.wrapping_sub(1); }
            if df & tilestate::FLOORCHANGE_SOUTH_ALT != 0 { dy = dy.wrapping_sub(2); }
            if df & tilestate::FLOORCHANGE_EAST != 0 { dx = dx.wrapping_sub(1); }
            if df & tilestate::FLOORCHANGE_EAST_ALT != 0 { dx = dx.wrapping_sub(2); }
            if df & tilestate::FLOORCHANGE_WEST != 0 { dx = dx.wrapping_add(1); }
        }

        let dest = map.get_tile(Position { x: dx, y: dy, z: dz });
        return dest.map(|_| (Position { x: dx, y: dy, z: dz }, FLAG_NOLIMIT));
    }

    // C++ ref: src/tile.cpp:785-814 — upward floor change (any non-DOWN floorchange flag)
    if tile_flags & tilestate::FLOORCHANGE != 0 {
        let mut dx = tile_pos.x;
        let mut dy = tile_pos.y;
        let dz = tile_pos.z.wrapping_sub(1);

        if tile_flags & tilestate::FLOORCHANGE_NORTH != 0 { dy = dy.wrapping_sub(1); }
        if tile_flags & tilestate::FLOORCHANGE_SOUTH != 0 { dy = dy.wrapping_add(1); }
        if tile_flags & tilestate::FLOORCHANGE_EAST != 0 { dx = dx.wrapping_add(1); }
        if tile_flags & tilestate::FLOORCHANGE_WEST != 0 { dx = dx.wrapping_sub(1); }
        if tile_flags & tilestate::FLOORCHANGE_SOUTH_ALT != 0 { dy = dy.wrapping_add(2); }
        if tile_flags & tilestate::FLOORCHANGE_EAST_ALT != 0 { dx = dx.wrapping_add(2); }

        let dest = map.get_tile(Position { x: dx, y: dy, z: dz });
        return dest.map(|_| (Position { x: dx, y: dy, z: dz }, FLAG_NOLIMIT));
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

    if (flags & FLAG_PATHFINDING) != 0
        && (body.flags & (tilestate::FLOORCHANGE | tilestate::TELEPORT)) != 0
    {
        return ReturnValue::NotPossible;
    }

    if (body.flags & (tilestate::PROTECTIONZONE | tilestate::FLOORCHANGE | tilestate::TELEPORT)) != 0 {
        return ReturnValue::NotPossible;
    }

    // `canpushcreatures` / `canpushitems` from monster type at spawn.
    let (can_push_creatures, can_push_items, is_summon) = match world.creatures.get(mover) {
        Some(CreatureKind::Monster(m)) => (m.can_push_creatures, m.can_push_items, m.base.is_summon()),
        _ => (false, false, false),
    };

    if (flags & FLAG_IGNOREBLOCKCREATURE) == 0 {
        if can_push_creatures && !is_summon {
            for &tile_c in &body.creatures {
                if tile_c == mover {
                    continue;
                }
                let other_ghost = world.creatures.get(tile_c).is_some_and(|k| {
                    matches!(k, CreatureKind::Player(p) if p.ghost_mode)
                });
                if other_ghost {
                    continue;
                }
                let Some(other) = world.creatures.get(tile_c) else {
                    return ReturnValue::NotPossible;
                };
                let other_monster_pushable = matches!(other, CreatureKind::Monster(_));
                let other_summon_with_player_master = other.is_summon()
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
                let other_ghost = world.creatures.get(tile_c).is_some_and(|k| {
                    matches!(k, CreatureKind::Player(p) if p.ghost_mode)
                });
                if !other_ghost {
                    return ReturnValue::NotEnoughRoom;
                }
            }
        }
    }

    if (body.flags & tilestate::IMMOVABLEBLOCKSOLID) != 0 {
        return ReturnValue::NotPossible;
    }

    if (flags & FLAG_PATHFINDING) != 0 && (body.flags & tilestate::IMMOVABLENOFIELDBLOCKPATH) != 0 {
        return ReturnValue::NotPossible;
    }

    if ((body.flags & tilestate::BLOCKSOLID) != 0
        || ((flags & FLAG_PATHFINDING) != 0 && (body.flags & tilestate::NOFIELDBLOCKPATH) != 0))
        && !(can_push_items || (flags & FLAG_IGNOREBLOCKITEM) != 0) {
            return ReturnValue::NotPossible;
        }

    // Full field immunity deferred until Monster combat fields land; block damaging fields without ignore flag.
    if (body.flags & tilestate::MAGICFIELD) != 0 && (flags & FLAG_IGNOREFIELDDAMAGE) == 0 {
        return ReturnValue::NotPossible;
    }

    ReturnValue::NoError
}

/// TFS `Tile::queryAdd` NPC / generic creature branch (`tile.cpp` ~598–628); NPCs cannot enter houses or PZ.
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

    if (flags & FLAG_PATHFINDING) != 0
        && (body.flags & (tilestate::FLOORCHANGE | tilestate::TELEPORT)) != 0
    {
        return ReturnValue::NotPossible;
    }

    if (body.flags & tilestate::PROTECTIONZONE) != 0 {
        return ReturnValue::NotPossible;
    }

    if (flags & FLAG_IGNOREBLOCKCREATURE) == 0 && !body.creatures.is_empty() {
        for &tile_c in &body.creatures {
            if tile_c == mover {
                continue;
            }
            let other_ghost = world.creatures.get(tile_c).is_some_and(|k| {
                matches!(k, CreatureKind::Player(p) if p.ghost_mode)
            });
            if !other_ghost {
                return ReturnValue::NotEnoughRoom;
            }
        }
    }

    if (flags & FLAG_IGNOREBLOCKITEM) == 0 {
        if (body.flags & tilestate::BLOCKSOLID) != 0 {
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

    // C++ ref: src/tile.cpp:567-573 — creature blocking (players)
    if (flags & FLAG_IGNOREBLOCKCREATURE) == 0 {
        for &tile_c in &body.creatures {
            if tile_c == mover {
                continue;
            }
            let other_ghost = world.creatures.get(tile_c).is_some_and(|k| {
                matches!(k, CreatureKind::Player(p) if p.ghost_mode)
            });
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

    ReturnValue::NoError
}
