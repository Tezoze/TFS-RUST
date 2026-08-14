//! Aggregated game-thread observability (OBS-1 / Phase 0).
//!
//! Fixed-bucket histograms + window counters; periodic `tracing` summary at
//! target `tfs_obs`. No per-action INFO in hot paths.
//!
//! Audit: `docs/GAME_LOOP_DECAY_IDLE_TODO_PERFORMANCE_AUDIT.md` § Observability.

use std::time::{Duration, Instant};

/// Default window between aggregated summary emits.
pub const OBS_SUMMARY_INTERVAL: Duration = Duration::from_secs(10);

/// Number of geometric buckets (plus overflow).
const HIST_BUCKETS: usize = 32;

/// Power-of-two / geometric histogram — game-thread only, no locks.
///
/// Bucket `i` covers `[2^(i-1), 2^i)` for `i >= 1`, with bucket 0 = `[0, 1)`.
/// Values at or above `2^31` land in the last bucket.
#[derive(Debug, Clone, Default)]
pub struct FixedHistogram {
    counts: [u64; HIST_BUCKETS],
    samples: u64,
    sum: u64,
    max: u64,
}

impl FixedHistogram {
    #[inline]
    pub fn record(&mut self, value: u64) {
        let idx = Self::bucket_index(value);
        self.counts[idx] = self.counts[idx].saturating_add(1);
        self.samples = self.samples.saturating_add(1);
        self.sum = self.sum.saturating_add(value);
        if value > self.max {
            self.max = value;
        }
    }

    #[inline]
    fn bucket_index(value: u64) -> usize {
        if value == 0 {
            return 0;
        }
        // floor(log2(value)) + 1, capped
        let bit = 63usize.saturating_sub(value.leading_zeros() as usize);
        (bit + 1).min(HIST_BUCKETS - 1)
    }

    /// Approximate percentile via cumulative bucket counts (upper edge of bucket).
    pub fn percentile(&self, p: f64) -> u64 {
        if self.samples == 0 {
            return 0;
        }
        let p = p.clamp(0.0, 100.0);
        let target = ((p / 100.0) * self.samples as f64).ceil().max(1.0) as u64;
        let mut cum = 0u64;
        for (i, &c) in self.counts.iter().enumerate() {
            cum = cum.saturating_add(c);
            if cum >= target {
                return Self::bucket_upper(i);
            }
        }
        self.max
    }

    #[inline]
    fn bucket_upper(idx: usize) -> u64 {
        if idx == 0 {
            0
        } else if idx >= HIST_BUCKETS - 1 {
            u64::from(u32::MAX)
        } else {
            1u64 << idx
        }
    }

    #[cfg(test)]
    pub fn samples(&self) -> u64 {
        self.samples
    }

    #[cfg(test)]
    pub fn max(&self) -> u64 {
        self.max
    }

    pub fn mean(&self) -> u64 {
        self.sum.checked_div(self.samples).unwrap_or(0)
    }

    #[cfg(test)]
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// One observation window for the game loop (OBS-1).
#[derive(Debug)]
pub struct GameObs {
    window_started: Instant,
    /// Cumulative game-lane commands since world creation (thin OBS-1 compat).
    pub commands_processed_total: u64,

    // --- loop turn ---
    pub commands_before_beat: FixedHistogram,
    pub command_queue_depth_max: u64,
    pub oldest_command_age_ms: FixedHistogram,
    pub beat_lateness_ms: FixedHistogram,
    pub beat_wall_ms: FixedHistogram,
    pub coalesced_beats: u64,
    pub beats: u64,
    pub login_load_us: FixedHistogram,
    pub login_loads: u64,
    pub concurrent_logins_max: u64,

    // --- subsystems (µs) ---
    pub creatures_us: FixedHistogram,
    pub cron_us: FixedHistogram,
    pub skills_us: FixedHistogram,
    pub other_us: FixedHistogram,
    pub todo_us: FixedHistogram,
    pub creatures_fired: u64,
    pub cron_fired: u64,
    pub skills_fired: u64,
    pub other_fired: u64,

    // --- ToDo ---
    pub todo_heap_max: u64,
    pub todo_popped: u64,
    pub todo_executed: u64,
    pub todo_stale: u64,
    pub todo_lateness_ms: FixedHistogram,

