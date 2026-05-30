//! Lua / script event surface injected into `GameWorld`.
//!
//! `EventDispatcher` uses [`tfs_rust_common::ScriptContext`] — not `tfs-rust-lua` — so
//! core's event trait stays lua-crate-agnostic (one-way: core → lua at wiring time only).
// C++ reference: `CreatureEvent::dispatch`, `LuaScriptInterface` hooks.

use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use tfs_rust_common::ScriptContext;

/// Script and engine events. Default bodies are no-ops until `tfs-rust-lua` implements dispatch.
pub trait EventDispatcher {
    fn on_login(&self, creature: CreatureId, ctx: &dyn ScriptContext) {}
    fn on_logout(&self, creature: CreatureId, ctx: &dyn ScriptContext) {}
    fn on_think(&self, creature: CreatureId, interval_ms: u32) {}
    fn on_prepare_death(&self, creature: CreatureId) {}
    fn on_death(&self, creature: CreatureId) {}
    fn on_kill(&self, killer: CreatureId, target: CreatureId) {}
    /// TFS `Creature::onWalkComplete` — walk queue empty after `getNextStep` false (`src/creature.cpp` ~215–219).
    fn on_walk_complete(&self, creature: CreatureId) {}
    fn on_advance(&self, creature: CreatureId, skill: u8, old_level: u32, new_level: u32) {}
    fn on_startup(&self) {}
    fn on_shutdown(&self) {}
    /// Spread LuaJIT GC across ticks (Phase 4 game loop). No-op without Lua.
    fn lua_gc_step(&self) {}
    /// TFS `MoveEvents::onPlayerEquip` with `isCheck == true` — `player.cpp` `queryAdd`.
    fn on_player_equip_check(
        &self,
        _player: CreatureId,
        _item: ItemId,
        _item_type: u16,
        _slot: u8,
        _player_level: u32,
    ) -> ReturnValue {
        ReturnValue::NoError
    }
    /// TFS `MoveEvent::onPlayerEquip` — `player.cpp` `postAddNotification` (`g_moveEvents->onPlayerEquip`).
    fn on_player_equip(
        &self,
        _player: CreatureId,
        _item: ItemId,
        _item_type: u16,
        _slot: u8,
        _player_level: u32,
    ) {
    }
    /// TFS `MoveEvent::onPlayerDeEquip` — `postRemoveNotification` (`g_moveEvents->onPlayerDeEquip`).
    fn on_player_deequip(
        &self,
        _player: CreatureId,
        _item: ItemId,
        _item_type: u16,
        _slot: u8,
        _player_level: u32,
    ) {
    }
    /// TFS `Events::eventPlayerOnInventoryUpdate` — `player.cpp` `postAddNotification` / `postRemoveNotification`.
    fn on_player_inventory_update(
        &self,
        _player: CreatureId,
        _item: ItemId,
        _slot: u8,
        _equip: bool,
    ) {
    }
}

/// Default no-op dispatcher for tests and early wiring.
#[derive(Debug, Default)]
pub struct NullEventDispatcher;

impl EventDispatcher for NullEventDispatcher {}
