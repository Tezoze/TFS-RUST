    use tfs_rust_common::{Position, ZoneType};

    use crate::test_world::support::{
        beat_driven_test_world, beat_driven_world, ensure_walkable_tile, insert_player,
        test_player, CountingEventDispatcher,
    };

    use super::*;

    /// Proxy so tests can share the counter via `Arc`.
    struct CountingEventDispatcherProxy(std::sync::Arc<CountingEventDispatcher>);

    impl crate::event_dispatcher::EventDispatcher for CountingEventDispatcherProxy {
        fn on_think(&self, creature: CreatureId, interval_ms: u32) {
            self.0.on_think(creature, interval_ms);
        }
    }

    /// RC1: `process_creatures` must NOT call `onThink` — C++ `ProcessCreatures`
    /// (`crmain.cc:1075–1138`) is regen + death safety only. AI is ToDoQueue-driven.
    #[test]
    fn process_creatures_does_not_call_on_think() {
        let (mut world, counter) = {
            let counter = std::sync::Arc::new(CountingEventDispatcher::default());
            let mut world = beat_driven_world();
            world.events = Box::new(CountingEventDispatcherProxy(counter.clone()));
            (world, counter)
        };

        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let npc = crate::test_world::support::insert_npc(&mut world, "Tom", pos, 100);

        const BEAT_MS: u64 = 200;
        // 9 beats = 1800 ms → creature counter fires once at 1750 ms threshold.
        for _ in 0..9 {
            world.advance_beat(BEAT_MS);
        }

        assert_eq!(
            counter.total_think_calls(),
            0,
            "RC1: process_creatures must not call onThink — AI is ToDoQueue-driven"
        );

        // 5 more beats = 2800 ms cumulative → second ProcessCreatures fire.
        for _ in 0..5 {
            world.advance_beat(BEAT_MS);
        }

        assert_eq!(
            counter.total_think_calls(),
            0,
            "RC1: second ProcessCreatures fire still must not call onThink"
        );
    }

    /// RC1: `process_creatures` retains the C++ death safety net
    /// (`crmain.cc:1113–1117`: `HP <= 0 && !IsDead → Death()`).
    #[test]
    fn process_creatures_applies_death_safety() {
        let mut world = beat_driven_world();

        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let monster = crate::test_world::support::insert_monster(&mut world, "Rat", pos, 200);

        // Simulate a creature that has HP <= 0 but wasn't killed through the normal path.
        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().health = 0;
        }
        assert!(world.creatures.contains_key(monster));

        world.process_creatures();

        assert!(
            !world.creatures.contains_key(monster),
            "RC1: process_creatures death safety must kill creatures with HP <= 0"
        );
    }

    /// RC1: `process_creatures` must not clear follow/attack targets.
    /// Previously `monster_on_think` → `creature_on_think` cleared targets out of view
    /// on a 1 Hz timer; C++ 772 only clears targets inside `IdleStimulus`.
    #[test]
    fn process_creatures_does_not_clear_targets() {
        use crate::test_world::support::{insert_player, test_player};

        let mut world = beat_driven_world();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(115, 100, 7); // beyond 10-tile targeting range
        ensure_walkable_tile(&mut world.map, mpos, 100);
        ensure_walkable_tile(&mut world.map, ppos, 100);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = crate::test_world::support::insert_monster(&mut world, "Rat", mpos, 200);

        // Manually set a target (simulating a chase that went out of view).
        if let Some(crate::creature::CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }

        world.process_creatures();

        let still_has_target = world
            .creatures
            .get(monster)
            .is_some_and(|k| k.base().follow_target == Some(player));
        assert!(
            still_has_target,
            "RC1: process_creatures must not clear targets — only IdleStimulus does (crnonpl.cc:2418)"
        );
    }

    #[test]
    fn decay_advances_on_server_ms_772() {
        let mut world = beat_driven_world();

        let corpse_id = world.items.insert(crate::item::Item::new(3058, 1));
        world.decay.schedule(corpse_id, 1_000, None);

        assert_eq!(world.server_ms, 0);
        for _ in 0..5 {
            world.advance_beat(200);
        }
        assert_eq!(world.server_ms, 1_000);
        let expired = world.decay.tick(world.server_ms);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].0, corpse_id);
    }

    // ─── F2 Part A: item regen (HP+1/Mana+4) tests ───
    // C++ reference: `crmain.cc:1087-1095` `ProcessCreatures` item regen.

    use crate::creature::CreatureKind;
    use crate::tile::{Tile, TileBody};
    use slotmap::Key;

    /// Insert a protection-zone ground tile at `pos` (mirrors `ensure_walkable_tile`
    /// but with `ZoneType::Protection` — `crmain.cc:1093` PZ gate).
    fn ensure_pz_tile(map: &mut crate::map::Map, pos: Position, ground_type: u16) {
        map.insert_tile(
            pos,
            Tile::Normal(TileBody {
                ground: Some(ground_type),
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: 0,
                zone: ZoneType::Protection,
            }),
        );
    }

    /// F2: `ProcessCreatures` item regen fires HP+1/Mana+4 when `food_level > 0`
    /// and `round_nr % food_level == 0` (`crmain.cc:1087-1095`).
    #[test]
    fn item_regen_fires_at_food_level_cadence() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);

        let mut player = test_player("Fed", pos);
        player.base.health = 90;
        player.base.max_health = 100;
        player.mana = 40;
        player.max_mana = 50;
        player.food_level = 12; // regen every 12 rounds
        let pid = insert_player(&mut world, player);

        // round_nr starts at 0; 0 % 12 == 0, so first call fires.
        world.round_nr = 0;
        world.process_creatures();

        let p = world.creatures.get(pid).unwrap();
        let CreatureKind::Player(p) = p else { panic!("not a player") };
        assert_eq!(p.base.health, 91, "HP should gain +1 from item regen");
        assert_eq!(p.mana, 44, "Mana should gain +4 from item regen");
    }

    /// F2: item regen does NOT fire when `food_level == 0` (`crmain.cc:1087`).
    #[test]
    fn item_regen_skipped_when_food_level_zero() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);

        let mut player = test_player("Hungry", pos);
        player.base.health = 90;
        player.base.max_health = 100;
        player.mana = 40;
        player.max_mana = 50;
        player.food_level = 0;
        let pid = insert_player(&mut world, player);

        world.round_nr = 0;
        world.process_creatures();

        let p = world.creatures.get(pid).unwrap();
        let CreatureKind::Player(p) = p else { panic!("not a player") };
        assert_eq!(p.base.health, 90, "no regen when food_level == 0");
        assert_eq!(p.mana, 40, "no regen when food_level == 0");
    }

    /// F2: item regen does NOT fire inside a protection zone (`crmain.cc:1093`).
    #[test]
    fn item_regen_skipped_in_protection_zone() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_pz_tile(&mut world.map, pos, 150);

        let mut player = test_player("PZ", pos);
        player.base.health = 90;
        player.base.max_health = 100;
        player.mana = 40;
        player.max_mana = 50;
        player.food_level = 12;
        let pid = insert_player(&mut world, player);

        world.round_nr = 0;
        world.process_creatures();

        let p = world.creatures.get(pid).unwrap();
        let CreatureKind::Player(p) = p else { panic!("not a player") };
        assert_eq!(p.base.health, 90, "no regen in PZ");
        assert_eq!(p.mana, 40, "no regen in PZ");
    }

    /// F2: item regen does NOT fire when the player is dead (`crmain.cc:1092` `!IsDead`).
    #[test]
    fn item_regen_skipped_when_dead() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);

        let mut player = test_player("Dead", pos);
        player.base.health = 0;
        player.base.max_health = 100;
        player.mana = 0;
        player.max_mana = 50;
        player.food_level = 12;
        let pid = insert_player(&mut world, player);

        world.round_nr = 0;
        world.process_creatures();

        // Player should be processed by death safety, not regen.
        // HP stays 0 (or creature is dead/removed by apply_creature_death).
        let p = world.creatures.get(pid);
        if let Some(creature) = p {
            if let CreatureKind::Player(p) = creature {
                assert!(p.base.health <= 0, "dead player should not gain HP from regen");
            }
        }
    }

    /// F2: item regen does NOT fire when `round_nr % food_level != 0` (`crmain.cc:1088`).
    #[test]
    fn item_regen_skipped_off_cadence() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);

        let mut player = test_player("OffCadence", pos);
        player.base.health = 90;
        player.base.max_health = 100;
        player.mana = 40;
        player.max_mana = 50;
        player.food_level = 12;
        let pid = insert_player(&mut world, player);

        // round_nr = 5; 5 % 12 != 0, so no regen.
        world.round_nr = 5;
        world.process_creatures();

        let p = world.creatures.get(pid).unwrap();
        let CreatureKind::Player(p) = p else { panic!("not a player") };
        assert_eq!(p.base.health, 90, "no regen off cadence");
        assert_eq!(p.mana, 40, "no regen off cadence");
    }

    /// F2: `EarliestLogoutRound` expiry clears the PK-mark timer (`crmain.cc:1102-1105`).
    /// Stub: the field is zeroed; full `ClearPlayerkillingMarks` is deferred.
    #[test]
    fn earliest_logout_round_expiry_clears_pk_marks() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);

        let mut player = test_player("PK", pos);
        player.earliest_logout_round = 10;
        let pid = insert_player(&mut world, player);

        // round_nr = 10; 10 <= 10, so timer expires.
        world.round_nr = 10;
        world.process_creatures();

        let p = world.creatures.get(pid).unwrap();
        let CreatureKind::Player(p) = p else { panic!("not a player") };
        assert_eq!(p.earliest_logout_round, 0, "PK-mark timer should be cleared");
    }

    /// F2: `EarliestLogoutRound` does NOT expire before the round (`crmain.cc:1102`).
    #[test]
    fn earliest_logout_round_not_expired_before_round() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);

        let mut player = test_player("PK2", pos);
        player.earliest_logout_round = 10;
        let pid = insert_player(&mut world, player);

        world.round_nr = 5;
        world.process_creatures();

        let p = world.creatures.get(pid).unwrap();
        let CreatureKind::Player(p) = p else { panic!("not a player") };
        assert_eq!(p.earliest_logout_round, 10, "PK-mark timer should not expire early");
    }

    /// F2: `player:feed(amount)` refills `food_remaining`, capped at `MAX_FOOD` (1200).
    /// C++ reference: `moveuse.cc:1846` `SetTimer(SKILL_FED, CurFoodTime + ObjFoodTime, ...)`.
    #[test]
    fn lua_feed_refills_food_remaining_capped() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);

        let mut player = test_player("Eater", pos);
        player.food_remaining = 100;
        let pid = insert_player(&mut world, player);

        // Feed 200 → 100 + 200 = 300.
        world.lua_script_player_feed(pid.data().as_ffi(), 200).unwrap();
        let p = world.creatures.get(pid).unwrap();
        let CreatureKind::Player(p) = p else { panic!("not a player") };
        assert_eq!(p.food_remaining, 300, "food should be 100 + 200 = 300");

        // Feed 1200 → 300 + 1200 = 1500, capped at 1200.
        world.lua_script_player_feed(pid.data().as_ffi(), 1200).unwrap();
        let p = world.creatures.get(pid).unwrap();
        let CreatureKind::Player(p) = p else { panic!("not a player") };
        assert_eq!(p.food_remaining, 1200, "food should be capped at MAX_FOOD");
    }

    /// F2: `player:feed` sets `food_level` to the regen interval on first eat.
    #[test]
    fn lua_feed_sets_food_level() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);

        let mut player = test_player("FirstEat", pos);
        player.food_level = 0;
        let pid = insert_player(&mut world, player);

        world.lua_script_player_feed(pid.data().as_ffi(), 100).unwrap();
        let p = world.creatures.get(pid).unwrap();
        let CreatureKind::Player(p) = p else { panic!("not a player") };
        assert_eq!(p.food_level, 12, "food_level should be set to 12 (default regen interval)");
    }
