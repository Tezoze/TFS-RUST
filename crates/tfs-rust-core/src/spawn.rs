//! Monster/NPC spawn scheduling from loaded spawn XML + OTBM references.
// C++ reference: `spawn.cpp` `Spawn::checkSpawn`, `Spawn::spawnMonster`, `Spawn::startup`, `Spawn::findPlayer`.

use rand::RngExt;
use tfs_rust_common::Position;
use tfs_rust_content::spawns::{MonsterWeight, SpawnEntry, SpawnZone};

use crate::ids::CreatureId;

/// One spawn block from XML — stored per slot (weighted lists rolled at spawn time).
#[derive(Debug, Clone)]
pub enum SpawnEntryKind {
    Monster { name: String },
    Monsters { weights: Vec<MonsterWeight> },
    Npc { name: String },
}

#[derive(Debug, Clone)]
pub struct SpawnSlot {
    pub zone_index: usize,
    pub entry_index: usize,
    pub position: Position,
    /// C++ spawn block radius (`spawn.cpp` / TVP `TvpSpawn`); `-1` → search distance 1.
    pub radius: i32,
    pub spawntime_ms: u64,
    pub direction: Option<u16>,
    pub entry: SpawnEntryKind,
    /// `false` for NPC spawn entries (TFS NPCs do not respawn on timers).
    pub respawns: bool,
    /// Live creature occupying this slot, if any.
    pub current: Option<CreatureId>,
    /// Earliest **logical ms** (`server_ms` on 772) this slot may respawn (set on death).
    /// C++ 772: `ProcessMonsterhomes` counts respawn delay in logical rounds, not wall time
    /// (audit Finding 13). Was `Instant`; now on the logical clock so it matches the beat loop.
    pub respawn_at: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct SpawnRequest {
    pub slot_index: usize,
    /// Resolved monster name for monster slots; `None` for NPC slots.
    pub monster_name: Option<String>,
    pub startup: bool,
}

#[derive(Debug)]
pub struct SpawnManager {
    pub zones: Vec<SpawnZone>,
    pub slots: Vec<SpawnSlot>,
    /// GCD of slot spawntimes — C++ `Spawn::interval` (`spawn.cpp` ~409).
    pub check_interval_ms: u64,
    /// Last respawn-check time in **logical ms** (audit Finding 13).
    pub last_check: Option<u64>,
    pub started: bool,
}

fn gcd_u64(a: u64, b: u64) -> u64 {
    if a == 0 {
        return b;
    }
    if b == 0 {
        return a;
    }
    let mut x = a;
    let mut y = b;
    while y != 0 {
        let r = x % y;
        x = y;
        y = r;
    }
    x
}

impl SpawnManager {
    /// Build slots from loaded zones (one slot per spawn XML entry).
    pub fn from_zones(zones: Vec<SpawnZone>) -> Self {
        let mut slots = Vec::new();
        let mut check_interval_ms = 0u64;

        for (zone_index, zone) in zones.iter().enumerate() {
            for (entry_index, entry) in zone.entries.iter().enumerate() {
                let (position, spawntime_ms, direction, kind, respawns) = match entry {
                    SpawnEntry::Monster {
                        name,
                        position,
                        spawntime_ms,
                        direction,
                    } => (
                        *position,
                        (*spawntime_ms).max(0) as u64,
                        *direction,
                        SpawnEntryKind::Monster { name: name.clone() },
                        true,
                    ),
                    SpawnEntry::Monsters {
                        position,
                        spawntime_ms,
                        monsters,
                    } => (
                        *position,
                        (*spawntime_ms).max(0) as u64,
                        None,
                        SpawnEntryKind::Monsters {
                            weights: monsters.clone(),
                        },
                        true,
                    ),
                    SpawnEntry::Npc {
                        name,
                        position,
                        spawntime_ms,
                        direction,
                    } => (
                        *position,
                        (*spawntime_ms).max(0) as u64,
                        *direction,
                        SpawnEntryKind::Npc { name: name.clone() },
                        false,
                    ),
                };

                if spawntime_ms > 0 {
                    check_interval_ms = if check_interval_ms == 0 {
                        spawntime_ms
                    } else {
                        gcd_u64(check_interval_ms, spawntime_ms)
                    };
                }

                slots.push(SpawnSlot {
                    zone_index,
                    entry_index,
                    position,
                    radius: zone.radius,
                    spawntime_ms: spawntime_ms.max(1000),
                    direction,
                    entry: kind,
                    respawns,
                    current: None,
                    respawn_at: None,
                });
            }
        }

        if check_interval_ms == 0 {
            check_interval_ms = 60_000;
        }

        Self {
            zones,
            slots,
            check_interval_ms,
            last_check: None,
            started: false,
        }
    }

