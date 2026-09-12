//! Headless scenario wall clock — caps `move_creatures` / `run_sim_tick`.
//!
//! C++ reference: `chase_kite_scenario.cc` wall-ms clamp. Production `GameWorld` never reads this.

use std::cell::RefCell;

#[derive(Debug, Default, Clone, Copy)]
struct HarnessScenarioClock {
    wall_ms: Option<u64>,
    segment_ms: Option<u64>,
}

thread_local! {
    static HARNESS_SCENARIO_CLOCK: RefCell<HarnessScenarioClock> =
        RefCell::new(HarnessScenarioClock::default());
}

/// Reset scenario clock — call from beat-driven world builders.
pub fn reset_harness_scenario_clock() {
    HARNESS_SCENARIO_CLOCK.with(|c| *c.borrow_mut() = HarnessScenarioClock::default());
}

fn with_harness_clock<R>(f: impl FnOnce(&HarnessScenarioClock) -> R) -> R {
    HARNESS_SCENARIO_CLOCK.with(|c| f(&c.borrow()))
}

fn with_harness_clock_mut<R>(f: impl FnOnce(&mut HarnessScenarioClock) -> R) -> R {
    HARNESS_SCENARIO_CLOCK.with(|c| f(&mut c.borrow_mut()))
}

pub fn set_harness_wall_ms(wall_ms: Option<u64>) {
    with_harness_clock_mut(|c| c.wall_ms = wall_ms);
}

pub fn set_harness_segment_ms(segment_ms: Option<u64>) {
    with_harness_clock_mut(|c| c.segment_ms = segment_ms);
}

pub fn harness_at_wall(server_ms: u64) -> bool {
    with_harness_clock(|c| c.wall_ms.is_some_and(|wall| server_ms >= wall))
}

pub fn harness_clamp_delay(server_ms: u64, delay_ms: u64) -> u64 {
    with_harness_clock(|c| {
        let Some(wall) = c.wall_ms else {
            return delay_ms;
        };
        wall.saturating_sub(server_ms).min(delay_ms)
    })
}
