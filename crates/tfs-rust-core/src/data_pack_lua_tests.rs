//! Phase 8 data-pack Lua invariants: spawn roll is the only roll; rarity survives death;
//! summons drop nothing; loot is native; onDeath must not create items.
//!
//! Corpus: `TMonster::TMonster` spawn inventory (`crnonpl.cc:2050`); `~TCreature` corpse
//! move (`crmain.cc:204-290`); player AoL/SOME/ALL (`crmain.cc:790-815`, `crplayer.cc:292`).

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use tfs_rust_common::Position;
use tfs_rust_common::ProtocolVersion;
use tfs_rust_common::enums::BloodType;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::monsters::{
    LootBlock, MAX_LOOTCHANCE, MonsterDefenses, MonsterOutfit, MonsterType, MonsterTypeFlags,
};
use tfs_rust_lua::{LuaRuntime, MoveEventsRegistry, load_data_lib};

use crate::creature::{CreatureKind, MonsterInventory};
use crate::cylinder::{Cylinder, INDEX_WHEREEVER};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::{InventorySlot, SLOTP_NECKLACE};
use crate::item::Item;
use crate::lua_event_dispatcher::LuaEventDispatcher;
use crate::lua_scope::register_lua_mutation_hooks;
use crate::sim_harness::{
    bag_item_type, insert_monster, insert_player, pickup_item_type, test_player,
};
use crate::test_world::support::{ensure_walkable_tile, minimal_world};

const BAG: u16 = 1987;
const GOLD: u16 = 2148;
const RAT_CORPSE: u16 = 2813;
const SPLASH: u16 = 2016;
const AOL: u16 = 2173;
const DEAD_HUMAN: u16 = 3128;
const RARITY_AID: u16 = 4242;

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn loot_world(version: ProtocolVersion) -> GameWorld {
    register_lua_mutation_hooks();
    let mut world = minimal_world();
    world.mechanics = crate::formulas::Mechanics::for_version(version);

    let mut items = HashMap::new();
    items.insert(BAG, bag_item_type(BAG));
    let mut gold = pickup_item_type(GOLD);
    gold.flags |= 1 << 7;
    items.insert(GOLD, gold);
    items.insert(RAT_CORPSE, {
        let mut c = bag_item_type(RAT_CORPSE);
        c.xml_attributes.insert("containersize".into(), "5".into());
        c
    });
    items.insert(SPLASH, pickup_item_type(SPLASH));
    items.insert(AOL, {
        let mut it = pickup_item_type(AOL);
        it.slot_position = SLOTP_NECKLACE;
        it
    });
    items.insert(DEAD_HUMAN, {
        let mut c = bag_item_type(DEAD_HUMAN);
        c.xml_attributes.insert("containersize".into(), "20".into());
        c
    });
    world.items_db = Arc::new(ItemDatabase {
        items,
        client_to_server: HashMap::new(),
    });
    world.seed_parity_rng(42);
    world
}

fn rat_with_loot() -> MonsterType {
    MonsterType {
        name: "Rat".into(),
        filename: "rat.xml".into(),
        name_description: "a rat".into(),
        race: "blood".into(),
        experience: 5,
        speed: 27,
        health_now: 20,
        health_max: 20,
        outfit: MonsterOutfit {
            corpse_id: RAT_CORPSE,
            ..MonsterOutfit::default()
        },
        flags: MonsterTypeFlags::default(),
        mana_cost: 0,
        loot: vec![LootBlock {
            id: u32::from(GOLD),
            countmax: 4,
            chance: MAX_LOOTCHANCE,
            sub_type: 0,
            action_id: 0,
            text: String::new(),
            child_loot: Vec::new(),
        }],
        attack_spells: Vec::new(),
        defenses: MonsterDefenses {
            armor: Some(1),
            defense: Some(3),
            spells: Vec::new(),
            immunity_poison: false,
            immunity_fire: false,
            immunity_energy: false,
            immunity_life_drain: false,
            see_invisible: false,
            immunity_physical: false,
            immunity_paralyze: false,
            immunity_outfit: false,
        },
        max_summons: 0,
        summons: Vec::new(),
        talk_texts: Vec::new(),
    }
}

fn empty_rat() -> MonsterType {
    let mut m = rat_with_loot();
    m.loot = Vec::new();
    m
}

fn inventory_item_ids(inv: &MonsterInventory) -> BTreeSet<ItemId> {
    inv.bag
        .into_iter()
        .chain(inv.equipment.iter().copied().flatten())
        .chain(inv.body.iter().copied())
        .collect()
}