    // --- decay ---
    pub decay_due: u64,
    pub decay_live_max: u64,
    pub decay_heap_max: u64,

    // --- path / idle ---
    pub path_searches: u64,
    pub path_failures: u64,
    pub path_expanded: u64,
    pub path_us: FixedHistogram,
    pub idle_passes: u64,
    pub idle_target_candidates: u64,

    // --- output ---
    pub output_queued_bytes_max: u64,
    pub output_full: u64,
    pub output_slow_shed: u64,
}

impl Default for GameObs {
    fn default() -> Self {
        Self::new()
    }
}

impl GameObs {
    pub fn new() -> Self {
        Self {
            window_started: Instant::now(),
            commands_processed_total: 0,
            commands_before_beat: FixedHistogram::default(),
            command_queue_depth_max: 0,
            oldest_command_age_ms: FixedHistogram::default(),
            beat_lateness_ms: FixedHistogram::default(),
            beat_wall_ms: FixedHistogram::default(),
            coalesced_beats: 0,
            beats: 0,
            login_load_us: FixedHistogram::default(),
            login_loads: 0,
            concurrent_logins_max: 0,
            creatures_us: FixedHistogram::default(),
            cron_us: FixedHistogram::default(),
            skills_us: FixedHistogram::default(),
            other_us: FixedHistogram::default(),
            todo_us: FixedHistogram::default(),
            creatures_fired: 0,
            cron_fired: 0,
            skills_fired: 0,
            other_fired: 0,
            todo_heap_max: 0,
            todo_popped: 0,
            todo_executed: 0,
            todo_stale: 0,
            todo_lateness_ms: FixedHistogram::default(),
            decay_due: 0,
            decay_live_max: 0,
            decay_heap_max: 0,
            path_searches: 0,
            path_failures: 0,
            path_expanded: 0,
            path_us: FixedHistogram::default(),
            idle_passes: 0,
            idle_target_candidates: 0,
            output_queued_bytes_max: 0,
            output_full: 0,
            output_slow_shed: 0,
        }
    }

    #[inline]
    pub fn record_commands_processed(&mut self, count: usize) {
        self.commands_processed_total = self.commands_processed_total.saturating_add(count as u64);
        if count > 0 {
            self.commands_before_beat.record(count as u64);
        }
    }

    #[inline]
    pub fn note_command_depth(&mut self, depth: usize) {
        let d = depth as u64;
        if d > self.command_queue_depth_max {
            self.command_queue_depth_max = d;
        }
    }

    #[inline]
    pub fn record_command_age_ms(&mut self, age_ms: u64) {
        self.oldest_command_age_ms.record(age_ms);
    }

    #[inline]
    pub fn record_beat(&mut self, coalesced: u64, lateness_ms: u64, wall_ms: u64) {
        self.beats = self.beats.saturating_add(1);
        self.coalesced_beats = self.coalesced_beats.saturating_add(coalesced);
        self.beat_lateness_ms.record(lateness_ms);
        self.beat_wall_ms.record(wall_ms);
    }

    #[inline]
    pub fn record_login_load(&mut self, latency_us: u64, concurrent: usize) {
        self.login_loads = self.login_loads.saturating_add(1);
        self.login_load_us.record(latency_us);
        let c = concurrent as u64;
        if c > self.concurrent_logins_max {
            self.concurrent_logins_max = c;
        }
    }

