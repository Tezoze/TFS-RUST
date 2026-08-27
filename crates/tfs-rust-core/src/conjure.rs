//! Native `Player:conjureItem` — rune blanks, arrows, enchant staff.
//!
//! Pack surface: TFS `data/scripts/functions.lua` `Player:conjureItem`.
//! Hands-only vs backpack is `MechanicsProfile.conjure_from_hands_only`
//! (`formulas.conjureFromHandsOnly`). Spell scripts keep calling
//! `creature:conjureItem(...)`.
//!
//! C++ reference: TFS pack Lua (not a decompile `Conjure` opcode). Dual-hand
//! extra mana uses the numeric cost (or `Spell.mana` when the first arg is a
//! Spell userdata).

use slotmap::Key;
use tfs_rust_lua::ConjureRequest;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::InventorySlot;
use crate::player::flags::PLAYER_FLAG_HAS_INFINITE_MANA;
use crate::return_value::ReturnValue;
use tfs_rust_common::ScriptContext;

/// `const.h` `CONST_ME_POFF`.
const CONST_ME_POFF: u8 = 3;
/// `itemAttrTypes` `ITEM_ATTRIBUTE_DURATION`.
const ITEM_ATTRIBUTE_DURATION: u32 = 1 << 17;

/// Apply pack `conjureItem`. Returns Lua boolean.
pub fn apply(world: &mut GameWorld, req: &ConjureRequest) -> bool {
    let Some(cid) = world.resolve_creature_u64(req.player) else {
        return false;
    };
    if !matches!(world.creatures.get(cid), Some(CreatureKind::Player(_))) {
        return false;
    }

    let Some(count) = resolve_conjure_count(world, req.conjure_id, req.conjure_count) else {
        return false;
    };

    if world.mechanics.profile.conjure_from_hands_only {
        conjure_from_hands(world, cid, req, count)
    } else {
        conjure_from_inventory(world, cid, req, count)
    }
}

/// Lua `if not conjureCount` (nil or 0) then `ItemType:getCharges()`.
/// `None` = unknown `conjureId` (`ItemType:getId() == 0`).
fn resolve_conjure_count(
    world: &GameWorld,
    conjure_id: u16,
    conjure_count: Option<u32>,
) -> Option<Option<u32>> {
    let missing = conjure_count.is_none_or(|c| c == 0);
    if !missing {
        return Some(conjure_count);
    }
    if conjure_id == 0 {
        return Some(conjure_count);
    }
    if !world.items_db.items.contains_key(&conjure_id) {
        return None;
    }
    let charges = world
        .items_db
        .items
        .get(&conjure_id)
        .map(|t| t.charges)
        .unwrap_or(0);
    if charges != 0 {
        Some(Some(charges))
    } else {
        Some(conjure_count)
    }
}

fn conjure_from_inventory(
    world: &mut GameWorld,
    cid: CreatureId,
    req: &ConjureRequest,
    count: Option<u32>,
) -> bool {
    let player = req.player;
    if req.reagent_id != 0 {
        let Some(reagent) = world.find_item_of_type(cid, req.reagent_id, true, -1) else {
            return fail(world, cid, ReturnValue::YouNeedAMagicItemToCastSpell);
        };
        let sub = count.map(|c| c as i32).unwrap_or(-1);
        if world
            .lua_script_item_transform(reagent.data().as_ffi(), req.conjure_id, sub)
            .ok()
            != Some(true)
        {
            return fail(world, cid, ReturnValue::NotPossible);
        }
        maybe_start_duration_decay(world, reagent);
        success_effect(world, cid, req.effect);
        return true;
    }

    let add_count = count.unwrap_or(1);
    let Ok(Some(iid)) = world.lua_script_player_add_item_full(
        player,
        req.conjure_id,
        add_count,
        -1,
        true,
        0,
    ) else {
        return fail(world, cid, ReturnValue::NotPossible);
    };
    if let Some(item) = world.resolve_item_u64(iid) {
        maybe_start_duration_decay(world, item);
    }
    success_effect(world, cid, req.effect);
    true
}

