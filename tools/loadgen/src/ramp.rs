//! Login ramp: at most 8 in flight and ≤ 8 starts per second
//! (`MAX_CONCURRENT_LOGIN_LOADS` in `login.rs`).

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

    /// Hold the slot until the returned permit is dropped (after login handshake).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_cap_is_eight() {
        let g = LoginGate::phase_d_default();
        assert_eq!(g.sem.available_permits(), MAX_CONCURRENT_LOGINS);
    }
}
