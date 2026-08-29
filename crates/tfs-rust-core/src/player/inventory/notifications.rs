//! `Player::postAddNotification` / `postRemoveNotification` side effects.
// C++ reference: `src/player.cpp` ~3076–3191, `src/container.cpp` ~697–725.

use tfs_rust_common::Position;

use crate::creature::CreatureKind;
use crate::creature::LightInfo;
use crate::cylinder::CylinderLink;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::lua_scope::{
    fire_on_player_deequip, fire_on_player_equip, fire_on_player_inventory_update,
};

/// Parent cylinder hint for `requireListUpdate` / shop refresh — `player.cpp` postAdd/postRemove.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NotificationParent {
    Player,
    Container(ItemId),
    Tile(Position),
    None,
}

impl GameWorld {
    /// 772 `CheckCombatValues` — `crcombat.cc:128-147`.
    ///
    /// Diffs the seven resolved weapon fields (Shield/Close/Missile/Throw/Wand/Ammo/Fist)
    /// against the player's last snapshot; `DelayAttack(2000)` only when identity changes.
    pub(crate) fn player_check_combat_values(&mut self, cid: CreatureId) {
        let new_weapons = self.player_resolve_combat_weapons(cid);
        let old_weapons = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.last_combat_weapons,
            _ => return,
        };
        if old_weapons == new_weapons {
            return;
        }
        let server_ms = self.server_ms;
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.last_combat_weapons = new_weapons;
            p.base.delay_attack_ms(server_ms, 2000);
        }
    }

    /// Slot-gated wrapper — only Left/Right/Ammo mutations re-check (`notifications` equip path).
    pub(crate) fn player_maybe_delay_attack_on_weapon_slot_change(
        &mut self,
        cid: CreatureId,
        slot: u8,
    ) {
        use crate::inventory::InventorySlot;
        if slot != InventorySlot::Left as u8
            && slot != InventorySlot::Right as u8
            && slot != InventorySlot::Ammo as u8
        {
            return;
        }
        self.player_check_combat_values(cid);
    }

    /// Equipment slot (1–11) directly holding `item_id`, if any. Used to resolve the slot for
    /// `broadcast_player_inventory_slot` after a count mutation on an equipped item (e.g.
    /// weapon/shield charge wearout). Only scans direct equipment slots — does not descend
    /// into containers (use `equipment_slot_holding_container` for that).
    pub(crate) fn equipment_slot_for_item(
        &self,
        player: CreatureId,
        item_id: ItemId,
    ) -> Option<u8> {
        let Some(CreatureKind::Player(p)) = self.creatures.get(player) else {
            return None;
        };
        for (idx, slot_item) in p.equipment_slots.iter().enumerate() {
            if *slot_item == Some(item_id) {
                return Some((idx + 1) as u8);
            }
        }
        None
    }

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
            if let Some(c) = self.container_registry.get(*root)
                && c.is_holding_item(&self.container_registry, container_root)
            {
                return Some((idx + 1) as u8);
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

    /// C++ `Player::getCreatureLight` — `player.cpp` ~3403: max of `internalLight` vs `itemsLight`.
    pub(crate) fn player_creature_light(&self, cid: CreatureId) -> LightInfo {
        self.creatures
            .get(cid)
            .and_then(|k| match k {
                CreatureKind::Player(p) => Some(LightInfo::max_of(p.internal_light, p.items_light)),
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
        let pkt = self
            .codec
            .encode_creature_light(pid, light.level, light.color, access_player)
            .into_bytes();
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

    /// C++ `Player::onUpdateInventoryItem` — `NotifyTrades` (`operate.cc:990`).
    fn on_update_inventory_item(
        &mut self,
        _cid: CreatureId,
        _slot: u8,
        old_item: Option<ItemId>,
        new_item: ItemId,
    ) {
        if let Some(old) = old_item {
            self.notify_trades(old);
        }
        self.notify_trades(new_item);
    }

    /// C++ `Player::onRemoveInventoryItem` — `NotifyTrades`.
    fn on_remove_inventory_item(&mut self, _cid: CreatureId, item_id: ItemId) {
        self.notify_trades(item_id);
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
        // O(1) parent chain — same as auto-close / Lua (`script_item_position`).
        self.script_item_position(top)
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
        if self.player_owns_depot_container_tree(cid, top) {
            self.refresh_container_ui_for_all_viewers(item_id);
            return;
        }
        if let Some(cpos) = self.container_item_position(item_id) {
            if !Self::positions_in_range_1(player_pos, cpos) {
                self.auto_close_containers_for_container_item(cid, item_id);
            } else {
                // 772 `CloseContainer(Con, false)` refreshes when the container is still accessible
                // (`operate.cc:1060-1100`).
                self.refresh_container_ui_for_all_viewers(item_id);
            }
            return;
        }
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
            fire_on_player_equip(self, cid, item_id, slot);
            fire_on_player_inventory_update(self, cid, item_id, slot, true);
            self.on_update_inventory_item(cid, slot, None, item_id);
            // 772 `CheckCombatValues` — weapon identity change → `DelayAttack(2000)`
            // (`crcombat.cc:128-147`). Hands + ammo only (Close/Missile/Throw/Wand/Shield/Ammo).
            self.player_maybe_delay_attack_on_weapon_slot_change(cid, slot);
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
            fire_on_player_deequip(self, cid, item_id, slot);
            fire_on_player_inventory_update(self, cid, item_id, slot, false);
            self.clear_inventory_ability_on_deequip(cid, slot);
            self.on_remove_inventory_item(cid, item_id);
            self.player_maybe_delay_attack_on_weapon_slot_change(cid, slot);
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
            self.player_post_add_notification(cid, item_id, slot, CylinderLink::TopParent, parent);
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
        let b = LightInfo {
            level: 7,
            color: 215,
        };
        assert_eq!(LightInfo::max_of(a, b), b);
    }

    /// Phase 4b — `Player::getCreatureLight` max(internal, items).
    #[test]
    fn player_creature_light_prefers_higher_of_internal_vs_items() {
        let mut p = test_player_stub();
        p.items_light = LightInfo { level: 3, color: 1 };
        p.internal_light = LightInfo {
            level: 8,
            color: 215,
        };
        assert_eq!(
            LightInfo::max_of(p.internal_light, p.items_light),
            p.internal_light
        );
        p.internal_light = LightInfo::default();
        assert_eq!(
            LightInfo::max_of(p.internal_light, p.items_light),
            p.items_light
        );
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
        use crate::CreatureBase;
        use crate::creature::{Outfit, PlayerEconomy, PlayerInventory, PlayerSkills, PlayerSocial};
        use std::collections::{HashMap, HashSet};
        use std::time::Instant;
        use tfs_rust_common::Position;
        use tfs_rust_common::enums::{Direction, SkullType};

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
                var_speed: 0,
                skull: SkullType::None,
                drunkenness: 0,
                active_conditions: Vec::new(),
                walk_queue: Default::default(),
                walk_destinations: Default::default(),
                last_step: None,
                last_step_cost: 1,
                last_step_ground_speed: 150,
                next_wakeup: None,
                last_step_server_ms: None,
                earliest_walk_server_ms: 0,
                earliest_spell_server_ms: 0,
                earliest_multiuse_server_ms: 0,
                cancel_next_walk: false,
                force_update_follow_path: false,
                walk_update_ticks: 0,
                is_updating_path: false,
                has_follow_path: false,
                movement_blocked: false,
                stairhop_blocked_until: None,
                follow_target: None,
                attack_target: None,
                master: None,
                damage_map: Default::default(),
                last_hit_by: None,
                poison_damage_origin: None,
                fire_damage_origin: None,
                energy_damage_origin: None,
                earliest_attack_ms: 0,
                latest_attack_round: 0,
                earliest_defend_ms: 0,
                last_defend_ms: 0,
                learning_points: 0,
                todo: Default::default(),
                chase_mode: Default::default(),
                last_auto_walk_armed_ms: u64::MAX,
                drop_loot: true,
                skill_loss: true,
            },
            account_id: 0,
            guid: 1,
            account_type: 1,
            group_id: 1,
            set_max_speed: false,
            sex: crate::creature::PlayerSex::Male,
            vocation_id: 0,
            vocation_profile: crate::creature::vocation::VocationProfile::none_vocation(),
            level: 1,
            experience: 0,
            mana: 0,
            max_mana: 0,
            capacity: 400,
            inventory: PlayerInventory::default(),
            skills: PlayerSkills::with_levels(10, 10, 10, 10, 10, 10, 10, 0),
            economy: PlayerEconomy {
                balance: 0,
                soul: 0,
            },
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
            lastip: 0,
            equipment_slots: std::array::from_fn(|_| None),
            inventory_weight: 0,
            items_light: LightInfo::default(),
            internal_light: LightInfo::default(),
            inventory_abilities: [false; 11],
            dact_skills: [0; 7],
            mdact_skills: [0; 7],
            last_combat_weapons: Default::default(),
            var_stats: [0; 4],
            condition_suppressions: 0,
            shop_owner: None,
            vip_list: Vec::new(),
            outfits: Vec::new(),
            health_hidden: false,
            last_activity: Instant::now(),
            last_command_round: 0,
            last_action_round: 0,
            food_remaining: 0,
            food_level: 0,
            soul_cycle: 0,
            soul_count: 0,
            soul_max_count: 0,
            earliest_logout_round: 0,
            attacked_players: Vec::new(),
            former_attacked_players: Vec::new(),
            aggressor: false,
            former_aggressor: false,
            former_logout_round: 0,
            playerkiller_end: 0,
            murder_timestamps: [0; 20],
            logging_out: false,
            logout_allowed: false,
            last_ping_sent: Instant::now(),
            last_pong_at: Instant::now(),
            next_action_until: None,
            walk_action: None,
            depot_chests: HashMap::new(),
            depot_lockers: HashMap::new(),
            inbox_root: None,
            last_depot_id: -1,
            persist: None,
            sim_melee_defense: 0,
            sim_melee_attack: 0,
            attack_mode: Default::default(),
            secure_mode: false,
            earliest_protection_zone_round: 0,
            client_icons: 0,
            message_buffer_count: 0,
            message_buffer_ticks: 0,
            blessings: 0,
            exact_lethal_blow: false,
            registered_creature_events: HashSet::new(),
        }
    }
}
