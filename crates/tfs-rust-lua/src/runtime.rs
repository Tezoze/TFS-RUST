//! Lua runtime and VM management.
//!
//! This module provides the LuaRuntime struct which owns the mlua::Lua VM
//! and manages script registry and global function registration.

use mlua::{Lua, RegistryKey, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use crate::constants::register_constants;
use crate::context::{CreatureRef, ItemRef};
use crate::userdata::PositionRef;
use tfs_rust_common::Position;
use crate::npc_dialogue::register_npc_dialogue;
use crate::npc_type::register_npc_type;
use crate::timer_events::{TimerEvents, execute_timer_event, register_add_event_stop_event};
use crate::userdata::{
    register_combat_metatable, register_condition_metatable, register_container_metatable,
    register_creature_metatable, register_group_metatable, register_item_metatable,
    register_npc_metatable,
    register_item_type_constructor, register_item_type_metatable,
    register_monster_type_constructor, register_position_metatable, register_spell_metatable,
    register_tile_constructor, register_vocation_metatable, register_weapon_metatable,
};

/// Wrapper for mlua::RegistryKey — !Send, must stay on game thread.
#[derive(Debug)]
pub struct CallbackRef(mlua::RegistryKey);

/// Lua runtime owning the VM and script registry.
///
/// This is !Send by design and must live exclusively on the game thread.
pub struct LuaRuntime {
    pub(crate) lua: Lua,
    script_registry: HashMap<String, ()>,
    /// `addEvent` / `stopEvent` timer-event registry (C++ `g_luaEnvironment.timerEvents`).
    /// `Rc<RefCell<…>>` so the Lua closures and `execute_timer_event` share access.
    timer_events: TimerEvents,
    /// Pending chat channels from Channel:register() calls (drained after directory scan).
    pending_chat_channels: Vec<PendingChatChannel>,
    /// Pending talkactions from TalkAction:register() calls (drained after directory scan).
    pending_talkactions: Vec<PendingTalkAction>,
    /// Pending actions from Action:register() calls (drained after directory scan).
    pending_actions: Vec<PendingAction>,
    /// PC-3a: `onCastSpell` callback registry keys, keyed by spell words (lowercased).
    /// Populated during `load_spell_scripts` from `_pending_spell_callbacks`.
    /// C++ reference: `Event::loadCallback` / `getEvent` (`baseevents.cpp:136`,
    /// `luascript.cpp:363`) — stores the Lua function reference for later invocation.
    spell_callbacks: HashMap<String, RegistryKey>,
    /// PC-3a: `onUseWeapon` callbacks keyed by item id (`weapon:{id}`).
    /// C++ `Weapon::executeUseWeapon` — `weapons.cpp:485`.
    weapon_callbacks: HashMap<u16, RegistryKey>,
    /// NPC-1: custom predicate/action callbacks keyed by opaque [`tfs_rust_content::npcs::NpcCallbackId`].
    /// Content defs store the id; RegistryKeys stay here (!Send, game thread).
    pub(crate) npc_callbacks: HashMap<tfs_rust_content::npcs::NpcCallbackId, RegistryKey>,
}

/// Pending channel definition from Lua Channel:register().
#[derive(Debug)]
pub struct PendingChatChannel {
    pub id: u16,
    pub name: String,
    pub public: bool,
    pub on_speak: Option<mlua::RegistryKey>,
    pub can_join: Option<mlua::RegistryKey>,
    pub on_join: Option<mlua::RegistryKey>,
    pub on_leave: Option<mlua::RegistryKey>,
}

/// Pending talkaction definition from Lua TalkAction:register().
///
/// C++ reference: `talkaction.h` `TalkAction` — words, separator, onSay callback.
/// `words` may be `;`-separated for multi-word registration (C++ `explodeString`).
#[derive(Debug)]
pub struct PendingTalkAction {
    pub words: String,
    pub separator: String,
    pub on_say: Option<mlua::RegistryKey>,
}

/// Pending action definition from Lua Action:register().
///
/// C++ reference: `actions.h` `Action` — item/action id lists + `onUse` callback.
#[derive(Debug)]
pub struct PendingAction {
    pub item_ids: Vec<u16>,
    pub action_ids: Vec<u16>,
    pub on_use: Option<mlua::RegistryKey>,
}

impl LuaRuntime {
    /// Create a new Lua runtime with minimal global functions registered.
    ///
    /// # Errors
    ///
    /// Returns an error if VM initialization or lib loading fails.
    pub fn new() -> Result<Self, LuaError> {
        let lua = Lua::new();

        // Register minimal global functions via RegisterLuaFunctions
        let registrar = MinimalGlobalFunctions;
        registrar
            .register_functions(&lua)
            .map_err(LuaError::Registration)?;

        register_creature_metatable(&lua).map_err(LuaError::Registration)?;
        register_npc_metatable(&lua).map_err(LuaError::Registration)?;
        register_item_metatable(&lua).map_err(LuaError::Registration)?;
        register_container_metatable(&lua).map_err(LuaError::Registration)?;
        register_vocation_metatable(&lua).map_err(LuaError::Registration)?;
        register_condition_metatable(&lua).map_err(LuaError::Registration)?;
        register_group_metatable(&lua).map_err(LuaError::Registration)?;
        register_position_metatable(&lua).map_err(LuaError::Registration)?;
        register_item_type_metatable(&lua).map_err(LuaError::Registration)?;
        register_event_script_bootstrap(&lua).map_err(LuaError::Registration)?;
        // Overwrite empty `Tile` / `Game` stubs from bootstrap with real constructors.
        register_tile_constructor(&lua).map_err(LuaError::Registration)?;
        register_game_api(&lua).map_err(LuaError::Registration)?;
        register_variant_constructor(&lua).map_err(LuaError::Registration)?;
        register_monster_type_constructor(&lua).map_err(LuaError::Registration)?;
        // TFS Lua global constants (ACCOUNT_TYPE_*, TALKTYPE_*, PlayerFlag_*,
        // VOCATION_NONE, CONDITION_*, RETURNVALUE_*, …). Mirrors
        // `luascript.cpp` `registerConstants`; 772 values from
        // `reference/tvp-772/gameserver/src/{const.h,enums.h}`. See
        // `constants.rs` and `tasks/lua-api-plan.md` LUA-1.
        register_constants(&lua).map_err(LuaError::Registration)?;

        // PC-2b: Combat / Spell / Weapon / Condition userdata + createCombatArea +
        // ~860 combat/spell/weapon/condition enums. Mirrors `luascript.cpp`
        // `registerClass("Combat"/"Spell"/"Weapon"/"Condition")` + `registerEnum` block.
        register_combat_metatable(&lua).map_err(LuaError::Registration)?;
        register_weapon_metatable(&lua).map_err(LuaError::Registration)?;
        register_spell_metatable(&lua).map_err(LuaError::Registration)?;
        crate::combat_enums::register_combat_enums(&lua).map_err(LuaError::Registration)?;
        register_do_challenge_creature(&lua).map_err(LuaError::Registration)?;

        // NPC-1: NpcType / NpcDialogue definition builders (no GameWorld).
        register_npc_dialogue(&lua).map_err(LuaError::Registration)?;
        register_npc_type(&lua).map_err(LuaError::Registration)?;

        // Initialize pending channel buffer for Channel:register()
        let pending_channels = lua.create_table()?;
        lua.globals().set("_pending_channels", pending_channels)?;

        // Initialize pending talkaction buffer for TalkAction:register()
        let pending_talkactions = lua.create_table()?;
        lua.globals()
            .set("_pending_talkactions", pending_talkactions)?;

        // Initialize pending action buffer for Action:register()
        let pending_actions = lua.create_table()?;
        lua.globals().set("_pending_actions", pending_actions)?;

        // NPC-1 pending buffers (also re-init'd in load_npc_definitions).
        lua.globals().set("_pending_npcs", lua.create_table()?)?;
        lua.globals()
            .set("_pending_npc_action_callbacks", lua.create_table()?)?;
        lua.globals()
            .set("_pending_npc_predicate_callbacks", lua.create_table()?)?;

        // `addEvent` / `stopEvent` globals (C++ `luascript.cpp:1126-1130`).
        // The `TimerScheduler` thread-local is set later from `run_server.rs`;
        // the closures read it at call time, not registration time.
        let timer_events: TimerEvents = Rc::new(RefCell::new(HashMap::new()));
        let next_timer_id = Rc::new(RefCell::new(1u64));
        register_add_event_stop_event(&lua, timer_events.clone(), next_timer_id)
            .map_err(LuaError::Registration)?;

        // Load data/lib/*.lua files (fatal if any fail)
        // TODO: Implement lib loading after we have a data directory path

        Ok(Self {
            lua,
            script_registry: HashMap::new(),
            timer_events,
            pending_chat_channels: Vec::new(),
            pending_talkactions: Vec::new(),
            pending_actions: Vec::new(),
            spell_callbacks: HashMap::new(),
            weapon_callbacks: HashMap::new(),
            npc_callbacks: HashMap::new(),
        })
    }

    /// Load and compile a Lua script file.
    ///
    /// # Errors
    ///
    /// Returns an error on syntax failure (non-fatal for script loading).
    pub fn load_script(&mut self, path: &str) -> Result<CallbackRef, LuaError> {
        let full_path = Path::new(path);
        let chunk = std::fs::read_to_string(full_path)
            .map_err(|e| LuaError::ScriptIo(full_path.display().to_string(), e.to_string()))?;
        self.lua
            .load(&chunk)
            .set_name(path)
            .exec()
            .map_err(LuaError::Init)?;

        let key = self.lua.create_registry_value(true)?;
        Ok(CallbackRef(key))
    }

    /// Load a Lua script file with channel registration support.
    ///
    /// This is used for chat channel scripts that call `Channel(...):register()`.
    /// The script is executed, and any channels registered via `Channel:register()` are
    /// captured in the pending buffer.
    ///
    /// # Errors
    ///
    /// Returns an error on syntax failure (non-fatal for script loading).
    pub fn load_channel_script(&mut self, path: &str) -> Result<(), LuaError> {
        let full_path = Path::new(path);
        let chunk = std::fs::read_to_string(full_path)
            .map_err(|e| LuaError::ScriptIo(full_path.display().to_string(), e.to_string()))?;

        // Clear pending buffer before loading
        self.lua
            .globals()
            .set("_pending_channels", self.lua.create_table()?)?;

        // Execute the script
        self.lua
            .load(&chunk)
            .set_name(path)
            .exec()
            .map_err(LuaError::Init)?;

        // Drain pending channels into our buffer
        let pending: mlua::Table = self.lua.globals().get("_pending_channels")?;
        for i in 1..=pending.len()? {
            if let Ok(channel_table) = pending.get::<mlua::Table>(i) {
                let id: u16 = channel_table.get("id")?;
                let name: String = channel_table.get("name")?;
                // `_public` (not `public`) — `public` is the fluent-setter method on the
                // channel table; the flag it toggles is stored under `_public`.
                let public: bool = channel_table.get("_public")?;

                let on_speak = channel_table
                    .get::<Option<mlua::Function>>("onSpeak")?
                    .map(|f| self.lua.create_registry_value(f))
                    .transpose()
                    .map_err(LuaError::Init)?;

                let can_join = channel_table
                    .get::<Option<mlua::Function>>("canJoin")?
                    .map(|f| self.lua.create_registry_value(f))
                    .transpose()
                    .map_err(LuaError::Init)?;

                let on_join = channel_table
                    .get::<Option<mlua::Function>>("onJoin")?
                    .map(|f| self.lua.create_registry_value(f))
                    .transpose()
                    .map_err(LuaError::Init)?;

                let on_leave = channel_table
                    .get::<Option<mlua::Function>>("onLeave")?
                    .map(|f| self.lua.create_registry_value(f))
                    .transpose()
                    .map_err(LuaError::Init)?;

                self.pending_chat_channels.push(PendingChatChannel {
                    id,
                    name,
                    public,
                    on_speak,
                    can_join,
                    on_join,
                    on_leave,
                });
            }
        }

        Ok(())
    }

    /// Drain pending chat channels accumulated from `load_channel_script` calls.
    ///
    /// This should be called after all channel scripts in a directory have been loaded.
    pub fn drain_pending_chat_channels(&mut self) -> Vec<PendingChatChannel> {
        std::mem::take(&mut self.pending_chat_channels)
    }

    /// Load a Lua script file with talkaction registration support.
    ///
    /// This is used for talkaction scripts that call `TalkAction(...):register()`.
    /// The script is executed, and any talkactions registered via
    /// `TalkAction:register()` are captured in the pending buffer.
    ///
    /// C++ reference: `talkaction.cpp` `TalkActions::registerLuaEvent`.
    pub fn load_talkaction_script(&mut self, path: &str) -> Result<(), LuaError> {
        let full_path = Path::new(path);
        let chunk = std::fs::read_to_string(full_path)
            .map_err(|e| LuaError::ScriptIo(full_path.display().to_string(), e.to_string()))?;

        // Clear pending buffer before loading
        self.lua
            .globals()
            .set("_pending_talkactions", self.lua.create_table()?)?;

        // Execute the script
        self.lua
            .load(&chunk)
            .set_name(path)
            .exec()
            .map_err(LuaError::Init)?;

        // Drain pending talkactions into our buffer
        let pending: mlua::Table = self.lua.globals().get("_pending_talkactions")?;
        for i in 1..=pending.len()? {
            if let Ok(ta_table) = pending.get::<mlua::Table>(i) {
                let words: String = ta_table.get("words")?;
                // `_separator` (not `separator`) — `separator` is the fluent-setter
                // method on the talkaction table; the flag it toggles is stored
                // under `_separator`.
                let separator: String = ta_table.get("_separator")?;

                let on_say = ta_table
                    .get::<Option<mlua::Function>>("onSay")?
                    .map(|f| self.lua.create_registry_value(f))
                    .transpose()
                    .map_err(LuaError::Init)?;

                self.pending_talkactions.push(PendingTalkAction {
                    words,
                    separator,
                    on_say,
                });
            }
        }

        Ok(())
    }

    /// Drain pending talkactions accumulated from `load_talkaction_script` calls.
    ///
    /// This should be called after all talkaction scripts in a directory have been loaded.
    pub fn drain_pending_talkactions(&mut self) -> Vec<PendingTalkAction> {
        std::mem::take(&mut self.pending_talkactions)
    }

    /// Load a Lua script that calls `Action():register()`.
    ///
    /// C++ reference: `actions.cpp` `Actions::registerLuaEvent`.
    pub fn load_action_script(&mut self, path: &str) -> Result<(), LuaError> {
        let full_path = Path::new(path);
        let chunk = std::fs::read_to_string(full_path)
            .map_err(|e| LuaError::ScriptIo(full_path.display().to_string(), e.to_string()))?;

        self.lua
            .globals()
            .set("_pending_actions", self.lua.create_table()?)?;

        self.lua
            .load(&chunk)
            .set_name(path)
            .exec()
            .map_err(LuaError::Init)?;

        let pending: mlua::Table = self.lua.globals().get("_pending_actions")?;
        for i in 1..=pending.len()? {
            if let Ok(action_table) = pending.get::<mlua::Table>(i) {
                let mut item_ids = Vec::new();
                let ids_table: mlua::Table = action_table.get("_ids")?;
                for j in 1..=ids_table.len()? {
                    if let Ok(id) = ids_table.get::<u16>(j) {
                        item_ids.push(id);
                    }
                }

                let mut action_ids = Vec::new();
                let aids_table: mlua::Table = action_table.get("_aids")?;
                for j in 1..=aids_table.len()? {
                    if let Ok(id) = aids_table.get::<u16>(j) {
                        action_ids.push(id);
                    }
                }

                let on_use = action_table
                    .get::<Option<mlua::Function>>("onUse")?
                    .map(|f| self.lua.create_registry_value(f))
                    .transpose()
                    .map_err(LuaError::Init)?;

                self.pending_actions.push(PendingAction {
                    item_ids,
                    action_ids,
                    on_use,
                });
            }
        }

        Ok(())
    }

    /// Drain pending actions accumulated from `load_action_script` calls.
    pub fn drain_pending_actions(&mut self) -> Vec<PendingAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Call an action `onUse` hook — `(player, item, fromPos, target, toPos) -> bool`.
    ///
    /// Returns `true` = handled (skip native fallthrough), `false` = not handled.
    /// C++ reference: `actions.cpp` `Action::executeUse` / `callFunction(6)` (no `isHotkey`).
    #[allow(clippy::too_many_arguments)]
    pub fn call_action_on_use(
        &self,
        callback: &mlua::RegistryKey,
        player: crate::context::CreatureId,
        item: crate::context::ItemId,
        from_pos: (u16, u16, u8),
        target_item: Option<crate::context::ItemId>,
        target_creature: Option<crate::context::CreatureId>,
        to_pos: (u16, u16, u8),
    ) -> Result<bool, LuaError> {
        let function: mlua::Function = self.lua.registry_value(callback).map_err(LuaError::Init)?;
        let player_ud = self
            .lua
            .create_userdata(CreatureRef(player))
            .map_err(LuaError::Init)?;
        let item_ud = self
            .lua
            .create_userdata(ItemRef(item))
            .map_err(LuaError::Init)?;
        let from_ud = self
            .lua
            .create_userdata(PositionRef {
                x: from_pos.0,
                y: from_pos.1,
                z: from_pos.2,
            })
            .map_err(LuaError::Init)?;
        let to_ud = self
            .lua
            .create_userdata(PositionRef {
                x: to_pos.0,
                y: to_pos.1,
                z: to_pos.2,
            })
            .map_err(LuaError::Init)?;

        let target: Value = if let Some(tid) = target_item {
            Value::UserData(
                self.lua
                    .create_userdata(ItemRef(tid))
                    .map_err(LuaError::Init)?,
            )
        } else if let Some(cid) = target_creature {
            Value::UserData(
                self.lua
                    .create_userdata(CreatureRef(cid))
                    .map_err(LuaError::Init)?,
            )
        } else {
            Value::Nil
        };

        function
            .call::<bool>((player_ud, item_ud, from_ud, target, to_ud))
            .map_err(LuaError::Init)
    }

    /// Call a talkaction `onSay` hook — `(player, words, param) -> bool`.
    ///
    /// Returns `true` = TALKACTION_CONTINUE (broadcast as normal chat),
    /// `false` = TALKACTION_BREAK (consumed, do not broadcast).
    /// C++ reference: `talkaction.cpp` `TalkAction::executeSay`.
    pub fn call_talkaction_on_say(
        &self,
        callback: &mlua::RegistryKey,
        player: crate::context::CreatureId,
        words: &str,
        param: &str,
    ) -> Result<bool, LuaError> {
        tracing::info!(player, words, param, "call_talkaction_on_say: invoking Lua");
        let function: mlua::Function = self.lua.registry_value(callback).map_err(LuaError::Init)?;
        let player_ud = self
            .lua
            .create_userdata(crate::context::CreatureRef(player))
            .map_err(LuaError::Init)?;
        // C++ `executeSay` pushes (player, words, param, type). The /i script
        // only uses (player, words, param) — `type` is ignored by all active
        // scripts. Pass 0 (TALKTYPE_SAY) as the type.
        let result = function
            .call::<bool>((player_ud, words, param, 0i32))
            .map_err(LuaError::Init);
        tracing::info!(
            player,
            words,
            ?result,
            "call_talkaction_on_say: Lua returned"
        );
        result
    }

    /// Execute a Lua chunk (bootstrap globals, compat stubs).
    pub fn exec_chunk(&self, name: &str, chunk: &str) -> Result<(), LuaError> {
        self.lua
            .load(chunk)
            .set_name(name)
            .exec()
            .map_err(LuaError::Init)
    }

    pub fn register_callback(
        &mut self,
        callback_key: String,
        global_function_name: &str,
    ) -> Result<CallbackRef, LuaError> {
        let globals = self.lua.globals();
        let function: mlua::Function = globals
            .get(global_function_name)
            .map_err(|_| LuaError::MissingFunction(global_function_name.to_string()))?;
        let registry_key = self.lua.create_registry_value(function)?;
        let callback = CallbackRef(registry_key);
        self.script_registry.insert(callback_key, ());
        Ok(callback)
    }

    pub fn call_creature_callback(
        &self,
        callback: &CallbackRef,
        creature: crate::context::CreatureId,
    ) -> Result<bool, LuaError> {
        let function: mlua::Function = self
            .lua
            .registry_value(&callback.0)
            .map_err(LuaError::Init)?;
        let player = self
            .lua
            .create_userdata(CreatureRef(creature))
            .map_err(LuaError::Init)?;
        function.call::<bool>(player).map_err(LuaError::Init)
    }

    /// Execute a fired `addEvent` timer callback.
    ///
    /// C++ reference: `LuaEnvironment::executeTimerEvent` (`luascript.cpp:18238`).
    /// Called from the game loop when `GameCommand::LuaCallback { event_id }` arrives.
    /// Returns `Ok(true)` if the event was found and executed, `Ok(false)` if it was
    /// already cancelled.
    pub fn execute_timer_event(&self, event_id: u64) -> Result<bool, LuaError> {
        execute_timer_event(&self.lua, &self.timer_events, event_id).map_err(LuaError::Init)
    }

    /// Register `TableName.methodName` from a loaded script (e.g. `Player.onInventoryUpdate`).
    pub fn register_table_method_callback(
        &mut self,
        callback_key: String,
        table_name: &str,
        method_name: &str,
    ) -> Result<CallbackRef, LuaError> {
        let globals = self.lua.globals();
        let table: mlua::Table = globals
            .get(table_name)
            .map_err(|_| LuaError::MissingFunction(format!("{table_name} table")))?;
        let function: mlua::Function = table
            .get(method_name)
            .map_err(|_| LuaError::MissingFunction(format!("{table_name}:{method_name}")))?;
        let registry_key = self.lua.create_registry_value(function)?;
        let callback = CallbackRef(registry_key);
        self.script_registry.insert(callback_key, ());
        Ok(callback)
    }

    /// TFS `Events::eventPlayerOnInventoryUpdate` — `Player:onInventoryUpdate(item, slot, equip)`.
    pub fn call_player_inventory_update(
        &self,
        callback: &CallbackRef,
        player: crate::context::CreatureId,
        item: crate::context::ItemId,
        slot: u8,
        equip: bool,
    ) -> Result<(), LuaError> {
        use crate::context::ItemRef;
        let function: mlua::Function = self
            .lua
            .registry_value(&callback.0)
            .map_err(LuaError::Init)?;
        let player_ud = self
            .lua
            .create_userdata(CreatureRef(player))
            .map_err(LuaError::Init)?;
        let item_ud = self
            .lua
            .create_userdata(ItemRef(item))
            .map_err(LuaError::Init)?;
        function
            .call::<()>((player_ud, item_ud, slot, equip))
            .map_err(LuaError::Init)
    }

    /// TFS `MoveEvent::executeEquip` — `(player, item, slot, isCheck)`.
    pub fn call_move_equip(
        &self,
        callback: &CallbackRef,
        player: crate::context::CreatureId,
        item: crate::context::ItemId,
        slot: u8,
        is_check: bool,
    ) -> Result<bool, LuaError> {
        let function: mlua::Function = self
            .lua
            .registry_value(&callback.0)
            .map_err(LuaError::Init)?;
        let player_ud = self
            .lua
            .create_userdata(CreatureRef(player))
            .map_err(LuaError::Init)?;
        let item_ud = self
            .lua
            .create_userdata(ItemRef(item))
            .map_err(LuaError::Init)?;
        function
            .call::<bool>((player_ud, item_ud, slot, is_check))
            .map_err(LuaError::Init)
    }

    /// TFS `MoveEvent::executeAddItem` — `(player, item, fromPosition, toPosition) -> bool`.
    pub fn call_move_item(
        &self,
        callback: &CallbackRef,
        player: crate::context::CreatureId,
        item: crate::context::ItemId,
        from: Position,
        to: Position,
    ) -> Result<bool, LuaError> {
        let function: mlua::Function = self
            .lua
            .registry_value(&callback.0)
            .map_err(LuaError::Init)?;
        let player_ud = self
            .lua
            .create_userdata(CreatureRef(player))
            .map_err(LuaError::Init)?;
        let item_ud = self
            .lua
            .create_userdata(ItemRef(item))
            .map_err(LuaError::Init)?;
        let from_ud = self
            .lua
            .create_userdata(PositionRef {
                x: from.x,
                y: from.y,
                z: from.z,
            })
            .map_err(LuaError::Init)?;
        let to_ud = self
            .lua
            .create_userdata(PositionRef {
                x: to.x,
                y: to.y,
                z: to.z,
            })
            .map_err(LuaError::Init)?;
        function
            .call::<bool>((player_ud, item_ud, from_ud, to_ud))
            .map_err(LuaError::Init)
    }

    /// TFS `MoveEvent::executeStep` — `(creature, item, position) -> bool`.
    pub fn call_move_step(
        &self,
        callback: &CallbackRef,
        creature: crate::context::CreatureId,
        item: crate::context::ItemId,
        pos: Position,
    ) -> Result<bool, LuaError> {
        let function: mlua::Function = self
            .lua
            .registry_value(&callback.0)
            .map_err(LuaError::Init)?;
        let creature_ud = self
            .lua
            .create_userdata(CreatureRef(creature))
            .map_err(LuaError::Init)?;
        let item_ud = self
            .lua
            .create_userdata(ItemRef(item))
            .map_err(LuaError::Init)?;
        let pos_ud = self
            .lua
            .create_userdata(PositionRef {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            })
            .map_err(LuaError::Init)?;
        function
            .call::<bool>((creature_ud, item_ud, pos_ud))
            .map_err(LuaError::Init)
    }

    /// Call a channel `canJoin` hook — `(player) -> bool`.
    ///
    /// C++ reference: `chat.cpp` `ChatChannel::executeCanJoinEvent` — `canJoinEvent` callback.
    pub fn call_channel_can_join(
        &self,
        callback: &mlua::RegistryKey,
        player: crate::context::CreatureId,
    ) -> Result<bool, LuaError> {
        let function: mlua::Function = self.lua.registry_value(callback).map_err(LuaError::Init)?;
        let player_ud = self
            .lua
            .create_userdata(CreatureRef(player))
            .map_err(LuaError::Init)?;
        function.call::<bool>(player_ud).map_err(LuaError::Init)
    }

    /// Call a channel `onSpeak` hook — `(player, type, message) -> type|bool`.
    ///
    /// C++ reference: `chat.cpp` `ChatChannel::executeOnSpeakEvent` — `onSpeakEvent` callback.
    pub fn call_channel_on_speak(
        &self,
        callback: &mlua::RegistryKey,
        player: crate::context::CreatureId,
        speak_type: i32,
        message: &str,
    ) -> Result<mlua::Value, LuaError> {
        let function: mlua::Function = self.lua.registry_value(callback).map_err(LuaError::Init)?;
        let player_ud = self
            .lua
            .create_userdata(CreatureRef(player))
            .map_err(LuaError::Init)?;
        function
            .call::<mlua::Value>((player_ud, speak_type, message))
            .map_err(LuaError::Init)
    }

    /// Call a channel `onJoin` hook — `(player)`.
    ///
    /// C++ reference: `chat.cpp` `ChatChannel::executeOnJoinEvent` — `onJoinEvent` callback.
    pub fn call_channel_on_join(
        &self,
        callback: &mlua::RegistryKey,
        player: crate::context::CreatureId,
    ) -> Result<(), LuaError> {
        let function: mlua::Function = self.lua.registry_value(callback).map_err(LuaError::Init)?;
        let player_ud = self
            .lua
            .create_userdata(CreatureRef(player))
            .map_err(LuaError::Init)?;
        function.call::<()>(player_ud).map_err(LuaError::Init)
    }

    /// Call a channel `onLeave` hook — `(player)`.
    ///
    /// C++ reference: `chat.cpp` `ChatChannel::executeOnLeaveEvent` — `onLeaveEvent` callback.
    pub fn call_channel_on_leave(
        &self,
        callback: &mlua::RegistryKey,
        player: crate::context::CreatureId,
    ) -> Result<(), LuaError> {
        let function: mlua::Function = self.lua.registry_value(callback).map_err(LuaError::Init)?;
        let player_ud = self
            .lua
            .create_userdata(CreatureRef(player))
            .map_err(LuaError::Init)?;
        function.call::<()>(player_ud).map_err(LuaError::Init)
    }

    /// Invoke a spell's `onCastSpell` Lua callback.
    ///
    /// C++ reference: `InstantSpell::playerCastInstant` / `executeCastSpell`
    /// (`spells.cpp` / `luascript.cpp`). The callback receives `(creature, variant)`
    /// and returns `true` on success.
    ///
    /// When `has_param` is true, builds `VARIANT_STRING` with `param` text
    /// (`spells.cpp` ~939–972). Otherwise builds `VARIANT_POSITION` (optionally
    /// direction-offset when `need_direction`).
    pub fn call_on_cast_spell(
        &self,
        spell_words: &str,
        creature: crate::context::CreatureId,
        need_direction: bool,
        has_param: bool,
        param: &str,
    ) -> Result<bool, LuaError> {
        let spec = if has_param {
            CastVariantSpec::String(param.to_string())
        } else {
            CastVariantSpec::Position { need_direction }
        };
        self.call_on_cast_spell_keyed_spec(&spell_words.to_lowercase(), creature, spec)
    }

    /// Invoke an `onCastSpell` callback by registry key (`words` or `rune:{id}`).
    pub fn call_on_cast_spell_keyed(
        &self,
        key: &str,
        creature: crate::context::CreatureId,
        target_number: Option<u64>,
        target_pos: Option<(u16, u16, u8)>,
    ) -> Result<bool, LuaError> {
        let spec = if let Some(n) = target_number {
            CastVariantSpec::Number(n)
        } else if let Some((x, y, z)) = target_pos {
            CastVariantSpec::FixedPosition { x, y, z }
        } else {
            CastVariantSpec::Position {
                need_direction: false,
            }
        };
        self.call_on_cast_spell_keyed_spec(key, creature, spec)
    }

    fn call_on_cast_spell_keyed_spec(
        &self,
        key: &str,
        creature: crate::context::CreatureId,
        spec: CastVariantSpec,
    ) -> Result<bool, LuaError> {
        let registry_key = self.spell_callbacks.get(key);
        let Some(registry_key) = registry_key else {
            return Ok(false);
        };
        let function: mlua::Function = self
            .lua
            .registry_value(registry_key)
            .map_err(LuaError::Init)?;
        let creature_ud = self
            .lua
            .create_userdata(CreatureRef(creature))
            .map_err(LuaError::Init)?;
        let variant = build_cast_variant_spec(&self.lua, creature, spec)?;
        function
            .call::<bool>((creature_ud, variant))
            .map_err(LuaError::Init)
    }

    /// PC-3a: Register a spell callback keyed by spell words.
    /// Called from `load_spell_scripts` after draining `_pending_spell_callbacks`.
    pub fn register_spell_callback(&mut self, words: &str, key: RegistryKey) {
        self.spell_callbacks.insert(words.to_lowercase(), key);
    }

    /// Register an `onUseWeapon` callback keyed by item id.
    pub fn register_weapon_callback(&mut self, item_id: u16, key: RegistryKey) {
        self.weapon_callbacks.insert(item_id, key);
    }

    /// Whether an `onUseWeapon` script is registered for this item.
    pub fn has_weapon_callback(&self, item_id: u16) -> bool {
        self.weapon_callbacks.contains_key(&item_id)
    }

    /// Invoke `onUseWeapon(player, variant[, hit])` — `weapons.cpp:485`.
    ///
    /// Hit → `VARIANT_NUMBER` (target creature id). Miss → `VARIANT_POSITION` at drop.
    /// Extra `hit` boolean matches `data/scripts/weapons/burst_arrow.lua` arity.
    pub fn call_on_use_weapon(
        &self,
        item_id: u16,
        creature: crate::context::CreatureId,
        target_number: Option<u64>,
        target_pos: Option<(u16, u16, u8)>,
        hit: bool,
    ) -> Result<bool, LuaError> {
        let Some(registry_key) = self.weapon_callbacks.get(&item_id) else {
            return Ok(false);
        };
        let function: mlua::Function = self
            .lua
            .registry_value(registry_key)
            .map_err(LuaError::Init)?;
        let creature_ud = self
            .lua
            .create_userdata(CreatureRef(creature))
            .map_err(LuaError::Init)?;
        let spec = if let Some(n) = target_number {
            CastVariantSpec::Number(n)
        } else if let Some((x, y, z)) = target_pos {
            CastVariantSpec::FixedPosition { x, y, z }
        } else {
            return Ok(false);
        };
        let variant = build_cast_variant_spec(&self.lua, creature, spec)?;
        // Always pass `hit` as 3rd arg — 2-arg scripts ignore it (Lua).
        let result: Value = function
            .call((creature_ud, variant, hit))
            .map_err(LuaError::Init)?;
        Ok(match result {
            Value::Boolean(b) => b,
            Value::Nil => true,
            _ => true,
        })
    }
}

/// How to build the Lua variant passed to `onCastSpell`.
enum CastVariantSpec {
    /// `VARIANT_STRING` — instant spells with `hasParams`.
    String(String),
    /// `VARIANT_NUMBER` — rune/self target creature id.
    Number(u64),
    /// `VARIANT_POSITION` from caster (optionally direction-offset).
    Position { need_direction: bool },
    /// `VARIANT_POSITION` at a fixed tile (rune use-with).
    FixedPosition { x: u16, y: u16, z: u8 },
}

/// LuaVariant type discriminants — `luascript.h` `LuaVariantType_t`.
const VARIANT_NUMBER: i32 = 1;
const VARIANT_POSITION: i32 = 2;
const VARIANT_STRING: i32 = 4;

fn build_cast_variant_spec(
    lua: &Lua,
    caster: crate::context::CreatureId,
    spec: CastVariantSpec,
) -> Result<mlua::Value, LuaError> {
    match spec {
        CastVariantSpec::String(text) => {
            let table = lua.create_table()?;
            table.set("type", VARIANT_STRING)?;
            table.set("string", text)?;
            attach_variant_methods(lua, table)
        }
        CastVariantSpec::Number(n) => {
            let table = lua.create_table()?;
            table.set("type", VARIANT_NUMBER)?;
            table.set("number", n)?;
            attach_variant_methods(lua, table)
        }
        CastVariantSpec::FixedPosition { x, y, z } => {
            build_position_variant(lua, x, y, z)
        }
        CastVariantSpec::Position { need_direction } => {
            use crate::context::CURRENT_CTX;
            let (x, y, z) = CURRENT_CTX.with(|c| {
                let ptr = (*c.borrow()).ok_or_else(|| {
                    LuaError::Init(mlua::Error::runtime(
                        "build_cast_variant: LuaContext not set",
                    ))
                })?;
                if ptr.is_null() {
                    return Err(LuaError::Init(mlua::Error::runtime(
                        "build_cast_variant: LuaContext is null",
                    )));
                }
                let ctx = unsafe { &*ptr };
                let pos = ctx.get_player_position(caster).ok_or_else(|| {
                    mlua::Error::runtime("build_cast_variant: caster position not found")
                })?;
                if !need_direction {
                    return Ok((pos.x, pos.y, pos.z));
                }
                let dir = ctx.get_player_direction(caster).ok_or_else(|| {
                    mlua::Error::runtime("build_cast_variant: caster direction not found")
                })?;
                let (dx, dy) = match dir {
                    0 => (0i16, -1i16),
                    1 => (1, 0),
                    2 => (0, 1),
                    3 => (-1, 0),
                    4 => (-1, 1),
                    5 => (1, 1),
                    6 => (-1, -1),
                    7 => (1, -1),
                    _ => (0, 0),
                };
                Ok((
                    (pos.x as i16 + dx) as u16,
                    (pos.y as i16 + dy) as u16,
                    pos.z,
                ))
            })?;
            build_position_variant(lua, x, y, z)
        }
    }
}

fn build_position_variant(lua: &Lua, x: u16, y: u16, z: u8) -> Result<mlua::Value, LuaError> {
    let table = lua.create_table()?;
    table.set("type", VARIANT_POSITION)?;
    let pos = lua.create_table()?;
    pos.set("x", x as i64)?;
    pos.set("y", y as i64)?;
    pos.set("z", z as i64)?;
    table.set("pos", pos)?;
    attach_variant_methods(lua, table)
}

/// Attach `getString` / `getNumber` / `getPosition` — TFS `pushVariant` metatable.
fn attach_variant_methods(lua: &Lua, table: mlua::Table) -> Result<mlua::Value, LuaError> {
    let mt = lua.create_table().map_err(LuaError::Init)?;
    let index = lua.create_table().map_err(LuaError::Init)?;
    index
        .set(
            "getString",
            lua.create_function(|_, t: mlua::Table| {
                Ok(t.get::<String>("string").unwrap_or_default())
            })
            .map_err(LuaError::Init)?,
        )
        .map_err(LuaError::Init)?;
    index
        .set(
            "getNumber",
            lua.create_function(|_, t: mlua::Table| {
                Ok(t.get::<u64>("number").unwrap_or(0))
            })
            .map_err(LuaError::Init)?,
        )
        .map_err(LuaError::Init)?;
    index
        .set(
            "getPosition",
            lua.create_function(|lua, t: mlua::Table| {
                use crate::userdata::position::PositionRef;
                if let Ok(pos) = t.get::<mlua::Table>("pos") {
                    let x: i64 = pos.get("x").unwrap_or(0);
                    let y: i64 = pos.get("y").unwrap_or(0);
                    let z: i64 = pos.get("z").unwrap_or(0);
                    let ud = lua.create_userdata(PositionRef {
                        x: x as u16,
                        y: y as u16,
                        z: z as u8,
                    })?;
                    return Ok(Value::UserData(ud));
                }
                Ok(Value::Nil)
            })
            .map_err(LuaError::Init)?,
        )
        .map_err(LuaError::Init)?;
    mt.set("__index", index).map_err(LuaError::Init)?;
    table.set_metatable(Some(mt));
    Ok(Value::Table(table))
}

/// `Variant(pos|creature|…)` — TFS `luaCreateVariant` / undead_legion.lua.
fn register_variant_constructor(lua: &Lua) -> Result<(), mlua::Error> {
    use crate::userdata::position::PositionRef;
    let f = lua.create_function(|lua, arg: Value| {
        match arg {
            Value::UserData(ud) => {
                if let Ok(pos) = ud.borrow::<PositionRef>() {
                    return build_position_variant(lua, pos.x, pos.y, pos.z)
                        .map_err(|e| mlua::Error::runtime(e.to_string()));
                }
                if let Ok(cref) = ud.borrow::<CreatureRef>() {
                    let table = lua.create_table()?;
                    table.set("type", VARIANT_NUMBER)?;
                    table.set("number", cref.0)?;
                    return attach_variant_methods(lua, table)
                        .map_err(|e| mlua::Error::runtime(e.to_string()));
                }
                Err(mlua::Error::runtime(
                    "Variant(): expected Position or Creature userdata",
                ))
            }
            Value::Table(t) => {
                // Already a variant-like table — attach methods.
                attach_variant_methods(lua, t).map_err(|e| mlua::Error::runtime(e.to_string()))
            }
            Value::String(s) => {
                let table = lua.create_table()?;
                table.set("type", VARIANT_STRING)?;
                table.set("string", s.to_str()?.to_string())?;
                attach_variant_methods(lua, table).map_err(|e| mlua::Error::runtime(e.to_string()))
            }
            Value::Integer(n) => {
                let table = lua.create_table()?;
                table.set("type", VARIANT_NUMBER)?;
                table.set("number", n as u64)?;
                attach_variant_methods(lua, table).map_err(|e| mlua::Error::runtime(e.to_string()))
            }
            _ => Err(mlua::Error::runtime(
                "Variant(): expected Position, Creature, string, number, or table",
            )),
        }
    })?;
    lua.globals().set("Variant", f)?;
    // `luascript.h` LuaVariantType_t
    let g = lua.globals();
    g.set("VARIANT_NUMBER", VARIANT_NUMBER)?;
    g.set("VARIANT_POSITION", VARIANT_POSITION)?;
    g.set("VARIANT_TARGETPOSITION", 3i32)?;
    g.set("VARIANT_STRING", VARIANT_STRING)?;
    Ok(())
}

// Keep register_spell_callback outside the removed block — already on impl above.

pub trait RegisterLuaFunctions {
    fn register_functions(&self, lua: &Lua) -> Result<(), mlua::Error>;
}

/// Class tables and stubs so `data/events/scripts/*.lua` can use `function Player:…`.
///
/// C++ reference: `LuaScriptInterface::registerClass` — `src/luascript.cpp`.
pub(crate) fn register_event_script_bootstrap(lua: &Lua) -> Result<(), mlua::Error> {
    let globals = lua.globals();

    globals.set("Player", lua.create_table()?)?;

    for name in [
        "Creature",
        "Monster",
        "Npc",
        "Game",
        "Tile",
        "Item",
        "Container",
    ] {
        globals.set(name, lua.create_table()?)?;
    }

    // `Channel(id, name)` — self-registering chat-channel constructor.
    //
    // Returns a plain Lua **table** (not userdata). Scripts attach hooks as table fields
    // (`function channel.onSpeak(...)`, `function channel.canJoin(...)`), which is a
    // `__newindex` write. Lua userdata rejects `__newindex` for arbitrary keys, so the
    // earlier `ChannelHandle` userdata raised "attempt to index a userdata value" the
    // moment a script defined its first hook. A table has no such restriction.
    //
    // C++ reference: `chat.cpp` `Chat::load` — channel registration (adapted from XML to
    // this self-registering Lua convention, mirroring the `Action`/`TalkAction` shape).
    let channel_constructor = lua.create_function(|lua, (id, name): (u16, String)| {
        let ch = lua.create_table()?;
        ch.set("id", id)?;
        ch.set("name", name)?;
        // Public flag stored under `_public`; the `public` key is the setter method below.
        ch.set("_public", false)?;
        // `channel:public(bool)` fluent setter (mirrors `talkaction:separator(...)`).
        ch.set(
            "public",
            lua.create_function(|_, (this, is_public): (mlua::Table, bool)| {
                this.set("_public", is_public)?;
                Ok(())
            })?,
        )?;
        // `channel:register()` — push the channel table into the loader's pending buffer;
        // `load_channel_script` drains it and reads id/name/_public + hook fields.
        ch.set(
            "register",
            lua.create_function(|lua, this: mlua::Table| {
                let pending: mlua::Table = lua.globals().get("_pending_channels")?;
                let len = pending.len()?;
                pending.set(len + 1, this)?;
                Ok(())
            })?,
        )?;
        Ok(ch)
    })?;
    globals.set("Channel", channel_constructor)?;

    // `Condition(type[, id])` — real `ConditionBuilder` userdata (LUA-4 §1.6).
    // Replaces the no-op soul stub. `player.lua`'s `soulCondition` build
    // (`Condition(CONDITION_SOUL, CONDITIONID_DEFAULT):setTicks(...)` /
    // `:setParameter(...)`) still loads unchanged — the builder supports both
    // `setTicks` and `setParameter` (regression guard, §4.1).
    // C++ reference: `luascript.cpp:11970-11984` `luaConditionCreate` —
    // `Condition(conditionType[, conditionId = CONDITIONID_COMBAT])`.
    // The second arg defaults to `CONDITIONID_COMBAT` (0) when omitted —
    // spell scripts call `Condition(CONDITION_LIGHT)` with one arg.
    let condition = lua.create_function(|lua, (ctype, cond_id): (i32, Option<i32>)| {
        let builder =
            crate::userdata::condition::ConditionBuilder::new(ctype, cond_id.unwrap_or(0));
        let ud = lua.create_userdata(builder)?;
        Ok(mlua::Value::UserData(ud))
    })?;
    globals.set("Condition", condition)?;

    // `ItemType(nameOrId)` — real `ItemTypeRef` userdata (CH-6). Resolves a
    // name string to a server item type id via `ScriptContext`, or wraps a
    // numeric id directly. `getId()` returns `0` if not found.
    // C++ reference: `luascript.cpp` `luaItemTypeCreate` — `items.h`.
    register_item_type_constructor(lua)?;

    // `string.splitTrimmed(sep)` and `string.trim()` — TFS Lua extensions
    // normally defined in `data/global.lua`. Defined here in the bootstrap so
    // talkaction scripts can use them without loading `global.lua` (which has
    // `dofile`/`os.time` dependencies not yet wired).
    // C++ reference: `data/global.lua:111-129` (TFS data pack).
    lua.load(
        r#"
        string.trim = function(str)
            return str:match'^()%s*$' and '' or str:match'^%s*(.*%S)'
        end
        string.splitTrimmed = function(str, sep)
            local res = {}
            for v in str:gmatch("([^" .. sep .. "]+)") do
                res[#res + 1] = v:trim()
            end
            return res
        end
        string.split = function(str, sep)
            local res = {}
            for v in str:gmatch("([^" .. sep .. "]+)") do
                res[#res + 1] = v
            end
            return res
        end
        "#,
    )
    .set_name("string_extensions")
    .exec()?;

    // `TalkAction(words)` — self-registering talkaction constructor (CH-6).
    //
    // Returns a plain Lua **table** (not userdata), mirroring the `Channel`
    // constructor pattern. Scripts attach `onSay` as a table field
    // (`function talkaction.onSay(player, words, param) ... end`), then call
    // `talkaction:register()` to push into the loader's pending buffer.
    //
    // C++ reference: `talkaction.h` `TalkAction` / `talkaction.cpp`
    // `TalkActions::registerLuaEvent`. Default separator is `" "` (space),
    // matching C++ `TalkAction::separator = "\""` → TFS data pack uses `" "`.
    let talkaction_constructor = lua.create_function(|lua, words: String| {
        let ta = lua.create_table()?;
        ta.set("words", words)?;
        ta.set("_separator", " ")?;
        // `talkaction:separator(sep)` fluent setter (mirrors C++
        // `TalkAction::setSeparator`).
        ta.set(
            "separator",
            lua.create_function(|_, (this, sep): (mlua::Table, String)| {
                this.set("_separator", sep)?;
                Ok(())
            })?,
        )?;
        // `talkaction:register()` — push the talkaction table into the loader's
        // pending buffer; `load_talkaction_script` drains it and reads
        // words/_separator + onSay.
        ta.set(
            "register",
            lua.create_function(|lua, this: mlua::Table| {
                let pending: mlua::Table = lua.globals().get("_pending_talkactions")?;
                let len = pending.len()?;
                pending.set(len + 1, this)?;
                Ok(())
            })?,
        )?;
        Ok(ta)
    })?;
    globals.set("TalkAction", talkaction_constructor)?;

    // `Action()` — self-registering action constructor (doors / food / levers).
    //
    // Plain Lua **table** (not userdata), same pattern as `TalkAction` / `Channel`.
    // Scripts set `function action.onUse(...)` then `:id` / `:aid` / `:register()`.
    //
    // C++ reference: `actions.h` `Action` / `actions.cpp` `Actions::registerLuaEvent`.
    let action_constructor = lua.create_function(|lua, ()| {
        let action = lua.create_table()?;
        action.set("_ids", lua.create_table()?)?;
        action.set("_aids", lua.create_table()?)?;
        // `action:id(...)` — append one or more item type ids.
        action.set(
            "id",
            lua.create_function(|_lua, (this, args): (mlua::Table, mlua::Variadic<Value>)| {
                let ids: mlua::Table = this.get("_ids")?;
                for arg in args.iter() {
                    let id = match arg {
                        Value::Integer(n) => *n as u16,
                        Value::Number(n) => *n as u16,
                        _ => continue,
                    };
                    let len = ids.len()?;
                    ids.set(len + 1, id)?;
                }
                Ok(this)
            })?,
        )?;
        // `action:aid(...)` — append action ids (TFS `actionItemMap`).
        action.set(
            "aid",
            lua.create_function(|_lua, (this, args): (mlua::Table, mlua::Variadic<Value>)| {
                let aids: mlua::Table = this.get("_aids")?;
                for arg in args.iter() {
                    let id = match arg {
                        Value::Integer(n) => *n as u16,
                        Value::Number(n) => *n as u16,
                        _ => continue,
                    };
                    let len = aids.len()?;
                    aids.set(len + 1, id)?;
                }
                Ok(this)
            })?,
        )?;
        action.set(
            "register",
            lua.create_function(|lua, this: mlua::Table| {
                let pending: mlua::Table = lua.globals().get("_pending_actions")?;
                let len = pending.len()?;
                pending.set(len + 1, this)?;
                Ok(())
            })?,
        )?;
        Ok(action)
    })?;
    globals.set("Action", action_constructor)?;

    // `Player(name)` — resolve an online player by name → `CreatureRef` userdata
    // or `nil`. LUA-4 §0.3 / `luascript.cpp` `luaPlayerCreate`.
    // Uses the scoped `ScriptContext::get_player_by_name` read.
    //
    // `Player` is already a class table (set above) for `function Player:method`
    // definitions. We set a `__call` metamethod on it so `Player(name)` works
    // as a constructor without losing the table semantics for method registration.
    let player_table: mlua::Table = globals.get("Player")?;
    let player_meta = lua.create_table()?;
    player_meta.set(
        "__call",
        lua.create_function(|lua, (_self, name): (mlua::Value, String)| {
            let id_opt = crate::context::current_ctx(|ctx| ctx.get_player_by_name(&name)).flatten();
            match id_opt {
                Some(id) => {
                    let ud = lua.create_userdata(crate::context::CreatureRef(id))?;
                    Ok(mlua::Value::UserData(ud))
                }
                None => Ok(mlua::Value::Nil),
            }
        })?,
    )?;
    player_table.set_metatable(Some(player_meta));

    // `Creature(id)` — resolve a creature by slotmap key bits → `CreatureRef`
    // userdata or `nil`. PC-3a Phase 3: `envenom_rune` / `soulfire_rune` use
    // `Creature(variant.number)`. C++ `luascript.cpp` `luaCreatureCreate`.
    // Keep `Creature` as a class table (for `function Creature:…` in
    // `functions.lua`) and attach `__call` like `Player(name)`.
    let creature_table: mlua::Table = globals.get("Creature")?;
    let creature_meta = lua.create_table()?;
    creature_meta.set(
        "__call",
        lua.create_function(|lua, (_self, id): (mlua::Value, u64)| {
            if id == 0 {
                return Ok(mlua::Value::Nil);
            }
            let exists = crate::context::current_ctx(|ctx| ctx.get_creature(id).is_some())
                .unwrap_or(true);
            if !exists {
                return Ok(mlua::Value::Nil);
            }
            let ud = lua.create_userdata(crate::context::CreatureRef(id))?;
            Ok(mlua::Value::UserData(ud))
        })?,
    )?;
    creature_table.set_metatable(Some(creature_meta));

    // `sendChannelMessage(channelId, type, message)` — LUA-4 §1.7.
    // Server-originated channel broadcast (anonymous speaker). Routes to
    // `LuaMutation::SendChannelMessage` via the mutation scope.
    // C++ reference: `chat.cpp` `sendChannelMessage` / `protocolgame.cpp`
    // `sendChannelMessage` (`0xAA` anonymous branch).
    let send_channel =
        lua.create_function(|_, (channel_id, speak_type, text): (u16, u8, String)| {
            crate::lua_mutation::call_lua_send_channel_message(channel_id, speak_type, text)
                .map_err(mlua::Error::runtime)?;
            Ok(())
        })?;
    globals.set("sendChannelMessage", send_channel)?;

    // `nextUseStaminaTime` — a mutable per-player stamina gate table read by
    // `data/events/scripts/player.lua` `onGainSkillTries`. Not a constant;
    // stays here alongside the other bootstrap state. The bare enum/flag
    // constants (CONDITION_SOUL, TALKTYPE_*, ACCOUNT_TYPE_*, PlayerFlag_*,
    // RETURNVALUE_*, APPLY_SKILL_MULTIPLIER, …) now live in `constants.rs`
    // (`register_constants`), called from `LuaRuntime::new` after this fn.
    globals.set("nextUseStaminaTime", lua.create_table()?)?;

    globals.set(
        "hasEventCallback",
        lua.create_function(|_, _: i32| Ok(false))?,
    )?;
    globals.set(
        "EventCallback",
        lua.create_function(|_, _: mlua::MultiValue| Ok(false))?,
    )?;

    Ok(())
}

/// `Game` table methods — PC-3a Phase 6 + Gap 5 (`Game.getWorldType` / `createMonster`).
/// C++ `luascript.cpp` `luaGameGetWorldType` / `luaGameCreateMonster`.
fn register_game_api(lua: &Lua) -> Result<(), mlua::Error> {
    use crate::lua_mutation::call_create_monster;
    use crate::userdata::position::PositionRef;
    let game = lua.create_table()?;
    game.set(
        "getWorldType",
        lua.create_function(|_, ()| {
            let wt = crate::context::current_ctx(|ctx| ctx.get_world_type()).unwrap_or(1);
            Ok(wt)
        })?,
    )?;
    game.set(
        "createMonster",
        lua.create_function(
            |lua, (name, pos, extended, force): (String, Value, Option<bool>, Option<bool>)| {
                let (x, y, z) = match pos {
                    Value::UserData(ud) => {
                        if let Ok(p) = ud.borrow::<PositionRef>() {
                            (p.x, p.y, p.z)
                        } else {
                            return Err(mlua::Error::runtime(
                                "Game.createMonster: position must be Position",
                            ));
                        }
                    }
                    Value::Table(t) => {
                        let x: i64 = t.get("x").or_else(|_| t.get(1))?;
                        let y: i64 = t.get("y").or_else(|_| t.get(2))?;
                        let z: i64 = t.get("z").or_else(|_| t.get(3))?;
                        (x as u16, y as u16, z as u8)
                    }
                    _ => {
                        return Err(mlua::Error::runtime(
                            "Game.createMonster: expected Position",
                        ));
                    }
                };
                match call_create_monster(
                    name,
                    x,
                    y,
                    z,
                    extended.unwrap_or(false),
                    force.unwrap_or(false),
                ) {
                    Ok(Some(id)) => {
                        let ud = lua.create_userdata(CreatureRef(id))?;
                        Ok(Value::UserData(ud))
                    }
                    Ok(None) => Ok(Value::Nil),
                    Err(e) => Err(mlua::Error::runtime(e)),
                }
            },
        )?,
    )?;
    lua.globals().set("Game", game)?;
    Ok(())
}

/// `doChallengeCreature(creature, target)` — PC-3a Phase 6.
/// C++ `luascript.cpp` `luaDoChallengeCreature` → `Monster::challengeCreature`.
fn register_do_challenge_creature(lua: &Lua) -> Result<(), mlua::Error> {
    let f = lua.create_function(|_, (creature, target): (Value, Value)| {
        let challenger = match creature {
            Value::UserData(ud) => ud.borrow::<crate::context::CreatureRef>()?.0,
            _ => {
                return Err(mlua::Error::runtime(
                    "doChallengeCreature: creature must be Creature userdata",
                ));
            }
        };
        let target_id = match target {
            Value::UserData(ud) => ud.borrow::<crate::context::CreatureRef>()?.0,
            _ => {
                return Err(mlua::Error::runtime(
                    "doChallengeCreature: target must be Creature userdata",
                ));
            }
        };
        crate::lua_mutation::call_do_challenge_creature(challenger, target_id)
            .map_err(mlua::Error::runtime)
    })?;
    lua.globals().set("doChallengeCreature", f)?;
    Ok(())
}

/// Minimal global functions for Track 1 PoC.
struct MinimalGlobalFunctions;

impl RegisterLuaFunctions for MinimalGlobalFunctions {
    fn register_functions(&self, lua: &Lua) -> Result<(), mlua::Error> {
        let globals = lua.globals();

        // debugPrint
        globals.set(
            "debugPrint",
            lua.create_function(|_, msg: String| {
                tracing::debug!("{}", msg);
                Ok(())
            })?,
        )?;

        // configManager (read-only access) — C++ `luaConfigManagerTable`.
        // Boolean keys are `ConfigManager::boolean_config_t` integers
        // (`configKeys.FREE_PREMIUM`); string keys (`"freePremium"`) also work.
        let config_manager = lua.create_table()?;
        config_manager.set(
            "getString",
            lua.create_function(|_, key: mlua::Value| {
                let name = config_key_to_lua_string(&key);
                let Some(name) = name else {
                    return Ok(Some(String::new()));
                };
                Ok(crate::context::current_ctx(|ctx| {
                    ctx.get_config_string(&name).unwrap_or_default()
                }))
            })?,
        )?;
        config_manager.set(
            "getNumber",
            lua.create_function(|_, _key: mlua::Value| {
                // TODO: broader integer config map
                Ok(Some(0i64))
            })?,
        )?;
        config_manager.set(
            "getBoolean",
            lua.create_function(|_, key: mlua::Value| {
                let name = config_key_to_lua_bool(&key);
                let Some(name) = name else {
                    return Ok(Some(false));
                };
                Ok(Some(crate::context::current_ctx(|ctx| {
                    ctx.get_config_bool(&name).unwrap_or(false)
                }).unwrap_or(false)))
            })?,
        )?;
        globals.set("configManager", config_manager)?;

        // getWorldTime() — returns world time in game-minutes (0..1439).
        globals.set(
            "getWorldTime",
            lua.create_function(|_, ()| {
                Ok(crate::context::current_ctx(|ctx| ctx.get_world_time()).unwrap_or(0))
            })?,
        )?;

        // getWorldLight() — returns level, color.
        globals.set(
            "getWorldLight",
            lua.create_function(|_, ()| {
                let (level, color) = crate::context::current_ctx(|ctx| ctx.get_world_light())
                    .unwrap_or((0xFF, 0xD7));
                Ok((level, color))
            })?,
        )?;

        // setWorldLight(level, color) — returns true if defaultWorldLight is false.
        globals.set(
            "setWorldLight",
            lua.create_function(|_, (level, color): (u8, u8)| {
                let ok = crate::lua_mutation::call_lua_set_world_light(level, color).unwrap_or(false);
                Ok(ok)
            })?,
        )?;

        Ok(())
    }
}

/// Map `configKeys.*` integer / `"freePremium"` string → `config.lua` key.
fn config_key_to_lua_bool(key: &mlua::Value) -> Option<&'static str> {
    match key {
        mlua::Value::String(s) => match s.to_str().ok()?.as_ref() {
            "freePremium" | "FREE_PREMIUM" => Some("freePremium"),
            "defaultWorldLight" | "DEFAULT_WORLD_LIGHT" => Some("defaultWorldLight"),
            _ => None,
        },
        mlua::Value::Integer(i) => match *i {
            // TVP `boolean_config_t::FREE_PREMIUM`
            7 => Some("freePremium"),
            // TVP `boolean_config_t::DEFAULT_WORLD_LIGHT`
            18 => Some("defaultWorldLight"),
            _ => None,
        },
        mlua::Value::Number(n) => {
            if (*n as i64) == 7 {
                Some("freePremium")
            } else {
                None
            }
        }
        _ => None,
    }
}

fn config_key_to_lua_string(key: &mlua::Value) -> Option<String> {
    match key {
        mlua::Value::String(s) => Some(s.to_str().ok()?.to_string()),
        _ => None,
    }
}

/// Lua runtime errors.
#[derive(Debug, thiserror::Error)]
pub enum LuaError {
    #[error("VM initialization failed: {0}")]
    Init(#[from] mlua::Error),

    #[error("Function registration failed: {0}")]
    Registration(mlua::Error),

    #[error("Script not found: {0}")]
    ScriptNotFound(String),

    #[error("Script syntax error: {0}")]
    SyntaxError(String),

    #[error("Script IO error for {0}: {1}")]
    ScriptIo(String, String),

    #[error("Missing global Lua function: {0}")]
    MissingFunction(String),

    #[error("Not implemented")]
    NotImplemented,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn workspace_data_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/events/scripts/player.lua")
    }

    fn workspace_data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    #[test]
    fn player_events_script_loads_with_bootstrap() {
        let path = workspace_data_path();
        if !path.exists() {
            return;
        }
        let mut runtime = LuaRuntime::new().expect("runtime");
        runtime
            .load_script(path.to_str().expect("utf8 path"))
            .expect("player.lua should load");
        // `player.lua` defines `onTurn` / `onLook` / … — no `onInventoryUpdate` in
        // the current data pack; smoke-test any registered Player method.
        runtime
            .register_table_method_callback("test::onTurn".to_string(), "Player", "onTurn")
            .expect("onTurn registered");
    }

    /// LUA-1 smoke test: every chat-channel script loads without
    /// `nil`-method / `nil`-constant failures, and the constants the active
    /// hook bodies compare against resolve to their 772 reference values.
    ///
    /// `ruleviolations.lua` is skipped by `load_chat_channel_scripts` (RVR
    /// non-goal), so we expect the remaining 8 channel scripts to register.
    #[test]
    fn channel_scripts_load_with_constants() {
        let data_root = workspace_data_root();
        let channels_dir = data_root.join("scripts/chatchannels");
        if !channels_dir.exists() {
            return;
        }

        let mut runtime = LuaRuntime::new().expect("runtime");

        // Spot-check the enum values the active hook bodies compare against.
        // Mirrors `constants.rs` tests; duplicated here so a regression in
        // either the registration call site or the values is caught.
        let globals = runtime.lua.globals();
        let get = |name: &str| globals.get::<i32>(name).expect(name);
        assert_eq!(get("ACCOUNT_TYPE_GOD"), 6);
        assert_eq!(get("TALKTYPE_CHANNEL_Y"), 5);
        assert_eq!(get("TALKTYPE_CHANNEL_O"), 12);
        assert_eq!(get("TALKTYPE_CHANNEL_R1"), 10);
        assert_eq!(get("PlayerFlag_CanTalkRedChannel"), 1 << 22);
        assert_eq!(get("PlayerFlag_TalkOrangeHelpChannel"), 1 << 23);
        assert_eq!(get("VOCATION_NONE"), 0);
        // §4.3 fix: CONDITIONID_DEFAULT must be -1, not the old wrong 0.
        assert_eq!(get("CONDITIONID_DEFAULT"), -1);
        // LUA-4: CONDITION_CHANNELMUTEDTICKS + CONDITION_PARAM_SUBID/TICKS.
        assert_eq!(get("CONDITION_CHANNELMUTEDTICKS"), 1 << 15);
        assert_eq!(get("CONDITION_PARAM_SUBID"), 45);
        assert_eq!(get("CONDITION_PARAM_TICKS"), 2);
        assert_eq!(get("RETURNVALUE_PLAYERWITHTHISNAMEISNOTONLINE"), 27);

        // Load every channel script — each must compile and self-register
        // without referencing a nil constant/method.
        let channels = crate::chat_channels::load_chat_channel_scripts(&mut runtime, &data_root)
            .expect("channel scripts load");
        // 8 active channels (ruleviolations.lua is skipped by the loader).
        assert_eq!(
            channels.len(),
            8,
            "expected 8 registered channels, got {}: {channels:?}",
            channels.len()
        );
    }
}
