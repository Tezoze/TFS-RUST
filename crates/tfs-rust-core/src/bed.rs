//! House bed sleep / wake — TFS-shaped domain, 772 kick (no offline-training dialog).
//!
//! Domain: `BedItem::{canUse,trySleep,sleep,wakeUp,updateAppearance}` — `bed.cpp`.
//! Outcomes: occupy transforms `malesleeper` / partner tile; kick after sleep.

use tfs_rust_common::Position;
use tfs_rust_common::enums::{Direction, PlayerSex, ZoneType};
use tfs_rust_net::Codec;

use crate::creature::CreatureKind;
use crate::cylinder::Cylinder;
use crate::game_world::GameWorld;
use crate::house::AccessHouseLevel;
use crate::ids::{CreatureId, ItemId};
use crate::player_flags::PLAYER_FLAG_CAN_EDIT_HOUSES;
use crate::return_value::ReturnValue;
use crate::tile::Tile;
use crate::walk::internal_teleport_player;

/// 772 `CONST_ME_POFF` (`const.h:14`). Occupied-bed fail puff (`bed.cpp` `trySleep`).
const ME_POFF: u8 = 3;
/// TFS `CONST_ME_SLEEP` (`const.h`) — 1098 only; 772 client range ends at 25.
const ME_SLEEP: u8 = 32;

impl GameWorld {
    /// C++ `Actions::internalUseItem` bed arm — `actions.cpp` ~325–342.
    pub(crate) fn player_use_bed(
        &mut self,
        conn_id: tfs_rust_common::ConnId,
        cid: CreatureId,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        match self.bed_can_use(cid, item_id) {
            Ok(()) => {}
            Err(rv) => return Err(rv),
        }
        if !self.bed_try_sleep(cid, item_id) {
            return Ok(());
        }
        self.bed_sleep(conn_id, cid, item_id)
    }

