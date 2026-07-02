//! 772 monster spawn loot, bag/equip routing, and equipment-derived combat stats.
//!
//! - `TMonster::TMonster` loot roll — `crnonpl.cc:2050-2103`.
//! - `CheckCombatValues` / `GetWeapon` / `GetArmorStrength` — `crcombat.cc:128,36,286`.

use rand::Rng;

use tfs_rust_common::enums::BloodType;
use tfs_rust_common::Position;
use tfs_rust_content::monsters::{LootBlock, MonsterType, MAX_LOOTCHANCE};
use tfs_rust_content::otb::ItemType;

use crate::container::Container;
use crate::creature::CreatureKind;
use crate::cylinder::CylinderFlags;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::{item_fits_equipment_slot, slot_to_array_index, WEAPON_NONE, WEAPON_SHIELD};
use crate::item::Item;

/// Default internal bag for spawn-rolled loot — TVP `item id="1987"`.
pub const DEFAULT_MONSTER_BAG_TYPE: u16 = 1987;

/// 1098 generic corpse fallback when race corpse is unset.
pub const GENERIC_CORPSE_TYPE: u16 = 3058;

/// Blood/slime splash item ids (TFS data pack — `src/const.h` `ITEM_FULLSPLASH`/`ITEM_SMALLSPLASH`;
/// CipSoft special objects `BLOOD_POOL`/`BLOOD_SPLASH`, `tibia-game-master/src/enums.hh:609`).
pub const ITEM_FULLSPLASH: u16 = 2016;
pub const ITEM_SMALLSPLASH: u16 = 2019;

/// 772 physical-hit graphical effect keyed on blood family (raw client wire effect byte).
/// C++ `TCreature::Damage` physical switch — `tibia-game-master/src/crmain.cc:709-745`; matches
/// tvp-772 `gameserver/src/game.cpp` `combatGetTypeInfo`. Values are shared 772/1098 `CONST_ME_*`.
pub(crate) fn physical_hit_effect_772(blood: BloodType) -> u8 {
    match blood {
        BloodType::Blood => 1,   // CONST_ME_DRAWBLOOD / EFFECT_BLOOD_HIT
        BloodType::Slime => 17,  // CONST_ME_HITBYPOISON / EFFECT_POISON_HIT
        BloodType::Bones => 10,  // CONST_ME_HITAREA / EFFECT_BONE_HIT
        BloodType::Fire => 1,    // CONST_ME_DRAWBLOOD (orange damage text in C++)
        BloodType::Energy => 12, // CONST_ME_ENERGYHIT / EFFECT_ENERGY_HIT
    }
}

/// Splash liquid subtype for a blood family — only BLOOD/SLIME leave a ground splash
/// (`crmain.cc:711-722`). tvp-772 `FluidTypes_t` (`gameserver/src/const.h:94`): `FLUID_BLOOD = 5`,
/// `FLUID_SLIME = 6`. The 772 codec maps these via `getLiquidColor` (5→2, 6→4).
pub(crate) fn splash_fluid_772(blood: BloodType) -> Option<u16> {
    match blood {
        BloodType::Blood => Some(5),
        BloodType::Slime => Some(6),
        _ => None,
    }
}