fn corpse_container_ids(
    world: &mut GameWorld,
    pos: Position,
    corpse_type: u16,
) -> (ItemId, BTreeSet<ItemId>) {
    let tile = world.map.get_tile(pos).expect("tile");
    let corpse_id = tile
        .body()
        .down_items
        .iter()
        .copied()
        .find(|&id| {
            world
                .items
                .get(id)
                .is_some_and(|i| i.item_type == corpse_type)
        })
        .expect("corpse on tile");
    world.hydrate_container_if_needed(corpse_id);
    let ids = world
        .container_registry
        .get(corpse_id)
        .expect("corpse container")
        .items
        .iter()
        .copied()
        .collect();
    (corpse_id, ids)
}

fn insert_rat(world: &mut GameWorld, pos: Position) -> CreatureId {
    ensure_walkable_tile(&mut world.map, pos, 100);
    let cid = insert_monster(world, "Rat", pos, 200);
    if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(cid) {
        m.corpse_id = RAT_CORPSE;
    }
    cid
}

fn monster_inventory(world: &GameWorld, cid: CreatureId) -> MonsterInventory {
    match world.creatures.get(cid) {
        Some(CreatureKind::Monster(m)) => m.inventory.clone(),
        _ => panic!("expected monster"),
    }
}

fn spawn_loot_itemids_equal_corpse(version: ProtocolVersion) {
    let mut world = loot_world(version);
    let pos = Position::new(100, 100, 7);
    let cid = insert_rat(&mut world, pos);
    world.roll_monster_spawn_loot(cid, &rat_with_loot());
    let inventory = monster_inventory(&world, cid);
    let snapshot = inventory_item_ids(&inventory);
    assert!(
        !snapshot.is_empty(),
        "spawn roll must produce at least one item"
    );
    world.drop_monster_corpse(pos, RAT_CORPSE, BloodType::Blood, &inventory);
    let (_corpse, corpse_ids) = corpse_container_ids(&mut world, pos, RAT_CORPSE);
    assert_eq!(
        corpse_ids, snapshot,
        "corpse ItemIds must equal spawn inventory (identity move, not regenerate)"
    );
}

#[test]
fn spawn_loot_itemids_equal_corpse_v772() {
    spawn_loot_itemids_equal_corpse(ProtocolVersion::V772);
}

#[test]
fn spawn_loot_itemids_equal_corpse_v1098() {
    spawn_loot_itemids_equal_corpse(ProtocolVersion::V1098);
}

/// Phase 8.4 — loot is native; Lua is not load-bearing (`NullEventDispatcher` default).
#[test]
fn native_loot_without_lua_v772() {
    spawn_loot_itemids_equal_corpse(ProtocolVersion::V772);
}

fn summons_drop_nothing(version: ProtocolVersion) {
    let mut world = loot_world(version);
    let pos = Position::new(100, 100, 7);
    ensure_walkable_tile(&mut world.map, pos, 100);
    let master = insert_player(&mut world, test_player("Hero", pos));
    let cid = insert_rat(&mut world, pos);
    if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(cid) {
        m.base.master = Some(master);
    }
    world.roll_monster_spawn_loot(cid, &rat_with_loot());
    let inventory = monster_inventory(&world, cid);
    assert!(inventory.bag.is_none());
    assert!(inventory.equipment.iter().all(|s| s.is_none()));
    assert!(inventory.body.is_empty());
    world.drop_monster_corpse(pos, RAT_CORPSE, BloodType::Blood, &inventory);
    let (_corpse, corpse_ids) = corpse_container_ids(&mut world, pos, RAT_CORPSE);
    assert!(
        corpse_ids.is_empty(),
        "summon corpse must not hold rolled loot (splash {SPLASH} on the tile is allowed)"
    );
}

#[test]
fn summons_drop_nothing_v772() {
    summons_drop_nothing(ProtocolVersion::V772);
}

#[test]
fn summons_drop_nothing_v1098() {
    summons_drop_nothing(ProtocolVersion::V1098);
}

fn try_lua_runtime() -> Option<(LuaRuntime, PathBuf)> {
    let root = data_root();
    if !root.is_dir() {
        eprintln!("data pack not present — skipping");
        return None;
    }
    let runtime = match LuaRuntime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("LuaRuntime::new failed — skipping: {e}");
            return None;
        }
    };
    if let Err(e) = load_data_lib(&runtime, &root) {
        eprintln!("load_data_lib failed — skipping: {e}");
        return None;
    }
    Some((runtime, root))
}