    /// C++ `BedItem::canUse` — `bed.cpp:80-103`.
    fn bed_can_use(&self, cid: CreatureId, item_id: ItemId) -> Result<(), ReturnValue> {
        let Some(house_id) = self.bed_house_id(item_id) else {
            return Err(ReturnValue::YouCannotUseThisBed);
        };
        if !self.player_is_premium(cid) {
            return Err(ReturnValue::YouNeedPremiumAccount);
        }
        let Some(pos) = self.item_tile_pos(item_id) else {
            return Err(ReturnValue::CannotUseThisObject);
        };
        let in_pz = self
            .map
            .get_tile(pos)
            .is_some_and(|t| t.body().zone == ZoneType::Protection);
        if !in_pz {
            return Err(ReturnValue::CannotUseThisObject);
        }
        let sleeper = self
            .items
            .get(item_id)
            .map(|i| i.sleeper_guid())
            .unwrap_or(0);
        if sleeper == 0 {
            return Ok(());
        }
        let guid = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.guid,
            _ => return Err(ReturnValue::NotPossible),
        };
        let can_edit = self.player_has_flag(cid, PLAYER_FLAG_CAN_EDIT_HOUSES);
        if self.houses.access_level(house_id, guid, can_edit) == AccessHouseLevel::Owner {
            return Ok(());
        }
        Err(ReturnValue::CannotUseThisObject)
    }

    /// C++ `BedItem::trySleep` — `bed.cpp:105-120`. `false` means occupied (poff sent).
    fn bed_try_sleep(&mut self, cid: CreatureId, item_id: ItemId) -> bool {
        let sleeper = self
            .items
            .get(item_id)
            .map(|i| i.sleeper_guid())
            .unwrap_or(0);
        if sleeper == 0 {
            return true;
        }
        let transform_free = self
            .items
            .get(item_id)
            .and_then(|i| self.items_db.items.get(&i.item_type))
            .map(|t| t.transform_to_free)
            .unwrap_or(0);
        let guid = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.guid,
            _ => return false,
        };
        let house_id = self.bed_house_id(item_id);
        let is_owner = house_id.is_some_and(|hid| {
            self.houses.houses.get(&hid).and_then(|h| h.owner_guid) == Some(guid)
        });
        if transform_free != 0 && is_owner {
            self.bed_wake_up(item_id, None);
        }
        if let Some(pos) = self.creatures.get(cid).map(|k| k.position()) {
            self.broadcast_magic_effect(pos, ME_POFF);
        }
        false
    }

    /// C++ `BedItem::sleep` — `bed.cpp:122-161`. 772: kick immediately (no training dialog).
    fn bed_sleep(
        &mut self,
        conn_id: tfs_rust_common::ConnId,
        cid: CreatureId,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        if self.bed_house_id(item_id).is_none() {
            return Err(ReturnValue::NotPossible);
        }
        if self
            .items
            .get(item_id)
            .is_some_and(|i| i.sleeper_guid() != 0)
        {
            return Err(ReturnValue::NotPossible);
        }
        let partner = self.bed_partner_item(item_id);
        self.bed_internal_set_sleeper(item_id, cid);
        if let Some(pid) = partner {
            self.bed_internal_set_sleeper(pid, cid);
        }
        let guid = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.guid,
            _ => return Err(ReturnValue::NotPossible),
        };
        self.houses.bed_sleepers.insert(guid, item_id);

        let Some(bed_pos) = self.item_tile_pos(item_id) else {
            return Err(ReturnValue::NotPossible);
        };
        let _ = internal_teleport_player(self, conn_id, cid, bed_pos, true);
        if let Some(pos) = self.creatures.get(cid).map(|k| k.position()) {
            if matches!(self.codec, Codec::V1098(_)) {
                self.broadcast_magic_effect(pos, ME_SLEEP);
            }
        }
        self.bed_update_appearance(item_id, Some(cid));
        if let Some(pid) = partner {
            self.bed_update_appearance(pid, Some(cid));
        }
        self.player_logout(conn_id, cid, false, true);
        Ok(())
    }

    /// C++ `BedItem::wakeUp` — `bed.cpp:163-200`.
    pub(crate) fn bed_wake_up(&mut self, item_id: ItemId, player: Option<CreatureId>) {
        if self.bed_house_id(item_id).is_none() {
            return;
        }
        let sleeper_guid = self
            .items
            .get(item_id)
            .map(|i| i.sleeper_guid())
            .unwrap_or(0);
        if sleeper_guid != 0 {
            if let Some(cid) = player {
                self.bed_regenerate_player(cid, item_id);
            }
            self.houses.bed_sleepers.remove(&sleeper_guid);
        }
        let partner = self.bed_partner_item(item_id);
        self.bed_internal_remove_sleeper(item_id);
        if let Some(pid) = partner {
            self.bed_internal_remove_sleeper(pid);
        }
        self.bed_update_appearance(item_id, None);
        if let Some(pid) = partner {
            self.bed_update_appearance(pid, None);
        }
    }

    /// C++ `BedItem::regeneratePlayer` — `bed.cpp:202-227` (HP/mana/soul from sleep duration).
    fn bed_regenerate_player(&mut self, cid: CreatureId, item_id: ItemId) {
        let sleep_start = self
            .items
            .get(item_id)
            .map(|i| i.sleep_start())
            .unwrap_or(0);
        let now = unix_secs();
        let slept = now.saturating_sub(sleep_start);
        let regen = (slept / 30) as i32;
        let soul = (slept / (60 * 15)) as i32;
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.base.health = (p.base.health + regen).min(p.base.max_health);
            p.mana = (p.mana + regen).min(p.max_mana);
            p.economy.soul = p.economy.soul.saturating_add(soul);
        }
    }

    /// C++ `BedItem::updateAppearance` — `bed.cpp:229-245`.
    fn bed_update_appearance(&mut self, item_id: ItemId, sleeper: Option<CreatureId>) {
        let Some(item) = self.items.get(item_id) else {
            return;
        };
        let Some(it) = self.items_db.items.get(&item.item_type) else {
            return;
        };
        if !it.is_bed() {
            return;
        }
        let new_type = if let Some(cid) = sleeper {
            let sex = match self.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => p.sex,
                _ => PlayerSex::Male,
            };
            let dest = it.transform_to_on_use[sex as usize];
            if dest != 0 { dest } else { 0 }
        } else {
            it.transform_to_free
        };
        if new_type == 0 {
            return;
        }
        if !self
            .items_db
            .items
            .get(&new_type)
            .is_some_and(|t| t.is_bed())
        {
            return;
        }
        self.change_item_type(item_id, new_type);
    }

    fn bed_internal_set_sleeper(&mut self, item_id: ItemId, cid: CreatureId) {
        let (name, guid) = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => (p.base.name.clone(), p.guid),
            _ => return,
        };
        let desc = format!("{name} is sleeping there.");
        if let Some(item) = self.items.get_mut(item_id) {
            item.set_sleeper_guid(guid);
            item.set_sleep_start(unix_secs());
            item.set_description(desc);
        }
    }

    fn bed_internal_remove_sleeper(&mut self, item_id: ItemId) {
        if let Some(item) = self.items.get_mut(item_id) {
            item.set_sleeper_guid(0);
            item.set_sleep_start(0);
            item.set_description("Nobody is sleeping there.");
        }
    }

    /// C++ `BedItem::getNextBedItem` — `bed.cpp:68-78`.
    fn bed_partner_item(&self, item_id: ItemId) -> Option<ItemId> {
        let item = self.items.get(item_id)?;
        let it = self.items_db.items.get(&item.item_type)?;
        let dir = Direction::try_from(it.bed_partner_dir).unwrap_or(Direction::North);
        let pos = self.item_tile_pos(item_id)?;
        let target = pos.offset(dir);
        self.bed_item_on_tile(target)
    }

    fn bed_item_on_tile(&self, pos: Position) -> Option<ItemId> {
        let body = self.map.get_tile(pos)?.body();
        body.ground_item
            .into_iter()
            .chain(body.down_items.iter().copied())
            .chain(body.top_items.iter().copied())
            .find(|&id| {
                self.items
                    .get(id)
                    .and_then(|i| self.items_db.items.get(&i.item_type))
                    .is_some_and(|t| t.is_bed())
            })
    }

    fn bed_house_id(&self, item_id: ItemId) -> Option<u32> {
        let pos = self.item_tile_pos(item_id)?;
        match self.map.get_tile(pos)? {
            Tile::House(h) => Some(h.house_id),
            Tile::Normal(_) => None,
        }
    }

    fn item_tile_pos(&self, item_id: ItemId) -> Option<Position> {
        match self.items.get(item_id)?.parent {
            Some(Cylinder::Tile { pos }) => Some(pos),
            _ => None,
        }
    }
}

