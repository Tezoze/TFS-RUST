//! Lua-based event dispatcher implementation.
//!
//! C++ reference: `src/movement.cpp` `MoveEvents::onPlayerEquip`, `MoveEvent::fireEquip`.

use std::collections::HashMap;

use crate::event_dispatcher::EventDispatcher;
use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use slotmap::Key;
use tfs_rust_lua::{
    CreatureEventType, CallbackRef, LuaRuntime, MoveEventKind, MoveEventsRegistry,
    PlayerEventType, with_lua_context,
};

/// Lua-based event dispatcher.
pub struct LuaEventDispatcher {
    runtime: LuaRuntime,
    creature_events: HashMap<CreatureEventType, Vec<CallbackRef>>,
    player_events: HashMap<PlayerEventType, Vec<CallbackRef>>,
    move_events: MoveEventsRegistry,
}

impl LuaEventDispatcher {
    pub fn new(
        runtime: LuaRuntime,
        creature_events: HashMap<CreatureEventType, Vec<CallbackRef>>,
        player_events: HashMap<PlayerEventType, Vec<CallbackRef>>,
        move_events: MoveEventsRegistry,
    ) -> Self {
        Self {
            runtime,
            creature_events,
            player_events,
            move_events,
        }
    }

    fn slot_mask_for_slot(slot: u8) -> u32 {
        match slot {
            1 => 1 << 0,
            2 => 1 << 1,
            3 => 1 << 2,
            4 => 1 << 3,
            5 => 1 << 4,
            6 => 1 << 5,
            7 => 1 << 6,
            8 => 1 << 7,
            9 => 1 << 8,
            10 => 1 << 9,
            _ => 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_move_equip(
        &self,
        kind: MoveEventKind,
        player: CreatureId,
        item: ItemId,
        item_type: u16,
        slot: u8,
        player_level: u32,
        is_check: bool,
    ) -> ReturnValue {
        let Some(entry) = self.move_events.get(kind, item_type) else {
            return ReturnValue::NoError;
        };
        if entry.req_level > 0 && player_level < entry.req_level {
            return if is_check {
                ReturnValue::NotEnoughLevel
            } else {
                ReturnValue::NoError
            };
        }
        if entry.slot_mask != 0 {
            let slot_mask = Self::slot_mask_for_slot(slot);
            if entry.slot_mask & slot_mask == 0 {
                return ReturnValue::NoError;
            }
        }
        match self.runtime.call_move_equip(
            &entry.callback,
            player.data().as_ffi(),
            item.data().as_ffi(),
            slot,
            is_check,
        ) {
            Ok(true) => ReturnValue::NoError,
            Ok(false) => {
                if is_check {
                    ReturnValue::CannotBeDressed
                } else {
                    ReturnValue::NoError
                }
            }
            Err(e) => {
                tracing::error!(
                    ?player,
                    ?item,
                    item_type,
                    slot,
                    ?kind,
                    is_check,
                    "MoveEvent equip Lua failed: {e}"
                );
                ReturnValue::NoError
            }
        }
    }
}

impl EventDispatcher for LuaEventDispatcher {
    fn on_player_equip_check(
        &self,
        player: CreatureId,
        item: ItemId,
        item_type: u16,
        slot: u8,
        player_level: u32,
    ) -> ReturnValue {
        self.dispatch_move_equip(
            MoveEventKind::Equip,
            player,
            item,
            item_type,
            slot,
            player_level,
            true,
        )
    }

    fn on_player_equip(
        &self,
        player: CreatureId,
        item: ItemId,
        item_type: u16,
        slot: u8,
        player_level: u32,
    ) {
        let _ = self.dispatch_move_equip(
            MoveEventKind::Equip,
            player,
            item,
            item_type,
            slot,
            player_level,
            false,
        );
    }

    fn on_player_deequip(
        &self,
        player: CreatureId,
        item: ItemId,
        item_type: u16,
        slot: u8,
        player_level: u32,
    ) {
        let _ = self.dispatch_move_equip(
            MoveEventKind::DeEquip,
            player,
            item,
            item_type,
            slot,
            player_level,
            false,
        );
    }

    fn on_player_inventory_update(
        &self,
        player: CreatureId,
        item: ItemId,
        slot: u8,
        equip: bool,
    ) {
        let Some(callbacks) = self
            .player_events
            .get(&PlayerEventType::InventoryUpdate)
        else {
            return;
        };
        for callback in callbacks {
            if let Err(e) = self.runtime.call_player_inventory_update(
                callback,
                player.data().as_ffi(),
                item.data().as_ffi(),
                slot,
                equip,
            ) {
                tracing::error!(
                    ?player,
                    ?item,
                    slot,
                    equip,
                    "Lua onInventoryUpdate failed: {e}"
                );
            }
        }
    }

    fn on_login(&self, creature: CreatureId, ctx: &dyn tfs_rust_common::ScriptContext) {
        with_lua_context(ctx, || {
            if let Some(callbacks) = self.creature_events.get(&CreatureEventType::Login) {
                for callback in callbacks {
                    match self
                        .runtime
                        .call_creature_callback(callback, creature.data().as_ffi())
                    {
                        Ok(true) => {}
                        Ok(false) => {
                            tracing::warn!("Lua onLogin returned false for {:?}", creature);
                        }
                        Err(e) => {
                            tracing::error!("Lua onLogin callback failed for {:?}: {}", creature, e);
                        }
                    }
                }
            }
        });
    }

    fn on_logout(&self, creature: CreatureId, ctx: &dyn tfs_rust_common::ScriptContext) {
        with_lua_context(ctx, || {
            if let Some(callbacks) = self.creature_events.get(&CreatureEventType::Logout) {
                for callback in callbacks {
                    match self
                        .runtime
                        .call_creature_callback(callback, creature.data().as_ffi())
                    {
                        Ok(true) => {}
                        Ok(false) => {
                            tracing::warn!("Lua onLogout returned false for {:?}", creature);
                        }
                        Err(e) => {
                            tracing::error!("Lua onLogout callback failed for {:?}: {}", creature, e);
                        }
                    }
                }
            }
        });
    }
}
