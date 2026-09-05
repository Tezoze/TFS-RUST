//! NPC shop window runtime — TFS pack surface.
//!
//! Pack: `Game::playerPurchaseItem` / `playerSellItem` / `playerCloseShop` / `playerLookInShop`
//! — `game.cpp`; `Player::openShopWindow` / `closeShopWindow` / `updateSaleShopList` /
//! `hasShopItemForSale` — `player.cpp`; `Npc::onPlayerTrade` — `npc.cpp`.
//! Wire: TVP / TFS `ProtocolGame::sendShop` (0x7A) / `sendSaleItemList` (0x7B) / `sendCloseShop` (0x7C).

use tfs_rust_common::ConnId;
use tfs_rust_net::item_encode::client_fluid_to_server;
use tfs_rust_net::outgoing_extra::{
    ShopItemWire, send_close_shop, send_sale_item_list, send_shop, send_text_message_simple,
};

use crate::container::ContainerIterator;
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::item::Item;
use crate::item_look::{item_get_description_cpp, look_distance_tfs};
use crate::login_out::creature_wire_id;
use crate::player::inventory::money::{ITEM_CRYSTAL_COIN, ITEM_GOLD_COIN, ITEM_PLATINUM_COIN};
use crate::player_money_lib::player_remove_total_money;

const MESSAGE_INFO_DESCR: u8 = 0x16;
const SHOP_MAX_AMOUNT: u8 = 100;

/// One line in the player's active shop catalog (`Player::shopItemList` / `ShopInfo`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveShopItem {
    pub item_id: u16,
    pub sub_type: u8,
    pub buy_price: u32,
    pub sell_price: u32,
    pub name: String,
}

impl GameWorld {
    /// `Player::openShopWindow` — assign owner, store catalog, push 0x7A + 0x7B.
    pub fn player_open_shop(
        &mut self,
        player: CreatureId,
        npc: CreatureId,
        items: Vec<ActiveShopItem>,
    ) {
        let Some(CreatureKind::Player(_)) = self.creatures.get(player) else {
            return;
        };
        let Some(CreatureKind::Npc(_)) = self.creatures.get(npc) else {
            return;
        };
        self.player_close_shop(player, false);
        let Some(npc_kind) = self.creatures.get(npc) else {
            return;
        };
        let wire_id = creature_wire_id(npc, npc_kind);
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(player) {
            p.shop_owner = Some(wire_id);
            p.shop_items = items;
        }
        self.send_shop_to_player(player, npc);
        self.player_update_sale_shop_list(player);
    }

    /// `Player::closeShopWindow` — clear state, optional 0x7C, drop Lua callback refs.
    pub fn player_close_shop(&mut self, player: CreatureId, send_wire: bool) {
        let had_shop = self
            .creatures
            .get(player)
            .and_then(|k| match k {
                CreatureKind::Player(p) => p.shop_owner,
                _ => None,
            })
            .is_some();
        if !had_shop {
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(player) {
                p.shop_items.clear();
            }
            return;
        }

        let npc = self.player_shop_npc(player);
        if let Some(npc_id) = npc {
            self.events.on_npc_shop_close(npc_id, player);
        }
        self.clear_player_shop_callbacks(player);

        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(player) {
            p.shop_owner = None;
            p.shop_items.clear();
        }

        if send_wire {
            self.send_close_shop_to_player(player);
        }
    }

    /// `Game::playerLookInShop` — item description via pack look helper.
    pub fn player_look_in_shop(
        &mut self,
        conn_id: ConnId,
        player: CreatureId,
        client_id: u16,
        count: u8,
    ) {
        let Some((server_id, sub_type)) = self.resolve_shop_client_item(player, client_id, count)
        else {
            return;
        };
        if !self.has_shop_item_for_sale(player, server_id, sub_type) {
            return;
        }
        let Some(it) = self.items_db.items.get(&server_id) else {
            return;
        };
        let item = if it.is_splash() || it.is_fluid_container() {
            Item::new(server_id, u16::from(sub_type))
        } else {
            Item::new_single(server_id)
        };
        let player_pos = self
            .creatures
            .get(player)
            .map(|k| k.position())
            .unwrap_or_default();
        let desc = item_get_description_cpp(
            &item,
            it,
            it.weight,
            look_distance_tfs(player_pos, player_pos),
            None,
            None,
            None,
            None,
        );
        let msg = format!("You see {desc}");
        self.enqueue_outgoing(
            conn_id,
            send_text_message_simple(MESSAGE_INFO_DESCR, &msg).into_bytes(),
        );
    }