fn unix_secs() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().min(u32::MAX as u64) as u32)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Item;
    use crate::sim_harness::{beat_driven_test_world, ensure_walkable_tile, insert_player, test_player};
    use crate::tile::{HouseTile, Tile, TileBody};
    use std::sync::Arc;
    use tfs_rust_common::ConnId;
    use tfs_rust_content::otb::ItemType;

    fn register_bed_pair(world: &mut GameWorld) {
        let mut db = (*world.items_db).clone();
        db.items.insert(
            1754,
            ItemType {
                id: 1754,
                server_id: 1754,
                type_tag: tfs_rust_content::items::ITEM_TYPE_BED,
                bed_partner_dir: Direction::South as u8,
                transform_to_on_use: [1762, 1762],
                ..ItemType::default()
            },
        );
        db.items.insert(
            1762,
            ItemType {
                id: 1762,
                server_id: 1762,
                type_tag: tfs_rust_content::items::ITEM_TYPE_BED,
                bed_partner_dir: Direction::South as u8,
                transform_to_free: 1754,
                ..ItemType::default()
            },
        );
        world.items_db = Arc::new(db);
    }

    #[test]
    fn sleep_transforms_free_bed_to_occupied() {
        let mut world = beat_driven_test_world();
        register_bed_pair(&mut world);
        let pos = Position::new(50, 50, 7);
        world.map.insert_tile(
            pos,
            Tile::House(HouseTile {
                inner: TileBody {
                    ground: Some(100),
                    ground_item: None,
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: 0,
                    zone: ZoneType::Protection,
                },
                house_id: 1,
            }),
        );
        ensure_walkable_tile(&mut world.map, Position::new(50, 51, 7), 100);
        world.houses.set_owner(1, 1);
        let mut player = test_player("Sleeper", pos);
        player.premium_ends_at = u32::MAX;
        player.guid = 1;
        let cid = insert_player(&mut world, player);
        world.map.register_creature_at(pos, cid);
        let bed = world.items.insert(Item::new_single(1754));
        world
            .internal_add_item_to_tile(pos, bed, crate::cylinder::CylinderFlags::NO_LIMIT)
            .expect("place bed");
        world
            .player_use_bed(ConnId(1), cid, bed)
            .expect("sleep");
        let ty = world.items.get(bed).map(|i| i.item_type);
        assert_eq!(ty, Some(1762));
        assert_eq!(world.items.get(bed).map(|i| i.sleeper_guid()), Some(1));
    }
}
