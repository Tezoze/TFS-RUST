//! 772 elevation-climb walk tests (audit Step 8 G1–G4).
//!
//! Corpus: `GetHeight` (`info.cc:689`), `GoExec` climb (`cract.cc` ~415–431),
//! `JumpPossible` (`info.cc:702`). Part A (7.4 step-up limit) is not in scope.

use std::time::Instant;

use super::remap_walk_step_err;
use super::walk_tile::{tile_elevation_sum, try_player_elevation_climb};
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::return_value::ReturnValue;
use crate::sim_harness::{
    TEST_SYNTHETIC_GROUND_WP, beat_driven_test_world, ensure_walkable_tile, insert_player,
    test_player,
};
use crate::tile::flags as tilestate;
use tfs_rust_common::Position;
use tfs_rust_common::enums::Direction;
use tfs_rust_content::otb::ItemType;

/// Avoids collision with synthetic ground 150 / bag 1987 / gold 2148.
const HEIGHT_ITEM_TYPE: u16 = 500;

fn height_item_type(elevation: i32) -> ItemType {
    ItemType {
        server_id: HEIGHT_ITEM_TYPE,
        flags: 1 << 3, // FLAG_HAS_HEIGHT
        elevation,
        ..Default::default()
    }
}

fn register_height_item(world: &mut GameWorld, elevation: i32) {
    let mut new_db = (*world.items_db).clone();
    new_db
        .items
        .insert(HEIGHT_ITEM_TYPE, height_item_type(elevation));
    world.items_db = std::sync::Arc::new(new_db);
}

fn place_height_item(world: &mut GameWorld, pos: Position) {
    let item_id = world
        .items
        .insert(crate::item::Item::new_single(HEIGHT_ITEM_TYPE));
    if let Some(tile) = world.map.get_tile_mut(pos) {
        tile.body_mut().down_items.push(item_id);
    }
}

fn place_n_height_items(world: &mut GameWorld, pos: Position, n: usize) {
    for _ in 0..n {
        place_height_item(world, pos);
    }
}

fn insert_player_on_tile(world: &mut GameWorld, name: &str, pos: Position) -> CreatureId {
    let cid = insert_player(world, test_player(name, pos));
    world.map.register_creature_at(pos, cid);
    cid
}

fn walk_east(world: &mut GameWorld, cid: CreatureId) -> Result<Position, ReturnValue> {
    world
        .internal_move_creature_step(cid, Direction::East, Instant::now())
        .map(|_| {
            world
                .creatures
                .get(cid)
                .map(|k| k.position())
                .expect("player still in world")
        })
}

/// G1: three HEIGHT items with field elevation 0 sum to 24 via the default-8 accessor.
#[test]
fn three_default_height_items_sum_to_24() {
    let mut world = beat_driven_test_world();
    register_height_item(&mut world, 0);
    let pos = Position::new(100, 100, 7);
    ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
    place_n_height_items(&mut world, pos, 3);
    let body = world.map.get_tile(pos).expect("tile").body();
    assert_eq!(
        tile_elevation_sum(body, world.items_db.as_ref(), &world.items),
        24
    );
}

/// G2: player on a 3-stack next to walkable ground stays on the same floor.
#[test]
fn climb_not_taken_when_flat_move_succeeds() {
    let mut world = beat_driven_test_world();
    register_height_item(&mut world, 0);
    let origin = Position::new(100, 100, 7);
    let dest = Position::new(101, 100, 7);
    let above_dest = Position::new(101, 100, 6);
    ensure_walkable_tile(&mut world.map, origin, TEST_SYNTHETIC_GROUND_WP);
    ensure_walkable_tile(&mut world.map, dest, TEST_SYNTHETIC_GROUND_WP);
    // Climb dest exists so a premature climb would succeed and be visible.
    ensure_walkable_tile(&mut world.map, above_dest, TEST_SYNTHETIC_GROUND_WP);
    place_n_height_items(&mut world, origin, 3);
    let cid = insert_player_on_tile(&mut world, "Walker", origin);

    let landed = walk_east(&mut world, cid).expect("flat walk");
    assert_eq!(landed, dest);
    assert_eq!(landed.z, origin.z);
}

/// G2: climb up only after the flat dest fails `MovePossible(Jump=false)`.
#[test]
fn climb_up_when_flat_blocked() {
    let mut world = beat_driven_test_world();
    register_height_item(&mut world, 0);
    let origin = Position::new(100, 100, 7);
    let climb_dest = Position::new(101, 100, 6);
    ensure_walkable_tile(&mut world.map, origin, TEST_SYNTHETIC_GROUND_WP);
    // Flat dest omitted → !BANK. Air above origin omitted → !BANK && !UNPASS.
    ensure_walkable_tile(&mut world.map, climb_dest, TEST_SYNTHETIC_GROUND_WP);
    place_n_height_items(&mut world, origin, 3);
    let cid = insert_player_on_tile(&mut world, "Climber", origin);

    let landed = walk_east(&mut world, cid).expect("climb up");
    assert_eq!(landed, climb_dest);
    assert_eq!(landed.z, origin.z - 1);
}

/// G3: climbing `z=8 → z=7` is allowed (TFS refused `currentPos.z == 8`).
#[test]
fn climb_up_from_z8() {
    let mut world = beat_driven_test_world();
    register_height_item(&mut world, 0);
    let origin = Position::new(100, 100, 8);
    let climb_dest = Position::new(101, 100, 7);
    ensure_walkable_tile(&mut world.map, origin, TEST_SYNTHETIC_GROUND_WP);
    ensure_walkable_tile(&mut world.map, climb_dest, TEST_SYNTHETIC_GROUND_WP);
    place_n_height_items(&mut world, origin, 3);
    let cid = insert_player_on_tile(&mut world, "DeepClimber", origin);

    let landed = walk_east(&mut world, cid).expect("climb from z=8");
    assert_eq!(landed.z, 7);
    assert_eq!(landed, climb_dest);
}