    #[inline]
    pub fn note_concurrent_logins(&mut self, concurrent: usize) {
        let c = concurrent as u64;
        if c > self.concurrent_logins_max {
            self.concurrent_logins_max = c;
        }
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn record_subsystems(
        &mut self,
        creatures_us: u64,
        cron_us: u64,
        skills_us: u64,
        other_us: u64,
        todo_us: u64,
        fired_creatures: bool,
        fired_cron: bool,
        fired_skills: bool,
        fired_other: bool,
    ) {
        self.creatures_us.record(creatures_us);
        self.cron_us.record(cron_us);
        self.skills_us.record(skills_us);
        self.other_us.record(other_us);
        self.todo_us.record(todo_us);
        if fired_creatures {
            self.creatures_fired = self.creatures_fired.saturating_add(1);
        }
        if fired_cron {
            self.cron_fired = self.cron_fired.saturating_add(1);
        }
        if fired_skills {
            self.skills_fired = self.skills_fired.saturating_add(1);
        }
        if fired_other {
            self.other_fired = self.other_fired.saturating_add(1);
        }
    }

    #[inline]
    pub fn record_todo_drain(
        &mut self,
        heap_before: usize,
        popped: u64,
        executed: u64,
        stale: u64,
    ) {
        let h = heap_before as u64;
        if h > self.todo_heap_max {
            self.todo_heap_max = h;
        }
        self.todo_popped = self.todo_popped.saturating_add(popped);
        self.todo_executed = self.todo_executed.saturating_add(executed);
        self.todo_stale = self.todo_stale.saturating_add(stale);
    }

    #[inline]
    pub fn record_todo_lateness_ms(&mut self, lateness_ms: u64) {
        self.todo_lateness_ms.record(lateness_ms);
    }

    #[inline]
    pub fn record_decay(&mut self, due: usize, live: usize, heap: usize) {
        self.decay_due = self.decay_due.saturating_add(due as u64);
        let live = live as u64;
        let heap = heap as u64;
        if live > self.decay_live_max {
            self.decay_live_max = live;
        }
        if heap > self.decay_heap_max {
            self.decay_heap_max = heap;
        }
    }

    #[inline]
    pub fn record_path_search(&mut self, us: u64, expanded: u64, ok: bool) {
        self.path_searches = self.path_searches.saturating_add(1);
        self.path_expanded = self.path_expanded.saturating_add(expanded);
        self.path_us.record(us);
        if !ok {
            self.path_failures = self.path_failures.saturating_add(1);
        }
    }

    #[inline]
    pub fn record_idle_pass(&mut self) {
        self.idle_passes = self.idle_passes.saturating_add(1);
    }

    #[inline]
    pub fn record_idle_candidates(&mut self, candidates: usize) {
        self.idle_target_candidates = self
            .idle_target_candidates
            .saturating_add(candidates as u64);
    }

    #[inline]
    pub fn note_output_queued_bytes(&mut self, bytes: usize) {
        let b = bytes as u64;
        if b > self.output_queued_bytes_max {
            self.output_queued_bytes_max = b;
        }
    }

    #[inline]
    pub fn record_output_full(&mut self) {
        self.output_full = self.output_full.saturating_add(1);
    }

    #[inline]
    pub fn record_output_slow_shed(&mut self) {
        self.output_slow_shed = self.output_slow_shed.saturating_add(1);
    }

    /// Emit aggregated summary when the window elapsed; resets window counters.
    pub fn maybe_emit(&mut self, now: Instant) {
        if now.duration_since(self.window_started) < OBS_SUMMARY_INTERVAL {
            return;
        }
        self.emit_summary();
        self.reset_window(now);
    }

    /// Force-emit (tests / shutdown).
    pub fn emit_summary(&self) {
        tracing::info!(
            target: "tfs_obs",
            beats = self.beats,
            coalesced_beats = self.coalesced_beats,
            cmd_before_beat_p50 = self.commands_before_beat.percentile(50.0),
            cmd_before_beat_p95 = self.commands_before_beat.percentile(95.0),
            cmd_before_beat_p99 = self.commands_before_beat.percentile(99.0),
            cmd_queue_depth_max = self.command_queue_depth_max,
            cmd_age_ms_p50 = self.oldest_command_age_ms.percentile(50.0),
            cmd_age_ms_p95 = self.oldest_command_age_ms.percentile(95.0),
            cmd_age_ms_p99 = self.oldest_command_age_ms.percentile(99.0),
            beat_lateness_ms_p50 = self.beat_lateness_ms.percentile(50.0),
            beat_lateness_ms_p95 = self.beat_lateness_ms.percentile(95.0),
            beat_lateness_ms_p99 = self.beat_lateness_ms.percentile(99.0),
            beat_wall_ms_p50 = self.beat_wall_ms.percentile(50.0),
            beat_wall_ms_p95 = self.beat_wall_ms.percentile(95.0),
            beat_wall_ms_p99 = self.beat_wall_ms.percentile(99.0),
            beat_wall_ms_mean = self.beat_wall_ms.mean(),
            login_loads = self.login_loads,
            login_us_p50 = self.login_load_us.percentile(50.0),
            login_us_p95 = self.login_load_us.percentile(95.0),
            login_us_p99 = self.login_load_us.percentile(99.0),
            concurrent_logins_max = self.concurrent_logins_max,
            creatures_us_p50 = self.creatures_us.percentile(50.0),
            creatures_us_p95 = self.creatures_us.percentile(95.0),
            creatures_us_p99 = self.creatures_us.percentile(99.0),
            cron_us_p50 = self.cron_us.percentile(50.0),
            cron_us_p95 = self.cron_us.percentile(95.0),
            cron_us_p99 = self.cron_us.percentile(99.0),
            skills_us_p50 = self.skills_us.percentile(50.0),
            skills_us_p95 = self.skills_us.percentile(95.0),
            skills_us_p99 = self.skills_us.percentile(99.0),
            other_us_p50 = self.other_us.percentile(50.0),
            other_us_p95 = self.other_us.percentile(95.0),
            other_us_p99 = self.other_us.percentile(99.0),
            todo_us_p50 = self.todo_us.percentile(50.0),
            todo_us_p95 = self.todo_us.percentile(95.0),
            todo_us_p99 = self.todo_us.percentile(99.0),
            creatures_fired = self.creatures_fired,
            cron_fired = self.cron_fired,
            skills_fired = self.skills_fired,
            other_fired = self.other_fired,
            todo_heap_max = self.todo_heap_max,
            todo_popped = self.todo_popped,
            todo_executed = self.todo_executed,
            todo_stale = self.todo_stale,
            todo_lateness_ms_p50 = self.todo_lateness_ms.percentile(50.0),
            todo_lateness_ms_p95 = self.todo_lateness_ms.percentile(95.0),
            todo_lateness_ms_p99 = self.todo_lateness_ms.percentile(99.0),
            decay_due = self.decay_due,
            decay_live_max = self.decay_live_max,
            decay_heap_max = self.decay_heap_max,
            path_searches = self.path_searches,
            path_failures = self.path_failures,
            path_expanded = self.path_expanded,
            path_us_p50 = self.path_us.percentile(50.0),
            path_us_p95 = self.path_us.percentile(95.0),
            path_us_p99 = self.path_us.percentile(99.0),
            idle_passes = self.idle_passes,
            idle_target_candidates = self.idle_target_candidates,
            output_queued_bytes_max = self.output_queued_bytes_max,
            output_full = self.output_full,
            output_slow_shed = self.output_slow_shed,
            commands_processed_total = self.commands_processed_total,
            "game_obs_summary"
        );
    }

    fn reset_window(&mut self, now: Instant) {
        let total = self.commands_processed_total;
        *self = Self::new();
        self.window_started = now;
        self.commands_processed_total = total;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histogram_bucket_edges() {
        let mut h = FixedHistogram::default();
        h.record(0);
        h.record(1);
        h.record(2);
        h.record(3);
        h.record(4);
        assert_eq!(h.samples(), 5);
        assert_eq!(h.percentile(0.0), 0);
        // Five samples: p50 is the 3rd → bucket covering 2–3 → upper edge ≤ 4
        let p50 = h.percentile(50.0);
        assert!(p50 <= 4, "p50={p50}");
        assert_eq!(h.max(), 4);
        assert!(h.percentile(100.0) >= 4);
    }

    #[test]
    fn histogram_percentile_uniform() {
        let mut h = FixedHistogram::default();
        for v in 1u64..=100 {
            h.record(v);
        }
        let p50 = h.percentile(50.0);
        let p95 = h.percentile(95.0);
        let p99 = h.percentile(99.0);
        assert!(p50 <= p95, "p50={p50} p95={p95}");
        assert!(p95 <= p99, "p95={p95} p99={p99}");
        assert!((32..=128).contains(&p50), "p50≈50..64 range, got {p50}");
        assert!(p95 >= 64, "p95={p95}");
        assert!(p99 >= 64, "p99={p99}");
    }

    #[test]
    fn empty_histogram_percentiles_are_zero() {
        let h = FixedHistogram::default();
        assert_eq!(h.percentile(50.0), 0);
        assert_eq!(h.percentile(99.0), 0);
        assert_eq!(h.mean(), 0);
    }

    #[test]
    fn game_obs_records_todo_drain() {
        let mut obs = GameObs::new();
        obs.record_todo_drain(10, 5, 3, 2);
        obs.record_todo_lateness_ms(40);
        assert_eq!(obs.todo_heap_max, 10);
        assert_eq!(obs.todo_popped, 5);
        assert_eq!(obs.todo_executed, 3);
        assert_eq!(obs.todo_stale, 2);
        assert_eq!(obs.todo_lateness_ms.samples(), 1);
    }

    #[test]
    fn drain_todo_queue_updates_obs_counters() {
        use crate::creature::MonsterAiConfig;
        use crate::test_world::support::{
            beat_driven_test_world, insert_monster_with_config, insert_player, test_player,
        };
        use tfs_rust_common::Position;

        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        let _player = insert_player(&mut world, test_player("Hero", pos));
        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            Position::new(102, 100, 7),
            100,
            MonsterAiConfig::default(),
        );

        // Arm a due wakeup then a stale heap entry (re-arm to a future time without
        // removing the old heap key — reference insert-and-recheck model).
        world.server_ms = 1000;
        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().next_wakeup = Some(500);
        }
        world.todo_queue.insert(500, monster);
        // Stale: current wakeup moved to the future, but old key remains.
        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().next_wakeup = Some(2000);
        }
        world.todo_queue.insert(2000, monster);

