//! Live run counters and `--progress` stderr ticker.
//!
//! Disconnects are counted when a game session drops before the measurement
//! window ends. Loadgen does **not** auto-reconnect (that would change offered
//! load mid-run). `reconnects` stays 0 and exists so the report and gate can
//! assert both sides behave the same.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::sync::oneshot;

const RELAXED: Ordering = Ordering::Relaxed;

#[derive(Debug, Default)]
pub struct LiveCounters {
    connected: AtomicU64,
    in_world: AtomicU64,
    actions_sent: AtomicU64,
    bytes_in: AtomicU64,
    bytes_discarded: AtomicU64,
    disconnects: AtomicU64,
    reconnects: AtomicU64,
}

impl LiveCounters {
    pub fn snapshot_line(&self, bots: usize) -> String {
        format!(
            "loadgen: connected={}/{} in_world={} actions_sent={} bytes_in={} bytes_discarded={} disconnects={} reconnects={}",
            self.connected.load(RELAXED),
            bots,
            self.in_world.load(RELAXED),
            self.actions_sent.load(RELAXED),
            self.bytes_in.load(RELAXED),
            self.bytes_discarded.load(RELAXED),
            self.disconnects.load(RELAXED),
            self.reconnects.load(RELAXED),
        )
    }
}

/// +1 `connected` while the game socket is up; `in_world` after self-appear.
pub struct SessionLease {
    live: Arc<LiveCounters>,
    connected: bool,
    in_world: bool,
}

impl SessionLease {
    pub fn new(live: Arc<LiveCounters>) -> Self {
        live.connected.fetch_add(1, RELAXED);
        Self {
            live,
            connected: true,
            in_world: false,
        }
    }

    pub fn mark_in_world(&mut self) {
        if !self.in_world {
            self.live.in_world.fetch_add(1, RELAXED);
            self.in_world = true;
        }
    }

    pub fn note_disconnect(&self) {
        self.live.disconnects.fetch_add(1, RELAXED);
    }

    pub fn note_action(&self) {
        self.live.actions_sent.fetch_add(1, RELAXED);
    }

    pub fn add_bytes(&self, bytes_in: u64, bytes_discarded: u64) {
        if bytes_in != 0 {
            self.live.bytes_in.fetch_add(bytes_in, RELAXED);
        }
        if bytes_discarded != 0 {
            self.live
                .bytes_discarded
                .fetch_add(bytes_discarded, RELAXED);
        }
    }
}

impl Drop for SessionLease {
    fn drop(&mut self) {
        if self.connected {
            self.live.connected.fetch_sub(1, RELAXED);
        }
        if self.in_world {
            self.live.in_world.fetch_sub(1, RELAXED);
        }
    }
}

pub async fn run_progress_ticker(
    live: Arc<LiveCounters>,
    bots: usize,
    stop: oneshot::Receiver<()>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tokio::pin!(stop);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                eprintln!("{}", live.snapshot_line(bots));
            }
            _ = &mut stop => {
                eprintln!("{}", live.snapshot_line(bots));
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_line_format() {
        let live = LiveCounters::default();
        live.connected.store(2, RELAXED);
        live.in_world.store(1, RELAXED);
        live.actions_sent.store(40, RELAXED);
        live.bytes_in.store(1000, RELAXED);
        live.bytes_discarded.store(3, RELAXED);
        live.disconnects.store(1, RELAXED);
        let line = live.snapshot_line(5);
        assert_eq!(
            line,
            "loadgen: connected=2/5 in_world=1 actions_sent=40 bytes_in=1000 bytes_discarded=3 disconnects=1 reconnects=0"
        );
    }

    #[test]
    fn lease_connected_in_world_and_drop() {
        let live = Arc::new(LiveCounters::default());
        {
            let mut lease = SessionLease::new(Arc::clone(&live));
            assert_eq!(live.connected.load(RELAXED), 1);
            lease.mark_in_world();
            lease.mark_in_world();
            assert_eq!(live.in_world.load(RELAXED), 1);
            lease.note_action();
            lease.add_bytes(10, 2);
            lease.note_disconnect();
            assert_eq!(live.actions_sent.load(RELAXED), 1);
            assert_eq!(live.bytes_in.load(RELAXED), 10);
            assert_eq!(live.bytes_discarded.load(RELAXED), 2);
            assert_eq!(live.disconnects.load(RELAXED), 1);
        }
        assert_eq!(live.connected.load(RELAXED), 0);
        assert_eq!(live.in_world.load(RELAXED), 0);
        assert_eq!(live.disconnects.load(RELAXED), 1);
        assert_eq!(live.reconnects.load(RELAXED), 0);
    }
}
