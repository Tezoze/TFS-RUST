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
            self.bed_wake_up(item_id);
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
        if let Some(pos) = self.creatures.get(cid).map(|k| k.position())
            && matches!(self.codec, Codec::V1098(_))
        {
            self.broadcast_magic_effect(pos, ME_SLEEP);
        }
        self.bed_update_appearance(item_id, Some(cid));
        if let Some(pid) = partner {
            self.bed_update_appearance(pid, Some(cid));
        }
        // TFS `BedItem::sleep` → `kickPlayer(id, false)` → `ProtocolGame::logout(false, true)`.
        // Forced so a protection-zone lock cannot keep the body. `logout_allowed` makes
        // the disconnect remove the creature; the body stays until then so the save sees it.
        // `pending_idle_kick` is drained after this beat and closes TCP (`handle_player_disconnect`).
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().logout_allowed = true;
        }
        self.pending_idle_kick.push((conn_id, true));
        Ok(())
    }

    /// 772 `TPlayer::Regenerate` — `crplayer.cc:1373`.
    ///
    /// Runs after the player is on the map. Only a bed on that tile whose text is
    /// the player name (case-insensitive) applies food HP/mana/soul and is cleared.
    /// Waking a bed only clears the sleeper and restores the appearance.
    pub(crate) fn apply_named_bed_login_regen(&mut self, cid: CreatureId) {
        let (name, pos, food, lastlogout, health, max_health, mana, max_mana, soul, soul_max) =
            match self.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => {
                    let lastlogout = p
                        .persist
                        .as_ref()
                        .map(|b| b.player_row.lastlogout)
                        .unwrap_or(0);
                    (
                        p.base.name.clone(),
                        p.base.position,
                        p.food_remaining,
                        lastlogout,
                        p.base.health,
                        p.effective_max_health(),
                        p.mana,
                        p.effective_max_mana(),
                        p.economy.soul,
                        p.vocation_profile.soul_max,
                    )
                }
                _ => return,
            };
        let Some(bed_id) = self.bed_with_owner_name(pos, &name) else {
            return;
        };
        let now = unix_secs() as u64;
        let offline = if lastlogout == 0 {
            0
        } else {
            now.saturating_sub(lastlogout)
        };
        let (food, health, mana, soul) = crate::login::named_bed_offline_regen(
            food, offline, health, max_health, mana, max_mana, soul, soul_max,
        );
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.food_remaining = food;
            p.base.health = health;
            p.mana = mana;
            p.economy.soul = soul;
        }
        if let Some(item) = self.items.get_mut(bed_id) {
            item.reset_text();
        }
        self.bed_wake_up(bed_id);
    }

    /// Bed on `pos` whose text equals `name` — `crplayer.cc:1394` `stricmp`.
    fn bed_with_owner_name(&self, pos: Position, name: &str) -> Option<ItemId> {
        let bed_id = self.bed_item_on_tile(pos)?;
        let text = self.items.get(bed_id)?.text();
        if text.eq_ignore_ascii_case(name) {
            Some(bed_id)
        } else {
            None
        }
    }

    /// Clear the sleeper and restore the free bed. Regen is `TPlayer::Regenerate` only.
    pub(crate) fn bed_wake_up(&mut self, item_id: ItemId) {
        if self.bed_house_id(item_id).is_none() {
            return;
        }
        let sleeper_guid = self
            .items
            .get(item_id)
            .map(|i| i.sleeper_guid())
            .unwrap_or(0);
        if sleeper_guid != 0 {
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
            // `TPlayer::Regenerate` matches `TEXTSTRING` to the sleeper name (`crplayer.cc:1394`).
            item.set_text(name);
        }
    }

    fn bed_internal_remove_sleeper(&mut self, item_id: ItemId) {
        if let Some(item) = self.items.get_mut(item_id) {
            item.set_sleeper_guid(0);
            item.set_sleep_start(0);
            item.set_description("Nobody is sleeping there.");
            item.reset_text();
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
            .chain(body.down_items().iter().copied())
            .chain(body.top_items().iter().copied())
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
    use crate::test_support::{
        beat_driven_test_world, ensure_walkable_tile, insert_player, test_player,
    };
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
                    stacks: None,
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
        world.player_use_bed(ConnId(1), cid, bed).expect("sleep");
        let ty = world.items.get(bed).map(|i| i.item_type);
        assert_eq!(ty, Some(1762));
        assert_eq!(world.items.get(bed).map(|i| i.sleeper_guid()), Some(1));
        assert!(
            world.creatures.get(cid).is_some(),
            "body stays until disconnect saves it"
        );
        assert!(
            world
                .creatures
                .get(cid)
                .is_some_and(|k| k.base().logout_allowed)
        );
        assert_eq!(world.pending_idle_kick, vec![(ConnId(1), true)]);
    }

    #[test]
    fn named_bed_login_applies_food_regen_and_clears_text() {
        let mut world = beat_driven_test_world();
        register_bed_pair(&mut world);
        let pos = Position::new(50, 50, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let mut player = test_player("Alice", pos);
        player.base.health = 40;
        player.mana = 10;
        player.food_remaining = 300;
        player.economy.soul = 0;
        let ago = unix_secs().saturating_sub(1500) as u64;
        if let Some(persist) = player.persist.as_mut() {
            persist.player_row.lastlogout = ago;
        }
        let cid = insert_player(&mut world, player);
        world.map.register_creature_at(pos, cid);
        let bed = world.items.insert(Item::new_single(1754));
        world.items.get_mut(bed).unwrap().set_text("alice");
        world
            .internal_add_item_to_tile(pos, bed, crate::cylinder::CylinderFlags::NO_LIMIT)
            .expect("place bed");

        world.apply_named_bed_login_regen(cid);

        let p = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p,
            _ => panic!("player"),
        };
        // Regen = min(300/3, 1500/15) = 100. HP +25, mana capped, food 0, soul +1.
        assert_eq!(p.base.health, 65);
        assert_eq!(p.mana, 50);
        assert_eq!(p.food_remaining, 0);
        assert_eq!(p.economy.soul, 1);
        assert_eq!(world.items.get(bed).map(|i| i.text()), Some(""));
    }

    /// Sleep writes the name into the bed text. Login uses corpus regen, not slept/30.
    #[test]
    fn sleep_writes_name_and_login_uses_corpus_regen() {
        let mut world = beat_driven_test_world();
        register_bed_pair(&mut world);
        let pos = Position::new(50, 50, 7);
        world.map.insert_tile(
            pos,
            Tile::House(HouseTile {
                inner: TileBody {
                    ground: Some(100),
                    ground_item: None,
                    stacks: None,
                    flags: 0,
                    zone: ZoneType::Protection,
                },
                house_id: 1,
            }),
        );
        world.houses.set_owner(1, 1);
        let mut player = test_player("Sleeper", pos);
        player.premium_ends_at = u32::MAX;
        player.guid = 1;
        player.base.health = 40;
        player.base.max_health = 400;
        player.mana = 10;
        player.max_mana = 200;
        player.food_remaining = 300;
        player.economy.soul = 90;
        player.vocation_profile.soul_max = 100;
        let ago = unix_secs().saturating_sub(1500) as u64;
        if let Some(persist) = player.persist.as_mut() {
            persist.player_row.lastlogout = ago;
        }
        let cid = insert_player(&mut world, player);
        world.map.register_creature_at(pos, cid);
        let bed = world.items.insert(Item::new_single(1754));
        world
            .internal_add_item_to_tile(pos, bed, crate::cylinder::CylinderFlags::NO_LIMIT)
            .expect("place bed");
        world.player_use_bed(ConnId(1), cid, bed).expect("sleep");
        assert_eq!(world.items.get(bed).map(|i| i.text()), Some("Sleeper"));
        // A long sleep would add slept/30 if the TFS wake formula still ran.
        world
            .items
            .get_mut(bed)
            .unwrap()
            .set_sleep_start(unix_secs().saturating_sub(3000));

        world.apply_named_bed_login_regen(cid);
        if let Some(bed_id) = world.houses.bed_sleepers.get(&1).copied() {
            world.bed_wake_up(bed_id);
        }

        let p = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p,
            _ => panic!("player"),
        };
        // Regen = min(300/3, 1500/15) = 100. HP +25, mana +100, food 0, soul +1 capped at 100.
        assert_eq!(p.base.health, 65);
        assert_eq!(p.mana, 110);
        assert_eq!(p.food_remaining, 0);
        assert_eq!(p.economy.soul, 91);
        assert!(p.economy.soul <= p.vocation_profile.soul_max);
        assert_eq!(world.items.get(bed).map(|i| i.text()), Some(""));
        assert_eq!(world.items.get(bed).map(|i| i.sleeper_guid()), Some(0));
    }

    #[test]
    fn login_off_bed_leaves_food_untouched() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(50, 50, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let mut player = test_player("Alice", pos);
        player.base.health = 40;
        player.food_remaining = 300;
        let ago = unix_secs().saturating_sub(1500) as u64;
        if let Some(persist) = player.persist.as_mut() {
            persist.player_row.lastlogout = ago;
        }
        let cid = insert_player(&mut world, player);
        world.map.register_creature_at(pos, cid);
        world.apply_named_bed_login_regen(cid);
        let p = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p,
            _ => panic!("player"),
        };
        assert_eq!(p.food_remaining, 300);
        assert_eq!(p.base.health, 40);
    }
}
