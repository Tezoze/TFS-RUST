//! Shop purchase planner.
//!
//! Pack surface: TFS `ShopModule:callbackOnBuy` / `doNpcSellItem` (`modules.lua`).
//! One plan decides units, bags, and cost, then delivery and payment happen together.
//! Bag slot count and weight come from the item database. Bag gold comes from the
//! shop catalog (or the shop's bag price), not a hardcoded fee.
//!
//! 772 has no shop-window purchase packet. This path runs for later codecs.

use crate::container::Container;
use crate::cylinder::{Cylinder, CylinderFlags, INDEX_WHEREEVER};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::item::Item;
use crate::player_money_lib::player_remove_total_money;

/// Shopping bag used when the client sets `inBackpacks` and the shop did not name one.
pub const DEFAULT_SHOPPING_BAG: u16 = 1988;

/// Why a purchase delivered nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopFail {
    NotForSale,
    Money,
    Space,
}

/// Bag used to wrap a purchase. Capacity is slot count from the item database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BagSpec {
    pub item_id: u16,
    pub capacity: u32,
    pub weight: u32,
    pub price: u32,
}

/// Inputs for [`plan_purchase`]. Amount is the requested unit count (packs or loose items).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PurchaseQuery {
    pub amount: u32,
    pub unit_price: u32,
    pub unit_weight: u32,
    pub stackable: bool,
    pub ignore_cap: bool,
    pub in_backpacks: bool,
    /// When set, each unit is one container holding this many of the item.
    pub pack_size: Option<u32>,
    pub money: u64,
    pub free_capacity: u32,
    pub bag: BagSpec,
}

/// How much of an order can be paid for and carried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PurchasePlan {
    pub units: u32,
    pub bags: u32,
    pub cost: u64,
    pub weight: u32,
}

/// Slots of product that fit in one shopping bag.
pub fn units_per_bag(stackable: bool, capacity: u32) -> u32 {
    let slots = capacity.max(1);
    if stackable {
        slots.saturating_mul(100)
    } else {
        slots
    }
}

/// Bags required to hold `units`. Packed offers use one container per unit.
pub fn bags_for(units: u32, query: &PurchaseQuery) -> u32 {
    if units == 0 {
        return 0;
    }
    if query.pack_size.is_some() {
        return units;
    }
    if !query.in_backpacks {
        return 0;
    }
    let per = units_per_bag(query.stackable, query.bag.capacity);
    units.div_ceil(per.max(1))
}

fn line_cost(units: u32, query: &PurchaseQuery, bags: u32) -> u64 {
    let goods = u64::from(query.unit_price).saturating_mul(u64::from(units));
    if query.pack_size.is_some() || !query.in_backpacks {
        return goods;
    }
    goods.saturating_add(u64::from(query.bag.price).saturating_mul(u64::from(bags)))
}

fn line_weight(units: u32, query: &PurchaseQuery, bags: u32) -> u32 {
    if let Some(pack) = query.pack_size {
        let one = query
            .bag
            .weight
            .saturating_add(query.unit_weight.saturating_mul(pack));
        return one.saturating_mul(units);
    }
    let goods = query.unit_weight.saturating_mul(units);
    if query.in_backpacks {
        goods.saturating_add(query.bag.weight.saturating_mul(bags))
    } else {
        goods
    }
}

/// Largest prefix of the order that fits the purse and free capacity.
pub fn plan_purchase(query: &PurchaseQuery) -> PurchasePlan {
    let amount = query.amount;
    if amount == 0 || query.unit_price == 0 {
        return PurchasePlan {
            units: 0,
            bags: 0,
            cost: 0,
            weight: 0,
        };
    }
    let mut best = PurchasePlan {
        units: 0,
        bags: 0,
        cost: 0,
        weight: 0,
    };
    for units in 1..=amount {
        let bags = bags_for(units, query);
        let cost = line_cost(units, query, bags);
        let weight = line_weight(units, query, bags);
        if cost > query.money {
            break;
        }
        if !query.ignore_cap && weight > query.free_capacity {
            break;
        }
        best = PurchasePlan {
            units,
            bags,
            cost,
            weight,
        };
    }
    best
}

/// Classify an empty plan. Money is checked before capacity.
pub fn empty_plan_reason(query: &PurchaseQuery) -> ShopFail {
    if query.unit_price == 0 || query.amount == 0 {
        return ShopFail::NotForSale;
    }
    let one = PurchaseQuery {
        amount: 1,
        money: u64::MAX,
        free_capacity: u32::MAX,
        ignore_cap: true,
        ..*query
    };
    let priced = plan_purchase(&one);
    if priced.cost > query.money {
        return ShopFail::Money;
    }
    ShopFail::Space
}

impl GameWorld {
    /// Deliver a planned purchase and take inventory plus bank gold for `cost`.
    pub(crate) fn deliver_shop_purchase(
        &mut self,
        player: CreatureId,
        item_id: u16,
        sub_type: u8,
        query: &PurchaseQuery,
        plan: &PurchasePlan,
    ) -> Option<(u32, u64)> {
        if plan.units == 0 || plan.cost == 0 {
            return None;
        }
        let delivered = if query.pack_size.is_some() || query.in_backpacks {
            self.give_shop_bags(player, item_id, sub_type, query, plan)
        } else {
            self.npc_give_to(player, item_id, plan.units, i32::from(sub_type))
                .is_ok()
                .then_some(plan.units)
                .unwrap_or(0)
        };
        if delivered == 0 {
            return None;
        }
        let bags = bags_for(delivered, query);
        let mut charged = query.clone();
        charged.amount = delivered;
        let cost = line_cost(delivered, &charged, bags);
        if player_remove_total_money(self, player, cost) {
            Some((delivered, cost))
        } else {
            None
        }
    }