    /// `Game::playerPurchaseItem` — validate catalog, Lua buy callback or native fallback.
    pub fn player_purchase_item(
        &mut self,
        player: CreatureId,
        client_id: u16,
        count: u8,
        amount: u8,
        ignore_cap: bool,
        in_backpacks: bool,
    ) {
        if amount == 0 || amount > SHOP_MAX_AMOUNT {
            return;
        }
        let Some(npc) = self.player_shop_npc(player) else {
            return;
        };
        let Some((server_id, sub_type)) = self.resolve_shop_client_item(player, client_id, count)
        else {
            return;
        };
        if !self.has_shop_item_for_buy(player, server_id, sub_type) {
            return;
        }

        let invoked = self.events.on_npc_shop_buy(
            npc,
            player,
            server_id,
            sub_type,
            amount,
            ignore_cap,
            in_backpacks,
        );
        if !invoked {
            let _ = self.native_shop_buy(player, server_id, sub_type, amount, ignore_cap);
        }
        self.player_update_sale_shop_list(player);
    }

    /// `Game::playerSellItem` — validate catalog, Lua sell callback or native fallback.
    pub fn player_sell_item(
        &mut self,
        player: CreatureId,
        client_id: u16,
        count: u8,
        amount: u8,
        ignore_equipped: bool,
    ) {
        if amount == 0 || amount > SHOP_MAX_AMOUNT {
            return;
        }
        let Some(npc) = self.player_shop_npc(player) else {
            return;
        };
        let Some((server_id, sub_type)) = self.resolve_shop_client_item(player, client_id, count)
        else {
            return;
        };
        if !self.has_shop_item_for_sell(player, server_id, sub_type) {
            return;
        }

        let invoked = self.events.on_npc_shop_sell(
            npc,
            player,
            server_id,
            sub_type,
            amount,
            ignore_equipped,
        );
        if !invoked {
            let _ = self.native_shop_sell(player, server_id, sub_type, amount, ignore_equipped);
        }
        self.player_update_sale_shop_list(player);
    }

    /// `Player::updateSaleShopList` — rebuild sellable counts and send 0x7B.
    pub fn player_update_sale_shop_list(&mut self, player: CreatureId) {
        let Some(CreatureKind::Player(p)) = self.creatures.get(player) else {
            return;
        };
        if p.shop_owner.is_none() {
            return;
        }
        let shop_items = p.shop_items.clone();
        let coins = self.player_shop_money_total(player);
        let sale_counts = self.build_sale_counts(player, &shop_items);
        let Some(conn) = self.conn_for_creature(player) else {
            return;
        };
        let pkt = send_sale_item_list(coins, &sale_counts);
        self.enqueue_outgoing(conn, pkt.into_bytes());
    }

    /// Resolve `Player::shopOwner` wire id to live NPC `CreatureId`.
    pub fn player_shop_npc(&self, player: CreatureId) -> Option<CreatureId> {
        let wire = match self.creatures.get(player) {
            Some(CreatureKind::Player(p)) => p.shop_owner?,
            _ => return None,
        };
        self.creature_by_wire_id(wire)
    }

    /// NPC with dialogue focus on `player` (shop open from `.npc` scripts).
    pub fn find_shop_npc_for_player(&self, player: CreatureId) -> Option<CreatureId> {
        self.creatures.iter().find_map(|(cid, kind)| {
            if let CreatureKind::Npc(n) = kind {
                (n.runtime.focus == Some(player)).then_some(cid)
            } else {
                None
            }
        })
    }

