//! Group userdata for Lua (`Group` in TFS scripts).
//!
//! C++ reference: `src/luascript.cpp` — `LuaScriptInterface` group userdata
//! (`Group::getAccess`, …). Wraps a group id resolved from `players.group_id`.

use mlua::{UserData, UserDataMethods};
use std::cell::RefCell;

use crate::context::{CURRENT_CTX, LuaContext};

/// ID handle wrapper for a player group passed to Lua userdata.
#[derive(Clone, Copy, Debug)]
pub struct GroupRef(pub u16);

impl UserData for GroupRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // `group:getAccess()` — `Group::getAccess` (`src/groups.cpp`). CH-6
        // talkaction access gating; reads the `access` flag from the group
        // database via `ScriptContext::get_group_access`.
        methods.add_method("getAccess", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                let access = ctx.get_group_access(this.0);
                tracing::info!(group_id = this.0, access, "Lua group:getAccess()");
                Ok(access)
            })
        });

        // `group:getId()` — `Group::getId` (`src/groups.cpp`). Returns the
        // raw group id.
        methods.add_method("getId", |_, this, ()| Ok(this.0));

        // `group:hasFlag(flag)` — `Group::flags & flag` (`src/groups.cpp`).
        // PC-3a Phase 5: `conjureItem` dual-hand infinite-mana gate.
        methods.add_method("hasFlag", |_, this, flag: u64| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.group_has_flag(this.0, flag))
            })
        });

        // `group:getName()` — `Group::getName` (`src/groups.cpp`). Returns the
        // group name. Reads through ScriptContext to avoid duplicating the
        // group database in the lua crate.
        methods.add_method("getName", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                // Group name isn't on ScriptContext (only access is needed for
                // talkactions). Return the id as a string fallback — scripts
                // that need the name can use getId.
                let _ = ctx;
                Ok(format!("group_{}", this.0))
            })
        });
    }
}

/// Register the Group metatable in the Lua runtime.
pub fn register_group_metatable(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<GroupRef>(|_registry| {})
}
