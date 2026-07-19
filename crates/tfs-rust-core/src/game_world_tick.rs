//! Game loop tick orchestration — unified beat advance.
//!
//! - 772 `AdvanceGame` — `tibia-game-master/src/main.cc`.
//! - 1098 observable behavior per `src/game.cpp` `Game::checkCreatures` (reproduced via
//!   profile knobs, not a separate loop).

use std::time::Instant;

use crate::game_world::GameWorld;

/// C++ `AdvanceGame` skips `MoveCreatures` when accumulated lag ≥ 1000 ms (`main.cc:445`).
const LAG_SKIP_MOVEMENT_MS: u64 = 1000;

impl GameWorld {
    /// Spawns, player pings, Lua GC — shared by `advance_beat` other counter.
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

        // Phase 6: `beat_driven_loop` collapsed — both eras run the 772 round-based subsystems.
        self.round_nr = self.round_nr.saturating_add(1);
        let kick = self.process_connections();
        self.tick_ambient_light();
        for conn_id in kick {
            self.pending_idle_kick.push(conn_id);
        }
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
            let now = Instant::now();
            self.run_other_subsystems(now, false);
        }
        let other_us = t0.elapsed().as_micros();

        // C++ `AdvanceGame` calls `MoveCreatures(Delay)` only when `Delay < 1000` (`main.cc:445-453`).
        // `MoveCreatures` itself always drains once invoked (`crmain.cc:1144`).
        let todo_len_before = self.todo_queue.len();
        let t0 = Instant::now();
        if delay_ms < LAG_SKIP_MOVEMENT_MS {
            self.server_ms = self.server_ms.saturating_add(delay_ms);
            self.drain_todo_queue();
            self.lag = false;
        } else {
            self.lag = true;
            if self.round_nr > 10 {
                tracing::error!(
                    delay_ms,
                    todo_queue_len = todo_len_before,
                    creatures = self.creatures.len(),
                    "772 beat advance skipped MoveCreatures due to lag (Delay >= 1000)"
                );
            }
        }
        let todo_us = t0.elapsed().as_micros();
        let wall_ms = wall_start.elapsed().as_millis();

        // Surface the hotspot when a beat (or coalesced burst) burns real time. `delay_ms` is
        // how far the wall clock already fell behind *before* this call; `wall_ms` is how long
        // *this* advance took (usually dominated by ToDo/IdleStimulus pathfinding).
        if wall_ms >= 100 || delay_ms >= LAG_SKIP_MOVEMENT_MS {
            tracing::warn!(
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
                obs_commands = self.obs_commands_processed,
                "772 beat advance timing"
            );
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
        world.advance_beat(1000);
        assert_eq!(
            world.server_ms, 500,
            "server_ms must not advance under lag guard"
        );
        assert!(world.lag);
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
            world.server_ms, 100,
            "movement clock must stay frozen under lag guard"
        );
        assert!(world.lag);
        assert!(
            world.decay_clock_now() >= 1,
            "round-based decay clock must advance while movement is paused"
        );
        let expired = world.decay.tick(world.decay_clock_now());
        assert_eq!(
            expired.len(),
            1,
            "scheduled decay must become due after round clock advances"
        );
    }
}
