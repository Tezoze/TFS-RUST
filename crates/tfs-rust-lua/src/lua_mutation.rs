//! Lua script mutations queued/applied during script execution.
//!
//! C++ reference: `LuaScriptInterface` methods that mutate game state (`luascript.cpp`).

use std::cell::Cell;
use std::sync::OnceLock;

/// Game-state mutation requested from Lua userdata methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LuaMutation {
    PlayerAddItem {
        creature_id: u64,
        item_type: u16,
        count: u16,
    },
    PlayerRemoveItem {
        creature_id: u64,
        item_type: u16,
        count: u32,
        sub_type: i32,
        ignore_equipped: bool,
    },
}

type LuaMutationApplier = fn(*mut (), LuaMutation) -> Result<(), String>;

static MUTATION_APPLIER: OnceLock<LuaMutationApplier> = OnceLock::new();

thread_local! {
    /// Opaque `&mut GameWorld` as `*mut ()` — set only for the duration of
    /// [`with_lua_mutation_scope`] on the game thread.
    static MUTATION_WORLD: Cell<*mut ()> = Cell::new(std::ptr::null_mut());
}

/// Register the core handler that applies mutations (called once at startup).
pub fn register_lua_mutation_applier(applier: LuaMutationApplier) {
    let _ = MUTATION_APPLIER.set(applier);
}

/// Execute `f` with an active Lua mutation scope bound to `world`.
///
/// `world` is an opaque pointer to `GameWorld` on the game thread; it must not
/// be stored past the returned scope.
pub fn with_lua_mutation_scope<F, R>(world: *mut (), f: F) -> R
where
    F: FnOnce() -> R,
{
    MUTATION_WORLD.set(world);
    let result = f();
    MUTATION_WORLD.set(std::ptr::null_mut());
    result
}

fn apply_mutation(mutation: LuaMutation) -> Result<(), String> {
    let world = MUTATION_WORLD.get();
    if world.is_null() {
        return Err("Lua mutation scope not active".into());
    }
    let applier = MUTATION_APPLIER
        .get()
        .ok_or_else(|| "Lua mutation applier not registered".to_string())?;
    applier(world, mutation)
}

pub fn call_lua_add_item(creature_id: u64, item_type: u16, count: u16) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerAddItem {
        creature_id,
        item_type,
        count,
    })
}

pub fn call_lua_remove_item(
    creature_id: u64,
    item_type: u16,
    count: u32,
    sub_type: i32,
    ignore_equipped: bool,
) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerRemoveItem {
        creature_id,
        item_type,
        count,
        sub_type,
        ignore_equipped,
    })
}
