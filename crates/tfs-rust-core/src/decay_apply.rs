//! Item decay apply — schedule, pause, transform/remove on expiry.
//!
//! Domain: TFS `Game::startDecay` / `stopDecay` / `internalDecayItem`,
//! `Item::canDecay` (`game.cpp` / `item.cpp`).
//! Outcomes: decompile `CronExpire` / `CronStop` / `ProcessCronSystem`
//! (`map.cc` / `operate.cc`) — deadline fire on all cylinders.

use crate::container_ui::ContainerContentChange;
use crate::cylinder::Cylinder;
use crate::decay::DecayEntry;
use crate::game_world::GameWorld;
use crate::ids::ItemId;
use crate::item::Item;
use crate::item_attributes::DecayState;

/// Resolved fire action for an expired (or immediately decaying) item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecayFire {
    Transform(u16),
    Remove,
    /// `decay_to < 0` — type cannot decay further.
    None,
}

impl GameWorld {
    /// TFS `Item::canDecay` — not removed, typed duration, decayTo ≥ 0, no uniqueId.
    pub fn can_decay(&self, item_id: ItemId) -> bool {
        let Some(item) = self.items.get(item_id) else {
            return false;
        };
        if item
            .attributes
            .as_ref()
            .is_some_and(|a| a.has_unique_id())
        {
            return false;
        }
        let Some(it) = self.items_db.items.get(&item.item_type) else {
            return false;
        };
        if it.decay_time == 0 {
            return false;
        }
        self.effective_decay_to(item) >= 0
    }

    /// TFS `Game::startDecay` — schedule remaining duration, or fire immediately if ≤ 0.
    pub fn start_decay(&mut self, item_id: ItemId) {
        if self.items.get(item_id).is_none() {
            return;
        }
        if !self.can_decay(item_id) {
            return;
        }
        if self
            .items
            .get(item_id)
            .is_some_and(|i| i.decaying() == DecayState::True)
        {
            return;
        }

        let decay_time_sec = self
            .items
            .get(item_id)
            .and_then(|i| self.items_db.items.get(&i.item_type))
            .map(|t| t.decay_time)
            .unwrap_or(0);

        let mut duration_ms = self.item_duration_raw_ms(item_id);
        if duration_ms <= 0 {
            duration_ms = (decay_time_sec as i32).saturating_mul(1000);
            if let Some(item) = self.items.get_mut(item_id) {
                item.set_duration(duration_ms);
            }
        }

        if duration_ms > 0 {
            let replace_with = match self
                .items
                .get(item_id)
                .map(|i| self.effective_decay_to(i))
                .unwrap_or(-1)
            {
                to if to > 0 => Some(to as u16),
                _ => None,
            };
            if let Some(item) = self.items.get_mut(item_id) {
                item.set_decaying(DecayState::True);
            }
            let deadline = self.server_ms.saturating_add(duration_ms as u64);
            self.decay.schedule(item_id, deadline, replace_with);
        } else {
            self.internal_decay_item(item_id, None);
        }
    }

    /// TFS `Game::stopDecay` — cancel cron and keep remaining ms on the item.
    pub fn stop_decay(&mut self, item_id: ItemId) -> u64 {
        let remaining = self
            .decay
            .remaining_ms(item_id, self.server_ms)
            .unwrap_or(0);
        self.decay.cancel(item_id);
        if let Some(item) = self.items.get_mut(item_id) {
            item.set_duration(remaining.min(i32::MAX as u64) as i32);
            item.set_decaying(DecayState::False);
        }
        remaining
    }

    /// Cancel scheduler entry when the item is being destroyed (`CronStop` / `stopDecaying`).
    pub(crate) fn cancel_item_decay(&mut self, item_id: ItemId) {
        self.decay.cancel(item_id);
    }

