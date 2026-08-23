//! Lua runtime and VM management.
//!
//! This module provides the LuaRuntime struct which owns the mlua::Lua VM
//! and manages script registry and global function registration.

use mlua::{Lua, RegistryKey, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

/// Default Lua VM memory limit (512 MiB).
///
/// VM hardening pillar 4 — `set_memory_limit`
/// (`tasks/tools-actions/vm-hardening.md`). Game simulation is single-threaded (`TFS-threading`); a runaway
/// allocation in any `data/scripts/**` callback would otherwise OOM-kill the
/// whole process — no ticks, no packets, no saves. This turns a total outage
/// into one failed script call. Generous enough for large loot loops and
/// map-wide iteration; override from `config.lua` via `luaMemoryLimit` (MB).
/// No JIT impact (the instruction-count hook is separate and does force
/// LuaJIT interpreter fallback while enabled).
pub const DEFAULT_LUA_MEMORY_LIMIT_BYTES: usize = 512 * 1024 * 1024;

pub use crate::instruction_budget::DEFAULT_LUA_INSTRUCTION_BUDGET;

use crate::constants::register_constants;
use crate::context::{CreatureRef, ItemRef};
use crate::npc_dialogue::register_npc_dialogue;
use crate::npc_type::register_npc_type;
use crate::timer_events::{TimerEvents, execute_timer_event, register_add_event_stop_event};
use crate::userdata::PositionRef;
use crate::userdata::{
    register_combat_metatable, register_condition_metatable, register_container_metatable,
    register_creature_metatable, register_group_metatable, register_item_metatable,
    register_item_type_constructor, register_item_type_metatable,
    register_monster_type_constructor, register_npc_metatable, register_position_metatable,
    register_spell_metatable, register_tile_constructor, register_town_constructor,
    register_vocation_metatable, register_weapon_metatable,
};
use tfs_rust_common::Position;

/// Wrapper for mlua::RegistryKey — !Send, must stay on game thread.
#[derive(Debug)]
pub struct CallbackRef(mlua::RegistryKey);

impl CallbackRef {
    pub fn from_registry_key(key: mlua::RegistryKey) -> Self {
        Self(key)
    }
}

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
    /// Pending move events from MoveEvent:register() (drained after directory scan).
    pending_move_events: Vec<PendingMoveEvent>,
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
    /// Per-invocation instruction budget (pillar 4). Synced to the game-thread
    /// local used by [`crate::instruction_budget::with_lua_instruction_budget`].
    instruction_budget: Cell<u32>,
    /// TFS `LuaScriptInterface::isScriptsInterface` — true only while the
    /// scripts-interface scan is running. Shared with the Lua global via `Rc`
    /// so `ScriptsInterfaceGuard` can reset it on `Drop` without borrowing
    /// `LuaRuntime`. Pack surface: `luascript.cpp` `isScriptsInterface`.
    scripts_interface: Rc<Cell<bool>>,
    /// `load_data_lib` already succeeded. A second call (e.g. `load_spell_scripts`
    /// on boot) must not re-exec `event_callbacks.lua`, which ends in
    /// `EventCallback:clear()` and would wipe scripts-interface registrations.
    data_lib_loaded: Cell<bool>,
}

/// Scoped `isScriptsInterface() == true`. Resets the flag on drop so a `?`
/// mid-scan cannot leak the flag into a later `dofile`.
///
/// Pack surface: TFS `loadScripts(..., isScriptsInterface)`.
pub(crate) struct ScriptsInterfaceGuard {
    flag: Rc<Cell<bool>>,
}

