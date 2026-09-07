//! Corpus `NetLoadCheck` / `EmergencyPing` — recv-bandwidth lag, not beat-stall.
//!
//! C++ reference: `communication.cc:141-229` `NetLoadCheck`, `main.cc:375-377` Other arm.
//! Free-account admission delay is out of scope (no login queue exists).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::connections::DeadConnState;
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;

const HISTORY_LEN: usize = 360;
const WARMUP_ROUNDS: u32 = 3600;
const MIN_PLAYERS: usize = 50;
const LAG_WINDOW_ROUNDS: u32 = 30;
const EMERGENCY_PING_IDLE: u32 = 80;
const EMERGENCY_REWIND: u32 = 100;

/// Recv-byte ring + lag window. Game thread only loads atomics; I/O increments Relaxed.
pub struct NetLoad {
    history: [i32; HISTORY_LEN],
    ptr: usize,
    total_load: i32,
    last_recv: u64,
    lag_end: u32,
    recv_bytes: Arc<AtomicU64>,
    send_bytes: Arc<AtomicU64>,
}

impl Default for NetLoad {
    fn default() -> Self {
        Self::new(Arc::new(AtomicU64::new(0)), Arc::new(AtomicU64::new(0)))
    }
}

impl NetLoad {
    pub fn new(recv_bytes: Arc<AtomicU64>, send_bytes: Arc<AtomicU64>) -> Self {
        Self {
            history: [0; HISTORY_LEN],
            ptr: 0,
            total_load: 0,
            last_recv: 0,
            lag_end: 0,
            recv_bytes,
            send_bytes,
        }
    }

    /// True while `round_nr` is inside the 30-round `LagDetected` window.
    pub fn lag_detected(&self, round_nr: u32) -> bool {
        self.lag_end > 0 && round_nr <= self.lag_end
    }

    /// Sample recv Δ, update the 360-slot ring. Returns `true` → run EmergencyPing.
    pub fn check(&mut self, round_nr: u32, players_online: usize) -> bool {
        let recv = self.recv_bytes.load(Ordering::Relaxed);
        if recv < self.last_recv {
            self.last_recv = recv;
            return false;
        }
        let delta = (recv - self.last_recv).min(i32::MAX as u64) as i32;
        self.last_recv = recv;

        let old = self.history[self.ptr];
        let per_player = if players_online == 0 {
            0
        } else {
            delta / players_online as i32
        };
        self.history[self.ptr] = per_player;
        self.total_load = self.total_load.saturating_sub(old).saturating_add(per_player);
        self.ptr = (self.ptr + 1) % HISTORY_LEN;

        if round_nr < WARMUP_ROUNDS || players_online < MIN_PLAYERS {
            return false;
        }
        let avg = self.total_load / HISTORY_LEN as i32;
        if per_player < avg / 2 {
            self.lag_end = round_nr.saturating_add(LAG_WINDOW_ROUNDS);
            tracing::warn!("Lag erkannt");
            return true;
        }
        false
    }

    /// Hourly `NetLoadSummary`: log totals then zero (`communication.cc:155-162`).
    pub fn summary(&mut self) -> (u64, u64) {
        let recv = self.recv_bytes.swap(0, Ordering::Relaxed);
        let send = self.send_bytes.swap(0, Ordering::Relaxed);
        self.last_recv = 0;
        (recv, send)
    }
}

impl GameWorld {
    /// Uncoupled from beat-stall [`GameWorld::lag`] (`communication.cc:141-229`).
    pub(crate) fn net_load_check(&mut self) {
        // Corpus `PlayersOnline` / `InGame()` includes CONNECTION_DEAD.
        let players = self.conn_to_creature.len() + self.dead_conn_state.len();
        if !self.net_load.check(self.round_nr, players) {
            return;
        }
        self.emergency_ping();
    }

