//! Container UI protocol (0x6E–0x72) — `protocolgame.cpp` `sendContainer`, etc.
// C++ ref: `ProtocolGame::sendContainer`, `sendAddContainerItem`, `sendUpdateContainerItem`,
//          `sendRemoveContainerItem`, `sendCloseContainer`.

use std::collections::VecDeque;
use std::time::Instant;

use tfs_rust_common::game_packet::{UseItemExPayload, UseItemPayload};
use tfs_rust_common::ConnId;
use tfs_rust_common::Position;
use tfs_rust_net::codec::{ContainerOpenWire, ItemTemplateArgs};
use tfs_rust_net::outgoing_extra::{send_close_container, send_remove_container_item_empty};

use crate::creature::PlayerWalkAction;
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::item_look::look_distance_tfs;
use crate::return_value::ReturnValue;

/// How to sync the client container window — `Player::onAddContainerItem` / full refresh (`player.cpp`).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) enum ContainerContentChange {
    /// Full `sendContainer` (0x6E) — open, seek, up, or unknown bulk change.
    #[default]
    FullRefresh,
    /// `sendAddContainerItem` (0x70) — new item at `slot` (visible index).
    Add { slot: u16 },
    /// `sendUpdateContainerItem` (0x71).
    Update { slot: u16 },
    /// `sendRemoveContainerItem` (0x72).
    Remove { slot: u16 },
}

#[derive(Clone)]
struct ChildItemWire {
    client_id: u16,
    count: u8,
    stackable: bool,
    splash_nf: bool,
    anim: bool,
}

impl GameWorld {
    /// Enqueue full `sendContainer` (0x6E) for one connection.
    // C++ ref: `protocolgame.cpp` `ProtocolGame::sendContainer`
    pub(crate) fn send_container_open_to_player(
        &mut self,
        conn_id: ConnId,
        viewer: CreatureId,
        client_cid: u8,
        container_item_id: ItemId,
        first_index: u16,
    ) {
        let Some(bytes) = self.build_container_open_packet(viewer, client_cid, container_item_id, first_index) else {
            return;
        };
        self.enqueue_outgoing(conn_id, bytes);
    }

    /// Push 0x6E to every player that has `container_item_id` open (each with their own client cid).
    // C++ ref: `Player::onSendContainer` — refresh all viewers (`player.cpp`).
    pub(crate) fn refresh_container_ui_for_all_viewers(&mut self, container_item_id: ItemId) {
        let Some(cont) = self.container_registry.get(container_item_id) else {
            return;
        };
        let triples: Vec<(CreatureId, u8, u16)> = cont
            .open_by
            .iter()
            .filter_map(|&pl| {
                let client_cid = self.container_registry.get_cid_for_container(pl, container_item_id)?;
                let fi = self
                    .container_registry
                    .get_container_first_index(pl, client_cid)
                    .unwrap_or(0);
                Some((pl, client_cid, fi))
            })
            .collect();
        for (pl, client_cid, fi) in triples {
            let Some(conn) = self.conn_id_for_creature(pl) else {
                continue;
            };
            self.send_container_open_to_player(conn, pl, client_cid, container_item_id, fi);
        }
    }

    fn item_with_description_flag(&self, viewer: CreatureId) -> bool {
        self.creatures
            .get(viewer)
            .and_then(|k| match k {
                CreatureKind::Player(p) => Some(p.item_with_description()),
                _ => None,
            })
            .unwrap_or(false)
    }

