//! Native `Player.removeTotalMoney` / `Player.canCarryMoney` — pack surface from `data/lib/core/player.lua`.
//!
//! Pack: `Player.removeTotalMoney`, `Player.canCarryMoney` — `data/lib/core/player.lua`.
//! C++ reference: TFS money/bank on `Player` (`player.cpp`); coin inventory — `player/inventory/money.rs`;
//! NPC bridge — `npc/host.rs` `player_remove_money_u64` / `player_set_bank_balance_u64`.

use slotmap::Key;

use crate::container::ContainerIterator;
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::InventorySlot;
use crate::player::inventory::money::{
    ITEM_CRYSTAL_COIN, ITEM_GOLD_COIN, ITEM_PLATINUM_COIN,
};

/// Lua `MESSAGE_INFO_DESCR` (`const.h`).
const MESSAGE_INFO_DESCR: u8 = 0x16;
const CRYSTAL_WORTH: u64 = 10_000;
const PLATINUM_WORTH: u64 = 100;

/// `Player.removeTotalMoney` — inventory coins first, then bank; bank debits notify the player.
pub fn player_remove_total_money(world: &mut GameWorld, cid: CreatureId, amount: u64) -> bool {
    if amount == 0 {
        return true;
    }
    let (money_count, bank_count) = player_money_and_bank(world, cid);
    if amount <= money_count {
        return world
            .player_delete_money(cid, i32::try_from(amount).unwrap_or(i32::MAX))
            .is_ok();
    }
    if amount > money_count.saturating_add(bank_count) {
        return false;
    }
    if money_count != 0 {
        if world
            .player_delete_money(cid, i32::try_from(money_count).unwrap_or(i32::MAX))
            .is_err()
        {
            return false;
        }
        let bank_paid = amount - money_count;
        let new_bank = bank_count - bank_paid;
        set_player_bank_balance(world, cid, new_bank);
        let msg = format!(
            "Paid {money_count} from inventory and {bank_paid} gold from bank account. Your account balance is now {new_bank} gold."
        );
        let _ = world.lua_script_player_send_text_message(
            cid.data().as_ffi(),
            MESSAGE_INFO_DESCR,
            msg,
        );
        return true;
    }
    let new_bank = bank_count - amount;
    set_player_bank_balance(world, cid, new_bank);
    let msg = format!(
        "Paid {amount} gold from bank account. Your account balance is now {new_bank} gold."
    );
    let _ = world.lua_script_player_send_text_message(
        cid.data().as_ffi(),
        MESSAGE_INFO_DESCR,
        msg,
    );
    true
}

/// `Player.canCarryMoney` — coin stack weight + recursive backpack empty slots.
pub fn player_can_carry_money(world: &GameWorld, cid: CreatureId, amount: u64) -> bool {
    if amount == 0 {
        return true;
    }
    let (total_weight, inventory_slots) = money_carry_requirements(world, amount);
    let Some(free_cap) = world.player_free_capacity_u32(cid) else {
        return false;
    };
    if free_cap < total_weight {
        return false;
    }
    let Some(backpack) = world.get_player_inventory_item(cid, InventorySlot::Backpack as u8) else {
        return false;
    };
    container_empty_slots_recursive(world, backpack) >= inventory_slots
}

impl GameWorld {
    /// `player:removeTotalMoney(amount)` — resolves script creature id.
    pub fn player_remove_total_money_u64(&mut self, creature_u64: u64, amount: u64) -> bool {
        let Some(cid) = self.resolve_creature_u64(creature_u64) else {
            return false;
        };
        player_remove_total_money(self, cid, amount)
    }

    /// `player:canCarryMoney(amount)` — resolves script creature id.
    pub fn player_can_carry_money_u64(&self, creature_u64: u64, amount: u64) -> bool {
        let Some(cid) = self.resolve_creature_u64(creature_u64) else {
            return false;
        };
        player_can_carry_money(self, cid, amount)
    }
}

fn player_money_and_bank(world: &GameWorld, cid: CreatureId) -> (u64, u64) {
    let money = world.player_count_money(cid);
    let bank = match world.creatures.get(cid) {
        Some(CreatureKind::Player(p)) => p.economy.balance,
        _ => 0,
    };
    (money, bank)
}

fn set_player_bank_balance(world: &mut GameWorld, cid: CreatureId, balance: u64) {
    if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
        p.economy.balance = balance;
    }
}

fn money_carry_requirements(world: &GameWorld, mut amount: u64) -> (u32, u32) {
    let mut total_weight = 0u32;
    let mut inventory_slots = 0u32;

    let mut crystal = amount / CRYSTAL_WORTH;
    amount %= CRYSTAL_WORTH;
    accumulate_coin_stacks(
        world,
        ITEM_CRYSTAL_COIN,
        &mut crystal,
        &mut total_weight,
        &mut inventory_slots,
    );

    let mut platinum = amount / PLATINUM_WORTH;
    amount %= PLATINUM_WORTH;
    accumulate_coin_stacks(
        world,
        ITEM_PLATINUM_COIN,
        &mut platinum,
        &mut total_weight,
        &mut inventory_slots,
    );

    if amount > 0 {
        accumulate_coin_stacks(
            world,
            ITEM_GOLD_COIN,
            &mut amount,
            &mut total_weight,
            &mut inventory_slots,
        );
    }

    (total_weight, inventory_slots)
}

fn accumulate_coin_stacks(
    world: &GameWorld,
    item_type: u16,
    remaining: &mut u64,
    total_weight: &mut u32,
    inventory_slots: &mut u32,
) {
    while *remaining > 0 {
        let count = (*remaining).min(100) as u16;
        *total_weight =
            total_weight.saturating_add(item_type_weight(world, item_type, count));
        *remaining -= u64::from(count);
        *inventory_slots += 1;
    }
}

/// TFS `ItemType::getWeight(count)` — `weight * max(1, count)`.
fn item_type_weight(world: &GameWorld, item_type: u16, count: u16) -> u32 {
    let unit = world
        .items_db
        .items
        .get(&item_type)
        .map(|t| t.weight)
        .unwrap_or(0);
    unit.saturating_mul(u32::from(count.max(1)))
}

/// TFS `Container::getEmptySlots(true)` — direct slots plus nested container free space.
fn container_empty_slots_recursive(world: &GameWorld, container_id: ItemId) -> u32 {
    let Some(root) = world.container_registry.get(container_id) else {
        return 0;
    };
    let mut slots = root.available_slots();
    for item_id in ContainerIterator::new(&world.container_registry, container_id) {
        if let Some(nested) = world.container_registry.get(item_id) {
            slots = slots.saturating_add(nested.available_slots());
        }
    }
    slots
}
