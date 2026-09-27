//! Hotkey use of carried runes and items from client protocol 780.
//!
//! - 772: `CheckSpecialCoordinates` rejects `INVENTORY_ANY` (`y == 0`) on use
//!   (`receiving.cc:29-39`). `CUseOnCreature` drops a player target, which
//!   includes yourself (`receiving.cc:512-514`).
//! - From 780: `(0xFFFF, 0, 0)` plus a sprite id is a carried-item search
//!   (`Game::internalGetThing`, `game.cpp`). Use-on-creature may target
//!   monsters, players, and yourself. The status line is
//!   `Actions::showUseHotkeyMessage` (`actions.cpp`).

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::lua_scope::is_hotkey_use_position;
use tfs_rust_common::{ConnId, Position, ProtocolVersion};
use tfs_rust_net::codec::client_color_to_fluid_800;
use tfs_rust_net::item_encode::client_fluid_to_server;
use tfs_rust_net::outgoing_extra::send_text_message_simple;

/// `TALK_INFO_MESSAGE` (`enums.hh:673`). Same byte as TFS `MESSAGE_INFO_DESCR`.
const TALK_INFO_MESSAGE: u8 = 22;

/// 772 drops the packet. From 780 the target is legal, including yourself.
pub(crate) fn use_on_creature_dropped(
    hotkey_object_use: bool,
    from_is_hotkey: bool,
    target_is_player: bool,
) -> bool {
    if hotkey_object_use {
        false
    } else {
        from_is_hotkey || target_is_player
    }
}

/// `Actions::showUseHotkeyMessage` (`actions.cpp`).
pub(crate) fn hotkey_use_text(name: &str, plural: &str, count: u32, show_count: bool) -> String {
    if !show_count {
        format!("Using one of {name}...")
    } else if count == 1 {
        format!("Using the last {name}...")
    } else {
        format!("Using one of {count} {plural}...")
    }
}

impl GameWorld {
    pub(crate) fn hotkey_use_enabled(&self) -> bool {
        self.codec.caps().hotkey_object_use
    }

    /// Silent drop, matching `receiving.cc` (no `SendResult`).
    pub(crate) fn drop_use_on_creature(&self, from: Position, target: CreatureId) -> bool {
        let target_is_player = self
            .creatures
            .get(target)
            .is_some_and(|k| matches!(k, CreatureKind::Player(_)));
        use_on_creature_dropped(
            self.hotkey_use_enabled(),
            is_hotkey_use_position(from),
            target_is_player,
        )
    }

    /// Carried item for a hotkey sprite. `None` below protocol 780.
    pub(crate) fn resolve_hotkey_use_item(
        &self,
        cid: CreatureId,
        sprite_id: u16,
        stack_pos: u8,
    ) -> Option<ItemId> {
        if !self.hotkey_use_enabled() {
            return None;
        }
        let server_id = self
            .items_db
            .items
            .contains_key(&sprite_id)
            .then_some(sprite_id)?;
        let sub_type = self
            .items_db
            .items
            .get(&server_id)
            .filter(|it| it.is_fluid_container())
            .map(|_| {
                let fluid = if self.codec.version() == ProtocolVersion::V800 {
                    client_color_to_fluid_800(stack_pos)
                } else {
                    client_fluid_to_server(stack_pos)
                };
                i32::from(fluid)
            })
            .unwrap_or(-1);
        self.find_item_of_type(cid, server_id, true, sub_type)
    }

    /// Status line before the use consumes a charge or a stack count.
    pub(crate) fn notify_hotkey_use(&mut self, cid: CreatureId, item_id: ItemId) {
        if !self.hotkey_use_enabled() {
            return;
        }
        let Some(conn_id) = self.conn_for_creature(cid) else {
            return;
        };
        let Some(text) = self.hotkey_use_line(cid, item_id) else {
            return;
        };
        self.enqueue_hotkey_text(conn_id, &text);
    }

    fn hotkey_use_line(&self, cid: CreatureId, item_id: ItemId) -> Option<String> {
        let item_type = self.items.get(item_id)?.item_type;
        let it = self.items_db.items.get(&item_type)?;
        let count = if it.is_rune() {
            self.rune_charge_total(cid, item_type)
        } else {
            let sub_type = if it.is_fluid_container() {
                i32::from(self.items.get(item_id)?.get_sub_type(it))
            } else {
                -1
            };
            self.player_get_item_type_count(cid, item_type, sub_type)
        };
        Some(hotkey_use_text(
            &it.name,
            &it.get_plural_name(),
            count,
            it.show_count,
        ))
    }

    fn rune_charge_total(&self, cid: CreatureId, item_type: u16) -> u32 {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return 0;
        };
        let mut total = 0u32;
        for slot in crate::inventory::PLAYER_INVENTORY_SLOT_FIRST
            ..=crate::inventory::PLAYER_INVENTORY_SLOT_LAST
        {
            let Some(slot_item) = p.equipment_slots[(slot - 1) as usize] else {
                continue;
            };
            total = total.saturating_add(self.rune_charges_in(slot_item, item_type));
        }
        total
    }

    fn rune_charges_in(&self, root: ItemId, item_type: u16) -> u32 {
        let mut total = 0u32;
        let mut pending = vec![root];
        let mut i = 0usize;
        while i < pending.len() {
            let id = pending[i];
            i += 1;
            let Some(item) = self.items.get(id) else {
                continue;
            };
            if item.item_type == item_type {
                // 8.0 draws this count on the rune (`multiCharge`). `charges()` can
                // be the type default from `seed_charges_from_type_if_missing`
                // (2) while the remaining uses sit in `count` (94).
                total = total.saturating_add(u32::from(item.count.max(1)));
            }
            if self.items_db.is_container(item.item_type)
                && let Some(cont) = self.container_registry.get(id)
            {
                pending.extend(cont.items.iter().copied());
            }
        }
        total
    }

    fn enqueue_hotkey_text(&mut self, conn_id: ConnId, text: &str) {
        self.enqueue_outgoing(
            conn_id,
            send_text_message_simple(TALK_INFO_MESSAGE, text).into_bytes(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_on_creature_drop_follows_version() {
        assert!(use_on_creature_dropped(false, true, false));
        assert!(use_on_creature_dropped(false, false, true));
        assert!(!use_on_creature_dropped(false, false, false));
        assert!(!use_on_creature_dropped(true, true, true));
    }

    #[test]
    fn hotkey_text_matches_show_use_hotkey_message() {
        assert_eq!(
            hotkey_use_text("rope", "ropes", 1, false),
            "Using one of rope..."
        );
        assert_eq!(
            hotkey_use_text("sudden death rune", "sudden death runes", 1, true),
            "Using the last sudden death rune..."
        );
        assert_eq!(
            hotkey_use_text("sudden death rune", "sudden death runes", 42, true),
            "Using one of 42 sudden death runes..."
        );
    }
}
