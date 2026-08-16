//! Lua / script event surface injected into `GameWorld`.
//!
//! `EventDispatcher` uses [`tfs_rust_common::ScriptContext`] — not `tfs-rust-lua` — so
//! core's event trait stays lua-crate-agnostic (one-way: core → lua at wiring time only).
// C++ reference: `CreatureEvent::dispatch`, `LuaScriptInterface` hooks.

use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use std::any::Any;
use tfs_rust_common::Position;
use tfs_rust_common::ScriptContext;
use tfs_rust_content::npcs::NpcCallbackId;

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
    /// Returns `false` to cancel logout (772/TFS `playerLogout` / `onLogout`).
    fn on_logout(&self, _creature: CreatureId, _ctx: &dyn ScriptContext) -> bool {
        true
    }
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

    /// TFS `MoveEvent::onRemoveItem` — `internal_move_item` SeparationEvent (OldCon != Con).
    fn on_remove_item(
        &self,
        _actor: Option<CreatureId>,
        _item: ItemId,
        _item_type: u16,
        _from: Position,
        _to: Position,
    ) -> bool {
        true
    }

    /// TFS `MoveEvent::onAddItem` — `internal_move_item` MovementEvent (OldCon != Con).
    fn on_add_item(
        &self,
        _actor: Option<CreatureId>,
        _item: ItemId,
        _item_type: u16,
        _from: Position,
        _to: Position,
    ) -> bool {
        true
    }

    /// TFS `MoveEvent::onStepOut` — creature/item leaving a tile.
    ///
    /// `pos` = tile being left; `from_pos` = creature last position (Lua 4th arg).
    /// `action_id` feeds `MoveEvents::getEvent` (uid skipped → aid → itemid).
    fn on_step_out(
        &self,
        _actor: Option<CreatureId>,
        _item: ItemId,
        _item_type: u16,
        _action_id: u16,
        _pos: Position,
        _from_pos: Position,
    ) -> bool {
        true
    }

    /// TFS `MoveEvent::onStepIn` — creature/item entering a tile.
    ///
    /// `pos` = tile entered; `from_pos` = creature last position (Lua 4th arg).
    /// `action_id` feeds `MoveEvents::getEvent` (uid skipped → aid → itemid).
    fn on_step_in(
        &self,
        _actor: Option<CreatureId>,
        _item: ItemId,
        _item_type: u16,
        _action_id: u16,
        _pos: Position,
        _from_pos: Position,
    ) -> bool {
        true
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
    fn on_hear(&self, _hearer: CreatureId, _speaker: CreatureId, _text: &str, _speak_type: u8) {}

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

    /// PC-3a: Dispatch a spell's `onCastSpell` Lua callback.
    ///
    /// C++ reference: `InstantSpell::castSpell` → `LuaEnvironment::callLuaFunction`
    /// (`spells.cpp` / `luascript.cpp:363` `getEvent`). The callback receives
    /// `(creature, variant)` and returns `true` on success.
    ///
    /// Returns `true` if the callback was found and executed successfully,
    /// `false` if no callback is registered for `spell_words` or the callback
    /// returned `false`.
    fn dispatch_on_cast_spell(
        &self,
        _spell_words: &str,
        _creature: CreatureId,
        _need_direction: bool,
        _has_param: bool,
        _param: &str,
    ) -> bool {
        false
    }

    /// PC-3a Gap 6: Dispatch a rune `onCastSpell` callback keyed by `rune:{id}`.
    ///
    /// `target_creature` is `Some` for `needTarget` runes (`VARIANT_NUMBER`);
    /// otherwise `target_pos` is used (`VARIANT_POSITION`).
    fn dispatch_on_cast_rune(
        &self,
        _rune_id: u16,
        _creature: CreatureId,
        _target_creature: Option<CreatureId>,
        _target_pos: Option<(u16, u16, u8)>,
    ) -> bool {
        false
    }

    /// Action `onUse` — `actions.cpp` `Action::executeUse`.
    ///
    /// Returns `true` if a script handled the use (skip native fallthrough).
    /// `item_type` / `action_id` drive `Actions::getAction` lookup (aid then type).
    /// `is_hotkey` is the 6th Lua arg (`callFunction(6)`).
    #[allow(clippy::too_many_arguments)]
    fn dispatch_on_use_action(
        &self,
        _player: CreatureId,
        _item: ItemId,
        _item_type: u16,
        _action_id: u16,
        _from: Position,
        _target_item: Option<ItemId>,
        _target_creature: Option<CreatureId>,
        _to: Position,
        _is_hotkey: bool,
    ) -> bool {
        false
    }

    /// TFS `Action::getAllowFarUse` — `actions.h`. ToDo Use Obj2 arm
    /// (`Actions::canExecuteAction` → `canUseFar`).
    fn action_allows_far_use(&self, _item_type: u16, _action_id: u16) -> bool {
        false
    }

    /// TFS `Weapon::executeUseWeapon` — `weapons.cpp:485`.
    /// Hit → `VARIANT_NUMBER`; miss → `VARIANT_POSITION` at drop tile.
    fn dispatch_on_use_weapon(
        &self,
        _item_id: u16,
        _creature: CreatureId,
        _target_creature: Option<CreatureId>,
        _target_pos: Option<(u16, u16, u8)>,
        _hit: bool,
    ) -> bool {
        false
    }

    /// Whether an `onUseWeapon` callback is registered for this item id.
    fn has_weapon_on_use(&self, _item_id: u16) -> bool {
        false
    }

    /// NPC-7: lifecycle / custom dialogue callbacks (opaque [`NpcCallbackId`]).
    fn on_npc_appear(&self, _npc: CreatureId, _callback: NpcCallbackId) {}
    fn on_npc_disappear(&self, _npc: CreatureId, _callback: NpcCallbackId) {}
    fn on_npc_move(
        &self,
        _npc: CreatureId,
        _callback: NpcCallbackId,
        _from: Position,
        _to: Position,
    ) {
    }
    fn on_npc_say(
        &self,
        _npc: CreatureId,
        _callback: NpcCallbackId,
        _speaker: CreatureId,
        _text: &str,
    ) {
    }
    fn on_npc_think(&self, _npc: CreatureId, _callback: NpcCallbackId, _interval_ms: u32) {}
    /// Custom dialogue predicate — `true` to keep the rule candidate.
    fn on_npc_custom_predicate(
        &self,
        _npc: CreatureId,
        _player: CreatureId,
        _callback: NpcCallbackId,
    ) -> bool {
        false
    }
    /// Custom dialogue action — `true` if the callback ran without error.
    fn on_npc_custom_action(
        &self,
        _npc: CreatureId,
        _player: CreatureId,
        _callback: NpcCallbackId,
    ) -> bool {
        false
    }
    /// NPC-8 stubs — shop window callbacks (no-op until shop subsystem).
    fn on_npc_shop_buy(
        &self,
        _npc: CreatureId,
        _player: CreatureId,
        _item_id: u16,
        _count: u16,
        _callback: Option<NpcCallbackId>,
    ) {
    }
    fn on_npc_shop_sell(
        &self,
        _npc: CreatureId,
        _player: CreatureId,
        _item_id: u16,
        _count: u16,
        _callback: Option<NpcCallbackId>,
    ) {
    }
    fn on_npc_shop_close(
        &self,
        _npc: CreatureId,
        _player: CreatureId,
        _callback: Option<NpcCallbackId>,
    ) {
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
