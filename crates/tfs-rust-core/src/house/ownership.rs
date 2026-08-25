//! House ownership change — TFS `House::setOwner` with corpus eviction target.
//!
//! Pack surface: `house.cpp` `House::setOwner` (~27) / `kickPlayer` (~143).
//! Corpus: `CleanHouse` (`houses.cc` ~855) moves TAKE items to the **town depot**, not inbox.

use slotmap::Key;
use tfs_rust_common::Position;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::item::Item;
use crate::item_constants::ITEM_LETTER_STAMPED;
use crate::player_flags::{PLAYER_FLAG_CAN_EDIT_HOUSES, has_player_flag};
use crate::tile::Tile;

use super::access::AccessHouseLevel;
use super::rent::HOUSE_MONTH_SECS;

impl GameWorld {
    /// C++ `House::setOwner` — eviction, kick, wake beds, clear lists, then assign.
    pub fn house_set_owner(&mut self, house_id: u32, new_guid: u32, now_unix: u32) {
        let old = self
            .houses
            .houses
            .get(&house_id)
            .and_then(|a| a.owner_guid)
            .unwrap_or(0);
        if old == new_guid && self.houses.records.get(&house_id).is_some() {
            self.houses.set_owner(house_id, new_guid);
            return;
        }
        if old != 0 {
            self.house_transfer_to_depot(house_id, old);
            self.house_kick_occupants(house_id);
            self.house_wake_beds(house_id);
            self.houses.clear_lists(house_id);
        }
        self.houses.set_owner(house_id, new_guid);
        if new_guid != 0
            && let Some(&cid) = self.player_by_guid.get(&new_guid)
            && let Some(k) = self.creatures.get(cid)
        {
            self.houses
                .set_owner_name(house_id, k.base().name.clone());
        }
        if let Some(rec) = self.houses.records.get_mut(&house_id) {
            rec.clear_bid();
            rec.warnings = 0;
            if old == 0 && new_guid != 0 {
                rec.paid_until = now_unix.saturating_add(HOUSE_MONTH_SECS);
            } else if new_guid != 0 {
                rec.paid_until = now_unix.saturating_add(HOUSE_MONTH_SECS);
            }
        }
        if new_guid != 0 && old == 0 {
            let town_id = self
                .houses
                .records
                .get(&house_id)
                .map(|r| r.town_id)
                .unwrap_or(0);
            let name = self
                .houses
                .records
                .get(&house_id)
                .map(|r| r.name.clone())
                .unwrap_or_default();
            self.house_deliver_letter(
                new_guid,
                town_id,
                format!("Welcome!\nYou now own {name}."),
            );
        }
    }

    /// Corpus `CleanHouse` — pickupables (+ contents of immovable containers) to town depot.
    pub fn house_transfer_to_depot(&mut self, house_id: u32, owner_guid: u32) {
        let tiles = self
            .houses
            .records
            .get(&house_id)
            .map(|r| r.tiles.clone())
            .unwrap_or_default();
        let town_id = self
            .houses
            .records
            .get(&house_id)
            .map(|r| r.town_id)
            .unwrap_or(0);
        let mut move_ids: Vec<ItemId> = Vec::new();
        for pos in tiles {
            let Some(tile) = self.map.get_tile(pos) else {
                continue;
            };
            collect_transferable(self, tile, &mut move_ids);
        }
        for iid in &move_ids {
            if let Some(pos) = self.map.find_item_position(*iid) {
                let _ = self.internal_remove_item_from_tile(pos, *iid, u16::MAX);
            }
        }
        if let Some(&cid) = self.player_by_guid.get(&owner_guid) {
            let Some(chest) = self.player_get_depot_chest(cid, town_id, true) else {
                return;
            };
            for iid in move_ids {
                add_to_container_front(self, chest, iid);
            }
            return;
        }
        self.houses
            .pending_depot_dumps
            .entry(owner_guid)
            .or_default()
            .extend(move_ids);
        self.houses.pending_depot_town.insert(owner_guid, town_id);
    }

