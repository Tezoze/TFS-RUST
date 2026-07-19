//! Item decay scheduling (per-tick).
//!
//! Domain: TFS `Game::startDecay` / `Game::checkDecay` (`game.cpp`).
//! Outcomes: decompile `CronExpire` / `ProcessCronSystem` / `CronCheck` (`map.cc` / `operate.cc`) —
//! XML `duration` is seconds → deadline on `server_ms`; due heads pop from a min-heap (772
//! `TCronEntry` heap), not a full `HashMap::retain` scan.

use crate::ids::ItemId;
use slotmap::Key;
use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap};

/// Absolute `server_ms` deadline from remaining duration in **seconds**.
///
/// C++: instance duration is ms (`decayTime * 1000`); decompile Cron delay is RoundNr seconds.
pub fn decay_deadline_ms(now_ms: u64, duration_sec: u32) -> u64 {
    now_ms.saturating_add(u64::from(duration_sec).saturating_mul(1000))
}

#[derive(Debug, Clone)]
pub struct DecayEntry {
    /// Absolute `server_ms` when the item should transform / vanish.
    pub deadline_tick: u64,
    pub replace_with: Option<u16>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct DecayHeapKey {
    deadline: u64,
    item_id: ItemId,
}

impl Ord for DecayHeapKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.deadline
            .cmp(&other.deadline)
            .then_with(|| self.item_id.data().as_ffi().cmp(&other.item_id.data().as_ffi()))
    }
}

impl PartialOrd for DecayHeapKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Cron-style decay scheduler — O(log n) schedule/cancel; `tick` only pops due heads.
///
/// C++ ref: `map.cc` `CronSet` / `CronCheck` / `CronDelete` (`TCronEntry` heap).
#[derive(Debug, Default)]
pub struct DecayManager {
    entries: HashMap<ItemId, DecayEntry>,
    /// Min-heap keyed by deadline. Stale keys (cancelled / rescheduled) skipped on pop.
    heap: BinaryHeap<Reverse<DecayHeapKey>>,
}

impl DecayManager {
    pub fn schedule(&mut self, id: ItemId, deadline_tick: u64, replace_with: Option<u16>) {
        self.entries.insert(
            id,
            DecayEntry {
                deadline_tick,
                replace_with,
            },
        );
        self.heap.push(Reverse(DecayHeapKey {
            deadline: deadline_tick,
            item_id: id,
        }));
    }

    pub fn cancel(&mut self, id: ItemId) {
        self.entries.remove(&id);
        // Heap entry becomes stale; skipped on tick.
    }

    /// Remaining ms until deadline, if the item is scheduled.
    pub fn remaining_ms(&self, id: ItemId, now_ms: u64) -> Option<u64> {
        self.entries
            .get(&id)
            .map(|e| e.deadline_tick.saturating_sub(now_ms))
    }

    /// Pop all entries with `deadline_tick <= now` (772 `CronCheck` loop shape).
    pub fn tick(&mut self, now: u64) -> Vec<(ItemId, DecayEntry)> {
        let mut done = Vec::new();
        while let Some(Reverse(key)) = self.heap.peek().copied() {
            if key.deadline > now {
                break;
            }
            self.heap.pop();
            let Some(entry) = self.entries.get(&key.item_id) else {
                continue; // cancelled
            };
            if entry.deadline_tick != key.deadline {
                continue; // rescheduled; newer heap key still pending
            }
            let entry = self.entries.remove(&key.item_id).expect("checked above");
            done.push((key.item_id, entry));
        }
        done
    }
}

#[cfg(test)]
mod tests {
    use super::{decay_deadline_ms, DecayManager};
    use crate::ids::ItemId;
    use slotmap::SlotMap;

    #[test]
    fn decay_deadline_matches_xml_seconds_to_ms() {
        // firefield 1487 duration=200; lit candelabrum 2042=3000; dead troll 2806=1800
        assert_eq!(decay_deadline_ms(0, 200), 200_000);
        assert_eq!(decay_deadline_ms(0, 3000), 3_000_000);
        assert_eq!(decay_deadline_ms(0, 1800), 1_800_000);
        assert_eq!(decay_deadline_ms(50_000, 10), 60_000);
    }

    #[test]
    fn remaining_ms_tracks_deadline() {
        let mut items: SlotMap<ItemId, ()> = SlotMap::with_key();
        let id = items.insert(());
        let mut decay = DecayManager::default();
        decay.schedule(id, 5_000, Some(100));
        assert_eq!(decay.remaining_ms(id, 2_000), Some(3_000));
        assert_eq!(decay.remaining_ms(id, 5_000), Some(0));
        decay.cancel(id);
        assert_eq!(decay.remaining_ms(id, 2_000), None);
    }

    #[test]
    fn tick_pops_only_due_entries() {
        let mut items: SlotMap<ItemId, ()> = SlotMap::with_key();
        let a = items.insert(());
        let b = items.insert(());
        let mut decay = DecayManager::default();
        decay.schedule(a, 100, None);
        decay.schedule(b, 500, Some(1));
        assert!(decay.tick(50).is_empty());
        let due = decay.tick(100);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].0, a);
        assert_eq!(decay.remaining_ms(b, 100), Some(400));
        let due = decay.tick(500);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].0, b);
    }

    #[test]
    fn cancel_skips_stale_heap_entry() {
        let mut items: SlotMap<ItemId, ()> = SlotMap::with_key();
        let id = items.insert(());
        let mut decay = DecayManager::default();
        decay.schedule(id, 100, None);
        decay.cancel(id);
        assert!(decay.tick(100).is_empty());
    }

    #[test]
    fn reschedule_uses_latest_deadline() {
        let mut items: SlotMap<ItemId, ()> = SlotMap::with_key();
        let id = items.insert(());
        let mut decay = DecayManager::default();
        decay.schedule(id, 100, None);
        decay.schedule(id, 300, Some(2));
        assert!(decay.tick(100).is_empty());
        let due = decay.tick(300);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].1.replace_with, Some(2));
    }
}
