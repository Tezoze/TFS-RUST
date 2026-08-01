//! Item decay apply — schedule, pause, transform/remove on expiry.
//!
//! Domain: TFS `Game::startDecay` / `stopDecay` / `internalDecayItem`,
//! `Item::canDecay` (`game.cpp` / `item.cpp`).
//! Outcomes: decompile `CronExpire` / `CronStop` / `ProcessCronSystem`
//! (`map.cc` / `operate.cc`) — deadline fire on all cylinders.

use crate::container::ContainerType;
use crate::container_ui::ContainerContentChange;
use crate::cylinder::{Cylinder, CylinderFlags};
use crate::decay::DecayEntry;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
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
    /// TVP `Item::canDecay` — uniqueId, quest actionId band, depot policy, typed duration.
    ///
    /// Domain: TVP `item.cpp` `Item::canDecay`; TFS uniqueId + duration/decayTo.
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
        let action_id = item.action_id();
        if (1000..=2000).contains(&action_id) {
            return false;
        }
        let Some(it) = self.items_db.items.get(&item.item_type) else {
            return false;
        };
        if it.decay_time == 0 {
            return false;
        }
        if self.effective_decay_to(item) < 0 {
            return false;
        }
        // TVP `itemsDecayInsideDepots` default false — cached at startup (DEC-4).
        if !self.items_decay_inside_depots && self.is_inside_depot_locker(item_id) {
            return false;
        }
        true
    }

    /// Whether any ancestor container is a depot chest/locker.
    fn is_inside_depot_locker(&self, item_id: ItemId) -> bool {
        if self.container_is_depot_root(item_id) {
            return true;
        }
        let mut cur = match self.resolve_item_parent_cylinder(item_id) {
            Some(Cylinder::Container {
                item_id: parent, ..
            }) => Some(parent),
            _ => None,
        };
        while let Some(id) = cur {
            if self.container_is_depot_root(id) {
                return true;
            }
            cur = self
                .container_registry
                .get(id)
                .and_then(|c| c.parent_container);
        }
        false
    }

    fn container_is_depot_root(&self, id: ItemId) -> bool {
        self.container_registry.get(id).is_some_and(|c| {
            c.container_type == ContainerType::Depot || c.depot_locker_town_id.is_some()
        })
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
            let deadline = self.decay_schedule_deadline(duration_ms);
            self.decay.schedule(item_id, deadline, replace_with);
        } else {
            self.internal_decay_item(item_id, None);
        }
    }

    /// TFS `Game::stopDecay` — cancel cron and keep remaining **item milliseconds** on the item.
    ///
    /// Returns remaining duration in **item ms** (same units as `Item::duration` / `change_item_type`),
    /// not raw decay-clock ticks. On 772 (`RoundNumber`) the heap stores rounds; converting here
    /// prevents `change_item_type` from treating rounds as milliseconds.
    pub fn stop_decay(&mut self, item_id: ItemId) -> u64 {
        let now = self.decay_clock_now();
        let remaining = self
            .decay
            .remaining_ms(item_id, now)
            .unwrap_or(0);
        self.decay.cancel(item_id);
        let item_ms = self.decay_clock_remaining_to_item_ms(remaining);
        if let Some(item) = self.items.get_mut(item_id) {
            item.set_duration(item_ms.min(i32::MAX as u64) as i32);
            item.set_decaying(DecayState::False);
        }
        item_ms
    }

    /// Cancel scheduler entry when the item is being destroyed (`CronStop` / `stopDecaying`).
    pub(crate) fn cancel_item_decay(&mut self, item_id: ItemId) {
        self.decay.cancel(item_id);
    }

    /// In-place type change with `stopduration` pause/resume (plan §4.3).
    ///
    /// Domain: TFS `Item::setID` + `transformItem` duration block (`item.cpp` / `game.cpp`).
    /// Outcomes: decompile `ChangeObject` Expire / ExpireStop (`map.cc`).
    ///
    /// On tiles: `Tile::updateThing` — `resetTileFlags(old)` then `setTileFlags(new)`
    /// (`tile.cpp:963-966`) so door open/close updates `BLOCKSOLID` / walkability.
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

        let tile_pos = match self.resolve_item_parent_cylinder(item_id) {
            Some(crate::cylinder::Cylinder::Tile { pos }) => Some(pos),
            _ => None,
        };

        // Reset old-type tile flags before the id swap — TFS `updateThing`
        // (`tile.cpp:963-966`): `resetTileFlags` then `setTileFlags`.
        if let Some(pos) = tile_pos {
            if let Some(old_it) = self.items_db.items.get(&old_type).cloned() {
                if let Some(tile) = self.map.get_tile(pos) {
                    let rem = crate::map::tile_remaining_props(
                        tile.body(),
                        &self.items,
                        &self.items_db,
                        item_id,
                    );
                    if let Some(tile) = self.map.get_tile_mut(pos) {
                        crate::map::reset_item_tile_flags(
                            tile.body_mut(),
                            &old_it,
                            &rem,
                            &self.items_db,
                        );
                    }
                }
            }
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

        if let Some(pos) = tile_pos {
            if let Some(new_it) = self.items_db.items.get(&new_type).cloned() {
                if let Some(tile) = self.map.get_tile_mut(pos) {
                    crate::map::apply_item_tile_flags(tile.body_mut(), &new_it, &self.items_db);
                }
            }
        }

        self.notify_item_appearance_changed(item_id);
    }

    /// TFS `Game::internalDecayItem` — transform to `decay_to` or remove.
    ///
    /// Outcomes: decompile `ProcessCronSystem` empties containers before `Change`
    /// (`operate.cc` `Empty` + `ProcessCronSystem`).
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
                    self.empty_container_for_expire(item_id, 0);
                    self.remove_decayed_item(item_id);
                    return;
                }
                let remainder = self.expire_empty_remainder(new_type);
                self.empty_container_for_expire(item_id, remainder);
                self.change_item_type(item_id, new_type);
            }
            DecayFire::Remove => {
                self.empty_container_for_expire(item_id, 0);
                self.remove_decayed_item(item_id);
            }
        }
    }

    /// Next-stage container capacity, or 0 if target is not a container (`operate.cc`).
    fn expire_empty_remainder(&self, new_type: u16) -> usize {
        if !self.items_db.is_container(new_type) {
            return 0;
        }
        self.items_db
            .items
            .get(&new_type)
            .map(|t| t.max_items as usize)
            .unwrap_or(0)
    }

    /// Decompile `Empty(Con, Remainder)` before expire transform (`operate.cc`).
    ///
    /// Corpse: delete excess beyond `remainder`. Non-corpse: move excess to parent cylinder.
    /// Batch path: snapshot children once, incremental chain deltas, one carry-weight notify.
    fn empty_container_for_expire(&mut self, container_id: ItemId, remainder: usize) {
        let Some(item_type) = self.items.get(container_id).map(|i| i.item_type) else {
            return;
        };
        if !self.items_db.is_container(item_type) {
            return;
        }

        // Ensure registry entry so we can iterate children (map corpses may be lazy).
        self.hydrate_container_if_needed(container_id);
        let Some((_viewers, children)) = self.container_registry.get(container_id).map(|c| {
            (c.open_by.clone(), c.items.clone())
        }) else {
            return;
        };

        let count = children.len();
        if count <= remainder {
            return;
        }

        let is_corpse = self.is_corpse_item_type(item_type);
        let excess = count - remainder;
        let to_remove: Vec<ItemId> = children[..excess].to_vec();

        let mut total_weight_delta = 0u32;
        let mut total_count_delta = 0u32;
        for &child in &to_remove {
            let (w, h) = self.item_subtree_weight_and_holding_count(child);
            total_weight_delta = total_weight_delta.saturating_add(w);
            total_count_delta = total_count_delta.saturating_add(h);
        }

        if let Some(cont) = self.container_registry.get_mut(container_id) {
            if remainder == 0 {
                cont.items.clear();
            } else {
                cont.items.drain(..excess);
            }
        }
        for &child in &to_remove {
            if let Some(ch) = self.container_registry.get_mut(child) {
                ch.parent_container = None;
            }
            if let Some(item) = self.items.get_mut(child) {
                item.parent = None;
            }
        }

        self.apply_container_remove_delta_chain(
            container_id,
            total_weight_delta,
            total_count_delta,
        );
        self.notify_container_front_removals(container_id, excess);

        for child in to_remove {
            if is_corpse {
                self.destroy_item_tree(child);
            } else {
                self.move_detached_item_to_parent_of(container_id, child);
            }
        }

        // 772 `CloseContainer(Con, false)` keeps the window open and refreshes it when the
        // container is still accessible (`operate.cc:1060-1100`).
        self.refresh_container_ui_for_all_viewers(container_id);
    }

    fn is_corpse_item_type(&self, item_type: u16) -> bool {
        self.items_db
            .items
            .get(&item_type)
            .is_some_and(|t| t.xml_attributes.contains_key("corpsetype"))
    }

    /// Iteratively destroy an item and nested container contents (post-order stack).
    ///
    /// Outcomes: decompile `Empty` corpse delete path (`operate.cc`).
    fn destroy_item_tree(&mut self, root: ItemId) {
        let mut stack = vec![root];
        let mut destroy_order = Vec::new();
        while let Some(id) = stack.pop() {
            destroy_order.push(id);
            if let Some(children) = self.container_registry.get(id).map(|c| c.items.clone()) {
                for child in children {
                    stack.push(child);
                }
            }
        }
        destroy_order.reverse();
        for item_id in destroy_order {
            self.auto_close_containers_for_all_viewers_of(item_id);
            if let Some(reg) = self.container_registry.get_mut(item_id) {
                reg.items.clear();
            }
            self.cancel_item_decay(item_id);
            self.items.remove(item_id);
        }
    }

    fn auto_close_containers_for_all_viewers_of(&mut self, container_id: ItemId) {
        let viewers: Vec<CreatureId> = self
            .container_registry
            .get(container_id)
            .map(|c| c.open_by.clone())
            .unwrap_or_default();
        for viewer in viewers {
            self.auto_close_containers_for_container_item(viewer, container_id);
        }
    }

    /// Move a detached child onto the parent cylinder of `container_id` (non-corpse Empty).
    fn move_detached_item_to_parent_of(&mut self, container_id: ItemId, child: ItemId) {
        match self.resolve_item_parent_cylinder(container_id) {
            Some(Cylinder::Tile { pos }) => {
                let _ = self.internal_add_item_to_tile(pos, child, CylinderFlags::NO_LIMIT);
            }
            Some(Cylinder::Container {
                item_id: parent, ..
            }) => {
                let _ = self.container_add_thing(parent, 0, child);
            }
            Some(Cylinder::Inventory { player_id, slot }) => {
                // Prefer tile under player if inventory add is awkward; try container-like fallback.
                if let Some(pos) = self.creatures.get(player_id).map(|c| c.position()) {
                    let _ = self.internal_add_item_to_tile(pos, child, CylinderFlags::NO_LIMIT);
                } else {
                    let _ = slot; // unused — destroy if no position
                    self.destroy_item_tree(child);
                }
            }
            None => {
                self.destroy_item_tree(child);
            }
        }
    }

    /// Cron apply — all cylinders. Equip branch strips abilities first.
    ///
    /// O(1) equip resolution via [`Item::parent`] — not `find_equipment_owner` scan.
    pub(crate) fn process_decay_expiry(&mut self, expired: &[(ItemId, DecayEntry)]) {
        for &(item_id, ref entry) in expired {
            if let Some(Cylinder::Inventory { player_id, slot }) =
                self.resolve_item_parent_cylinder(item_id)
            {
                self.strip_equip_abilities_keep_type(player_id, item_id, slot);
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
                let (tvp_stack_pos, cip_stack_pos) = self.item_stack_pos_pair(pos, item_id);
                // Only broadcast if the item is still findable on the tile.
                if self
                    .map
                    .get_tile(pos)
                    .and_then(|t| t.get_item_stack_pos(item_id))
                    .is_some()
                {
                    self.broadcast_tile_item_update(pos, item_id, tvp_stack_pos, cip_stack_pos);
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
    use crate::creature::CreatureKind;
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
        if let Some(item) = world.items.get_mut(iid) {
            item.parent = Some(crate::cylinder::Cylinder::Tile { pos });
        }
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
    fn stop_decay_returns_item_ms_under_round_clock() {
        use crate::formulas::DecayClockModel;

        let mut world = minimal_world();
        world.mechanics.profile.decay_clock = DecayClockModel::RoundNumber;
        world.round_nr = 100;

        let mut lit = ItemType::default();
        lit.decay_time = 30; // 30s → 30 rounds on schedule
        lit.decay_to = 2041;
        register_type(&mut world, 2042, lit);

        let mut unlit = ItemType::default();
        unlit.stop_time = true;
        register_type(&mut world, 2041, unlit);

        let iid = world.items.insert(Item::new_single(2042));
        world.start_decay(iid);
        world.round_nr = 110; // 10 rounds elapsed → 20s left
        assert_eq!(
            world.stop_decay(iid),
            20_000,
            "stop_decay must return item ms, not rounds"
        );
        assert_eq!(world.item_duration_raw_ms(iid), 20_000);

        // Resume path via change_item_type must keep item-ms (not raw rounds).
        if let Some(item) = world.items.get_mut(iid) {
            item.set_duration(20_000);
        }
        world.start_decay(iid);
        world.round_nr = 110;
        world.change_item_type(iid, 2041);
        assert_eq!(world.item_duration_raw_ms(iid), 20_000);
    }

    #[test]
    fn item_decay_remaining_ms_uses_round_clock() {
        use crate::formulas::DecayClockModel;

        let mut world = minimal_world();
        world.mechanics.profile.decay_clock = DecayClockModel::RoundNumber;
        world.round_nr = 100;
        world.server_ms = 50_000; // must not be used as heap "now"

        let mut it = ItemType::default();
        it.decay_time = 30;
        it.decay_to = 0;
        register_type(&mut world, 1490, it);

        let iid = world.items.insert(Item::new_single(1490));
        world.start_decay(iid);
        world.round_nr = 110;
        assert_eq!(
            world.item_decay_remaining_ms(iid),
            Some(20_000),
            "look/save helper must convert rounds → item ms"
        );
        // Bug shape: querying with server_ms would yield 0 / nonsense.
        assert_eq!(
            world.decay.remaining_ms(iid, world.server_ms),
            Some(0),
            "sanity: raw heap query with server_ms is wrong on RoundNumber"
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

    #[test]
    fn can_decay_false_with_quest_action_id() {
        let mut world = minimal_world();
        let mut it = ItemType::default();
        it.decay_time = 100;
        it.decay_to = 0;
        register_type(&mut world, 1001, it);

        let iid = world.items.insert(Item::new_single(1001));
        if let Some(item) = world.items.get_mut(iid) {
            item.set_action_id(1500);
        }
        assert!(!world.can_decay(iid));
    }

    #[test]
    fn can_decay_false_inside_depot_by_default() {
        use crate::test_world::support::{insert_player, test_player};

        let mut world = minimal_world();
        let mut it = ItemType::default();
        it.decay_time = 100;
        it.decay_to = 0;
        register_type(&mut world, 2169, it);

        let pos = Position::new(50, 50, 7);
        let cid = insert_player(&mut world, test_player("depot_decay", pos));
        let chest = world
            .player_get_depot_chest(cid, 1, true)
            .expect("depot chest");
        let iid = world.items.insert(Item::new_single(2169));
        if let Some(cont) = world.container_registry.get_mut(chest) {
            let _ = cont.add_item(iid);
        }
        if let Some(item) = world.items.get_mut(iid) {
            item.parent = Some(crate::cylinder::Cylinder::Container {
                item_id: chest,
                index: crate::cylinder::INDEX_WHEREEVER,
            });
        }
        assert!(!world.can_decay(iid));
        world.start_decay(iid);
        assert!(world.decay.remaining_ms(iid, world.server_ms).is_none());
    }

    #[test]
    fn corpse_empty_trims_excess_before_stage_transform() {
        use crate::container::Container;
        use std::collections::HashMap;

        let mut world = minimal_world();
        world.server_ms = 1_000;

        let mut stage1 = ItemType::default();
        stage1.group = ItemType::GROUP_CONTAINER;
        stage1.max_items = 7;
        stage1.decay_time = 10;
        stage1.decay_to = 2810;
        stage1.xml_attributes = HashMap::from([("corpsetype".into(), "blood".into())]);
        register_type(&mut world, 2806, stage1);

        let mut stage2 = ItemType::default();
        stage2.group = ItemType::GROUP_CONTAINER;
        stage2.max_items = 3;
        stage2.decay_time = 20;
        stage2.decay_to = 0;
        stage2.xml_attributes = HashMap::from([("corpsetype".into(), "blood".into())]);
        register_type(&mut world, 2810, stage2);

        let pos = Position::new(70, 70, 7);
        let corpse = place_on_tile(&mut world, pos, 2806);
        world
            .container_registry
            .register(Container::new(corpse, 7));

        let mut loot_ids = Vec::new();
        for _ in 0..5 {
            let loot = world.items.insert(Item::new_single(2148));
            if let Some(cont) = world.container_registry.get_mut(corpse) {
                let _ = cont.add_item(loot);
            }
            loot_ids.push(loot);
        }

        world.start_decay(corpse);
        let expired = world.decay.tick(world.server_ms + 10_000);
        world.process_decay_expiry(&expired);

        assert_eq!(world.items.get(corpse).map(|i| i.item_type), Some(2810));
        let remaining = world
            .container_registry
            .get(corpse)
            .map(|c| c.items.len())
            .unwrap_or(0);
        assert_eq!(remaining, 3);
        // First two children destroyed (Empty walks from front).
        assert!(world.items.get(loot_ids[0]).is_none());
        assert!(world.items.get(loot_ids[1]).is_none());
        assert!(world.items.get(loot_ids[2]).is_some());
    }

    #[test]
    fn non_corpse_empty_moves_excess_to_tile() {
        use crate::container::Container;

        let mut world = minimal_world();
        world.server_ms = 1_000;

        let mut bag = ItemType::default();
        bag.group = ItemType::GROUP_CONTAINER;
        bag.max_items = 5;
        bag.decay_time = 5;
        bag.decay_to = 0;
        register_type(&mut world, 1988, bag);

        let pos = Position::new(71, 71, 7);
        let bag_id = place_on_tile(&mut world, pos, 1988);
        world
            .container_registry
            .register(Container::new(bag_id, 5));

        let loot = world.items.insert(Item::new_single(2148));
        if let Some(cont) = world.container_registry.get_mut(bag_id) {
            let _ = cont.add_item(loot);
        }

        world.start_decay(bag_id);
        let expired = world.decay.tick(world.server_ms + 5_000);
        world.process_decay_expiry(&expired);

        assert!(world.items.get(bag_id).is_none());
        assert!(
            world
                .map
                .get_tile(pos)
                .is_some_and(|t| t.has_item(loot)),
            "loot moved to tile before bag removed"
        );
    }

    #[test]
    fn field_expire_transforms_via_cron() {
        use crate::cylinder::CylinderFlags;

        let mut world = minimal_world();
        world.server_ms = 1_000;

        let mut fire = ItemType::default();
        fire.decay_time = 200;
        fire.decay_to = 1488;
        register_type(&mut world, 1487, fire);

        let mut mid = ItemType::default();
        mid.decay_time = 200;
        mid.decay_to = 0;
        register_type(&mut world, 1488, mid);

        let pos = Position::new(72, 72, 7);
        world.map.insert_tile(pos, Tile::empty_normal());
        let iid = world.items.insert(Item::new_single(1487));
        world
            .internal_add_item_to_tile(pos, iid, CylinderFlags::NO_LIMIT)
            .expect("add field");

        let expired = world.decay.tick(world.server_ms + 200_000);
        world.process_decay_expiry(&expired);

        assert_eq!(world.items.get(iid).map(|i| i.item_type), Some(1488));
        assert!(world.decay.remaining_ms(iid, world.server_ms).is_some());
    }

    #[test]
    fn equipped_decay_strips_abilities_via_parent() {
        use crate::inventory::InventorySlot;
        use crate::test_world::support::{insert_player, test_player};
        use tfs_rust_content::item_abilities::ItemAbilities;

        let mut world = minimal_world();
        world.server_ms = 1_000;
        let pos = Position::new(80, 80, 7);
        let cid = insert_player(&mut world, test_player("ring_decay", pos));
        let slot = InventorySlot::Ring as u8;

        let mut ring = ItemType::default();
        ring.decay_time = 10;
        ring.decay_to = 0;
        let mut abl = ItemAbilities::default();
        abl.speed = 15;
        ring.abilities = abl;
        register_type(&mut world, 2169, ring);

        let iid = world.items.insert(Item::new_single(2169));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            let idx = crate::inventory::slot_to_array_index(slot).expect("slot");
            p.equipment_slots[idx] = Some(iid);
        }
        if let Some(item) = world.items.get_mut(iid) {
            item.parent = Some(crate::cylinder::Cylinder::Inventory {
                player_id: cid,
                slot,
            });
        }
        world.apply_equip_item_abilities(cid, iid, slot);
        assert_eq!(
            match world.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => p.base.var_speed,
                _ => panic!("player"),
            },
            15
        );

        world.start_decay(iid);
        let expired = world.decay.tick(world.server_ms + 10_000);
        assert_eq!(expired.len(), 1);
        world.process_decay_expiry(&expired);

        assert!(world.items.get(iid).is_none());
        assert_eq!(
            match world.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => p.base.var_speed,
                _ => panic!("player"),
            },
            0,
            "equip abilities stripped via parent cylinder, not owner scan"
        );
    }

    #[test]
    fn decay_burst_processes_many_expiries() {
        let mut world = minimal_world();
        world.server_ms = 1_000;

        let mut it = ItemType::default();
        it.decay_time = 5;
        it.decay_to = 0;
        register_type(&mut world, 1490, it);

        let mut ids = Vec::new();
        for i in 0..12 {
            let pos = Position::new(90 + i, 90, 7);
            let iid = place_on_tile(&mut world, pos, 1490);
            world.start_decay(iid);
            ids.push(iid);
        }

        let expired = world.decay.tick(world.server_ms + 5_000);
        assert_eq!(expired.len(), 12);
        world.process_decay_expiry(&expired);

        for iid in ids {
            assert!(world.items.get(iid).is_none(), "burst item removed");
        }
    }

    /// Audit #5 full-scale: thousands of tile expiries with wall-time bound.
    #[test]
    fn decay_burst_thousands_bounded_wall_time() {
        use std::time::Instant;

        let mut world = minimal_world();
        world.server_ms = 1_000;

        let mut it = ItemType::default();
        it.decay_time = 5;
        it.decay_to = 0;
        register_type(&mut world, 1490, it);

        const N: usize = 2_000;
        let mut ids = Vec::with_capacity(N);
        for i in 0..N {
            let x = 50 + (i % 200) as u16;
            let y = 50 + (i / 200) as u16;
            let pos = Position::new(x, y, 7);
            let iid = place_on_tile(&mut world, pos, 1490);
            world.start_decay(iid);
            ids.push(iid);
        }

        let start = Instant::now();
        let expired = world.decay.tick(world.server_ms + 5_000);
        assert_eq!(expired.len(), N);
        world.process_decay_expiry(&expired);
        let elapsed = start.elapsed();

        for iid in &ids {
            assert!(world.items.get(*iid).is_none(), "burst item removed");
        }
        assert!(
            elapsed.as_secs() < 5,
            "2k decay burst must finish in <5s (took {elapsed:?})"
        );
    }

    #[test]
    fn corpse_empty_remainder_zero_destroys_all_loot() {
        use crate::container::Container;
        use std::collections::HashMap;

        let mut world = minimal_world();
        world.server_ms = 1_000;

        let mut stage1 = ItemType::default();
        stage1.group = ItemType::GROUP_CONTAINER;
        stage1.max_items = 7;
        stage1.decay_time = 10;
        stage1.decay_to = 2148;
        stage1.xml_attributes = HashMap::from([("corpsetype".into(), "blood".into())]);
        register_type(&mut world, 2806, stage1);

        let mut coin = ItemType::default();
        coin.decay_time = 0;
        coin.decay_to = -1;
        register_type(&mut world, 2148, coin);

        let pos = Position::new(75, 75, 7);
        let corpse = place_on_tile(&mut world, pos, 2806);
        world
            .container_registry
            .register(Container::new(corpse, 7));

        let mut loot_ids = Vec::new();
        for _ in 0..5 {
            let loot = world.items.insert(Item::new_single(2148));
            if let Some(cont) = world.container_registry.get_mut(corpse) {
                let _ = cont.add_item(loot);
            }
            if let Some(item) = world.items.get_mut(loot) {
                item.parent = Some(crate::cylinder::Cylinder::Container {
                    item_id: corpse,
                    index: crate::cylinder::INDEX_WHEREEVER,
                });
            }
            loot_ids.push(loot);
        }

        world.start_decay(corpse);
        let expired = world.decay.tick(world.server_ms + 10_000);
        world.process_decay_expiry(&expired);

        assert_eq!(world.items.get(corpse).map(|i| i.item_type), Some(2148));
        assert!(
            world
                .container_registry
                .get(corpse)
                .is_none_or(|c| c.items.is_empty()),
            "remainder==0 fast path clears all corpse loot"
        );
        for loot in loot_ids {
            assert!(world.items.get(loot).is_none());
        }
    }

    /// Phase 2 doors: closed→open transform must clear `BLOCKSOLID` so the tile is walkable.
    /// C++ `Tile::updateThing` — `resetTileFlags` + `setTileFlags` (`tile.cpp:963-966`).
    #[test]
    fn transform_door_updates_blocksolid_tile_flag() {
        use crate::cylinder::CylinderFlags;
        use crate::tile::flags;

        let mut world = minimal_world();
        const CLOSED: u16 = 9101;
        const OPEN: u16 = 9102;

        let mut closed = ItemType::default();
        closed.block_solid_override = Some(true);
        closed.moveable_override = Some(false);
        register_type(&mut world, CLOSED, closed);

        let mut open = ItemType::default();
        open.block_solid_override = Some(false);
        open.moveable_override = Some(false);
        register_type(&mut world, OPEN, open);

        let pos = Position::new(50, 50, 7);
        world.map.insert_tile(pos, Tile::empty_normal());
        if let Some(tile) = world.map.get_tile_mut(pos) {
            tile.body_mut().ground = Some(100);
        }

        let iid = world.items.insert(Item::new_single(CLOSED));
        world
            .internal_add_item_to_tile(pos, iid, CylinderFlags::NO_LIMIT)
            .expect("place closed door");

        assert_ne!(
            world.map.get_tile(pos).expect("tile").body().flags & flags::BLOCKSOLID,
            0,
            "closed door must set BLOCKSOLID"
        );

        world.change_item_type(iid, OPEN);
        assert_eq!(world.items.get(iid).map(|i| i.item_type), Some(OPEN));
        assert_eq!(
            world.map.get_tile(pos).expect("tile").body().flags & flags::BLOCKSOLID,
            0,
            "open door must clear BLOCKSOLID"
        );

        world.change_item_type(iid, CLOSED);
        assert_ne!(
            world.map.get_tile(pos).expect("tile").body().flags & flags::BLOCKSOLID,
            0,
            "re-close must restore BLOCKSOLID"
        );
    }

    /// Phase 2: magic-field remove clears `MAGICFIELD` tile flag.
    #[test]
    fn remove_magic_field_clears_magicfield_flag() {
        use crate::cylinder::CylinderFlags;
        use crate::tile::flags;

        let mut world = minimal_world();
        const FIELD: u16 = 9103;
        let mut field = ItemType::default();
        field.type_tag = 6; // ITEM_TYPE_MAGICFIELD
        register_type(&mut world, FIELD, field);

        let pos = Position::new(51, 51, 7);
        world.map.insert_tile(pos, Tile::empty_normal());
        if let Some(tile) = world.map.get_tile_mut(pos) {
            tile.body_mut().ground = Some(100);
        }

        let iid = world.items.insert(Item::new_single(FIELD));
        world
            .internal_add_item_to_tile(pos, iid, CylinderFlags::NO_LIMIT)
            .expect("place field");
        assert_ne!(
            world.map.get_tile(pos).expect("tile").body().flags & flags::MAGICFIELD,
            0
        );

        world
            .internal_remove_item_from_tile(pos, iid, u16::MAX)
            .expect("remove field");
        assert_eq!(
            world.map.get_tile(pos).expect("tile").body().flags & flags::MAGICFIELD,
            0,
            "removing magic field must clear MAGICFIELD flag"
        );
    }
}