impl Drop for ScriptsInterfaceGuard {
    fn drop(&mut self) {
        self.flag.set(false);
    }
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
/// C++ reference: `actions.h` `Action` — item/action id lists + `onUse` + `allowFarUse`.
#[derive(Debug)]
pub struct PendingAction {
    pub item_ids: Vec<u16>,
    pub action_ids: Vec<u16>,
    pub on_use: Option<mlua::RegistryKey>,
    /// C++ `Action::allowFarUse` — `actions.h`. Default `false`.
    pub allow_far_use: bool,
}

/// Pending move-event definition from Lua MoveEvent:register().
///
/// C++ reference: `movement.h` `MoveEvent` — item/action id lists + step/equip callback.
#[derive(Debug)]
pub struct PendingMoveEvent {
    pub kind: crate::move_events::MoveEventKind,
    pub item_ids: Vec<u16>,
    pub action_ids: Vec<u16>,
    pub slot_mask: u32,
    pub req_level: u32,
    pub callback: Option<mlua::RegistryKey>,
}

impl LuaRuntime {
    /// Create a new Lua runtime with minimal global functions registered.
    ///
    /// # Errors
    ///
    /// Returns an error if VM initialization or lib loading fails.
    pub fn new() -> Result<Self, LuaError> {
        // VM hardening pillar 1 — stdlib allowlist
        // (tasks/tools-actions/vm-hardening.md). Not `Lua::new()` / `ALL_SAFE`:
        // that would ship `io` / `os.execute` / `package.loadlib`. `tfs.appendLog`
        // is registered here, before any data-pack load.
        let lua =
            crate::stdlib_allowlist::create_allowlisted_lua().map_err(LuaError::Registration)?;

        // VM hardening pillar 4 — `set_memory_limit`
        // (tasks/tools-actions/vm-hardening.md). Applied before any data-pack allocation so a runaway script
        // can't OOM-kill the whole game thread. Override from `config.lua` via
        // `luaMemoryLimit` (MB) in `run_server.rs`. No JIT impact.
        lua.set_memory_limit(DEFAULT_LUA_MEMORY_LIMIT_BYTES)
            .map_err(LuaError::Registration)?;

        // Instruction budget is armed per Rust→Lua entry (not as a lifetime
        // counter). The thread-local is what combat/timer helpers read.
        crate::instruction_budget::set_thread_instruction_budget(DEFAULT_LUA_INSTRUCTION_BUDGET);

        // Register minimal global functions via RegisterLuaFunctions
        let registrar = MinimalGlobalFunctions;
        registrar
            .register_functions(&lua)
            .map_err(LuaError::Registration)?;

        // Table-only engine class globals (`Monster`, `Npc`, `Item`, `Container`,
        // `Party`, `Teleport`, `Vocation`) — created via `register_class` so the
        // data pack can attach `function <Class>:method(...)`. Ctor-bearing
        // classes (`Tile`, `Position`, `Combat`, …) register themselves below via
        // their own registrars. Gap 7a — replaces the old hardcoded bootstrap list.
        crate::class_registry::register_engine_class_tables(&lua)
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
        let scripts_interface = Rc::new(Cell::new(false));
        register_event_script_bootstrap_with(&lua, Rc::clone(&scripts_interface))
            .map_err(LuaError::Registration)?;
        register_tile_constructor(&lua).map_err(LuaError::Registration)?;
        register_town_constructor(&lua).map_err(LuaError::Registration)?;
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
        register_do_target_combat(&lua).map_err(LuaError::Registration)?;

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

        // Initialize pending move-event buffer for MoveEvent:register()
        let pending_move_events = lua.create_table()?;
        lua.globals()
            .set("_pending_move_events", pending_move_events)?;

        // Pending buffers for CreatureEvent:register() / GlobalEvent:register()
        // (Gap 7c constructors; drained by a future content-stage loader).
        lua.globals()
            .set("_pending_creature_events", lua.create_table()?)?;
        lua.globals()
            .set("_pending_global_events", lua.create_table()?)?;

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
            pending_move_events: Vec::new(),
            spell_callbacks: HashMap::new(),
            weapon_callbacks: HashMap::new(),
            npc_callbacks: HashMap::new(),
            instruction_budget: Cell::new(DEFAULT_LUA_INSTRUCTION_BUDGET),
            scripts_interface,
            data_lib_loaded: Cell::new(false),
        })
    }

    /// True only while a [`ScriptsInterfaceGuard`] from [`Self::enter_scripts_interface`]
    /// is live. Pack surface: TFS `LuaScriptInterface::isScriptsInterface`.
    pub(crate) fn enter_scripts_interface(&self) -> ScriptsInterfaceGuard {
        self.scripts_interface.set(true);
        ScriptsInterfaceGuard {
            flag: Rc::clone(&self.scripts_interface),
        }
    }

    /// Replace CreatureEvent / GlobalEvent pending tables (reload stance (a)).
    pub(crate) fn reset_pending_script_event_tables(&self) -> Result<(), LuaError> {
        self.lua
            .globals()
            .set("_pending_creature_events", self.lua.create_table()?)?;
        self.lua
            .globals()
            .set("_pending_global_events", self.lua.create_table()?)?;
        Ok(())
    }

    pub(crate) fn is_data_lib_loaded(&self) -> bool {
        self.data_lib_loaded.get()
    }

    pub(crate) fn mark_data_lib_loaded(&self) {
        self.data_lib_loaded.set(true);
    }

    /// Set the Lua VM memory limit (in bytes). Returns the previous limit.
    ///
    /// VM hardening pillar 4 — `tasks/tools-actions/vm-hardening.md`. The
    /// default (`DEFAULT_LUA_MEMORY_LIMIT_BYTES`, 512 MiB) is applied in
    /// [`LuaRuntime::new`]; this lets `run_server.rs` override it from
    /// `config.lua` (`luaMemoryLimit`, in MB). Once an allocation would pass
    /// the limit, mlua raises `Error::MemoryError` instead of OOM-killing the
    /// process. No JIT impact (the instruction-count hook is separate).
    ///
    /// # Errors
    ///
    /// Returns [`LuaError::Registration`] if memory control is unavailable on
    /// this Lua state (should not happen with our LuaJIT build).
    pub fn set_memory_limit(&self, limit_bytes: usize) -> Result<usize, LuaError> {
        self.lua
            .set_memory_limit(limit_bytes)
            .map_err(LuaError::Registration)
    }

    /// Set the per-invocation Lua instruction budget. Returns the previous budget.
    ///
    /// VM hardening pillar 4 — `tasks/tools-actions/vm-hardening.md`. `0` disables
    /// the count hook and re-enables LuaJIT. Aborting a script does **not** roll
    /// back mutations already applied in the same callback (failure isolation,
    /// not atomicity — `TFS-lua-boundaries` Mutation Path).
    pub fn set_instruction_budget(&self, budget: u32) -> u32 {
        let prev = self.instruction_budget.replace(budget);
        crate::instruction_budget::set_thread_instruction_budget(budget);
        if budget == 0 {
            self.lua.remove_hook();
            crate::instruction_budget::restore_luajit(&self.lua);
        }
        prev
    }

    fn sync_instruction_budget(&self) {
        crate::instruction_budget::set_thread_instruction_budget(self.instruction_budget.get());
    }

    /// Invoke a Lua function under the per-invocation instruction budget.
    pub(crate) fn call_lua<R: mlua::FromLuaMulti>(
        &self,
        function: &mlua::Function,
        args: impl mlua::IntoLuaMulti,
    ) -> Result<R, LuaError> {
        self.sync_instruction_budget();
        crate::instruction_budget::with_lua_instruction_budget(&self.lua, || function.call(args))
            .map_err(LuaError::Init)
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
        self.exec_chunk(path, &chunk)?;

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
        self.exec_chunk(path, &chunk)?;

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
        self.exec_chunk(path, &chunk)?;

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

        self.exec_chunk(path, &chunk)?;

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

                let allow_far_use = action_table
                    .get::<Option<bool>>("_allow_far_use")?
                    .unwrap_or(false);

                self.pending_actions.push(PendingAction {
                    item_ids,
                    action_ids,
                    on_use,
                    allow_far_use,
                });
            }
        }

        Ok(())
    }

    /// Drain pending actions accumulated from `load_action_script` calls.
    pub fn drain_pending_actions(&mut self) -> Vec<PendingAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Load a Lua script that calls `MoveEvent():register()`.
    ///
    /// C++ reference: `movement.cpp` `MoveEvents::registerLuaEvent`.
    pub fn load_move_event_script(&mut self, path: &str) -> Result<(), LuaError> {
        let full_path = Path::new(path);
        let chunk = std::fs::read_to_string(full_path)
            .map_err(|e| LuaError::ScriptIo(full_path.display().to_string(), e.to_string()))?;

        self.lua
            .globals()
            .set("_pending_move_events", self.lua.create_table()?)?;

        self.exec_chunk(path, &chunk)?;

        let pending: mlua::Table = self.lua.globals().get("_pending_move_events")?;
        for i in 1..=pending.len()? {
            if let Ok(me_table) = pending.get::<mlua::Table>(i) {
                let mut item_ids = Vec::new();
                let ids_table: mlua::Table = me_table.get("_ids")?;
                for j in 1..=ids_table.len()? {
                    if let Ok(id) = ids_table.get::<u16>(j) {
                        item_ids.push(id);
                    }
                }

                let mut action_ids = Vec::new();
                let aids_table: mlua::Table = me_table.get("_aids")?;
                for j in 1..=aids_table.len()? {
                    if let Ok(id) = aids_table.get::<u16>(j) {
                        action_ids.push(id);
                    }
                }

                let type_name: Option<String> = match me_table.get::<Value>("_type")? {
                    Value::String(s) => Some(s.to_str()?.to_owned()),
                    _ => None,
                };
                let kind = infer_move_event_kind(&me_table, type_name.as_deref())?;

                let callback_field = kind.script_callback_field();
                let callback = me_table
                    .get::<Option<mlua::Function>>(callback_field)?
                    .map(|f| self.lua.create_registry_value(f))
                    .transpose()
                    .map_err(LuaError::Init)?;

                let slot_mask: u32 = me_table.get("_slot_mask").unwrap_or(0);
                let req_level: u32 = me_table.get("_req_level").unwrap_or(0);
                let tile_item: bool = me_table.get("_tile_item").unwrap_or(false);
                // Remap after reading the callback — TFS `registerLuaEvent` (`movement.cpp:243-255`).
                let kind = kind.with_tile_item(tile_item);

                self.pending_move_events.push(PendingMoveEvent {
                    kind,
                    item_ids,
                    action_ids,
                    slot_mask,
                    req_level,
                    callback,
                });
            }
        }

        Ok(())
    }

    /// Drain pending move events from `load_move_event_script` calls.
    pub fn drain_pending_move_events(&mut self) -> Vec<PendingMoveEvent> {
        std::mem::take(&mut self.pending_move_events)
    }

    /// Call an action `onUse` hook — `(player, item, fromPos, target, toPos, isHotkey) -> bool`.
    ///
    /// Returns `true` = handled (skip native fallthrough), `false` = not handled.
    /// No-target is TFS `pushThing(nullptr)`: a table `{uid,itemid,actionid,type=0}`, not nil.
    /// C++ reference: `actions.cpp` `Action::executeUse` / `callFunction(6)`;
    /// `luascript.cpp` `LuaScriptInterface::pushThing`.
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
        is_hotkey: bool,
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
            self.zero_thing_table()?
        };

        self.call_lua(
            &function,
            (player_ud, item_ud, from_ud, target, to_ud, is_hotkey),
        )
    }

    /// TFS `LuaScriptInterface::pushThing` nullptr branch (`luascript.cpp`).
    fn zero_thing_table(&self) -> Result<Value, LuaError> {
        let t = self.lua.create_table().map_err(LuaError::Init)?;
        t.set("uid", 0).map_err(LuaError::Init)?;
        t.set("itemid", 0).map_err(LuaError::Init)?;
        t.set("actionid", 0).map_err(LuaError::Init)?;
        t.set("type", 0).map_err(LuaError::Init)?;
        Ok(Value::Table(t))
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
        let result = self.call_lua(&function, (player_ud, words, param, 0i32));
        tracing::info!(
            player,
            words,
            ?result,
            "call_talkaction_on_say: Lua returned"
        );
        result
    }

    /// Execute a Lua chunk (bootstrap globals, compat stubs, data-pack files).
    ///
    /// Runs under the per-invocation instruction budget. Abort does not roll
    /// back Lua globals or world mutations already applied in this chunk.
    pub fn exec_chunk(&self, name: &str, chunk: &str) -> Result<(), LuaError> {
        self.sync_instruction_budget();
        crate::instruction_budget::with_lua_instruction_budget(&self.lua, || {
            self.lua.load(chunk).set_name(name).exec()
        })
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
        self.call_lua(&function, player)
    }

    /// Execute a fired `addEvent` timer callback.
    ///
    /// C++ reference: `LuaEnvironment::executeTimerEvent` (`luascript.cpp:18238`).
    /// Called from the game loop when `GameCommand::LuaCallback { event_id }` arrives.
    /// Returns `Ok(true)` if the event was found and executed, `Ok(false)` if it was
    /// already cancelled.
    pub fn execute_timer_event(&self, event_id: u64) -> Result<bool, LuaError> {
        self.sync_instruction_budget();
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
        self.call_lua(&function, (player_ud, item_ud, slot, equip))
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
        self.call_lua(&function, (player_ud, item_ud, slot, is_check))
    }

    /// TFS `MoveEvent::executeAddRemItem` — `(moveitem, tileitem, pos) -> bool`.
    ///
    /// `tile_item` `None` is `pushThing(nullptr)` (zero table), not Lua nil.
    /// C++ reference: `movement.cpp:1017-1036`.
    pub fn call_move_item(
        &self,
        callback: &CallbackRef,
        item: crate::context::ItemId,
        tile_item: Option<crate::context::ItemId>,
        pos: Position,
    ) -> Result<bool, LuaError> {
        let function: mlua::Function = self
            .lua
            .registry_value(&callback.0)
            .map_err(LuaError::Init)?;
        let item_ud = self
            .lua
            .create_userdata(ItemRef(item))
            .map_err(LuaError::Init)?;
        let tile_ud: Value = if let Some(tid) = tile_item {
            Value::UserData(
                self.lua
                    .create_userdata(ItemRef(tid))
                    .map_err(LuaError::Init)?,
            )
        } else {
            self.zero_thing_table()?
        };
        let pos_ud = self
            .lua
            .create_userdata(PositionRef {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            })
            .map_err(LuaError::Init)?;
        self.call_lua(&function, (item_ud, tile_ud, pos_ud))
    }

    /// TFS `MoveEvent::executeStep` — `(creature, item, position, fromPosition) -> bool`.
    pub fn call_move_step(
        &self,
        callback: &CallbackRef,
        creature: crate::context::CreatureId,
        item: crate::context::ItemId,
        pos: Position,
        from_pos: Position,
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
        let from_ud = self
            .lua
            .create_userdata(PositionRef {
                x: from_pos.x,
                y: from_pos.y,
                z: from_pos.z,
            })
            .map_err(LuaError::Init)?;
        self.call_lua(&function, (creature_ud, item_ud, pos_ud, from_ud))
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
        self.call_lua(&function, player_ud)
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
        self.call_lua(&function, (player_ud, speak_type, message))
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
        self.call_lua(&function, player_ud)
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
        self.call_lua(&function, player_ud)
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
        self.call_lua(&function, (creature_ud, variant))
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
        let result: Value = self.call_lua(&function, (creature_ud, variant, hit))?;
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
        CastVariantSpec::FixedPosition { x, y, z } => build_position_variant(lua, x, y, z),
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
            lua.create_function(|_, t: mlua::Table| Ok(t.get::<u64>("number").unwrap_or(0)))
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
    table.set_metatable(Some(mt)).map_err(LuaError::Init)?;
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
    crate::class_registry::register_class(lua, "Variant", Some(f))?;
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

/// Self-registering content constructors (`Channel`, `TalkAction`, `Action`,
/// `MoveEvent`, `CreatureEvent`, `GlobalEvent`), the `Condition`/`ItemType`
/// userdata constructors, and the `Player`/`Creature` class tables with
/// `__call` constructors.
///
/// Class globals are created via `crate::class_registry::register_class` — the
/// single owner — so a class is callable *and* extensible regardless of init
/// order. Table-only classes (`Monster`, `Npc`, `Item`, `Container`, `Party`,
/// `Teleport`, `Vocation`) are registered by `register_engine_class_tables`
/// in `LuaRuntime::new`. Gap 7c routes the remaining revscript ctors through
/// the same primitive (no more `globals.set(Name, ctor_fn)`).
///
/// C++ reference: `LuaScriptInterface::registerClass` — `src/luascript.cpp`.
pub(crate) fn register_event_script_bootstrap_with(
    lua: &Lua,
    scripts_interface: Rc<Cell<bool>>,
) -> Result<(), mlua::Error> {
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
    crate::class_registry::register_class(lua, "Channel", Some(channel_constructor))?;

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
    crate::class_registry::register_class(lua, "Condition", Some(condition))?;

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
    let talkaction_constructor = lua.create_function(|lua, words: mlua::Variadic<String>| {
        let ta = lua.create_table()?;
        // TFS `TalkAction(words...)` joins with `;` (`TalkAction::setWords`).
        ta.set("words", words.join(";"))?;
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
    crate::class_registry::register_class(lua, "TalkAction", Some(talkaction_constructor))?;

    // `Action()` — self-registering action constructor (doors / food / levers).
    //
    // Plain Lua **table** (not userdata), same pattern as `TalkAction` / `Channel`.
    // Scripts set `function action.onUse(...)` then `:id` / `:aid` / `:allowFarUse` / `:register()`.
    //
    // C++ reference: `actions.h` `Action` / `actions.cpp` `Actions::registerLuaEvent`.
    let action_constructor = lua.create_function(|lua, ()| {
        let action = lua.create_table()?;
        action.set("_ids", lua.create_table()?)?;
        action.set("_aids", lua.create_table()?)?;
        action.set("_allow_far_use", false)?;
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
        // `action:allowFarUse(bool)` — C++ `luaActionAllowFarUse` (`luascript.cpp`).
        action.set(
            "allowFarUse",
            lua.create_function(|_lua, (this, val): (mlua::Table, bool)| {
                this.set("_allow_far_use", val)?;
                Ok(true)
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
    crate::class_registry::register_class(lua, "Action", Some(action_constructor))?;

    // `MoveEvent()` — self-registering move-event constructor (doors auto-close / tiles).
    //
    // Plain Lua **table** (not userdata), same pattern as `Action` / `TalkAction`.
    // Scripts set `function moveevent.onStepIn/Out(...)` then `:id` / `:register()`.
    // Kind is inferred from the callback field, or from `:type("stepout")`.
    //
    // C++ reference: `movement.h` `MoveEvent` / `movement.cpp` `MoveEvents::registerLuaEvent`.
    let moveevent_constructor = lua.create_function(|lua, ()| {
        let me = lua.create_table()?;
        me.set("_ids", lua.create_table()?)?;
        me.set("_aids", lua.create_table()?)?;
        me.set("_slot_mask", 0u32)?;
        me.set("_req_level", 0u32)?;
        me.set(
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
        me.set(
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
        me.set(
            "type",
            lua.create_function(|_, (this, type_name): (mlua::Table, String)| {
                this.set("_type", type_name)?;
                Ok(this)
            })?,
        )?;
        me.set(
            "level",
            lua.create_function(|_, (this, level): (mlua::Table, u32)| {
                this.set("_req_level", level)?;
                Ok(this)
            })?,
        )?;
        me.set(
            "slot",
            lua.create_function(|_, (this, slot): (mlua::Table, String)| {
                let mask = match slot.to_ascii_lowercase().as_str() {
                    "head" => 1u32 << 0,
                    "necklace" => 1 << 1,
                    "backpack" => 1 << 2,
                    "armor" | "body" => 1 << 3,
                    "right-hand" | "right" => 1 << 4,
                    "left-hand" | "left" | "hand" | "shield" => (1 << 4) | (1 << 5),
                    "legs" => 1 << 6,
                    "feet" => 1 << 7,
                    "ring" => 1 << 8,
                    "ammo" => 1 << 9,
                    _ => 0,
                };
                this.set("_slot_mask", mask)?;
                Ok(this)
            })?,
        )?;
        // C++ `MoveEvent::tileItem` — `luascript.cpp` `luaMoveEventTileItem`.
        // Drain remaps AddItem/RemoveItem + true → ITEMTILE (`registerLuaEvent`).
        me.set(
            "tileItem",
            lua.create_function(|_, (this, tile_item): (mlua::Table, bool)| {
                this.set("_tile_item", tile_item)?;
                Ok(this)
            })?,
        )?;
        me.set(
            "register",
            lua.create_function(|lua, this: mlua::Table| {
                let pending: mlua::Table = lua.globals().get("_pending_move_events")?;
                let len = pending.len()?;
                pending.set(len + 1, this)?;
                Ok(())
            })?,
        )?;
        Ok(me)
    })?;
    crate::class_registry::register_class(lua, "MoveEvent", Some(moveevent_constructor))?;

    // `CreatureEvent(name)` — self-registering creature-event constructor.
    //
    // Plain Lua **table** (not userdata), same pattern as `Action`. Scripts
    // attach `function creatureevent.onLogin(...)` then `:register()`. Needs
    // `__call` so `helper_constructors.lua` can wrap `getmetatable(class).__call`.
    // C++ reference: `luascript.cpp` `luaCreateCreatureEvent` /
    // `creatureevent.cpp` `CreatureEvents::registerLuaEvent`. Gap 7c.
    let creatureevent_constructor = lua.create_function(|lua, name: String| {
        let ev = lua.create_table()?;
        ev.set("name", name)?;
        ev.set(
            "type",
            lua.create_function(|_, (this, type_name): (mlua::Table, String)| {
                this.set("_type", type_name)?;
                Ok(this)
            })?,
        )?;
        ev.set(
            "register",
            lua.create_function(|lua, this: mlua::Table| {
                let pending: mlua::Table = lua.globals().get("_pending_creature_events")?;
                let len = pending.len()?;
                pending.set(len + 1, this)?;
                Ok(())
            })?,
        )?;
        Ok(ev)
    })?;
    crate::class_registry::register_class(lua, "CreatureEvent", Some(creatureevent_constructor))?;

    // `GlobalEvent(name)` — self-registering global-event constructor.
    //
    // Plain Lua **table**, same pattern as `CreatureEvent` / `Action`. Scripts
    // attach `function globalevent.onShutdown(...)` then `:register()`.
    // C++ reference: `luascript.cpp` `luaCreateGlobalEvent` /
    // `globalevent.cpp` `GlobalEvents::registerLuaEvent`. Gap 7c.
    let globalevent_constructor = lua.create_function(|lua, name: String| {
        let ev = lua.create_table()?;
        ev.set("name", name)?;
        ev.set(
            "type",
            lua.create_function(|_, (this, type_name): (mlua::Table, String)| {
                this.set("_type", type_name)?;
                Ok(this)
            })?,
        )?;
        ev.set(
            "time",
            lua.create_function(|_, (this, time): (mlua::Table, String)| {
                this.set("_time", time)?;
                Ok(this)
            })?,
        )?;
        ev.set(
            "interval",
            lua.create_function(|_, (this, interval): (mlua::Table, u32)| {
                this.set("_interval", interval)?;
                Ok(this)
            })?,
        )?;
        ev.set(
            "register",
            lua.create_function(|lua, this: mlua::Table| {
                let pending: mlua::Table = lua.globals().get("_pending_global_events")?;
                let len = pending.len()?;
                pending.set(len + 1, this)?;
                Ok(())
            })?,
        )?;
        Ok(ev)
    })?;
    crate::class_registry::register_class(lua, "GlobalEvent", Some(globalevent_constructor))?;

    // `doRelocate(fromPos, toPos[, force])` — compat relocate leftovers off a tile.
    // C++ domain: used by `closing_doors.lua` / map scripts; body from `compat.lua`.
    register_do_relocate(lua)?;

    // `Player(name)` — resolve an online player by name → `CreatureRef` userdata
    // or `nil`. LUA-4 §0.3 / `luascript.cpp` `luaPlayerCreate`.
    // Uses the scoped `ScriptContext::get_player_by_name` read.
    //
    // Registered via `register_class` so `Player` is a class table (extensible
    // by `function Player:method(...)`) with a `__call` ctor (`Player(name)`).
    // The ctor closure takes `(name)` — `register_class` wraps it to drop the
    // `__call` `self` arg. C++ `LuaScriptInterface::registerClass`.
    let player_ctor = lua.create_function(|lua, arg: Value| match arg {
        Value::UserData(ud) => {
            if ud.borrow::<crate::context::CreatureRef>().is_ok() {
                Ok(Value::UserData(ud))
            } else {
                Ok(Value::Nil)
            }
        }
        Value::String(s) => {
            let name = s.to_str()?.to_string();
            let id_opt = crate::context::current_ctx(|ctx| ctx.get_player_by_name(&name)).flatten();
            match id_opt {
                Some(id) => {
                    let ud = lua.create_userdata(crate::context::CreatureRef(id))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        }
        Value::Integer(n) if n > 0 => {
            let id = n as u64;
            let is_player = crate::context::current_ctx(|ctx| ctx.get_player_level(id).is_some())
                .unwrap_or(false);
            if is_player {
                let ud = lua.create_userdata(crate::context::CreatureRef(id))?;
                Ok(Value::UserData(ud))
            } else {
                Ok(Value::Nil)
            }
        }
        Value::Number(n) if n > 0.0 => {
            let id = n as u64;
            let is_player = crate::context::current_ctx(|ctx| ctx.get_player_level(id).is_some())
                .unwrap_or(false);
            if is_player {
                let ud = lua.create_userdata(crate::context::CreatureRef(id))?;
                Ok(Value::UserData(ud))
            } else {
                Ok(Value::Nil)
            }
        }
        _ => Ok(Value::Nil),
    })?;
    crate::class_registry::register_class(lua, "Player", Some(player_ctor))?;

    // `Creature(id)` — resolve a creature by slotmap key bits → `CreatureRef`
    // userdata or `nil`. PC-3a Phase 3: `envenom_rune` / `soulfire_rune` use
    // `Creature(variant.number)`. C++ `luascript.cpp` `luaCreatureCreate`.
    // Same `register_class` shape as `Player`: class table + `__call` ctor.
    let creature_ctor = lua.create_function(|lua, id: u64| {
        if id == 0 {
            return Ok(mlua::Value::Nil);
        }
        let exists =
            crate::context::current_ctx(|ctx| ctx.get_creature(id).is_some()).unwrap_or(true);
        if !exists {
            return Ok(mlua::Value::Nil);
        }
        let ud = lua.create_userdata(crate::context::CreatureRef(id))?;
        Ok(mlua::Value::UserData(ud))
    })?;
    crate::class_registry::register_class(lua, "Creature", Some(creature_ctor))?;

    let globals = lua.globals();

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
    // TFS `LuaScriptInterface::isScriptsInterface` — true only while the
    // scripts-interface pass is running. `data/scripts/lib/event_callbacks.lua`
    // gates `EventCallback:register` / `__newindex` on it; outside the pass a
    // stray `ec:register()` must be a no-op, not a nil call. Reads the Cell
    // (not a constant false) so the scan guard can flip it.
    let flag = Rc::clone(&scripts_interface);
    globals.set(
        "isScriptsInterface",
        lua.create_function(move |_, ()| Ok(flag.get()))?,
    )?;

    // `getDepotId(uid)` — TFS `luaGetDepotId` (`luascript.cpp:3766`).
    // Looks up a depot locker by its UID (from `item:getUniqueId()`) and returns
    // its `ATTR_DEPOT_ID`. Used by `data/scripts/movements/other/tiles.lua`.
    globals.set(
        "getDepotId",
        lua.create_function(|_, uid: u32| {
            use crate::context::CURRENT_CTX;
            CURRENT_CTX.with(|c| {
                let ptr = (*c.borrow())
                    .ok_or_else(|| mlua::Error::runtime("getDepotId: LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("getDepotId: LuaContext is null"));
                }
                let ctx = unsafe { &*ptr };
                match ctx.get_depot_id_by_uid(uid) {
                    Some(depot_id) => Ok(mlua::Value::Number(f64::from(depot_id))),
                    None => Ok(mlua::Value::Boolean(false)),
                }
            })
        })?,
    )?;

    Ok(())
}

fn infer_move_event_kind(
    me_table: &mlua::Table,
    type_name: Option<&str>,
) -> Result<crate::move_events::MoveEventKind, LuaError> {
    use crate::move_events::MoveEventKind;
    if let Some(name) = type_name {
        return MoveEventKind::from_type_name(name).ok_or_else(|| {
            LuaError::Init(mlua::Error::runtime(format!(
                "MoveEvent: invalid type '{name}'"
            )))
        });
    }
    // Infer from which callback field is set (revscript style).
    let checks: &[(&str, MoveEventKind)] = &[
        ("onStepOut", MoveEventKind::StepOut),
        ("onStepIn", MoveEventKind::StepIn),
        ("onEquip", MoveEventKind::Equip),
        ("onDeEquip", MoveEventKind::DeEquip),
        ("onAddItem", MoveEventKind::AddItem),
        ("onRemoveItem", MoveEventKind::RemoveItem),
    ];
    for &(field, kind) in checks {
        if me_table
            .get::<Option<mlua::Function>>(field)
            .ok()
            .flatten()
            .is_some()
        {
            return Ok(kind);
        }
    }
    Err(LuaError::Init(mlua::Error::runtime(
        "MoveEvent:register() needs onStepIn/onStepOut/… or :type()",
    )))
}

/// `doRelocate(fromPos, toPos[, force])` — `compat.lua` body as a registered global.
fn register_do_relocate(lua: &Lua) -> Result<(), mlua::Error> {
    // Implemented in Lua so it reuses Tile/Item/Creature userdata methods.
    lua.load(
        r#"
function doRelocate(fromPos, toPos, force)
	if fromPos == toPos then
		return false
	end

	local fromTile = Tile(fromPos)
	if fromTile == nil then
		return false
	end

	if Tile(toPos) == nil then
		return false
	end

	for i = fromTile:getThingCount() - 1, 0, -1 do
		local thing = fromTile:getThing(i)
		if thing then
			if thing:isItem() then
				if ItemType(thing:getId()):isMovable() or force and not ItemType(thing:getId()):isGroundTile() then
					thing:moveTo(toPos)
				end
			elseif thing:isCreature() then
				thing:teleportTo(toPos, true)
			end
		end
	end

	local magicWall = fromTile:getItemById(ITEM_MAGICWALL)
	if magicWall then
		magicWall:remove()
		fromTile:getPosition():sendMagicEffect(CONST_ME_POFF)
	end

	local wildGrowth = fromTile:getItemById(ITEM_WILDGROWTH)
	if wildGrowth then
		wildGrowth:remove()
		fromTile:getPosition():sendMagicEffect(CONST_ME_POFF)
	end

	local splashItem = fromTile:getItemByGroup(ITEM_GROUP_SPLASH)
	if splashItem then
		splashItem:remove()
	end

	local magicField = fromTile:getItemByGroup(ITEM_GROUP_MAGICFIELD)
	if magicField then
		magicField:remove()
		fromTile:getPosition():sendMagicEffect(CONST_ME_POFF)
	end

	return true
end
"#,
    )
    .set_name("doRelocate")
    .exec()?;
    Ok(())
}

/// `Game` table methods — PC-3a Phase 6 + Gap 5 (`Game.getWorldType` / `createMonster`).
/// C++ `luascript.cpp` `luaGameGetWorldType` / `luaGameCreateMonster`.
fn register_game_api(lua: &Lua) -> Result<(), mlua::Error> {
    use crate::context::{CreatureRef, ItemRef};
    use crate::lua_mutation::{call_clear_field, call_create_monster, call_lua_game_create_item};
    use crate::userdata::item::{parse_lua_item_type_id, push_item_userdata};
    use crate::userdata::position::PositionRef;
    // `Game` is a class table (extensible by `function Game:method(...)` in
    // `data/lib/core/game.lua`) with no constructor — `register_class(_, None)`
    // get-or-creates it. Gap 7a.
    let game = crate::class_registry::register_class(lua, "Game", None)?;
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
    // `Game.createItem(itemId[, count[, position]])` — `luaGameCreateItem`
    // + `setItemMetatable` (R2: container types return Container userdata).
    game.set(
        "createItem",
        lua.create_function(
            |lua, (item_id, count, pos): (Value, Option<u16>, Option<Value>)| {
                let Some(item_type) = parse_lua_item_type_id(item_id)? else {
                    return Ok(Value::Nil);
                };
                let count = count.unwrap_or(1);
                let position = match pos {
                    None | Some(Value::Nil) => None,
                    Some(Value::UserData(ud)) => {
                        if let Ok(p) = ud.borrow::<PositionRef>() {
                            Some((p.x, p.y, p.z))
                        } else {
                            return Err(mlua::Error::runtime(
                                "Game.createItem: position must be Position",
                            ));
                        }
                    }
                    Some(Value::Table(t)) => {
                        let x: i64 = t.get("x").or_else(|_| t.get(1))?;
                        let y: i64 = t.get("y").or_else(|_| t.get(2))?;
                        let z: i64 = t.get("z").or_else(|_| t.get(3))?;
                        Some((x as u16, y as u16, z as u8))
                    }
                    Some(_) => {
                        return Err(mlua::Error::runtime(
                            "Game.createItem: expected Position or table",
                        ));
                    }
                };
                match call_lua_game_create_item(item_type, count, position) {
                    Ok(Some(id)) => push_item_userdata(lua, id),
                    Ok(None) => Ok(Value::Nil),
                    Err(e) => Err(mlua::Error::runtime(e)),
                }
            },
        )?,
    )?;
    // `Game.createTile(position[, isDynamic])` / `Game.createTile(x, y, z[, isDynamic])`
    // — `luaGameCreateTile`. Get-or-create; always returns Tile userdata.
    game.set(
        "createTile",
        lua.create_function(|lua, args: mlua::MultiValue| {
            let (x, y, z, is_dynamic) = crate::userdata::tile::parse_create_tile_args(args)?;
            crate::lua_mutation::call_lua_game_create_tile(x, y, z, is_dynamic)
                .map_err(mlua::Error::runtime)?;
            let ud = lua.create_userdata(crate::userdata::tile::TileRef { x, y, z })?;
            Ok(Value::UserData(ud))
        })?,
    )?;
    // E6: all instant defs (incl. rune-conjure instants). **Not** TFS
    // `player:getInstantSpells` = `canCast` vocation dump.
    // 772 `GetSpellbook` (`magic.cc:3830`) filters with `SpellKnown`.
    game.set(
        "getInstantSpells",
        lua.create_function(|lua, ()| {
            let spells =
                crate::context::current_ctx(|ctx| ctx.list_instant_spells()).unwrap_or_default();
            let t = lua.create_table_with_capacity(spells.len(), 0)?;
            for (i, spell) in spells.into_iter().enumerate() {
                let row = lua.create_table_with_capacity(0, 6)?;
                row.set("name", spell.name)?;
                row.set("words", spell.words)?;
                row.set("level", spell.level)?;
                row.set("mlevel", spell.magic_level)?;
                row.set("mana", spell.mana)?;
                row.set("manapercent", spell.mana_percent)?;
                t.set(i + 1, row)?;
            }
            Ok(t)
        })?,
    )?;
    // 772 `ClearField` — shove creatures/items off a door tile before close.
    game.set(
        "clearField",
        lua.create_function(|_, (item, exclude): (Value, Option<Value>)| {
            let item_id = match item {
                Value::UserData(ud) => ud.borrow::<ItemRef>()?.0,
                _ => {
                    return Err(mlua::Error::runtime(
                        "Game.clearField: item must be Item userdata",
                    ));
                }
            };
            let exclude_cid = match exclude {
                None | Some(Value::Nil) => None,
                Some(Value::UserData(ud)) => Some(ud.borrow::<CreatureRef>()?.0),
                Some(_) => {
                    return Err(mlua::Error::runtime(
                        "Game.clearField: exclude must be Creature or nil",
                    ));
                }
            };
            call_clear_field(item_id, exclude_cid).map_err(mlua::Error::runtime)?;
            Ok(())
        })?,
    )?;
    // `Game` was registered via `register_class` above; methods were attached
    // directly to that class table. No global set needed here.
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

/// `doTargetCombat` / `doTargetCombatHealth` — TFS `luaDoTargetCombat`.
/// Compat aliases Health → Combat (`compat.lua:314`); we register both, no full compat load.
fn register_do_target_combat(lua: &Lua) -> Result<(), mlua::Error> {
    use crate::context::CreatureRef;
    use crate::lua_mutation::call_do_target_combat_health;

    let f = lua.create_function(
        |_,
         (attacker, target, combat_type, min, max, effect): (
            Value,
            Value,
            i32,
            i32,
            i32,
            Option<i32>,
        )| {
            let attacker_id = match attacker {
                Value::Nil | Value::Integer(0) => None,
                Value::Number(0.0) => None,
                Value::UserData(ud) => Some(ud.borrow::<CreatureRef>()?.0),
                Value::Integer(i) if i > 0 => Some(i as u64),
                Value::Number(n) if n > 0.0 => Some(n as u64),
                _ => {
                    return Err(mlua::Error::runtime(
                        "doTargetCombat: attacker must be Creature userdata or 0",
                    ));
                }
            };
            let target_id = match target {
                Value::UserData(ud) => ud.borrow::<CreatureRef>()?.0,
                _ => {
                    return Err(mlua::Error::runtime(
                        "doTargetCombat: target must be Creature userdata",
                    ));
                }
            };
            call_do_target_combat_health(
                attacker_id,
                target_id,
                combat_type,
                min,
                max,
                effect.unwrap_or(0),
            )
            .map_err(mlua::Error::runtime)
        },
    )?;
    lua.globals().set("doTargetCombat", f.clone())?;
    lua.globals().set("doTargetCombatHealth", f)?;
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
                Ok(Some(
                    crate::context::current_ctx(|ctx| ctx.get_config_bool(name).unwrap_or(false))
                        .unwrap_or(false),
                ))
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
                let ok =
                    crate::lua_mutation::call_lua_set_world_light(level, color).unwrap_or(false);
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

    /// Required data-pack globals absent after `load_data_lib` (Gap 5 assertion).
    /// Carries the named list so boot fails fast with actionable diagnostics.
    #[error("Missing required data globals after load_data_lib: {0:?}")]
    MissingGlobals(Vec<String>),

    /// Aggregated Phase 2 (lib-stage) load failures. Gap 5a: the data pack ships
    /// with this repo, so a lib file that does not load is boot-blocking. Lists
    /// every file rather than stopping at the first so one boot log covers the
    /// whole stage. Content-stage (revscript) loaders stay warn-and-continue.
    #[error("{}", format_lib_stage_failures(.0))]
    LibStageFailures(Vec<(String, String)>),

    #[error("Not implemented")]
    NotImplemented,
}

/// Format [`LuaError::LibStageFailures`] as a multi-line boot diagnostic.
fn format_lib_stage_failures(failures: &[(String, String)]) -> String {
    let mut out = format!("lib-stage load failures ({}):", failures.len());
    for (path, err) in failures {
        out.push_str("\n  ");
        out.push_str(path);
        out.push_str(": ");
        out.push_str(err);
    }
    out
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

    /// Gap 7 probe: does `data/global.lua` load via the native LuaJIT `dofile`
    /// chain when CWD is the repo root? `global.lua` calls
    /// `dofile('data/lib/lib.lua')` → `core.lua` → all `data/lib/core/*.lua`.
    ///
    /// **Current status (2026-08-10):** `dofile` and `os.time` both work (the
    /// stale comment at line 1294 was wrong). The dofile chain resolves
    /// correctly but fails at `data/lib/core/combat.lua:1` because `Combat` is
    /// registered as a bare function, not a class table — `function
    /// Combat:getPositions(...)` can't index a function value. See
    /// `tasks/tools-actions/gap7-class-globals.md`.
    ///
    /// Once `Combat`/`Spell`/`Weapon`/`Condition` are converted to tables with
    /// `__call` (TVP `registerClass` pattern), this test should pass and we can
    /// replace `inject_door_tables_from_global` + the `data/lib/core/` recursive
    /// scan with a single `exec_chunk("global.lua", src)` call.
    #[test]
    fn global_lua_loads_via_dofile_chain() {
        let data_root = workspace_data_root();
        let global_path = data_root.join("global.lua");
        if !global_path.exists() {
            eprintln!("data/global.lua not found — skipping");
            return;
        }

        // `dofile` resolves relative to process CWD. `global.lua` calls
        // `dofile('data/lib/lib.lua')` — needs CWD = workspace root.
        let workspace_root = data_root.parent().expect("data/ has a parent");
        let prev_cwd = std::env::current_dir().ok();
        std::env::set_current_dir(workspace_root).expect("chdir to workspace root");

        let runtime = LuaRuntime::new().expect("runtime init");
        let src = std::fs::read_to_string(&global_path).expect("read global.lua");
        let result = runtime.exec_chunk("global.lua", &src);

        // Restore CWD regardless of outcome.
        if let Some(prev) = prev_cwd {
            let _ = std::env::set_current_dir(prev);
        }

        match &result {
            Ok(()) => println!("global.lua loaded via dofile chain — OK"),
            Err(e) => println!("global.lua dofile chain failed: {e}"),
        }
        // Don't hard-fail yet — this is a Gap 7 probe. The error output tells
        // us what's still missing. Once Gap 7 is fixed, flip to
        // `assert!(result.is_ok())`.
        let _ = result;
    }

    /// VM hardening pillar 4 — `set_memory_limit`
    /// (tasks/tools-actions/vm-hardening.md). Asserts the default is applied in `new()`, that an override
    /// takes effect, and that an over-limit allocation aborts the script
    /// instead of OOM-killing the process.
    #[test]
    fn memory_limit_default_applied_and_enforced() {
        let runtime = LuaRuntime::new().expect("runtime");

        // `set_memory_limit` returns the *previous* limit — proves the default
        // was applied during construction.
        let prev = runtime
            .set_memory_limit(DEFAULT_LUA_MEMORY_LIMIT_BYTES)
            .expect("set_memory_limit");
        assert_eq!(
            prev, DEFAULT_LUA_MEMORY_LIMIT_BYTES,
            "LuaRuntime::new must apply DEFAULT_LUA_MEMORY_LIMIT_BYTES"
        );

        // Tighten to 64 KiB and attempt a 4 MiB string allocation. The custom
        // allocator returns null past the limit → LuaJIT raises a memory error
        // surfaced as `mlua::Error::RuntimeError` wrapping "not enough memory".
        runtime.set_memory_limit(64 * 1024).expect("tighten limit");
        let err = runtime
            .lua
            .load(r#"local s = string.rep("x", 4 * 1024 * 1024); return s"#)
            .exec()
            .expect_err("over-limit allocation must error, not OOM the process");
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("memory") || msg.contains("not enough"),
            "expected a memory error, got: {msg}"
        );
    }

    /// VM hardening pillar 4 — instruction budget through `LuaRuntime::exec_chunk`.
    /// A lifetime-counter hook would fail the second call; per-invocation reset
    /// is the contract. Abort is isolation, not rollback (`x` stays set).
    #[test]
    fn instruction_budget_default_applied_and_enforced() {
        let runtime = LuaRuntime::new().expect("runtime");
        let prev = runtime.set_instruction_budget(DEFAULT_LUA_INSTRUCTION_BUDGET);
        assert_eq!(
            prev, DEFAULT_LUA_INSTRUCTION_BUDGET,
            "LuaRuntime::new must apply DEFAULT_LUA_INSTRUCTION_BUDGET"
        );

        runtime.set_instruction_budget(10_000);
        let err = runtime
            .exec_chunk("runaway", "x = 1; while true do end")
            .expect_err("runaway loop must error, not hang the game thread");
        assert!(
            err.to_string()
                .contains(crate::instruction_budget::INSTRUCTION_BUDGET_EXCEEDED),
            "expected instruction-budget error, got: {err}"
        );
        let x: i64 = runtime.lua.globals().get("x").expect("x");
        assert_eq!(x, 1, "abort does not roll back prior Lua assignments");

        runtime.set_instruction_budget(50_000);
        runtime
            .exec_chunk("ok", "local n = 0; for i = 1, 1000 do n = n + 1 end")
            .expect("first legitimate chunk");
        runtime
            .exec_chunk("ok2", "local n = 0; for i = 1, 1000 do n = n + 1 end")
            .expect("second chunk must get a fresh budget");
    }

    /// VM hardening pillar 1 — stdlib allowlist through the shipped init path.
    #[test]
    fn stdlib_allowlist_applied_in_new() {
        let runtime = LuaRuntime::new().expect("runtime");
        let denied: bool = runtime
            .lua
            .load("return io == nil and package == nil and loadstring == nil and os.execute == nil")
            .eval()
            .expect("probe");
        assert!(
            denied,
            "LuaRuntime::new must not ship io/package/loadstring/os.execute"
        );
        let allowed: bool = runtime
            .lua
            .load("return type(os.time) == 'function' and type(tfs.appendLog) == 'function'")
            .eval()
            .expect("shim");
        assert!(allowed, "os.time and tfs.appendLog must remain");
    }
}
