//! Item decay scheduling (per-tick).
// C++ reference: `items.cpp` decay / `Game::checkDecay`.

use crate::ids::ItemId;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DecayEntry {
    /// Game tick index when the item should transform / vanish.
    pub deadline_tick: u64,
    pub replace_with: Option<u16>,
}

#[derive(Debug, Default)]
pub struct DecayManager {
    entries: HashMap<ItemId, DecayEntry>,
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
    }

    pub fn cancel(&mut self, id: ItemId) {
        self.entries.remove(&id);
    }

    /// Run after other per-tick work; returns items that expired this tick.
    pub fn tick(&mut self, now: u64) -> Vec<(ItemId, DecayEntry)> {
        let mut done = Vec::new();
        self.entries.retain(|id, e| {
            if e.deadline_tick <= now {
                done.push((*id, e.clone()));
                false
            } else {
                true
            }
        });
        done
    }
}
