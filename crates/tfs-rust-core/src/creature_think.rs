//! Creature think cadence — TFS `Game::checkCreatures` → `Creature::onThink` dispatch.
//!
//! - `Game::checkCreatures` — `game.cpp` (~3819).
//! - `Creature::onThink` — `creature.cpp` (~123).
//! - `Creature::onAttacking` / `Monster::doAttacking` — `creature.cpp` (~172), `monster.cpp` (~806).
//! - `Monster::onThink` / `Npc::onThink` — `monster.cpp` (~732), `npc.cpp` (~606).

use std::time::{Duration, Instant};

use rand::Rng;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// TFS `creature.h` `EVENT_CREATURE_THINK_INTERVAL`.
pub const EVENT_CREATURE_THINK_INTERVAL_MS: u32 = 1000;

/// TFS `creature.h` `EVENT_CREATURECOUNT` — bucket count for staggered checks.
pub const EVENT_CREATURECOUNT: u32 = 10;

/// TFS `creature.h` `EVENT_CHECK_CREATURE_INTERVAL` = think interval / bucket count.
pub const EVENT_CHECK_CREATURE_INTERVAL_MS: u32 =
    EVENT_CREATURE_THINK_INTERVAL_MS / EVENT_CREATURECOUNT;

/// Ms between follow path recomputes when following (`creature.cpp` ~153).
const FOLLOW_PATH_UPDATE_INTERVAL_MS: u32 = 200;

impl GameWorld {
    /// TFS `Game::addCreatureCheck` — random bucket assignment (`game.cpp` ~3798).
    pub(crate) fn add_creature_think_check(&mut self, cid: CreatureId) {
        let Some(k) = self.creatures.get_mut(cid) else {
            return;
        };
        if k.base().health <= 0 {
            return;
        }
        let bucket = rand::thread_rng().gen_range(0..EVENT_CREATURECOUNT) as u8;
        k.base_mut().think_check_bucket = Some(bucket);
    }