/// G3: stepping down `z=7 → z=8` onto a 3-stack through a !BANK !UNPASS dest.
#[test]
fn step_down_from_z7_onto_stack() {
    let mut world = beat_driven_test_world();
    register_height_item(&mut world, 0);
    let origin = Position::new(100, 100, 7);
    let stack = Position::new(101, 100, 8);
    ensure_walkable_tile(&mut world.map, origin, TEST_SYNTHETIC_GROUND_WP);
    ensure_walkable_tile(&mut world.map, stack, TEST_SYNTHETIC_GROUND_WP);
    place_n_height_items(&mut world, stack, 3);
    let cid = insert_player_on_tile(&mut world, "Descender", origin);

    let landed = walk_east(&mut world, cid).expect("step down onto stack");
    assert_eq!(landed, stack);
    assert_eq!(landed.z, 8);
}

/// 772 regression: walking from ground onto a 2-stack (elev 16) must succeed.
/// Part A (7.4 step-up limit) is not in this profile.
#[test]
fn two_stack_walkable_from_ground_on_772() {
    let mut world = beat_driven_test_world();
    register_height_item(&mut world, 0);
    let origin = Position::new(100, 100, 7);
    let dest = Position::new(101, 100, 7);
    ensure_walkable_tile(&mut world.map, origin, TEST_SYNTHETIC_GROUND_WP);
    ensure_walkable_tile(&mut world.map, dest, TEST_SYNTHETIC_GROUND_WP);
    place_n_height_items(&mut world, dest, 2);
    let cid = insert_player_on_tile(&mut world, "Stepper", origin);

    let landed = walk_east(&mut world, cid).expect("2-stack is walkable on 772");
    assert_eq!(landed, dest);
    assert_eq!(landed.z, 7);
}

/// G4: walk-step `NotEnoughRoom` remaps to `NotPossible` ("Sorry, not possible.").
#[test]
fn blocked_walk_noroom_is_not_possible() {
    let mut world = beat_driven_test_world();
    let origin = Position::new(100, 100, 7);
    let dest = Position::new(101, 100, 7);
    ensure_walkable_tile(&mut world.map, origin, TEST_SYNTHETIC_GROUND_WP);
    ensure_walkable_tile(&mut world.map, dest, TEST_SYNTHETIC_GROUND_WP);
    if let Some(tile) = world.map.get_tile_mut(dest) {
        tile.body_mut().flags |= tilestate::BLOCKSOLID;
    }
    let cid = insert_player_on_tile(&mut world, "Blocked", origin);

    let rv = world.internal_move_creature_step(cid, Direction::East, Instant::now());
    assert_eq!(rv.err(), Some(ReturnValue::NotPossible));
    assert_eq!(
        ReturnValue::NotPossible.description(),
        "Sorry, not possible."
    );
}

/// G4: PZ-lock throws from `MovePossible` are not remapped.
#[test]
fn pz_locked_walk_keeps_player_is_pz_locked() {
    let mut world = beat_driven_test_world();
    world.round_nr = 100;
    let origin = Position::new(100, 100, 7);
    let dest = Position::new(101, 100, 7);
    ensure_walkable_tile(&mut world.map, origin, TEST_SYNTHETIC_GROUND_WP);
    ensure_walkable_tile(&mut world.map, dest, TEST_SYNTHETIC_GROUND_WP);
    if let Some(tile) = world.map.get_tile_mut(dest) {
        tile.body_mut().zone = tfs_rust_common::enums::ZoneType::Protection;
        tile.body_mut().flags |= tilestate::PROTECTIONZONE;
    }
    let cid = insert_player_on_tile(&mut world, "Locked", origin);
    if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
        p.earliest_protection_zone_round = 160;
    }

    let rv = world.internal_move_creature_step(cid, Direction::East, Instant::now());
    assert_eq!(rv.err(), Some(ReturnValue::PlayerIsPzLocked));
}

#[test]
fn remap_walk_step_err_only_noroom() {
    assert_eq!(
        remap_walk_step_err(ReturnValue::NotEnoughRoom),
        ReturnValue::NotPossible
    );
    assert_eq!(
        remap_walk_step_err(ReturnValue::PlayerIsPzLocked),
        ReturnValue::PlayerIsPzLocked
    );
    assert_eq!(
        remap_walk_step_err(ReturnValue::PlayerIsNotInvited),
        ReturnValue::PlayerIsNotInvited
    );
}

/// Climb helper is a no-op when the caller has not yet failed a flat step — covered
/// indirectly by `climb_not_taken_when_flat_move_succeeds`. This asserts JumpPossible
/// dest selection for a constructed climb-up.
#[test]
fn try_player_elevation_climb_up_uses_jump_possible() {
    let mut world = beat_driven_test_world();
    register_height_item(&mut world, 0);
    let origin = Position::new(100, 100, 7);
    let flat = Position::new(101, 100, 7);
    let climb_dest = Position::new(101, 100, 6);
    ensure_walkable_tile(&mut world.map, origin, TEST_SYNTHETIC_GROUND_WP);
    ensure_walkable_tile(&mut world.map, climb_dest, TEST_SYNTHETIC_GROUND_WP);
    place_n_height_items(&mut world, origin, 3);
    let cid = insert_player_on_tile(&mut world, "Helper", origin);

    let climbed = try_player_elevation_climb(&world, cid, origin, flat)
        .expect("no PZ/house throw")
        .expect("climb dest");
    assert_eq!(climbed, climb_dest);
}
