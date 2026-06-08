//! 772 global action scheduler — min-heap keyed by logical `ServerMilliseconds`.
//!
//! C++ reference: `tibia-game-master/src/cr.hh` (`ToDoQueue`),
//! `crmain.cc:1106` `MoveCreatures`, `cract.cc:955` `ToDoStart`.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::ids::CreatureId;

/// One heap entry: creature wakeup at `execution_time` (logical ms).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToDoEntry {
    pub execution_time: u64,
    pub creature_id: CreatureId,
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
            .then_with(|| self.creature_id.cmp(&other.creature_id))
    }
}

/// Global priority queue — `BinaryHeap<Reverse<ToDoEntry>>` for min-heap behavior.
#[derive(Debug, Default)]
pub struct ToDoQueue {
    heap: BinaryHeap<std::cmp::Reverse<ToDoEntry>>,
}

impl ToDoQueue {
    pub fn insert(&mut self, execution_time: u64, creature_id: CreatureId) {
        self.heap.push(std::cmp::Reverse(ToDoEntry {
            execution_time,
            creature_id,
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
    fn tie_breaks_on_creature_id() {
        let a = cid(1);
        let b = cid(2);
        let mut q = ToDoQueue::default();
        q.insert(200, b);
        q.insert(200, a);
        let first = q.pop().unwrap();
        let second = q.pop().unwrap();
        assert_eq!(first.execution_time, 200);
        assert_eq!(second.execution_time, 200);
        assert!(first.creature_id < second.creature_id);
    }
}
