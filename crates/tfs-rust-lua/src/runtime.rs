//! Lua runtime and VM management.
//!
//! This module provides the LuaRuntime struct which owns the mlua::Lua VM
//! and manages script registry and global function registration.

use mlua::{Lua, RegistryKey};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use crate::constants::register_constants;
use crate::context::{CreatureRef, ItemRef};
use crate::timer_events::{TimerEvents, execute_timer_event, register_add_event_stop_event};
use crate::userdata::{
    register_combat_metatable, register_condition_metatable, register_container_metatable,
    register_creature_metatable, register_group_metatable, register_item_metatable,
    register_item_type_constructor, register_item_type_metatable, register_position_metatable,
    register_spell_metatable, register_vocation_metatable, register_weapon_metatable,
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
    /// PC-3a: `onCastSpell` callback registry keys, keyed by spell words (lowercased).
    /// Populated during `load_spell_scripts` from `_pending_spell_callbacks`.
    /// C++ reference: `Event::loadCallback` / `getEvent` (`baseevents.cpp:136`,
    /// `luascript.cpp:363`) — stores the Lua function reference for later invocation.
    spell_callbacks: HashMap<String, RegistryKey>,
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
        register_item_metatable(&lua).map_err(LuaError::Registration)?;
        register_container_metatable(&lua).map_err(LuaError::Registration)?;
        register_vocation_metatable(&lua).map_err(LuaError::Registration)?;
        register_condition_metatable(&lua).map_err(LuaError::Registration)?;
        register_group_metatable(&lua).map_err(LuaError::Registration)?;
        register_position_metatable(&lua).map_err(LuaError::Registration)?;
        register_item_type_metatable(&lua).map_err(LuaError::Registration)?;
        register_event_script_bootstrap(&lua).map_err(LuaError::Registration)?;
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

        // Initialize pending channel buffer for Channel:register()
        let pending_channels = lua.create_table()?;
        lua.globals().set("_pending_channels", pending_channels)?;

        // Initialize pending talkaction buffer for TalkAction:register()
        let pending_talkactions = lua.create_table()?;
        lua.globals()
            .set("_pending_talkactions", pending_talkactions)?;

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
            spell_callbacks: HashMap::new(),
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

    /// PC-3a: Call a spell's `onCastSpell` callback.
    ///
    /// C++ reference: `InstantSpell::castSpell` → `LuaEnvironment::callLuaFunction`
    /// (`spells.cpp` / `luascript.cpp`). The callback receives `(creature, variant)`
    /// and returns `true` on success.
    ///
    /// Looks up the callback by spell words (lowercased) in the `spell_callbacks`
    /// map populated during `load_spell_scripts`.
    pub fn call_on_cast_spell(
        &self,
        spell_words: &str,
        creature: crate::context::CreatureId,
    ) -> Result<bool, LuaError> {
        let key = self.spell_callbacks.get(spell_words.to_lowercase().as_str());
        let Some(registry_key) = key else {
            // No Lua callback registered — the spell has no script-side cast logic.
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
        // The variant is a Lua table — for now we pass nil (the combat:execute
        // method resolves the variant from the caster position). Full variant
        // construction (target position, target creature) is a follow-up.
        function
            .call::<bool>((creature_ud, mlua::Value::Nil))
            .map_err(LuaError::Init)
    }

    /// PC-3a: Register a spell callback keyed by spell words.
    /// Called from `load_spell_scripts` after draining `_pending_spell_callbacks`.
    pub fn register_spell_callback(&mut self, words: &str, key: RegistryKey) {
        self.spell_callbacks.insert(words.to_lowercase(), key);
    }
}
pub trait RegisterLuaFunctions {
    fn register_functions(&self, lua: &Lua) -> Result<(), mlua::Error>;
}

/// Class tables and stubs so `data/events/scripts/*.lua` can use `function Player:…`.
///
/// C++ reference: `LuaScriptInterface::registerClass` — `src/luascript.cpp`.
fn register_event_script_bootstrap(lua: &Lua) -> Result<(), mlua::Error> {
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

    // `Condition(type, id)` — real `ConditionBuilder` userdata (LUA-4 §1.6).
    // Replaces the no-op soul stub. `player.lua`'s `soulCondition` build
    // (`Condition(CONDITION_SOUL, CONDITIONID_DEFAULT):setTicks(...)` /
    // `:setParameter(...)`) still loads unchanged — the builder supports both
    // `setTicks` and `setParameter` (regression guard, §4.1).
    // C++ reference: `luascript.cpp` `luaCreateCondition` — `condition.cpp`.
    let condition = lua.create_function(|lua, (ctype, cond_id): (i32, i32)| {
        let builder = crate::userdata::condition::ConditionBuilder::new(ctype, cond_id);
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

        // configManager (read-only access)
        let config_manager = lua.create_table()?;
        config_manager.set(
            "getString",
            lua.create_function(|_, _key: String| {
                // TODO: Implement config lookup
                Ok(Some(String::new()))
            })?,
        )?;
        config_manager.set(
            "getNumber",
            lua.create_function(|_, _key: String| {
                // TODO: Implement config lookup
                Ok(Some(0))
            })?,
        )?;
        config_manager.set(
            "getBoolean",
            lua.create_function(|_, _key: String| {
                // TODO: Implement config lookup
                Ok(Some(false))
            })?,
        )?;
        globals.set("configManager", config_manager)?;

        Ok(())
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
        runtime
            .register_table_method_callback(
                "test::onInventoryUpdate".to_string(),
                "Player",
                "onInventoryUpdate",
            )
            .expect("onInventoryUpdate registered");
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