/// Spawn-rolled body items — mirror player equip + optional bag.
#[derive(Debug, Clone, Default)]
pub struct MonsterInventory {
    /// Player-style slots 1..=10 (`equipment[0]` = slot 1).
    pub equipment: [Option<ItemId>; 11],
    /// Internal bag item when any loot goes to bag; empty bag omitted (`crnonpl.cc:2100`).
    pub bag: Option<ItemId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LootDestination {
    Bag,
    Equip(u8),
}

/// Whether rolled loot goes to internal bag or an equip slot (`crnonpl.cc:2050-2103`).
fn loot_destination(item_type: &ItemType) -> LootDestination {
    if item_type.weapon_type != WEAPON_NONE {
        return LootDestination::Bag;
    }
    if item_type.stackable() {
        return LootDestination::Bag;
    }
    for slot in [1u8, 2, 4, 5, 6, 7, 8, 9, 10] {
        if item_fits_equipment_slot(slot, item_type) {
            return LootDestination::Equip(slot);
        }
    }
    LootDestination::Bag
}

fn roll_loot_count<R: Rng + ?Sized>(rng: &mut R, countmax: i32) -> u16 {
    let max = countmax.max(1);
    rng.gen_range(1..=max) as u16
}

fn roll_loot_count_glibc(world: &GameWorld, countmax: i32) -> u16 {
    let max = countmax.max(1);
    world.parity_random(1, max) as u16
}

fn loot_block_passes_glibc(world: &GameWorld, chance: i32) -> bool {
    if chance <= 0 {
        return false;
    }
    if chance >= MAX_LOOTCHANCE {
        return true;
    }
    // C++ `TMonster` ctor — `random(0, 999) > Probability` skip (`crnonpl.cc:2056`).
    let cip_prob = (chance * 1000) / MAX_LOOTCHANCE;
    world.parity_random(0, 999) <= cip_prob
}

fn roll_loot_block_glibc(
    world: &mut GameWorld,
    block: &LootBlock,
    registry: &mut crate::container::ContainerRegistry,
    owner: CreatureId,
) -> Option<ItemId> {
    if !loot_block_passes_glibc(world, block.chance) {
        return None;
    }
    let server_id = block.id as u16;
    let _item_type = world.items_db.items.get(&server_id)?;
    let count = roll_loot_count_glibc(world, block.countmax);

    let mut item = Item::new(server_id, count);
    if block.sub_type != 0 {
        item.set_duration(block.sub_type);
    }
    if block.action_id != 0 {
        item.set_action_id(block.action_id as u16);
    }
    if !block.text.is_empty() {
        item.set_text(block.text.clone());
    }

    let item_id = world.items.insert(item);

    if !block.child_loot.is_empty() && world.items_db.is_container(server_id) {
        let cap = world.container_capacity(server_id);
        let mut container = Container::new(item_id, cap);
        for child in &block.child_loot {
            if let Some(child_id) = roll_loot_block_glibc(world, child, registry, owner) {
                let _ = container.add_item(child_id);
                if let Some(ch) = registry.get_mut(child_id) {
                    ch.parent_container = Some(item_id);
                }
            }
        }
        registry.register(container);
    }

    Some(item_id)
}

fn roll_loot_block<R: Rng + ?Sized>(
    world: &mut GameWorld,
    rng: &mut R,
    block: &LootBlock,
    registry: &mut crate::container::ContainerRegistry,
    owner: CreatureId,
) -> Option<ItemId> {
    if block.chance > 0 && block.chance < MAX_LOOTCHANCE {
        if rng.gen_range(0..MAX_LOOTCHANCE) >= block.chance {
            return None;
        }
    } else if block.chance <= 0 {
        return None;
    }

    let server_id = block.id as u16;
    let _item_type = world.items_db.items.get(&server_id)?;
    let count = roll_loot_count(rng, block.countmax);

    let mut item = Item::new(server_id, count);
    if block.sub_type != 0 {
        item.set_duration(block.sub_type);
    }
    if block.action_id != 0 {
        item.set_action_id(block.action_id as u16);
    }
    if !block.text.is_empty() {
        item.set_text(block.text.clone());
    }

    let item_id = world.items.insert(item);

    if !block.child_loot.is_empty() && world.items_db.is_container(server_id) {
        let cap = world.container_capacity(server_id);
        let mut container = Container::new(item_id, cap);
        for child in &block.child_loot {
            if let Some(child_id) = roll_loot_block(world, rng, child, registry, owner) {
                let _ = container.add_item(child_id);
                if let Some(ch) = registry.get_mut(child_id) {
                    ch.parent_container = Some(item_id);
                }
            }
        }
        registry.register(container);
    }

    Some(item_id)
}

/// Combat stats after equipped loot — weapon override + slot-matched armor sum.
pub fn effective_monster_combat_stats(
    base_skill: i32,
    base_attack: i32,
    base_armor: i32,
    inventory: &MonsterInventory,
    items: &slotmap::SlotMap<ItemId, Item>,
    items_db: &tfs_rust_content::items::ItemDatabase,
) -> (i32, i32, i32) {
    let skill = base_skill;
    let mut attack = base_attack;
    let mut armor = base_armor;

    for slot in [5u8, 6] {
        let Some(idx) = slot_to_array_index(slot) else {
            continue;
        };
        let Some(item_id) = inventory.equipment[idx] else {
            continue;
        };
        let Some(item) = items.get(item_id) else {
            continue;
        };
        let Some(it) = items_db.items.get(&item.item_type) else {
            continue;
        };
        if it.weapon_type != WEAPON_NONE && it.weapon_type != WEAPON_SHIELD {
            if it.attack > 0 {
                attack = it.attack;
            }
            break;
        }
    }

    for slot in [1u8, 4, 7, 8] {
        let Some(idx) = slot_to_array_index(slot) else {
            continue;
        };
        let Some(item_id) = inventory.equipment[idx] else {
            continue;
        };
        let Some(item) = items.get(item_id) else {
            continue;
        };
        let Some(it) = items_db.items.get(&item.item_type) else {
            continue;
        };
        if item_fits_equipment_slot(slot, it) && it.armor > 0 {
            armor = armor.saturating_add(it.armor);
        }
    }

    let _ = skill; // race melee_skill until item pack exposes weapon skill
    (skill, attack, armor)
}

impl GameWorld {
    /// Roll `MonsterType.loot` once at spawn; skip summons (`Master != 0`).
    ///
    /// C++ reference: `TMonster::TMonster` — `crnonpl.cc:2050`.
    pub(crate) fn roll_monster_spawn_loot(&mut self, monster_id: CreatureId, mtype: &MonsterType) {
        if !self.beat_driven_loop {
            return;
        }
        if self
            .creatures
            .get(monster_id)
            .is_some_and(|k| k.base().master.is_some())
        {
            return;
        }
        if mtype.loot.is_empty() {
            return;
        }

        let mut rolled: Vec<(LootDestination, ItemId)> = Vec::new();

        if self.beat_driven_loop || crate::sim_glibc_rand::sim_glibc_rng_enabled() {
            let mut registry = std::mem::take(&mut self.container_registry);
            for block in &mtype.loot {
                let Some(item_id) = roll_loot_block_glibc(self, block, &mut registry, monster_id)
                else {
                    continue;
                };
                let dest = self
                    .items_db
                    .items
                    .get(&(block.id as u16))
                    .map(loot_destination)
                    .unwrap_or(LootDestination::Bag);
                rolled.push((dest, item_id));
            }
            self.container_registry = registry;
        } else {
            self.with_ai_rng(|rng, world| {
                let mut registry = std::mem::take(&mut world.container_registry);
                for block in &mtype.loot {
                    let Some(item_id) =
                        roll_loot_block(world, rng, block, &mut registry, monster_id)
                    else {
                        continue;
                    };
                    let dest = world
                        .items_db
                        .items
                        .get(&(block.id as u16))
                        .map(loot_destination)
                        .unwrap_or(LootDestination::Bag);
                    rolled.push((dest, item_id));
                }
                world.container_registry = registry;
            });
        }

        let mut bag_items: Vec<ItemId> = Vec::new();
        let mut equip_placements: Vec<(u8, ItemId)> = Vec::new();

        for (dest, item_id) in rolled {
            match dest {
                LootDestination::Bag => bag_items.push(item_id),
                LootDestination::Equip(slot) => equip_placements.push((slot, item_id)),
            }
        }

        let mut bag_id: Option<ItemId> = None;
        if !bag_items.is_empty() {
            let bag_item = self.items.insert(Item::new(DEFAULT_MONSTER_BAG_TYPE, 1));
            let cap = self.container_capacity(DEFAULT_MONSTER_BAG_TYPE);
            let mut bag = Container::new(bag_item, cap);
            let mut registry = std::mem::take(&mut self.container_registry);
            for item_id in bag_items {
                let _ = bag.add_item(item_id);
                if let Some(ch) = registry.get_mut(item_id) {
                    ch.parent_container = Some(bag_item);
                }
            }
            registry.register(bag);
            self.container_registry = registry;
            self.refresh_container_chain(bag_item);
            bag_id = Some(bag_item);
        }

        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) {
            m.inventory.bag = bag_id;
            for (slot, item_id) in equip_placements {
                let Some(idx) = slot_to_array_index(slot) else {
                    if let Some(bag) = bag_id {
                        if let Some(cont) = self.container_registry.get_mut(bag) {
                            let _ = cont.add_item(item_id);
                            if let Some(ch) = self.container_registry.get_mut(item_id) {
                                ch.parent_container = Some(bag);
                            }
                        }
                    }
                    continue;
                };
                if m.inventory.equipment[idx].is_none() {
                    m.inventory.equipment[idx] = Some(item_id);
                } else if let Some(bag) = bag_id {
                    if let Some(cont) = self.container_registry.get_mut(bag) {
                        let _ = cont.add_item(item_id);
                        if let Some(ch) = self.container_registry.get_mut(item_id) {
                            ch.parent_container = Some(bag);
                        }
                    }
                }
            }
        }
    }

