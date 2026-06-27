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

    // ---- Phase 0 oracle: an independent literal transcription of the CipSoft algorithm. ----
    // Guards the production `ToDoQueue` against any future "optimization" that would change the
    // structural tie order. Both must agree for every insert/pop sequence.
    #[derive(Default)]
    struct CipSoftOracle {
        key: Vec<u64>,
        data: Vec<u64>,
        entries: usize,
    }
    impl CipSoftOracle {
        fn insert(&mut self, k: u64, d: u64) {
            self.entries += 1;
            let cur = self.entries;
            if self.key.is_empty() {
                self.key.push(0);
                self.data.push(0);
            }
            if cur >= self.key.len() {
                self.key.resize(cur + 1, 0);
                self.data.resize(cur + 1, 0);
            }
            self.key[cur] = k;
            self.data[cur] = d;
            let mut i = cur;
            while i > 1 {
                let p = i / 2;
                if self.key[p] <= self.key[i] {
                    break;
                }
                self.key.swap(p, i);
                self.data.swap(p, i);
                i = p;
            }
        }
        fn pop(&mut self) -> Option<u64> {
            if self.entries < 1 {
                return None;
            }
            let out = self.data[1];
            if self.entries > 1 {
                let last = self.entries;
                self.key.swap(1, last);
                self.data.swap(1, last);
                let mut i = 1usize;
                loop {
                    let mut s = i * 2;
                    if s >= last {
                        break;
                    }
                    if s + 1 < last && self.key[s + 1] < self.key[s] {
                        s += 1;
                    }
                    if self.key[i] <= self.key[s] {
                        break;
                    }
                    self.key.swap(i, s);
                    self.data.swap(i, s);
                    i = s;
                }
            }
            self.entries -= 1;
            Some(out)
        }
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
        assert!(q.pop().is_none());
    }

    #[test]
    fn equal_key_order_is_structural_not_fifo() {
        // Hand-traced against `containers.hh`: insert A,B,C at equal key →
        // pop order A, C, B (NOT FIFO A,B,C). See audit Finding 6.
        let a = cid(1);
        let b = cid(2);
        let c = cid(3);
        let mut q = ToDoQueue::default();
        q.insert(1000, a);
        q.insert(1000, b);
        q.insert(1000, c);
        let popped: Vec<CreatureId> = std::iter::from_fn(|| q.pop().map(|e| e.creature_id)).collect();
        assert_eq!(popped, vec![a, c, b]);
    }

    #[test]
    fn matches_cipsoft_oracle_on_randomized_sequences() {
        // Differential test: production queue vs the literal oracle transcription, over many
        // pseudo-random interleaved insert/pop sequences. Data = a monotonic id so we can compare
        // the exact pop order (including equal-key structural ties).
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..200 {
            let mut q = ToDoQueue::default();
            let mut oracle = CipSoftOracle::default();
            // map a synthetic u64 data id -> CreatureId so we can compare order
            let mut ids: Vec<CreatureId> = Vec::new();
            let ops = 40 + (next() % 60) as usize;
            let mut counter: u64 = 0;
            for _ in 0..ops {
                if (next() % 3) != 0 || q.is_empty() {
                    // insert: small key space to force frequent equal-key ties
                    let key = next() % 5;
                    let data = counter;
                    counter += 1;
                    let c = cid(data);
                    if data as usize >= ids.len() {
                        ids.resize(data as usize + 1, c);
                    }
                    ids[data as usize] = c;
                    q.insert(key, c);
                    oracle.insert(key, data);
                } else {
                    let a = q.pop().map(|e| e.creature_id);
                    let b = oracle.pop().map(|d| ids[d as usize]);
                    assert_eq!(a, b, "production queue diverged from CipSoft oracle");
                }
            }
            // drain both fully
            loop {
                let a = q.pop().map(|e| e.creature_id);
                let b = oracle.pop().map(|d| ids[d as usize]);
                assert_eq!(a, b, "drain order diverged from CipSoft oracle");
                if a.is_none() {
                    break;
                }
            }
        }
    }

    #[test]
    fn len_and_is_empty_track_entries() {
        let mut q = ToDoQueue::default();
        assert!(q.is_empty());
        q.insert(10, cid(1));
        q.insert(20, cid(2));
        assert_eq!(q.len(), 2);
        q.pop();
        assert_eq!(q.len(), 1);
        q.pop();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
    }
}
