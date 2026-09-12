use super::*;

#[test]
fn is_fleeing_gate() {
    assert!(!is_fleeing(10, 5, false));
    assert!(is_fleeing(5, 5, false));
    assert!(!is_fleeing(5, 5, true));
}

#[test]
fn is_in_spawn_range_chebyshev_and_z() {
    let spawn = Position::new(100, 100, 7);
    assert!(is_in_spawn_range(Position::new(110, 110, 7), spawn, 50, 2));
    assert!(!is_in_spawn_range(Position::new(200, 100, 7), spawn, 50, 2));
    assert!(!is_in_spawn_range(
        Position::new(100, 100, 10),
        spawn,
        50,
        2
    ));
}

/// 772 `MonsterhomeInRange` — `crnonpl.cc:1515`. Axis box `|dx|<=R && |dy|<=R && |dz|<=2`.
#[test]
fn monsterhome_in_range_axis_box_and_z() {
    let home = Position::new(100, 100, 7);
    let r = 5;
    assert!(monsterhome_in_range(Position::new(105, 105, 9), home, r));
    assert!(!monsterhome_in_range(Position::new(106, 100, 7), home, r));
    assert!(!monsterhome_in_range(Position::new(100, 106, 7), home, r));
    assert!(!monsterhome_in_range(Position::new(100, 100, 10), home, r));
    assert!(monsterhome_in_range(Position::new(200, 200, 15), home, 0));
    assert!(monsterhome_in_range(Position::new(200, 200, 15), home, -1));
}

/// Finding 17/17b — an ATTACKING monster follows its target beyond the home radius (leash
/// skipped), while a roaming (Idle) monster is bounded by its per-home `home_radius`.
#[test]
fn chase_leash_skipped_when_attacking_bounded_when_roaming() {
    use crate::creature::{MonsterAiConfig, MonsterState};
    use crate::test_support::{
        beat_driven_world, ensure_walkable_tile, insert_monster_with_config,
    };

    let mut world = beat_driven_world();
    // Global despawn radius is large (50); the per-home radius is small (3).
    world.monster_world_config.despawn_radius = 50;

    let spawn = Position::new(100, 100, 7);
    let far = Position::new(110, 100, 7); // chebyshev 10: > home_radius 3, < despawn 50
    ensure_walkable_tile(&mut world.map, spawn, 1);
    ensure_walkable_tile(&mut world.map, far, 1);

    let monster =
        insert_monster_with_config(&mut world, "Rat", spawn, 200, MonsterAiConfig::default());
    if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
        m.spawn_position = spawn;
        m.home_radius = 3;
        m.state = MonsterState::Idle;
    }
    assert!(
        !world.monster_can_occupy_chase_tile(monster, far),
        "roaming monster must stay within its home radius"
    );

    if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
        m.state = MonsterState::Attacking;
    }
    assert!(
        world.monster_can_occupy_chase_tile(monster, far),
        "ATTACKING monster must chase past the home radius"
    );
}

/// Finding 17b — with no per-home radius (`home_radius == 0`) the roam leash falls back to the
/// global despawn radius (no behavior change for synthetic/test monsters).
#[test]
fn roam_leash_falls_back_to_despawn_radius_when_home_unset() {
    use crate::creature::{MonsterAiConfig, MonsterState};
    use crate::test_support::{
        beat_driven_world, ensure_walkable_tile, insert_monster_with_config,
    };

    let mut world = beat_driven_world();
    world.monster_world_config.despawn_radius = 50;

    let spawn = Position::new(100, 100, 7);
    let near = Position::new(110, 100, 7); // cheb 10 ≤ despawn 50
    ensure_walkable_tile(&mut world.map, spawn, 1);
    ensure_walkable_tile(&mut world.map, near, 1);

    let monster =
        insert_monster_with_config(&mut world, "Rat", spawn, 200, MonsterAiConfig::default());
    if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
        m.spawn_position = spawn;
        m.home_radius = 0;
        m.state = MonsterState::Idle;
    }
    assert!(
        world.monster_can_occupy_chase_tile(monster, near),
        "unset home_radius roams within the global despawn radius"
    );
}

#[test]
fn is_within_walk_to_spawn_range_axis_box() {
    let spawn = Position::new(100, 100, 7);
    assert!(is_within_walk_to_spawn_range(
        Position::new(110, 110, 7),
        spawn,
        15
    ));
    assert!(!is_within_walk_to_spawn_range(
        Position::new(120, 100, 7),
        spawn,
        15
    ));
    assert!(is_within_walk_to_spawn_range(
        Position::new(100, 100, 7),
        spawn,
        15
    ));
}

#[test]
fn compute_look_faces_target() {
    let from = Position::new(10, 10, 7);
    assert_eq!(
        compute_look_toward_target(from, Position::new(12, 10, 7), Direction::North),
        Direction::East
    );
    assert_eq!(
        compute_look_toward_target(from, Position::new(10, 8, 7), Direction::East),
        Direction::North
    );
}
