//! Lua / script event surface injected into `GameWorld`.
//!
//! `EventDispatcher` uses [`tfs_rust_common::ScriptContext`] — not `tfs-rust-lua` — so
//! core's event trait stays lua-crate-agnostic (one-way: core → lua at wiring time only).
// C++ reference: `CreatureEvent::dispatch`, `LuaScriptInterface` hooks.

use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use tfs_rust_common::Position;
use tfs_rust_common::ScriptContext;
use std::any::Any;

/// Talkaction dispatch result — mirrors C++ `TalkActionResult_t`
/// (`talkaction.h:13-17`).
///
/// - `NotMatched` — no talkaction matched the text; continue to spell check.
/// - `Continue` — talkaction matched, `onSay` returned `true` (C++
///   `TALKACTION_CONTINUE`); fall through to spell check / normal chat.
/// - `Break` — talkaction matched, `onSay` returned `false` (C++
///   `TALKACTION_BREAK`); text is consumed, do not broadcast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalkActionResult {
    NotMatched,
    Continue,
    Break,
}

/// Script and engine events. Default bodies are no-ops until `tfs-rust-lua` implements dispatch.
pub trait EventDispatcher {
    fn on_login(&self, _creature: CreatureId, _ctx: &dyn ScriptContext) {}
    fn on_logout(&self, _creature: CreatureId, _ctx: &dyn ScriptContext) {}
    fn on_think(&self, _creature: CreatureId, _interval_ms: u32) {}
    fn on_prepare_death(&self, _creature: CreatureId) {}
    fn on_death(&self, _creature: CreatureId) {}
    fn on_kill(&self, _killer: CreatureId, _target: CreatureId) {}
    /// TFS `Creature::onWalkComplete` — walk queue empty after `getNextStep` false (`src/creature.cpp` ~215–219).
    fn on_walk_complete(&self, _creature: CreatureId) {}
    fn on_advance(&self, _creature: CreatureId, _skill: u8, _old_level: u32, _new_level: u32) {}
    fn on_startup(&self) {}
    fn on_shutdown(&self) {}
    /// C++ `Events::eventMonsterOnSpawn` — default allow (`events.cpp`).
    fn on_monster_spawn(&self, _name: &str, _pos: Position, _startup: bool) -> bool {
        true
    }
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

    /// TFS `Creature::onCreatureSay` — per-creature hear callback (e.g. NPC dialog,
    /// creaturescript). C++ fires this for **every** spectator including the speaker
    /// (`game.cpp:3540`). Default no-op until the NPC/creaturescript Lua runtime lands.
    // C++ reference: `Creature::onCreatureSay` — `creature.cpp`; `Game::internalCreatureSay`
    // event-method loop — `gameserver/src/game.cpp:3538-3544`.
    fn on_creature_say(
        &self,
        _hearer: CreatureId,
        _speaker: CreatureId,
        _speak_type: u8,
        _text: &str,
    ) {
    }

    /// TFS `Events::eventCreatureOnHear` — script-side hear hook, **excludes self**
    /// (`creature != spectator`, `game.cpp:3541-3543`). Default no-op; the Lua
    /// creaturescript body is out of scope for the chat plan (§1 non-goals) but the
    /// call site is wired now so it doesn't need revisiting later.
    // C++ reference: `Events::eventCreatureOnHear` — `gameserver/src/game.cpp:3542`.
    fn on_hear(
        &self,
        _hearer: CreatureId,
        _speaker: CreatureId,
        _text: &str,
        _speak_type: u8,
    ) {
    }

    /// Execute a fired `addEvent` timer callback.
    ///
    /// C++ reference: `LuaEnvironment::executeTimerEvent` (`luascript.cpp:18238`).
    /// Called from the game loop when `GameCommand::LuaCallback { event_id }` arrives.
    /// Returns `true` if the event was found and executed.
    fn execute_timer_event(&self, _event_id: u64) -> bool {
        false
    }

    /// CH-6: Dispatch a talkaction (`/i`, `/a`, …) — `talkaction.cpp:84-134`
    /// `TalkActions::playerSaySpell`. Looks up `text` in the talkaction
    /// registry, and if matched, calls the `onSay` Lua callback. Returns
    /// [`TalkActionResult::NotMatched`] if no talkaction matched,
    /// [`TalkActionResult::Continue`] if `onSay` returned `true` (C++
    /// `TALKACTION_CONTINUE` — fall through to spells), or
    /// [`TalkActionResult::Break`] if `onSay` returned `false` (C++
    /// `TALKACTION_BREAK` — consumed).
    fn dispatch_talkaction(&self, _text: &str, _creature: CreatureId) -> TalkActionResult {
        TalkActionResult::NotMatched
    }

    /// Downcast to `Any` for runtime type checking (e.g., to access Lua runtime).
    fn as_any(&self) -> &dyn Any
    where
        Self: Sized;

    /// Downcast to `Any` for mutable runtime type checking (e.g., to inject
    /// the talkaction registry into `LuaEventDispatcher` after construction).
    fn as_any_mut(&mut self) -> &mut dyn Any
    where
        Self: Sized;
}

/// Default no-op dispatcher for tests and early wiring.
#[derive(Debug, Default)]
pub struct NullEventDispatcher;

impl EventDispatcher for NullEventDispatcher {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
