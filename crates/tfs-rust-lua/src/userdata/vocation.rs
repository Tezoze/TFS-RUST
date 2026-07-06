//! Vocation userdata for Lua (`player:getVocation()` → `:getId()`).
//!
//! C++ reference: `src/luascript.cpp` `luaPlayerGetVocation` — returns a
//! `Vocation*` userdata whose `:getId()` resolves to `Vocation::id` (the
//! `players.vocation` column). LUA-2 implements the minimal surface the
//! channel scripts touch (`getId`); `getName` / `getPromotion` land later
//! when a second consumer needs them (see `tasks/lua-api-plan.md` §1.4).

use mlua::{Lua, UserData, UserDataMethods};

/// Vocation handle — wraps the raw `players.vocation` id (`enums.h:297`
/// `VOCATION_NONE = 0`). Stored by value inside the userdata; no SlotMap
/// lookup needed (the id is the entire payload).
#[derive(Clone, Copy, Debug)]
pub struct VocationRef(pub i32);

pub fn register_vocation_metatable(lua: &Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<VocationRef>(|_registry| {})
}

impl UserData for VocationRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // `vocation:getId()` — `Vocation::id` (`player.h` / `vocation.h`).
        // Channel scripts compare against `VOCATION_NONE` (0).
        methods.add_method("getId", |_, this, ()| Ok(this.0));
    }
}
