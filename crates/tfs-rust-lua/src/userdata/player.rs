//! Player / creature userdata bindings for Lua (`Player` / `Creature`).
//!
//! C++ reference: `src/luascript.cpp` — `Creature` / `Player` userdata methods.

use mlua::{UserData, UserDataMethods, Value};
use std::cell::RefCell;

use crate::context::{CreatureData, CreatureRef, ItemRef, CURRENT_CTX, LuaContext};
use crate::lua_mutation::{
    call_lua_add_item, call_lua_add_item_full, call_lua_get_depot_chest, call_lua_get_inbox,
    call_lua_remove_item,
};
use crate::userdata::container::ContainerRef;

/// Register the Creature metatable in the Lua runtime.
pub fn register_creature_metatable(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<CreatureRef>(|_registry| {})
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

impl UserData for CreatureRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getId", |_, this, ()| Ok(this.0));

        methods.add_method("getName", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_creature(this.0)
                    .map(|c: CreatureData| c.name)
                    .ok_or_else(|| mlua::Error::runtime("creature not found"))
            })
        });

        methods.add_method("getGuid", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_creature(this.0)
                    .map(|c: CreatureData| c.guid)
                    .ok_or_else(|| mlua::Error::runtime("creature not found"))
            })
        });

        methods.add_method("getSlotItem", |lua, this, slot: u8| {
            let id_opt = with_ctx(|ctx| Ok(ctx.get_player_slot_item_id(this.0, slot)))?;
            match id_opt {
                Some(iid) => {
                    let ud = lua.create_userdata(ItemRef(iid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("getCapacity", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_player_capacity(this.0)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        methods.add_method("getFreeCapacity", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_player_free_capacity(this.0)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        methods.add_method(
            "addItem",
            |lua,
             this,
             (item_type, count, can_drop, sub_type, slot): (
                mlua::Value,
                Option<u32>,
                Option<bool>,
                Option<i32>,
                Option<u8>,
            )| {
                let (item_type, count, sub_type) = match item_type {
                    mlua::Value::Integer(n) => (n as u16, count.unwrap_or(1), sub_type.unwrap_or(-1)),
                    mlua::Value::Number(n) => (n as u16, count.unwrap_or(1), sub_type.unwrap_or(-1)),
                    mlua::Value::String(s) => {
                        let name = s.to_str()?.to_string();
                        let ty = with_ctx(|ctx| {
                            ctx.get_item_type_id_by_name(&name)
                                .ok_or_else(|| mlua::Error::runtime("unknown item name"))
                        })?;
                        (ty, count.unwrap_or(1), sub_type.unwrap_or(-1))
                    }
                    _ => return Err(mlua::Error::runtime("invalid item type")),
                };
                let can_drop = can_drop.unwrap_or(true);
                let slot = slot.unwrap_or(0);

                if can_drop || sub_type != -1 || slot != 0 {
                    let id_opt = call_lua_add_item_full(
                        this.0,
                        item_type,
                        count,
                        sub_type,
                        can_drop,
                        slot,
                    )
                    .map_err(mlua::Error::runtime)?;
                    match id_opt {
                        Some(iid) => {
                            let ud = lua.create_userdata(ItemRef(iid))?;
                            Ok(Value::UserData(ud))
                        }
                        None => Ok(Value::Nil),
                    }
                } else {
                    call_lua_add_item(this.0, item_type, count.min(u16::MAX as u32) as u16)
                        .map_err(mlua::Error::runtime)?;
                    Ok(Value::Nil)
                }
            },
        );

        methods.add_method("getItemCount", |_, this, (item_type, sub_type): (u16, Option<i32>)| {
            let sub_type = sub_type.unwrap_or(-1);
            with_ctx(|ctx| {
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
                    .map_err(mlua::Error::runtime)
            },
        );

        methods.add_method(
            "getItemById",
            |lua,
             this,
             (item_type, deep_search, sub_type): (mlua::Value, bool, Option<i32>)| {
                let sub_type = sub_type.unwrap_or(-1);
                let item_id = match item_type {
                    mlua::Value::Integer(n) => n as u16,
                    mlua::Value::Number(n) => n as u16,
                    mlua::Value::String(s) => {
                        let name = s.to_str()?.to_string();
                        with_ctx(|ctx| {
                            ctx.get_item_type_id_by_name(&name)
                                .ok_or_else(|| mlua::Error::runtime("unknown item name"))
                        })?
                    }
                    _ => return Err(mlua::Error::runtime("invalid item id")),
                };
                let id_opt = with_ctx(|ctx| {
                    Ok(ctx.find_player_item_by_type(this.0, item_id, deep_search, sub_type))
                })?;
                match id_opt {
                    Some(iid) => {
                        let ud = lua.create_userdata(ItemRef(iid))?;
                        Ok(Value::UserData(ud))
                    }
                    None => Ok(Value::Nil),
                }
            },
        );

        methods.add_method(
            "getDepotChest",
            |lua, this, (depot_id, auto_create): (u32, Option<bool>)| {
                let auto_create = auto_create.unwrap_or(false);
                let id_opt = call_lua_get_depot_chest(this.0, depot_id, auto_create)
                    .map_err(mlua::Error::runtime)?;
                match id_opt {
                    Some(iid) => {
                        let ud = lua.create_userdata(ContainerRef(iid))?;
                        Ok(Value::UserData(ud))
                    }
                    None => Ok(Value::Boolean(false)),
                }
            },
        );

        methods.add_method("getInbox", |lua, this, ()| {
            let id_opt =
                call_lua_get_inbox(this.0).map_err(mlua::Error::runtime)?;
            match id_opt {
                Some(iid) => {
                    let ud = lua.create_userdata(ContainerRef(iid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Boolean(false)),
            }
        });

        methods.add_method("getContainerId", |_, this, container: mlua::AnyUserData| {
            let container_id = container
                .borrow::<ContainerRef>()
                .map(|c| c.0)
                .or_else(|_| container.borrow::<ItemRef>().map(|i| i.0))?;
            with_ctx(|ctx| {
                Ok(ctx
                    .get_player_container_id(this.0, container_id)
                    .map(i32::from)
                    .unwrap_or(-1))
            })
        });

        methods.add_method("getContainerById", |lua, this, client_cid: u8| {
            let id_opt = with_ctx(|ctx| Ok(ctx.get_player_container_by_cid(this.0, client_cid)))?;
            match id_opt {
                Some(iid) => {
                    let ud = lua.create_userdata(ContainerRef(iid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("getContainerIndex", |_, this, client_cid: u8| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_player_container_index(this.0, client_cid)
                    .map(i32::from)
                    .unwrap_or(-1))
            })
        });
    }
}