    /// Per-slot delta to one viewer (`0x70`/`0x71`/`0x72`).
    fn enqueue_container_slot_delta(
        &mut self,
        viewer: CreatureId,
        conn_id: ConnId,
        client_cid: u8,
        change: ContainerContentChange,
        container_item_id: ItemId,
    ) {
        let with_desc = self.item_with_description_flag(viewer);
        match change {
            ContainerContentChange::FullRefresh => {}
            ContainerContentChange::Remove { slot } => {
                let pkt = send_remove_container_item_empty(client_cid, slot).into_bytes();
                self.enqueue_outgoing(conn_id, pkt);
            }
            ContainerContentChange::Add { slot } => {
                self.enqueue_container_add_or_update_slot(
                    conn_id,
                    client_cid,
                    slot,
                    container_item_id,
                    with_desc,
                    true,
                );
            }
            ContainerContentChange::Update { slot } => {
                self.enqueue_container_add_or_update_slot(
                    conn_id,
                    client_cid,
                    slot,
                    container_item_id,
                    with_desc,
                    false,
                );
            }
        }
    }

    fn enqueue_container_add_or_update_slot(
        &mut self,
        conn_id: ConnId,
        client_cid: u8,
        slot: u16,
        container_item_id: ItemId,
        with_desc: bool,
        is_add: bool,
    ) {
        let Some(iid) = self
            .container_registry
            .get(container_item_id)
            .and_then(|c| c.get_item(slot as usize))
        else {
            self.refresh_container_ui_for_all_viewers(container_item_id);
            return;
        };
        let Some(ch) = self.items.get(iid) else {
            self.refresh_container_ui_for_all_viewers(container_item_id);
            return;
        };
        let ch_sid = ch.item_type;
        let ccid = self.items_db.client_id_for_server(ch_sid);
        if ccid == 0 {
            self.refresh_container_ui_for_all_viewers(container_item_id);
            return;
        }
        let ccnt = ch.client_count().max(1);
        let cstack = self.items_db.items.get(&ch_sid).map(|t| t.stackable()).unwrap_or(false);
        let csplash = self.items_db.is_splash_or_fluid_for_server(ch_sid);
        let canim = self.items_db.is_animation_for_server(ch_sid);
        let args = ItemTemplateArgs {
            client_id: ccid,
            count: ccnt,
            stackable: cstack,
            is_splash_or_fluid: csplash,
            is_animation: canim,
            with_description: with_desc,
        };
        let pkt = if is_add {
            self.codec.encode_add_container_item(client_cid, slot, args)
        } else {
            self.codec
                .encode_update_container_item(client_cid, slot, args)
        };
        self.enqueue_outgoing(conn_id, pkt.into_bytes());
    }

    /// TFS `Player::autoCloseContainers` / visibility — close windows the player can no longer interact with.
    pub(crate) fn auto_close_containers_for_player(&mut self, viewer: CreatureId) {
        let entries = self.container_registry.open_container_entries(viewer);
        let to_close: Vec<u8> = entries
            .into_iter()
            .filter(|(_, root_id)| !self.player_may_view_open_container_window(viewer, *root_id))
            .map(|(ccid, _)| ccid)
            .collect();
        let Some(conn) = self.conn_id_for_creature(viewer) else {
            return;
        };
        for client_cid in to_close {
            let _ = self.container_registry.close_container_for_player(viewer, client_cid);
            self.send_close_container_packet(conn, client_cid);
        }
    }

    /// Close open windows whose chain includes `container_item_id` — `Player::autoCloseContainers` (`player.cpp`).
    pub(crate) fn auto_close_containers_for_container_item(
        &mut self,
        viewer: CreatureId,
        container_item_id: ItemId,
    ) {
        let entries = self.container_registry.open_container_entries(viewer);
        let to_close: Vec<u8> = entries
            .into_iter()
            .filter(|(_, root_id)| {
                *root_id == container_item_id
                    || self
                        .container_registry
                        .get(*root_id)
                        .is_some_and(|c| c.is_holding_item(&self.container_registry, container_item_id))
            })
            .map(|(ccid, _)| ccid)
            .collect();
        let Some(conn) = self.conn_id_for_creature(viewer) else {
            return;
        };
        for client_cid in to_close {
            let _ = self.container_registry.close_container_for_player(viewer, client_cid);
            self.send_close_container_packet(conn, client_cid);
        }
    }

