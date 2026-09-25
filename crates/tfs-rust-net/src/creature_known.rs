//! Per-connection known-creature slots — decompile `TConnection::KnownCreatureTable`.
//!
//! - 772: `connections.hh` `KnownCreatureTable[150]`; `connections.cc` `KnownCreature` /
//!   `NewKnownCreature`.
//! - 1098 domain: repo-root TFS `protocolgame.cpp` (`size() > 1300`).

/// 150-slot (772) / 1300-slot (1098) table. Slot value `0` is free
/// (`GetCreature(0) == NULL` — `crmain.cc`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KnownCreatureTable {
    slots: Vec<u32>,
}

impl KnownCreatureTable {
    pub fn with_limit(limit: usize) -> Self {
        Self {
            slots: vec![0; limit],
        }
    }

    fn ensure_len(&mut self, limit: usize) {
        if self.slots.len() < limit {
            self.slots.resize(limit, 0);
        }
    }

    pub fn contains(&self, id: u32) -> bool {
        id != 0 && self.slots.contains(&id)
    }

    pub fn occupied_len(&self) -> usize {
        self.slots.iter().filter(|&&id| id != 0).count()
    }

    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.slots.iter().copied().filter(|&id| id != 0)
    }

    /// Zero slots that fail `keep`. Next insert reuses the first hole
    /// (`KNOWNCREATURE_FREE`) without a visibility walk.
    pub fn retain(&mut self, mut keep: impl FnMut(u32) -> bool) {
        for slot in &mut self.slots {
            if *slot != 0 && !keep(*slot) {
                *slot = 0;
            }
        }
    }

    /// Occupy the first free slot, growing if every slot is filled. Tests / empty-table seeds.
    pub fn insert(&mut self, id: u32) -> bool {
        if id == 0 || self.contains(id) {
            return false;
        }
        if let Some(slot) = self.slots.iter_mut().find(|s| **s == 0) {
            *slot = id;
            return true;
        }
        self.slots.push(id);
        true
    }
}

/// `TConnection::KnownCreature` then `NewKnownCreature` (`connections.cc:380-454`).
///
/// Walks from slot 0 and stops: already known; first free (`0`); first `!can_see`.
/// A full table of visible creatures is not mutated (`KnownCreatureTable ausgelastet`).
pub fn check_creature_known<F: FnMut(u32) -> bool>(
    id: u32,
    known: &mut KnownCreatureTable,
    can_see_creature: &mut F,
    limit: usize,
) -> (bool, u32) {
    if id == 0 {
        return (false, 0);
    }
    known.ensure_len(limit);
    let n = known.slots.len().min(limit);

    for i in 0..n {
        if known.slots[i] == id {
            return (true, 0);
        }
    }

    for i in 0..n {
        if known.slots[i] == 0 {
            known.slots[i] = id;
            return (false, 0);
        }
    }

    for i in 0..n {
        let old = known.slots[i];
        if !can_see_creature(old) {
            known.slots[i] = id;
            return (false, old);
        }
    }

    (false, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_limit_no_remove() {
        let mut known = KnownCreatureTable::default();
        let mut see_calls = 0u32;
        let mut see = |_| {
            see_calls += 1;
            true
        };
        let (known_flag, remove) = check_creature_known(1, &mut known, &mut see, 150);
        assert!(!known_flag);
        assert_eq!(remove, 0);
        assert_eq!(known.occupied_len(), 1);
        assert_eq!(see_calls, 0);
        assert!(known.contains(1));
    }

    #[test]
    fn already_known_returns_known() {
        let mut known = KnownCreatureTable::default();
        known.insert(42);
        let mut see_calls = 0u32;
        let mut see = |_| {
            see_calls += 1;
            true
        };
        let (known_flag, remove) = check_creature_known(42, &mut known, &mut see, 150);
        assert!(known_flag);
        assert_eq!(remove, 0);
        assert_eq!(known.occupied_len(), 1);
        assert_eq!(see_calls, 0);
    }

    #[test]
    fn over_limit_evicts_first_unseen() {
        let mut known = KnownCreatureTable::with_limit(150);
        known.insert(10);
        known.insert(7);
        for id in 11..=158 {
            known.insert(id);
        }
        assert_eq!(known.occupied_len(), 150);
        let mut see_calls = 0u32;
        let mut see = |id: u32| {
            see_calls += 1;
            id != 7
        };
        let (known_flag, remove) = check_creature_known(999, &mut known, &mut see, 150);
        assert!(!known_flag);
        assert_eq!(remove, 7);
        assert_eq!(see_calls, 2);
        assert_eq!(known.occupied_len(), 150);
        assert!(known.contains(999));
        assert!(!known.contains(7));
        assert!(known.contains(10));
    }

    #[test]
    fn full_visible_table_does_not_insert() {
        let mut known = KnownCreatureTable::with_limit(150);
        for id in 1..=150 {
            known.insert(id);
        }
        let mut see_calls = 0u32;
        let mut see = |_| {
            see_calls += 1;
            true
        };
        let (known_flag, remove) = check_creature_known(999, &mut known, &mut see, 150);
        assert!(!known_flag);
        assert_eq!(remove, 0);
        assert_eq!(see_calls, 150);
        assert_eq!(known.occupied_len(), 150);
        assert!(!known.contains(999));
        assert!(known.contains(1));
    }

    #[test]
    fn retain_zeros_holes_next_insert_skips_can_see() {
        let mut known = KnownCreatureTable::with_limit(150);
        known.insert(1);
        known.insert(2);
        known.retain(|id| id == 2);
        let mut see_calls = 0u32;
        let mut see = |_| {
            see_calls += 1;
            true
        };
        let (known_flag, remove) = check_creature_known(3, &mut known, &mut see, 150);
        assert!(!known_flag);
        assert_eq!(remove, 0);
        assert_eq!(see_calls, 0);
        assert!(known.contains(2));
        assert!(known.contains(3));
        assert!(!known.contains(1));
    }
}