    /// In-place type change with `stopduration` pause/resume (plan §4.3).
    ///
    /// Domain: TFS `Item::setID` + `transformItem` duration block (`item.cpp` / `game.cpp`).
    /// Outcomes: decompile `ChangeObject` Expire / ExpireStop (`map.cc`).
    pub fn change_item_type(&mut self, item_id: ItemId, new_type: u16) {
        let Some(item) = self.items.get(item_id) else {
            return;
        };
        let old_type = item.item_type;
        if old_type == new_type {
            return;
        }
        if self.items_db.items.get(&new_type).is_none() {
            return;
        }

        let old_decay_time = self
            .items_db
            .items
            .get(&old_type)
            .map(|t| t.decay_time)
            .unwrap_or(0);
        let remaining = if old_decay_time > 0 {
            self.stop_decay(item_id)
        } else {
            self.item_duration_raw_ms(item_id).max(0) as u64
        };

        let (new_stop_time, new_decay_time) = self
            .items_db
            .items
            .get(&new_type)
            .map(|t| (t.stop_time, t.decay_time))
            .unwrap_or((false, 0));

        if let Some(item) = self.items.get_mut(item_id) {
            item.item_type = new_type;
        }

        if new_stop_time {
            if let Some(item) = self.items.get_mut(item_id) {
                item.set_duration(remaining.min(i32::MAX as u64) as i32);
                item.set_decaying(DecayState::False);
            }
        } else if new_decay_time > 0 {
            let duration_ms = if remaining == 0 {
                (new_decay_time as i32).saturating_mul(1000)
            } else {
                remaining.min(i32::MAX as u64) as i32
            };
            if let Some(item) = self.items.get_mut(item_id) {
                item.set_duration(duration_ms);
                item.set_decaying(DecayState::False);
            }
            self.start_decay(item_id);
        } else if let Some(item) = self.items.get_mut(item_id) {
            item.set_duration(0);
            item.set_decaying(DecayState::False);
        }

        self.notify_item_appearance_changed(item_id);
    }

    /// TFS `Game::internalDecayItem` — transform to `decay_to` or remove.
    ///
    /// `replace_hint` comes from a popped [`DecayEntry`] when firing from cron;
    /// otherwise the effective type/attr `decay_to` is used.
    pub fn internal_decay_item(&mut self, item_id: ItemId, replace_hint: Option<u16>) {
        if self.items.get(item_id).is_none() {
            self.decay.cancel(item_id);
            return;
        }

        let fire = match replace_hint {
            Some(id) if id > 0 => DecayFire::Transform(id),
            Some(_) => DecayFire::Remove,
            None => {
                let to = self
                    .items
                    .get(item_id)
                    .map(|i| self.effective_decay_to(i))
                    .unwrap_or(-1);
                match to {
                    t if t > 0 => DecayFire::Transform(t as u16),
                    0 => DecayFire::Remove,
                    _ => DecayFire::None,
                }
            }
        };

        // Cron already removed the entry; cancel any residual / transform reschedule path.
        self.decay.cancel(item_id);

        match fire {
            DecayFire::None => {}
            DecayFire::Transform(new_type) => {
                if self.items_db.items.get(&new_type).is_none() {
                    self.remove_decayed_item(item_id);
                    return;
                }
                self.change_item_type(item_id, new_type);
            }
            DecayFire::Remove => {
                self.remove_decayed_item(item_id);
            }
        }
    }

    /// Cron apply — all cylinders. Equip branch strips abilities first.
    pub(crate) fn process_decay_expiry(&mut self, expired: &[(ItemId, DecayEntry)]) {
        for &(item_id, ref entry) in expired {
            if let Some((cid, slot)) = self.find_equipment_owner(item_id) {
                self.strip_equip_abilities_keep_type(cid, item_id, slot);
            }
            // Scheduler `replace_with: None` means vanish (`decayto` 0).
            match entry.replace_with {
                Some(id) if id > 0 => self.internal_decay_item(item_id, Some(id)),
                _ => self.internal_decay_item(item_id, Some(0)),
            }
        }
    }

    fn item_duration_raw_ms(&self, item_id: ItemId) -> i32 {
        self.items
            .get(item_id)
            .and_then(|i| i.attributes.as_ref())
            .map(|a| a.get_duration_raw())
            .unwrap_or(0)
    }

    /// Attr `DecayTo` override, else [`ItemType::decay_to`] (default −1).
    fn effective_decay_to(&self, item: &Item) -> i32 {
        if let Some(attrs) = item.attributes.as_ref() {
            if attrs.has_decay_to() {
                return attrs.get_decay_to() as i32;
            }
        }
        self.items_db
            .items
            .get(&item.item_type)
            .map(|t| t.decay_to)
            .unwrap_or(-1)
    }

