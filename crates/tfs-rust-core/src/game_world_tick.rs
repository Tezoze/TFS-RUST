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
    /// 772 Other arm — `main.cc:347–437`. Order: RoundNr++ → connections → homes → raids.
    pub(crate) fn run_other_subsystems(&mut self, delay_ms: u64) {
        self.round_nr = self.round_nr.saturating_add(1);
        let kick = self.process_connections();
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

    /// Minute jobs on Other (`main.cc:375–436`) — house rent/auctions; not a Tokio cron.
    fn tick_other_minute_jobs(&mut self) {
        if self.round_nr < self.next_minute_round {
            return;
        }
        let now = chrono::Local::now()
            .timestamp()
            .clamp(0, i64::from(u32::MAX)) as u32;
        let period = self.house_rent_period_from_config();
        let grace = self.house_grace_secs_from_config();
        self.process_houses_online(now, period, grace);
        self.next_minute_round = Self::get_round_for_next_minute(self.round_nr);
    }

    /// `NetLoadCheck` / `EmergencyPing` (`main.cc:375–377`) — under lag, rewind command
    /// stamps 100 rounds and ping so idle timeouts still progress.
    fn net_load_check(&mut self) {
        if !self.lag {
            return;
        }
        let online: Vec<(tfs_rust_common::ConnId, crate::ids::CreatureId)> = self
            .conn_to_creature
            .iter()
            .map(|(&conn, &cid)| (conn, cid))
            .collect();
        for (conn_id, cid) in online {
            if let Some(crate::creature::CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                p.last_command_round = p.last_command_round.saturating_sub(100);
            }
            self.enqueue_outgoing(conn_id, tfs_rust_net::outgoing::send_ping().into_bytes());
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
}
