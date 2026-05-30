//! `Player::postAddNotification` / `postRemoveNotification` side effects.
// C++ reference: `src/player.cpp` ~3076–3191, `src/container.cpp` ~697–725.

use tfs_rust_common::Position;
use tfs_rust_net::outgoing_extra::send_creature_light;

use crate::creature::LightInfo;
use crate::creature::CreatureKind;
use crate::cylinder::CylinderLink;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::lua_scope::fire_on_player_inventory_update;

/// Parent cylinder hint for `requireListUpdate` / shop refresh — `player.cpp` postAdd/postRemove.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NotificationParent {
    Player,
    Container(ItemId),
    Tile(Position),
    None,
}

impl GameWorld {
    /// Equipment slot (1–11) carrying `container_root`, if any.
    pub(crate) fn equipment_slot_holding_container(
        &self,
        player: CreatureId,
        container_root: ItemId,
    ) -> Option<u8> {
        let Some(CreatureKind::Player(p)) = self.creatures.get(player) else {
            return None;
        };
        for (idx, slot_item) in p.equipment_slots.iter().enumerate() {
            let Some(root) = slot_item else {
                continue;
            };
            if *root == container_root {
                return Some((idx + 1) as u8);
            }
            if let Some(c) = self.container_registry.get(*root) {
                if c.is_holding_item(&self.container_registry, container_root) {
                    return Some((idx + 1) as u8);
                }
            }
        }
        None
    }

    /// C++ `Item::getLightInfo` — `item.cpp` ~1707.
    pub(crate) fn item_light_info(&self, server_type: u16) -> LightInfo {
        self.items_db
            .items
            .get(&server_type)
            .map(|t| LightInfo {
                level: t.light_level,
                color: t.light_color,
            })
            .unwrap_or_default()
    }