    /// `Player::hasShopItemForSale` — catalog buy-price + fluid subtype gate.
    pub fn has_shop_item_for_sale(
        &self,
        player: CreatureId,
        item_id: u16,
        sub_type: u8,
    ) -> bool {
        self.has_shop_item_for_buy(player, item_id, sub_type)
    }

    fn has_shop_item_for_buy(&self, player: CreatureId, item_id: u16, sub_type: u8) -> bool {
        self.shop_line_matches(player, item_id, sub_type, true, false)
    }

    fn has_shop_item_for_sell(&self, player: CreatureId, item_id: u16, sub_type: u8) -> bool {
        self.shop_line_matches(player, item_id, sub_type, false, true)
    }

    fn shop_line_matches(
        &self,
        player: CreatureId,
        item_id: u16,
        sub_type: u8,
        require_buy: bool,
        require_sell: bool,
    ) -> bool {
        let Some(CreatureKind::Player(p)) = self.creatures.get(player) else {
            return false;
        };
        let Some(it) = self.items_db.items.get(&item_id) else {
            return false;
        };
        p.shop_items.iter().any(|entry| {
            entry.item_id == item_id
                && (!require_buy || entry.buy_price != 0)
                && (!require_sell || entry.sell_price != 0)
                && (!it.is_fluid_container() || entry.sub_type == sub_type)
        })
    }

    /// Build wire rows for `sendShop` from the player's active catalog.
    pub fn build_shop_wire_items(&self, player: CreatureId) -> Vec<ShopItemWire> {
        let Some(CreatureKind::Player(p)) = self.creatures.get(player) else {
            return Vec::new();
        };
        p.shop_items
            .iter()
            .filter_map(|entry| {
                let it = self.items_db.items.get(&entry.item_id)?;
                let display_name = if entry.name.is_empty() {
                    it.name.clone()
                } else {
                    entry.name.clone()
                };
                Some(ShopItemWire {
                    client_id: self.items_db.client_id_for_server(entry.item_id),
                    fluid_subtype: entry.sub_type,
                    is_fluid: it.is_splash() || it.is_fluid_container(),
                    real_name: display_name,
                    weight: it.weight,
                    buy_price: entry.buy_price,
                    sell_price: entry.sell_price,
                })
            })
            .collect()
    }

    /// Build `(client_item_id, count)` pairs for `sendSaleItemList`.
    pub fn build_sale_counts(
        &self,
        player: CreatureId,
        shop_items: &[ActiveShopItem],
    ) -> Vec<(u16, u8)> {
        let mut out = Vec::new();
        for entry in shop_items {
            if entry.sell_price == 0 {
                continue;
            }
            let Some(it) = self.items_db.items.get(&entry.item_id) else {
                continue;
            };
            let subtype_query = if it.is_fluid_container() || it.is_splash() {
                i32::from(entry.sub_type)
            } else if !it.stackable() && it.charges > 0 {
                if entry.sub_type == 0 {
                    -1
                } else {
                    i32::from(entry.sub_type)
                }
            } else {
                -1
            };
            let count = self.player_get_item_type_count(player, entry.item_id, subtype_query);
            if count > 0 {
                let client_id = self.items_db.client_id_for_server(entry.item_id);
                out.push((client_id, count.min(u8::MAX as u32) as u8));
            }
        }
        out
    }

    fn native_shop_buy(
        &mut self,
        player: CreatureId,
        item_id: u16,
        sub_type: u8,
        amount: u8,
        ignore_cap: bool,
    ) -> bool {
        let buy_price = self
            .shop_line(player, item_id, sub_type)
            .map(|l| l.buy_price)
            .unwrap_or(0);
        if buy_price == 0 {
            return false;
        }
        let total = u64::from(buy_price) * u64::from(amount);
        if self.player_shop_money_total(player) < total {
            return false;
        }
        if !ignore_cap && !self.player_can_carry_shop_purchase(player, item_id, amount) {
            return false;
        }
        if self
            .npc_give_to(player, item_id, u32::from(amount), i32::from(sub_type))
            .is_err()
        {
            return false;
        }
        player_remove_total_money(self, player, total)
    }