    /// C++ `Spawn::startup` — force-spawn every empty slot (`spawn.cpp` ~344).
    pub fn startup_requests(&self) -> Vec<SpawnRequest> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.current.is_none())
            .filter_map(|(slot_index, slot)| build_spawn_request(slot_index, slot, true))
            .collect()
    }

    /// C++ `Spawn::checkSpawn` — slots due for respawn (`spawn.cpp` ~353). `now_ms` is logical.
    pub fn due_slot_indices(&self, now_ms: u64) -> Vec<usize> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.current.is_none() && slot.respawns)
            .filter(|(_, slot)| slot.respawn_at.is_none_or(|at| now_ms >= at))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn should_run_check(&self, now_ms: u64) -> bool {
        match self.last_check {
            Some(last) => now_ms.saturating_sub(last) >= self.check_interval_ms,
            None => true,
        }
    }

    pub fn mark_checked(&mut self, now_ms: u64) {
        self.last_check = Some(now_ms);
    }

    /// C++ `StartMonsterhomeTimer` — resets the spawn timer to a fresh `spawntime` cycle
    /// (`crnonpl.cc:1296-1323`). Called when placement fails (player nearby, blocked tile) or
    /// when a player stands on the spawn tile (TFS `Block` mode). The C++ uses
    /// `random(spawntime/2, spawntime)`; we use the full `spawntime` as the base (the random
    /// jitter is a minor detail — the key is NOT retrying immediately, which caused the
    /// per-second WARN spam when a player stood near a spawn point after killing the monster).
    pub fn stall_respawn(&mut self, slot_index: usize, now_ms: u64) {
        if let Some(slot) = self.slots.get_mut(slot_index) {
            slot.respawn_at = Some(now_ms.saturating_add(slot.spawntime_ms));
        }
    }

    /// C++ `Spawn::checkSpawn` — slots due for respawn (`spawn.cpp` ~353). `now_ms` is logical.
    pub fn due_spawns<F>(&mut self, now_ms: u64, find_player: F) -> Vec<SpawnRequest>
    where
        F: Fn(Position) -> bool,
    {
        if !self.should_run_check(now_ms) {
            return Vec::new();
        }
        self.mark_checked(now_ms);

        let mut out = Vec::new();
        for slot_index in self.due_slot_indices(now_ms) {
            let Some(slot) = self.slots.get(slot_index) else {
                continue;
            };
            if find_player(slot.position) {
                self.stall_respawn(slot_index, now_ms);
                continue;
            }
            if let Some(req) = build_spawn_request(slot_index, slot, false) {
                out.push(req);
            }
        }
        out
    }

    /// C++ `spawnedMap` insert — link live creature to slot.
    pub fn on_creature_spawned(&mut self, slot_index: usize, cid: CreatureId) {
        if let Some(slot) = self.slots.get_mut(slot_index) {
            slot.current = Some(cid);
            slot.respawn_at = None;
        }
    }

    /// Schedule respawn when spawn-linked creature is removed. `now_ms` is logical (audit Finding 13).
    /// `delay_ms` is the respawn delay — fixed `spawntime_ms` on 1098, or the 772
    /// `StartMonsterhomeTimer` draw (`random(regen/2, regen)` scaled by player count) computed by
    /// [`GameWorld::compute_respawn_delay_ms`] (`crnonpl.cc:1296`).
    pub fn on_creature_removed(&mut self, slot_index: usize, now_ms: u64, delay_ms: u64) {
        if let Some(slot) = self.slots.get_mut(slot_index) {
            slot.current = None;
            if slot.respawns {
                slot.respawn_at = Some(now_ms.saturating_add(delay_ms));
            }
        }
    }

    pub fn slot_for_creature(&self, cid: CreatureId) -> Option<usize> {
        self.slots
            .iter()
            .enumerate()
            .find(|(_, s)| s.current == Some(cid))
            .map(|(i, _)| i)
    }

    pub fn count_occupied_in_zone(&self, zone_index: usize) -> usize {
        self.slots
            .iter()
            .filter(|s| s.zone_index == zone_index && s.current.is_some())
            .count()
    }

    pub fn zone_center(&self, zone_index: usize) -> Option<Position> {
        self.zones.get(zone_index).map(|z| z.center)
    }

    pub fn slot(&self, index: usize) -> Option<&SpawnSlot> {
        self.slots.get(index)
    }
}

