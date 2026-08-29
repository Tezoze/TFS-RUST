//! Player-to-player trade.
//!
//! Pack surface: TFS `Game::internalStartTrade` / `playerLookInTrade` / house transfer item.
//! Corpus: `TCreature::ToDoTrade` / `Trade` (`cract.cc`), `TPlayer::InspectTrade` /
//! `AcceptTrade` / `RejectTrade` (`crplayer.cc`), `NotifyTrades` (`operate.cc`).
//! Wire: TVP `gameserver/src/protocolgame.cpp` `sendTradeItemRequest` / `sendCloseTrade`.

use std::collections::{HashMap, VecDeque};

use tfs_rust_common::{ConnId, Position};
use tfs_rust_net::codec::ItemTemplateArgs;
use tfs_rust_net::outgoing_extra::send_text_message_simple;

use crate::container::ContainerType;
use crate::creature::CreatureKind;
use crate::creature_todo::{ActionObjectRef, CreatureAction};
use crate::cylinder::Cylinder;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::InventorySlot;
use crate::item::Item;
use crate::item_constants::ITEM_DOCUMENT_RO;
use crate::item_look::{item_get_description_cpp, look_distance_tfs};
use crate::return_value::ReturnValue;
use crate::thing::Thing;
use crate::walk::are_in_range_1_1_0;

const TRADE_MAX_OBJECTS: u32 = 100;
const TRADE_DISTANCE: i32 = 2;
const MESSAGE_INFO_DESCR: u8 = 0x16;
const TRADE_CANCELLED: &str = "Trade cancelled.";

#[derive(Debug, Clone, Copy)]
pub struct TradeSide {
    pub partner: CreatureId,
    pub item: ItemId,
    pub accepted: bool,
    pub house_id: Option<u32>,
}

#[derive(Debug, Default)]
pub struct TradeRegistry {
    pub sides: HashMap<CreatureId, TradeSide>,
}

impl TradeRegistry {
    pub fn clear_player(&mut self, cid: CreatureId) {
        self.sides.remove(&cid);
    }
}

fn object_distance(a: Position, b: Position) -> i32 {
    if a.z != b.z {
        return i32::MAX;
    }
    (a.x.abs_diff(b.x)).max(a.y.abs_diff(b.y)) as i32
}

impl GameWorld {
    pub fn player_request_trade(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        pos: Position,
        sprite_id: u16,
        stack_pos: u8,
        partner_wire: u32,
    ) {
        let obj = ActionObjectRef {
            pos,
            stack_pos,
            sprite_id,
        };
        self.player_todo_clear_with_snapback(conn_id, cid);
        if let Err(rv) = self.enqueue_player_trade(cid, obj, partner_wire) {
            self.send_cancel_message(conn_id, rv);
        } else {
            self.todo_start_from_action(cid, 1);
        }
    }

