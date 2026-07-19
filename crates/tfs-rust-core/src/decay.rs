//! Item decay scheduling (per-tick).
//!
//! Domain: TFS `Game::startDecay` / `Game::checkDecay` (`game.cpp`).
//! Outcomes: decompile `CronExpire` / `ProcessCronSystem` / `CronCheck` (`map.cc` / `operate.cc`) —
//! XML `duration` is seconds → deadline on the active decay clock (`MechanicsProfile::decay_clock`);
//! 772 `RoundNr + duration_seconds` (`map.cc`), 1098 movement `server_ms`.
//!
//! Indexed heap: `CronDelete` (`map.cc`) removes live entries by object id — heap size tracks
//! live decays, not historical schedule/cancel churn.

use crate::ids::ItemId;
use slotmap::Key;
use std::cmp::Ordering;
use std::collections::HashMap;

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
struct DecayHeapNode {
    deadline: u64,
    item_id: ItemId,
}

fn node_ord(a: &DecayHeapNode, b: &DecayHeapNode) -> Ordering {
    a.deadline
        .cmp(&b.deadline)
        .then_with(|| a.item_id.data().as_ffi().cmp(&b.item_id.data().as_ffi()))
}

fn node_less(a: &DecayHeapNode, b: &DecayHeapNode) -> bool {
    node_ord(a, b) == Ordering::Less
}