    fn give_shop_bags(
        &mut self,
        player: CreatureId,
        item_id: u16,
        sub_type: u8,
        query: &PurchaseQuery,
        plan: &PurchasePlan,
    ) -> u32 {
        let per_bag = if query.pack_size.is_some() {
            query.pack_size.unwrap_or(1).max(1)
        } else {
            units_per_bag(query.stackable, query.bag.capacity)
        };
        let mut left = plan.units;
        let mut given = 0u32;
        while left > 0 {
            let chunk = left.min(per_bag);
            if self
                .give_one_shop_bag(player, item_id, sub_type, chunk, query)
                .is_err()
            {
                break;
            }
            given = given.saturating_add(chunk);
            left -= chunk;
        }
        given
    }

    fn give_one_shop_bag(
        &mut self,
        player: CreatureId,
        item_id: u16,
        sub_type: u8,
        count: u32,
        query: &PurchaseQuery,
    ) -> Result<(), String> {
        if count == 0 {
            return Ok(());
        }
        let bag_type = query.bag.item_id;
        let bag_iid = self.items.insert(Item::new_single(bag_type));
        let mut cont = Container::new(bag_iid, query.bag.capacity.max(1));
        let stackable = query.stackable && query.pack_size.is_none();
        let mut remaining = count;
        while remaining > 0 {
            let chunk = if stackable { remaining.min(100) } else { 1 };
            let mut item = Item::new(item_id, chunk as u16);
            if sub_type > 0 {
                if let Some(it) = self.items_db.items.get(&item_id) {
                    if it.is_fluid_container() || it.is_splash() {
                        item.set_fluid_type(u16::from(sub_type));
                        item.count = u16::from(sub_type).max(1);
                    } else if it.charges != 0 {
                        item.set_charges(u16::from(sub_type));
                    }
                }
            }
            let iid = self.items.insert(item);
            cont.add_item(iid)
                .map_err(|_| "shop bag is full".to_string())?;
            if let Some(child) = self.items.get_mut(iid) {
                child.parent = Some(Cylinder::Container {
                    item_id: bag_iid,
                    index: INDEX_WHEREEVER,
                });
            }
            remaining -= chunk;
        }
        self.container_registry.register(cont);
        match self.lua_place_detached_item_on_player(player, bag_iid, true, 0, CylinderFlags::NONE)
        {
            crate::return_value::ReturnValue::NoError => Ok(()),
            _ => {
                self.drop_detached_bag(bag_iid);
                Err("could not place shopping bag".into())
            }
        }
    }

    fn drop_detached_bag(&mut self, bag_iid: crate::ids::ItemId) {
        let (cont, _) = self.container_registry.remove(bag_iid);
        if let Some(cont) = cont {
            for child in cont.items {
                self.items.remove(child);
            }
        }
        self.items.remove(bag_iid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bag() -> BagSpec {
        BagSpec {
            item_id: 1988,
            capacity: 20,
            weight: 10,
            price: 5,
        }
    }

    fn loose(amount: u32, money: u64, cap: u32) -> PurchaseQuery {
        PurchaseQuery {
            amount,
            unit_price: 10,
            unit_weight: 1,
            stackable: false,
            ignore_cap: false,
            in_backpacks: false,
            pack_size: None,
            money,
            free_capacity: cap,
            bag: bag(),
        }
    }

    #[test]
    fn partial_fill_stops_at_money() {
        let plan = plan_purchase(&loose(5, 25, 10_000));
        assert_eq!(plan.units, 2);
        assert_eq!(plan.cost, 20);
        assert_eq!(plan.bags, 0);
    }

    #[test]
    fn partial_fill_stops_at_capacity() {
        let plan = plan_purchase(&loose(5, 10_000, 3));
        assert_eq!(plan.units, 3);
        assert_eq!(plan.weight, 3);
    }

    #[test]
    fn empty_when_one_unit_is_unaffordable() {
        let query = loose(4, 9, 10_000);
        let plan = plan_purchase(&query);
        assert_eq!(plan.units, 0);
        assert_eq!(empty_plan_reason(&query), ShopFail::Money);
    }

    #[test]
    fn backpacks_add_bag_price_and_count() {
        let mut query = loose(25, 10_000, 10_000);
        query.in_backpacks = true;
        query.stackable = false;
        let plan = plan_purchase(&query);
        assert_eq!(plan.units, 25);
        assert_eq!(plan.bags, 2);
        assert_eq!(plan.cost, 25 * 10 + 2 * 5);
    }

    #[test]
    fn stackable_backpack_holds_one_hundred_per_slot() {
        let mut query = loose(150, 100_000, 100_000);
        query.in_backpacks = true;
        query.stackable = true;
        let plan = plan_purchase(&query);
        assert_eq!(plan.bags, 1);
        assert_eq!(plan.units, 150);
    }

    #[test]
    fn packed_offer_is_one_container_per_unit() {
        let mut query = loose(3, 100_000, 100_000);
        query.pack_size = Some(20);
        query.unit_price = 40;
        let plan = plan_purchase(&query);
        assert_eq!(plan.units, 3);
        assert_eq!(plan.bags, 3);
        assert_eq!(plan.cost, 120);
        assert_eq!(plan.weight, 3 * (10 + 20));
    }
}
