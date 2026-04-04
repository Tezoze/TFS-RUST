//! Monster/NPC spawn scheduling from loaded spawn XML + OTBM references.
// C++ reference: `spawns.cpp` `Spawn::checkSpawn`, `SpawnMonster`.

use std::time::Instant;

use tfs_rust_content::spawns::SpawnZone;

#[derive(Debug)]
pub struct SpawnManager {
    pub zones: Vec<SpawnZone>,
    /// Last respawn bookkeeping (expanded in Phase 5 with creature ids).
    pub last_check: Option<Instant>,
}

impl SpawnManager {
    pub fn from_zones(zones: Vec<SpawnZone>) -> Self {
        Self {
            zones,
            last_check: None,
        }
    }

    pub fn tick(&mut self, _now: Instant) {
        self.last_check = Some(_now);
        // Phase 5: spawn / respawn monsters from `zones`.
    }
}