    /// TFS `Game::removeCreatureCheck` — idle / removed creatures skip think sweeps.
    pub(crate) fn remove_creature_think_check(&mut self, cid: CreatureId) {
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().think_check_bucket = None;
        }
    }

    #[cfg(test)]
    pub(crate) fn set_creature_think_check_bucket(&mut self, cid: CreatureId, bucket: u8) {
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().think_check_bucket = Some(bucket % EVENT_CREATURECOUNT as u8);
        }
    }

    /// TFS `Game::checkCreatures` — one bucket every 100 ms, full cycle 1 s (`game.cpp` ~3819).
    pub fn check_creatures(&mut self, now: Instant) {
        let Some(last) = self.last_creature_bucket_tick else {
            self.last_creature_bucket_tick = Some(now);
            return;
        };

        if now.duration_since(last)
            < Duration::from_millis(u64::from(EVENT_CHECK_CREATURE_INTERVAL_MS))
        {
            return;
        }

        self.last_creature_bucket_tick = Some(now);

        let bucket = self.check_creature_bucket_index;
        self.check_creature_bucket_index =
            (self.check_creature_bucket_index + 1) % EVENT_CREATURECOUNT;

        let interval_ms = EVENT_CREATURE_THINK_INTERVAL_MS;
        let bucket_u8 = bucket as u8;

        let ids: Vec<CreatureId> = self
            .creatures
            .iter()
            .filter(|(_, k)| {
                matches!(k, CreatureKind::Monster(_) | CreatureKind::Npc(_))
                    && k.base().think_check_bucket == Some(bucket_u8)
            })
            .map(|(id, _)| id)
            .collect();

        for cid in ids {
            if !self.creature_alive_for_think(cid) {
                continue;
            }

            match self.creatures.get(cid) {
                Some(CreatureKind::Monster(_)) => {
                    self.monster_on_think(cid, interval_ms);
                    // C++ `checkCreatures`: `onAttacking` after `onThink` (`game.cpp` ~3837–3840).
                    if self.creature_alive_for_think(cid) {
                        self.creature_on_attacking(cid, interval_ms);
                    }
                }
                Some(CreatureKind::Npc(_)) => self.npc_on_think(cid, interval_ms),
                _ => continue,
            }
        }
    }

    /// 772 `ProcessCreatures` — full monster/NPC sweep ~1 Hz (`main.cc` `AdvanceGame`).
    pub fn process_creatures_772(&mut self) {
        let interval_ms = EVENT_CREATURE_THINK_INTERVAL_MS;
        let ids: Vec<CreatureId> = self
            .creatures
            .iter()
            .filter(|(_, k)| {
                matches!(k, CreatureKind::Monster(_) | CreatureKind::Npc(_))
                    && k.base().think_check_bucket.is_some()
            })
            .map(|(id, _)| id)
            .collect();

        for cid in ids {
            if !self.creature_alive_for_think(cid) {
                continue;
            }

            match self.creatures.get(cid) {
                Some(CreatureKind::Monster(_)) => {
                    self.monster_on_think(cid, interval_ms);
                    if self.creature_alive_for_think(cid) {
                        self.creature_on_attacking(cid, interval_ms);
                    }
                }
                Some(CreatureKind::Npc(_)) => self.npc_on_think(cid, interval_ms),
                _ => continue,
            }
        }
    }

    /// Whether `cid` should receive `onThink` this sweep (C++ `getHealth() > 0` gate).
    fn creature_alive_for_think(&self, cid: CreatureId) -> bool {
        self.creatures
            .get(cid)
            .is_some_and(|k| k.base().health > 0)
    }

    /// TFS `Creature::onThink` — shared base logic for all creature kinds (D.2 subset).
    pub fn creature_on_think(&mut self, cid: CreatureId, interval_ms: u32) {
        let (follow, attack, master) = match self.creatures.get(cid) {
            Some(k) => (
                k.base().follow_target,
                k.base().attack_target,
                k.base().master,
            ),
            None => return,
        };

        if let Some(follow_id) = follow {
            if master != Some(follow_id) && !self.can_see_creature(cid, follow_id) {
                if let Some(k) = self.creatures.get_mut(cid) {
                    k.base_mut().clear_follow_for_target(follow_id);
                }
            }
        }

        if let Some(attack_id) = attack {
            if master != Some(attack_id) && !self.can_see_creature(cid, attack_id) {
                if let Some(k) = self.creatures.get_mut(cid) {
                    k.base_mut().clear_attack_for_target(attack_id);
                }
            }
        }

        let follow_id = self.creatures.get(cid).and_then(|k| k.base().follow_target);
        let skip_repath_at_goal = follow_id
            .is_some_and(|fid| self.monster_should_skip_follow_repath(cid, fid));

        if !self.beat_driven_loop {
            let mut run_follow_repath = false;
            if let Some(k) = self.creatures.get_mut(cid) {
                let base = k.base_mut();
                if let Some(_follow_id) = base.follow_target {
                    base.walk_update_ticks = base.walk_update_ticks.saturating_add(interval_ms);
                    let wants_repath = base.force_update_follow_path
                        || base.walk_update_ticks >= FOLLOW_PATH_UPDATE_INTERVAL_MS;
                    if wants_repath {
                        base.walk_update_ticks = 0;
                        base.force_update_follow_path = false;
                        if skip_repath_at_goal {
                            base.has_follow_path = true;
                        } else {
                            base.is_updating_path = true;
                        }
                    }
                }
                run_follow_repath = base.is_updating_path;
                if run_follow_repath {
                    base.is_updating_path = false;
                }
            }
            if run_follow_repath {
                self.go_to_follow_creature(cid);
            }
        }

        self.events.on_think(cid, interval_ms);
    }

    /// TFS `Creature::onAttacking` — `creature.cpp` (~172–189).
    pub fn creature_on_attacking(&mut self, cid: CreatureId, interval_ms: u32) {
        let (attack_id, is_summon) = match self.creatures.get(cid) {
            Some(k) => (k.base().attack_target, k.base().is_summon()),
            None => return,
        };
        let Some(attack_id) = attack_id else {
            return;
        };
        if is_summon && attack_id == cid {
            return;
        }
        if !self.creatures.contains_key(attack_id) {
            return;
        }

        // TODO: `onAttacked` / target `onAttacked` callbacks (`creature.cpp` ~178–179).

        let (my_pos, target_pos) = match self.creatures.get(cid).zip(self.creatures.get(attack_id)) {
            Some((attacker, target)) => (attacker.position(), target.position()),
            None => return,
        };
        if !self.map.is_sight_clear(my_pos, target_pos) {
            return;
        }

        if self.creatures.get(cid).is_some_and(|k| matches!(k, CreatureKind::Monster(_))) {
            self.monster_do_attacking(cid, interval_ms);
        }
    }

    /// TFS `Monster::onThink` — base think + native AI (D.4).
    pub fn monster_on_think(&mut self, cid: CreatureId, interval_ms: u32) {
        self.creature_on_think(cid, interval_ms);
        self.monster_native_on_think(cid, interval_ms);
    }

    /// TFS `Npc::onThink` — base think + stub for idle walk / focus (D.6).
    pub fn npc_on_think(&mut self, cid: CreatureId, interval_ms: u32) {
        self.creature_on_think(cid, interval_ms);
        // D.6: random step within master_radius, focus / turn-to-speaker.
        let _ = interval_ms;
        let _ = cid;
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use tfs_rust_common::Position;

    
    use crate::test_world::support::{
        beat_driven_world, ensure_walkable_tile, insert_npc, minimal_world, CountingEventDispatcher,
    };

    use super::*;

    fn step_ticks(world: &mut crate::game_world::GameWorld, start: Instant, count: u32, step_ms: u64) -> Instant {
        let mut now = start;
        for _ in 0..count {
            world.on_tick(now);
            now += Duration::from_millis(step_ms);
        }
        now
    }

    fn world_with_counter() -> (crate::game_world::GameWorld, std::sync::Arc<CountingEventDispatcher>) {
        let counter = std::sync::Arc::new(CountingEventDispatcher::default());
        let mut world = minimal_world();
        world.events = Box::new(CountingEventDispatcherProxy(counter.clone()));
        (world, counter)
    }

    /// Proxy so tests can share the counter via `Arc`.
    struct CountingEventDispatcherProxy(std::sync::Arc<CountingEventDispatcher>);

    impl crate::event_dispatcher::EventDispatcher for CountingEventDispatcherProxy {
        fn on_think(&self, creature: CreatureId, interval_ms: u32) {
            self.0.on_think(creature, interval_ms);
        }
    }

    #[test]
    fn think_sweep_fires_at_1hz_per_bucket() {
        let (mut world, counter) = world_with_counter();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);

        let npc = insert_npc(&mut world, "Tom", pos, 100);
        world.set_creature_think_check_bucket(npc, 0);

        let start = Instant::now();
        step_ticks(&mut world, start, 50, 50);

        assert_eq!(
            counter.total_think_calls(),
            3,
            "NPC in bucket 0 should think at 100 ms, 1100 ms, and 2100 ms within 2.5 s"
        );
    }

    #[test]
    fn idle_monster_not_in_think_sweep() {
        let (mut world, counter) = world_with_counter();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);

        let cid = crate::test_world::support::insert_monster(&mut world, "rat", pos, 200);
        assert!(
            world
                .creatures
                .get(cid)
                .is_some_and(|k| k.base().think_check_bucket.is_none()),
            "idle monster must not be registered for think checks"
        );

        let start = Instant::now();
        step_ticks(&mut world, start, 50, 50);

        assert_eq!(
            counter.total_think_calls(),
            0,
            "idle monsters must not receive onThink"
        );
    }

    #[test]
    fn npc_included_in_sweep() {
        let (mut world, counter) = world_with_counter();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);

        let npc = insert_npc(&mut world, "Tom", pos, 100);
        world.set_creature_think_check_bucket(npc, 0);

        let start = Instant::now();
        step_ticks(&mut world, start, 20, 50);

        assert_eq!(
            counter.total_think_calls(),
            1,
            "NPC in bucket 0 should think once after first bucket cycle (~1 s)"
        );
    }

    #[test]
    fn interval_is_fixed_1000() {
        let (mut world, counter) = world_with_counter();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);

        let npc = insert_npc(&mut world, "Tom", pos, 100);
        world.set_creature_think_check_bucket(npc, 0);

        let start = Instant::now();
        step_ticks(&mut world, start, 25, 50);

        assert!(
            counter.intervals().iter().all(|&ms| ms == EVENT_CREATURE_THINK_INTERVAL_MS),
            "onThink interval must be fixed 1000 ms for C++ parity"
        );
    }

    #[test]
    fn staggered_buckets_spread_thinks() {
        let (mut world, counter) = world_with_counter();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);

        let npc0 = insert_npc(&mut world, "Tom", pos, 100);
        let npc5 = insert_npc(&mut world, "Tim", Position::new(101, 100, 7), 100);
        world.set_creature_think_check_bucket(npc0, 0);
        world.set_creature_think_check_bucket(npc5, 5);

        let start = Instant::now();
        step_ticks(&mut world, start, 10, 100);

        assert_eq!(
            counter.total_think_calls(),
            2,
            "buckets 0 and 5 each fire once within the first 1 s cycle"
        );
    }

    #[test]
    fn creature_removed_during_think_safe() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let cid = crate::test_world::support::insert_monster(&mut world, "rat", pos, 200);

        world.monster_on_think(cid, EVENT_CREATURE_THINK_INTERVAL_MS);
        world.remove_creature(cid);

        let start = Instant::now();
        step_ticks(&mut world, start, 25, 50);
    }

    #[test]
    fn process_creatures_772_thinks_at_1hz() {
        let (mut world, counter) = {
            let counter = std::sync::Arc::new(CountingEventDispatcher::default());
            let mut world = beat_driven_world();
            world.events = Box::new(CountingEventDispatcherProxy(counter.clone()));
            world.walk_wake_tx = None;
            (world, counter)
        };

        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let npc = insert_npc(&mut world, "Tom", pos, 100);
        world.set_creature_think_check_bucket(npc, 0);

        const BEAT_MS: u64 = 200;
        for _ in 0..9 {
            world.advance_beat_772(BEAT_MS);
        }

        assert_eq!(
            counter.total_think_calls(),
            1,
            "772 creature counter fires once at 1750 ms (9×200 ms beats)"
        );

        for _ in 0..5 {
            world.advance_beat_772(BEAT_MS);
        }

        assert_eq!(
            counter.total_think_calls(),
            2,
            "second ProcessCreatures at ~2750 ms cumulative"
        );
    }

    #[test]
    fn tick_counter_advances_on_beat_for_decay() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;

        assert_eq!(world.tick_counter, 0);
        for _ in 0..5 {
            world.advance_beat_772(200);
        }
        assert_eq!(
            world.tick_counter, 20,
            "each 200 ms beat adds 4 fifty-ms tick units for decay parity"
        );
    }
}