    /// TFS `Player::updateItemsLight` — `player.cpp` ~3411.
    pub(crate) fn update_player_items_light(&mut self, cid: CreatureId, internal: bool) {
        let Some(CreatureKind::Player(_)) = self.creatures.get(cid) else {
            return;
        };
        let mut max_light = LightInfo::default();
        for slot in 1u8..=11 {
            let Some(iid) = self.get_player_inventory_item(cid, slot) else {
                continue;
            };
            let Some(item) = self.items.get(iid) else {
                continue;
            };
            let cur = self.item_light_info(item.item_type);
            if cur.level > max_light.level {
                max_light = cur;
            }
        }
        let changed = {
            let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) else {
                return;
            };
            let prev = p.items_light;
            p.items_light = max_light;
            prev != max_light
        };
        if changed && !internal {
            self.change_creature_light(cid);
        }
    }

    /// C++ `Player::getCreatureLight` — internal condition light not ported; items only for now.
    pub(crate) fn player_creature_light(&self, cid: CreatureId) -> LightInfo {
        self.creatures
            .get(cid)
            .and_then(|k| match k {
                CreatureKind::Player(p) => Some(p.items_light),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// TFS `Game::changeLight` — `game.cpp` ~3911.
    pub(crate) fn change_creature_light(&mut self, cid: CreatureId) {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };
        let pos = p.base.position;
        let pid = p.guid;
        let light = self.player_creature_light(cid);
        let access_player = false;
        let pkt = send_creature_light(pid, light.level, light.color, access_player).into_bytes();
        self.broadcast_to_spectators(pos, pkt);
    }

    fn notification_require_list_update(
        &self,
        player: CreatureId,
        parent: NotificationParent,
        is_add: bool,
    ) -> bool {
        match parent {
            NotificationParent::Container(container_id) => {
                let top = self.top_container_item_id(container_id);
                !self.player_holds_container_tree(player, top)
            }
            NotificationParent::Player => false,
            NotificationParent::Tile(_) | NotificationParent::None => {
                // C++: oldParent/newParent != this
                is_add
            }
        }
    }

    /// C++ `Player::updateSaleShopList` — stub until NPC shop runtime (`player.cpp` ~3193).
    fn try_update_sale_shop_list(&self, cid: CreatureId, item_id: ItemId) {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };
        if p.shop_owner.is_some() {
            tracing::debug!(
                ?cid,
                ?item_id,
                "updateSaleShopList deferred until shop runtime"
            );
        }
    }

    /// C++ `Player::onUpdateInventoryItem` — trade guards deferred (`player.cpp` ~1461).
    fn on_update_inventory_item(
        &mut self,
        _cid: CreatureId,
        _slot: u8,
        _old_item: Option<ItemId>,
        _new_item: ItemId,
    ) {
        // Trade `checkTradeState` when trade port lands.
    }

    /// C++ `Player::onRemoveInventoryItem` — trade guards deferred (`player.cpp` ~1472).
    fn on_remove_inventory_item(&mut self, _cid: CreatureId, _item_id: ItemId) {
        // Trade `checkTradeState` when trade port lands.
    }

    fn clear_inventory_ability_on_deequip(&mut self, cid: CreatureId, slot: u8) {
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.set_item_ability(slot, false);
        }
    }

    fn positions_in_range_1(a: Position, b: Position) -> bool {
        if a.z != b.z {
            return false;
        }
        a.x.abs_diff(b.x) <= 1 && a.y.abs_diff(b.y) <= 1
    }

    fn container_item_position(&self, container_item_id: ItemId) -> Option<Position> {
        let top = self.top_container_item_id(container_item_id);
        self.map.find_item_position(top)
    }

    fn post_remove_container_item_notification(&mut self, cid: CreatureId, item_id: ItemId) {
        let player_pos = self
            .creatures
            .get(cid)
            .map(|k| k.position())
            .unwrap_or(Position::new(0, 0, 0));
        let top = self.top_container_item_id(item_id);
        if self.player_holds_container_tree(cid, top) {
            self.refresh_container_ui_for_all_viewers(item_id);
            return;
        }
        if let Some(cpos) = self.container_item_position(item_id) {
            if !Self::positions_in_range_1(player_pos, cpos) {
                self.auto_close_containers_for_container_item(cid, item_id);
                return;
            }
        }
        // Depot owner branch deferred until P4 — auto-close when not held.
        self.auto_close_containers_for_container_item(cid, item_id);
    }

    /// TFS `Player::postAddNotification` — `player.cpp` ~3076.
    pub(crate) fn player_post_add_notification(
        &mut self,
        cid: CreatureId,
        item_id: ItemId,
        slot: u8,
        link: CylinderLink,
        old_parent: NotificationParent,
    ) {
        if link == CylinderLink::Owner {
            self.events.on_player_equip(cid, item_id, slot);
            fire_on_player_inventory_update(self, cid, item_id, slot, true);
            self.on_update_inventory_item(cid, slot, None, item_id);
        }

        if link == CylinderLink::Owner || link == CylinderLink::TopParent {
            let require_list_update = self.notification_require_list_update(cid, old_parent, true);
            self.recompute_player_inventory_weight(cid);
            self.update_player_items_light(cid, false);
            self.send_player_stats(cid);
            if require_list_update {
                self.try_update_sale_shop_list(cid, item_id);
            }
        }

        if self
            .items
            .get(item_id)
            .is_some_and(|i| self.items_db.is_container(i.item_type))
        {
            self.refresh_container_ui_for_all_viewers(item_id);
        }
    }

    /// TFS `Player::postRemoveNotification` — `player.cpp` ~3131.
    pub(crate) fn player_post_remove_notification(
        &mut self,
        cid: CreatureId,
        item_id: ItemId,
        slot: u8,
        link: CylinderLink,
        new_parent: NotificationParent,
    ) {
        if link == CylinderLink::Owner {
            self.events.on_player_deequip(cid, item_id, slot);
            fire_on_player_inventory_update(self, cid, item_id, slot, false);
            self.clear_inventory_ability_on_deequip(cid, slot);
            self.on_remove_inventory_item(cid, item_id);
        }

        if link == CylinderLink::Owner || link == CylinderLink::TopParent {
            let require_list_update = self.notification_require_list_update(cid, new_parent, false);
            self.recompute_player_inventory_weight(cid);
            self.update_player_items_light(cid, false);
            self.send_player_stats(cid);
            if require_list_update {
                self.try_update_sale_shop_list(cid, item_id);
            }
        }

        if self
            .items
            .get(item_id)
            .is_some_and(|i| self.items_db.is_container(i.item_type))
        {
            self.post_remove_container_item_notification(cid, item_id);
        }
    }

    /// Notify player after direct slot equip/add — wraps postAdd + 0x78.
    pub(crate) fn notify_player_inventory_slot_add(
        &mut self,
        cid: CreatureId,
        slot: u8,
        item_id: ItemId,
        old_parent: NotificationParent,
    ) {
        self.player_post_add_notification(cid, item_id, slot, CylinderLink::Owner, old_parent);
        self.broadcast_player_inventory_slot(cid, slot, Some(item_id));
    }

    /// Notify player after direct slot unequip/remove — wraps postRemove + 0x78.
    pub(crate) fn notify_player_inventory_slot_remove(
        &mut self,
        cid: CreatureId,
        slot: u8,
        item_id: ItemId,
        new_parent: NotificationParent,
    ) {
        self.player_post_remove_notification(cid, item_id, slot, CylinderLink::Owner, new_parent);
        self.broadcast_player_inventory_slot(cid, slot, None);
    }

    /// Weight/light/stats when a carried container tree changes (LINK_TOPPARENT).
    pub(crate) fn notify_player_container_tree_changed(
        &mut self,
        cid: CreatureId,
        container_root: ItemId,
        item_id: ItemId,
        is_add: bool,
        parent: NotificationParent,
    ) {
        let slot = self
            .equipment_slot_holding_container(cid, container_root)
            .unwrap_or(0);
        if is_add {
            self.player_post_add_notification(
                cid,
                item_id,
                slot,
                CylinderLink::TopParent,
                parent,
            );
        } else {
            self.player_post_remove_notification(
                cid,
                item_id,
                slot,
                CylinderLink::TopParent,
                parent,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LightInfo;
    use crate::creature::Player;

    #[test]
    fn light_info_max_picks_brighter() {
        let a = LightInfo { level: 3, color: 1 };
        let b = LightInfo { level: 7, color: 215 };
        assert_eq!(LightInfo::max_of(a, b), b);
    }

    #[test]
    fn inventory_abilities_set_and_query() {
        let mut p = test_player_stub();
        assert!(!p.is_item_ability_enabled(5));
        p.set_item_ability(5, true);
        assert!(p.is_item_ability_enabled(5));
        p.set_item_ability(5, false);
        assert!(!p.is_item_ability_enabled(5));
    }

    #[test]
    fn slot_to_array_index_maps_inventory_abilities() {
        let mut p = test_player_stub();
        p.set_item_ability(11, true);
        assert!(p.inventory_abilities[10]);
        assert!(!p.inventory_abilities[0]);
    }

    fn test_player_stub() -> Player {
        use std::collections::HashMap;
        use std::time::Instant;
        use tfs_rust_common::enums::{Direction, SkullType};
        use tfs_rust_common::Position;
        use crate::CreatureBase;
        use crate::creature::{Outfit, PlayerEconomy, PlayerInventory, PlayerSkills, PlayerSocial};

        Player {
            base: CreatureBase {
                name: "t".into(),
                position: Position::new(0, 0, 7),
                direction: Direction::North,
                health: 100,
                max_health: 100,
                outfit: Outfit::default(),
                speed: 220,
                base_speed: 220,
                skull: SkullType::None,
                drunkenness: 0,
                active_conditions: Vec::new(),
                walk_queue: Default::default(),
                last_step: None,
                last_step_cost: 1,
                last_step_ground_speed: 150,
                next_walk_check: None,
                walk_timer: Default::default(),
                cancel_next_walk: false,
                force_update_follow_path: false,
                movement_blocked: false,
                stairhop_blocked_until: None,
                follow_target: None,
                attack_target: None,
                master: None,
                damage_map: Default::default(),
            },
            account_id: 0,
            guid: 1,
            vocation_id: 0,
            level: 1,
            experience: 0,
            mana: 0,
            max_mana: 0,
            capacity: 400,
            inventory: PlayerInventory::default(),
            skills: PlayerSkills {
                fist: 10,
                club: 10,
                sword: 10,
                axe: 10,
                dist: 10,
                shielding: 10,
                fishing: 10,
                maglevel: 0,
            },
            economy: PlayerEconomy { balance: 0, soul: 0 },
            social: PlayerSocial::default(),
            town_id: 0,
            premium_ends_at: 0,
            stamina_minutes: 0,
            offline_training_ms: 0,
            spell_cooldown_end: HashMap::new(),
            spell_group_cooldown_end: HashMap::new(),
            operating_system: 0,
            otclient_v8: 0,
            ghost_mode: false,
            equipment_slots: std::array::from_fn(|_| None),
            inventory_weight: 0,
            items_light: LightInfo::default(),
            inventory_abilities: [false; 11],
            shop_owner: None,
            vip_list: Vec::new(),
            health_hidden: false,
            last_activity: Instant::now(),
            last_ping_sent: Instant::now(),
            last_pong_at: Instant::now(),
            next_action_until: None,
            walk_action: None,
            walk_action_due: None,
            persist: None,
        }
    }
}