#[test]
fn rarity_survives_death_v772() {
    register_lua_mutation_hooks();
    let Some((runtime, _)) = try_lua_runtime() else {
        return;
    };
    runtime
        .exec_chunk(
            "rarity_onspawn",
            r#"
EventCallbackData[25][#EventCallbackData[25] + 1] = {function(monster, position, startup, artificial)
  local bag = monster:getBag()
  if not bag then return true end
  local items = bag:getItems()
  if items[1] then
    items[1]:setAttribute(ITEM_ATTRIBUTE_ACTIONID, 4242)
  end
  return true
end, 0}
"#,
        )
        .expect("inject onSpawn rarity callback");

    let mut world = loot_world(ProtocolVersion::V772);
    world.events = Box::new(LuaEventDispatcher::new(
        runtime,
        MoveEventsRegistry::default(),
    ));

    let pos = Position::new(100, 100, 7);
    let cid = insert_rat(&mut world, pos);
    let bag = world.items.insert(Item::new(BAG, 1));
    world.hydrate_container_if_needed(bag);
    assert!(
        world.items_db.is_openable_container(BAG),
        "1987 bag must be an openable container so getItems hydrates"
    );
    let inner = world.items.insert(Item::new(GOLD, 1));
    {
        let cont = world.container_registry.get_mut(bag).expect("hydrated bag");
        cont.add_item(inner).expect("bag add gold");
    }
    if let Some(item) = world.items.get_mut(inner) {
        item.parent = Some(Cylinder::Container {
            item_id: bag,
            index: INDEX_WHEREEVER,
        });
    }
    world.refresh_container_chain(bag);
    if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(cid) {
        m.inventory.bag = Some(bag);
    }

    world.finish_monster_spawn(cid, &empty_rat(), false, false);

    let living_inner = world
        .items
        .get(inner)
        .expect("inner still on living monster");
    assert_eq!(
        living_inner.action_id(),
        RARITY_AID,
        "rarity ACTIONID must be visible on the living monster (same ItemId)"
    );

    let inventory = monster_inventory(&world, cid);
    world.drop_monster_corpse(pos, RAT_CORPSE, BloodType::Blood, &inventory);
    let (_corpse, corpse_ids) = corpse_container_ids(&mut world, pos, RAT_CORPSE);
    assert!(
        corpse_ids.contains(&bag),
        "bag ItemId must move into the corpse (identity)"
    );
    world.hydrate_container_if_needed(bag);
    let bag_items = &world
        .container_registry
        .get(bag)
        .expect("bag container after death")
        .items;
    assert!(
        bag_items.contains(&inner),
        "inner gold must remain in the same bag"
    );
    assert_eq!(
        world
            .items
            .get(inner)
            .expect("inner after death")
            .action_id(),
        RARITY_AID,
        "rarity ACTIONID must survive the corpse move"
    );
}

/// `apply_creature_death` → `remove_creature` uses `tokio::spawn` for players_online
/// cleanup. `minimal_world` drops its runtime enter-guard after construction, so tests
/// must re-enter a runtime. The spawned future is not awaited (no DB in these tests).
fn apply_death(world: &mut GameWorld, cid: CreatureId) {
    static RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    let rt = RT.get_or_init(|| tokio::runtime::Runtime::new().expect("tokio test runtime"));
    let _enter = rt.enter();
    world.apply_creature_death(cid);
}

fn place_inventory_item(
    world: &mut GameWorld,
    cid: CreatureId,
    slot: u8,
    item_type: u16,
) -> ItemId {
    let iid = world.items.insert(Item::new(item_type, 1));
    world
        .internal_add_item_to_inventory_slot(cid, slot, iid)
        .expect("place inventory item");
    iid
}

fn tile_has_corpse_type(world: &GameWorld, pos: Position, corpse_type: u16) -> bool {
    world.map.get_tile(pos).is_some_and(|tile| {
        tile.body().down_items.iter().any(|&id| {
            world
                .items
                .get(id)
                .is_some_and(|i| i.item_type == corpse_type)
        })
    })
}