    fn player_can_carry_shop_purchase(
        &self,
        player: CreatureId,
        item_id: u16,
        amount: u8,
    ) -> bool {
        let Some(it) = self.items_db.items.get(&item_id) else {
            return false;
        };
        let units = u32::from(amount.max(1));
        let need = it.weight.saturating_mul(units);
        self.player_free_capacity_u32(player)
            .is_some_and(|free| free >= need)
    }

    fn native_shop_sell(
        &mut self,
        player: CreatureId,
        item_id: u16,
        sub_type: u8,
        amount: u8,
        ignore_equipped: bool,
    ) -> bool {
        let sell_price = self
            .shop_line(player, item_id, sub_type)
            .map(|l| l.sell_price)
            .unwrap_or(0);
        if sell_price == 0 {
            return false;
        }
        let data = if self
            .items_db
            .items
            .get(&item_id)
            .is_some_and(|t| t.is_fluid_container())
        {
            i32::from(sub_type)
        } else {
            -1
        };
        if !self.player_remove_item_of_type(
            player,
            item_id,
            u32::from(amount),
            data,
            ignore_equipped,
        ) {
            return false;
        }
        let payout = i32::try_from(u32::from(sell_price) * u32::from(amount)).unwrap_or(i32::MAX);
        self.player_create_money(player, payout).is_ok()
    }

