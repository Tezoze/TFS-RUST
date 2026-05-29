//! Delayed events (`addEvent`) mapped onto Tokio timers → game thread channel.
// C++ reference: `scheduler.cpp` `Scheduler::addEvent`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::runtime::Handle;
use tokio::sync::mpsc::UnboundedSender;

use tfs_rust_common::GameCommand;

#[derive(Debug)]
pub struct Scheduler {
    tx: UnboundedSender<GameCommand>,
    handle: Handle,
    next_id: AtomicU64,
}

impl Scheduler {
    pub fn new(tx: UnboundedSender<GameCommand>, handle: Handle) -> Self {
        Self {
            tx,
            handle,
            next_id: AtomicU64::new(1),
        }
    }

    /// Returns scheduler event id delivered via `GameCommand::LuaCallback`.
    pub fn schedule_after(&self, delay: Duration) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let tx = self.tx.clone();
        self.handle.spawn(async move {
            tokio::time::sleep(delay).await;
            let _ = tx.send(GameCommand::LuaCallback { event_id: id });
        });
        id
    }
}
