//! Player / creature userdata bindings for Lua (`Player` / `Creature`).
//!
//! C++ reference: `src/luascript.cpp` — `Creature` / `Player` userdata methods.

use mlua::UserData;
use mlua::UserDataMethods;
use std::cell::RefCell;

use crate::context::{CreatureData, CreatureRef, ItemRef, CURRENT_CTX, LuaContext};
use crate::lua_mutation::{call_lua_add_item, call_lua_remove_item};

/// Register the Creature metatable in the Lua runtime.
pub fn register_creature_metatable(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<CreatureRef>(|_registry| {
        // The UserData impl is registered via the impl UserData block below
    })
}

impl UserData for CreatureRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getId", |_, this, ()| {
            Ok(this.0)
        });

        methods.add_method("getName", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                // SAFETY: Pointer is valid for the duration of the Lua call,
                // set by with_lua_context immediately before, cleared immediately after.
                // Game thread only, never stored, never outlives the &dyn LuaContext.
                let ctx = unsafe { &*ptr };
                ctx.get_creature(this.0)
                    .map(|c: CreatureData| c.name)
                    .ok_or_else(|| mlua::Error::runtime("creature not found"))
            })
        });

        methods.add_method("getGuid", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                // SAFETY: Same invariant as getName above.
                let ctx = unsafe { &*ptr };
                ctx.get_creature(this.0).map(|c: CreatureData| c.guid)
                    .ok_or_else(|| mlua::Error::runtime("creature not found"))
            })
        });

        methods.add_method("getSlotItem", |lua, this, slot: u8| {
            let id_opt = CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_player_slot_item_id(this.0, slot))
            })?;
            match id_opt {
                Some(iid) => {
                    let ud = lua.create_userdata(ItemRef(iid))?;
                    Ok(mlua::Value::UserData(ud))
                }
                None => Ok(mlua::Value::Nil),
            }
        });

        methods.add_method("getCapacity", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                ctx.get_player_capacity(this.0)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        methods.add_method("getFreeCapacity", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                ctx.get_player_free_capacity(this.0)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        methods.add_method("addItem", |_, this, (item_type, count): (u16, Option<u16>)| {
            let count = count.unwrap_or(1).max(1);
            call_lua_add_item(this.0, item_type, count)
                .map_err(|e| mlua::Error::runtime(e))
        });

        methods.add_method("getItemCount", |_, this, (item_type, sub_type): (u16, Option<i32>)| {
            let sub_type = sub_type.unwrap_or(-1);
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                ctx.get_player_item_type_count(this.0, item_type, sub_type)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        methods.add_method(
            "removeItem",
            |_,
             this,
             (item_type, count, sub_type, ignore_equipped): (u16, u32, Option<i32>, Option<bool>)| {
                let sub_type = sub_type.unwrap_or(-1);
                let ignore_equipped = ignore_equipped.unwrap_or(false);
                call_lua_remove_item(this.0, item_type, count, sub_type, ignore_equipped)
                    .map_err(|e| mlua::Error::runtime(e))
            },
        );
    }
}