        world.drain_todo_queue();
        assert!(world.obs.todo_popped >= 1);
        assert_eq!(
            world.obs.todo_stale, 1,
            "due key with future NextWakeup must count as stale"
        );
        // Future key 2000 must remain on the heap.
        assert!(!world.todo_queue.is_empty());
    }

    /// Audit #10: randomized schedule/clear/reschedule — stale pops skip; live wakeups execute.
    #[test]
    fn randomized_todo_schedule_clear_reschedule_stale_accounting() {
        use crate::creature::MonsterAiConfig;
        use crate::test_world::support::{
            beat_driven_test_world, ensure_walkable_tile, insert_monster_with_config,
        };
        use tfs_rust_common::Position;

        let mut world = beat_driven_test_world();
        let pos = Position::new(150, 150, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let monster =
            insert_monster_with_config(&mut world, "Rat", pos, 100, MonsterAiConfig::default());

        // Deterministic LCG — avoid flaky wall-clock RNG in CI.
        let mut state = 0xC0FFEE_u64;
        let mut next_u64 = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            state
        };

        let mut expected_stale = 0u64;
        world.server_ms = 10_000;
        for _ in 0..40 {
            let t1 = world.server_ms.saturating_sub(100 + (next_u64() % 50));
            world.schedule_creature_wakeup(monster, t1);
            // Clear without removing heap keys (reference insert-and-recheck).
            if next_u64() % 3 == 0 {
                world.stop_event_walk(monster);
                // Old due key is now stale when drained.
                expected_stale = expected_stale.saturating_add(1);
            } else {
                let t2 = world.server_ms.saturating_add(500 + (next_u64() % 200));
                // Reschedule into the future — previous due key becomes stale on drain.
                if let Some(k) = world.creatures.get_mut(monster) {
                    let prev = k.base().next_wakeup;
                    k.base_mut().next_wakeup = Some(t2);
                    if prev.is_some_and(|p| p <= world.server_ms) {
                        expected_stale = expected_stale.saturating_add(1);
                    }
                }
                world.todo_queue.insert(t2, monster);
            }
        }

        let before_stale = world.obs.todo_stale;
        let before_popped = world.obs.todo_popped;
        world.drain_todo_queue();
        let stale = world.obs.todo_stale.saturating_sub(before_stale);
        let popped = world.obs.todo_popped.saturating_sub(before_popped);
        assert!(popped >= 1, "drain must pop at least one heap entry");
        assert!(
            stale >= 1,
            "randomized clear/reschedule must produce stale skips (stale={stale})"
        );
        // Live wakeup, if any, must be the creature's current next_wakeup only.
        if let Some(wake) = world
            .creatures
            .get(monster)
            .and_then(|k| k.base().next_wakeup)
        {
            assert!(
                wake > world.server_ms
                    || world.todo_queue.is_empty()
                    || wake == world.server_ms + 1,
                "surviving wakeup must be current NextWakeup ({wake})"
            );
        }
        let _ = expected_stale;
    }
}