    /// Whether `viewer` may keep a window open on `container_root` (held in inventory or sees map tile).
    // C++ ref: `Player::autoCloseContainers`, `Thing::getTile` (`player.cpp`, `thing.h`).
    fn player_may_view_open_container_window(
        &self,
        viewer: CreatureId,
        container_root: ItemId,
    ) -> bool {
        let top = self.top_container_item_id(container_root);
        if self.player_holds_container_tree(viewer, top) {
            return true;
        }
        if self.player_owns_depot_container_tree(viewer, top) {
            return true;
        }
        if let Some(pos) = self.map.find_item_position(top) {
            return self.can_see_position(viewer, pos);
        }
        false
    }

    /// If the open container chain is carried by a player, refresh weight/light/stats via TopParent notify.
    pub(crate) fn notify_container_owner_carry_weight(&mut self, container_item_id: ItemId) {
        let top = self.top_container_item_id(container_item_id);
        let holders: Vec<(CreatureId, ItemId)> = self
            .creatures
            .iter()
            .filter_map(|(cid, k)| {
                if matches!(k, CreatureKind::Player(_)) && self.player_holds_container_tree(cid, top) {
                    Some((cid, top))
                } else {
                    None
                }
            })
            .collect();
        for (cid, root) in holders {
            self.notify_player_container_tree_changed(
                cid,
                root,
                container_item_id,
                true,
                crate::player_inventory_notifications::NotificationParent::Player,
            );
        }
    }

    fn build_container_open_packet(
        &self,
        viewer: CreatureId,
        client_cid: u8,
        container_item_id: ItemId,
        first_index: u16,
    ) -> Option<Vec<u8>> {
        let cont = self.container_registry.get(container_item_id)?;
        let container_wrapped = self.items.get(container_item_id)?;
        let sid = container_wrapped.item_type;
        let it = self.items_db.items.get(&sid)?;
        let name = it.name.clone();
        let client_id_hdr = self.items_db.client_id_for_server(sid);
        if client_id_hdr == 0 {
            return None;
        }
        let cnt = container_wrapped.client_count().max(1);
        let stackable = self.items_db.stackable_for_server(sid);
        let splash = self.items_db.is_splash_or_fluid_for_server(sid);
        let anim = self.items_db.is_animation_for_server(sid);
        let with_desc = self
            .creatures
            .get(viewer)
            .and_then(|k| match k {
                CreatureKind::Player(p) => Some(p.item_with_description()),
                _ => None,
            })
            .unwrap_or(false);

        let capacity = cont.capacity.min(255) as u8;
        let has_parent = cont.parent_container.is_some();
        let unlocked = cont.unlocked;
        let total_items = cont.items.len() as u16;
        let pagination = cont.pagination;
        let first = first_index.min(total_items);
        let remain = total_items.saturating_sub(first);
        let n_show = remain.min(u16::from(capacity)) as u8;

        let child_items: Vec<ItemId> = cont
            .items
            .iter()
            .skip(first as usize)
            .take(n_show as usize)
            .copied()
            .collect();

        let mut children: Vec<ChildItemWire> = Vec::with_capacity(child_items.len());
        for iid in &child_items {
            let Some(ch) = self.items.get(*iid) else {
                continue;
            };
            let ch_sid = ch.item_type;
            let ccid = self.items_db.client_id_for_server(ch_sid);
            if ccid == 0 {
                continue;
            }
            let ccnt = ch.client_count().max(1);
            let cstack = self.items_db.items.get(&ch_sid).map(|t| t.stackable()).unwrap_or(false);
            let csplash = self.items_db.is_splash_or_fluid_for_server(ch_sid);
            let canim = self.items_db.is_animation_for_server(ch_sid);
            children.push(ChildItemWire {
                client_id: ccid,
                count: ccnt,
                stackable: cstack,
                splash_nf: csplash && !cstack,
                anim: canim,
            });
        }

        let wire = ContainerOpenWire {
            cid: client_cid,
            header_item: ItemTemplateArgs {
                client_id: client_id_hdr,
                count: cnt,
                stackable,
                is_splash_or_fluid: splash && !stackable,
                is_animation: anim,
                with_description: with_desc,
            },
            name,
            capacity,
            has_parent,
            unlocked,
            pagination,
            total_size: total_items,
            first_index: first,
            items: children
                .into_iter()
                .map(|ch| ItemTemplateArgs {
                    client_id: ch.client_id,
                    count: ch.count,
                    stackable: ch.stackable,
                    is_splash_or_fluid: ch.splash_nf,
                    is_animation: ch.anim,
                    with_description: with_desc,
                })
                .collect(),
        };
        Some(self.codec.encode_container_open(&wire).into_bytes())
    }

