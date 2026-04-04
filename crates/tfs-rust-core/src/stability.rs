//! Error counters and tick health (Phase 10 expands recovery hooks).
// C++ reference: operational monitoring; pairs with `tasks.md` 10.10.

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    Network,
    Script,
    Database,
    Thread,
    TickOverrun,
}

#[derive(Debug, Default)]
pub struct StabilityManager {
    counts: DashMap<ErrorCategory, AtomicU64>,
}

impl StabilityManager {
    pub fn record_error(&self, cat: ErrorCategory) {
        self.counts
            .entry(cat)
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn get(&self, cat: ErrorCategory) -> u64 {
        self.counts
            .get(&cat)
            .map(|a| a.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
}
