//! Inventory cylinder moves, equip UI updates, quick-equip, look-at.
// C++ reference: `src/game.cpp` `internalMoveItem`, `playerEquipItem`, `playerLookAt`.

/// 772 `SKILL_FED` `Max` — food counter cap. The decompile defaults `Max` to
/// `INT_MAX` (`crskill.cc:234`), but `food.lua` uses `1200` as the "full"
/// threshold (`data/scripts/actions/other/food.lua:52`). We cap at 1200 to
/// match the Lua script's observable behavior.
pub(crate) const MAX_FOOD: u32 = 1200;

use tfs_rust_common::ConnId;
use tfs_rust_common::Position;
use tfs_rust_net::codec::ItemTemplateArgs;
use tfs_rust_net::outgoing_extra::{send_inventory_slot_empty, send_text_message_simple};

use crate::container_ui::ContainerContentChange;
use crate::creature::CreatureKind;
use crate::cylinder::{Cylinder, CylinderFlags, INDEX_WHEREEVER};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::{slot_type_for_item_type, InventorySlot};
use crate::item::Item;
use crate::item_look::{item_get_description_cpp, look_distance_tfs};
use crate::player_inventory_notifications::NotificationParent;
use crate::player_inventory_util::{InventoryItemRef, ItemCylinder};
use crate::return_value::ReturnValue;
use crate::thing::LookTarget;
use slotmap::Key;

impl GameWorld {
    /// O(1) reverse lookup via `creature_to_conn` (audit #4).
    pub(crate) fn conn_id_for_creature(&self, cid: CreatureId) -> Option<ConnId> {
        self.creature_to_conn.get(&cid).copied()
    }

    /// Remaining duration for look `showduration` — scheduler when decaying, else raw attr.
    pub(crate) fn item_look_duration_ms(&self, item_id: ItemId) -> Option<i32> {
        let item = self.items.get(item_id)?;
        let attrs = item.attributes.as_ref()?;
        if !attrs.has_duration() {
            return None;
        }
        if item.decaying() == crate::item_attributes::DecayState::True {
            if let Some(rem) = self.item_decay_remaining_ms(item_id) {
                return Some(rem.min(i32::MAX as u64) as i32);
            }
        }
        Some(attrs.get_duration_raw())
    }

