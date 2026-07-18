//! Item decay scheduling (per-tick).
//!
//! Domain: TFS `Game::startDecay` / `Game::checkDecay` (`game.cpp`).
//! Outcomes: decompile `CronExpire` / `ProcessCronSystem` (`map.cc` / `operate.cc`) —
//! XML `duration` is seconds → deadline on `server_ms`.

use crate::ids::ItemId;
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

    /// Remaining ms until deadline, if the item is scheduled.
    pub fn remaining_ms(&self, id: ItemId, now_ms: u64) -> Option<u64> {
        self.entries
            .get(&id)
            .map(|e| e.deadline_tick.saturating_sub(now_ms))
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
    }
}
