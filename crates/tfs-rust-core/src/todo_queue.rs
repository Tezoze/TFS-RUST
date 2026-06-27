//! 772 global action scheduler — min-heap keyed by logical `ServerMilliseconds`.
//!
//! C++ reference: `tibia-game-master/src/cr.hh` (`ToDoQueue`),
//! `containers.hh` `priority_queue` (Key-only), `crmain.cc:1137` `MoveCreatures`,
//! `cract.cc:1015` `ToDoStart`.
//!
//! Harness multi-monster scenarios use an explicit secondary tie (`sequence`) because
//! equal `ServerMilliseconds` drain order is path-dependent (appear LIFO vs go-step order).

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::ids::CreatureId;

/// One heap entry: creature wakeup at `execution_time` (logical ms).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToDoEntry {
    pub execution_time: u64,
    pub creature_id: CreatureId,
    /// Secondary tie when `execution_time` matches — harness parity (P2.5g).
    pub sequence: u64,
}

impl PartialOrd for ToDoEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ToDoEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.execution_time
            .cmp(&other.execution_time)
            .then_with(|| self.sequence.cmp(&other.sequence))
    }
}

/// Global priority queue — min-heap via `BinaryHeap<Reverse<ToDoEntry>>`.
#[derive(Debug, Default)]
pub struct ToDoQueue {
    heap: BinaryHeap<std::cmp::Reverse<ToDoEntry>>,
    next_sequence: u64,
}

impl ToDoQueue {
    pub fn insert(&mut self, execution_time: u64, creature_id: CreatureId) {
        let tie = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        self.insert_with_tie(execution_time, creature_id, tie);
    }

    pub fn bump_sequence(&mut self) -> u64 {
        let tie = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        tie
    }

    pub fn insert_with_tie(&mut self, execution_time: u64, creature_id: CreatureId, tie: u64) {
        self.heap.push(std::cmp::Reverse(ToDoEntry {
            execution_time,
            creature_id,
            sequence: tie,
        }));
    }

    pub fn peek(&self) -> Option<ToDoEntry> {
        self.heap.peek().map(|r| r.0)
    }

    pub fn pop(&mut self) -> Option<ToDoEntry> {
        self.heap.pop().map(|r| r.0)
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Drop pending wakeups for `creature_id` — reschedule after idle_dance go (`kite @6000`).
    pub fn remove_creature(&mut self, creature_id: CreatureId) {
        let kept: Vec<_> = self
            .heap
            .iter()
            .filter(|r| r.0.creature_id != creature_id)
            .cloned()
            .collect();
        self.heap = kept.into_iter().collect();
    }
}

/// Secondary tie for harness multi-monster equal-key wakeups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeupTiePolicy {
    /// Forward `ToDoYield` — last yielded drains first (`chase_kite_scenario.cc`).
    HarnessAppearIdle,
    /// First `Go` after idle @4000 — C++ oracle cyclops quad (`chase_path_cip_cyclops.log`).
    HarnessGoStep,
    /// Production / single-monster — FIFO insertion order.
    Fifo,
}

/// P2.5g — `go_exec` @4000 index order NW, S, far-N, E for quad cyclops spawn layout.
pub fn harness_go_step_tie(spawn_order: u16) -> u64 {
    match spawn_order {
        4 => 0, // NW
        3 => 1, // south
        1 => 2, // far-N
        2 => 3, // east
        n => u64::from(n),
    }
}

/// Real-map cyclops bowl dual spawn — `kite_cyclops_two_real` drain @400/2000.
///
/// C++ `MoveCreatures` drains north cyclops (spawn 2) before east (spawn 1) on equal key.
pub fn harness_go_step_tie_realmap_bowl(spawn_order: u16) -> u64 {
    match spawn_order {
        2 => 0, // east-north @ `(32454,32066)` — drains first
        1 => 1, // east @ `(32454,32065)` — drains second
        n => u64::from(n),
    }
}

pub fn harness_appear_idle_tie(spawn_order: u16) -> u64 {
    u64::from(u16::MAX - spawn_order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    fn cid(n: u64) -> CreatureId {
        let mut map: SlotMap<CreatureId, ()> = SlotMap::with_key();
        for _ in 0..n {
            map.insert(());
        }
        map.insert(())
    }

    fn pops_with_tie(inserts: &[(CreatureId, u64)], key: u64) -> Vec<CreatureId> {
        let mut q = ToDoQueue::default();
        for &(creature_id, tie) in inserts {
            q.insert_with_tie(key, creature_id, tie);
        }
        std::iter::from_fn(|| q.pop().map(|e| e.creature_id))
            .take(inserts.len())
            .collect()
    }

    #[test]
    fn min_heap_pops_earliest_first() {
        let a = cid(1);
        let b = cid(2);
        let c = cid(3);
        let mut q = ToDoQueue::default();
        q.insert(500, b);
        q.insert(100, a);
        q.insert(300, c);
        assert_eq!(q.pop().unwrap().execution_time, 100);
        assert_eq!(q.pop().unwrap().execution_time, 300);
        assert_eq!(q.pop().unwrap().execution_time, 500);
    }

    #[test]
    fn harness_appear_idle_lifo_spawn() {
        let far_n = cid(1);
        let east = cid(2);
        let south = cid(3);
        let nw = cid(4);
        let inserts = [
            (far_n, harness_appear_idle_tie(1)),
            (east, harness_appear_idle_tie(2)),
            (south, harness_appear_idle_tie(3)),
            (nw, harness_appear_idle_tie(4)),
        ];
        assert_eq!(pops_with_tie(&inserts, 2_000), vec![nw, south, east, far_n]);
    }

    #[test]
    fn harness_go_step_realmap_bowl_dual_at_400() {
        let east = cid(1);
        let north = cid(2);
        let inserts = [
            (east, harness_go_step_tie_realmap_bowl(1)),
            (north, harness_go_step_tie_realmap_bowl(2)),
        ];
        assert_eq!(pops_with_tie(&inserts, 400), vec![north, east]);
    }

    #[test]
    fn harness_go_step_cyclops_quad_at_4000() {
        let far_n = cid(1);
        let east = cid(2);
        let south = cid(3);
        let nw = cid(4);
        // Idle @2000 schedules @2001 in process order NW, S, E, far-N.
        let inserts = [
            (nw, harness_go_step_tie(4)),
            (south, harness_go_step_tie(3)),
            (east, harness_go_step_tie(2)),
            (far_n, harness_go_step_tie(1)),
        ];
        assert_eq!(pops_with_tie(&inserts, 2_001), vec![nw, south, far_n, east]);
    }
}