    fn shop_line<'a>(
        &'a self,
        player: CreatureId,
        item_id: u16,
        sub_type: u8,
    ) -> Option<&'a ActiveShopItem> {
        let CreatureKind::Player(p) = self.creatures.get(player)? else {
            return None;
        };
        p.shop_items.iter().find(|entry| {
            entry.item_id == item_id
                && (!self
                    .items_db
                    .items
                    .get(&item_id)
                    .is_some_and(|t| t.is_fluid_container())
                    || entry.sub_type == sub_type)
        })
    }

    fn resolve_shop_client_item(
        &self,
        player: CreatureId,
        client_id: u16,
        count: u8,
    ) -> Option<(u16, u8)> {
        let _ = player;
        let server_id = self.items_db.server_id_for_client(client_id)?;
        let it = self.items_db.items.get(&server_id)?;
        let sub_type = if it.is_splash() || it.is_fluid_container() {
            client_fluid_to_server(count)
        } else {
            count
        };
        Some((server_id, sub_type))
    }

    fn player_shop_money_total(&self, player: CreatureId) -> u64 {
        let coins = self.player_count_money(player);
        let bank = match self.creatures.get(player) {
            Some(CreatureKind::Player(p)) => p.economy.balance,
            _ => 0,
        };
        coins.saturating_add(bank)
    }

    fn send_shop_to_player(&mut self, player: CreatureId, npc: CreatureId) {
        let Some(conn) = self.conn_for_creature(player) else {
            return;
        };
        let npc_name = self
            .creatures
            .get(npc)
            .map(|k| k.base().name.clone())
            .unwrap_or_default();
        let items = self.build_shop_wire_items(player);
        let pkt = send_shop(&npc_name, &items);
        self.enqueue_outgoing(conn, pkt.into_bytes());
    }

    fn send_close_shop_to_player(&mut self, player: CreatureId) {
        let Some(conn) = self.conn_for_creature(player) else {
            return;
        };
        let pkt = send_close_shop();
        self.enqueue_outgoing(conn, pkt.into_bytes());
    }

    fn clear_player_shop_callbacks(&mut self, player: CreatureId) {
        self.events.clear_player_shop_lua_callbacks(player);
    }

    /// Inventory/container change hook — `Player::updateSaleShopList`.
    pub(crate) fn try_update_sale_shop_list(&mut self, player: CreatureId, item_id: crate::ids::ItemId) {
        let should_refresh = match self.creatures.get(player) {
            Some(CreatureKind::Player(p)) if p.shop_owner.is_some() => {
                self.shop_inventory_change_affects_sale_list(player, item_id)
            }
            _ => false,
        };
        if should_refresh {
            self.player_update_sale_shop_list(player);
        }
    }

    fn shop_inventory_change_affects_sale_list(
        &self,
        player: CreatureId,
        item_id: crate::ids::ItemId,
    ) -> bool {
        let Some(item) = self.items.get(item_id) else {
            return false;
        };
        let item_type = item.item_type;
        if matches!(
            item_type,
            ITEM_GOLD_COIN | ITEM_PLATINUM_COIN | ITEM_CRYSTAL_COIN
        ) {
            return true;
        }
        let Some(CreatureKind::Player(p)) = self.creatures.get(player) else {
            return false;
        };
        if p
            .shop_items
            .iter()
            .any(|entry| entry.sell_price != 0 && entry.item_id == item_type)
        {
            return true;
        }
        if self.container_registry.get(item_id).is_some() {
            for child in ContainerIterator::new(&self.container_registry, item_id) {
                if self.shop_inventory_change_affects_sale_list(player, child) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::sim_harness::{
        ensure_walkable_tile, insert_npc, insert_player, minimal_world, pickup_item_type,
        test_player,
    };
    use tfs_rust_common::ConnId;
    use tfs_rust_common::Position;

    fn shop_fixture() -> (GameWorld, CreatureId, CreatureId) {
        let mut world = minimal_world();
        {
            let db = Arc::make_mut(&mut world.items_db);
            for &sid in &[2148u16, 1987u16] {
                if let Some(it) = db.items.get_mut(&sid) {
                    it.client_id = sid;
                    if sid == 2148 {
                        it.flags |= 1 << 7; // ItemType::FLAG_STACKABLE
                    }
                }
                db.client_to_server.insert(sid, sid);
            }
        }
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let player = insert_player(&mut world, test_player("Buyer", pos));
        let npc = insert_npc(&mut world, "Merchant", pos, 100);
        world.register_conn_mapping(ConnId(1), player);
        equip_backpack(&mut world, player);
        if let Some(CreatureKind::Npc(n)) = world.creatures.get_mut(npc) {
            n.runtime.focus = Some(player);
        }
        (world, player, npc)
    }

    fn equip_backpack(world: &mut GameWorld, cid: CreatureId) {
        use crate::container::Container;
        use crate::inventory::InventorySlot;
        let bp = world.items.insert(Item::new_single(1987));
        world
            .internal_add_item_to_inventory_slot(cid, InventorySlot::Backpack as u8, bp)
            .expect("backpack slot");
        let mut reg = std::mem::take(&mut world.container_registry);
        reg.register(Container::new(bp, 20));
        world.container_registry = reg;
    }

    fn gold_shop_item() -> ActiveShopItem {
        ActiveShopItem {
            item_id: ITEM_GOLD_COIN,
            sub_type: 0,
            buy_price: 1,
            sell_price: 1,
            name: "gold coin".into(),
        }
    }

    #[test]
    fn open_and_close_shop_clears_state() {
        let (mut world, player, npc) = shop_fixture();
        world.player_open_shop(player, npc, vec![gold_shop_item()]);
        assert!(world.creatures.get(player).and_then(|k| match k {
            CreatureKind::Player(p) => p.shop_owner,
            _ => None,
        }).is_some());
        world.player_close_shop(player, false);
        assert!(world.creatures.get(player).and_then(|k| match k {
            CreatureKind::Player(p) => p.shop_owner,
            _ => None,
        }).is_none());
        assert!(world
            .creatures
            .get(player)
            .and_then(|k| match k {
                CreatureKind::Player(p) => Some(p.shop_items.is_empty()),
                _ => None,
            })
            .unwrap());
    }

    #[test]
    fn buy_requires_money() {
        let (mut world, player, npc) = shop_fixture();
        world.player_open_shop(player, npc, vec![gold_shop_item()]);
        let client_id = world.items_db.client_id_for_server(ITEM_GOLD_COIN);
        world.player_purchase_item(player, client_id, 0, 5, false, false);
        assert_eq!(world.player_get_item_type_count(player, ITEM_GOLD_COIN, -1), 0);
    }

    fn bag_shop_item() -> ActiveShopItem {
        ActiveShopItem {
            item_id: 1987,
            sub_type: 0,
            buy_price: 5,
            sell_price: 0,
            name: "backpack".into(),
        }
    }

    #[test]
    fn buy_with_money_adds_items() {
        let (mut world, player, npc) = shop_fixture();
        world
            .player_create_money(player, 10)
            .expect("seed money");
        let money_before = world.player_count_money(player);
        world.player_open_shop(player, npc, vec![bag_shop_item()]);
        let client_id = world.items_db.client_id_for_server(1987);
        world.player_purchase_item(player, client_id, 0, 1, false, false);
        assert!(
            world.player_get_item_type_count(player, 1987, -1) >= 2,
            "expected an extra backpack after purchase"
        );
        assert!(world.player_count_money(player) < money_before);
        assert_eq!(world.player_count_money(player), money_before - 5);
    }

    #[test]
    fn remove_gold_works_while_shop_open() {
        let (mut world, player, npc) = shop_fixture();
        world
            .player_add_item_count(player, ITEM_GOLD_COIN, 8, -1)
            .expect("seed stack");
        world.player_open_shop(player, npc, vec![gold_shop_item()]);
        assert!(world.player_remove_item_of_type(player, ITEM_GOLD_COIN, 3, -1, false));
        assert_eq!(world.player_get_item_type_count(player, ITEM_GOLD_COIN, -1), 5);
    }

    fn bag_sell_item() -> ActiveShopItem {
        ActiveShopItem {
            item_id: 1987,
            sub_type: 0,
            buy_price: 0,
            sell_price: 2,
            name: "backpack".into(),
        }
    }

    #[test]
    fn sell_removes_items_and_pays() {
        let (mut world, player, npc) = shop_fixture();
        world
            .player_add_item_count(player, 1987, 1, -1)
            .expect("seed extra backpack");
        let bags_before = world.player_get_item_type_count(player, 1987, -1);
        assert!(bags_before >= 2);
        let money_before = world.player_count_money(player);
        world.player_open_shop(player, npc, vec![bag_sell_item()]);
        let client_id = world.items_db.client_id_for_server(1987);
        assert_ne!(client_id, 0);
        assert!(world.has_shop_item_for_sell(player, 1987, 0));
        world.player_sell_item(player, client_id, 0, 1, true);
        assert_eq!(
            world.player_get_item_type_count(player, 1987, -1),
            bags_before - 1
        );
        assert_eq!(world.player_count_money(player), money_before + 2);
    }

    #[test]
    fn sale_list_counts_refresh_on_inventory_change() {
        let (mut world, player, npc) = shop_fixture();
        world
            .player_add_item_count(player, ITEM_GOLD_COIN, 4, -1)
            .expect("seed stack");
        world.player_open_shop(player, npc, vec![gold_shop_item()]);
        let shop_items = world
            .creatures
            .get(player)
            .and_then(|k| match k {
                CreatureKind::Player(p) => Some(p.shop_items.clone()),
                _ => None,
            })
            .unwrap();
        let gold_client = world.items_db.client_id_for_server(ITEM_GOLD_COIN);
        let before = world.build_sale_counts(player, &shop_items);
        assert_eq!(
            before.iter().find(|(id, _)| *id == gold_client).map(|(_, c)| *c),
            Some(4)
        );
        world.player_remove_item_of_type(player, ITEM_GOLD_COIN, 2, -1, false);
        let after = world.build_sale_counts(player, &shop_items);
        assert_eq!(
            after.iter().find(|(id, _)| *id == gold_client).map(|(_, c)| *c),
            Some(2)
        );
        world.player_update_sale_shop_list(player);
    }

    #[test]
    fn find_shop_npc_matches_focus() {
        let (world, player, npc) = shop_fixture();
        assert_eq!(world.find_shop_npc_for_player(player), Some(npc));
    }
}
