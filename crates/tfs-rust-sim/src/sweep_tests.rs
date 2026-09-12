//! Tier 2 sweep tests — synthetic `beat_driven_world`, no OTBM.

use crate::sweep::{SYNTHETIC_TEST_CENTER, SweepAxis, SweepPoint, run_point};
use crate::world::beat_driven_world;

#[test]
fn monster_sweep_records_beat_wall_and_path_searches() {
    let mut world = beat_driven_world();
    let result = run_point(
        &mut world,
        SweepAxis::Monsters,
        SweepPoint {
            n: 50,
            beats: 20,
            warmup: 0,
        },
        42,
        SYNTHETIC_TEST_CENTER,
    );
    assert_eq!(
        result.beat_wall_samples, 20,
        "measured beats must fill the wall histogram"
    );
    assert!(
        result.path_searches > 0,
        "chase load should search paths (path_searches=0)"
    );
}

#[test]
fn players_sweep_produces_outgoing_bytes() {
    let mut world = beat_driven_world();
    let result = run_point(
        &mut world,
        SweepAxis::Players,
        SweepPoint {
            n: 8,
            beats: 10,
            warmup: 0,
        },
        42,
        SYNTHETIC_TEST_CENTER,
    );
    assert!(
        result.outgoing_bytes_per_beat > 0,
        "player walks with conn mappings must enqueue outgoing bytes"
    );
}