/// Cron-style decay scheduler — O(log n) schedule/cancel; `tick` only pops due heads.
///
/// C++ ref: `map.cc` `CronSet` / `CronCheck` / `CronDelete` (`TCronEntry` indexed heap).
#[derive(Debug, Default)]
pub struct DecayManager {
    entries: HashMap<ItemId, DecayEntry>,
    heap: Vec<DecayHeapNode>,
    /// ItemId → index in `heap` for O(log n) cancel/reschedule.
    heap_index: HashMap<ItemId, usize>,
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
        if let Some(&index) = self.heap_index.get(&id) {
            self.heap[index].deadline = deadline_tick;
            self.sift_up(index);
            self.sift_down(index);
        } else {
            self.push_node(DecayHeapNode {
                deadline: deadline_tick,
                item_id: id,
            });
        }
    }

    pub fn cancel(&mut self, id: ItemId) {
        self.entries.remove(&id);
        if let Some(index) = self.heap_index.remove(&id) {
            self.remove_at(index);
        }
    }

    /// Live scheduled entries (OBS-1 / DEC-2 diagnostics).
    #[inline]
    pub fn live_count(&self) -> usize {
        self.entries.len()
    }

    /// Indexed heap length — should match [`Self::live_count`] when cancel is eager.
    #[inline]
    pub fn heap_len(&self) -> usize {
        self.heap.len()
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
        while let Some(node) = self.heap.first().copied() {
            if node.deadline > now {
                break;
            }
            self.remove_at(0);
            let entry = self
                .entries
                .remove(&node.item_id)
                .expect("heap node must have matching entry");
            done.push((node.item_id, entry));
        }
        done
    }

    fn push_node(&mut self, node: DecayHeapNode) {
        let index = self.heap.len();
        self.heap_index.insert(node.item_id, index);
        self.heap.push(node);
        self.sift_up(index);
    }

    fn remove_at(&mut self, index: usize) {
        let item_id = self.heap[index].item_id;
        let last = self.heap.len() - 1;
        if index != last {
            self.heap.swap(index, last);
            self.heap_index.insert(self.heap[index].item_id, index);
            self.heap.pop();
            self.sift_up(index);
            self.sift_down(index);
        } else {
            self.heap.pop();
        }
        self.heap_index.remove(&item_id);
    }

    fn sift_up(&mut self, mut index: usize) {
        while index > 0 {
            let parent = (index - 1) / 2;
            if !node_less(&self.heap[index], &self.heap[parent]) {
                break;
            }
            self.swap_nodes(index, parent);
            index = parent;
        }
    }

    fn sift_down(&mut self, mut index: usize) {
        let len = self.heap.len();
        loop {
            let left = index * 2 + 1;
            if left >= len {
                break;
            }
            let right = left + 1;
            let mut smallest = index;
            if node_less(&self.heap[left], &self.heap[smallest]) {
                smallest = left;
            }
            if right < len && node_less(&self.heap[right], &self.heap[smallest]) {
                smallest = right;
            }
            if smallest == index {
                break;
            }
            self.swap_nodes(index, smallest);
            index = smallest;
        }
    }

    fn swap_nodes(&mut self, a: usize, b: usize) {
        self.heap.swap(a, b);
        self.heap_index.insert(self.heap[a].item_id, a);
        self.heap_index.insert(self.heap[b].item_id, b);
    }

    #[cfg(test)]
    fn live_heap_len(&self) -> usize {
        self.heap.len()
    }

    #[cfg(test)]
    fn live_entry_count(&self) -> usize {
        self.entries.len()
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
        assert_eq!(decay.live_heap_len(), 0);
    }

    #[test]
    fn tick_pops_only_due_entries() {
        let mut items: SlotMap<ItemId, ()> = SlotMap::with_key();
        let a = items.insert(());
        let b = items.insert(());
        let mut decay = DecayManager::default();
        decay.schedule(a, 100, None);
        decay.schedule(b, 500, Some(1));
        assert_eq!(decay.live_heap_len(), 2);
        assert!(decay.tick(50).is_empty());
        let due = decay.tick(100);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].0, a);
        assert_eq!(decay.remaining_ms(b, 100), Some(400));
        assert_eq!(decay.live_heap_len(), 1);
        let due = decay.tick(500);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].0, b);
        assert_eq!(decay.live_heap_len(), 0);
    }

    #[test]
    fn cancel_removes_heap_entry() {
        let mut items: SlotMap<ItemId, ()> = SlotMap::with_key();
        let id = items.insert(());
        let mut decay = DecayManager::default();
        decay.schedule(id, 100, None);
        assert_eq!(decay.live_heap_len(), 1);
        decay.cancel(id);
        assert_eq!(decay.live_heap_len(), 0);
        assert!(decay.tick(100).is_empty());
    }

    #[test]
    fn reschedule_uses_latest_deadline() {
        let mut items: SlotMap<ItemId, ()> = SlotMap::with_key();
        let id = items.insert(());
        let mut decay = DecayManager::default();
        decay.schedule(id, 100, None);
        assert_eq!(decay.live_heap_len(), 1);
        decay.schedule(id, 300, Some(2));
        assert_eq!(decay.live_heap_len(), 1);
        assert!(decay.tick(100).is_empty());
        let due = decay.tick(300);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].1.replace_with, Some(2));
        assert_eq!(decay.live_heap_len(), 0);
    }

    #[test]
    fn heap_size_tracks_live_entries_not_churn() {
        let mut items: SlotMap<ItemId, ()> = SlotMap::with_key();
        let mut decay = DecayManager::default();
        let far_future = 10_000_000_u64;

        for round in 0_u64..200 {
            let id = items.insert(());
            decay.schedule(id, far_future + round, None);
            assert_eq!(
                decay.live_heap_len(),
                decay.live_entry_count(),
                "schedule must keep heap in sync"
            );
            decay.cancel(id);
            assert_eq!(decay.live_heap_len(), 0, "cancel must remove heap node");
            assert_eq!(decay.live_entry_count(), 0);
        }

        // Keep a small live set while churning schedule/cancel on other ids.
        let _live: Vec<ItemId> = (0_u64..5)
            .map(|i| {
                let id = items.insert(());
                decay.schedule(id, far_future + i, None);
                id
            })
            .collect();
        assert_eq!(decay.live_heap_len(), 5);

        for round in 0_u64..500 {
            let ephemeral = items.insert(());
            decay.schedule(ephemeral, far_future + 1_000 + round, None);
            decay.cancel(ephemeral);
            assert_eq!(
                decay.live_heap_len(),
                decay.live_entry_count(),
                "churn round {round}: heap must not retain cancelled entries"
            );
            assert_eq!(decay.live_heap_len(), 5);
        }

        decay.tick(far_future + 10);
        assert_eq!(decay.live_heap_len(), 0);
    }
}
