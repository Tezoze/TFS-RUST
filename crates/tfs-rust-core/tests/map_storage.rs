//! Map System Audit Phase 2 — storage invariants (findings #3, #7).
//!
//! - `register_creature_at` on a void tile must be a no-op that is observable
//!   (`tracing::error!` in release, `debug_assert!` panic in debug) — audit #3.
//! - After a batch of register/move/unregister calls through the `*_at` seam,
//!   `TileBody.creatures` and `Chunk.creatures` must agree — audit #7.

use slotmap::SlotMap;
use tfs_rust_common::Position;
use tfs_rust_common::ZoneType;
use tfs_rust_core::ids::CreatureId;
use tfs_rust_core::map::{Map, SparseGrid};
use tfs_rust_core::tile::{Tile, TileBody};

fn ground_tile() -> Tile {
    Tile::Normal(TileBody {
        ground: Some(100),

        ground_item: None,
        down_items: vec![],
        top_items: vec![],
        creatures: vec![],
        flags: 0,
        zone: ZoneType::Normal,
    })
}

fn flat_map(w: u16, h: u16) -> Map {
    let mut map = Map {
        width: w,
        height: h,
        grid: SparseGrid::new(),
        towns: std::collections::HashMap::new(),
        waypoints: std::collections::HashMap::new(),
        house_tiles: Vec::new(),
    };
    for x in 0..w {
        for y in 0..h {
            map.insert_tile(Position::new(x, y, 7), ground_tile());
        }
    }
    map
}

fn fresh_creature() -> (SlotMap<CreatureId, ()>, CreatureId) {
    let mut sm: SlotMap<CreatureId, ()> = SlotMap::with_key();
    let id = sm.insert(());
    (sm, id)
}

/// Audit #3 — registering on a void (unloaded) tile is a no-op. In debug builds the
/// `debug_assert!` fires (observable); in release the `tracing::error!` logs and the
/// creature is dropped from neither the tile stack nor the chunk spatial index.
#[test]
fn register_creature_at_on_void_tile_is_noop() {
    let mut map = flat_map(4, 4);
    let (_sm, id) = fresh_creature();
    // (200, 200) falls in chunk (3, 3) — no tile was inserted there, so the chunk itself is
    // absent. This is the true #3 "silent drop from both lists" scenario (a position in a
    // populated chunk but on a missing tile is the #7 desync case, caught separately by the
    // debug_assert and by `debug_assert_creature_lists_agree`).
    let void_pos = Position::new(200, 200, 7);

    // In debug the `debug_assert!` panics; catch it so we can verify no state changed.
    // In release there is no panic — the call simply runs and drops the creature.
    // Borrow `map` mutably inside the closure so ownership stays in the outer scope and
    // we can inspect state after the (possibly-caught) panic.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        map.register_creature_at(void_pos, id);
    }));

    #[cfg(debug_assertions)]
    assert!(
        result.is_err(),
        "debug_assert! must fire when registering a creature on a void tile"
    );
    #[cfg(not(debug_assertions))]
    assert!(
        result.is_ok(),
        "release build must not panic on void placement"
    );

    // No tile was created at the void position.
    assert!(
        map.get_tile(void_pos).is_none(),
        "void tile must not be created"
    );

    // The creature is not in any chunk spatial list near the void position (chunk absent).
    let mut spectators = Vec::new();
    map.grid
        .collect_spectators(200, 200, 7, 5, 5, &mut spectators);
    assert!(
        !spectators.contains(&id),
        "creature placed on a void tile in an absent chunk must not appear in the chunk spatial index"
    );
}

/// Audit #3 — unregistering on a void tile is a no-op (warn-logged, no panic in either
/// build). Verifies the dual lists are not corrupted by a spurious unregister.
#[test]
fn unregister_creature_at_on_void_tile_is_noop() {
    let mut map = flat_map(4, 4);
    let (_sm, id) = fresh_creature();
    let void_pos = Position::new(200, 200, 7); // absent chunk

    // Must not panic in either build (unregister uses `warn`, not `debug_assert!`).
    map.unregister_creature_at(void_pos, id);

    assert!(map.get_tile(void_pos).is_none());
    let mut spectators = Vec::new();
    map.grid
        .collect_spectators(200, 200, 7, 5, 5, &mut spectators);
    assert!(!spectators.contains(&id));
}

/// Audit #7 — after a batch of register / move / unregister calls routed through the
/// `*_at` seam, the dual `TileBody.creatures` and `Chunk.creatures` lists must agree.
#[test]
fn creature_lists_agree_after_batch_of_moves() {
    let mut map = flat_map(8, 8);
    let mut sm: SlotMap<CreatureId, ()> = SlotMap::with_key();
    let a = sm.insert(());
    let b = sm.insert(());
    let c = sm.insert(());

    let p0 = Position::new(1, 1, 7);
    let p1 = Position::new(2, 2, 7);
    let p2 = Position::new(6, 6, 7); // different 64×64 chunk boundary region (same chunk here)

    // Register all three.
    map.register_creature_at(p0, a);
    map.register_creature_at(p1, b);
    map.register_creature_at(p2, c);
    map.debug_assert_creature_lists_agree();

    // Move a → p1 (joins b), b → p2 (joins c).
    map.unregister_creature_at(p0, a);
    map.register_creature_at(p1, a);
    map.unregister_creature_at(p1, b);
    map.register_creature_at(p2, b);
    map.debug_assert_creature_lists_agree();

    // Verify tile stacks reflect the moves.
    let tile_p1 = map.get_tile(p1).expect("tile p1");
    assert!(tile_p1.body().creatures.contains(&a));
    assert!(!tile_p1.body().creatures.contains(&b));
    let tile_p2 = map.get_tile(p2).expect("tile p2");
    assert!(tile_p2.body().creatures.contains(&b));
    assert!(tile_p2.body().creatures.contains(&c));

    // Unregister one and confirm agreement.
    map.unregister_creature_at(p2, c);
    map.debug_assert_creature_lists_agree();

    let tile_p2 = map.get_tile(p2).expect("tile p2");
    assert!(!tile_p2.body().creatures.contains(&c));
}
