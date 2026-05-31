//! Container userdata for Lua (`Container` extends `Item`).
//!
//! C++ reference: `src/luascript.cpp` — `Container` class registration ~2343–2359.

use mlua::{Lua, UserData, UserDataMethods, Value};
use std::cell::RefCell;

use crate::context::{CURRENT_CTX, ItemRef, LuaContext};
use crate::lua_mutation::call_lua_container_add_item;

/// Container handle — same underlying item id as [`ItemRef`].
#[derive(Clone, Copy, Debug)]
pub struct ContainerRef(pub u64);

pub fn register_container_metatable(lua: &Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<ContainerRef>(|_registry| {})
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

impl UserData for ContainerRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getSize", |_, this, ()| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_container_data(this.0)
                    .map(|d| d.size)
                    .unwrap_or(0))
            })
        });

        methods.add_method("getCapacity", |_, this, ()| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_container_data(this.0)
                    .map(|d| d.capacity)
                    .unwrap_or(0))
            })
        });

        methods.add_method("getEmptySlots", |_, this, ()| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_container_data(this.0)
                    .map(|d| d.empty_slots)
                    .unwrap_or(0))
            })
        });

        methods.add_method("getItemHoldingCount", |_, this, ()| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_container_data(this.0)
                    .map(|d| d.item_holding_count)
                    .unwrap_or(0))
            })
        });

        methods.add_method("getCorpseOwner", |_, this, ()| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_container_data(this.0)
                    .map(|d| d.corpse_owner)
                    .unwrap_or(0))
            })
        });

        methods.add_method("getItem", |lua, this, index: u32| {
            let id_opt = with_ctx(|ctx| Ok(ctx.get_container_item_at(this.0, index)))?;
            match id_opt {
                Some(iid) => {
                    let ud = lua.create_userdata(ItemRef(iid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("getItems", |lua, this, ()| {
            let ids = with_ctx(|ctx| Ok(ctx.get_container_items(this.0)))?;
            let table = lua.create_table()?;
            for (i, id) in ids.into_iter().enumerate() {
                let ud = lua.create_userdata(ItemRef(id))?;
                table.set(i + 1, ud)?;
            }
            Ok(table)
        });

        methods.add_method(
            "getItemCountById",
            |_, this, (item_type, sub_type): (u16, Option<i32>)| {
                let sub_type = sub_type.unwrap_or(-1);
                with_ctx(|ctx| Ok(ctx.get_container_item_count_by_id(this.0, item_type, sub_type)))
            },
        );

        methods.add_method("hasItem", |_, this, item: mlua::AnyUserData| {
            let item_id = if let Ok(r) = item.borrow::<ItemRef>() {
                r.0
            } else if let Ok(r) = item.borrow::<ContainerRef>() {
                r.0
            } else {
                return Ok(false);
            };
            with_ctx(|ctx| Ok(ctx.container_has_item(this.0, item_id)))
        });

        methods.add_method(
            "addItem",
            |lua,
             this,
             (item_type, count, index, flags): (u16, Option<u32>, Option<i32>, Option<u32>)| {
                let count = count.unwrap_or(1);
                let index = index.unwrap_or(-1);
                let flags = flags.unwrap_or(0);
                let id_opt = call_lua_container_add_item(this.0, item_type, count, index, flags)
                    .map_err(mlua::Error::runtime)?;
                match id_opt {
                    Some(iid) => {
                        let ud = lua.create_userdata(ItemRef(iid))?;
                        Ok(Value::UserData(ud))
                    }
                    None => Ok(Value::Nil),
                }
            },
        );

        // Item base methods — C++ `Container` extends `Item`.
        methods.add_method("getId", |_, this, ()| Ok(this.0));
        methods.add_method("getType", |_, this, ()| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_item_data(this.0)
                    .map(|d| d.item_type)
                    .unwrap_or(0))
            })
        });
        methods.add_method("getCount", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.get_item_data(this.0).map(|d| d.count).unwrap_or(0)))
        });
        methods.add_method("getWeight", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.get_item_data(this.0).map(|d| d.weight).unwrap_or(0)))
        });
        methods.add_method("getName", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_item_data(this.0)
                    .map(|d| d.name)
                    .ok_or_else(|| mlua::Error::runtime("item not found"))
            })
        });
    }
}
