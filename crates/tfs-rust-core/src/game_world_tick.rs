//! Game loop tick orchestration — unified beat advance.
//!
//! - 772 `AdvanceGame` — `tibia-game-master/src/main.cc`.
//! - 1098 observable behavior per `src/game.cpp` `Game::checkCreatures` (reproduced via
//!   profile knobs, not a separate loop).

use std::time::Instant;

use chrono::Timelike;

use crate::game_world::GameWorld;

/// C++ `AdvanceGame` skips `MoveCreatures` when accumulated lag ≥ 1000 ms (`main.cc:445`).
const LAG_SKIP_MOVEMENT_MS: u64 = 1000;

impl GameWorld {
    /// 772 Other arm — `main.cc:347–437`. Order: RoundNr++ → connections → homes → raids.
    pub(crate) fn run_other_subsystems(&mut self, delay_ms: u64) {
        self.round_nr = self.round_nr.saturating_add(1);
        let kick = self.process_connections();
        self.process_communication_control();
        self.poll_spawn_respawns(self.round_nr);
        self.process_monster_raids();
        self.tick_ambient_light();
        self.npc_tick_conversation_timeouts();
        if self.round_nr.is_multiple_of(10) {
            self.net_load_check();
        }
        self.tick_other_minute_jobs();
        // N1: Lua GC stays off the movement path; skip when this AdvanceGame is already lagging.
        if delay_ms < LAG_SKIP_MOVEMENT_MS {
            self.events.lua_gc_step();
        }
        for kick in kick {
            self.pending_idle_kick.push(kick);
        }
    }

    /// `GetRoundForNextMinute` — `time.cc:106–109`.
    fn get_round_for_next_minute(round_nr: u32) -> u32 {
        let local = chrono::Local::now();
        let secs_to_next_minute = 60u32.saturating_sub(local.timestamp() as u32 % 60);
        round_nr
            .saturating_add(secs_to_next_minute)
            .saturating_add(30)
    }

    /// Minute jobs on Other (`main.cc:375–436`) — `RefreshCylinders`, kill stats, netload.
    /// House `ProcessHouses` runs at boot and on save/reboot fire (`houses.cc:1943-1960`).
    fn tick_other_minute_jobs(&mut self) {
        if self.round_nr < self.next_minute_round {
            return;
        }
        // One `GetRealTime` sample for the whole minute arm (`main.cc:380-381`).
        let now = chrono::Local::now();
        let minute = now.minute();
        let unix = now.timestamp();
        // Corpus minute arm is `RefreshCylinders`, not `RefreshMap` (`main.cc:383` vs `:428`).
        let _ = self.refresh_cylinders();
        if minute.is_multiple_of(5) {
            self.log_online_player_list(unix);
        }
        if minute == 0 {
            let (recv, send) = self.net_load.summary();
            tracing::info!(target: "netload", recv, send, "network load");
        }
        // `WriteKillStatistics` at wall-clock minute 55 (`main.cc:393-394`).
        if minute == 55 {
            self.spawn_kill_statistics_flush();
        }
        // 5/3/1 warnings and the reboot compare only inside this arm (`main.cc:397-433`).
        self.tick_server_save(unix);
        self.next_minute_round = Self::get_round_for_next_minute(self.round_nr);
    }

