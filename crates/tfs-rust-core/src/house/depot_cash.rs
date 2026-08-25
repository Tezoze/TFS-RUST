//! Depot cash helpers for house rent / auctions.
//! Corpus `GetMoney` / `DeleteMoney` on the house town depot (`houses.cc`).

use tfs_rust_db::ItemRecord;

use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::player::inventory::money::{ITEM_CRYSTAL_COIN, ITEM_GOLD_COIN, ITEM_PLATINUM_COIN};

const GOLD_WORTH: u64 = 1;
const PLATINUM_WORTH: u64 = 100;
const CRYSTAL_WORTH: u64 = 10_000;

fn coin_worth(item_type: u16, count: u16) -> u64 {
    let unit = match item_type {
        ITEM_GOLD_COIN => GOLD_WORTH,
        ITEM_PLATINUM_COIN => PLATINUM_WORTH,
        ITEM_CRYSTAL_COIN => CRYSTAL_WORTH,
        _ => return 0,
    };
    unit.saturating_mul(u64::from(count))
}

/// Sum gold-equivalent of coin stacks in a container tree.
pub fn container_tree_money(world: &GameWorld, root: ItemId) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if let Some(item) = world.items.get(id) {
            total = total.saturating_add(coin_worth(item.item_type, item.count));
        }
        if let Some(c) = world.container_registry.get(id) {
            stack.extend(c.items.iter().copied());
        }
    }
    total
}

pub fn player_town_depot_money(world: &mut GameWorld, cid: CreatureId, town_id: u32) -> u64 {
    let Some(chest) = world.player_get_depot_chest(cid, town_id, true) else {
        return 0;
    };
    container_tree_money(world, chest)
}

/// Remove coin stacks totaling `amount` (gold-equivalent) from a container tree.
/// Returns false if the tree does not contain enough.
pub fn container_tree_delete_money(world: &mut GameWorld, root: ItemId, amount: u64) -> bool {
    if container_tree_money(world, root) < amount {
        return false;
    }
    let mut remaining = amount;
    let mut ids = Vec::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        ids.push(id);
        if let Some(c) = world.container_registry.get(id) {
            stack.extend(c.items.iter().copied());
        }
    }
    for id in ids {
        if remaining == 0 {
            break;
        }
        if id == root {
            continue;
        }
        let Some(item) = world.items.get(id) else {
            continue;
        };
        let item_type = item.item_type;
        let count = item.count;
        let worth = coin_worth(item_type, count);
        if worth == 0 {
            continue;
        }
        let parent = match item.parent {
            Some(crate::cylinder::Cylinder::Container { item_id, .. }) => item_id,
            _ => continue,
        };
        if worth <= remaining {
            remaining -= worth;
            let _ = world.internal_remove_item_from_container(parent, id);
        } else {
            let unit = match item_type {
                ITEM_GOLD_COIN => GOLD_WORTH,
                ITEM_PLATINUM_COIN => PLATINUM_WORTH,
                ITEM_CRYSTAL_COIN => CRYSTAL_WORTH,
                _ => continue,
            };
            let take = remaining.div_ceil(unit).min(u64::from(count)) as u16;
            remaining = remaining.saturating_sub(unit.saturating_mul(u64::from(take)));
            let mut empty = false;
            if let Some(item_mut) = world.items.get_mut(id) {
                item_mut.count = item_mut.count.saturating_sub(take);
                empty = item_mut.count == 0;
            }
            if empty {
                let _ = world.internal_remove_item_from_container(parent, id);
            }
        }
    }
    remaining == 0
}

/// Gold-equivalent of coin rows in `player_depotitems` (offline owners).
pub fn depot_records_money(rows: &[ItemRecord]) -> u64 {
    rows.iter()
        .map(|r| coin_worth(r.itemtype, r.count.max(0) as u16))
        .sum()
}

/// Deduct `amount` gold-equivalent from depot rows in place. Returns false if short.
pub fn deduct_depot_records(rows: &mut Vec<ItemRecord>, amount: u64) -> bool {
    if depot_records_money(rows) < amount {
        return false;
    }
    let mut remaining = amount;
    for row in rows.iter_mut() {
        if remaining == 0 {
            break;
        }
        let unit = match row.itemtype {
            ITEM_GOLD_COIN => GOLD_WORTH,
            ITEM_PLATINUM_COIN => PLATINUM_WORTH,
            ITEM_CRYSTAL_COIN => CRYSTAL_WORTH,
            _ => continue,
        };
        let count = row.count.max(0) as u64;
        let worth = unit.saturating_mul(count);
        if worth == 0 {
            continue;
        }
        if worth <= remaining {
            remaining -= worth;
            row.count = 0;
        } else {
            let take = remaining.div_ceil(unit).min(count);
            remaining = remaining.saturating_sub(unit.saturating_mul(take));
            row.count = (count - take) as i16;
        }
    }
    rows.retain(|r| r.count > 0 || coin_worth(r.itemtype, 1) == 0);
    remaining == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deducts_gold_rows() {
        let mut rows = vec![
            ItemRecord {
                pid: 1,
                sid: 1,
                itemtype: ITEM_GOLD_COIN,
                count: 50,
                attributes: Vec::new(),
            },
            ItemRecord {
                pid: 1,
                sid: 2,
                itemtype: ITEM_PLATINUM_COIN,
                count: 2,
                attributes: Vec::new(),
            },
        ];
        assert_eq!(depot_records_money(&rows), 250);
        assert!(deduct_depot_records(&mut rows, 30));
        assert_eq!(depot_records_money(&rows), 220);
    }
}