pub(crate) fn build_spawn_request(
    slot_index: usize,
    slot: &SpawnSlot,
    startup: bool,
) -> Option<SpawnRequest> {
    match &slot.entry {
        SpawnEntryKind::Monster { name } => Some(SpawnRequest {
            slot_index,
            monster_name: Some(name.clone()),
            startup,
        }),
        SpawnEntryKind::Monsters { weights } => {
            let name = pick_weighted_monster(weights)?;
            Some(SpawnRequest {
                slot_index,
                monster_name: Some(name),
                startup,
            })
        }
        SpawnEntryKind::Npc { .. } => Some(SpawnRequest {
            slot_index,
            monster_name: None,
            startup,
        }),
    }
}

/// C++ `spawnMonster(sb)` weighted roll (`spawn.cpp` ~276–311).
pub fn pick_weighted_monster(weights: &[MonsterWeight]) -> Option<String> {
    if weights.is_empty() {
        return None;
    }
    if weights.len() == 1 {
        return Some(weights[0].name.clone());
    }
    let mut rng = rand::rng();
    for w in weights {
        let roll: u16 = rng.random_range(1..=100);
        if w.chance >= roll {
            return Some(w.name.clone());
        }
    }
    // Fallback without chance check — C++ second `spawnFunc(false)` pass.
    Some(weights[0].name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfs_rust_content::spawns::SpawnZone;

    fn sample_zone() -> SpawnZone {
        SpawnZone {
            center: Position::new(100, 100, 7),
            radius: 5,
            entries: vec![
                SpawnEntry::Monster {
                    name: "Rat".into(),
                    position: Position::new(101, 101, 7),
                    spawntime_ms: 60_000,
                    direction: Some(2),
                },
                SpawnEntry::Npc {
                    name: "Tom".into(),
                    position: Position::new(102, 102, 7),
                    spawntime_ms: 60_000,
                    direction: None,
                },
            ],
        }
    }

    #[test]
    fn from_zones_builds_slots() {
        let mgr = SpawnManager::from_zones(vec![sample_zone()]);
        assert_eq!(mgr.slots.len(), 2);
        assert!(mgr.slots[0].respawns);
        assert!(!mgr.slots[1].respawns);
        assert_eq!(mgr.check_interval_ms, 60_000);
    }

    #[test]
    fn startup_requests_cover_empty_slots() {
        let mgr = SpawnManager::from_zones(vec![sample_zone()]);
        let reqs = mgr.startup_requests();
        assert_eq!(reqs.len(), 2);
        assert!(reqs.iter().all(|r| r.startup));
    }

    #[test]
    fn due_spawns_respects_timer_and_find_player() {
        let mut mgr = SpawnManager::from_zones(vec![sample_zone()]);
        let t0: u64 = 0;
        mgr.on_creature_removed(0, t0, 60_000);
        assert!(mgr.due_spawns(t0, |_| false).is_empty());
        let later = t0 + 61_000;
        let reqs = mgr.due_spawns(later, |_| false);
        assert!(!reqs.is_empty());

        mgr.on_creature_removed(0, t0, 60_000);
        mgr.due_spawns(t0 + 61_000, |_| true);
        let slot = &mgr.slots[0];
        assert!(slot.respawn_at.is_some());
    }
}
