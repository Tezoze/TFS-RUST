//! 772 global action scheduler — binary min-heap keyed by logical `ServerMilliseconds`.
//!
//! C++ reference: `tibia-game-master/src/containers.hh:150–227` (`priority_queue<K,T>`),
//! `crmain.cc:1142` `MoveCreatures`, `cract.cc:1015` `ToDoStart`.
//!
//! This is a **verbatim port** of CipSoft's hand-rolled `priority_queue`: a 1-indexed array
//! binary heap keyed on `ExecutionTime` only (`Data = CreatureId`), with **no secondary key**.
//! Equal-`ExecutionTime` pop order is therefore the *structural* order produced by the exact
//! `insert` sift-up (`Parent.Key <= Current.Key` ⇒ stop) and `deleteMin` sift-down (strict
//! left-child bias `Other.Key < Smallest.Key`). Reproducing this structure — rather than a
//! Rust `BinaryHeap` plus a FIFO/`sequence` tie — is what makes multi-creature drain order match
//! the oracle without per-scenario tie maps. See `docs/GAME_LOOP_772_AUDIT.md` Findings 6 / Phase 1.

use crate::ids::CreatureId;

/// One heap entry: creature wakeup at `execution_time` (logical ms). `Data` in the C++ queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToDoEntry {
    pub execution_time: u64,
    pub creature_id: CreatureId,
}

/// Global priority queue — verbatim CipSoft `priority_queue<uint32, uint32>` (`containers.hh:150`).
///
/// 1-indexed: `slots[0]` is an unused sentinel so the arithmetic matches the C++ (`at(1)` is the
/// root, parent of `i` is `i/2`, children are `2i` / `2i+1`). `entries` is the live count.
#[derive(Debug, Default)]
pub struct ToDoQueue {
    slots: Vec<ToDoEntry>,
    entries: usize,
}

impl ToDoQueue {
    /// `priority_queue::insert` (`containers.hh:162`) — append at `entries`, then sift up while the
    /// parent key is **strictly greater** (`Parent->Key <= Current->Key` ⇒ break, so equal keys do
    /// not bubble past existing equal-key entries).
    pub fn insert(&mut self, execution_time: u64, creature_id: CreatureId) {
        let entry = ToDoEntry {
            execution_time,
            creature_id,
        };
        self.entries += 1;
        let mut current = self.entries;
        if self.slots.is_empty() {
            // sentinel at index 0
            self.slots.push(entry);
        }
        if current >= self.slots.len() {
            self.slots.resize(current + 1, entry);
        }
        self.slots[current] = entry;
        while current > 1 {
            let parent = current / 2;
            if self.slots[parent].execution_time <= self.slots[current].execution_time {
                break;
            }
            self.slots.swap(parent, current);
            current = parent;
        }
    }

    /// The minimum entry (`Entry->at(1)`), without removing it.
    pub fn peek(&self) -> Option<ToDoEntry> {
        if self.entries < 1 {
            None
        } else {
            Some(self.slots[1])
        }
    }

    /// Read the min (`Entry->at(1)`) then `deleteMin` — mirrors `MoveCreatures`
    /// (`auto Entry = *ToDoQueue.Entry->at(1); … ToDoQueue.deleteMin();`).
    pub fn pop(&mut self) -> Option<ToDoEntry> {
        if self.entries < 1 {
            return None;
        }
        let result = self.slots[1];
        self.delete_min();
        Some(result)
    }

    /// `priority_queue::deleteMin` (`containers.hh:177`) — swap root with last, then sift the new
    /// root down over `[1, LastIndex)` with `LastIndex == entries` (the count *before* decrement),
    /// strict left-child bias on ties (`Other->Key < Smallest->Key`).
    fn delete_min(&mut self) {
        if self.entries < 1 {
            return;
        }
        if self.entries > 1 {
            let last = self.entries;
            self.slots.swap(1, last);
            let mut current = 1usize;
            loop {
                let mut smallest = current * 2;
                if smallest >= last {
                    break;
                }
                if smallest + 1 < last
                    && self.slots[smallest + 1].execution_time < self.slots[smallest].execution_time
                {
                    smallest += 1;
                }
                if self.slots[current].execution_time <= self.slots[smallest].execution_time {
                    break;
                }
                self.slots.swap(current, smallest);
                current = smallest;
            }
        }
        self.entries -= 1;
    }

    pub fn is_empty(&self) -> bool {
        self.entries < 1
    }

    pub fn len(&self) -> usize {
        self.entries
    }
}

#[cfg(test)]
#[path = "todo_queue_tests.rs"]
mod tests;
