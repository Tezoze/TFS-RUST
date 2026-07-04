//! Game loop tick orchestration — 1098 `on_tick` and 772 beat advance.
//!
//! - `Game::checkCreatures` / subsystem polling — `game.cpp`.
//! - 772 `AdvanceGame` — `tibia-game-master/src/main.cc`.

use std::time::Instant;

use crate::game_world::GameWorld;

/// C++ `AdvanceGame` skips `MoveCreatures` when accumulated lag ≥ 1000 ms (`main.cc:445`).
const LAG_SKIP_MOVEMENT_MS: u64 = 1000;

impl GameWorld {
    /// One simulation tick (~50 ms target) — 1098 loop only.
    pub fn on_tick(&mut self, now: std::time::Instant) {
        if self.walk_wake_tx.is_none() && !self.beat_driven_loop {
            self.process_walk_deadlines();
        }
        self.process_walk_action_tasks();

        self.tick_counter = self.tick_counter.wrapping_add(1);

        self.check_creatures(now);

        let _ = self.decay.tick(self.tick_counter);
        self.run_other_subsystems(now, true);
    }

    /// Spawns, player pings, Lua GC — shared by 1098 `on_tick` and 772 other counter.
    pub(crate) fn run_other_subsystems(&mut self, now: Instant, lua_gc_every_five_ticks: bool) {
        self.poll_spawn_respawns(self.now_ms());
        if lua_gc_every_five_ticks {
            if self.tick_counter.is_multiple_of(5) {
                self.events.lua_gc_step();
            }
        } else {
            self.events.lua_gc_step();
        }
        // 1098 proactive `sendPing` uses wallclock `Instant` (`Player::sendPing` `player.cpp` ~902).
        // 772 `Player::sendPing` (`player.cpp:754`) is also wallclock-based (5000ms from `onThink`)
        // and runs alongside the round-based `ProcessConnections` idle ping — both eras use it.
        self.tick_player_pings(now);

        if self.beat_driven_loop {
            self.round_nr_772 = self.round_nr_772.saturating_add(1);
            let kick = self.process_connections_772();
            self.tick_ambiente_light_772();
            for conn_id in kick {
                self.pending_idle_kick_772.push(conn_id);
            }
        }
    }

    /// 772 `AdvanceGame` beat step — staggered subsystems + logical clock + ToDoQueue drain.
    /// C++ ref: `tibia-game-master/src/main.cc` `AdvanceGame`, `crmain.cc` `MoveCreatures`.
    pub fn advance_beat_772(&mut self, delay_ms: u64) {
        let fired = self.subsystem_counters_772.accumulate(delay_ms);

        if fired.creatures {
            self.process_creatures_772();
        }
        if fired.cron {
            let _ = self.decay.tick(self.server_ms);
        }
        if fired.skills {
            self.process_skills_772();
        }
        if fired.other {
            let now = Instant::now();
            self.run_other_subsystems(now, false);
        }

        // C++ `AdvanceGame` calls `MoveCreatures(Delay)` only when `Delay < 1000` (`main.cc:445-453`).
        // `MoveCreatures` itself always drains once invoked (`crmain.cc:1144`).
        if delay_ms < LAG_SKIP_MOVEMENT_MS {
            self.server_ms = self.server_ms.saturating_add(delay_ms);
            self.drain_todo_queue();
            self.lag_772 = false;
        } else {
            self.lag_772 = true;
            if self.round_nr_772 > 10 {
                tracing::error!(
                    delay_ms,
                    "772 beat advance skipped MoveCreatures due to lag (Delay >= 1000)"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_world::support::beat_driven_test_world;

    #[test]
    fn lag_guard_skips_move_creatures_at_1000ms() {
        let mut world = beat_driven_test_world();
        world.server_ms = 500;
        world.advance_beat_772(1000);
        assert_eq!(world.server_ms, 500, "server_ms must not advance under lag guard");
        assert!(world.lag_772);
    }

    #[test]
    fn subsystems_still_run_under_lag_guard() {
        let mut world = beat_driven_test_world();
        // Cross all subsystem thresholds in one coalesced step.
        world.advance_beat_772(2000);
        assert!(world.lag_772);
        assert_eq!(world.server_ms, 0);
        assert!(world.round_nr_772 > 0, "Other subsystem should have fired");
    }
}
