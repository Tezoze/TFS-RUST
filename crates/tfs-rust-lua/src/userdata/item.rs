//! Item userdata for Lua (`Item` in TFS scripts).
//!
//! C++ reference: `src/luascript.cpp` — `LuaScriptInterface` item userdata (`Item::getID`, `getName`, …).

use mlua::UserData;
use mlua::UserDataMethods;
use std::cell::RefCell;

use crate::context::{CURRENT_CTX, ItemData, ItemRef, LuaContext};

/// Register the Item metatable in the Lua runtime.
pub fn register_item_metatable(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<ItemRef>(|_registry| {})
}

impl UserData for ItemRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getId", |_, this, ()| Ok(this.0));

        methods.add_method("getType", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx
                    .get_item_data(this.0)
                    .map(|d: ItemData| d.item_type)
                    .unwrap_or(0))
            })
        });

        methods.add_method("getCount", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx
                    .get_item_data(this.0)
                    .map(|d: ItemData| d.count)
                    .unwrap_or(0))
            })
        });

        methods.add_method("getWeight", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx
                    .get_item_data(this.0)
                    .map(|d: ItemData| d.weight)
                    .unwrap_or(0))
            })
        });

        methods.add_method("getName", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                ctx.get_item_data(this.0)
                    .map(|d: ItemData| d.name)
                    .ok_or_else(|| mlua::Error::runtime("item not found"))
            })
        });
    }
}