fn conjure_from_hands(
    world: &mut GameWorld,
    cid: CreatureId,
    req: &ConjureRequest,
    count: Option<u32>,
) -> bool {
    let player = req.player;
    let left = InventorySlot::Left as u8;
    let right = InventorySlot::Right as u8;
    let left_item = world.get_player_inventory_item(cid, left);
    let right_item = world.get_player_inventory_item(cid, right);

    if req.reagent_id != 0 {
        let mut total: u32 = 0;
        if slot_is_reagent(world, left_item, req.reagent_id) {
            let Some(iid) = left_item else {
                return fail(world, cid, ReturnValue::NotPossible);
            };
            let _ = world.lua_script_item_remove(iid.data().as_ffi(), -1);
            let sub = count.map(|c| c as i32).unwrap_or(-1);
            let Ok(Some(new_id)) =
                world.lua_script_player_add_item_full(player, req.conjure_id, 1, sub, true, left)
            else {
                return fail(world, cid, ReturnValue::NotPossible);
            };
            if let Some(item) = world.resolve_item_u64(new_id) {
                maybe_start_duration_decay(world, item);
            }
            success_effect(world, cid, req.effect);
            total = 1;
        }

        if slot_is_reagent(world, right_item, req.reagent_id) {
            let infinite = world.player_has_flag(cid, PLAYER_FLAG_HAS_INFINITE_MANA);
            let mana = world
                .creatures
                .get(cid)
                .and_then(|k| match k {
                    CreatureKind::Player(p) => Some(p.mana),
                    _ => None,
                })
                .unwrap_or(0);
            let need_two = req.mana_cost.saturating_mul(2);
            let not_enough = mana < need_two && !infinite;
            if total == 1 && not_enough {
                return true;
            }
            if total == 1 {
                let _ = world.lua_script_player_add_mana(player, -req.mana_cost);
                let spent = req.mana_cost.max(0) as u64;
                let _ = world.lua_script_player_add_mana_spent(player, spent);
            }
            let Some(iid) = right_item else {
                return fail(world, cid, ReturnValue::NotPossible);
            };
            let _ = world.lua_script_item_remove(iid.data().as_ffi(), -1);
            let sub = count.map(|c| c as i32).unwrap_or(-1);
            let Ok(Some(new_id)) =
                world.lua_script_player_add_item_full(player, req.conjure_id, 1, sub, true, right)
            else {
                return fail(world, cid, ReturnValue::NotPossible);
            };
            if let Some(item) = world.resolve_item_u64(new_id) {
                maybe_start_duration_decay(world, item);
            }
            success_effect(world, cid, req.effect);
            total += 1;
        }

        if total == 0 {
            return fail(world, cid, ReturnValue::YouNeedAMagicItemToCastSpell);
        }
        return true;
    }

    let add_count = count.unwrap_or(1);
    let Ok(Some(iid)) = world.lua_script_player_add_item_full(
        player,
        req.conjure_id,
        add_count,
        -1,
        true,
        0,
    ) else {
        return fail(world, cid, ReturnValue::NotPossible);
    };
    if let Some(item) = world.resolve_item_u64(iid) {
        maybe_start_duration_decay(world, item);
    }
    success_effect(world, cid, req.effect);
    true
}

fn slot_is_reagent(world: &GameWorld, slot: Option<ItemId>, reagent_id: u16) -> bool {
    slot.and_then(|iid| world.items.get(iid))
        .is_some_and(|item| item.item_type == reagent_id)
}

fn maybe_start_duration_decay(world: &mut GameWorld, iid: ItemId) {
    if world.item_has_attribute(iid.data().as_ffi(), ITEM_ATTRIBUTE_DURATION) {
        world.start_decay(iid);
    }
}