    /// Walk the map and attach house tiles, doors, and beds to [`HouseManager`].
    pub fn house_scan_map(&mut self) {
        let mut found: Vec<(u32, Position, Vec<ItemId>, Vec<ItemId>)> = Vec::new();
        self.map.for_each_tile(|pos, tile| {
            let Tile::House(h) = tile else {
                return;
            };
            let body = tile.body();
            let items: Vec<ItemId> = body
                .down_items
                .iter()
                .copied()
                .chain(body.top_items.iter().copied())
                .collect();
            found.push((h.house_id, pos, items, Vec::new()));
        });
        for (house_id, pos, items, _) in found {
            self.houses.attach_tile(house_id, pos);
            for iid in items {
                let Some(item) = self.items.get(iid) else {
                    continue;
                };
                let door = item
                    .attributes
                    .as_deref()
                    .map(|a| a.get_door_id())
                    .unwrap_or(0);
                if door != 0 {
                    self.houses.attach_door(house_id, door, iid);
                }
                if self
                    .items_db
                    .items
                    .get(&item.item_type)
                    .is_some_and(|t| t.is_bed())
                {
                    self.houses.attach_bed(house_id, iid);
                }
            }
        }
    }

    pub fn house_kick_occupants(&mut self, house_id: u32) {
        let entry = self
            .houses
            .records
            .get(&house_id)
            .map(|r| r.entry_pos)
            .unwrap_or_default();
        let tiles = self
            .houses
            .records
            .get(&house_id)
            .map(|r| r.tiles.clone())
            .unwrap_or_default();
        let mut cids = Vec::new();
        for pos in tiles {
            if let Some(tile) = self.map.get_tile(pos) {
                cids.extend(tile.body().creatures.iter().copied());
            }
        }
        for cid in cids {
            let _ = self.lua_script_creature_teleport(
                cid.data().as_ffi(),
                entry.x,
                entry.y,
                entry.z,
                false,
            );
        }
    }

    pub fn house_wake_beds(&mut self, house_id: u32) {
        let beds: Vec<ItemId> = self
            .houses
            .records
            .get(&house_id)
            .map(|r| r.beds.clone())
            .unwrap_or_default();
        for bed in beds {
            self.bed_wake_up(bed, None);
        }
    }

    /// C++ `House::kickPlayer` — `house.cpp` (~143).
    pub fn house_kick_player(
        &mut self,
        house_id: u32,
        kicker: CreatureId,
        target: CreatureId,
    ) -> bool {
        let Some(CreatureKind::Player(kp)) = self.creatures.get(kicker) else {
            return false;
        };
        let kicker_guid = kp.guid;
        let kicker_edit = has_player_flag(self.player_group_flags(kicker), PLAYER_FLAG_CAN_EDIT_HOUSES);
        let Some(CreatureKind::Player(_)) = self.creatures.get(target) else {
            return false;
        };
        let target_guid = match self.creatures.get(target) {
            Some(CreatureKind::Player(p)) => p.guid,
            _ => return false,
        };
        let target_edit = has_player_flag(self.player_group_flags(target), PLAYER_FLAG_CAN_EDIT_HOUSES);
        if target_edit {
            return false;
        }
        let kicker_lv = self
            .houses
            .access_level(house_id, kicker_guid, kicker_edit);
        let target_lv = self
            .houses
            .access_level(house_id, target_guid, false);
        if kicker_lv < target_lv || kicker_lv == AccessHouseLevel::NotInvited {
            return false;
        }
        let pos = self.creatures.get(target).map(|k| k.position());
        let on_house = pos.is_some_and(|p| {
            matches!(self.map.get_tile(p), Some(Tile::House(h)) if h.house_id == house_id)
        });
        if !on_house {
            return false;
        }
        let entry = self
            .houses
            .records
            .get(&house_id)
            .map(|r| r.entry_pos)
            .unwrap_or_default();
        self.lua_script_creature_teleport(target.data().as_ffi(), entry.x, entry.y, entry.z, false)
            .unwrap_or(false)
    }

