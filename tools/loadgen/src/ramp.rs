//! Login ramp: at most 8 in flight from 7171 through the game-port first
//! packet (`MAX_CONCURRENT_LOGIN_LOADS` in `login.rs`). Do not hold until
//! self-appear — that serializes ingest on lagged `SendAll`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use tokio::sync::Mutex;
use tokio::sync::Semaphore;

/// Same cap as `tfs-rust-core::login::MAX_CONCURRENT_LOGIN_LOADS` (value 8, not the line number).
pub const MAX_CONCURRENT_LOGINS: usize = 8;

pub struct LoginGate {
    sem: Arc<Semaphore>,
    min_interval: Duration,
    last_start: Mutex<Option<Instant>>,
}

impl LoginGate {
    pub fn new(max_concurrent: usize, per_sec: usize) -> Self {
        let per_sec = per_sec.max(1);
        Self {
            sem: Arc::new(Semaphore::new(max_concurrent.max(1))),
            min_interval: Duration::from_millis(1000 / per_sec as u64),
            last_start: Mutex::new(None),
        }
    }

    pub fn phase_d_default() -> Self {
        Self::new(MAX_CONCURRENT_LOGINS, MAX_CONCURRENT_LOGINS)
    }

    /// Cap 8 in flight, no extra 8/s spacing. Use with [`login_start_delay`] so
    /// ingest is the spread, not the interval race.
    pub fn concurrent_only() -> Self {
        Self {
            sem: Arc::new(Semaphore::new(MAX_CONCURRENT_LOGINS)),
            min_interval: Duration::ZERO,
            last_start: Mutex::new(None),
        }
    }

    /// Hold the slot until the returned permit is dropped (after 7172 first packet).
    pub async fn acquire(&self) -> Result<tokio::sync::SemaphorePermit<'_>> {
        let permit = self
            .sem
            .acquire()
            .await
            .map_err(|_| anyhow!("login ramp semaphore closed"))?;
        let sleep_for = {
            let last = self.last_start.lock().await;
            last.and_then(|t| {
                let next = t + self.min_interval;
                next.checked_duration_since(Instant::now())
            })
        };
        if let Some(d) = sleep_for {
            tokio::time::sleep(d).await;
        }
        *self.last_start.lock().await = Some(Instant::now());
        Ok(permit)
    }
}

/// Bot `index` of `bots` starts this long after process start. First bot is 0,
/// last is `spread`. Cap stays 8 (`LoginGate`).
pub fn login_start_delay(index: usize, bots: usize, spread: Duration) -> Duration {
    if bots <= 1 || spread.is_zero() {
        return Duration::ZERO;
    }
    spread.mul_f64(index as f64 / (bots - 1) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_cap_is_eight() {
        let g = LoginGate::phase_d_default();
        assert_eq!(g.sem.available_permits(), MAX_CONCURRENT_LOGINS);
    }

    #[test]
    fn spread_first_zero_last_equals_spread() {
        let spread = Duration::from_secs(120);
        assert_eq!(login_start_delay(0, 1000, spread), Duration::ZERO);
        assert_eq!(login_start_delay(999, 1000, spread), spread);
        assert_eq!(login_start_delay(0, 1, spread), Duration::ZERO);
        assert_eq!(login_start_delay(5, 1000, Duration::ZERO), Duration::ZERO);
    }

    #[tokio::test]
    async fn concurrent_only_is_eight_and_no_interval() {
        let g = LoginGate::concurrent_only();
        assert_eq!(g.sem.available_permits(), MAX_CONCURRENT_LOGINS);
        assert!(g.min_interval.is_zero());
    }
}
