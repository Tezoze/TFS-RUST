    use std::time::{Duration, Instant};

    use tfs_rust_common::Position;

    use crate::test_world::support::{
        beat_driven_world, ensure_walkable_tile, insert_npc, minimal_world, CountingEventDispatcher,
    };

    use super::*;

    fn step_ticks(
        world: &mut crate::game_world::GameWorld,
        start: Instant,
        count: u32,
        step_ms: u64,
    ) -> Instant {
        let mut now = start;
        for _ in 0..count {
            world.on_tick(now);
            now += Duration::from_millis(step_ms);
        }
        now
    }

    fn world_with_counter() -> (
        crate::game_world::GameWorld,
        std::sync::Arc<CountingEventDispatcher>,
    ) {
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
            counter
                .intervals()
                .iter()
                .all(|&ms| ms == EVENT_CREATURE_THINK_INTERVAL_MS),
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

    /// RC1: `process_creatures_772` must NOT call `onThink` — C++ `ProcessCreatures`
    /// (`crmain.cc:1075–1138`) is regen + death safety only. AI is ToDoQueue-driven.
    #[test]
    fn process_creatures_772_does_not_call_on_think() {
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
        // 9 beats = 1800 ms → creature counter fires once at 1750 ms threshold.
        for _ in 0..9 {
            world.advance_beat_772(BEAT_MS);
        }

        assert_eq!(
            counter.total_think_calls(),
            0,
            "RC1: process_creatures_772 must not call onThink — AI is ToDoQueue-driven"
        );

        // 5 more beats = 2800 ms cumulative → second ProcessCreatures fire.
        for _ in 0..5 {
            world.advance_beat_772(BEAT_MS);
        }

        assert_eq!(
            counter.total_think_calls(),
            0,
            "RC1: second ProcessCreatures fire still must not call onThink"
        );
    }

    /// RC1: `process_creatures_772` retains the C++ death safety net
    /// (`crmain.cc:1113–1117`: `HP <= 0 && !IsDead → Death()`).
    #[test]
    fn process_creatures_772_applies_death_safety() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;

        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let monster = crate::test_world::support::insert_monster(&mut world, "Rat", pos, 200);

        // Simulate a creature that has HP <= 0 but wasn't killed through the normal path.
        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().health = 0;
        }
        assert!(world.creatures.contains_key(monster));

        world.process_creatures_772();

        assert!(
            !world.creatures.contains_key(monster),
            "RC1: process_creatures_772 death safety must kill creatures with HP <= 0"
        );
    }

    /// RC1: `process_creatures_772` must not clear follow/attack targets.
    /// Previously `monster_on_think` → `creature_on_think` cleared targets out of view
    /// on a 1 Hz timer; C++ 772 only clears targets inside `IdleStimulus`.
    #[test]
    fn process_creatures_772_does_not_clear_targets() {
        use crate::test_world::support::{insert_player, test_player};

        let mut world = beat_driven_world();
        world.walk_wake_tx = None;

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(115, 100, 7); // beyond 10-tile targeting range
        ensure_walkable_tile(&mut world.map, mpos, 100);
        ensure_walkable_tile(&mut world.map, ppos, 100);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = crate::test_world::support::insert_monster(&mut world, "Rat", mpos, 200);

        // Manually set a target (simulating a chase that went out of view).
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }
        world.add_creature_think_check(monster);

        world.process_creatures_772();

        let still_has_target = world
            .creatures
            .get(monster)
            .is_some_and(|k| k.base().follow_target == Some(player));
        assert!(
            still_has_target,
            "RC1: process_creatures_772 must not clear targets — only IdleStimulus does (crnonpl.cc:2418)"
        );
    }

    #[test]
    fn decay_advances_on_server_ms_772() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;

        let corpse_id = world.items.insert(crate::item::Item::new(3058, 1));
        world.decay.schedule(corpse_id, 1_000, None);

        assert_eq!(world.server_ms, 0);
        for _ in 0..5 {
            world.advance_beat_772(200);
        }
        assert_eq!(world.server_ms, 1_000);
        let expired = world.decay.tick(world.server_ms);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].0, corpse_id);
    }
