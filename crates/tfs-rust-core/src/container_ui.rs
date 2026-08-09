//! Container UI protocol (`0x6E`–`0x72`) — TFS-style domain, idiomatic Rust.
//! Domain: `src/protocolgame.cpp` `sendContainer` / `sendAddContainerItem` / `sendUpdateContainerItem`
//!         / `sendRemoveContainerItem` / `sendCloseContainer`.
//! 772 outcomes: `sending.cc:696-798` `SendContainer` / `SendChangeInContainer` / `SendDeleteInContainer`,
//!              `moveuse.cc:1536` `UseContainer`, `receiving.cc:609` `CUpContainer`,
//!              `operate.cc:128` `AnnounceChangedContainer` / `:1060` `CloseContainer`.

use std::collections::VecDeque;

use slotmap::Key;
use tfs_rust_common::ConnId;
use tfs_rust_common::Position;
use tfs_rust_net::codec::{ContainerOpenWire, ItemTemplateArgs};
use tfs_rust_net::outgoing_extra::send_close_container;

use crate::creature::CreatureKind;
use crate::cylinder::Cylinder;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use crate::walk::are_in_range_1_1_0;

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
        let Some(bytes) =
            self.build_container_open_packet(viewer, client_cid, container_item_id, first_index)
        else {
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
                let client_cid = self
                    .container_registry
                    .get_cid_for_container(pl, container_item_id)?;
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
                let pkt = self
                    .codec
                    .encode_remove_container_item(client_cid, slot)
                    .into_bytes();
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
        let cstack = self
            .items_db
            .items
            .get(&ch_sid)
            .map(|t| t.stackable())
            .unwrap_or(false);
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
            let _ = self
                .container_registry
                .close_container_for_player(viewer, client_cid);
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
                    || self.container_registry.get(*root_id).is_some_and(|c| {
                        c.is_holding_item(&self.container_registry, container_item_id)
                    })
            })
            .map(|(ccid, _)| ccid)
            .collect();
        let Some(conn) = self.conn_id_for_creature(viewer) else {
            return;
        };
        for client_cid in to_close {
            let _ = self
                .container_registry
                .close_container_for_player(viewer, client_cid);
            self.send_close_container_packet(conn, client_cid);
        }
    }

    /// 772 `CloseContainer(Con, true)` for `CreatureID == 0` — close every open window that
    /// references `container_item_id` (as a root or nested container) for every current viewer.
    pub(crate) fn auto_close_all_containers_for_item(&mut self, container_item_id: ItemId) {
        let Some(cont) = self.container_registry.get(container_item_id) else {
            return;
        };
        let viewers: Vec<CreatureId> = cont.open_by.clone();
        for viewer in viewers {
            self.auto_close_containers_for_container_item(viewer, container_item_id);
        }
    }

    /// Whether `viewer` may keep a window open on `container_root` (held in inventory or adjacent map tile).
    // C++ ref: `Player::postAddNotification` walk check — `Position::areInRange<1,1,0>`
    // (`player.cpp` ~3117-3127). Depot lockers are per-player virtual containers with no
    // tile, so `getPosition()` returns nullptr_tile (0xFFFF,0xFFFF,0xFF) — always out of
    // range, so they close on walk (matches TFS).
    fn player_may_view_open_container_window(
        &self,
        viewer: CreatureId,
        container_root: ItemId,
    ) -> bool {
        let top = self.top_container_item_id(container_root);
        if self.player_holds_container_tree(viewer, top) {
            return true;
        }
        let Some(viewer_pos) = self.creatures.get(viewer).map(|k| k.position()) else {
            return false;
        };
        // O(1) via `Item.parent` — C++ `Thing::getPosition` / parent cylinder walk.
        // Never `map.find_item_position` here: that is a full-world tile scan and runs on
        // every player step while a ground corpse/container window is open (`walk/mod.rs`
        // → `auto_close_containers_for_player`).
        // Depot containers have no tile parent → `script_item_position` returns None → closed.
        if let Some(pos) = self.script_item_position(top) {
            return are_in_range_1_1_0(viewer_pos, pos);
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
                if matches!(k, CreatureKind::Player(_))
                    && self.player_holds_container_tree(cid, top)
                {
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
        // 772 `HasUpContainer` is true whenever the container has a parent that is not a body slot
        // (`sending.cc:714`). A bag on the ground has a tile cylinder parent, so the up arrow shows.
        let parent_cyl = self.resolve_item_parent_cylinder(container_item_id);
        let has_parent =
            parent_cyl.is_some_and(|c| !matches!(c, Cylinder::Inventory { .. }));
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
            let cstack = self
                .items_db
                .items
                .get(&ch_sid)
                .map(|t| t.stackable())
                .unwrap_or(false);
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
                    let Some(client_cid) = self
                        .container_registry
                        .get_cid_for_container(pl, container_item_id)
                    else {
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

    /// Batch front-slot removals for expiry `Empty` — one viewer clone, one carry-weight notify.
    ///
    /// Outcomes: decompile `Empty(Con, Remainder)` walks from front (`operate.cc`).
    pub(crate) fn notify_container_front_removals(
        &mut self,
        container_item_id: ItemId,
        remove_count: usize,
    ) {
        if remove_count == 0 {
            return;
        }
        let Some(cont) = self.container_registry.get(container_item_id) else {
            return;
        };
        let viewers: Vec<CreatureId> = cont.open_by.clone();
        let change = ContainerContentChange::Remove { slot: 0 };
        for pl in viewers {
            let Some(client_cid) = self
                .container_registry
                .get_cid_for_container(pl, container_item_id)
            else {
                continue;
            };
            let Some(conn) = self.conn_id_for_creature(pl) else {
                continue;
            };
            for _ in 0..remove_count {
                self.enqueue_container_slot_delta(
                    pl,
                    conn,
                    client_cid,
                    change,
                    container_item_id,
                );
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
    ///
    /// `stack_pos` is an index into the tile as *that viewer* was told it, so the walk has to
    /// use the same order the tile description and `0x6A` / `0x6C` used for them.
    // C++ ref: `Game::internalGetThing` + `Tile::getUseItem` (`game.cpp`, `tile.cpp`).
    pub(crate) fn item_id_for_tile_use(
        &self,
        cid: CreatureId,
        pos: Position,
        stack_pos: u8,
    ) -> Option<ItemId> {
        let tile = self.map.get_tile(pos)?;
        tile.item_id_for_use(
            stack_pos,
            self.uses_cip_map_order(cid),
            |item_id| self.item_is_cip_priority_bottom(item_id),
            |item_id| {
                self.items
                    .get(item_id)
                    .map(|i| self.items_db.is_container(i.item_type))
                    .unwrap_or(false)
            },
        )
    }

    /// Items on `pos` in map-container chain order — the order 772 `GetObject` walks
    /// (`info.cc:412-419`) and the order the client stores the tile in.
    ///
    /// `PRIORITY_BOTTOM` downs (fields, pools) precede the always-on-top group; ordinary
    /// `PRIORITY_LOW` downs come last, newest first (`map.cc` `GetObjectPriority` /
    /// `PlaceObject`). Creatures are skipped — this yields items only.
    fn tile_items_in_chain_order(&self, pos: Position) -> Option<impl Iterator<Item = ItemId> + '_> {
        let body = self.map.get_tile(pos)?.body();
        let bottoms = body
            .down_items
            .iter()
            .rev()
            .copied()
            .filter(|&id| self.item_is_cip_priority_bottom(id));
        let lows = body
            .down_items
            .iter()
            .copied()
            .filter(|&id| !self.item_is_cip_priority_bottom(id));
        Some(bottoms.chain(body.top_items.iter().copied()).chain(lows))
    }

    /// 772 `GetObject` map branch (`info.cc:412-419`): the client's `RNum` is never an index
    /// into the tile — the chain is walked from the head and matched on the client `TypeID`.
    /// Also the fallback when a `stack_pos` walk misses.
    pub(crate) fn find_tile_item_by_client_sprite(
        &self,
        pos: Position,
        sprite_id: u16,
    ) -> Option<ItemId> {
        self.tile_items_in_chain_order(pos)?
            .find(|&item_id| self.validate_item_sprite(item_id, sprite_id))
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
            return self.item_id_for_tile_use(cid, pos, stack_pos);
        }
        if pos.y & 0x40 != 0 {
            let client_cid = (pos.y & 0x0F) as u8;
            let slot = pos.z as usize;
            let container_id = self
                .container_registry
                .get_container_by_cid(cid, client_cid)?;
            let co = self.container_registry.get(container_id)?;
            return co.items.get(slot).copied();
        }
        self.item_id_for_inventory_use(cid, pos.y as u8)
    }

    /// 772 `GetObject` (`info.cc:398-431`) — resolve a Use / Turn / MultiUse object reference
    /// from the wire triple (position, stackpos, client `TypeID`).
    ///
    /// On a map tile 772 matches the `TypeID` along the object chain and never treats `RNum`
    /// as an index (`info.cc:412-419`), so a real 7.72 client needs no stackpos agreement at
    /// all. TVP / OTClient / 1098 do send a meaningful stackpos, so they keep the TFS
    /// `Tile::getUseItem` walk and use the sprite scan only as a fallback.
    ///
    /// Does **not** resolve bare ground — ground has no SlotMap [`ItemId`]. Use
    /// [`Self::resolve_ground_use_type`] for `UseItem` on the bank / rope-hole floor.
    pub(crate) fn resolve_use_object(
        &self,
        cid: CreatureId,
        pos: Position,
        stack_pos: u8,
        sprite_id: u16,
    ) -> Option<ItemId> {
        if pos.x == 0xFFFF {
            return self.resolve_item_at_position(cid, pos, stack_pos);
        }
        if self.uses_cip_map_order(cid) {
            return self
                .find_tile_item_by_client_sprite(pos, sprite_id)
                .or_else(|| self.item_id_for_tile_use(cid, pos, stack_pos));
        }
        self.item_id_for_tile_use(cid, pos, stack_pos)
            .or_else(|| self.find_tile_item_by_client_sprite(pos, sprite_id))
    }

    /// Resolve bare **ground** for single-object Use (`CUseObject`).
    ///
    /// Rust stores ground as `Option<u16>` only ([`crate::tile::Tile::item_id_for_use`]),
    /// so Use must accept it without an `ItemId`. Match is **TypeID-only** like 772
    /// `GetObject` (`info.cc:412–419`): walk would find the bank by `getDisguise() == Type`;
    /// `RNum` / stackpos is not an index. Wrong TypeID → `None` → enqueue `NotPossible`
    /// (no walk).
    pub(crate) fn resolve_ground_use_type(
        &self,
        pos: Position,
        _stack_pos: u8,
        sprite_id: u16,
    ) -> Option<u16> {
        if pos.x == 0xFFFF {
            return None;
        }
        let ground = self.map.get_tile(pos)?.body().ground?;
        let client_id = self.items_db.client_id_for_server(ground);
        if sprite_id == client_id || sprite_id == ground {
            return Some(ground);
        }
        None
    }

    /// C++ `Actions::internalUseItem` house door pre-check — `Door::canUse` (`actions.cpp` ~304,
    /// `house.cpp` ~535). Non-house tiles and non-door items pass. Deny → `NOTPOSSIBLE`
    /// ("Sorry, not possible.").
    pub(crate) fn house_door_can_use_or_deny(
        &self,
        cid: CreatureId,
        item_id: ItemId,
    ) -> Result<(), ReturnValue> {
        let item = self.items.get(item_id).ok_or(ReturnValue::NotPossible)?;
        let is_door = self
            .items_db
            .items
            .get(&item.item_type)
            .is_some_and(|t| t.is_door());
        if !is_door {
            return Ok(());
        }
        let pos = match item.parent {
            Some(Cylinder::Tile { pos }) => pos,
            _ => return Ok(()),
        };
        let house_id = match self.map.get_tile(pos) {
            Some(crate::tile::Tile::House(h)) => h.house_id,
            _ => return Ok(()), // Door without house → canUse true
        };
        let door_id = item
            .attributes
            .as_ref()
            .map(|a| a.get_door_id())
            .unwrap_or(0);
        let guid = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.guid,
            _ => return Err(ReturnValue::NotPossible),
        };
        let can_edit = self.player_has_flag(
            cid,
            crate::player_flags::PLAYER_FLAG_CAN_EDIT_HOUSES,
        );
        if self
            .houses
            .door_can_use(house_id, door_id, guid, can_edit)
        {
            Ok(())
        } else {
            Err(ReturnValue::NotPossible)
        }
    }

    /// F8 S5 — core use-item logic for the ToDo execute arm ([`execute_player_use`]).
    /// Skips the ready check + walk-to-reach (the ToDo arm handles adjacency via
    /// `Go`-prepend and timing via `Wait{100}` + `CalculateDelay`). C++ ref:
    /// `actions.cpp` container branch + `game.cpp` teleport-floor-use (`~2227`).
    pub(crate) fn player_use_item_core(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        item_id: ItemId,
        is_map_tile: bool,
        pos: Position,
        preferred_cid: Option<u8>,
    ) -> Result<(), ReturnValue> {
        // Native house door gate before Lua (`actions.cpp` `internalUseItem` → `Door::canUse`).
        if let Err(ret) = self.house_door_can_use_or_deny(cid, item_id) {
            return Err(ret);
        }

        // Action `onUse` before native teleport / container (`actions.cpp` `internalUseItem`).
        let from = if is_map_tile {
            pos
        } else {
            self.script_item_position(item_id).unwrap_or(pos)
        };
        if crate::lua_scope::fire_on_use_action(
            self,
            cid,
            item_id,
            from,
            Some(item_id),
            None,
            from,
        ) {
            return Ok(());
        }

        let item_type = self.items.get(item_id).map(|i| i.item_type).unwrap_or(0);
        if is_map_tile && crate::floor_change_use::is_teleport_floor_use_item(item_type) {
            let dest = crate::floor_change_use::resolve_teleport_use_destination(
                self, cid, item_type, pos,
            );
            let ret = crate::walk::internal_teleport_player(self, conn_id, cid, dest, false);
            if ret != ReturnValue::NoError {
                return Err(ret);
            }
            return Ok(());
        }
        self.try_open_container_for_item(conn_id, cid, item_id, preferred_cid);
        Ok(())
    }

    /// Single-object Use aimed at **bare ground** (no SlotMap item) — rope holes / floor
    /// teleports whose type lives in `body.ground`.
    ///
    /// Decompile: no `USEEVENT` / failed use → `NOTUSABLE` → "You cannot use this object."
    /// (`moveuse.cc` `UseObject`; `sending.cc`). Teleport types keep the floor-change path.
    pub(crate) fn player_use_ground_core(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        ground_type: u16,
        pos: Position,
    ) -> Result<(), ReturnValue> {
        if crate::floor_change_use::is_teleport_floor_use_item(ground_type) {
            let dest = crate::floor_change_use::resolve_teleport_use_destination(
                self,
                cid,
                ground_type,
                pos,
            );
            let ret = crate::walk::internal_teleport_player(self, conn_id, cid, dest, false);
            if ret != ReturnValue::NoError {
                return Err(ret);
            }
            return Ok(());
        }
        Err(ReturnValue::CannotUseThisObject)
    }

    /// F8 S5 — core two-object use logic for the ToDo execute arm ([`execute_player_use`]).
    /// Skips the ready check + walk-to-reach (ToDo arm handles those). On success,
    /// sets multiuse exhaustion via `player_apply_multiuse_exhaust` (`cract.cc:765`).
    ///
    /// PC-3a Gap 6: if `item_id` is a registered rune, dispatch `onCastSpell` instead
    /// of container open (`spells.cpp` `RuneSpell::playerCastRune`).
    pub(crate) fn player_use_item_ex_core(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        item_id: ItemId,
        target: crate::creature_todo::ActionObjectRef,
    ) -> Result<(), ReturnValue> {
        let item_type = self.items.get(item_id).map(|i| i.item_type).unwrap_or(0);
        if let Some(rune) = self.spells.runes_by_id.get(&item_type).cloned() {
            return self.player_cast_rune(conn_id, cid, item_id, &rune, target);
        }

        // Action use-with after rune miss (`actions.cpp` `useItemEx` → `executeUse`).
        let from = self
            .script_item_position(item_id)
            .unwrap_or(Position::new(0, 0, 0));
        let to = target.pos;
        let target_item =
            self.resolve_use_object(cid, target.pos, target.stack_pos, target.sprite_id);
        let target_creature = if target_item.is_none() {
            self.resolve_creature_at_action_target(cid, target)
        } else {
            None
        };
        if crate::lua_scope::fire_on_use_action(
            self,
            cid,
            item_id,
            from,
            target_item,
            target_creature,
            to,
        ) {
            self.player_apply_multiuse_exhaust(cid);
            return Ok(());
        }

        self.try_open_container_for_item(conn_id, cid, item_id, None);
        self.player_apply_multiuse_exhaust(cid);
        Ok(())
    }

    /// PC-3a Gap 6: rune use-with → Lua `onCastSpell`.
    /// C++ `RuneSpell::castSpell` / `playerCastRune` — `spells.cpp`.
    fn player_cast_rune(
        &mut self,
        _conn_id: ConnId,
        cid: CreatureId,
        item_id: ItemId,
        rune: &tfs_rust_content::spells::RuneSpellDef,
        target: crate::creature_todo::ActionObjectRef,
    ) -> Result<(), ReturnValue> {
        // `needTarget` → creature variant; else position (AoE / floor runes like GFB).
        // C++ `Spell::playerRuneCheck` — `spells.cpp:743-746`: miss → cancel + CONST_ME_POFF.
        // Cancel text is sent by `apply_todo_result_catch` on the ToDo path.
        let target_creature = if rune.need_target {
            self.resolve_creature_at_action_target(cid, target)
        } else {
            None
        };
        if rune.need_target && target_creature.is_none() {
            if let Some(pos) = self.creatures.get(cid).map(|k| k.position()) {
                self.broadcast_magic_effect(pos, 3u8); // CONST_ME_POFF
            }
            return Err(ReturnValue::CanOnlyUseThisRuneOnCreatures);
        }
        // C++ `Spell::playerRuneSpellCheck` range arm (`spells.cpp:719–722`):
        // `range != -1 && !canThrowObjectTo(..., range, range)` → DESTINATIONOUTOFREACH.
        // `range > 0` only — Default/`-1` skip; Chebyshev ≤ range (LOS left for throw path).
        if rune.range > 0 {
            let Some(caster_pos) = self.creatures.get(cid).map(|k| k.position()) else {
                return Err(ReturnValue::NotPossible);
            };
            let to = target.pos;
            if to.x != 0xFFFF {
                let dx = (caster_pos.x as i32 - to.x as i32).unsigned_abs();
                let dy = (caster_pos.y as i32 - to.y as i32).unsigned_abs();
                if dx > rune.range as u32 || dy > rune.range as u32 {
                    if let Some(pos) = self.creatures.get(cid).map(|k| k.position()) {
                        self.broadcast_magic_effect(pos, 3u8);
                    }
                    return Err(ReturnValue::DestinationOutOfReach);
                }
            }
        }
        let target_pos = if target_creature.is_none() {
            Some((target.pos.x, target.pos.y, target.pos.z))
        } else {
            None
        };
        let ok = crate::lua_scope::fire_on_cast_rune(
            self,
            rune.rune_id,
            cid,
            target_creature,
            target_pos,
        );
        if !ok {
            if let Some(pos) = self.creatures.get(cid).map(|k| k.position()) {
                self.broadcast_magic_effect(pos, 3u8);
            }
            return Err(ReturnValue::NotPossible);
        }
        // 772 `UseMagicItem` — `BlockLogout(60, false)` when Aggressive (`magic.cc:4304-4306`).
        // PZ-entry lock only if combat damage hits a player (`combat_execute_with_stimulus`).
        if rune.is_aggressive {
            self.player_block_logout_infight(cid, false);
        }
        // Consume one charge / count — TFS `transformItem` count-1.
        if let Some(item) = self.items.get_mut(item_id) {
            if item.count > 1 {
                item.count -= 1;
            } else {
                // Remove empty rune — best-effort via lua item remove path.
                let _ = self.lua_script_item_remove(item_id.data().as_ffi(), 1);
            }
        }
        self.player_apply_rune_exhaust(cid, rune);
        Ok(())
    }

    /// 772 Use multiuse (+1000) always; TFS `cooldownSpellTime` optionally bumps spell clock.
    pub(crate) fn player_apply_rune_exhaust(
        &mut self,
        cid: CreatureId,
        rune: &tfs_rust_content::spells::RuneSpellDef,
    ) {
        self.player_apply_multiuse_exhaust(cid);
        if rune.cooldown_spell_time {
            let delay = self.spell_exhaust_delay_ms(rune.cooldown);
            self.player_apply_spell_exhaust_ms(cid, delay);
        }
    }

    /// Resolve a creature at a use-with target (map tile stack or inventory skip).
    fn resolve_creature_at_action_target(
        &self,
        _caster: CreatureId,
        target: crate::creature_todo::ActionObjectRef,
    ) -> Option<CreatureId> {
        if target.pos.x == 0xFFFF {
            return None;
        }
        let tile = self.map.get_tile(target.pos)?;
        let body = tile.body();
        if body.creatures.is_empty() {
            return None;
        }
        // UseWithCreature synthesizes stack_pos=0 / sprite_id=0 — take first creature.
        // UseItemEx on a creature stack may still land on the tile's creature list.
        body.creatures.first().copied()
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
            if let Some(open_cid) = self
                .container_registry
                .get_cid_for_container(cid, locker_id)
            {
                let _ = self
                    .container_registry
                    .close_container_for_player(cid, open_cid);
                self.send_close_container_packet(conn_id, open_cid);
                return;
            }
            let mut reg = std::mem::take(&mut self.container_registry);
            self.ensure_container_registered_simple(&mut reg, locker_id, cid);
            self.container_registry = reg;
            let Some(client_cid) =
                self.container_registry
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
            let _ = self
                .container_registry
                .close_container_for_player(cid, open_cid);
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
        if self
            .container_registry
            .get_container_by_cid(cid, client_cid)
            .is_none()
        {
            return;
        }
        let _ = self
            .container_registry
            .close_container_for_player(cid, client_cid);
        self.send_close_container_packet(conn_id, client_cid);
    }

    /// `Game::playerMoveUpContainer` / up arrow — show parent bag or close when at the root.
    pub fn player_up_container(&mut self, conn_id: ConnId, cid: CreatureId, client_cid: u8) {
        let Some(current_id) = self
            .container_registry
            .get_container_by_cid(cid, client_cid)
        else {
            return;
        };
        // 772 `CUpContainer` (`receiving.cc:609`) walks up one cylinder. If the resolved parent is a
        // map tile or inventory slot (the root), the window is closed instead of opened.
        let parent_cyl = self.resolve_item_parent_cylinder(current_id);
        let Some(Cylinder::Container { item_id: parent_id, .. }) = parent_cyl else {
            self.send_close_container_packet(conn_id, client_cid);
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
        let Some(root) = self
            .container_registry
            .get_container_by_cid(cid, client_cid)
        else {
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
        let Some(root) = self
            .container_registry
            .get_container_by_cid(cid, client_cid)
        else {
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
            if self
                .items_db
                .is_container(self.items.get(r).map(|i| i.item_type).unwrap_or(0))
            {
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
                    if self
                        .items_db
                        .is_container(self.items.get(ch).map(|i| i.item_type).unwrap_or(0))
                    {
                        queue.push_back(ch);
                    }
                }
            }
            if !item
                .attributes
                .as_deref()
                .is_some_and(|a| a.has_auto_open())
            {
                continue;
            }
            let saved_cid = item
                .attributes
                .as_deref()
                .map(|a| a.get_auto_open())
                .unwrap_or(0);
            if saved_cid >= crate::container::MAX_CONTAINER_WINDOWS {
                continue;
            }
            let mut reg = std::mem::take(&mut self.container_registry);
            self.ensure_container_registered_simple(&mut reg, slot_item, cid);
            self.container_registry = reg;
            let Some(ccid) =
                self.container_registry
                    .add_container(cid, slot_item, Some(saved_cid), 0)
            else {
                continue;
            };
            self.send_container_open_to_player(conn_id, cid, ccid, slot_item, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::container::Container;
    use crate::cylinder::Cylinder;
    use crate::item::Item;
    use crate::sim_harness::{insert_player, minimal_world, test_player};
    use crate::tile::{Tile, TileBody};
    use tfs_rust_common::{ConnId, Position};

    /// 772/TVP `operate.cc` and TFS 1.4.2 `player.cpp` ~3119 close a ground container when the
    /// viewer is no longer adjacent (`Position::areInRange<1,1,0>`). The viewport visibility
    /// check (`can_see_position`) was too permissive and kept windows open across the map.
    #[test]
    fn ground_container_closes_when_player_moves_away() {
        let mut world = minimal_world();
        let container_pos = Position::new(100, 100, 7);
        let adjacent_pos = Position::new(101, 100, 7);
        let far_pos = Position::new(103, 100, 7);

        // Place a walkable tile with a ground container.
        let container_item_id = world.items.insert(Item::new(1987, 1));
        world.items.get_mut(container_item_id).unwrap().parent =
            Some(Cylinder::Tile { pos: container_pos });
        world
            .container_registry
            .register(Container::new(container_item_id, 20));
        world.map.insert_tile(
            container_pos,
            Tile::Normal(TileBody {
                ground: Some(100),

                ground_item: None,
                down_items: vec![container_item_id],
                ..TileBody::new()
            }),
        );

        // Insert a player next to the container and open it.
        let cid = insert_player(&mut world, test_player("Hero", adjacent_pos));
        let conn = ConnId(0);
        world.register_conn_mapping(conn, cid);
        let ccid = world
            .container_registry
            .add_container(cid, container_item_id, Some(0), 0)
            .expect("open container");
        assert!(world
            .container_registry
            .open_container_entries(cid)
            .iter()
            .any(|(open_cid, root)| *open_cid == ccid && *root == container_item_id));

        // Move two tiles away and run the auto-close sweep.
        world.creatures.get_mut(cid).unwrap().set_position(far_pos);
        world.auto_close_containers_for_player(cid);

        // The window should be closed.
        assert!(
            world
                .container_registry
                .open_container_entries(cid)
                .is_empty(),
            "ground container must close when player leaves adjacency"
        );
    }

    /// A ground container that is right under the player stays open.
    #[test]
    fn ground_container_stays_open_when_player_adjacent() {
        let mut world = minimal_world();
        let container_pos = Position::new(100, 100, 7);
        let adjacent_pos = Position::new(101, 100, 7);

        let container_item_id = world.items.insert(Item::new(1987, 1));
        world.items.get_mut(container_item_id).unwrap().parent =
            Some(Cylinder::Tile { pos: container_pos });
        world
            .container_registry
            .register(Container::new(container_item_id, 20));
        world.map.insert_tile(
            container_pos,
            Tile::Normal(TileBody {
                ground: Some(100),

                ground_item: None,
                down_items: vec![container_item_id],
                ..TileBody::new()
            }),
        );

        let cid = insert_player(&mut world, test_player("Hero", adjacent_pos));
        let conn = ConnId(0);
        world.register_conn_mapping(conn, cid);
        let ccid = world
            .container_registry
            .add_container(cid, container_item_id, Some(0), 0)
            .expect("open container");

        world.auto_close_containers_for_player(cid);

        assert!(
            world
                .container_registry
                .open_container_entries(cid)
                .iter()
                .any(|(open_cid, root)| *open_cid == ccid && *root == container_item_id),
            "ground container must stay open while player is adjacent"
        );
    }
}
