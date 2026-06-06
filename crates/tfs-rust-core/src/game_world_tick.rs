//! Game loop tick orchestration — 1098 `on_tick` and 772 beat advance.
//!
//! - `Game::checkCreatures` / subsystem polling — `game.cpp`.
//! - CipSoft `AdvanceGame` — `tibia-game-master/src/main.cc`.

use std::time::Instant;

use crate::game_world::GameWorld;

impl GameWorld {
    /// One simulation tick (~50 ms target) — 1098 loop only.
    pub fn on_tick(&mut self, now: std::time::Instant) {
        if self.walk_wake_tx.is_none() && !self.beat_driven_loop {
            self.process_walk_deadlines();
        }
        self.process_walk_action_tasks(now);

        self.tick_counter = self.tick_counter.wrapping_add(1);

        self.check_creatures(now);

        let _ = self.decay.tick(self.tick_counter);
        self.run_other_subsystems(now, true);
    }

    /// Spawns, player pings, Lua GC — shared by 1098 `on_tick` and 772 other counter.
    pub(crate) fn run_other_subsystems(&mut self, now: Instant, lua_gc_every_five_ticks: bool) {
        self.poll_spawn_respawns(now);
        if lua_gc_every_five_ticks {
            if self.tick_counter.is_multiple_of(5) {
                self.events.lua_gc_step();
            }
        } else {
            self.events.lua_gc_step();
        }
        self.tick_player_pings(now);
    }

    /// CipSoft `AdvanceGame` beat step — staggered subsystems + logical clock + ToDoQueue drain.
    /// C++ ref: `tibia-game-master/src/main.cc` `AdvanceGame`, `crmain.cc` `MoveCreatures`.
    pub fn advance_beat_772(&mut self, delay_ms: u64) {
        let fired = self.subsystem_counters_772.accumulate(delay_ms);

        if fired.creatures {
            self.process_creatures_772();
        }
        if fired.cron {
            tracing::trace!("772 cron subsystem tick — no cron engine yet");
        }
        if fired.skills {
            let _ = self.decay.tick(self.tick_counter);
        }
        if fired.other {
            let now = Instant::now();
            self.run_other_subsystems(now, false);
        }

        self.process_walk_action_tasks(Instant::now());
        self.tick_counter = self.tick_counter.saturating_add(delay_ms / 50);

        self.server_ms = self.server_ms.saturating_add(delay_ms);
        if delay_ms < 1000 {
            self.drain_todo_queue();
        }
    }
}