    /// After any change to container contents, sync clients (`0x6E` or per-slot `0x70`–`0x72`).
    pub(crate) fn notify_container_content_changed(
        &mut self,
        container_item_id: ItemId,
        change: ContainerContentChange,
    ) {
        match change {
            ContainerContentChange::FullRefresh => {
                self.refresh_container_ui_for_all_viewers(container_item_id);
            }
            ContainerContentChange::Add { .. }
            | ContainerContentChange::Update { .. }
            | ContainerContentChange::Remove { .. } => {
                let Some(cont) = self.container_registry.get(container_item_id) else {
                    return;
                };
                let viewers: Vec<CreatureId> = cont.open_by.clone();
                for pl in viewers {
                    let Some(client_cid) = self.container_registry.get_cid_for_container(pl, container_item_id) else {
                        continue;
                    };
                    let Some(conn) = self.conn_id_for_creature(pl) else {
                        continue;
                    };
                    self.enqueue_container_slot_delta(
                        pl,
                        conn,
                        client_cid,
                        change,
                        container_item_id,
                    );
                }
            }
        }
        self.notify_container_owner_carry_weight(container_item_id);
    }

    /// Enqueue `sendCloseContainer` (0x6F) for one player.
    pub(crate) fn send_close_container_packet(&mut self, conn_id: ConnId, client_cid: u8) {
        self.enqueue_outgoing(conn_id, send_close_container(client_cid).into_bytes());
    }

    /// Resolve `UseItem` / look-ups: inventory slot (`0xFFFF` + slot in `y`).
    // C++ ref: `Game::internalGetCylinder` inventory branch (`game.cpp`).
    pub(crate) fn item_id_for_inventory_use(&self, cid: CreatureId, slot: u8) -> Option<ItemId> {
        self.get_player_inventory_item(cid, slot)
    }

    /// Resolve item on map tile for `UseItem` (`STACKPOS_USEITEM` / `Tile::getUseItem`).
    // C++ ref: `Game::internalGetThing` + `Tile::getUseItem` (`game.cpp`, `tile.cpp`).
    pub(crate) fn item_id_for_tile_use(&self, pos: Position, stack_pos: u8) -> Option<ItemId> {
        let tile = self.map.get_tile(pos)?;
        tile.item_id_for_use(stack_pos, |item_id| {
            self.items
                .get(item_id)
                .map(|i| self.items_db.is_container(i.item_type))
                .unwrap_or(false)
        })
    }

    /// Fallback when client `stack_pos` does not resolve but `sprite_id` matches a tile item.
    pub(crate) fn find_tile_item_by_client_sprite(
        &self,
        pos: Position,
        sprite_id: u16,
    ) -> Option<ItemId> {
        let tile = self.map.get_tile(pos)?;
        let body = tile.body();
        body
            .down_items
            .iter()
            .chain(body.top_items.iter()).find(|&&item_id| self.validate_item_sprite(item_id, sprite_id)).copied()
    }

