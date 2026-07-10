//! Position userdata for Lua (`Position` in TFS scripts).
//!
//! C++ reference: `src/luascript.cpp` — `LuaScriptInterface` position userdata
//! (`Position::sendMagicEffect`, …). Wraps `(x, y, z)` coordinates and provides
//! methods that mutate game state via the mutation scope.

use mlua::{UserData, UserDataMethods};

use crate::lua_mutation::call_lua_send_magic_effect;

/// Position handle wrapping `(x, y, z)` coordinates for Lua.
#[derive(Clone, Copy, Debug)]
pub struct PositionRef {
    pub x: u16,
    pub y: u16,
    pub z: u8,
}

impl UserData for PositionRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // `Position:sendMagicEffect(effect)` — `game.cpp:4816` `addMagicEffect`.
        // CH-6 talkaction `/i` green-sparkle at player position. Broadcasts the
        // effect to all spectators at this position via the mutation scope.
        methods.add_method("sendMagicEffect", |_, this, effect: u8| {
            tracing::info!(
                x = this.x,
                y = this.y,
                z = this.z,
                effect,
                "Lua Position:sendMagicEffect()"
            );
            call_lua_send_magic_effect(this.x, this.y, this.z, effect).map_err(mlua::Error::runtime)
        });

        // Accessor fields — some scripts read `pos.x` / `pos.y` / `pos.z`.
        methods.add_method("getX", |_, this, ()| Ok(this.x));
        methods.add_method("getY", |_, this, ()| Ok(this.y));
        methods.add_method("getZ", |_, this, ()| Ok(this.z));
    }
}

/// Register the Position metatable in the Lua runtime.
pub fn register_position_metatable(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<PositionRef>(|_registry| {})
}
