//! Lua-based event dispatcher implementation.
//!
//! C++ reference: `src/movement.cpp` `MoveEvents::onPlayerEquip`, `MoveEvent::fireEquip`.

use std::any::Any;
use std::collections::HashMap;

use crate::event_dispatcher::{EventDispatcher, TalkActionResult};
use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use crate::talkactions::TalkActionRegistry;
use slotmap::Key;
use tfs_rust_common::Position;
use tfs_rust_lua::{
    with_lua_context, CallbackRef, CreatureEventType, LuaRuntime, MoveEventKind,
    MoveEventsRegistry, PlayerEventType,
};

/// Lua-based event dispatcher.
pub struct LuaEventDispatcher {
    runtime: LuaRuntime,
    creature_events: HashMap<CreatureEventType, Vec<CallbackRef>>,
    player_events: HashMap<PlayerEventType, Vec<CallbackRef>>,
    move_events: MoveEventsRegistry,
    /// CH-6: talkaction registry — `/i`, `/a`, … GM commands. The
    /// `mlua::RegistryKey`s are tied to `runtime`'s `Lua` instance.
    talkactions: TalkActionRegistry,
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
            talkactions: TalkActionRegistry::default(),
        }
    }

    /// CH-6: Set the talkaction registry (called after loading talkaction scripts).
    pub fn set_talkactions(&mut self, talkactions: TalkActionRegistry) {
        self.talkactions = talkactions;
    }

    /// CH-6: Number of registered talkactions (for startup diagnostics).
    pub fn talkactions_count(&self) -> usize {
        self.talkactions.entries.len()
    }

    /// Get mutable access to the Lua runtime (for loading chat channels, etc.).
    pub fn runtime_mut(&mut self) -> &mut LuaRuntime {
        &mut self.runtime
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

    fn dispatch_move_item(
        &self,
        kind: MoveEventKind,
        actor: Option<CreatureId>,
        item: ItemId,
        item_type: u16,
        from: Position,
        to: Position,
    ) -> bool {
        let Some(actor) = actor else {
            return true;
        };
        let Some(entry) = self.move_events.get(kind, item_type) else {
            return true;
        };
        match self.runtime.call_move_item(
            &entry.callback,
            actor.data().as_ffi(),
            item.data().as_ffi(),
            from,
            to,
        ) {
            Ok(allow) => allow,
            Err(e) => {
                tracing::error!(?actor, ?item, item_type, ?kind, "Lua move item event failed: {e}");
                true
            }
        }
    }

    fn dispatch_move_step(
        &self,
        kind: MoveEventKind,
        actor: Option<CreatureId>,
        item: ItemId,
        item_type: u16,
        pos: Position,
    ) -> bool {
        let Some(actor) = actor else {
            return true;
        };
        let Some(entry) = self.move_events.get(kind, item_type) else {
            return true;
        };
        match self.runtime.call_move_step(
            &entry.callback,
            actor.data().as_ffi(),
            item.data().as_ffi(),
            pos,
        ) {
            Ok(allow) => allow,
            Err(e) => {
                tracing::error!(?actor, ?item, item_type, ?kind, "Lua move step event failed: {e}");
                true
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

    fn on_player_inventory_update(&self, player: CreatureId, item: ItemId, slot: u8, equip: bool) {
        let Some(callbacks) = self.player_events.get(&PlayerEventType::InventoryUpdate) else {
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

    fn on_remove_item(
        &self,
        actor: Option<CreatureId>,
        item: ItemId,
        item_type: u16,
        from: Position,
        to: Position,
    ) -> bool {
        self.dispatch_move_item(MoveEventKind::RemoveItem, actor, item, item_type, from, to)
    }

    fn on_add_item(
        &self,
        actor: Option<CreatureId>,
        item: ItemId,
        item_type: u16,
        from: Position,
        to: Position,
    ) -> bool {
        self.dispatch_move_item(MoveEventKind::AddItem, actor, item, item_type, from, to)
    }

    fn on_step_out(
        &self,
        actor: Option<CreatureId>,
        item: ItemId,
        item_type: u16,
        pos: Position,
    ) -> bool {
        self.dispatch_move_step(MoveEventKind::StepOut, actor, item, item_type, pos)
    }

    fn on_step_in(
        &self,
        actor: Option<CreatureId>,
        item: ItemId,
        item_type: u16,
        pos: Position,
    ) -> bool {
        self.dispatch_move_step(MoveEventKind::StepIn, actor, item, item_type, pos)
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
                            tracing::error!(
                                "Lua onLogin callback failed for {:?}: {}",
                                creature,
                                e
                            );
                        }
                    }
                }
            }
        });
    }

    fn on_logout(&self, creature: CreatureId, ctx: &dyn tfs_rust_common::ScriptContext) -> bool {
        // C++: `g_creatureEvents->playerLogout(player)` — false cancels logout.
        let mut allow = true;
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
                            allow = false;
                        }
                        Err(e) => {
                            tracing::error!(
                                "Lua onLogout callback failed for {:?}: {}",
                                creature,
                                e
                            );
                        }
                    }
                }
            }
        });
        allow
    }

    fn execute_timer_event(&self, event_id: u64) -> bool {
        // C++ reference: `LuaEnvironment::executeTimerEvent` (`luascript.cpp:18238`).
        match self.runtime.execute_timer_event(event_id) {
            Ok(found) => found,
            Err(e) => {
                tracing::error!(event_id, "Lua addEvent callback failed: {e}");
                false
            }
        }
    }

    fn dispatch_talkaction(&self, text: &str, creature: CreatureId) -> TalkActionResult {
        // C++ reference: `talkaction.cpp:84-134` `TalkActions::playerSaySpell`.
        let Some((entry, param)) = self.talkactions.find_match(text) else {
            return TalkActionResult::NotMatched;
        };
        match self.runtime.call_talkaction_on_say(
            &entry.on_say,
            creature.data().as_ffi(),
            &entry.words,
            &param,
        ) {
            Ok(true) => TalkActionResult::Continue,
            Ok(false) => TalkActionResult::Break,
            Err(e) => {
                tracing::error!(
                    ?creature,
                    words = %entry.words,
                    "Lua talkaction onSay failed: {e}"
                );
                TalkActionResult::Break
            }
        }
    }

    fn dispatch_on_cast_spell(
        &self,
        spell_words: &str,
        creature: CreatureId,
        need_direction: bool,
        has_param: bool,
        param: &str,
    ) -> bool {
        // PC-3a: C++ reference: `InstantSpell::castSpell` →
        // `LuaEnvironment::callLuaFunction` (`spells.cpp`).
        match self.runtime.call_on_cast_spell(
            spell_words,
            creature.data().as_ffi(),
            need_direction,
            has_param,
            param,
        ) {
            Ok(success) => success,
            Err(e) => {
                tracing::error!(
                    ?creature,
                    spell_words,
                    "Lua onCastSpell callback failed: {e}"
                );
                false
            }
        }
    }

    fn dispatch_on_cast_rune(
        &self,
        rune_id: u16,
        creature: CreatureId,
        target_creature: Option<CreatureId>,
        target_pos: Option<(u16, u16, u8)>,
    ) -> bool {
        let key = format!("rune:{rune_id}");
        let target_num = target_creature.map(|c| c.data().as_ffi());
        match self.runtime.call_on_cast_spell_keyed(
            &key,
            creature.data().as_ffi(),
            target_num,
            target_pos,
        ) {
            Ok(success) => success,
            Err(e) => {
                tracing::error!(?creature, rune_id, "Lua rune onCastSpell failed: {e}");
                false
            }
        }
    }

    fn dispatch_on_use_weapon(
        &self,
        item_id: u16,
        creature: CreatureId,
        target_creature: Option<CreatureId>,
        target_pos: Option<(u16, u16, u8)>,
        hit: bool,
    ) -> bool {
        let target_num = target_creature.map(|c| c.data().as_ffi());
        match self.runtime.call_on_use_weapon(
            item_id,
            creature.data().as_ffi(),
            target_num,
            target_pos,
            hit,
        ) {
            Ok(success) => success,
            Err(e) => {
                tracing::error!(
                    ?creature,
                    item_id,
                    "Lua onUseWeapon callback failed: {e}"
                );
                false
            }
        }
    }

    fn has_weapon_on_use(&self, item_id: u16) -> bool {
        self.runtime.has_weapon_callback(item_id)
    }

    fn on_npc_appear(&self, npc: CreatureId, callback: tfs_rust_content::npcs::NpcCallbackId) {
        if let Err(e) = self
            .runtime
            .call_npc_callback_npc_only(callback, npc.data().as_ffi())
        {
            tracing::error!(?npc, "Lua onNpcAppear failed: {e}");
        }
    }

    fn on_npc_disappear(&self, npc: CreatureId, callback: tfs_rust_content::npcs::NpcCallbackId) {
        if let Err(e) = self
            .runtime
            .call_npc_callback_npc_only(callback, npc.data().as_ffi())
        {
            tracing::error!(?npc, "Lua onNpcDisappear failed: {e}");
        }
    }

    fn on_npc_move(
        &self,
        npc: CreatureId,
        callback: tfs_rust_content::npcs::NpcCallbackId,
        from: tfs_rust_common::Position,
        to: tfs_rust_common::Position,
    ) {
        if let Err(e) = self.runtime.call_npc_callback_move(
            callback,
            npc.data().as_ffi(),
            (from.x, from.y, from.z),
            (to.x, to.y, to.z),
        ) {
            tracing::error!(?npc, "Lua onNpcMove failed: {e}");
        }
    }

    fn on_npc_say(
        &self,
        npc: CreatureId,
        callback: tfs_rust_content::npcs::NpcCallbackId,
        speaker: CreatureId,
        text: &str,
    ) {
        if let Err(e) = self.runtime.call_npc_callback_say(
            callback,
            npc.data().as_ffi(),
            speaker.data().as_ffi(),
            text,
        ) {
            tracing::error!(?npc, "Lua onNpcSay failed: {e}");
        }
    }

    fn on_npc_think(
        &self,
        npc: CreatureId,
        callback: tfs_rust_content::npcs::NpcCallbackId,
        interval_ms: u32,
    ) {
        if let Err(e) = self.runtime.call_npc_callback_think(
            callback,
            npc.data().as_ffi(),
            interval_ms,
        ) {
            tracing::error!(?npc, "Lua onNpcThink failed: {e}");
        }
    }

    fn on_npc_custom_predicate(
        &self,
        npc: CreatureId,
        player: CreatureId,
        callback: tfs_rust_content::npcs::NpcCallbackId,
    ) -> bool {
        match self.runtime.call_npc_callback_with_player(
            callback,
            npc.data().as_ffi(),
            player.data().as_ffi(),
        ) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(?npc, ?player, "Lua custom predicate failed: {e}");
                false
            }
        }
    }

    fn on_npc_custom_action(
        &self,
        npc: CreatureId,
        player: CreatureId,
        callback: tfs_rust_content::npcs::NpcCallbackId,
    ) -> bool {
        match self.runtime.call_npc_callback_with_player(
            callback,
            npc.data().as_ffi(),
            player.data().as_ffi(),
        ) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(?npc, ?player, "Lua custom action failed: {e}");
                false
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