    /// 772 `AdvanceGame` beat step — staggered subsystems + logical clock + ToDoQueue drain.
    /// C++ ref: `tibia-game-master/src/main.cc` `AdvanceGame`, `crmain.cc` `MoveCreatures`.
    pub fn advance_beat(&mut self, delay_ms: u64) {
        let wall_start = Instant::now();
        let fired = self.subsystem_counters.accumulate(delay_ms);

        let t0 = Instant::now();
        if fired.creatures {
            self.process_creatures();
        }
        let creatures_us = t0.elapsed().as_micros();

        let t0 = Instant::now();
        if fired.cron {
            let expired = self.decay.tick(self.decay_clock_now());
            self.obs.record_decay(
                expired.len(),
                self.decay.live_count(),
                self.decay.heap_len(),
            );
            if !expired.is_empty() {
                self.process_decay_expiry(&expired);
            }
        }
        let cron_us = t0.elapsed().as_micros();

        let t0 = Instant::now();
        if fired.skills {
            self.process_skills();
        }
        let skills_us = t0.elapsed().as_micros();

        let t0 = Instant::now();
        if fired.other {
            self.run_other_subsystems(delay_ms);
        }
        let other_us = t0.elapsed().as_micros();

        // C++ `AdvanceGame` calls `MoveCreatures(Delay)` only when `Delay < 1000` (`main.cc:445-453`).
        // `MoveCreatures` itself always drains once invoked (`crmain.cc:1142`).
        let todo_len_before = self.todo_queue.len();
        let t0 = Instant::now();
        let beat_ms = u64::from(self.mechanics.profile.beat_ms.max(1));
        // `Log("lag")` whenever this wake's delay exceeds one beat (`main.cc:440-442`).
        if delay_ms > beat_ms {
            tracing::info!(target: "lag", "delay {delay_ms} msec");
        }
        if delay_ms < LAG_SKIP_MOVEMENT_MS {
            self.move_creatures(delay_ms);
            self.lag = false;
        } else if !self.lag && self.round_nr > 10 {
            tracing::error!(
                delay_ms,
                todo_queue_len = todo_len_before,
                creatures = self.creatures.len(),
                "772 beat advance skipped MoveCreatures due to lag (Delay >= 1000)"
            );
            self.lag = true;
        } else {
            self.lag = true;
        }
        let todo_us = t0.elapsed().as_micros();
        let wall_ms = wall_start.elapsed().as_millis();

        self.obs.record_subsystems(
            creatures_us as u64,
            cron_us as u64,
            skills_us as u64,
            other_us as u64,
            todo_us as u64,
            fired.creatures,
            fired.cron,
            fired.skills,
            fired.other,
        );

        // Surface the hotspot when a beat (or coalesced burst) burns real time. `delay_ms` is
        // how far the wall clock already fell behind *before* this call; `wall_ms` is how long
        // *this* advance took (usually dominated by ToDo/IdleStimulus pathfinding).
        if wall_ms >= 100 || delay_ms >= LAG_SKIP_MOVEMENT_MS {
            tracing::debug!(
                delay_ms,
                wall_ms,
                creatures_us,
                cron_us,
                skills_us,
                other_us,
                todo_us,
                todo_queue_len = todo_len_before,
                fired_creatures = fired.creatures,
                fired_skills = fired.skills,
                fired_other = fired.other,
                decay_live = self.decay.live_count(),
                decay_heap = self.decay.heap_len(),
                obs_commands = self.obs.commands_processed_total,
                "772 beat advance timing"
            );
        }
    }

    /// Read-only OBS-1 window (beat histograms / subsystem µs).
    #[inline]
    pub fn obs(&self) -> &crate::obs::GameObs {
        &self.obs
    }

    /// Snapshot the current OBS-1 window and start a new one.
    ///
    /// Preserves `commands_processed_total` on the replacement window, matching
    /// `GameObs::reset_window` (cumulative since world creation).
    pub fn take_obs_window(&mut self) -> crate::obs::GameObs {
        let total = self.obs.commands_processed_total;
        let window = std::mem::take(&mut self.obs);
        self.obs.commands_processed_total = total;
        window
    }

    /// Logical `ServerMilliseconds` — C++ `time.cc` / `common.hh`.
    #[inline]
    pub fn server_ms(&self) -> u64 {
        self.server_ms
    }

    /// Next ToDo heap wakeup, if any — C++ `ToDoQueue.Entry->at(1).Key`.
    #[inline]
    pub fn next_todo_execution_ms(&self) -> Option<u64> {
        self.todo_queue.peek().map(|e| e.execution_time)
    }

    /// C++ `MoveCreatures` (`crmain.cc:1142`): advance `ServerMilliseconds` and drain due todos.
    ///
    /// No lag guard and no subsystem firing — those belong to [`Self::advance_beat`] / `AdvanceGame`.
    pub fn move_creatures(&mut self, delay_ms: u64) {
        self.server_ms = self.server_ms.saturating_add(delay_ms);
        self.drain_todo_queue();
    }
}

#[cfg(test)]
mod tests {
    use crate::test_world::support::beat_driven_test_world;

    #[test]
    fn lag_guard_skips_move_creatures_at_1000ms() {
        let mut world = beat_driven_test_world();
        world.server_ms = 500;
        world.advance_beat(1000);
        assert_eq!(
            world.server_ms(),
            500,
            "server_ms must not advance under lag guard"
        );
        assert!(world.lag);
    }

    #[test]
    fn lag_flag_stays_set_until_a_short_beat() {
        let mut world = beat_driven_test_world();
        world.round_nr = 11;
        world.advance_beat(1000);
        assert!(world.lag);
        world.advance_beat(1000);
        assert!(
            world.lag,
            "a second stalled beat stays inside the same episode"
        );
        world.advance_beat(50);
        assert!(!world.lag);
    }

    #[test]
    fn server_save_warning_waits_for_the_minute_arm() {
        let mut world = beat_driven_test_world();
        let now = chrono::Local::now().timestamp();
        world.server_save.set_next_save_unix(now + 60);
        world.next_minute_round = 10_000;
        world.advance_beat(2000);
        assert_eq!(
            world.game_state,
            crate::game_state::GameState::Normal,
            "poll stays off until RoundNr >= NextMinute"
        );
        world.next_minute_round = 0;
        world.advance_beat(2000);
        assert_eq!(
            world.game_state,
            crate::game_state::GameState::Closed,
            "the minute arm broadcasts the 5-minute close"
        );
    }

