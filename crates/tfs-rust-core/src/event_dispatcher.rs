//! Lua / script event surface injected into `GameWorld` (avoids `tfs-rust-core` → `tfs-rust-lua`).
// C++ reference: `CreatureEvent::dispatch`, `LuaScriptInterface` hooks.

use crate::ids::CreatureId;

/// Script and engine events. Default bodies are no-ops until `tfs-rust-lua` implements dispatch.
pub trait EventDispatcher: Send + Sync + 'static {
    fn on_login(&self, _creature: CreatureId) {}
    fn on_logout(&self, _creature: CreatureId) {}
    fn on_think(&self, _creature: CreatureId, _interval_ms: u32) {}
    fn on_prepare_death(&self, _creature: CreatureId) {}
    fn on_death(&self, _creature: CreatureId) {}
    fn on_kill(&self, _killer: CreatureId, _target: CreatureId) {}
    /// TFS `Creature::onWalkComplete` — walk queue empty after `getNextStep` false (`src/creature.cpp` ~215–219).
    fn on_walk_complete(&self, _creature: CreatureId) {}
    fn on_advance(&self, _creature: CreatureId, _skill: u8, _old_level: u32, _new_level: u32) {}
    fn on_startup(&self) {}
    fn on_shutdown(&self) {}
    /// Spread LuaJIT GC across ticks (Phase 4 game loop). No-op without Lua.
    fn lua_gc_step(&self) {}
}

/// Default no-op dispatcher for tests and early wiring.
#[derive(Debug, Default)]
pub struct NullEventDispatcher;

impl EventDispatcher for NullEventDispatcher {}
