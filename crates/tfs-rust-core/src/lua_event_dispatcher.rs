//! Lua-based event dispatcher implementation.
//!
//! This module provides LuaEventDispatcher which implements EventDispatcher
//! by dispatching events to Lua callbacks.

use std::collections::HashMap;

use crate::event_dispatcher::EventDispatcher;
use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use slotmap::Key;
use tfs_rust_lua::{
    CreatureEventType, CallbackRef, LuaRuntime, PlayerEventType, with_lua_context,
};

/// Lua-based event dispatcher.
///
/// Owns the LuaRuntime and maps event types to registered Lua callbacks.
pub struct LuaEventDispatcher {
    runtime: LuaRuntime,
    creature_events: HashMap<CreatureEventType, Vec<CallbackRef>>,
    player_events: HashMap<PlayerEventType, Vec<CallbackRef>>,
}

impl LuaEventDispatcher {
    pub fn new(
        runtime: LuaRuntime,
        creature_events: HashMap<CreatureEventType, Vec<CallbackRef>>,
        player_events: HashMap<PlayerEventType, Vec<CallbackRef>>,
    ) -> Self {
        Self {
            runtime,
            creature_events,
            player_events,
        }
    }
}

impl EventDispatcher for LuaEventDispatcher {
    fn on_player_equip_check(
        &self,
        player: CreatureId,
        item: ItemId,
        slot: u8,
    ) -> ReturnValue {
        tracing::trace!(?player, ?item, slot, "LuaEventDispatcher::on_player_equip_check");
        ReturnValue::NoError
    }

    fn on_player_equip(&self, player: CreatureId, item: ItemId, slot: u8) {
        tracing::trace!(?player, ?item, slot, "LuaEventDispatcher::on_player_equip");
    }

    fn on_player_deequip(&self, player: CreatureId, item: ItemId, slot: u8) {
        tracing::trace!(?player, ?item, slot, "LuaEventDispatcher::on_player_deequip");
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

    // Other methods: no-op until Track 2
}