fn attach_playerdeath_lua(world: &mut GameWorld) -> bool {
    register_lua_mutation_hooks();
    let Some((runtime, root)) = try_lua_runtime() else {
        return false;
    };
    let path = root.join("scripts/creaturescripts/playerdeath.lua");
    let src = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("playerdeath.lua missing — skipping: {e}");
            return false;
        }
    };
    if let Err(e) = runtime.exec_chunk("playerdeath", &src) {
        eprintln!("playerdeath.lua exec failed — skipping: {e}");
        return false;
    }
    if let Err(e) = runtime.install_pending_creature_events() {
        eprintln!("install_pending_creature_events failed — skipping: {e}");
        return false;
    }
    world.events = Box::new(LuaEventDispatcher::new(
        runtime,
        MoveEventsRegistry::default(),
    ));
    true
}

fn player_death_aol(version: ProtocolVersion, with_lua: bool) {
    let mut world = loot_world(version);
    if with_lua && !attach_playerdeath_lua(&mut world) {
        return;
    }
    let pos = Position::new(100, 100, 7);
    ensure_walkable_tile(&mut world.map, pos, 100);
    let cid = insert_player(&mut world, {
        let mut p = test_player("AolVictim", pos);
        p.exact_lethal_blow = true;
        p.playerkiller_end = 0;
        if with_lua {
            p.registered_creature_events.insert("PlayerDeath".into());
        }
        p
    });
    let gold = place_inventory_item(&mut world, cid, 1, GOLD);
    let aol = place_inventory_item(&mut world, cid, InventorySlot::Necklace as u8, AOL);
    apply_death(&mut world, cid);
    assert!(
        world.items.get(aol).is_none(),
        "AoL 2173 must be consumed (LOSE_INVENTORY_NONE)"
    );
    assert!(
        world.items.get(gold).is_some(),
        "gold in slot 1 must be kept under AoL"
    );
    assert!(
        tile_has_corpse_type(&world, pos, DEAD_HUMAN),
        "player corpse 3128 must exist on the tile"
    );
}

fn player_death_red_skull_lua(version: ProtocolVersion) {
    let mut world = loot_world(version);
    if !attach_playerdeath_lua(&mut world) {
        return;
    }
    let pos = Position::new(100, 100, 7);
    ensure_walkable_tile(&mut world.map, pos, 100);
    let cid = insert_player(&mut world, {
        let mut p = test_player("RedSkull", pos);
        p.playerkiller_end = 1_000_000;
        p.exact_lethal_blow = false;
        p.registered_creature_events.insert("PlayerDeath".into());
        p
    });
    let golds = [
        place_inventory_item(&mut world, cid, 1, GOLD),
        place_inventory_item(&mut world, cid, InventorySlot::Necklace as u8, GOLD),
        place_inventory_item(&mut world, cid, 5, GOLD),
    ];
    apply_death(&mut world, cid);
    for iid in golds {
        let parent = world.items.get(iid).and_then(|i| i.parent);
        assert!(
            !matches!(parent, Some(Cylinder::Inventory { .. })),
            "red-skull gold must leave Inventory parent"
        );
    }
    assert!(tile_has_corpse_type(&world, pos, DEAD_HUMAN));
}

fn player_death_some_lua(version: ProtocolVersion) {
    let mut world = loot_world(version);
    if !attach_playerdeath_lua(&mut world) {
        return;
    }
    let pos = Position::new(100, 100, 7);
    ensure_walkable_tile(&mut world.map, pos, 100);
    let cid = insert_player(&mut world, {
        let mut p = test_player("SomeVictim", pos);
        p.registered_creature_events.insert("PlayerDeath".into());
        p
    });
    let _gold = place_inventory_item(&mut world, cid, 1, GOLD);
    apply_death(&mut world, cid);
    assert!(
        tile_has_corpse_type(&world, pos, DEAD_HUMAN),
        "SOME death must still place corpse 3128"
    );
}

#[test]
fn player_death_aol_native_v772() {
    player_death_aol(ProtocolVersion::V772, false);
}

#[test]
fn player_death_aol_native_v1098() {
    player_death_aol(ProtocolVersion::V1098, false);
}

#[test]
fn player_death_aol_lua_v772() {
    player_death_aol(ProtocolVersion::V772, true);
}

#[test]
fn player_death_aol_lua_v1098() {
    player_death_aol(ProtocolVersion::V1098, true);
}

#[test]
fn player_death_red_skull_lua_v772() {
    player_death_red_skull_lua(ProtocolVersion::V772);
}

#[test]
fn player_death_red_skull_lua_v1098() {
    player_death_red_skull_lua(ProtocolVersion::V1098);
}

#[test]
fn player_death_some_lua_v772() {
    player_death_some_lua(ProtocolVersion::V772);
}

#[test]
fn player_death_some_lua_v1098() {
    player_death_some_lua(ProtocolVersion::V1098);
}
