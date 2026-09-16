//! Open-loop schedule: actions fire at intended `Instant`s (CLOCK_MONOTONIC via `Instant`).
//!
//! If a prior action is still outstanding when the next is due, that wait is already
//! included because latency is ack − *intended*, not ack − send.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use tfs_rust_common::Position;

use crate::latency::Correlate;

#[derive(Debug, Clone)]
pub enum ActionKind {
    Walk(u8),
    Attack(u32),
    Say(String),
    UseItemEx {
        from: Position,
        from_sprite: u16,
        /// Server item id; 0 means use `from_sprite` as-is.
        from_server_id: u16,
        to_sprite: u16,
        /// If true, dest is `last_other_creature_pos` (else self pos) at send time.
        target_other: bool,
        /// Expected `0x83` effect id for the ack (`None` = any effect at the
        /// tile). Set for area runes (GFB `CONST_ME_FIREAREA`) so unrelated
        /// splashes on the target tile do not count as acks.
        expect_effect: Option<u8>,
    },
    LookAt(Position),
}

impl ActionKind {
    pub fn correlate(&self) -> Option<(Correlate, Option<Position>)> {
        match self {
            Self::Walk(_) => Some((Correlate::Walk, None)),
            Self::UseItemEx { .. } => Some((Correlate::SpellRune, None)),
            Self::Say(_) => Some((Correlate::SpellRune, None)),
            Self::Attack(_) | Self::LookAt(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScheduledAction {
    pub intended: Instant,
    pub kind: ActionKind,
}

pub struct OpenLoop {
    queue: VecDeque<ScheduledAction>,
}

impl Default for OpenLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenLoop {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, action: ScheduledAction) {
        self.queue.push_back(action);
    }

    pub fn peek_due(&self, now: Instant) -> Option<&ScheduledAction> {
        match self.queue.front() {
            Some(a) if a.intended <= now => Some(a),
            _ => None,
        }
    }

    pub fn pop_due(&mut self, now: Instant) -> Option<ScheduledAction> {
        if self.peek_due(now).is_some() {
            self.queue.pop_front()
        } else {
            None
        }
    }

    pub fn next_intended(&self) -> Option<Instant> {
        self.queue.front().map(|a| a.intended)
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Pre-schedule `count` walks alternating N/S starting at `start`, period `period`.
    pub fn schedule_walk_ns(&mut self, start: Instant, count: u32, period: Duration) {
        use tfs_rust_common::protocol_opcodes::client;
        for i in 0..count {
            let op = if i.is_multiple_of(2) {
                client::MOVE_NORTH
            } else {
                client::MOVE_SOUTH
            };
            self.push(ScheduledAction {
                intended: start + period * i,
                kind: ActionKind::Walk(op),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_only_after_intended() {
        let mut ol = OpenLoop::new();
        let t0 = Instant::now();
        ol.schedule_walk_ns(t0 + Duration::from_secs(10), 2, Duration::from_millis(200));
        assert!(ol.peek_due(t0).is_none());
        assert!(ol.peek_due(t0 + Duration::from_secs(11)).is_some());
        let a = ol.pop_due(t0 + Duration::from_secs(11)).expect("due");
        assert!(matches!(a.kind, ActionKind::Walk(_)));
    }
}