fn fail(world: &mut GameWorld, cid: CreatureId, rv: ReturnValue) -> bool {
    let _ = world.lua_script_player_send_cancel_message(cid.data().as_ffi(), rv.description().into());
    if let Some(pos) = world.creatures.get(cid).map(|k| k.position()) {
        world.broadcast_magic_effect(pos, CONST_ME_POFF);
    }
    false
}

fn success_effect(world: &mut GameWorld, cid: CreatureId, effect: u8) {
    if let Some(pos) = world.creatures.get(cid).map(|k| k.position()) {
        world.broadcast_magic_effect(pos, effect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cylinder::Cylinder;
    use crate::item::Item;
    use crate::sim_harness::{
        ensure_walkable_tile_if_absent, insert_player, minimal_world, pickup_item_type, test_player,
    };
    use std::sync::Arc;
    use tfs_rust_common::Position;
    use tfs_rust_lua::ConjureRequest;

    const BLANK: u16 = 2260;
    const RUNE: u16 = 2287;
    const ARROW: u16 = 2544;

    fn req(
        player: u64,
        mana: i32,
        reagent: u16,
        conjure: u16,
        count: Option<u32>,
    ) -> ConjureRequest {
        ConjureRequest {
            player,
            mana_cost: mana,
            reagent_id: reagent,
            conjure_id: conjure,
            conjure_count: count,
            effect: 14,
        }
    }

    fn register_types(world: &mut GameWorld, ids: &[u16]) {
        let db = Arc::get_mut(&mut world.items_db).expect("unique items_db");
        for &id in ids {
            db.items.entry(id).or_insert_with(|| pickup_item_type(id));
        }
    }

    fn spawn(world: &mut GameWorld) -> CreatureId {
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile_if_absent(&mut world.map, pos);
        world.mechanics.profile.classic_equipment_slots = true;
        let cid = insert_player(world, test_player("c", pos));
        let bag = equip(world, cid, InventorySlot::Backpack as u8, 1987);
        world.hydrate_container_if_needed(bag);
        cid
    }

    fn spawn_with(world: &mut GameWorld, player: crate::creature::Player) -> CreatureId {
        let pos = player.base.position;
        ensure_walkable_tile_if_absent(&mut world.map, pos);
        world.mechanics.profile.classic_equipment_slots = true;
        let cid = insert_player(world, player);
        let bag = equip(world, cid, InventorySlot::Backpack as u8, 1987);
        world.hydrate_container_if_needed(bag);
        cid
    }

    fn equip(world: &mut GameWorld, cid: CreatureId, slot: u8, type_id: u16) -> ItemId {
        let mut item = Item::new(type_id, 1);
        item.parent = Some(Cylinder::Inventory {
            player_id: cid,
            slot,
        });
        let iid = world.items.insert(item);
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            p.equipment_slots[(slot - 1) as usize] = Some(iid);
        }
        iid
    }

    #[test]
    fn unknown_conjure_id_returns_false() {
        let mut world = minimal_world();
        let cid = spawn(&mut world);
        assert!(!apply(
            &mut world,
            &req(cid.data().as_ffi(), 120, BLANK, 9, None)
        ));
    }

    #[test]
    fn charge_fallback_when_count_omitted() {
        let mut world = minimal_world();
        {
            let db = Arc::get_mut(&mut world.items_db).expect("db");
            let mut it = pickup_item_type(RUNE);
            it.charges = 5;
            db.items.insert(RUNE, it);
        }
        let cid = spawn(&mut world);
        register_types(&mut world, &[BLANK]);
        equip(&mut world, cid, InventorySlot::Left as u8, BLANK);
        assert!(apply(
            &mut world,
            &req(cid.data().as_ffi(), 120, BLANK, RUNE, None)
        ));
        let left = world
            .get_player_inventory_item(cid, InventorySlot::Left as u8)
            .expect("left");
        let item = world.items.get(left).expect("item");
        assert_eq!(item.item_type, RUNE);
        assert_eq!(item.count, 5);
    }

    #[test]
    fn missing_hand_reagent_fails() {
        let mut world = minimal_world();
        register_types(&mut world, &[BLANK, RUNE]);
        let cid = spawn(&mut world);
        assert!(!apply(
            &mut world,
            &req(cid.data().as_ffi(), 120, BLANK, RUNE, Some(5))
        ));
    }

    #[test]
    fn left_hand_reagent_succeeds() {
        let mut world = minimal_world();
        register_types(&mut world, &[BLANK, RUNE]);
        let cid = spawn(&mut world);
        equip(&mut world, cid, InventorySlot::Left as u8, BLANK);
        assert!(apply(
            &mut world,
            &req(cid.data().as_ffi(), 120, BLANK, RUNE, Some(5))
        ));
        let left = world
            .get_player_inventory_item(cid, InventorySlot::Left as u8)
            .expect("left");
        assert_eq!(world.items.get(left).map(|i| i.item_type), Some(RUNE));
    }

    #[test]
    fn dual_hand_skips_second_when_mana_short() {
        let mut world = minimal_world();
        register_types(&mut world, &[BLANK, RUNE]);
        let mut p = test_player("c", Position::new(100, 100, 7));
        p.mana = 50;
        p.max_mana = 200;
        let cid = spawn_with(&mut world, p);
        equip(&mut world, cid, InventorySlot::Left as u8, BLANK);
        equip(&mut world, cid, InventorySlot::Right as u8, BLANK);
        assert!(apply(
            &mut world,
            &req(cid.data().as_ffi(), 80, BLANK, RUNE, Some(1))
        ));
        let left = world
            .get_player_inventory_item(cid, InventorySlot::Left as u8)
            .expect("left");
        let right = world
            .get_player_inventory_item(cid, InventorySlot::Right as u8)
            .expect("right");
        assert_eq!(world.items.get(left).map(|i| i.item_type), Some(RUNE));
        assert_eq!(world.items.get(right).map(|i| i.item_type), Some(BLANK));
        if let Some(CreatureKind::Player(pl)) = world.creatures.get(cid) {
            assert_eq!(pl.mana, 50);
        }
    }

    #[test]
    fn reagent_zero_adds_to_inventory() {
        let mut world = minimal_world();
        register_types(&mut world, &[ARROW]);
        {
            let db = Arc::get_mut(&mut world.items_db).expect("db");
            if let Some(it) = db.items.get_mut(&ARROW) {
                it.flags |= 1 << 7; // FLAG_STACKABLE
            }
        }
        let cid = spawn(&mut world);
        assert!(apply(
            &mut world,
            &req(cid.data().as_ffi(), 100, 0, ARROW, Some(10))
        ));
        let ammo = world.get_player_inventory_item(cid, InventorySlot::Ammo as u8);
        let found = ammo
            .or_else(|| world.find_item_of_type(cid, ARROW, true, -1))
            .expect("arrows");
        assert_eq!(world.items.get(found).map(|i| i.item_type), Some(ARROW));
        assert_eq!(world.items.get(found).map(|i| i.count), Some(10));
    }

    #[test]
    fn backpack_path_transforms_non_hand_reagent() {
        let mut world = minimal_world();
        world.mechanics.profile.conjure_from_hands_only = false;
        register_types(&mut world, &[BLANK, RUNE]);
        let cid = spawn(&mut world);
        equip(&mut world, cid, InventorySlot::Ammo as u8, BLANK);
        assert!(apply(
            &mut world,
            &req(cid.data().as_ffi(), 120, BLANK, RUNE, Some(5))
        ));
        let ammo = world
            .get_player_inventory_item(cid, InventorySlot::Ammo as u8)
            .expect("ammo");
        assert_eq!(world.items.get(ammo).map(|i| i.item_type), Some(RUNE));
    }
}