    fn notify_item_appearance_changed(&mut self, item_id: ItemId) {
        let Some(parent) = self.resolve_item_parent_cylinder(item_id) else {
            return;
        };
        match parent {
            Cylinder::Inventory { player_id, slot } => {
                self.broadcast_player_inventory_slot(player_id, slot, Some(item_id));
            }
            Cylinder::Container {
                item_id: container_id,
                ..
            } => {
                if let Some(slot) = self.get_thing_index_in_container(container_id, item_id) {
                    self.notify_container_content_changed(
                        container_id,
                        ContainerContentChange::Update {
                            slot: slot as u16,
                        },
                    );
                } else {
                    self.notify_container_content_changed(
                        container_id,
                        ContainerContentChange::FullRefresh,
                    );
                }
            }
            Cylinder::Tile { pos } => {
                if let Some(tile) = self.map.get_tile(pos) {
                    if let Some(stack_pos) = tile.get_item_stack_pos(item_id) {
                        self.broadcast_tile_item_update(pos, item_id, stack_pos);
                    }
                }
            }
        }
    }

    fn remove_decayed_item(&mut self, item_id: ItemId) {
        self.decay.cancel(item_id);
        match self.resolve_item_parent_cylinder(item_id) {
            Some(Cylinder::Inventory { player_id, slot }) => {
                self.unequip_decayed_item(player_id, slot, item_id);
            }
            Some(Cylinder::Tile { pos }) => {
                let _ = self.internal_remove_item_from_tile(pos, item_id, u16::MAX);
            }
            Some(Cylinder::Container {
                item_id: container_id,
                ..
            }) => {
                let _ = self.container_remove_thing(container_id, item_id, u32::MAX);
            }
            None => {
                self.items.remove(item_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Item;
    use crate::sim_harness::minimal_world;
    use crate::tile::Tile;
    use tfs_rust_common::Position;
    use tfs_rust_content::otb::ItemType;

    fn register_type(world: &mut GameWorld, item_type_id: u16, mut it: ItemType) {
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

    fn place_on_tile(world: &mut GameWorld, pos: Position, type_id: u16) -> ItemId {
        world.map.insert_tile(pos, Tile::empty_normal());
        let iid = world.items.insert(Item::new_single(type_id));
        world
            .map
            .get_tile_mut(pos)
            .expect("tile")
            .add_item(iid);
        iid
    }

    #[test]
    fn start_decay_schedules_deadline_from_typed_duration() {
        let mut world = minimal_world();
        world.server_ms = 10_000;
        let mut it = ItemType::default();
        it.decay_time = 200;
        it.decay_to = 1488;
        register_type(&mut world, 1487, it);

        let iid = world.items.insert(Item::new_single(1487));
        world.start_decay(iid);

        assert_eq!(
            world.items.get(iid).map(|i| i.decaying()),
            Some(DecayState::True)
        );
        assert_eq!(
            world.decay.remaining_ms(iid, world.server_ms),
            Some(200_000)
        );
    }

    #[test]
    fn can_decay_false_with_unique_id() {
        let mut world = minimal_world();
        let mut it = ItemType::default();
        it.decay_time = 100;
        it.decay_to = 0;
        register_type(&mut world, 1000, it);

        let iid = world.items.insert(Item::new_single(1000));
        if let Some(item) = world.items.get_mut(iid) {
            item.set_unique_id(42);
        }
        assert!(!world.can_decay(iid));
        world.start_decay(iid);
        assert!(world.decay.remaining_ms(iid, world.server_ms).is_none());
    }

    #[test]
    fn tile_item_decays_to_next_type_and_reschedules() {
        let mut world = minimal_world();
        world.server_ms = 1_000;

        let mut stage1 = ItemType::default();
        stage1.decay_time = 10;
        stage1.decay_to = 2810;
        register_type(&mut world, 2806, stage1);

        let mut stage2 = ItemType::default();
        stage2.decay_time = 20;
        stage2.decay_to = 0;
        register_type(&mut world, 2810, stage2);

        let pos = Position::new(50, 50, 7);
        let iid = place_on_tile(&mut world, pos, 2806);
        world.start_decay(iid);

        let expired = world.decay.tick(world.server_ms + 10_000);
        assert_eq!(expired.len(), 1);
        world.process_decay_expiry(&expired);

        assert_eq!(world.items.get(iid).map(|i| i.item_type), Some(2810));
        assert!(
            world
                .map
                .get_tile(pos)
                .is_some_and(|t| t.has_item(iid)),
            "transformed item remains on tile"
        );
        assert_eq!(
            world.decay.remaining_ms(iid, world.server_ms),
            Some(20_000)
        );
    }

    #[test]
    fn tile_item_decayto_zero_removes() {
        let mut world = minimal_world();
        world.server_ms = 1_000;

        let mut it = ItemType::default();
        it.decay_time = 5;
        it.decay_to = 0;
        register_type(&mut world, 1490, it);

        let pos = Position::new(51, 51, 7);
        let iid = place_on_tile(&mut world, pos, 1490);
        world.start_decay(iid);

        let expired = world.decay.tick(world.server_ms + 5_000);
        world.process_decay_expiry(&expired);

        assert!(world.items.get(iid).is_none());
        assert!(
            world
                .map
                .get_tile(pos)
                .is_none_or(|t| !t.has_item(iid))
        );
    }

    #[test]
    fn stopduration_pause_keeps_remaining_unscheduled() {
        let mut world = minimal_world();
        world.server_ms = 5_000;

        let mut lit = ItemType::default();
        lit.decay_time = 3000;
        lit.decay_to = 2041;
        register_type(&mut world, 2042, lit);

        let mut unlit = ItemType::default();
        unlit.stop_time = true;
        unlit.decay_time = 0;
        unlit.decay_to = -1;
        register_type(&mut world, 2041, unlit);

        let iid = world.items.insert(Item::new_single(2042));
        world.start_decay(iid);
        // Burn 1000ms of the 3000s lamp.
        world.server_ms = 6_000;
        let before = world.decay.remaining_ms(iid, world.server_ms).unwrap();
        assert!(before < 3_000_000);

        world.change_item_type(iid, 2041);
        assert_eq!(world.items.get(iid).map(|i| i.item_type), Some(2041));
        assert_eq!(
            world.items.get(iid).map(|i| i.decaying()),
            Some(DecayState::False)
        );
        assert!(world.decay.remaining_ms(iid, world.server_ms).is_none());
        let kept = world.item_duration_raw_ms(iid) as u64;
        assert_eq!(kept, before);
    }

    #[test]
    fn stopduration_resume_restarts_with_remaining() {
        let mut world = minimal_world();
        world.server_ms = 1_000;

        let mut lit = ItemType::default();
        lit.decay_time = 100;
        lit.decay_to = 0;
        register_type(&mut world, 2042, lit);

        let mut unlit = ItemType::default();
        unlit.stop_time = true;
        register_type(&mut world, 2041, unlit);

        let iid = world.items.insert(Item::new_single(2042));
        if let Some(item) = world.items.get_mut(iid) {
            item.set_duration(40_000);
        }
        world.start_decay(iid);
        world.server_ms = 11_000; // 10s elapsed → 30s left
        world.change_item_type(iid, 2041);
        let paused = world.item_duration_raw_ms(iid);
        assert_eq!(paused, 30_000);

        world.change_item_type(iid, 2042);
        assert_eq!(
            world.items.get(iid).map(|i| i.decaying()),
            Some(DecayState::True)
        );
        assert_eq!(
            world.decay.remaining_ms(iid, world.server_ms),
            Some(30_000)
        );
    }

    #[test]
    fn tile_add_schedules_decaying_type() {
        use crate::cylinder::CylinderFlags;

        let mut world = minimal_world();
        world.server_ms = 2_000;

        let mut it = ItemType::default();
        it.decay_time = 200;
        it.decay_to = 1488;
        register_type(&mut world, 1487, it);

        let pos = Position::new(60, 60, 7);
        world.map.insert_tile(pos, Tile::empty_normal());
        let iid = world.items.insert(Item::new_single(1487));
        let placed = world
            .internal_add_item_to_tile(pos, iid, CylinderFlags::NO_LIMIT)
            .expect("add");
        assert_eq!(placed, iid);
        assert_eq!(
            world.decay.remaining_ms(iid, world.server_ms),
            Some(200_000)
        );
    }

    #[test]
    fn tile_remove_cancels_decay() {
        use crate::cylinder::CylinderFlags;

        let mut world = minimal_world();
        world.server_ms = 2_000;

        let mut it = ItemType::default();
        it.decay_time = 50;
        it.decay_to = 0;
        register_type(&mut world, 1490, it);

        let pos = Position::new(61, 61, 7);
        world.map.insert_tile(pos, Tile::empty_normal());
        let iid = world.items.insert(Item::new_single(1490));
        world
            .internal_add_item_to_tile(pos, iid, CylinderFlags::NO_LIMIT)
            .expect("add");
        assert!(world.decay.remaining_ms(iid, world.server_ms).is_some());

        world
            .internal_remove_item_from_tile(pos, iid, u16::MAX)
            .expect("remove");
        assert!(world.decay.remaining_ms(iid, world.server_ms).is_none());
        assert!(world.items.get(iid).is_none());
    }
}
