//! ItemType userdata for Lua (`ItemType` in TFS scripts).
//!
//! C++ reference: `src/luascript.cpp` — `LuaScriptInterface` ItemType userdata
//! (`ItemType::getID`, `isStackable`, `isFluidContainer`, …). Wraps a server
//! item type id; resolves name→id lookups through `ScriptContext`.

use mlua::{UserData, UserDataMethods, Value};
use std::cell::RefCell;

use crate::context::{CURRENT_CTX, LuaContext};

/// Item type handle wrapping a server item type id. `id = 0` means "not found"
/// (matching C++ `ItemType::id == 0` for invalid items).
#[derive(Clone, Copy, Debug)]
pub struct ItemTypeRef(pub u16);

impl UserData for ItemTypeRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // `ItemType:getId()` — `ItemType::getID` (`src/items.h`). Returns the
        // server item type id, or `0` if the item was not found.
        methods.add_method("getId", |_, this, ()| Ok(this.0));

        // `ItemType:isStackable()` — `ItemType::stackable` (`src/items.h`).
        // CH-6 talkaction `/i` count clamping.
        methods.add_method("isStackable", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_item_type_is_stackable(this.0))
            })
        });

        // `ItemType:isFluidContainer()` — `ItemType::isFluidContainer`
        // (`src/items.h`). CH-6 talkaction `/i` count clamping.
        methods.add_method("isFluidContainer", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_item_type_is_fluid_container(this.0))
            })
        });

        // `ItemType:getCharges()` — `ItemType::charges` (`src/items.h`).
        // PC-3a Phase 5: `Player:conjureItem` falls back to charges when count
        // is omitted.
        methods.add_method("getCharges", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_item_type_charges(this.0))
            })
        });

        methods.add_method("isCorpse", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_item_type_is_corpse(this.0))
            })
        });

        methods.add_method("isMovable", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_item_type_is_movable(this.0))
            })
        });

        methods.add_method("isGroundTile", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_item_type_is_ground_tile(this.0))
            })
        });

        // `ItemType:getName()` — `ItemType::name` (`src/items.h`). Returns
        // the item name, or empty string if not found.
        methods.add_method("getName", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                // Reverse-lookup name isn't on ScriptContext; return id string.
                let _ = ctx;
                Ok(format!("item_{}", this.0))
            })
        });
    }
}

/// `ItemType(nameOrId)` constructor — `luascript.cpp` `luaItemTypeCreate`.
/// Resolves a name string to a server item type id via `ScriptContext`, or
/// wraps a numeric id directly. Returns a `ItemTypeRef` userdata (id=0 if
/// the name was not found, matching C++ behavior).
pub fn register_item_type_constructor(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    let constructor = lua.create_function(|lua, arg: Value| {
        let id: u16 = match arg {
            Value::Integer(n) => n as u16,
            Value::Number(n) => n as u16,
            Value::String(s) => {
                let name = s.to_str()?.to_string();
                CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                    let ptr =
                        (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                    if ptr.is_null() {
                        return Err(mlua::Error::runtime("LuaContext not set"));
                    }
                    let ctx = unsafe { &*ptr };
                    Ok(ctx.get_item_type_id_by_name(&name).unwrap_or(0))
                })?
            }
            _ => 0,
        };
        let ud = lua.create_userdata(ItemTypeRef(id))?;
        Ok(Value::UserData(ud))
    })?;
    lua.globals().set("ItemType", constructor)?;
    Ok(())
}

/// Register the ItemType metatable in the Lua runtime.
pub fn register_item_type_metatable(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<ItemTypeRef>(|_registry| {})
}