    /// Login on a house tile without invite → entry (`cract.cc` ~321).
    pub fn house_relocate_if_uninvited(&mut self, cid: CreatureId) {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };
        let guid = p.guid;
        let edit = has_player_flag(self.player_group_flags(cid), PLAYER_FLAG_CAN_EDIT_HOUSES);
        let pos = p.base.position;
        let Some(Tile::House(h)) = self.map.get_tile(pos) else {
            return;
        };
        let house_id = h.house_id;
        if self.houses.is_invited(house_id, guid) || edit {
            return;
        }
        let Some(entry) = self.houses.records.get(&house_id).map(|r| r.entry_pos) else {
            return;
        };
        let _ = self.lua_script_creature_teleport(cid.data().as_ffi(), entry.x, entry.y, entry.z, false);
    }

    pub fn house_deliver_letter(&mut self, owner_guid: u32, town_id: u32, text: String) {
        let mut letter = Item::new_single(ITEM_LETTER_STAMPED);
        letter.set_text(text);
        let iid = self.items.insert(letter);
        if let Some(&cid) = self.player_by_guid.get(&owner_guid) {
            if let Some(chest) = self.player_get_depot_chest(cid, town_id, true) {
                add_to_container_front(self, chest, iid);
                return;
            }
        }
        self.houses
            .pending_depot_dumps
            .entry(owner_guid)
            .or_default()
            .push(iid);
        self.houses.pending_depot_town.insert(owner_guid, town_id);
    }

    pub fn lua_script_house_set_owner(&mut self, house_id: u32, guid: u32) {
        let now = super::persist::unix_now();
        self.house_set_owner(house_id, guid, now);
    }

    pub fn lua_script_house_set_access_list(&mut self, house_id: u32, list_id: u32, text: String) {
        let cache = self.houses.name_to_guid.clone();
        self.houses
            .apply_list_row(house_id, list_id, &text, |n| cache.get(n).copied());
    }

    pub fn lua_script_house_kick_player(
        &mut self,
        house_id: u32,
        kicker: u64,
        target: u64,
    ) -> bool {
        let Some(kicker_cid) = self.resolve_creature_u64(kicker) else {
            return false;
        };
        let Some(target_cid) = self.resolve_creature_u64(target) else {
            return false;
        };
        self.house_kick_player(house_id, kicker_cid, target_cid)
    }

    pub fn house_add_item_to_town_depot(&mut self, cid: CreatureId, town_id: u32, item_id: ItemId) {
        if let Some(chest) = self.player_get_depot_chest(cid, town_id, true) {
            add_to_container_front(self, chest, item_id);
        }
    }
}

fn collect_transferable(world: &GameWorld, tile: &Tile, out: &mut Vec<ItemId>) {
    let body = tile.body();
    for &id in body.down_items.iter().chain(body.top_items.iter()) {
        let Some(item) = world.items.get(id) else {
            continue;
        };
        let pickup = world
            .items_db
            .items
            .get(&item.item_type)
            .is_some_and(|t| t.pickupable());
        if pickup {
            out.push(id);
            continue;
        }
        if let Some(c) = world.container_registry.get(id) {
            for &child in &c.items {
                if world
                    .items
                    .get(child)
                    .and_then(|i| world.items_db.items.get(&i.item_type))
                    .is_some_and(|t| t.pickupable())
                {
                    out.push(child);
                }
            }
        }
    }
}

fn add_to_container_front(world: &mut GameWorld, container: ItemId, item_id: ItemId) {
    let mut reg = std::mem::take(&mut world.container_registry);
    if reg.get(container).is_none() {
        let cap = 32;
        reg.register(crate::container::Container::new(container, cap));
    }
    if let Some(c) = reg.get_mut(container) {
        c.internal_add_item_front(item_id);
    }
    if let Some(ch) = reg.get_mut(item_id) {
        ch.parent_container = Some(container);
    }
    if let Some(item) = world.items.get_mut(item_id) {
        item.parent = Some(crate::cylinder::Cylinder::Container {
            item_id: container,
            index: crate::cylinder::INDEX_WHEREEVER,
        });
    }
    world.container_registry = reg;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::house::HouseManager;
    use crate::sim_harness::{insert_player, minimal_world, test_player};
    use crate::tile::{HouseTile, Tile, TileBody};
    use tfs_rust_common::Position;
    use tfs_rust_common::enums::ZoneType;

    #[test]
    fn eviction_clears_lists_and_kicks() {
        let mut world = minimal_world();
        let pos = Position::new(50, 50, 7);
        let entry = Position::new(51, 50, 7);
        world.map.insert_tile(
            pos,
            Tile::House(HouseTile {
                inner: TileBody {
                    flags: 0,
                    zone: ZoneType::Protection,
                    ..TileBody::new()
                },
                house_id: 1,
            }),
        );
        world.map.insert_tile(entry, Tile::empty_normal());
        world.houses.ensure_houses([1]);
        if let Some(rec) = world.houses.records.get_mut(&1) {
            rec.entry_pos = entry;
            rec.tiles.push(pos);
            rec.town_id = 1;
        }
        world.houses.set_owner(1, 10);
        world.houses.apply_list_row(1, crate::house::GUEST_LIST, "guest", |_| Some(30));
        let cid = insert_player(&mut world, test_player("Owner", pos));
        if let crate::creature::CreatureKind::Player(p) = world.creatures.get_mut(cid).unwrap() {
            p.guid = 10;
        }
        world.player_by_guid.insert(10, cid);
        world.house_set_owner(1, 0, 1_000);
        assert!(world.houses.houses.get(&1).unwrap().owner_guid.is_none());
        assert!(world.houses.houses.get(&1).unwrap().guests.is_empty());
        let _ = HouseManager::default();
    }
}