    /// Match client sprite id to `ItemId` when multiple items could match (validates `sprite_id`).
    pub(crate) fn validate_item_sprite(&self, item_id: ItemId, sprite_id: u16) -> bool {
        let Some(item) = self.items.get(item_id) else {
            return false;
        };
        self.items_db.client_id_for_server(item.item_type) == sprite_id
    }

    /// Resolve `Position` + stack to an item instance for `UseItem` / `UseItemEx`.
    // C++ ref: `Game::internalGetThing` (`game.cpp`).
    pub(crate) fn resolve_item_at_position(
        &self,
        cid: CreatureId,
        pos: Position,
        stack_pos: u8,
    ) -> Option<ItemId> {
        if pos.x != 0xFFFF {
            return self.item_id_for_tile_use(pos, stack_pos);
        }
        if pos.y & 0x40 != 0 {
            let client_cid = (pos.y & 0x0F) as u8;
            let slot = pos.z as usize;
            let container_id = self.container_registry.get_container_by_cid(cid, client_cid)?;
            let co = self.container_registry.get(container_id)?;
            return co.items.get(slot).copied();
        }
        self.item_id_for_inventory_use(cid, pos.y as u8)
    }

    /// `Game::playerUseItem` — open container when item is a bag (`actions.cpp` container branch).
    pub fn player_use_item(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        payload: UseItemPayload,
        now: Instant,
    ) {
        // C++ `internalGetCylinder`: map tile when `pos.x != 0xFFFF` — `game.cpp` ~199.
        // `pos.y & 0x40` is container encoding only when x is 0xFFFF, not a map-tile test.
        let is_map_tile = payload.pos.x != 0xFFFF;
        if !self.player_timed_action_ready(cid, now) {
            // C++ `createSchedulerTask(delay, playerUseItem)` when `!canDoAction` (`game.cpp` ~2246).
            self.defer_player_walk_action(cid, PlayerWalkAction::UseItem(payload.clone()), now);
            return;
        }
        let item_id = if let Some(id) =
            self.resolve_item_at_position(cid, payload.pos, payload.stack_pos)
        {
            Some(id)
        } else if is_map_tile {
            self.find_tile_item_by_client_sprite(payload.pos, payload.sprite_id)
        } else {
            None
        };
        let Some(item_id) = item_id else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };
        if !self.validate_item_sprite(item_id, payload.sprite_id) {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        }
        if is_map_tile {
            let Some(pp) = self.creatures.get(cid).map(|k| k.position()) else {
                self.send_cancel_message(conn_id, ReturnValue::NotPossible);
                return;
            };
            if look_distance_tfs(pp, payload.pos) > 1 {
                let action = PlayerWalkAction::UseItem(payload.clone());
                if !self.try_walk_to_and_action(conn_id, cid, payload.pos, action, now) {
                    self.send_cancel_message(conn_id, ReturnValue::ThereIsNoWay);
                }
                return;
            }
        }
        let preferred_cid = (payload.index < crate::container::MAX_CONTAINER_WINDOWS)
            .then_some(payload.index);
        let item_type = self.items.get(item_id).map(|i| i.item_type).unwrap_or(0);
        if is_map_tile && crate::floor_change_use::is_teleport_floor_use_item(item_type) {
            let dest = crate::floor_change_use::resolve_teleport_use_destination(
                self,
                cid,
                item_type,
                payload.pos,
            );
            let ret = crate::walk::internal_teleport_player(self, conn_id, cid, dest);
            if ret != ReturnValue::NoError {
                self.send_cancel_message(conn_id, ret);
            }
            return;
        }
        self.try_open_container_for_item(conn_id, cid, item_id, preferred_cid);
    }

    /// Use-with: if the source item is a container, open it (minimal parity until full use-with).
    pub fn player_use_item_ex(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        payload: UseItemExPayload,
        now: Instant,
    ) {
        let is_map_tile = payload.from_pos.x != 0xFFFF;
        if !self.player_timed_action_ready(cid, now) {
            self.defer_player_walk_action(cid, PlayerWalkAction::UseItemEx(payload.clone()), now);
            return;
        }
        let item_id = if let Some(id) =
            self.resolve_item_at_position(cid, payload.from_pos, payload.from_stack_pos)
        {
            Some(id)
        } else if is_map_tile {
            self.find_tile_item_by_client_sprite(payload.from_pos, payload.from_sprite_id)
        } else {
            None
        };
        let Some(item_id) = item_id else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };
        if !self.validate_item_sprite(item_id, payload.from_sprite_id) {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        }
        if is_map_tile {
            let Some(pp) = self.creatures.get(cid).map(|k| k.position()) else {
                self.send_cancel_message(conn_id, ReturnValue::NotPossible);
                return;
            };
            if look_distance_tfs(pp, payload.from_pos) > 1 {
                let action = PlayerWalkAction::UseItemEx(payload.clone());
                if !self.try_walk_to_and_action(conn_id, cid, payload.from_pos, action, now) {
                    self.send_cancel_message(conn_id, ReturnValue::ThereIsNoWay);
                }
                return;
            }
        }
        // `UseItemEx` has no index byte; new window uses client-chosen cid via normal `UseItem`.
        self.try_open_container_for_item(conn_id, cid, item_id, None);
    }

    /// C++ `Actions::internalUseItem` container branch — toggle if already open; else `addContainer(index, ...)`.
    fn try_open_container_for_item(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        item_id: ItemId,
        preferred_cid: Option<u8>,
    ) {
        let item_type = self.items.get(item_id).map(|i| i.item_type).unwrap_or(0);
        if self.items_db.is_depot(item_type) {
            let fallback_town = self
                .creatures
                .get(cid)
                .and_then(|k| match k {
                    CreatureKind::Player(p) => Some(p.town_id),
                    _ => None,
                })
                .unwrap_or(0);
            let depot_id = self.depot_id_from_locker_item(item_id, fallback_town);
            let Some(locker_id) = self.player_get_depot_locker(cid, depot_id) else {
                self.send_cancel_message(conn_id, ReturnValue::NotPossible);
                return;
            };
            self.player_set_last_depot_id(cid, depot_id);
            if let Some(open_cid) = self.container_registry.get_cid_for_container(cid, locker_id) {
                let _ = self.container_registry.close_container_for_player(cid, open_cid);
                self.send_close_container_packet(conn_id, open_cid);
                return;
            }
            let mut reg = std::mem::take(&mut self.container_registry);
            self.ensure_container_registered_simple(&mut reg, locker_id, cid);
            self.container_registry = reg;
            let Some(client_cid) = self
                .container_registry
                .add_container(cid, locker_id, preferred_cid, 0)
            else {
                self.send_cancel_message(conn_id, ReturnValue::NotPossible);
                return;
            };
            self.send_container_open_to_player(conn_id, cid, client_cid, locker_id, 0);
            return;
        }

        let Some(item) = self.items.get(item_id) else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };
        if !self.items_db.is_openable_container(item.item_type) {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        }

        if let Some(open_cid) = self.container_registry.get_cid_for_container(cid, item_id) {
            let _ = self.container_registry.close_container_for_player(cid, open_cid);
            self.send_close_container_packet(conn_id, open_cid);
            return;
        }

        let mut reg = std::mem::take(&mut self.container_registry);
        self.ensure_container_registered_simple(&mut reg, item_id, cid);
        self.container_registry = reg;

        let Some(client_cid) =
            self.container_registry
                .add_container(cid, item_id, preferred_cid, 0)
        else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };
        self.send_container_open_to_player(conn_id, cid, client_cid, item_id, 0);
    }

    /// `Game::playerCloseContainer` (`game.cpp`).
    pub fn player_close_container(&mut self, conn_id: ConnId, cid: CreatureId, client_cid: u8) {
        if self.container_registry.get_container_by_cid(cid, client_cid).is_none() {
            return;
        }
        let _ = self.container_registry.close_container_for_player(cid, client_cid);
        self.send_close_container_packet(conn_id, client_cid);
    }

    /// `Game::playerMoveUpContainer` / up arrow — show parent bag in same window (`game.cpp`).
    pub fn player_up_container(&mut self, conn_id: ConnId, cid: CreatureId, client_cid: u8) {
        let Some(current_id) = self.container_registry.get_container_by_cid(cid, client_cid) else {
            return;
        };
        let Some(parent_id) = self
            .container_registry
            .get(current_id)
            .and_then(|c| c.parent_container)
        else {
            return;
        };
        let mut reg = std::mem::take(&mut self.container_registry);
        self.ensure_container_registered_simple(&mut reg, parent_id, cid);
        self.container_registry = reg;

        let Some(_) = self
            .container_registry
            .add_container(cid, parent_id, Some(client_cid), 0)
        else {
            self.send_cancel_message(conn_id, ReturnValue::NotPossible);
            return;
        };
        self.send_container_open_to_player(conn_id, cid, client_cid, parent_id, 0);
    }

    /// `Game::playerUpdateContainer` — full refresh (`game.cpp`).
    pub fn player_update_container(&mut self, conn_id: ConnId, cid: CreatureId, client_cid: u8) {
        let Some(root) = self.container_registry.get_container_by_cid(cid, client_cid) else {
            return;
        };
        let fi = self
            .container_registry
            .get_container_first_index(cid, client_cid)
            .unwrap_or(0);
        self.send_container_open_to_player(conn_id, cid, client_cid, root, fi);
    }

    /// `Game::playerSeekInContainer` — pagination (`game.cpp`).
    pub fn player_seek_in_container(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        client_cid: u8,
        first_index: u16,
    ) {
        let Some(root) = self.container_registry.get_container_by_cid(cid, client_cid) else {
            return;
        };
        let _ = self
            .container_registry
            .set_container_index(cid, client_cid, first_index);
        self.send_container_open_to_player(conn_id, cid, client_cid, root, first_index);
    }

    /// TFS `Player::autoOpenContainers` — after inventory loaded (`player.cpp`).
    pub(crate) fn auto_open_containers_on_login(&mut self, conn_id: ConnId, cid: CreatureId) {
        let equipment_roots: Vec<ItemId> = {
            let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
                return;
            };
            p.equipment_slots.iter().flatten().copied().collect()
        };
        let mut queue: VecDeque<ItemId> = VecDeque::new();
        for r in equipment_roots {
            if self.items_db.is_container(self.items.get(r).map(|i| i.item_type).unwrap_or(0)) {
                queue.push_back(r);
            }
        }
        while let Some(slot_item) = queue.pop_front() {
            let Some(item) = self.items.get(slot_item) else {
                continue;
            };
            if !self.items_db.is_container(item.item_type) {
                continue;
            }
            if let Some(c) = self.container_registry.get(slot_item) {
                for &ch in &c.items {
                    if self.items_db.is_container(self.items.get(ch).map(|i| i.item_type).unwrap_or(0)) {
                        queue.push_back(ch);
                    }
                }
            }
            if !item.attributes.as_deref().is_some_and(|a| a.has_auto_open()) {
                continue;
            }
            let saved_cid = item.attributes.as_deref().map(|a| a.get_auto_open()).unwrap_or(0);
            if saved_cid >= crate::container::MAX_CONTAINER_WINDOWS {
                continue;
            }
            let mut reg = std::mem::take(&mut self.container_registry);
            self.ensure_container_registered_simple(&mut reg, slot_item, cid);
            self.container_registry = reg;
            let Some(ccid) = self
                .container_registry
                .add_container(cid, slot_item, Some(saved_cid), 0)
            else {
                continue;
            };
            self.send_container_open_to_player(conn_id, cid, ccid, slot_item, 0);
        }
    }
}