    #[test]
    fn subsystems_still_run_under_lag_guard() {
        let mut world = beat_driven_test_world();
        // Cross all subsystem thresholds in one coalesced step.
        world.advance_beat(2000);
        assert!(world.lag);
        assert_eq!(world.server_ms, 0);
        assert!(world.round_nr > 0, "Other subsystem should have fired");
    }

    /// DEC-3: 772 decay uses `RoundNr`, not movement `server_ms` — lag guard must not freeze expiry.
    #[test]
    fn lag_guard_does_not_freeze_decay_clock() {
        use crate::formulas::DecayClockModel;
        use crate::ids::ItemId;
        use slotmap::SlotMap;

        let mut world = beat_driven_test_world();
        assert_eq!(
            world.mechanics.profile.decay_clock,
            DecayClockModel::RoundNumber
        );

        let mut scratch: SlotMap<ItemId, ()> = SlotMap::with_key();
        let item_id = scratch.insert(());
        world.round_nr = 0;
        world.decay.schedule(item_id, 1, None);

        world.server_ms = 100;
        world.advance_beat(2000);

        assert_eq!(
            world.server_ms(),
            100,
            "movement clock must stay frozen under lag guard"
        );
        assert!(world.lag);
        assert!(
            world.decay_clock_now() >= 1,
            "round-based decay clock must advance while movement is paused"
        );
        let expired = world.decay.tick(world.decay_clock_now());
        assert!(
            expired.len() == 1,
            "scheduled decay must become due after round clock advances"
        );
    }

    #[test]
    fn other_does_not_send_5s_wallclock_ping() {
        use std::time::{Duration, Instant};

        use tfs_rust_common::{ConnId, Position};

        use crate::creature::CreatureKind;
        use crate::test_world::support::{ensure_walkable_tile, insert_player, test_player};

        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let player = insert_player(&mut world, test_player("Pinged", pos));
        let conn = ConnId(1);
        world.register_conn_mapping(conn, player);
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.last_ping_sent = Instant::now()
                .checked_sub(Duration::from_secs(30))
                .unwrap_or_else(Instant::now);
            p.last_command_round = 0;
        }
        world.round_nr = 0;
        world.pending_outgoing.clear();
        world.run_other_subsystems(200);
        assert_eq!(world.round_nr, 1);
        let has_keepalive_ping = world
            .pending_outgoing
            .get(&conn)
            .is_some_and(|q| q.iter().any(|b| matches!(b.first(), Some(0x1D | 0x1E))));
        assert!(
            !has_keepalive_ping,
            "772 Other must not emit a 5s wallclock ping (0x1D/0x1E); round-based ping is 30/60"
        );
    }

    #[test]
    fn spawn_poll_sees_round_nr_after_increment() {
        let mut world = beat_driven_test_world();
        world.round_nr = 0;
        world.run_other_subsystems(200);
        assert_eq!(world.round_nr, 1);
        assert_eq!(world.spawns.last_check, Some(1));
    }

    #[test]
    fn move_creatures_advances_clock_and_drains_due_todos() {
        use tfs_rust_common::Position;

        use crate::test_world::support::{ensure_walkable_tile, insert_monster};

        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let cid = insert_monster(&mut world, "Rat", pos, 200);
        world.schedule_creature_wakeup(cid, 50);
        world.move_creatures(50);
        assert_eq!(world.server_ms(), 50);
        assert!(
            world
                .next_todo_execution_ms()
                .is_none_or(|t| t > world.server_ms()),
            "due todos must be drained; remaining entries are strictly in the future"
        );
    }

    #[test]
    fn move_creatures_has_no_lag_guard() {
        let mut world = beat_driven_test_world();
        world.move_creatures(1000);
        assert_eq!(world.server_ms(), 1000);
        assert!(!world.lag);
    }

    #[test]
    fn take_obs_window_preserves_commands_processed_total() {
        let mut world = beat_driven_test_world();
        world.obs.record_commands_processed(7);
        world.advance_beat(50);
        assert!(world.obs().creatures_us.samples() >= 1);
        let snapshot = world.take_obs_window();
        assert_eq!(snapshot.commands_processed_total, 7);
        assert!(snapshot.creatures_us.samples() >= 1);
        assert_eq!(world.obs().commands_processed_total, 7);
        assert_eq!(world.obs().creatures_us.samples(), 0);
        assert_eq!(world.obs().beats, 0);
    }
}