    fn emergency_ping(&mut self) {
        let round = self.round_nr;
        let online: Vec<(tfs_rust_common::ConnId, crate::ids::CreatureId)> = self
            .conn_to_creature
            .iter()
            .map(|(&conn, &cid)| (conn, cid))
            .collect();
        for (conn_id, cid) in online {
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid)
                && round.saturating_sub(p.last_command_round) < EMERGENCY_PING_IDLE
            {
                p.last_command_round = round.saturating_sub(EMERGENCY_REWIND);
            }
            self.enqueue_periodic_ping(conn_id, cid);
        }
        let dead: Vec<(tfs_rust_common::ConnId, DeadConnState)> = self
            .dead_conn_state
            .iter()
            .map(|(&c, &s)| (c, s))
            .collect();
        for (conn_id, mut state) in dead {
            if round.saturating_sub(state.last_command_round) < EMERGENCY_PING_IDLE {
                state.last_command_round = round.saturating_sub(EMERGENCY_REWIND);
                self.dead_conn_state.insert(conn_id, state);
            }
            self.enqueue_conn_ping(conn_id, state.is_otclient);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::CreatureKind;
    use crate::game_world_lifecycle::LogoutPossible;
    use crate::test_world::support::{
        beat_driven_test_world, ensure_walkable_tile, insert_player, test_player,
    };
    use tfs_rust_common::{ConnId, Position};

    fn primed_recv(load: &mut NetLoad, bytes: u64) {
        load.recv_bytes.store(bytes, Ordering::Relaxed);
    }

    #[test]
    fn warmup_does_not_lag() {
        let mut n = NetLoad::default();
        primed_recv(&mut n, 1_000_000);
        assert!(!n.check(100, 80));
        assert!(!n.lag_detected(100));
    }

    #[test]
    fn below_50_players_does_not_lag() {
        let mut n = NetLoad::default();
        primed_recv(&mut n, 1_000_000);
        assert!(!n.check(4000, 10));
        assert!(!n.lag_detected(4000));
    }

    #[test]
    fn recv_halves_triggers_lag_window() {
        let mut n = NetLoad::default();
        // Fill the ring with healthy ~1000-byte / 50-player samples (per-player = 20).
        for i in 0..HISTORY_LEN {
            primed_recv(&mut n, 1000 * (i as u64 + 1));
            n.check(3600, 50);
        }
        assert!(!n.lag_detected(3600));
        // Next interval: tiny Δ vs ~1000 avg.
        primed_recv(&mut n, 1000 * HISTORY_LEN as u64 + 10);
        assert!(n.check(3601, 50));
        assert!(n.lag_detected(3601));
        assert!(n.lag_detected(3631));
        assert!(!n.lag_detected(3632));
    }

    #[test]
    fn emergency_ping_only_if_last_command_under_80() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let fresh = insert_player(&mut world, test_player("Fresh", pos));
        let stale = insert_player(&mut world, test_player("Stale", Position::new(101, 100, 7)));
        ensure_walkable_tile(&mut world.map, Position::new(101, 100, 7), 150);
        world.register_conn_mapping(ConnId(1), fresh);
        world.register_conn_mapping(ConnId(2), stale);
        world.round_nr = 200;
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(fresh) {
            p.last_command_round = 150; // idle 50 < 80
        }
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(stale) {
            p.last_command_round = 100; // idle 100 >= 80
        }
        world.emergency_ping();
        let fresh_cmd = match world.creatures.get(fresh) {
            Some(CreatureKind::Player(p)) => p.last_command_round,
            _ => panic!(),
        };
        let stale_cmd = match world.creatures.get(stale) {
            Some(CreatureKind::Player(p)) => p.last_command_round,
            _ => panic!(),
        };
        assert_eq!(fresh_cmd, 100, "round-100 rewind");
        assert_eq!(stale_cmd, 100, "idle ≥80 keeps stamp");
        assert!(world.pending_outgoing.get(&ConnId(1)).is_some());
        assert!(world.pending_outgoing.get(&ConnId(2)).is_some());
    }

    #[test]
    fn beat_stall_does_not_trigger_net_load() {
        let mut world = beat_driven_test_world();
        world.lag = true;
        world.round_nr = 4000;
        world.net_load_check();
        assert!(!world.net_load.lag_detected(4000));
    }

    #[test]
    fn lag_detected_grants_logout_in_combat() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let mut p = test_player("LagLogout", pos);
        p.earliest_logout_round = 500;
        let pid = insert_player(&mut world, p);
        world.round_nr = 100;
        assert_eq!(world.player_logout_possible(pid), LogoutPossible::Combat);

        world.net_load.lag_end = 130;
        assert_eq!(world.player_logout_possible(pid), LogoutPossible::Ok);
    }

    #[test]
    fn summary_zeros_byte_counters() {
        let mut n = NetLoad::default();
        primed_recv(&mut n, 5000);
        n.send_bytes.store(2000, Ordering::Relaxed);
        n.last_recv = 5000;
        let (recv, send) = n.summary();
        assert_eq!(recv, 5000);
        assert_eq!(send, 2000);
        assert_eq!(n.recv_bytes.load(Ordering::Relaxed), 0);
        assert_eq!(n.send_bytes.load(Ordering::Relaxed), 0);
        assert_eq!(n.last_recv, 0);
    }
}
