//! Shared `knownCreatureSet` eviction — C++ `ProtocolGame::checkCreatureAsKnown`.
//!
//! - 772 outcomes / wire: TVP `protocolgame.cpp` `checkCreatureAsKnown` (`size() > 150`);
//!   CipSoft `KnownCreatureTable[150]` (`connections.hh`).
//! - 1098 domain: repo-root TFS `protocolgame.cpp` (`size() > 1300`).

use std::collections::HashSet;

/// C++ `ProtocolGame::checkCreatureAsKnown`.
///
/// `limit` is `ProtocolCaps::known_creature_limit` (150 for 772, 1300 for 1098). Evicts when
/// `known_set.len() > limit` after insert — same `>` semantics as TVP/TFS.
pub fn check_creature_known<F: FnMut(u32) -> bool>(
    id: u32,
    known_set: &mut HashSet<u32>,
    can_see_creature: &mut F,
    limit: usize,
) -> (bool, u32) {
    if !known_set.insert(id) {
        return (true, 0);
    }

    if known_set.len() <= limit {
        return (false, 0);
    }

    let mut others: Vec<u32> = known_set.iter().copied().filter(|&k| k != id).collect();
    others.sort_unstable();

    for cid in &others {
        if !can_see_creature(*cid) {
            known_set.remove(cid);
            return (false, *cid);
        }
    }

    if let Some(first) = others.first() {
        let removed = *first;
        known_set.remove(&removed);
        return (false, removed);
    }

    (false, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_limit_no_remove() {
        let mut known = HashSet::new();
        let mut see = |_| true;
        let (known_flag, remove) = check_creature_known(1, &mut known, &mut see, 150);
        assert!(!known_flag);
        assert_eq!(remove, 0);
        assert_eq!(known.len(), 1);
    }

    #[test]
    fn already_known_returns_known() {
        let mut known = HashSet::from([42]);
        let mut see = |_| true;
        let (known_flag, remove) = check_creature_known(42, &mut known, &mut see, 150);
        assert!(known_flag);
        assert_eq!(remove, 0);
        assert_eq!(known.len(), 1);
    }

    #[test]
    fn over_limit_150_evicts_unseen() {
        let mut known: HashSet<u32> = (1..=150).collect();
        let mut see = |id: u32| id != 7;
        let (known_flag, remove) = check_creature_known(999, &mut known, &mut see, 150);
        assert!(!known_flag);
        assert_eq!(remove, 7);
        assert_eq!(known.len(), 150);
        assert!(known.contains(&999));
        assert!(!known.contains(&7));
    }

    #[test]
    fn over_limit_150_evicts_lowest_when_all_visible() {
        let mut known: HashSet<u32> = (1..=150).collect();
        let mut see = |_| true;
        let (known_flag, remove) = check_creature_known(999, &mut known, &mut see, 150);
        assert!(!known_flag);
        assert_eq!(remove, 1);
        assert_eq!(known.len(), 150);
        assert!(known.contains(&999));
        assert!(!known.contains(&1));
    }
}
