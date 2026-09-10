use tfs_rust_common::{Position, ZoneType};

use crate::test_world::support::{
    CountingEventDispatcher, beat_driven_test_world, beat_driven_world, ensure_walkable_tile,
    insert_player, test_player,
};

use super::*;

/// Proxy so tests can share the counter via `Arc`.
struct CountingEventDispatcherProxy(std::sync::Arc<CountingEventDispatcher>);

impl crate::event_dispatcher::EventDispatcher for CountingEventDispatcherProxy {
    fn on_think(&self, creature: CreatureId, interval_ms: u32) {
        self.0.on_think(creature, interval_ms);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
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
// C++ reference: `crmain.cc:1087-1095` — cadence is equipped DAct, not eating.

use crate::creature::CreatureKind;
use crate::inventory::InventorySlot;
use crate::tile::{Tile, TileBody};
use slotmap::Key;
use tfs_rust_common::ConnId;
use tfs_rust_common::enums::ConditionType;
use tfs_rust_content::item_abilities::ItemAbilities;
use tfs_rust_content::otb::ItemType;

fn register_item_type(
    world: &mut crate::game_world::GameWorld,
    item_type_id: u16,
    mut it: ItemType,
) {
    it.id = item_type_id;
    it.server_id = item_type_id;
    let mut items = std::collections::HashMap::clone(&world.items_db.items);
    items.insert(item_type_id, it);
    let client_to_server = std::collections::HashMap::clone(&world.items_db.client_to_server);
    world.items_db = std::sync::Arc::new(tfs_rust_content::items::ItemDatabase {
        items,
        client_to_server,
    });
}

fn equip_regen_ring(
    world: &mut crate::game_world::GameWorld,
    cid: crate::ids::CreatureId,
    item_type_id: u16,
    health_ticks: u32,
) -> crate::ids::ItemId {
    let mut abl = ItemAbilities::default();
    abl.regeneration = true;
    abl.health_gain = 1;
    abl.health_ticks = health_ticks;
    abl.mana_gain = 4;
    abl.mana_ticks = health_ticks;
    let mut it = ItemType::default();
    it.abilities = abl;
    register_item_type(world, item_type_id, it);
    let iid = world
        .items
        .insert(crate::item::Item::new_single(item_type_id));
    let slot = InventorySlot::Ring as u8;
    if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
        let idx = crate::inventory::slot_to_array_index(slot).expect("slot");
        p.equipment_slots[idx] = Some(iid);
    }
    world.apply_equip_item_abilities(cid, iid, slot);
    iid
}

/// Insert a protection-zone ground tile at `pos` (mirrors `ensure_walkable_tile`
/// but with `ZoneType::Protection` — `crmain.cc:1093` PZ gate).
fn ensure_pz_tile(map: &mut crate::map::Map, pos: Position, ground_type: u16) {
    map.insert_tile(
        pos,
        Tile::Normal(TileBody {
            ground: Some(ground_type),

            ground_item: None,
            down_items: Vec::new(),
            top_items: Vec::new(),
            creatures: Vec::new(),
            flags: 0,
            zone: ZoneType::Protection,
        }),
    );
}

/// Eating must not arm Creatures-arm item regen (`moveuse.cc:1846` SetTimer Cycle only).
#[test]
fn eating_does_not_arm_item_regen() {
    let mut world = beat_driven_test_world();
    let pos = Position::new(100, 100, 7);
    ensure_walkable_tile(&mut world.map, pos, 150);

    let mut player = test_player("Eater", pos);
    player.base.health = 90;
    player.base.max_health = 100;
    player.mana = 40;
    player.max_mana = 50;
    let pid = insert_player(&mut world, player);
    world
        .lua_script_player_feed(pid.data().as_ffi(), 200)
        .unwrap();

    for round in 1..=60 {
        world.round_nr = round;
        world.process_creatures();
    }

    let p = world.creatures.get(pid).unwrap();
    let CreatureKind::Player(p) = p else {
        panic!("not a player")
    };
    assert_eq!(p.item_regen_interval, 0);
    assert_eq!(p.base.health, 90, "eating must not grant item regen");
    assert_eq!(p.mana, 40, "eating must not grant item regen");
}

/// Life ring 2205: DAct 3 → +1 HP / +4 mana at rounds 3, 6, 9.
#[test]
fn life_ring_regens_every_3_rounds() {
    let mut world = beat_driven_test_world();
    let pos = Position::new(100, 100, 7);
    ensure_walkable_tile(&mut world.map, pos, 150);

    let mut player = test_player("LifeRing", pos);
    player.base.health = 90;
    player.base.max_health = 100;
    player.mana = 40;
    player.max_mana = 200;
    let pid = insert_player(&mut world, player);
    let conn = ConnId(1);
    world.register_conn_mapping(conn, pid);
    let _ = equip_regen_ring(&mut world, pid, 2205, 3000);

    for round in 1..=9 {
        world.pending_outgoing.clear();
        world.round_nr = round;
        world.process_creatures();
        if round.is_multiple_of(3) {
            assert!(
                world
                    .pending_outgoing
                    .get(&conn)
                    .is_some_and(|p| !p.is_empty()),
                "stats/health packets at round {round}"
            );
        }
    }

    let p = world.creatures.get(pid).unwrap();
    let CreatureKind::Player(p) = p else {
        panic!("not a player")
    };
    assert_eq!(p.item_regen_interval, 3);
    assert_eq!(p.base.health, 93);
    assert_eq!(p.mana, 52);
}

/// Ring of healing 2216: DAct 1 → regen every round.
#[test]
fn ring_of_healing_regens_every_round() {
    let mut world = beat_driven_test_world();
    let pos = Position::new(100, 100, 7);
    ensure_walkable_tile(&mut world.map, pos, 150);

    let mut player = test_player("HealRing", pos);
    player.base.health = 90;
    player.base.max_health = 100;
    player.mana = 40;
    player.max_mana = 200;
    let pid = insert_player(&mut world, player);
    let _ = equip_regen_ring(&mut world, pid, 2216, 1000);

    for round in 1..=4 {
        world.round_nr = round;
        world.process_creatures();
    }

    let p = world.creatures.get(pid).unwrap();
    let CreatureKind::Player(p) = p else {
        panic!("not a player")
    };
    assert_eq!(p.item_regen_interval, 1);
    assert_eq!(p.base.health, 94);
    assert_eq!(p.mana, 56);
}

#[test]
fn item_regen_blocked_in_pz() {
    let mut world = beat_driven_test_world();
    let pos = Position::new(100, 100, 7);
    ensure_pz_tile(&mut world.map, pos, 150);

    let mut player = test_player("PZ", pos);
    player.base.health = 90;
    player.base.max_health = 100;
    player.mana = 40;
    player.max_mana = 50;
    let pid = insert_player(&mut world, player);
    let _ = equip_regen_ring(&mut world, pid, 2216, 1000);

    world.round_nr = 1;
    world.process_creatures();

    let p = world.creatures.get(pid).unwrap();
    let CreatureKind::Player(p) = p else {
        panic!("not a player")
    };
    assert_eq!(p.base.health, 90, "no regen in PZ");
    assert_eq!(p.mana, 40, "no regen in PZ");
}

#[test]
fn item_regen_skips_dead() {
    let mut world = beat_driven_test_world();
    let pos = Position::new(100, 100, 7);
    ensure_walkable_tile(&mut world.map, pos, 150);

    let mut player = test_player("Dead", pos);
    player.base.health = 0;
    player.base.max_health = 100;
    player.mana = 0;
    player.max_mana = 50;
    let pid = insert_player(&mut world, player);
    if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(pid) {
        p.item_regen_interval = 1;
    }

    world.round_nr = 1;
    assert!(
        !world.process_item_regen(pid, 1),
        "dead player must not receive item regen"
    );

    let p = world.creatures.get(pid);
    if let Some(creature) = p
        && let CreatureKind::Player(p) = creature
    {
        assert_eq!(p.base.health, 0);
        assert_eq!(p.mana, 0);
    }
}

#[test]
fn item_regen_stops_on_unequip() {
    let mut world = beat_driven_test_world();
    let pos = Position::new(100, 100, 7);
    ensure_walkable_tile(&mut world.map, pos, 150);

    let mut player = test_player("Unequip", pos);
    player.base.health = 90;
    player.base.max_health = 100;
    player.mana = 40;
    player.max_mana = 50;
    let pid = insert_player(&mut world, player);
    let iid = equip_regen_ring(&mut world, pid, 2216, 1000);
    world.round_nr = 1;
    world.process_creatures();
    world.remove_equip_item_abilities(pid, iid, InventorySlot::Ring as u8);
    world.round_nr = 2;
    world.process_creatures();

    let p = world.creatures.get(pid).unwrap();
    let CreatureKind::Player(p) = p else {
        panic!("not a player")
    };
    assert_eq!(p.item_regen_interval, 0);
    assert_eq!(p.base.health, 91, "only the equipped round granted HP");
    assert_eq!(p.mana, 44);
}

/// F2: `EarliestLogoutRound` expiry runs `ClearPlayerkillingMarks` (`crmain.cc:1102-1105`).
#[test]
fn earliest_logout_round_expiry_clears_pk_marks() {
    let mut world = beat_driven_test_world();
    let pos = Position::new(100, 100, 7);
    ensure_walkable_tile(&mut world.map, pos, 150);

    let victim = insert_player(&mut world, test_player("Victim", pos));
    let mut player = test_player("PK", pos);
    player.earliest_logout_round = 10;
    player.aggressor = true;
    player.attacked_players.push(victim);
    let pid = insert_player(&mut world, player);

    // round_nr = 10; 10 <= 10, so timer expires.
    world.round_nr = 10;
    world.process_creatures();

    let p = world.creatures.get(pid).unwrap();
    let CreatureKind::Player(p) = p else {
        panic!("not a player")
    };
    assert_eq!(
        p.earliest_logout_round, 0,
        "PK-mark timer should be cleared"
    );
    assert!(!p.aggressor);
    assert!(p.former_aggressor);
    assert!(p.attacked_players.is_empty());
    assert!(p.former_attacked_players.contains(&victim));
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
    let CreatureKind::Player(p) = p else {
        panic!("not a player")
    };
    assert_eq!(
        p.earliest_logout_round, 10,
        "PK-mark timer should not expire early"
    );
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
    world
        .lua_script_player_feed(pid.data().as_ffi(), 200)
        .unwrap();
    let p = world.creatures.get(pid).unwrap();
    let CreatureKind::Player(p) = p else {
        panic!("not a player")
    };
    assert_eq!(p.food_remaining, 300, "food should be 100 + 200 = 300");

    // Feed 1200 → 300 + 1200 = 1500, capped at 1200.
    world
        .lua_script_player_feed(pid.data().as_ffi(), 1200)
        .unwrap();
    let p = world.creatures.get(pid).unwrap();
    let CreatureKind::Player(p) = p else {
        panic!("not a player")
    };
    assert_eq!(p.food_remaining, 1200, "food should be capped at MAX_FOOD");
}

#[test]
fn process_creatures_sends_icons_when_round_crosses_logout() {
    use tfs_rust_common::ConnId;

    let mut world = beat_driven_test_world();
    let pos = Position::new(100, 100, 7);
    ensure_walkable_tile(&mut world.map, pos, 150);
    let player = insert_player(&mut world, test_player("Icons", pos));
    let conn = ConnId(1);
    world.register_conn_mapping(conn, player);
    world.round_nr = 10;
    if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
        p.earliest_logout_round = 10;
        p.client_icons = 0x80;
    }
    world.pending_outgoing.clear();
    world.process_creatures();
    let outgoing = world.pending_outgoing.get(&conn);
    assert!(
        outgoing.is_some_and(|q| q.iter().any(|b| !b.is_empty() && b[0] == 0xA2)),
        "CheckState sweep must send 0xA2 when swords/logout round expires"
    );
}
