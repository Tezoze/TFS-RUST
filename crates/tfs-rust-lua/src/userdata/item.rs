//! Item userdata for Lua (`Item` in TFS scripts).
//!
//! C++ reference: `src/luascript.cpp` — `LuaScriptInterface` item userdata (`Item::getID`, `getName`, …).

use mlua::{Lua, UserData, UserDataMethods, Value};
use std::cell::RefCell;

use crate::context::{CreatureRef, CURRENT_CTX, ItemData, ItemRef, LuaContext};
use crate::lua_mutation::{
    call_lua_item_move_to, call_lua_item_remove, call_lua_set_action_id, call_lua_set_store_item,
    call_lua_set_unique_id, LuaMoveDestination,
};
use crate::userdata::container::ContainerRef;

/// Register the Item metatable in the Lua runtime.
pub fn register_item_metatable(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<ItemRef>(|_registry| {})
}

fn with_ctx<F, R>(f: F) -> Result<R, mlua::Error>
where
    F: FnOnce(&dyn LuaContext) -> Result<R, mlua::Error>,
{
    CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
        let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
        if ptr.is_null() {
            return Err(mlua::Error::runtime("LuaContext not set"));
        }
        let ctx = unsafe { &*ptr };
        f(ctx)
    })
}

fn push_cylinder(lua: &Lua, cyl: tfs_rust_common::ScriptCylinder) -> Result<Value, mlua::Error> {
    match cyl {
        tfs_rust_common::ScriptCylinder::Player(id) => {
            let ud = lua.create_userdata(CreatureRef(id))?;
            Ok(Value::UserData(ud))
        }
        tfs_rust_common::ScriptCylinder::Container(id) => {
            let ud = lua.create_userdata(ContainerRef(id))?;
            Ok(Value::UserData(ud))
        }
        tfs_rust_common::ScriptCylinder::Tile(pos) => {
            let table = lua.create_table()?;
            table.set("x", pos.x)?;
            table.set("y", pos.y)?;
            table.set("z", pos.z)?;
            Ok(Value::Table(table))
        }
    }
}

fn parse_move_destination(_lua: &Lua, value: Value) -> Result<LuaMoveDestination, mlua::Error> {
    match value {
        Value::UserData(ud) => {
            if let Ok(cref) = ud.borrow::<CreatureRef>() {
                return Ok(LuaMoveDestination::Player {
                    creature_id: cref.0,
                });
            }
            if let Ok(cont) = ud.borrow::<ContainerRef>() {
                return Ok(LuaMoveDestination::Container {
                    item_id: cont.0,
                });
            }
            if let Ok(item) = ud.borrow::<ItemRef>() {
                return Ok(LuaMoveDestination::Container {
                    item_id: item.0,
                });
            }
            Err(mlua::Error::runtime("invalid moveTo destination"))
        }
        Value::Table(t) => {
            let x: u16 = t.get("x")?;
            let y: u16 = t.get("y")?;
            let z: u8 = t.get("z")?;
            Ok(LuaMoveDestination::Tile { x, y, z })
        }
        _ => Err(mlua::Error::runtime("invalid moveTo destination")),
    }
}

impl UserData for ItemRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getId", |_, this, ()| Ok(this.0));

        methods.add_method("getType", |_, this, ()| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_item_data(this.0)
                    .map(|d: ItemData| d.item_type)
                    .unwrap_or(0))
            })
        });

        methods.add_method("getCount", |_, this, ()| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_item_data(this.0)
                    .map(|d: ItemData| d.count)
                    .unwrap_or(0))
            })
        });

        methods.add_method("getWeight", |_, this, ()| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_item_data(this.0)
                    .map(|d: ItemData| d.weight)
                    .unwrap_or(0))
            })
        });

        methods.add_method("getName", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_item_data(this.0)
                    .map(|d: ItemData| d.name)
                    .ok_or_else(|| mlua::Error::runtime("item not found"))
            })
        });

        methods.add_method("getActionId", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.get_item_data(this.0).map(|d| d.action_id).unwrap_or(0)))
        });

        methods.add_method("setActionId", |_, this, action_id: u16| {
            call_lua_set_action_id(this.0, action_id).map_err(|e| mlua::Error::runtime(e))
        });

        methods.add_method("getUniqueId", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.get_item_data(this.0).map(|d| d.unique_id).unwrap_or(0)))
        });

        methods.add_method("setUniqueId", |_, this, unique_id: u16| {
            call_lua_set_unique_id(this.0, unique_id).map_err(|e| mlua::Error::runtime(e))
        });

        methods.add_method("isStoreItem", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.get_item_data(this.0).map(|d| d.is_store_item).unwrap_or(false)))
        });

        methods.add_method("setStoreItem", |_, this, store: bool| {
            call_lua_set_store_item(this.0, store).map_err(|e| mlua::Error::runtime(e))
        });

        methods.add_method("isContainer", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.is_registered_container(this.0)))
        });

        methods.add_method("getContainer", |lua, this, ()| {
            let is_cont = with_ctx(|ctx| Ok(ctx.is_registered_container(this.0)))?;
            if is_cont {
                let ud = lua.create_userdata(ContainerRef(this.0))?;
                Ok(Value::UserData(ud))
            } else {
                Ok(Value::Nil)
            }
        });

        methods.add_method("getParent", |lua, this, ()| {
            let parent = with_ctx(|ctx| Ok(ctx.get_item_parent(this.0)))?;
            match parent {
                Some(cyl) => push_cylinder(lua, cyl),
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("getTopParent", |lua, this, ()| {
            let parent = with_ctx(|ctx| Ok(ctx.get_item_top_parent(this.0)))?;
            match parent {
                Some(cyl) => push_cylinder(lua, cyl),
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("getPosition", |_, this, ()| {
            with_ctx(|ctx| {
                let pos = ctx
                    .get_item_position(this.0)
                    .ok_or_else(|| mlua::Error::runtime("item not found"))?;
                Ok((pos.x, pos.y, pos.z))
            })
        });

        methods.add_method("moveTo", |lua, this, (dest, flags): (Value, Option<u32>)| {
            let dest = parse_move_destination(lua, dest)?;
            let flags = flags.unwrap_or(0);
            call_lua_item_move_to(this.0, dest, flags).map_err(|e| mlua::Error::runtime(e))
        });

        methods.add_method("remove", |_, this, count: Option<i32>| {
            let count = count.unwrap_or(-1);
            call_lua_item_remove(this.0, count).map_err(|e| mlua::Error::runtime(e))
        });
    }
}
