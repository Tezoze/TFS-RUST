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