    /// Recompute melee/armor from spawn inventory — `CheckCombatValues` (`crcombat.cc:128`).
    pub(crate) fn recompute_monster_combat_from_equipment(&mut self, monster_id: CreatureId) {
        if !self.beat_driven_loop {
            return;
        }
        let snapshot = self.creatures.get(monster_id).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            Some((m.melee_skill, m.melee_attack, m.armor, m.inventory.clone()))
        });
        let Some((base_skill, base_attack, base_armor, inventory)) = snapshot else {
            return;
        };
        let (skill, attack, armor) = effective_monster_combat_stats(
            base_skill,
            base_attack,
            base_armor,
            &inventory,
            &self.items,
            &self.items_db,
        );
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) {
            m.melee_skill = skill;
            m.melee_attack = attack;
            m.armor = armor;
        }
    }

    fn move_body_item_into_corpse(&mut self, corpse_id: ItemId, item_id: ItemId) {
        self.hydrate_container_if_needed(corpse_id);
        if let Some(cont) = self.container_registry.get_mut(corpse_id) {
            if cont.is_full() {
                return;
            }
            let _ = cont.add_item(item_id);
        }
        if let Some(ch) = self.container_registry.get_mut(item_id) {
            ch.parent_container = Some(corpse_id);
        }
        self.refresh_container_chain(corpse_id);
    }

    /// Drop race corpse with bag + equipped loot on the death tile (772 only).
    ///
    /// C++ reference: `~TCreature` — `crmain.cc:204-290`.
    pub(crate) fn drop_monster_corpse_772(
        &mut self,
        pos: tfs_rust_common::Position,
        corpse_type: u16,
        blood: BloodType,
        inventory: &MonsterInventory,
    ) {
        if !self.beat_driven_loop {
            return;
        }
        let corpse_type = if corpse_type > 0 {
            corpse_type
        } else {
            GENERIC_CORPSE_TYPE
        };

        let corpse_id = self.items.insert(Item::new(corpse_type, 1));
        self.hydrate_container_if_needed(corpse_id);

        if let Some(bag) = inventory.bag {
            self.move_body_item_into_corpse(corpse_id, bag);
        }
        for slot_item in inventory.equipment.iter().flatten() {
            self.move_body_item_into_corpse(corpse_id, *slot_item);
        }

        // C++ `~TCreature` creates the blood/slime pool BEFORE the corpse (`crmain.cc:210-226`),
        // keyed on the race blood family — NOT on the corpse item's `fluidsource` attribute
        // (audit finding #14). Only BLOOD/SLIME races pool.
        if let Some(fluid) = splash_fluid_772(blood) {
            self.create_liquid_splash_772(pos, ITEM_FULLSPLASH, fluid);
        }

        let decay_clock = if self.beat_driven_loop {
            self.now_ms()
        } else {
            self.tick_counter
        };
        let decay_unit_ms = if self.beat_driven_loop { 50 } else { 1 };
        let (deadline, replace_with) =
            item_decay_schedule(&self.items_db, corpse_type, decay_clock, decay_unit_ms);
        self.decay.schedule(corpse_id, deadline, replace_with);

        if self
            .internal_add_item_to_tile(pos, corpse_id, CylinderFlags::NO_LIMIT)
            .is_err()
        {
            tracing::warn!(
                ?pos,
                corpse_type,
                "monster corpse could not be placed on tile"
            );
        }
    }

    /// Blood family of a creature — monsters carry it from their race; players and NPCs are
    /// `RACE_BLOOD` (`player.h:519` `getRace()`).
    pub(crate) fn creature_blood_type(&self, cid: CreatureId) -> BloodType {
        match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => m.blood,
            _ => BloodType::Blood,
        }
    }

    /// Create a decaying liquid splash/pool item on a tile (772 only).
    /// C++ `CreatePool(GetSpecialObject(BLOOD_SPLASH|BLOOD_POOL), liquid)` — `crmain.cc:216,771`.
    ///
    /// The splash renders its colour from the item's `count` byte (the tile wire path encodes
    /// `Item::client_count()`, and the codec runs it through 772 `getLiquidColor` / 1098 `fluidMap`),
    /// so the fluid subtype is stored in `count`. The `fluid_type` attribute is mirrored for
    /// `Item::get_sub_type` parity on the container/query paths.
    pub(crate) fn create_liquid_splash_772(
        &mut self,
        pos: Position,
        splash_item_id: u16,
        fluid_subtype: u16,
    ) {
        if !self.beat_driven_loop {
            return;
        }
        // C++ `Tile::addThing` removes any existing splash before adding a new one
        // (`tile.cpp:881-894`) — a tile holds at most ONE splash. Without this, sustained combat
        // (e.g. a bear meleeing the player each beat) piles splash items onto the victim's tile,
        // overflowing the client's 10-object tile stack and desyncing it.
        let existing_splashes: Vec<ItemId> = self
            .map
            .get_tile(pos)
            .map(|t| {
                let b = t.body();
                b.top_items
                    .iter()
                    .chain(b.down_items.iter())
                    .copied()
                    .filter(|&iid| {
                        self.items
                            .get(iid)
                            .and_then(|it| self.items_db.items.get(&it.item_type))
                            .is_some_and(|ty| ty.is_splash())
                    })
                    .collect()
            })
            .unwrap_or_default();
        for iid in existing_splashes {
            let _ = self.internal_remove_item_from_tile(pos, iid, u16::MAX);
        }

        let mut item = Item::new(splash_item_id, fluid_subtype);
        item.set_fluid_type(fluid_subtype);
        let id = self.items.insert(item);
        let clock = self.now_ms();
        let (deadline, replace_with) =
            item_decay_schedule(&self.items_db, splash_item_id, clock, 50);
        self.decay.schedule(id, deadline, replace_with);
        if self
            .internal_add_item_to_tile(pos, id, CylinderFlags::NO_LIMIT)
            .is_err()
        {
            self.items.remove(id);
        }
    }

    /// Emit the 772 physical-hit blood visual: the race-keyed hit effect plus a blood/slime
    /// small-splash on the victim's tile. C++ `TCreature::Damage` physical branch
    /// (`crmain.cc:762-775`). No PZ gate in 772 (unlike 1098). Call on physical damage that lands.
    pub(crate) fn apply_physical_hit_blood_772(&mut self, target: CreatureId, pos: Position) {
        if !self.beat_driven_loop {
            return;
        }
        let blood = self.creature_blood_type(target);
        self.broadcast_magic_effect(pos, physical_hit_effect_772(blood));
        if let Some(fluid) = splash_fluid_772(blood) {
            self.create_liquid_splash_772(pos, ITEM_SMALLSPLASH, fluid);
        }
    }
}

