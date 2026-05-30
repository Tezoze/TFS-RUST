//! Shared `knownCreatureSet` eviction — C++ `ProtocolGame::checkCreatureAsKnown`.
// C++ reference: `src/protocolgame.cpp` ~744–776.

use std::collections::HashSet;

/// C++ `ProtocolGame::checkCreatureAsKnown` (`src/protocolgame.cpp` ~744–776).
pub fn check_creature_known<F: FnMut(u32) -> bool>(
    id: u32,
    known_set: &mut HashSet<u32>,
    can_see_creature: &mut F,
) -> (bool, u32) {
    if !known_set.insert(id) {
        return (true, 0);
    }

    if known_set.len() <= 1300 {
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
