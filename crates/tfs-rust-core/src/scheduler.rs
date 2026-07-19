//! Delayed events (`addEvent`) mapped onto Tokio timers → game thread channel.
//!
//! C++ reference: `scheduler.cpp` `Scheduler::addEvent` / `stopEvent` / `shutdown`.
//!
//! The `event_id` returned by [`Scheduler::schedule_after`] is the same id delivered
//! via `GameCommand::LuaCallback { event_id }` and the same id used by
//! [`Scheduler::stop_event`] for cancellation. In C++ the Lua-facing `lastTimerEventId`
//! and the scheduler-internal `eventId` are separate, but in our unified design they
//! coincide because the `Scheduler` is only used by `addEvent`/`stopEvent`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::runtime::Handle;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::AbortHandle;

use tfs_rust_common::GameCommand;
use tfs_rust_lua::TimerScheduler;
use tfs_rust_net::GameCmdTx;

#[derive(Debug)]
pub struct Scheduler {
    tx: GameCmdTx,
    handle: Handle,
    next_id: AtomicU64,
    /// `event_id → AbortHandle` for cancellation via `stopEvent`.
    /// Game-thread only — accessed via `&self` + `RefCell` so the `TimerScheduler`
    /// trait (which takes `&self`) can mutate it from Lua closures.
    timers: RefCell<HashMap<u64, AbortHandle>>,
}

impl Scheduler {
    pub fn new(tx: GameCmdTx, handle: Handle) -> Self {
        Self {
            tx,
            handle,
            next_id: AtomicU64::new(1),
            timers: RefCell::new(HashMap::new()),
        }
    }

    /// Remove a completed timer's abort handle after the game loop dispatches
    /// `LuaCallback`. The spawned task has already exited; this just cleans up the
    /// stale entry so the map doesn't grow unbounded.
    pub fn forget(&self, event_id: u64) {
        self.timers.borrow_mut().remove(&event_id);
    }

    /// Control-lane sender for tests that need the raw channel.
    pub fn ctrl_sender(&self) -> UnboundedSender<GameCommand> {
        self.tx.ctrl_sender()
    }
}

impl TimerScheduler for Scheduler {
    /// Spawn a one-shot timer that sends `GameCommand::LuaCallback { event_id }` after
    /// `delay`. Returns the `event_id` (also used for `stop_event`).
    ///
    /// C++ reference: `Scheduler::addEvent` (`scheduler.cpp:10`).
    fn schedule_after(&self, delay: Duration) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let tx = self.tx.clone();
        let join_handle = self.handle.spawn(async move {
            tokio::time::sleep(delay).await;
            let _ = tx.send(GameCommand::LuaCallback { event_id: id });
        });
        self.timers
            .borrow_mut()
            .insert(id, join_handle.abort_handle());
        id
    }

    /// Cancel a pending timer. Returns `true` if the id was found and aborted.
    ///
    /// C++ reference: `Scheduler::stopEvent` (`scheduler.cpp:39`).
    fn stop_event(&self, event_id: u64) -> bool {
        let mut timers = self.timers.borrow_mut();
        if let Some(handle) = timers.remove(&event_id) {
            handle.abort();
            true
        } else {
            false
        }
    }
}