fn item_decay_schedule(
    items_db: &tfs_rust_content::items::ItemDatabase,
    server_id: u16,
    clock: u64,
    unit_ms: u64,
) -> (u64, Option<u16>) {
    let Some(it) = items_db.items.get(&server_id) else {
        return (clock.saturating_add(600u64.saturating_mul(unit_ms)), None);
    };
    let duration: u64 = it
        .xml_attributes
        .get("duration")
        .and_then(|s| s.parse().ok())
        .unwrap_or(600);
    let decayto = it
        .xml_attributes
        .get("decayto")
        .and_then(|s| s.parse::<u16>().ok())
        .filter(|&id| id > 0);
    (
        clock.saturating_add(duration.saturating_mul(unit_ms)),
        decayto,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use slotmap::SlotMap;
    use tfs_rust_common::enums::CombatType;
    use tfs_rust_common::Position;
    use tfs_rust_content::items::ItemDatabase;
    use tfs_rust_content::monsters::{
        LootBlock, MonsterDefenses, MonsterOutfit, MonsterType, MonsterTypeFlags, MAX_LOOTCHANCE,
    };
    use tfs_rust_content::otb::ItemType;

    use super::{effective_monster_combat_stats, MonsterInventory};
    use crate::combat::{CombatDamage, CombatParams};
    use crate::creature::{CreatureKind, MonsterAiConfig};
    use crate::game_world::GameWorld;
    use crate::ids::{CreatureId, ItemId};
    use crate::inventory::{SLOTP_ARMOR, WEAPON_SWORD};
    use crate::item::Item;
    use crate::sim_harness::{bag_item_type, insert_monster, pickup_item_type};
    use crate::test_world::support::{
        ensure_walkable_tile, insert_player, minimal_world, test_player,
    };

    fn armor_item_type(server_id: u16, armor: i32) -> ItemType {
        let mut it = pickup_item_type(server_id);
        it.armor = armor;
        it.slot_position = SLOTP_ARMOR;
        it
    }

    fn sword_item_type(server_id: u16, attack: i32) -> ItemType {
        let mut it = pickup_item_type(server_id);
        it.weapon_type = WEAPON_SWORD;
        it.attack = attack;
        it
    }

    fn beat_world(items: HashMap<u16, ItemType>) -> GameWorld {
        let mut world = minimal_world();
        world.mechanics =
            crate::formulas::Mechanics::for_version(tfs_rust_common::ProtocolVersion::V772);
        world.beat_driven_loop = true;
        world.items_db = Arc::new(ItemDatabase {
            items: items,
            client_to_server: HashMap::new(),
        });
        world
    }

    fn insert_test_monster(world: &mut GameWorld, pos: Position) -> CreatureId {
        ensure_walkable_tile(&mut world.map, pos, 150);
        insert_monster(world, "Rat", pos, 200)
    }

    #[test]
    fn test_e6_equipped_armor_increases_armor() {
        let mut items = HashMap::new();
        items.insert(2464u16, armor_item_type(2464, 10));
        let mut world_items = SlotMap::<ItemId, Item>::with_key();
        let mut inv = MonsterInventory::default();
        let armor_id = world_items.insert(Item::new(2464, 1));
        inv.equipment[3] = Some(armor_id);

        let items_db = ItemDatabase {
            items,
            client_to_server: HashMap::new(),
        };
        let (skill, attack, armor) =
            effective_monster_combat_stats(15, 7, 1, &inv, &world_items, &items_db);
        assert_eq!(skill, 15);
        assert_eq!(attack, 7);
        assert_eq!(armor, 11, "race armor 1 + equipped chain 10");
    }

    #[test]
    fn test_e6_weapon_overrides_attack() {
        let mut items = HashMap::new();
        items.insert(2406u16, sword_item_type(2406, 25));
        let mut world_items = SlotMap::<ItemId, Item>::with_key();
        let mut inv = MonsterInventory::default();
        let sword_id = world_items.insert(Item::new(2406, 1));
        inv.equipment[4] = Some(sword_id);

        let items_db = ItemDatabase {
            items,
            client_to_server: HashMap::new(),
        };
        let (_, attack, _) =
            effective_monster_combat_stats(15, 7, 1, &inv, &world_items, &items_db);
        assert_eq!(attack, 25);
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
                corpse_id: 2813,
                ..MonsterOutfit::default()
            },
            flags: MonsterTypeFlags::default(),
            loot: vec![LootBlock {
                id: 2148,
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
                see_invisible: false,
            },
            talk_texts: Vec::new(),
        }
    }

    fn stackable_pickup_item_type(server_id: u16) -> ItemType {
        let mut it = pickup_item_type(server_id);
        it.flags |= 1 << 7; // ItemType::FLAG_STACKABLE
        it
    }

    #[test]
    fn test_e6_corpse_contains_spawn_loot() {
        let mut items = HashMap::new();
        items.insert(1987u16, bag_item_type(1987));
        items.insert(2148u16, stackable_pickup_item_type(2148));
        items.insert(2813u16, {
            let mut c = bag_item_type(2813);
            c.xml_attributes.insert("containersize".into(), "5".into());
            c
        });
        let mut world = beat_world(items);
        world.ai_rng = StdRng::seed_from_u64(42);

        let pos = Position::new(100, 100, 7);
        let monster = insert_test_monster(&mut world, pos);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.corpse_id = 2813;
        }

        world.roll_monster_spawn_loot(monster, &rat_with_loot());

        assert!(
            world
                .creatures
                .get(monster)
                .and_then(|k| match k {
                    CreatureKind::Monster(m) => m.inventory.bag,
                    _ => None,
                })
                .is_some(),
            "guaranteed gold loot must create a bag"
        );

        let inventory = world
            .creatures
            .get(monster)
            .and_then(|k| match k {
                CreatureKind::Monster(m) => Some(m.inventory.clone()),
                _ => None,
            })
            .expect("monster");

        world.drop_monster_corpse_772(
            pos,
            2813,
            tfs_rust_common::enums::BloodType::Blood,
            &inventory,
        );

        let tile = world.map.get_tile(pos).expect("tile");
        let corpse_item_id = tile
            .body()
            .down_items
            .iter()
            .find(|id| world.items.get(**id).is_some_and(|i| i.item_type == 2813))
            .copied()
            .expect("corpse on tile");

        world.hydrate_container_if_needed(corpse_item_id);
        let cont = world
            .container_registry
            .get(corpse_item_id)
            .expect("corpse container");
        assert!(
            !cont.items.is_empty(),
            "corpse must contain spawn-rolled bag/loot"
        );
    }

    #[test]
    fn test_e6_summon_spawns_without_loot() {
        let mut items = HashMap::new();
        items.insert(1987u16, bag_item_type(1987));
        items.insert(2148u16, stackable_pickup_item_type(2148));
        let mut world = beat_world(items);
        world.ai_rng = StdRng::seed_from_u64(99);

        let pos = Position::new(100, 100, 7);
        let player = insert_player(&mut world, test_player("Hero", pos));
        let monster = insert_test_monster(&mut world, pos);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.master = Some(player);
        }

        world.roll_monster_spawn_loot(monster, &rat_with_loot());

        let m = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m,
            _ => panic!("monster"),
        };
        assert!(m.inventory.bag.is_none());
        assert!(m.inventory.equipment.iter().all(|s| s.is_none()));
    }

    #[test]
    fn test_e6_rat_experience_on_death() {
        let mut world = beat_world(HashMap::new());
        let pos = Position::new(100, 100, 7);
        let player = insert_player(&mut world, test_player("Hero", pos));
        let exp_before = match world.creatures.get(player) {
            Some(CreatureKind::Player(p)) => p.experience,
            _ => 0,
        };

        let monster = insert_test_monster(&mut world, pos);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.experience = 5;
        }

        let applied = world.combat_execute_with_stimulus(
            Some(player),
            monster,
            &CombatDamage {
                primary: (CombatType::Physical, -100),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams::default(),
        );
        assert!(applied, "lethal hit must apply");
        assert!(
            world.creatures.get(monster).is_none(),
            "monster must be removed after death hook"
        );

        let exp_after = match world.creatures.get(player) {
            Some(CreatureKind::Player(p)) => p.experience,
            _ => 0,
        };
        assert_eq!(
            exp_after.saturating_sub(exp_before),
            5,
            "rat race experience=5 must grant to sole killer"
        );
    }
}