    /// `Player` inventory slot item — `Game::internalGetThing` inventory branch (`game.cpp` ~320–326).
    pub fn get_player_inventory_item(&self, cid: CreatureId, slot: u8) -> Option<ItemId> {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return None;
        };
        let idx = crate::inventory::slot_to_array_index(slot)?;
        p.equipment_slots[idx]
    }

    pub(crate) fn validate_inventory_item(
        &self,
        cid: CreatureId,
        slot: u8,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        let Some(have) = self.get_player_inventory_item(cid, slot) else {
            return Err(ReturnValue::NotPossible);
        };
        if have != item_id {
            return Err(ReturnValue::NotPossible);
        }
        Ok(())
    }

    /// Capacity-only path for child containers (`Player::queryAdd` + `FLAG_CHILDISOWNER`).
    pub(crate) fn query_add_item_to_inventory(
        &mut self,
        cid: CreatureId,
        item_id: ItemId,
    ) -> ReturnValue {
        let count = self.items.get(item_id).map(|i| i.count as u32).unwrap_or(1);
        self.player_query_add(
            cid,
            crate::inventory::InventorySlot::Wherever as u8,
            item_id,
            count,
            CylinderFlags::CHILD_IS_OWNER,
        )
    }

    pub(crate) fn internal_remove_item_from_inventory_slot(
        &mut self,
        cid: CreatureId,
        slot: u8,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        self.validate_inventory_item(cid, slot, item_id)?;
        let idx = crate::inventory::slot_to_array_index(slot).ok_or(ReturnValue::NotPossible)?;
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.equipment_slots[idx] = None;
        }
        if let Some(item) = self.items.get_mut(item_id) {
            item.parent = None;
        }
        Ok(())
    }

    pub(crate) fn internal_add_item_to_inventory_slot(
        &mut self,
        cid: CreatureId,
        slot: u8,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        let idx = crate::inventory::slot_to_array_index(slot).ok_or(ReturnValue::NotPossible)?;
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.equipment_slots[idx] = Some(item_id);
        }
        if let Some(item) = self.items.get_mut(item_id) {
            item.parent = Some(crate::cylinder::Cylinder::Inventory {
                player_id: cid,
                slot,
            });
        }
        // C++: carrying a bag updates parent chain / `totalWeight` — ground→inventory must keep registry in sync for look/capacity.
        self.hydrate_container_if_needed(item_id);
        Ok(())
    }

    /// Slot mutation + `postAddNotification` (Owner) + 0x78.
    pub(crate) fn equip_item_to_inventory_slot(
        &mut self,
        cid: CreatureId,
        slot: u8,
        item_id: ItemId,
        old_parent: crate::player_inventory_notifications::NotificationParent,
    ) -> Result<(), ReturnValue> {
        self.internal_add_item_to_inventory_slot(cid, slot, item_id)?;
        self.notify_player_inventory_slot_add(cid, slot, item_id, old_parent);
        self.start_decay(item_id);
        Ok(())
    }

    /// Slot mutation + `postRemoveNotification` (Owner) + 0x78 empty.
    pub(crate) fn unequip_item_from_inventory_slot(
        &mut self,
        cid: CreatureId,
        slot: u8,
        item_id: ItemId,
        new_parent: crate::player_inventory_notifications::NotificationParent,
    ) -> Result<(), ReturnValue> {
        self.internal_remove_item_from_inventory_slot(cid, slot, item_id)?;
        self.notify_player_inventory_slot_remove(cid, slot, item_id, new_parent);
        Ok(())
    }

    /// Remove a specific item instance from a registered container (`container.cpp`).
    // Parity helper; pairs with the inventory-slot remove path. Retained ahead of caller.
    #[allow(dead_code)]
    pub(crate) fn internal_remove_item_from_container(
        &mut self,
        container_item_id: ItemId,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        self.container_detach_item(container_item_id, item_id)
    }

    /// Resolve `CreatureId` from Lua / protocol `KeyData::as_ffi` bits.
    pub(crate) fn resolve_creature_u64(&self, id: u64) -> Option<CreatureId> {
        self.creatures
            .iter()
            .find(|(k, _)| k.data().as_ffi() == id)
            .map(|(k, _)| k)
    }

    /// Lua `Player:addItem` — adds a new stack into the backpack container (`luascript.cpp` `Player::addItem` subset).
    pub fn lua_script_add_item(
        &mut self,
        creature_u64: u64,
        item_type: u16,
        count: u16,
    ) -> Result<(), String> {
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "creature not found".to_string())?;
        let count = count.max(1);
        let new_item = Item::new(item_type, count);
        let iid = self.items.insert(new_item);
        if self.query_add_item_to_inventory(cid, iid) != ReturnValue::NoError {
            self.items.remove(iid);
            return Err("not enough capacity".into());
        }
        let backpack = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.equipment_slots[2],
            _ => None,
        };
        let Some(bp) = backpack else {
            self.items.remove(iid);
            return Err("no backpack".into());
        };
        let mut registry = std::mem::take(&mut self.container_registry);
        self.ensure_container_registered_simple(&mut registry, bp, cid);
        self.container_registry = registry;
        // C++ `Container::addThing` does `push_front` + `onAddContainerItem`
        // (`container.cpp:524-530`). `container_add_thing` mirrors both: insert at index 0 and
        // notify all viewers (`0x70` addContainerItem) + owner carry-weight/stats. The previous
        // `cont.add_item` (push to end) + `refresh_container_chain` (weights only) sent no
        // client notification and used the wrong slot order.
        if self.container_add_thing(bp, 0, iid).is_err() {
            self.items.remove(iid);
            return Err("backpack full".into());
        }
        Ok(())
    }

    /// Lua `Player:removeItem` — `luascript.cpp` `luaPlayerRemoveItem` → `removeItemOfType`.
    pub fn lua_script_remove_item(
        &mut self,
        creature_u64: u64,
        item_type: u16,
        count: u32,
        sub_type: i32,
        ignore_equipped: bool,
    ) -> Result<(), String> {
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "creature not found".to_string())?;
        if !self.player_remove_item_of_type(cid, item_type, count, sub_type, ignore_equipped) {
            return Err("not enough items".into());
        }
        Ok(())
    }

    /// Default Lua `item:moveTo` flags — `luascript.cpp` `luaItemMoveTo`.
    pub(crate) fn lua_default_move_flags() -> CylinderFlags {
        CylinderFlags::NO_LIMIT
            .union(CylinderFlags::IGNORE_BLOCK_ITEM)
            .union(CylinderFlags::IGNORE_BLOCK_CREATURE)
            .union(CylinderFlags::IGNORE_NOT_MOVEABLE)
    }

    /// Lua `player:getDepotChest` — `luascript.cpp` `luaPlayerGetDepotChest`.
    pub fn lua_script_get_depot_chest(
        &mut self,
        creature_u64: u64,
        depot_id: u32,
        auto_create: bool,
    ) -> Result<Option<u64>, String> {
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "creature not found".to_string())?;
        let chest = self.player_get_depot_chest(cid, depot_id, auto_create);
        if let Some(iid) = chest {
            self.player_set_last_depot_id(cid, depot_id);
            Ok(Some(iid.data().as_ffi()))
        } else {
            Ok(None)
        }
    }

    /// Lua `player:getInbox` — `luascript.cpp` `luaPlayerGetInbox`.
    pub fn lua_script_get_inbox(&mut self, creature_u64: u64) -> Result<Option<u64>, String> {
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "creature not found".to_string())?;
        Ok(self.player_get_inbox(cid, true).map(|i| i.data().as_ffi()))
    }

    /// Lua `item:moveTo` — `luascript.cpp` `luaItemMoveTo`.
    pub fn lua_script_item_move_to(
        &mut self,
        item_u64: u64,
        dest: tfs_rust_lua::LuaMoveDestination,
        flags_bits: u32,
    ) -> Result<bool, String> {
        use tfs_rust_lua::LuaMoveDestination;
        let item_id = self
            .resolve_item_u64(item_u64)
            .ok_or_else(|| "item not found".to_string())?;
        let from = self
            .resolve_item_parent_cylinder(item_id)
            .ok_or_else(|| "item has no parent".to_string())?;
        let flags = if flags_bits == 0 {
            Self::lua_default_move_flags()
        } else {
            CylinderFlags { bits: flags_bits }
        };
        let acting = self
            .resolve_creature_u64(match from {
                Cylinder::Inventory { player_id, .. } => player_id.data().as_ffi(),
                _ => 0,
            })
            .or_else(|| dest_player_creature(&dest).and_then(|u| self.resolve_creature_u64(u)));
        let to = match dest {
            LuaMoveDestination::Container { item_id: cid_u64 } => {
                let cid = self
                    .resolve_item_u64(cid_u64)
                    .ok_or_else(|| "container not found".to_string())?;
                Cylinder::Container {
                    item_id: cid,
                    index: INDEX_WHEREEVER,
                }
            }
            LuaMoveDestination::Player { creature_id } => {
                let pid = self
                    .resolve_creature_u64(creature_id)
                    .ok_or_else(|| "player not found".to_string())?;
                Cylinder::Inventory {
                    player_id: pid,
                    slot: InventorySlot::Wherever as u8,
                }
            }
            LuaMoveDestination::Tile { x, y, z } => Cylinder::Tile {
                pos: tfs_rust_common::Position::new(x, y, z),
            },
        };
        if from == to {
            return Ok(true);
        }
        let count = self.items.get(item_id).map(|i| i.count).unwrap_or(1);
        let ok = self
            .internal_move_item(acting, from, to, item_id, count, flags)
            .is_ok();
        Ok(ok)
    }

    /// Lua `item:remove` — `luascript.cpp` `luaItemRemove`.
    pub fn lua_script_item_remove(&mut self, item_u64: u64, count: i32) -> Result<bool, String> {
        let item_id = self
            .resolve_item_u64(item_u64)
            .ok_or_else(|| "item not found".to_string())?;
        let parent = self
            .resolve_item_parent_cylinder(item_id)
            .ok_or_else(|| "item has no parent".to_string())?;
        let item_count = i32::from(self.items.get(item_id).map(|i| i.count).unwrap_or(1));
        let remove_count = if count < 0 {
            item_count
        } else {
            count.min(item_count)
        };
        if remove_count <= 0 {
            return Ok(true);
        }
        let rv = match parent {
            Cylinder::Tile { pos } => {
                self.internal_remove_item_from_tile(pos, item_id, remove_count as u16)
            }
            Cylinder::Inventory { player_id, slot } => {
                let taken =
                    self.remove_from_inventory_slot(player_id, slot, item_id, remove_count as u32);
                if taken > 0 {
                    Ok(())
                } else {
                    Err(ReturnValue::NotPossible)
                }
            }
            Cylinder::Container {
                item_id: container_id,
                ..
            } => self.container_remove_thing(container_id, item_id, remove_count as u32),
        };
        Ok(rv.is_ok())
    }

    /// Lua `container:addItem` — `luascript.cpp` `luaContainerAddItem`.
    pub fn lua_script_container_add_item(
        &mut self,
        container_u64: u64,
        item_type: u16,
        count: u32,
        index: i32,
        flags_bits: u32,
    ) -> Result<Option<u64>, String> {
        let container_id = self
            .resolve_item_u64(container_u64)
            .ok_or_else(|| "container not found".to_string())?;
        let stackable = self
            .items_db
            .items
            .get(&item_type)
            .map(|t| t.stackable())
            .unwrap_or(false);
        let stack_count = if stackable {
            count.clamp(1, 100) as u16
        } else {
            count.clamp(1, u32::from(u16::MAX)) as u16
        };
        let new_item = Item::new(item_type, stack_count);
        let iid = self.items.insert(new_item);
        let flags = CylinderFlags { bits: flags_bits };
        let rv = self.container_query_add(
            container_id,
            index,
            iid,
            u32::from(stack_count),
            flags,
            None,
        );
        if rv.is_error() {
            self.items.remove(iid);
            return Ok(None);
        }
        if self.container_add_thing(container_id, index, iid).is_err() {
            self.items.remove(iid);
            return Ok(None);
        }
        Ok(Some(iid.data().as_ffi()))
    }

    /// Lua `Player:addItem` full — `luascript.cpp` `luaPlayerAddItem`.
    pub fn lua_script_player_add_item_full(
        &mut self,
        creature_u64: u64,
        item_type: u16,
        count: u32,
        sub_type: i32,
        can_drop_on_map: bool,
        slot: u8,
    ) -> Result<Option<u64>, String> {
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "creature not found".to_string())?;
        let stackable = self
            .items_db
            .items
            .get(&item_type)
            .map(|t| t.stackable())
            .unwrap_or(false);
        let stack_count = if stackable {
            count.clamp(1, 100) as u16
        } else {
            count.clamp(1, u32::from(u16::MAX)) as u16
        };
        let mut new_item = Item::new(item_type, stack_count);
        if sub_type > 0 && !stackable {
            new_item.count = sub_type as u16;
        }
        let iid = self.items.insert(new_item);
        self.hydrate_player_equipment_containers(cid);

        let target_slot = if slot == 0 {
            InventorySlot::Wherever as u8
        } else {
            slot
        };

        if target_slot != InventorySlot::Wherever as u8 {
            let rv = self.player_query_add(
                cid,
                target_slot,
                iid,
                u32::from(stack_count),
                CylinderFlags::NONE,
            );
            if rv == ReturnValue::NoError
                && self
                    .internal_add_item_to_inventory_slot(cid, target_slot, iid)
                    .is_ok()
            {
                self.notify_player_inventory_slot_add(
                    cid,
                    target_slot,
                    iid,
                    NotificationParent::None,
                );
                return Ok(Some(iid.data().as_ffi()));
            }
        }

        if self.query_add_item_to_inventory(cid, iid) == ReturnValue::NoError {
            let backpack = match self.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => p.equipment_slots[2],
                _ => None,
            };
            if let Some(bp) = backpack {
                let mut registry = std::mem::take(&mut self.container_registry);
                self.ensure_container_registered_simple(&mut registry, bp, cid);
                self.container_registry = registry;
                // C++ `Container::addThing` does `push_front` (`container.cpp:524`) and calls
                // `onAddContainerItem` which sends `sendAddContainerItem` to all viewers
                // (`container.cpp:528-530`). `container_add_thing` mirrors both: it inserts at
                // index 0 and calls `notify_container_content_changed` which enqueues the
                // `0x70` addContainerItem packet to every player with the container open, plus
                // `notify_container_owner_carry_weight` → `player_post_add_notification` which
                // recomputes weight and sends `send_player_stats`. The previous code called
                // `Container::add_item` (push to end) + `refresh_container_chain` (weights only)
                // — no client notification, wrong slot order.
                if self.container_add_thing(bp, 0, iid).is_ok() {
                    return Ok(Some(iid.data().as_ffi()));
                }
            }
        }

        if can_drop_on_map {
            let pos = self
                .creatures
                .get(cid)
                .map(|k| k.position())
                .unwrap_or_default();
            if self
                .internal_add_item_to_tile(pos, iid, CylinderFlags::NO_LIMIT)
                .is_ok()
            {
                return Ok(Some(iid.data().as_ffi()));
            }
        }

        self.items.remove(iid);
        Ok(None)
    }

    pub fn lua_script_set_action_id(
        &mut self,
        item_u64: u64,
        action_id: u16,
    ) -> Result<(), String> {
        let iid = self
            .resolve_item_u64(item_u64)
            .ok_or_else(|| "item not found".to_string())?;
        if let Some(item) = self.items.get_mut(iid) {
            item.set_action_id(action_id);
            Ok(())
        } else {
            Err("item not found".into())
        }
    }

    pub fn lua_script_set_unique_id(
        &mut self,
        item_u64: u64,
        unique_id: u16,
    ) -> Result<(), String> {
        let iid = self
            .resolve_item_u64(item_u64)
            .ok_or_else(|| "item not found".to_string())?;
        if let Some(item) = self.items.get_mut(iid) {
            item.set_unique_id(unique_id);
            Ok(())
        } else {
            Err("item not found".into())
        }
    }

    pub fn lua_script_set_store_item(&mut self, item_u64: u64, store: bool) -> Result<(), String> {
        let iid = self
            .resolve_item_u64(item_u64)
            .ok_or_else(|| "item not found".to_string())?;
        if let Some(item) = self.items.get_mut(iid) {
            item.attributes
                .get_or_insert_with(|| Box::new(crate::item_attributes::ItemAttributes::new()))
                .set_store_item(if store { 1 } else { 0 });
            Ok(())
        } else {
            Err("item not found".into())
        }
    }

    /// 772 `player:feed(amount)` — refill `food_remaining` (`moveuse.cc:1846`
    /// `SetTimer(SKILL_FED, CurFoodTime + ObjFoodTime, ...)`).
    ///
    /// Capped at `MAX_FOOD` (1200) matching the `food.lua` "full" threshold.
    /// Also sets `food_level` to the regen interval — the decompile's `SKILL_FED`
    /// `Act` field controls the `ProcessCreatures` item-regen cadence
    /// (`crmain.cc:1087`). Eating sets `food_level = 12` (the decompile default
    /// `SecsPerHP` for `PROFESSION_NONE`, `crskill.cc:828-835`).
    pub fn lua_script_player_feed(&mut self, creature_u64: u64, amount: u32) -> Result<(), String> {
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "creature not found".to_string())?;
        if let Some(crate::creature::CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.food_remaining = (p.food_remaining.saturating_add(amount)).min(MAX_FOOD);
            // Eating sets the regen interval — decompile `SKILL_FED` `Act` is the
            // `ProcessCreatures` `RegenInterval` (`crmain.cc:1087`). We use 12
            // (the `PROFESSION_NONE`/Knight default `SecsPerHP`, `crskill.cc:833`).
            if p.food_level <= 0 {
                p.food_level = 12;
            }
            Ok(())
        } else {
            Err("not a player".into())
        }
    }

    /// Lua `player:addMana(manaChange)` — `luascript.cpp` `luaPlayerAddMana`
    /// with `animationOnLoss=false` → `Player::changeMana`.
    /// PC-3a Phase 5: `conjureItem` dual-hand mana deduction.
    pub fn lua_script_player_add_mana(
        &mut self,
        creature_u64: u64,
        mana_change: i32,
    ) -> Result<(), String> {
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "creature not found".to_string())?;
        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) else {
            return Err("not a player".into());
        };
        let new_mana = (p.mana + mana_change).clamp(0, p.max_mana);
        p.mana = new_mana;
        self.send_player_stats(cid);
        Ok(())
    }

    /// Lua `player:addManaSpent(amount)` — `luascript.cpp` `luaPlayerAddManaSpent`.
    /// PC-3a Phase 5: advances magic level via `Player::magic_increase`.
    pub fn lua_script_player_add_mana_spent(
        &mut self,
        creature_u64: u64,
        amount: u64,
    ) -> Result<(), String> {
        use crate::config::ConfigManager;

        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "creature not found".to_string())?;
        let magic_tries =
            ConfigManager::scale_tries(amount, self.config.rate_magic().unwrap_or(1.0));
        let profile = self.mechanics.profile;
        let (levels_gained, new_maglevel) = {
            let hooks = &self.mechanics.hooks;
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                let gained = p.magic_increase(magic_tries, &profile, hooks);
                (gained, p.skills.maglevel)
            } else {
                return Err("not a player".into());
            }
        };
        self.notify_magic_tries_gained(cid, levels_gained, new_maglevel);
        Ok(())
    }

    /// Lua `item:transform(itemId[, count/subType])` — pragmatic subset of
    /// `Game::transformItem` (`game.cpp:1612`). PC-3a Phase 5: in-place type
    /// + subtype change with inventory/container/tile notify.
    pub fn lua_script_item_transform(
        &mut self,
        item_u64: u64,
        new_type: u16,
        sub_type: i32,
    ) -> Result<bool, String> {
        let item_id = self
            .resolve_item_u64(item_u64)
            .ok_or_else(|| "item not found".to_string())?;
        let old_type = self
            .items
            .get(item_id)
            .map(|i| i.item_type)
            .ok_or_else(|| "item not found".to_string())?;
        let old_count = self.items.get(item_id).map(|i| i.count).unwrap_or(1);
        if old_type == new_type && (sub_type < 0 || sub_type as u16 == old_count) {
            return Ok(true);
        }
        if self.items_db.items.get(&new_type).is_none() {
            return Ok(true);
        }
        let stackable = self
            .items_db
            .items
            .get(&new_type)
            .map(|t| t.stackable())
            .unwrap_or(false);
        let new_count = if sub_type < 0 {
            old_count
        } else if stackable {
            (sub_type as u16).min(100).max(1)
        } else {
            sub_type as u16
        };
        if let Some(item) = self.items.get_mut(item_id) {
            item.count = new_count.max(1);
        }
        if old_type != new_type {
            self.change_item_type(item_id, new_type);
        } else if let Some(parent) = self.resolve_item_parent_cylinder(item_id) {
            match parent {
                Cylinder::Inventory { player_id, slot } => {
                    self.broadcast_player_inventory_slot(player_id, slot, Some(item_id));
                }
                Cylinder::Container {
                    item_id: container_id,
                    ..
                } => {
                    if let Some(slot) =
                        self.get_thing_index_in_container(container_id, item_id)
                    {
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
        Ok(true)
    }

    /// TFS `Player::removeItemOfType` — `player.cpp` ~2998–3047.
    pub fn player_remove_item_of_type(
        &mut self,
        cid: CreatureId,
        item_id: u16,
        amount: u32,
        sub_type: i32,
        ignore_equipped: bool,
    ) -> bool {
        if amount == 0 {
            return true;
        }
        self.hydrate_player_equipment_containers(cid);
        let entries = self.collect_items_of_type(cid, item_id, sub_type, ignore_equipped);
        let total = self.sum_collected_item_counts(&entries, item_id, sub_type);
        if total < amount {
            return false;
        }
        let stackable = self
            .items_db
            .items
            .get(&item_id)
            .map(|t| t.stackable())
            .unwrap_or(false);
        self.internal_remove_items(cid, entries, amount, stackable)
    }

    /// TFS `Game::internalRemoveItems` — `game.cpp` ~5785–5801.
    pub(crate) fn internal_remove_items(
        &mut self,
        cid: CreatureId,
        entries: Vec<InventoryItemRef>,
        mut amount: u32,
        stackable: bool,
    ) -> bool {
        if amount == 0 {
            return true;
        }
        let mut removed_any = false;
        if stackable {
            for entry in entries {
                if amount == 0 {
                    break;
                }
                let item_count = self
                    .items
                    .get(entry.item_id)
                    .map(|i| u32::from(i.count.max(1)))
                    .unwrap_or(0);
                if item_count > amount {
                    let taken = self.remove_inventory_item_count(cid, entry, amount);
                    if taken > 0 {
                        amount = 0;
                        removed_any = true;
                    }
                    break;
                }
                let taken = self.remove_inventory_item_count(cid, entry, item_count);
                if taken > 0 {
                    amount = amount.saturating_sub(taken);
                    removed_any = true;
                }
            }
        } else {
            for entry in entries {
                if amount == 0 {
                    break;
                }
                let item_count = self
                    .items
                    .get(entry.item_id)
                    .map(|i| u32::from(i.count.max(1)))
                    .unwrap_or(0);
                let take = item_count.min(amount);
                let taken = self.remove_inventory_item_count(cid, entry, take);
                if taken > 0 {
                    amount = amount.saturating_sub(taken);
                    removed_any = true;
                }
            }
        }
        if removed_any {
            self.recompute_player_inventory_weight(cid);
            self.update_player_items_light(cid, false);
            self.send_player_stats(cid);
        }
        amount == 0
    }

    fn remove_inventory_item_count(
        &mut self,
        cid: CreatureId,
        entry: InventoryItemRef,
        count: u32,
    ) -> u32 {
        if count == 0 {
            return 0;
        }
        match entry.cylinder {
            ItemCylinder::Inventory { slot } => {
                self.remove_from_inventory_slot(cid, slot, entry.item_id, count)
            }
            ItemCylinder::Container { parent_container } => {
                if self
                    .container_remove_thing(parent_container, entry.item_id, count)
                    .is_ok()
                {
                    if let Some(slot) = self.equipment_slot_holding_container(cid, parent_container)
                    {
                        if let Some(root_id) = self.get_player_inventory_item(cid, slot) {
                            self.notify_player_container_tree_changed(
                                cid,
                                root_id,
                                entry.item_id,
                                false,
                                NotificationParent::None,
                            );
                        }
                    }
                    count
                } else {
                    0
                }
            }
        }
    }

    fn remove_from_inventory_slot(
        &mut self,
        cid: CreatureId,
        slot: u8,
        item_id: ItemId,
        count: u32,
    ) -> u32 {
        let item_count = self
            .items
            .get(item_id)
            .map(|i| u32::from(i.count.max(1)))
            .unwrap_or(0);
        let stackable = self
            .items
            .get(item_id)
            .and_then(|i| self.items_db.items.get(&i.item_type))
            .map(|t| t.stackable())
            .unwrap_or(false);
        if stackable && count < item_count {
            if let Some(item) = self.items.get_mut(item_id) {
                item.count = item.count.saturating_sub(count as u16);
            }
            self.broadcast_player_inventory_slot(cid, slot, Some(item_id));
            count
        } else {
            let removed = item_count.min(count);
            let _ =
                self.unequip_item_from_inventory_slot(cid, slot, item_id, NotificationParent::None);
            self.cancel_item_decay(item_id);
            self.items.remove(item_id);
            removed
        }
    }

    pub(crate) fn broadcast_player_inventory_slot(
        &mut self,
        cid: CreatureId,
        slot: u8,
        item_id: Option<ItemId>,
    ) {
        let Some(conn) = self.conn_id_for_creature(cid) else {
            return;
        };
        let with_desc = self
            .creatures
            .get(cid)
            .and_then(|k| match k {
                CreatureKind::Player(p) => Some(p.item_with_description()),
                _ => None,
            })
            .unwrap_or(false);

        if let Some(iid) = item_id {
            let Some(item) = self.items.get(iid) else {
                self.enqueue_outgoing(conn, send_inventory_slot_empty(slot).into_bytes());
                return;
            };
            let sid = item.item_type;
            let cid_client = self.items_db.client_id_for_server(sid);
            if cid_client == 0 {
                self.enqueue_outgoing(conn, send_inventory_slot_empty(slot).into_bytes());
                return;
            }
            let cnt = item.client_count().max(1);
            let stackable = self.items_db.stackable_for_server(sid);
            let splash = self.items_db.is_splash_or_fluid_for_server(sid);
            let anim = self.items_db.is_animation_for_server(sid);
            self.enqueue_encoded(
                conn,
                self.codec.encode_inventory_item(
                    slot,
                    ItemTemplateArgs {
                        client_id: cid_client,
                        count: cnt,
                        stackable,
                        is_splash_or_fluid: splash && !stackable,
                        is_animation: anim,
                        with_description: with_desc,
                    },
                ),
            );
        } else {
            self.enqueue_outgoing(conn, send_inventory_slot_empty(slot).into_bytes());
        }
        self.send_player_stats(cid);
    }

    /// `Game::playerEquipItem` — `game.cpp` ~1851–1877.
    pub fn player_quick_equip(&mut self, conn_id: ConnId, cid: CreatureId, sprite_id: u16) {
        let Some(server_id) = self.items_db.server_id_for_client(sprite_id) else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };
        let Some(it) = self.items_db.items.get(&server_id) else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };
        let target_slot = slot_type_for_item_type(it);
        let backpack_id = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.equipment_slots[2],
            _ => None,
        };
        let Some(backpack_id) = backpack_id else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };
        let bp_type = self
            .items
            .get(backpack_id)
            .map(|i| i.item_type)
            .unwrap_or(0);
        if !self.items_db.is_container(bp_type) {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        }
        let mut registry = std::mem::take(&mut self.container_registry);
        self.ensure_container_registered_simple(&mut registry, backpack_id, cid);
        self.container_registry = registry;

        let Some(found_id) = self.search_item_in_container_by_type(backpack_id, server_id) else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };

        if let Some(equipped_id) = self.get_player_inventory_item(cid, target_slot) {
            if self.items.get(equipped_id).map(|i| i.item_type) == Some(server_id) {
                let n = self.items.get(equipped_id).map(|i| i.count).unwrap_or(1);
                let from = Cylinder::Inventory {
                    player_id: cid,
                    slot: target_slot,
                };
                let to = Cylinder::Container {
                    item_id: backpack_id,
                    index: INDEX_WHEREEVER,
                };
                if self
                    .internal_move_item(Some(cid), from, to, equipped_id, n, CylinderFlags::NONE)
                    .is_err()
                {
                    self.send_cancel_message(conn_id, ReturnValue::NotPossible);
                }
                return;
            }
            self.send_cancel_message(conn_id, ReturnValue::NeedExchange);
            return;
        }

        let n = self.items.get(found_id).map(|i| i.count).unwrap_or(1);
        let from = Cylinder::Container {
            item_id: backpack_id,
            index: INDEX_WHEREEVER,
        };
        let to = Cylinder::Inventory {
            player_id: cid,
            slot: target_slot,
        };
        if self
            .internal_move_item(Some(cid), from, to, found_id, n, CylinderFlags::NONE)
            .is_err()
        {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
        }
    }

    /// C++ `ITEM_STACK_SIZE` — `items.h`.
    const ITEM_STACK_MAX: u16 = 100;

    /// Remaining count that can merge into `merge_id` before hitting the stack cap.
    pub(crate) fn stack_merge_room(&self, merge_id: ItemId) -> u16 {
        Self::ITEM_STACK_MAX.saturating_sub(self.items.get(merge_id).map(|i| i.count).unwrap_or(0))
    }

    /// TFS stack-merge room check — `Game::internalMoveItem` merge paths (`game.cpp`).
    pub(crate) fn ensure_stack_merge_room(
        &self,
        merge_id: ItemId,
        m_move: u16,
        not_enough: ReturnValue,
    ) -> Result<(), ReturnValue> {
        if self.stack_merge_room(merge_id) < m_move {
            return Err(not_enough);
        }
        Ok(())
    }

    /// Partial stack merge — source item **still on its cylinder** (tile, hand slot, container).
    ///
    /// Subtracts `m_move` from `source_id` and adds it to `merge_id`. Call this when the
    /// source stack remains in place (e.g. tile `broadcast_tile_item_update`, hand slot still
    /// holds the item). Do **not** use after `remove_item_by_id`, `internal_remove_item_from_inventory_slot`,
    /// or `container_remove_thing` — use [`Self::merge_detached_stack_counts`] instead.
    pub(crate) fn merge_partial_stack_counts(
        &mut self,
        source_id: ItemId,
        merge_id: ItemId,
        m_move: u16,
    ) {
        if let Some(src) = self.items.get_mut(source_id) {
            src.count = src.count.saturating_sub(m_move);
        }
        if let Some(t) = self.items.get_mut(merge_id) {
            t.count = t.count.saturating_add(m_move);
        }
    }

    /// Full stack merge — source **already detached** from its cylinder.
    ///
    /// Adds `m_move` to `merge_id` only. The caller must have removed the source from tile,
    /// inventory, or container (and optionally `items.remove`) before calling — never pair this
    /// with a prior subtract on the same move.
    pub(crate) fn merge_detached_stack_counts(&mut self, merge_id: ItemId, m_move: u16) {
        if let Some(t) = self.items.get_mut(merge_id) {
            t.count = t.count.saturating_add(m_move);
        }
    }

    /// Hydrate container registry and notify clients that a stack slot changed.
    pub(crate) fn notify_container_stack_merge(&mut self, container_id: ItemId, merge_id: ItemId) {
        self.hydrate_container_if_needed(container_id);
        let merge_slot = self
            .get_thing_index_in_container(container_id, merge_id)
            .map(|i| i as u16)
            .unwrap_or(0);
        self.notify_container_content_changed(
            container_id,
            ContainerContentChange::Update { slot: merge_slot },
        );
    }

    fn search_item_in_container_by_type(
        &self,
        container_item_id: ItemId,
        server_type: u16,
    ) -> Option<ItemId> {
        let c = self.container_registry.get(container_item_id)?;
        for &iid in &c.items {
            if self.items.get(iid).map(|i| i.item_type) == Some(server_type) {
                return Some(iid);
            }
            let child_type = self.items.get(iid).map(|i| i.item_type)?;
            if self.items_db.is_container(child_type) {
                if let Some(found) = self.search_item_in_container_by_type(iid, server_type) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// `Game::playerLookAt` — `game.cpp` ~3156–3187.
    pub fn player_look_at(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        pos: Position,
        stack_pos: u8,
    ) {
        let Some(target) = self.internal_get_thing_look(cid, pos, stack_pos) else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };

        let player_pos = self.creatures.get(cid).map(|k| k.position()).unwrap_or(pos);
        // C++ `thing->getPosition()` for map targets; inventory/container use player tile.
        let thing_pos = if pos.x == 0xFFFF { player_pos } else { pos };
        // C++ `player->canSee(thingPos)` — `ProtocolGame::canSee` (`protocolgame.cpp` ~796).
        if !self.can_see_position(cid, thing_pos) {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        }
        let look_d = look_distance_tfs(player_pos, thing_pos);

        let msg = match target {
            LookTarget::Creature(target_cid) => self.player_look_description(cid, target_cid),
            LookTarget::Ground(ground_type) => {
                let ephemeral = Item::new_single(ground_type);
                if let Some(it) = self.items_db.items.get(&ground_type) {
                    let rune_vocs = self
                        .spells
                        .get_rune(ground_type)
                        .map(|r| r.vocations.as_slice());
                    format!(
                        "You see {}",
                        item_get_description_cpp(
                            &ephemeral,
                            it,
                            it.weight,
                            look_d,
                            None,
                            None,
                            rune_vocs,
                        )
                    )
                } else {
                    format!("You see an item of type {ground_type}.")
                }
            }
            LookTarget::Item(item_id) => {
                self.hydrate_container_if_needed(item_id);
                let w = self.item_recursive_weight_oz(item_id);
                let show_duration_ms = self.item_look_duration_ms(item_id);
                let Some(item) = self.items.get(item_id) else {
                    self.send_cancel_message(conn_id, ReturnValue::NotPossible);
                    return;
                };
                let container_capacity = self.container_registry.get(item_id).map(|c| c.capacity);
                let rune_vocs = self
                    .spells
                    .get_rune(item.item_type)
                    .map(|r| r.vocations.clone());
                if let Some(it) = self.items_db.items.get(&item.item_type) {
                    format!(
                        "You see {}",
                        item_get_description_cpp(
                            item,
                            it,
                            w,
                            look_d,
                            container_capacity,
                            show_duration_ms,
                            rune_vocs.as_deref(),
                        )
                    )
                } else {
                    format!("You see an item of type {}.", item.item_type)
                }
            }
        };
        self.enqueue_outgoing(conn_id, send_text_message_simple(22, &msg).into_bytes());
    }

    /// `Player::getDescription` for the creature-look branch of `playerLookAt`.
    ///
    /// C++ reference (772 outcomes): `reference/cipsoft-772/tibia-game-master/src/operate.cc:1849-1916`
    /// — builds `"You see %s. %s."` with `Name`/`Vocation` split by self-vs-other and sex pronoun.
    /// TFS 1.4.2 `src/player.cpp:91-125` `Player::getDescription` confirms the same shape; the
    /// `default_onLook.lua` callback prepends `"You see "` to the returned description, yielding
    /// identical wire text for both eras in the no-guild case.
    ///
    /// - Self:  `You see yourself. You are a sorcerer.` (or `You have no vocation.`)
    /// - Other: `You see <Name> (Level <N>). He is a sorcerer.` (or `She` / `has no vocation`)
    ///
    /// Non-player creatures (monsters/npcs) fall back to `You see <Name>.` — matches the
    /// decompile `else` branch at `operate.cc:1917-1918` (`"You see %s.", Target->Name`).
    fn player_look_description(&self, viewer_cid: CreatureId, target_cid: CreatureId) -> String {
        let Some(target) = self.creatures.get(target_cid) else {
            return "You see nothing special.".to_string();
        };
        match target {
            CreatureKind::Player(p) => {
                // C++ `lookDistance == -1` ⇔ `thing == player` (`game.cpp:3178-3184`).
                let looking_at_self = target_cid == viewer_cid;
                // `vocation->getVocDescription()` (`vocation.h:18-20`); id 0 ⇒ "no vocation".
                let voc_desc = self
                    .vocations
                    .get(p.vocation_id)
                    .filter(|v| v.id != 0)
                    .map(|v| v.description.as_str());
                if looking_at_self {
                    let vocation_clause = match voc_desc {
                        Some(d) => format!("You are {d}."),
                        None => "You have no vocation.".to_string(),
                    };
                    format!("You see yourself. {vocation_clause}")
                } else {
                    // C++ pronoun: `PLAYERSEX_FEMALE` ⇒ "She", else "He" (`player.cpp:112-116`).
                    let pronoun = if p.sex == crate::creature::PlayerSex::Female {
                        "She"
                    } else {
                        "He"
                    };
                    let vocation_clause = match voc_desc {
                        Some(d) => format!("{pronoun} is {d}."),
                        None => format!("{pronoun} has no vocation."),
                    };
                    format!(
                        "You see {} (Level {}). {vocation_clause}",
                        p.base.name, p.level
                    )
                }
            }
            // Monster: TFS `Monster::getDescription` returns `nameDescription + '.'`
            // (`monster.h:62-64`); `nameDescription` defaults to `"a " + lowercase(name)`
            // (`monsters.cpp:860`). The 772 decompile uses `Target->Name` directly
            // (`operate.cc:1918`) which is already lowercase in `.mon` files — both
            // produce lowercase output. We look up `name_description` from `MonsterType`
            // for the article + lowercase form, falling back to lowercased `base.name`.
            CreatureKind::Monster(m) => {
                let desc = self
                    .monsters_db
                    .monsters
                    .get(&m.base.name.to_lowercase())
                    .map(|t| t.name_description.as_str())
                    .filter(|d| !d.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| m.base.name.to_lowercase());
                format!("You see {desc}.")
            }
            // Npc: TFS `Npc::getDescription` returns `name + '.'` (`npc.cpp:365-371`);
            // 772 decompile uses `Target->Name` (`operate.cc:1918`). Both use the name
            // as-is (NPC names are proper nouns — "Leeland", not "leeland").
            CreatureKind::Npc(_) => format!("You see {}.", target.base().name),
        }
    }
}

fn dest_player_creature(dest: &tfs_rust_lua::LuaMoveDestination) -> Option<u64> {
    match dest {
        tfs_rust_lua::LuaMoveDestination::Player { creature_id } => Some(*creature_id),
        _ => None,
    }
}

#[cfg(test)]
mod look_tests {
    use super::*;
    use crate::sim_harness::{insert_player, minimal_world, test_player};
    use std::sync::Arc;
    use tfs_rust_common::Position;
    use tfs_rust_content::vocations::{VocationDef, VocationFormula, VocationRegistry};

    /// `data/defs/vocations.lua` sorcerer (id=1) — only the fields `player_look_description`
    /// reads (`id`, `description`) need to be accurate; the rest are zeros for the test.
    fn sorcerer_vocation_db() -> Arc<VocationRegistry> {
        let mut vocations = std::collections::HashMap::new();
        vocations.insert(
            1u16,
            VocationDef {
                id: 1,
                client_id: 3,
                name: "Sorcerer".into(),
                description: "a sorcerer".into(),
                from_vocation: 1,
                gain_cap: 0,
                gain_hp: 0,
                gain_mana: 0,
                gain_hp_ticks: 0,
                gain_hp_amount: 0,
                gain_mana_ticks: 0,
                gain_mana_amount: 0,
                mana_multiplier: 0.0,
                attack_speed_ms: 0,
                base_speed: 0,
                soul_max: 0,
                gain_soul_ticks: 0,
                allow_pvp: false,
                base_hp: 0,
                base_mana: 0,
                base_cap: 0,
                formula: VocationFormula::default(),
                skill_multipliers: [0.0; 7],
            },
        );
        Arc::new(VocationRegistry { vocations })
    }

    /// 772 decompile `operate.cc:1863-1872` — self look yields
    /// `"You see yourself. You are a sorcerer."`.
    #[test]
    fn self_look_includes_vocation() {
        let mut world = minimal_world();
        world.vocations = sorcerer_vocation_db();
        let pos = Position::new(100, 100, 7);
        let mut viewer = test_player("Sorcerer Sample", pos);
        viewer.vocation_id = 1;
        viewer.level = 100;
        let viewer_cid = insert_player(&mut world, viewer);
        let msg = world.player_look_description(viewer_cid, viewer_cid);
        assert_eq!(msg, "You see yourself. You are a sorcerer.");
    }

    /// Self look with vocation 0 ⇒ `"You have no vocation."` (`player.cpp:103`).
    #[test]
    fn self_look_no_vocation() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let viewer_cid = insert_player(&mut world, test_player("Nobody", pos));
        let msg = world.player_look_description(viewer_cid, viewer_cid);
        assert_eq!(msg, "You see yourself. You have no vocation.");
    }

    /// 772 decompile `operate.cc:1873-1886` — other-player look yields
    /// `"You see <Name> (Level <N>). He is a sorcerer."` for a male target.
    #[test]
    fn other_player_look_male() {
        let mut world = minimal_world();
        world.vocations = sorcerer_vocation_db();
        let pos = Position::new(100, 100, 7);
        let viewer_cid = insert_player(&mut world, test_player("Viewer", pos));
        let mut target = test_player("Sorcerer Sample", pos);
        target.vocation_id = 1;
        target.level = 100;
        // `test_player` defaults to `PlayerSex::Male` (sim_harness).
        let target_cid = insert_player(&mut world, target);
        let msg = world.player_look_description(viewer_cid, target_cid);
        assert_eq!(
            msg,
            "You see Sorcerer Sample (Level 100). He is a sorcerer."
        );
    }

    /// `PLAYERSEX_FEMALE` ⇒ "She" pronoun (`player.cpp:112-113`).
    #[test]
    fn other_player_look_female() {
        let mut world = minimal_world();
        world.vocations = sorcerer_vocation_db();
        let pos = Position::new(100, 100, 7);
        let viewer_cid = insert_player(&mut world, test_player("Viewer", pos));
        let mut target = test_player("Sorceress Sample", pos);
        target.vocation_id = 1;
        target.level = 100;
        target.sex = crate::creature::PlayerSex::Female;
        let target_cid = insert_player(&mut world, target);
        let msg = world.player_look_description(viewer_cid, target_cid);
        assert_eq!(
            msg,
            "You see Sorceress Sample (Level 100). She is a sorcerer."
        );
    }

    /// Other player with no vocation ⇒ `"<Pronoun> has no vocation."` (`player.cpp:123`).
    #[test]
    fn other_player_look_no_vocation() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let viewer_cid = insert_player(&mut world, test_player("Viewer", pos));
        let target_cid = insert_player(&mut world, test_player("RookSample", pos));
        let msg = world.player_look_description(viewer_cid, target_cid);
        assert_eq!(msg, "You see RookSample (Level 8). He has no vocation.");
    }

    /// Helper: build a minimal `CreatureBase` for monster/npc look tests.
    /// Reuses `test_player`'s `base` (fully constructed) and overrides the name.
    fn creature_base_named(name: &str, pos: Position) -> crate::CreatureBase {
        let mut base = test_player("x", pos).base;
        base.name = name.into();
        base
    }

    /// Monster look uses `MonsterType::name_description` (TFS `Monster::getDescription`
    /// `monster.h:62-64`; default `"a " + lowercase(name)` `monsters.cpp:860`).
    /// `base.name` is "Dog" (capitalized from TFS XML) but `name_description` is "a dog".
    #[test]
    fn monster_look_uses_name_description() {
        use tfs_rust_content::monsters::{
            MonsterDatabase, MonsterDefenses, MonsterOutfit, MonsterType, MonsterTypeFlags,
        };

        let mut world = minimal_world();
        // Register "Dog" in the monster DB with `name_description = "a dog"`.
        let mut monsters = std::collections::HashMap::new();
        monsters.insert(
            "dog".into(),
            MonsterType {
                name: "Dog".into(),
                filename: "dog.xml".into(),
                name_description: "a dog".into(),
                race: "blood".into(),
                experience: 0,
                speed: 22,
                health_now: 20,
                health_max: 20,
                outfit: MonsterOutfit::default(),
                flags: MonsterTypeFlags::default(),
                mana_cost: 0,
                loot: Vec::new(),
                attack_spells: Vec::new(),
                defenses: MonsterDefenses {
                    armor: None,
                    defense: None,
                    spells: Vec::new(),
                    immunity_poison: false,
                    immunity_fire: false,
                    immunity_energy: false,
                    immunity_life_drain: false,
                    see_invisible: false,
                    immunity_physical: false,
                },
                max_summons: 0,
                summons: Vec::new(),
                talk_texts: Vec::new(),
            },
        );
        world.monsters_db = Arc::new(MonsterDatabase { monsters });

        // Insert a monster with `base.name = "Dog"` (as `spawn_monster` would set).
        let pos = Position::new(100, 100, 7);
        let monster_cid =
            world
                .creatures
                .insert(CreatureKind::Monster(crate::creature::Monster::new(
                    creature_base_named("Dog", pos),
                    pos,
                )));

        let viewer_cid = insert_player(&mut world, test_player("Viewer", pos));
        let msg = world.player_look_description(viewer_cid, monster_cid);
        assert_eq!(msg, "You see a dog.");
    }

    /// Monster not in `monsters_db` falls back to lowercased `base.name`
    /// (matches 772 decompile `Target->Name` which is lowercase in `.mon` files).
    #[test]
    fn monster_look_falls_back_to_lowercase_name() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let monster_cid =
            world
                .creatures
                .insert(CreatureKind::Monster(crate::creature::Monster::new(
                    creature_base_named("Cyclops", pos),
                    pos,
                )));
        let viewer_cid = insert_player(&mut world, test_player("Viewer", pos));
        let msg = world.player_look_description(viewer_cid, monster_cid);
        assert_eq!(msg, "You see cyclops.");
    }

    /// NPC look uses `base.name` as-is (proper noun) — TFS `Npc::getDescription`
    /// (`npc.cpp:365-371`); 772 decompile `operate.cc:1918`.
    #[test]
    fn npc_look_uses_name_as_is() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let npc_cid = world
            .creatures
            .insert(CreatureKind::Npc(crate::creature::Npc::placeholder(
                creature_base_named("Leeland", pos),
            )));
        let viewer_cid = insert_player(&mut world, test_player("Viewer", pos));
        let msg = world.player_look_description(viewer_cid, npc_cid);
        assert_eq!(msg, "You see Leeland.");
    }
}
