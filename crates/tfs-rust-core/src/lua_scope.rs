//! Scoped Lua script execution with read context + mutation applier.
//!
//! C++ reference: `LuaScriptInterface::executeTimer` / creature event dispatch — single game thread.

use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use tfs_rust_lua::{self, LuaMutation, with_lua_context, with_lua_mutation_scope};

fn apply_lua_mutation(world_ptr: *mut (), mutation: LuaMutation) -> Result<(), String> {
    if world_ptr.is_null() {
        return Err("Lua mutation scope not active".into());
    }
    let world = world_ptr as *mut GameWorld;
    // SAFETY: `world_ptr` is set by `with_lua_mutation_scope` immediately before Lua runs
    // and cleared before returning. Game thread only.
    match mutation {
        LuaMutation::PlayerAddItem {
            creature_id,
            item_type,
            count,
        } => unsafe { &mut *world }.lua_script_add_item(creature_id, item_type, count),
        LuaMutation::PlayerRemoveItem {
            creature_id,
            item_type,
            count,
        } => unsafe { &mut *world }.lua_script_remove_item(creature_id, item_type, count),
    }
}

/// Register the mutation applier once at server startup.
pub fn register_lua_mutation_hooks() {
    tfs_rust_lua::register_lua_mutation_applier(apply_lua_mutation);
}

/// Run a creature login script with read context and mutation scope active.
///
/// Encapsulates the re-entrant `&mut GameWorld` access required while Lua callbacks
/// may mutate inventory (`Player:addItem` / `removeItem`).
pub fn fire_on_login(world: &mut GameWorld, cid: CreatureId) {
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        // SAFETY: `world_ptr` is valid for this scope on the game thread.
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            world.events.on_login(cid, world);
        });
    });
}