    /// `TCreature::ToDoTrade` (`cract.cc:1202-1256`).
    pub(crate) fn enqueue_player_trade(
        &mut self,
        cid: CreatureId,
        obj: ActionObjectRef,
        partner_wire: u32,
    ) -> Result<(), ReturnValue> {
        let thing = self
            .internal_get_thing_move(cid, obj.pos, obj.stack_pos, obj.sprite_id)
            .ok_or(ReturnValue::NotPossible)?;
        let Thing::Item(item_id) = thing else {
            return Err(ReturnValue::NotPossible);
        };
        let item_type = self.items.get(item_id).map(|i| i.item_type).unwrap_or(0);
        let Some(it) = self.items_db.items.get(&item_type) else {
            return Err(ReturnValue::NotPossible);
        };
        if !it.moveable() {
            return Err(ReturnValue::NotMoveable);
        }
        if !it.pickupable() {
            return Err(ReturnValue::CannotPickup);
        }
        if self.items.get(item_id).is_some_and(|i| i.unique_id() != 0)
            || self.item_inside_depot(item_id)
        {
            return Err(ReturnValue::NotPossible);
        }
        if partner_wire == 0 {
            return Err(ReturnValue::ThisIsImpossible);
        }
        let Some(partner) = self.creature_by_wire_id(partner_wire) else {
            return Err(ReturnValue::PlayerWithThisNameIsNotOnline);
        };
        if !matches!(self.creatures.get(partner), Some(CreatureKind::Player(_))) {
            return Err(ReturnValue::ThisIsImpossible);
        }
        if obj.pos.x != 0xFFFF {
            self.validate_action_object_z_floor(cid, obj)?;
            if !self.object_in_range(cid, obj.pos, 1) {
                let now = std::time::Instant::now();
                self.setup_player_walk_to_target(cid, obj.pos, now)?;
                let has_steps = self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| !k.base().walk_queue.is_empty());
                if has_steps && let Some(k) = self.creatures.get_mut(cid) {
                    k.base_mut().todo.queue.push_back(CreatureAction::Go);
                }
            }
        }
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut()
                .todo
                .queue
                .push_back(CreatureAction::Trade { obj, partner_wire });
        }
        Ok(())
    }

    /// `TCreature::Trade` (`cract.cc:653-725`).
    pub(crate) fn player_execute_trade(
        &mut self,
        cid: CreatureId,
        obj: ActionObjectRef,
        partner_wire: u32,
    ) -> Result<(), ReturnValue> {
        if !matches!(self.creatures.get(cid), Some(CreatureKind::Player(_))) {
            return Err(ReturnValue::NotPossible);
        }
        let thing = self
            .internal_get_thing_move(cid, obj.pos, obj.stack_pos, obj.sprite_id)
            .ok_or(ReturnValue::NotPossible)?;
        let Thing::Item(item_id) = thing else {
            return Err(ReturnValue::NotPossible);
        };
        let partner = self
            .creature_by_wire_id(partner_wire)
            .ok_or(ReturnValue::PlayerWithThisNameIsNotOnline)?;
        self.internal_start_trade(cid, partner, item_id, None)
    }

    /// Start or complete a counter-offer. `house_id` marks a house-transfer document.
    pub(crate) fn internal_start_trade(
        &mut self,
        cid: CreatureId,
        partner: CreatureId,
        item_id: ItemId,
        house_id: Option<u32>,
    ) -> Result<(), ReturnValue> {
        if cid == partner {
            return Err(ReturnValue::ThisIsImpossible);
        }
        if !matches!(self.creatures.get(partner), Some(CreatureKind::Player(_))) {
            return Err(ReturnValue::PlayerWithThisNameIsNotOnline);
        }
        if self.trades.sides.contains_key(&cid) {
            return Err(ReturnValue::YouAreAlreadyTrading);
        }
        let is_house_doc = self
            .items
            .get(item_id)
            .is_some_and(|i| i.item_type == ITEM_DOCUMENT_RO);
        if house_id.is_none()
            && !is_house_doc
            && !self.trade_item_accessible(cid, item_id)
        {
            return Err(ReturnValue::NotPossible);
        }
        if !is_house_doc
            && (self.items.get(item_id).is_some_and(|i| i.unique_id() != 0)
                || self.item_inside_depot(item_id))
        {
            return Err(ReturnValue::NotPossible);
        }
        if self.count_trade_objects(item_id) > TRADE_MAX_OBJECTS {
            return Err(ReturnValue::TooManyTradeObjects);
        }
        let pos = self
            .creatures
            .get(cid)
            .map(|k| k.position())
            .ok_or(ReturnValue::NotPossible)?;
        let partner_pos = self
            .creatures
            .get(partner)
            .map(|k| k.position())
            .ok_or(ReturnValue::PlayerWithThisNameIsNotOnline)?;
        if object_distance(pos, partner_pos) > TRADE_DISTANCE {
            return Err(ReturnValue::TooFarAway);
        }
        if !self.map.throw_possible(pos, partner_pos, 0) {
            return Err(ReturnValue::CannotThrow);
        }
        if let Some(other) = self.trades.sides.get(&partner) {
            if other.partner != cid {
                return Err(ReturnValue::ThisPlayerIsAlreadyTrading);
            }
            if self.trade_contains(item_id, other.item) || self.trade_contains(other.item, item_id)
            {
                return Err(ReturnValue::NotPossible);
            }
        }
        let name = self
            .creatures
            .get(cid)
            .map(|k| k.base().name.clone())
            .unwrap_or_default();
        self.trades.sides.insert(
            cid,
            TradeSide {
                partner,
                item: item_id,
                accepted: false,
                house_id,
            },
        );
        if self
            .trades
            .sides
            .get(&partner)
            .is_some_and(|s| s.partner == cid)
        {
            self.send_trade_offer(partner, &name, false, item_id);
            let partner_name = self
                .creatures
                .get(partner)
                .map(|k| k.base().name.clone())
                .unwrap_or_default();
            let partner_item = self.trades.sides.get(&partner).map(|s| s.item);
            self.send_trade_offer(cid, &name, true, item_id);
            if let Some(pitem) = partner_item {
                self.send_trade_offer(cid, &partner_name, false, pitem);
            }
        } else {
            self.send_trade_info(
                partner,
                &format!("{name} wants to trade with you."),
            );
            self.send_trade_offer(cid, &name, true, item_id);
        }
        Ok(())
    }

    fn item_inside_depot(&self, item_id: ItemId) -> bool {
        let mut cur = Some(item_id);
        while let Some(id) = cur {
            if self
                .container_registry
                .get(id)
                .is_some_and(|c| c.container_type == ContainerType::Depot)
            {
                return true;
            }
            cur = match self.items.get(id).and_then(|i| i.parent) {
                Some(Cylinder::Container { item_id, .. }) => Some(item_id),
                _ => None,
            };
        }
        false
    }

    pub fn player_look_in_trade(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        counter_offer: bool,
        index: u8,
    ) {
        let Some(item_id) = self.inspect_trade(cid, !counter_offer, index as i32) else {
            return;
        };
        let Some(item) = self.items.get(item_id) else {
            return;
        };
        let Some(it) = self.items_db.items.get(&item.item_type) else {
            return;
        };
        let player_pos = self.creatures.get(cid).map(|k| k.position()).unwrap_or_default();
        let thing_pos = self.script_item_position(item_id).unwrap_or(player_pos);
        let look_d = look_distance_tfs(player_pos, thing_pos);
        let rune_vocs = self
            .spells
            .get_rune(item.item_type)
            .map(|r| r.vocations.as_slice());
        let desc = item_get_description_cpp(
            item,
            it,
            it.weight,
            look_d,
            None,
            None,
            rune_vocs,
            None,
        );
        let msg = format!("You see {desc}");
        self.enqueue_outgoing(conn_id, send_text_message_simple(MESSAGE_INFO_DESCR, &msg).into_bytes());
    }

    /// `TPlayer::InspectTrade` (`crplayer.cc:811-843`). `own_offer` true = own root.
    fn inspect_trade(&self, cid: CreatureId, own_offer: bool, mut position: i32) -> Option<ItemId> {
        let mut obj = if own_offer {
            self.trades.sides.get(&cid)?.item
        } else {
            let side = self.trades.sides.get(&cid)?;
            let partner_side = self.trades.sides.get(&side.partner)?;
            if partner_side.partner != cid {
                return None;
            }
            partner_side.item
        };
        while position > 0 {
            let n = self.count_trade_objects(obj) as i32;
            if position < n {
                obj = *self.trade_child_items(obj).first()?;
                position -= 1;
            } else {
                obj = self.trade_next_sibling(obj)?;
                position -= n;
            }
        }
        Some(obj)
    }

    fn trade_next_sibling(&self, item_id: ItemId) -> Option<ItemId> {
        let parent = match self.items.get(item_id).and_then(|i| i.parent) {
            Some(Cylinder::Container { item_id, .. }) => item_id,
            _ => return None,
        };
        let kids = self.trade_child_items(parent);
        let i = kids.iter().position(|&id| id == item_id)?;
        kids.get(i + 1).copied()
    }

    pub fn player_accept_trade(&mut self, cid: CreatureId) {
        let Some(side) = self.trades.sides.get(&cid).copied() else {
            return;
        };
        let Some(partner_side) = self.trades.sides.get(&side.partner).copied() else {
            return;
        };
        if partner_side.partner != cid {
            return;
        }
        if let Some(s) = self.trades.sides.get_mut(&cid) {
            s.accepted = true;
        }
        if !partner_side.accepted {
            return;
        }
        let a = cid;
        let b = side.partner;
        let item_a = side.item;
        let item_b = partner_side.item;
        if let Some(house_id) = side.house_id.or(partner_side.house_id) {
            let buyer = if side.house_id.is_some() {
                side.partner
            } else {
                cid
            };
            let buyer_guid = match self.creatures.get(buyer) {
                Some(CreatureKind::Player(p)) => p.guid,
                _ => {
                    self.close_trade_pair(a, b, false);
                    return;
                }
            };
            self.lua_script_house_set_owner(house_id, buyer_guid);
            self.items.remove(item_a);
            self.items.remove(item_b);
            self.close_trade_pair(a, b, false);
            return;
        }
        if let Err(rv) = self.validate_trade_accept(a, b, item_a, item_b) {
            if let Some(conn) = self.conn_for_creature(a) {
                self.send_cancel_message(conn, rv);
            }
            self.close_trade_pair(a, b, false);
            return;
        }
        let dest_a = self.trade_dest_cylinder(b, item_a);
        let dest_b = self.trade_dest_cylinder(a, item_b);
        let (Some(dest_a), Some(dest_b)) = (dest_a, dest_b) else {
            if let Some(conn) = self.conn_for_creature(a) {
                self.send_cancel_message(conn, ReturnValue::NotEnoughRoom);
            }
            self.close_trade_pair(a, b, false);
            return;
        };
        self.trades.sides.remove(&a);
        self.trades.sides.remove(&b);
        self.send_close_trade_to(a);
        self.send_close_trade_to(b);
        let pos_a = self.creatures.get(a).map(|k| k.position()).unwrap_or_default();
        let pos_b = self.creatures.get(b).map(|k| k.position()).unwrap_or_default();
        let from_a = self.items.get(item_a).and_then(|i| i.parent);
        let from_b = self.items.get(item_b).and_then(|i| i.parent);
        let tile_b = Cylinder::Tile { pos: pos_b };
        let tile_a = Cylinder::Tile { pos: pos_a };
        if let Some(from) = from_a {
            let _ = self.internal_move_item(None, from, tile_b, item_a, u16::MAX, crate::cylinder::CylinderFlags::NONE, None);
        }
        if let Some(from) = from_b {
            let _ = self.internal_move_item(None, from, tile_a, item_b, u16::MAX, crate::cylinder::CylinderFlags::NONE, None);
        }
        let _ = self.internal_move_item(
            None,
            tile_b,
            dest_a,
            item_a,
            u16::MAX,
            crate::cylinder::CylinderFlags::NONE,
            None,
        );
        let _ = self.internal_move_item(
            None,
            tile_a,
            dest_b,
            item_b,
            u16::MAX,
            crate::cylinder::CylinderFlags::NONE,
            None,
        );
    }

    fn validate_trade_accept(
        &mut self,
        a: CreatureId,
        b: CreatureId,
        item_a: ItemId,
        item_b: ItemId,
    ) -> Result<(), ReturnValue> {
        if !self.trade_item_accessible(a, item_a) || !self.trade_item_accessible(b, item_b) {
            return Err(ReturnValue::NotPossible);
        }
        self.check_trade_carry(b, item_a, item_b)?;
        self.check_trade_carry(a, item_b, item_a)?;
        Ok(())
    }

    fn check_trade_carry(
        &self,
        receiver: CreatureId,
        incoming: ItemId,
        outgoing: ItemId,
    ) -> Result<(), ReturnValue> {
        let Some(CreatureKind::Player(p)) = self.creatures.get(receiver) else {
            return Err(ReturnValue::NotPossible);
        };
        let cap = p.capacity as u32;
        let mut weight = p.inventory_weight;
        weight = weight.saturating_add(self.complete_item_weight(incoming));
        if self.item_held_by_player(outgoing, receiver) {
            weight = weight.saturating_sub(self.complete_item_weight(outgoing));
        }
        if weight > cap {
            return Err(ReturnValue::NotEnoughCapacity);
        }
        Ok(())
    }

    fn trade_dest_cylinder(&mut self, receiver: CreatureId, item_id: ItemId) -> Option<Cylinder> {
        self.hydrate_player_equipment_containers(receiver);
        if self.query_add_item_to_inventory(receiver, item_id) == ReturnValue::NoError {
            let backpack = match self.creatures.get(receiver) {
                Some(CreatureKind::Player(p)) => p.equipment_slots[2],
                _ => None,
            };
            if let Some(bp) = backpack {
                return Some(Cylinder::Container {
                    item_id: bp,
                    index: crate::cylinder::INDEX_WHEREEVER,
                });
            }
            return Some(Cylinder::Inventory {
                player_id: receiver,
                slot: InventorySlot::Wherever as u8,
            });
        }
        None
    }

    /// `TPlayer::RejectTrade` (`crplayer.cc:990-1000`).
    pub fn player_close_trade(&mut self, cid: CreatureId) {
        self.reject_trade(cid);
    }

    pub(crate) fn reject_trade(&mut self, cid: CreatureId) {
        let Some(side) = self.trades.sides.remove(&cid) else {
            return;
        };
        if let Some(ps) = self.trades.sides.get(&side.partner)
            && ps.partner == cid
        {
            self.trades.sides.remove(&side.partner);
            self.send_close_trade_to(side.partner);
            self.send_trade_failure(side.partner, TRADE_CANCELLED);
        }
    }

    /// Close both windows and optionally message both (`NotifyTrades` / walk).
    pub(crate) fn cancel_trade_for_player(&mut self, cid: CreatureId) {
        if !self.trades.sides.contains_key(&cid) {
            return;
        }
        self.send_close_trade_to(cid);
        self.send_trade_failure(cid, TRADE_CANCELLED);
        self.reject_trade(cid);
    }

    /// `NotifyTrades` (`operate.cc:990-1023`).
    pub(crate) fn notify_trades(&mut self, obj: ItemId) {
        let players: Vec<CreatureId> = self.trades.sides.keys().copied().collect();
        for cid in players {
            let Some(root) = self.trades.sides.get(&cid).map(|s| s.item) else {
                continue;
            };
            if self.trade_contains(obj, root) || self.trade_contains(root, obj) {
                self.cancel_trade_for_player(cid);
            }
        }
    }

    /// `NotifyGo` trade cancel (`cract.cc:1489-1511`).
    pub(crate) fn player_check_trade_walk(&mut self, cid: CreatureId) {
        if !matches!(self.creatures.get(cid), Some(CreatureKind::Player(_))) {
            return;
        }
        let Some(side) = self.trades.sides.get(&cid).copied() else {
            return;
        };
        let partner_ok = matches!(
            self.creatures.get(side.partner),
            Some(CreatureKind::Player(_))
        );
        let accessible = self.trade_item_accessible(cid, side.item);
        let (pos, partner_pos) = match (
            self.creatures.get(cid).map(|k| k.position()),
            self.creatures.get(side.partner).map(|k| k.position()),
        ) {
            (Some(a), Some(b)) => (a, b),
            _ => {
                self.cancel_trade_for_player(cid);
                return;
            }
        };
        if !partner_ok
            || !accessible
            || object_distance(pos, partner_pos) > TRADE_DISTANCE
            || !self.map.throw_possible(pos, partner_pos, 0)
        {
            self.cancel_trade_for_player(cid);
        }
    }

    pub(crate) fn player_trade_item_id(&self, player_id: CreatureId) -> Option<ItemId> {
        self.trades.sides.get(&player_id).map(|s| s.item)
    }

    pub(crate) fn lua_house_start_trade(
        &mut self,
        house_id: u32,
        player_id: u64,
        partner_id: u64,
    ) -> i32 {
        let Some(player) = self.resolve_creature_u64(player_id) else {
            return ReturnValue::NotPossible as i32;
        };
        let Some(partner) = self.resolve_creature_u64(partner_id) else {
            return ReturnValue::NotPossible as i32;
        };
        let doc_seller = self.items.insert(Item::new_single(ITEM_DOCUMENT_RO));
        let doc_buyer = self.items.insert(Item::new_single(ITEM_DOCUMENT_RO));
        if let Some(item) = self.items.get_mut(doc_seller) {
            item.parent = Some(Cylinder::Inventory {
                player_id: player,
                slot: InventorySlot::Wherever as u8,
            });
        }
        if let Some(item) = self.items.get_mut(doc_buyer) {
            item.parent = Some(Cylinder::Inventory {
                player_id: partner,
                slot: InventorySlot::Wherever as u8,
            });
        }
        if let Err(rv) = self.internal_start_trade(player, partner, doc_seller, Some(house_id)) {
            self.items.remove(doc_seller);
            self.items.remove(doc_buyer);
            return rv as i32;
        }
        if let Err(rv) = self.internal_start_trade(partner, player, doc_buyer, None) {
            self.cancel_trade_for_player(player);
            self.items.remove(doc_seller);
            self.items.remove(doc_buyer);
            return rv as i32;
        }
        ReturnValue::NoError as i32
    }

    fn trade_item_accessible(&self, cid: CreatureId, item_id: ItemId) -> bool {
        let Some(item) = self.items.get(item_id) else {
            return false;
        };
        match item.parent {
            Some(Cylinder::Inventory { player_id, .. }) => player_id == cid,
            Some(Cylinder::Container { item_id: cont, .. }) => {
                self.get_container_owner(cont) == Some(cid)
                    || self.trade_item_accessible(cid, cont)
            }
            Some(Cylinder::Tile { pos }) => self
                .creatures
                .get(cid)
                .is_some_and(|k| are_in_range_1_1_0(k.position(), pos)),
            None => true,
        }
    }

    fn count_trade_objects(&self, item_id: ItemId) -> u32 {
        1 + self
            .trade_child_items(item_id)
            .into_iter()
            .map(|c| self.count_trade_objects(c))
            .sum::<u32>()
    }

    fn trade_child_items(&self, item_id: ItemId) -> Vec<ItemId> {
        self.container_registry
            .get(item_id)
            .map(|c| c.items.clone())
            .unwrap_or_default()
    }

    fn trade_contains(&self, inner: ItemId, outer: ItemId) -> bool {
        if inner == outer {
            return true;
        }
        let mut cur = inner;
        for _ in 0..64 {
            let Some(item) = self.items.get(cur) else {
                return false;
            };
            match item.parent {
                Some(Cylinder::Container { item_id, .. }) if item_id == outer => return true,
                Some(Cylinder::Container { item_id, .. }) => cur = item_id,
                _ => return false,
            }
        }
        false
    }

    fn item_held_by_player(&self, item_id: ItemId, cid: CreatureId) -> bool {
        self.trade_item_accessible(cid, item_id)
            && !matches!(
                self.items.get(item_id).and_then(|i| i.parent),
                Some(Cylinder::Tile { .. })
            )
    }

    fn complete_item_weight(&self, item_id: ItemId) -> u32 {
        let Some(item) = self.items.get(item_id) else {
            return 0;
        };
        let base = self
            .items_db
            .items
            .get(&item.item_type)
            .map(|t| t.weight.saturating_mul(u32::from(item.count.max(1))))
            .unwrap_or(0);
        let nested = self
            .container_registry
            .get(item_id)
            .map(|c| c.total_weight)
            .unwrap_or(0);
        base.saturating_add(nested)
    }

    fn flatten_trade_items(&self, root: ItemId) -> Vec<ItemTemplateArgs> {
        let mut out = Vec::new();
        let mut q = VecDeque::from([root]);
        while let Some(id) = q.pop_front() {
            if let Some(args) = self.item_template_args(id) {
                out.push(args);
            }
            if let Some(c) = self.container_registry.get(id) {
                for &child in &c.items {
                    q.push_back(child);
                }
            }
        }
        out
    }

    fn item_template_args(&self, item_id: ItemId) -> Option<ItemTemplateArgs> {
        let item = self.items.get(item_id)?;
        let it = self.items_db.items.get(&item.item_type)?;
        Some(ItemTemplateArgs {
            client_id: it.client_id,
            count: item.wire_count_byte(it),
            stackable: it.stackable(),
            is_splash_or_fluid: it.is_splash() || it.is_fluid_container(),
            is_animation: it.is_animation(),
            with_description: false,
        })
    }

    fn close_trade_pair(&mut self, a: CreatureId, b: CreatureId, _message: bool) {
        self.trades.sides.remove(&a);
        self.trades.sides.remove(&b);
        self.send_close_trade_to(a);
        self.send_close_trade_to(b);
    }

    fn send_trade_offer(&mut self, cid: CreatureId, name: &str, own_offer: bool, item_id: ItemId) {
        let Some(conn) = self.conn_for_creature(cid) else {
            return;
        };
        let items = self.flatten_trade_items(item_id);
        let pkt = self
            .codec
            .encode_trade_item_request(name, own_offer, &items);
        self.enqueue_encoded(conn, pkt);
    }

    fn send_close_trade_to(&mut self, cid: CreatureId) {
        let Some(conn) = self.conn_for_creature(cid) else {
            return;
        };
        let pkt = self.codec.encode_close_trade();
        self.enqueue_encoded(conn, pkt);
    }

    fn send_trade_failure(&mut self, cid: CreatureId, text: &str) {
        let Some(conn) = self.conn_for_creature(cid) else {
            return;
        };
        let ty = self.codec.failure_message_type();
        self.enqueue_outgoing(conn, send_text_message_simple(ty, text).into_bytes());
    }

    fn send_trade_info(&mut self, cid: CreatureId, text: &str) {
        let Some(conn) = self.conn_for_creature(cid) else {
            return;
        };
        self.enqueue_outgoing(
            conn,
            send_text_message_simple(MESSAGE_INFO_DESCR, text).into_bytes(),
        );
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::Container;
    use crate::sim_harness::{
        ensure_walkable_tile, insert_player, minimal_world, test_player,
    };
    use slotmap::Key;
    use tfs_rust_common::ConnId;

    fn equip_backpack(world: &mut GameWorld, cid: CreatureId) {
        let bp = world.items.insert(Item::new_single(1987));
        world
            .internal_add_item_to_inventory_slot(cid, InventorySlot::Backpack as u8, bp)
            .expect("backpack slot");
        let mut reg = std::mem::take(&mut world.container_registry);
        reg.register(Container::new(bp, 20));
        world.container_registry = reg;
    }
    fn two_traders() -> (GameWorld, CreatureId, CreatureId, ItemId, ItemId) {
        let mut world = minimal_world();
        let a_pos = Position::new(100, 100, 7);
        let b_pos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, a_pos, 100);
        ensure_walkable_tile(&mut world.map, b_pos, 100);
        let mut pa = test_player("Alice", a_pos);
        pa.guid = 10;
        let mut pb = test_player("Bob", b_pos);
        pb.guid = 11;
        let a = insert_player(&mut world, pa);
        let b = insert_player(&mut world, pb);
        world.register_conn_mapping(ConnId(1), a);
        world.register_conn_mapping(ConnId(2), b);
        let gold_a = world.items.insert(Item::new(2148, 1));
        let gold_b = world.items.insert(Item::new(2148, 1));
        world
            .internal_add_item_to_tile(a_pos, gold_a, crate::cylinder::CylinderFlags::NONE)
            .expect("place a");
        world
            .internal_add_item_to_tile(b_pos, gold_b, crate::cylinder::CylinderFlags::NONE)
            .expect("place b");
        (world, a, b, gold_a, gold_b)
    }

    #[test]
    fn first_offer_locks_only_initiator() {
        let (mut world, a, b, gold_a, _) = two_traders();
        world
            .internal_start_trade(a, b, gold_a, None)
            .expect("offer");
        assert!(world.trades.sides.contains_key(&a));
        assert!(!world.trades.sides.contains_key(&b));
    }

    #[test]
    fn counter_offer_opens_both() {
        let (mut world, a, b, gold_a, gold_b) = two_traders();
        world.internal_start_trade(a, b, gold_a, None).unwrap();
        world.internal_start_trade(b, a, gold_b, None).unwrap();
        assert_eq!(world.trades.sides.get(&a).unwrap().partner, b);
        assert_eq!(world.trades.sides.get(&b).unwrap().partner, a);
    }

    #[test]
    fn reject_clears_partner() {
        let (mut world, a, b, gold_a, gold_b) = two_traders();
        world.internal_start_trade(a, b, gold_a, None).unwrap();
        world.internal_start_trade(b, a, gold_b, None).unwrap();
        world.player_close_trade(a);
        assert!(world.trades.sides.is_empty());
    }

    #[test]
    fn moving_offered_item_cancels() {
        let (mut world, a, b, gold_a, _) = two_traders();
        world.internal_start_trade(a, b, gold_a, None).unwrap();
        let dest = Position::new(100, 101, 7);
        ensure_walkable_tile(&mut world.map, dest, 100);
        world
            .internal_move_item(
                Some(a),
                Cylinder::Tile {
                    pos: Position::new(100, 100, 7),
                },
                Cylinder::Tile { pos: dest },
                gold_a,
                1,
                crate::cylinder::CylinderFlags::NONE,
                None,
            )
            .expect("move");
        world.notify_trades(gold_a);
        assert!(world.trades.sides.is_empty());
    }

    #[test]
    fn too_far_rejected() {
        let mut world = minimal_world();
        let a_pos = Position::new(100, 100, 7);
        let b_pos = Position::new(110, 100, 7);
        ensure_walkable_tile(&mut world.map, a_pos, 100);
        ensure_walkable_tile(&mut world.map, b_pos, 100);
        let a = insert_player(&mut world, test_player("A", a_pos));
        let b = insert_player(&mut world, test_player("B", b_pos));
        let gold = world.items.insert(Item::new(2148, 1));
        world
            .internal_add_item_to_tile(a_pos, gold, crate::cylinder::CylinderFlags::NONE)
            .unwrap();
        let rv = world.internal_start_trade(a, b, gold, None).unwrap_err();
        assert_eq!(rv, ReturnValue::TooFarAway);
    }

    #[test]
    fn inspect_root_index_zero() {
        let (mut world, a, b, gold_a, _) = two_traders();
        world.internal_start_trade(a, b, gold_a, None).unwrap();
        assert_eq!(world.inspect_trade(a, true, 0), Some(gold_a));
    }

    #[test]
    fn inspect_nested_container_index_one() {
        let (mut world, a, b, _, _) = two_traders();
        let bag = world.items.insert(Item::new_single(1987));
        let coin = world.items.insert(Item::new(2148, 5));
        {
            let mut reg = std::mem::take(&mut world.container_registry);
            reg.register(Container::new(bag, 20));
            reg.get_mut(bag).unwrap().add_item(coin).expect("coin in bag");
            world.container_registry = reg;
        }
        if let Some(item) = world.items.get_mut(coin) {
            item.parent = Some(Cylinder::Container {
                item_id: bag,
                index: 0,
            });
        }
        world
            .internal_add_item_to_tile(
                Position::new(100, 100, 7),
                bag,
                crate::cylinder::CylinderFlags::NONE,
            )
            .expect("bag on tile");
        world.internal_start_trade(a, b, bag, None).unwrap();
        assert_eq!(world.inspect_trade(a, true, 0), Some(bag));
        assert_eq!(world.inspect_trade(a, true, 1), Some(coin));
    }

    #[test]
    fn dual_accept_swaps_items() {
        let (mut world, a, b, gold_a, gold_b) = two_traders();
        equip_backpack(&mut world, a);
        equip_backpack(&mut world, b);
        world.internal_start_trade(a, b, gold_a, None).unwrap();
        world.internal_start_trade(b, a, gold_b, None).unwrap();
        world.player_accept_trade(a);
        world.player_accept_trade(b);
        assert!(world.trades.sides.is_empty());
        assert!(world.trade_item_accessible(b, gold_a));
        assert!(world.trade_item_accessible(a, gold_b));
        assert!(!world.trade_item_accessible(a, gold_a));
        assert!(!world.trade_item_accessible(b, gold_b));
    }

    #[test]
    fn house_dual_accept_transfers_owner() {
        let (mut world, a, b, _, _) = two_traders();
        world.houses.ensure_houses([1]);
        world.houses.set_owner(1, 10);
        let seller = a.data().as_ffi() as u64;
        let buyer = b.data().as_ffi() as u64;
        assert_eq!(world.lua_house_start_trade(1, seller, buyer), 0);
        world.player_accept_trade(a);
        world.player_accept_trade(b);
        assert_eq!(
            world.houses.houses.get(&1).and_then(|h| h.owner_guid),
            Some(11)
        );
        assert!(world.trades.sides.is_empty());
    }
}
