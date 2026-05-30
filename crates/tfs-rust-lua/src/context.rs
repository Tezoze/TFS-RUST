//! LuaContext trait and thread-local context passing.
//!
//! The read trait lives in `tfs-rust-common::ScriptContext` so `tfs-rust-core` never
//! depends on `tfs-rust-lua` for event dispatch. This module re-exports it and provides
//! the thread-local scoped pointer pattern for Lua userdata methods.

use std::cell::RefCell;

pub use tfs_rust_common::{
    ScriptContainerData, ScriptContext as LuaContext, ScriptCreatureData as CreatureData,
    ScriptCreatureId as CreatureId, ScriptCylinder, ScriptItemData as ItemData,
    ScriptItemId as ItemId,
};

/// ID handle wrapper for creatures passed to Lua userdata (local newtype for mlua `UserData`).
#[derive(Clone, Copy, Debug)]
pub struct CreatureRef(pub CreatureId);

/// ID handle wrapper for items passed to Lua userdata (local newtype for mlua `UserData`).
#[derive(Clone, Copy, Debug)]
pub struct ItemRef(pub ItemId);

// Thread-local scoped pointer for passing context to Lua calls
// SAFETY: Pointer is set immediately before Lua call, cleared immediately after,
// on game thread only, never stored. Valid for duration of the Lua call only.
thread_local! {
    pub static CURRENT_CTX: RefCell<Option<*const (dyn LuaContext + 'static)>> = RefCell::new(None);
}

/// Execute a Lua call with a scoped [`LuaContext`] pointer.
///
/// The context is available to UserData methods via [`CURRENT_CTX`] during the call.
///
/// # Safety
///
/// The pointer is valid for the duration of the closure `f`, set immediately
/// before the call and cleared immediately after. This runs on the game thread
/// only and never stores the pointer.
pub fn with_lua_context<F, R>(ctx: &dyn LuaContext, f: F) -> R
where
    F: FnOnce() -> R,
{
    CURRENT_CTX.with(|slot| {
        let ptr: *const dyn LuaContext = ctx;
        let erased: *const (dyn LuaContext + 'static) = unsafe {
            // SAFETY: The pointer is only stored for the duration of this scoped call,
            // then restored/cleared before returning. It never outlives `ctx`.
            std::mem::transmute(ptr)
        };
        let prev = slot.replace(Some(erased));
        let result = f();
        slot.replace(prev);
        result
    })
}
