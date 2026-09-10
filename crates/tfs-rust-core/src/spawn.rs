//! Monster/NPC spawn scheduling — serial per-(zone, race) home timer.
//! Pack surface: TFS `spawns.xml` slots. Corpus: `crnonpl.cc` `ProcessMonsterhomes`
//! / `StartMonsterhomeTimer` / `NotifyMonsterhomeOfDeath` (`:1296-1512`).

use std::collections::HashMap;

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
    /// Index into [`SpawnManager::homes`]; `None` for NPC occupancy-only slots.
    pub home_index: Option<usize>,
}

/// Per-(zone, race) serial respawn timer (`crnonpl.cc:1510-1512` `StartMonsterhomeTimer`).
#[derive(Debug, Clone)]
pub struct MonsterHome {
    pub slot_indices: Vec<usize>,
    pub max_monsters: u32,
    pub act_monsters: u32,
    pub timer_at: Option<u32>,
    pub spawntime_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum HomeKey {
    Named {
        zone_index: usize,
        name: String,
    },
    Weighted {
        zone_index: usize,
        entry_index: usize,
    },
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
    pub homes: Vec<MonsterHome>,
    /// GCD of slot spawntimes in XML ms — kept for pack diagnostics; 772 polls every Other.
    pub check_interval_ms: u64,
    /// Last Other RoundNr that scanned slots.
    pub last_check: Option<u32>,
    pub started: bool,
}

/// XML / TFS `spawntime` is stored as ms; 772 Timer ticks once per Other (`ms/1000` rounds).
pub fn ms_to_rounds(ms: u64) -> u32 {
    if ms == 0 {
        return 0;
    }
    u32::try_from(ms / 1000).unwrap_or(u32::MAX).max(1)
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

fn group_monster_homes(slots: &mut [SpawnSlot]) -> Vec<MonsterHome> {
    let mut order: Vec<HomeKey> = Vec::new();
    let mut grouped: HashMap<HomeKey, Vec<usize>> = HashMap::new();
    for (idx, slot) in slots.iter().enumerate() {
        if !slot.respawns {
            continue;
        }
        let key = match &slot.entry {
            SpawnEntryKind::Monster { name } => HomeKey::Named {
                zone_index: slot.zone_index,
                name: name.clone(),
            },
            SpawnEntryKind::Monsters { .. } => HomeKey::Weighted {
                zone_index: slot.zone_index,
                entry_index: slot.entry_index,
            },
            SpawnEntryKind::Npc { .. } => continue,
        };
        if !grouped.contains_key(&key) {
            order.push(key.clone());
        }
        grouped.entry(key).or_default().push(idx);
    }
    let mut homes = Vec::with_capacity(order.len());
    for key in order {
        let indices = grouped.remove(&key).unwrap_or_default();
        let spawntime_ms = indices
            .first()
            .and_then(|&i| slots.get(i))
            .map(|s| s.spawntime_ms)
            .unwrap_or(1000);
        let home_index = homes.len();
        for &i in &indices {
            if let Some(slot) = slots.get_mut(i) {
                slot.home_index = Some(home_index);
            }
        }
        homes.push(MonsterHome {
            max_monsters: indices.len() as u32,
            slot_indices: indices,
            act_monsters: 0,
            timer_at: None,
            spawntime_ms,
        });
    }
    homes
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
                    home_index: None,
                });
            }
        }

        if check_interval_ms == 0 {
            check_interval_ms = 60_000;
        }

        let homes = group_monster_homes(&mut slots);
        tracing::info!(
            homes = homes.len(),
            slots = slots.len(),
            "spawn homes vs slots"
        );

        Self {
            zones,
            slots,
            homes,
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

    /// Homes whose serial timer has expired and that still have a free slot.
    pub fn due_home_indices(&self, now_round: u32) -> Vec<usize> {
        self.homes
            .iter()
            .enumerate()
            .filter(|(_, h)| h.act_monsters < h.max_monsters)
            .filter(|(_, h)| h.timer_at.is_some_and(|at| now_round >= at))
            .map(|(i, _)| i)
            .collect()
    }

    /// First empty slot in a home (one placement per expiry).
    pub fn empty_slot_in_home(&self, home_index: usize) -> Option<usize> {
        let home = self.homes.get(home_index)?;
        home.slot_indices
            .iter()
            .copied()
            .find(|&i| self.slots.get(i).is_some_and(|s| s.current.is_none()))
    }

    /// C++ `ProcessMonsterhomes` — empty respawning slots whose Timer has reached 0.
    /// Production poll uses [`Self::due_home_indices`]; this helper remains for tests.
    pub fn due_slot_indices(&self, now_round: u32) -> Vec<usize> {
        self.due_home_indices(now_round)
            .into_iter()
            .filter_map(|hi| self.empty_slot_in_home(hi))
            .collect()
    }

    /// 772 scans every Other; GCD interval is TFS `checkSpawn` only.
    pub fn should_run_check(&self, _now_round: u32) -> bool {
        true
    }

    pub fn mark_checked(&mut self, now_round: u32) {
        self.last_check = Some(now_round);
    }

    /// Arm (or reset) a home timer. Call sites pass [`GameWorld::compute_respawn_delay_ms`].
    pub fn arm_home(&mut self, home_index: usize, now_round: u32, delay_rounds: u32) {
        if let Some(home) = self.homes.get_mut(home_index) {
            home.timer_at = Some(now_round.saturating_add(delay_rounds));
        }
    }

    pub fn clear_home_timer(&mut self, home_index: usize) {
        if let Some(home) = self.homes.get_mut(home_index) {
            home.timer_at = None;
        }
    }

    /// Failed placement / Block-mode stall — `StartMonsterhomeTimer` on the slot's home.
    pub fn stall_respawn(&mut self, slot_index: usize, now_round: u32, delay_rounds: u32) {
        let Some(home_index) = self.slots.get(slot_index).and_then(|s| s.home_index) else {
            return;
        };
        self.arm_home(home_index, now_round, delay_rounds);
    }

    /// C++ `ProcessMonsterhomes` due scan. `now_round` is RoundNr.
    pub fn due_spawns<F>(&mut self, now_round: u32, find_player: F) -> Vec<SpawnRequest>
    where
        F: Fn(Position) -> bool,
    {
        if !self.should_run_check(now_round) {
            return Vec::new();
        }
        self.mark_checked(now_round);

        let mut out = Vec::new();
        for home_index in self.due_home_indices(now_round) {
            let Some(slot_index) = self.empty_slot_in_home(home_index) else {
                continue;
            };
            let Some(slot) = self.slots.get(slot_index) else {
                continue;
            };
            if find_player(slot.position) {
                let delay = ms_to_rounds(slot.spawntime_ms);
                self.arm_home(home_index, now_round, delay);
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
        let home_index = self.slots.get(slot_index).and_then(|s| s.home_index);
        if let Some(slot) = self.slots.get_mut(slot_index) {
            let was_empty = slot.current.is_none();
            slot.current = Some(cid);
            if was_empty
                && let Some(hi) = home_index
                && let Some(home) = self.homes.get_mut(hi)
            {
                home.act_monsters = home.act_monsters.saturating_add(1);
                if home.act_monsters >= home.max_monsters {
                    home.timer_at = None;
                }
            }
        }
    }

    /// Schedule respawn when spawn-linked creature is removed.
    /// `now_round` / `delay_rounds` are RoundNr (`StartMonsterhomeTimer`, `crnonpl.cc:1296`).
    /// Arms **only if** `timer_at.is_none()` (`crnonpl.cc:1510-1512`).
    pub fn on_creature_removed(&mut self, slot_index: usize, now_round: u32, delay_rounds: u32) {
        let home_index = self.slots.get(slot_index).and_then(|s| s.home_index);
        let respawns = self.slots.get(slot_index).is_some_and(|s| s.respawns);
        if let Some(slot) = self.slots.get_mut(slot_index) {
            slot.current = None;
        }
        let Some(hi) = home_index else {
            return;
        };
        if let Some(home) = self.homes.get_mut(hi) {
            home.act_monsters = home.act_monsters.saturating_sub(1);
            if respawns && home.timer_at.is_none() {
                home.timer_at = Some(now_round.saturating_add(delay_rounds));
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

    pub fn count_occupied_in_home(&self, home_index: usize) -> usize {
        self.homes
            .get(home_index)
            .map(|h| h.act_monsters as usize)
            .unwrap_or(0)
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

    fn three_rat_zone() -> SpawnZone {
        SpawnZone {
            center: Position::new(100, 100, 7),
            radius: 5,
            entries: (0..3)
                .map(|i| SpawnEntry::Monster {
                    name: "Rat".into(),
                    position: Position::new(101 + i, 101, 7),
                    spawntime_ms: 60_000,
                    direction: None,
                })
                .collect(),
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
        let t0: u32 = 0;
        mgr.on_creature_removed(0, t0, 60);
        assert!(mgr.due_spawns(t0, |_| false).is_empty());
        let later = t0 + 61;
        let reqs = mgr.due_spawns(later, |_| false);
        assert!(!reqs.is_empty());

        mgr.on_creature_removed(0, t0, 60);
        mgr.due_spawns(t0 + 61, |_| true);
        let home = &mgr.homes[0];
        assert!(home.timer_at.is_some());
    }

    #[test]
    fn mixed_race_zone_splits_into_homes() {
        let zone = SpawnZone {
            center: Position::new(100, 100, 7),
            radius: 5,
            entries: vec![
                SpawnEntry::Monster {
                    name: "Rat".into(),
                    position: Position::new(101, 101, 7),
                    spawntime_ms: 60_000,
                    direction: None,
                },
                SpawnEntry::Monster {
                    name: "Cave Rat".into(),
                    position: Position::new(102, 101, 7),
                    spawntime_ms: 60_000,
                    direction: None,
                },
            ],
        };
        let mgr = SpawnManager::from_zones(vec![zone]);
        assert_eq!(mgr.homes.len(), 2);
        assert_eq!(mgr.homes[0].max_monsters, 1);
        assert_eq!(mgr.homes[1].max_monsters, 1);
    }

    #[test]
    fn three_identical_monsters_share_one_home() {
        let mgr = SpawnManager::from_zones(vec![three_rat_zone()]);
        assert_eq!(mgr.slots.len(), 3);
        assert_eq!(mgr.homes.len(), 1);
        assert_eq!(mgr.homes[0].max_monsters, 3);
    }

    #[test]
    fn wiped_three_slot_home_refills_one_per_cycle() {
        let mut mgr = SpawnManager::from_zones(vec![three_rat_zone()]);
        mgr.on_creature_spawned(0, CreatureId::default());
        mgr.on_creature_spawned(1, CreatureId::default());
        mgr.on_creature_spawned(2, CreatureId::default());
        mgr.on_creature_removed(0, 10, 60);
        mgr.on_creature_removed(1, 11, 60);
        mgr.on_creature_removed(2, 12, 60);
        assert_eq!(
            mgr.due_slot_indices(70).len(),
            1,
            "one placement per expiry"
        );
        mgr.on_creature_spawned(0, CreatureId::default());
        mgr.arm_home(0, 70, 60);
        assert!(
            mgr.due_slot_indices(70).is_empty(),
            "re-arm after one spawn must not refill the rest this round"
        );
        assert_eq!(mgr.homes[0].act_monsters, 1);
    }

    #[test]
    fn second_death_does_not_rearm_running_timer() {
        let mut mgr = SpawnManager::from_zones(vec![three_rat_zone()]);
        mgr.on_creature_spawned(0, CreatureId::default());
        mgr.on_creature_spawned(1, CreatureId::default());
        mgr.on_creature_spawned(2, CreatureId::default());
        mgr.on_creature_removed(0, 10, 60);
        let first = mgr.homes[0].timer_at;
        assert!(first.is_some());
        mgr.on_creature_removed(1, 15, 60);
        assert_eq!(mgr.homes[0].timer_at, first, "second death must not re-arm");
        assert_eq!(mgr.homes[0].act_monsters, 1);
    }
}
