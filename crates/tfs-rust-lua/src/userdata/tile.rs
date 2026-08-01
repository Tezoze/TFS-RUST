//! Tile userdata for Lua (`Tile` in TFS scripts) — PC-3a Gaps 5–6 surface.
//!
//! C++ reference: `luascript.cpp` `luaTileCreate` / `luaTileHasFlag` /
//! `luaTileGetGround` / `luaTileGetTopDownItem` / `luaTileGetItems` /
//! `luaTileGetItemByType` / `luaTileGetCreatures`.

use mlua::{UserData, UserDataMethods, Value};

use crate::context::{CURRENT_CTX, CreatureRef, ItemRef};
use crate::userdata::position::PositionRef;

/// Position-backed tile handle for Lua.
#[derive(Clone, Copy, Debug)]
pub struct TileRef {
    pub x: u16,
    pub y: u16,
    pub z: u8,
}

impl UserData for TileRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("hasProperty", |_, this, prop: i32| {
            CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_has_property(this.x, this.y, this.z, prop))
            })
        });

        // `tile:hasFlag(TILESTATE_*)` — `luascript.cpp` `luaTileHasFlag`.
        methods.add_method("hasFlag", |_, this, flags: i32| {
            CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_has_flag(this.x, this.y, this.z, flags))
            })
        });

        methods.add_method("getPosition", |lua, this, ()| {
            let ud = lua.create_userdata(PositionRef {
                x: this.x,
                y: this.y,
                z: this.z,
            })?;
            Ok(Value::UserData(ud))
        });

        methods.add_method("getGround", |lua, this, ()| {
            use crate::userdata::item_type::ItemTypeRef;
            let typ = CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_get_ground_type(this.x, this.y, this.z))
            })?;
            match typ {
                Some(t) if t != 0 => {
                    // Ground is stored as server type id (not SlotMap ItemId).
                    // ItemTypeRef exposes `:getId()` used by magic_rope / levitate.
                    let ud = lua.create_userdata(ItemTypeRef(t))?;
                    Ok(Value::UserData(ud))
                }
                _ => Ok(Value::Nil),
            }
        });

        methods.add_method("getTopDownItem", |lua, this, ()| {
            let id = CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_get_top_down_item(this.x, this.y, this.z))
            })?;
            match id {
                Some(iid) => {
                    let ud = lua.create_userdata(ItemRef(iid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("getItems", |lua, this, ()| {
            let ids = CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_get_items(this.x, this.y, this.z))
            })?;
            let t = lua.create_table_with_capacity(ids.len(), 0)?;
            for (i, iid) in ids.into_iter().enumerate() {
                let ud = lua.create_userdata(ItemRef(iid))?;
                t.set(i + 1, ud)?;
            }
            Ok(Value::Table(t))
        });

        // `tile:getItemByType(ITEM_TYPE_*)` — `luascript.cpp` `luaTileGetItemByType`.
        methods.add_method("getItemByType", |lua, this, type_tag: i32| {
            let id = CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_get_item_by_type(this.x, this.y, this.z, type_tag))
            })?;
            match id {
                Some(iid) => {
                    let ud = lua.create_userdata(ItemRef(iid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        // `tile:getCreatures()` — `luascript.cpp` `luaTileGetCreatures`.
        // Returns a 1-based array of Creature userdata (empty table when none).
        methods.add_method("getCreatures", |lua, this, ()| {
            let ids = CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_get_creatures(this.x, this.y, this.z))
            })?;
            let t = lua.create_table_with_capacity(ids.len(), 0)?;
            for (i, cid) in ids.into_iter().enumerate() {
                let ud = lua.create_userdata(CreatureRef(cid))?;
                t.set(i + 1, ud)?;
            }
            Ok(Value::Table(t))
        });

        // `tile:getTopVisibleThing([creature])` — `luascript.cpp` `luaTileGetTopVisibleThing`.
        // Doors Phase 3: key use-with re-resolves the door via this API.
        methods.add_method("getTopVisibleThing", |lua, this, viewer: Option<Value>| {
            let viewer_id = match viewer {
                None | Some(Value::Nil) => None,
                Some(Value::UserData(ud)) => {
                    if let Ok(cref) = ud.borrow::<CreatureRef>() {
                        Some(cref.0)
                    } else {
                        None
                    }
                }
                Some(Value::Integer(i)) => Some(i as u64),
                Some(Value::Number(n)) => Some(n as u64),
                _ => None,
            };
            let thing = CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_get_top_visible_thing(this.x, this.y, this.z, viewer_id))
            })?;
            match thing {
                Some(tfs_rust_common::ScriptThing::Item(iid)) => {
                    let ud = lua.create_userdata(ItemRef(iid))?;
                    Ok(Value::UserData(ud))
                }
                Some(tfs_rust_common::ScriptThing::Creature(cid)) => {
                    let ud = lua.create_userdata(CreatureRef(cid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        // `tile:getCreatureCount()` — `luascript.cpp` `luaTileGetCreatureCount`.
        methods.add_method("getCreatureCount", |_, this, ()| {
            CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_get_creature_count(this.x, this.y, this.z))
            })
        });

        // `tile:getThingCount()` — `luascript.cpp` `luaTileGetThingCount`.
        methods.add_method("getThingCount", |_, this, ()| {
            CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_get_thing_count(this.x, this.y, this.z))
            })
        });

        // `tile:getThing(index)` — `luascript.cpp` `luaTileGetThing`.
        methods.add_method("getThing", |lua, this, index: u32| {
            let thing = CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_get_thing(this.x, this.y, this.z, index))
            })?;
            match thing {
                Some(tfs_rust_common::ScriptThing::Item(iid)) => {
                    let ud = lua.create_userdata(ItemRef(iid))?;
                    Ok(Value::UserData(ud))
                }
                Some(tfs_rust_common::ScriptThing::Creature(cid)) => {
                    let ud = lua.create_userdata(CreatureRef(cid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        // `tile:getItemById(itemId)` — `luascript.cpp` `luaTileGetItemById`.
        methods.add_method("getItemById", |lua, this, item_id: u16| {
            let id = CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_get_item_by_id(this.x, this.y, this.z, item_id))
            })?;
            match id {
                Some(iid) => {
                    let ud = lua.create_userdata(ItemRef(iid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        // `tile:getItemByGroup(ITEM_GROUP_*)` — splash / magicfield (doors auto-close).
        methods.add_method("getItemByGroup", |lua, this, group: i32| {
            let id = CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_get_item_by_group(this.x, this.y, this.z, group))
            })?;
            match id {
                Some(iid) => {
                    let ud = lua.create_userdata(ItemRef(iid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        // `tile:queryAdd(thing[, flags])` — `luascript.cpp` `luaTileQueryAdd`.
        methods.add_method(
            "queryAdd",
            |_, this, (thing, flags): (Value, Option<u32>)| {
                let flags = flags.unwrap_or(0);
                let creature_id = match thing {
                    Value::UserData(ud) => {
                        if let Ok(cref) = ud.borrow::<CreatureRef>() {
                            cref.0
                        } else {
                            return Ok(1i32); // NOTPOSSIBLE
                        }
                    }
                    Value::Integer(i) => i as u64,
                    Value::Number(n) => n as u64,
                    _ => return Ok(1i32),
                };
                CURRENT_CTX.with(|c| {
                    let ptr =
                        (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                    if ptr.is_null() {
                        return Err(mlua::Error::runtime("LuaContext not set"));
                    }
                    let ctx = unsafe { &*ptr };
                    Ok(ctx.tile_query_add_creature(this.x, this.y, this.z, creature_id, flags))
                })
            },
        );

        // Lib `Tile:isWalkable` used by moveUpstairs.
        methods.add_method("isWalkable", |_, this, ()| {
            CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_is_walkable(this.x, this.y, this.z))
            })
        });
    }
}

fn parse_tile_args(args: mlua::MultiValue) -> Result<(u16, u16, u8), mlua::Error> {
    let mut iter = args.into_iter();
    let first = iter.next().unwrap_or(Value::Nil);
    match first {
        Value::UserData(ud) => {
            if let Ok(pos) = ud.borrow::<PositionRef>() {
                return Ok((pos.x, pos.y, pos.z));
            }
            if let Ok(tile) = ud.borrow::<TileRef>() {
                return Ok((tile.x, tile.y, tile.z));
            }
            Err(mlua::Error::runtime(
                "Tile(): expected Position or Tile userdata",
            ))
        }
        Value::Table(t) => {
            let x: i64 = t
                .get("x")
                .or_else(|_| t.get(1))
                .map_err(|_| mlua::Error::runtime("Tile(): pos missing x"))?;
            let y: i64 = t
                .get("y")
                .or_else(|_| t.get(2))
                .map_err(|_| mlua::Error::runtime("Tile(): pos missing y"))?;
            let z: i64 = t
                .get("z")
                .or_else(|_| t.get(3))
                .map_err(|_| mlua::Error::runtime("Tile(): pos missing z"))?;
            Ok((x as u16, y as u16, z as u8))
        }
        Value::Integer(x) => {
            let y = match iter.next().unwrap_or(Value::Nil) {
                Value::Integer(i) => i as u16,
                Value::Number(n) => n as u16,
                _ => return Err(mlua::Error::runtime("Tile(x,y,z): missing y")),
            };
            let z = match iter.next().unwrap_or(Value::Nil) {
                Value::Integer(i) => i as u8,
                Value::Number(n) => n as u8,
                _ => return Err(mlua::Error::runtime("Tile(x,y,z): missing z")),
            };
            Ok((x as u16, y, z))
        }
        Value::Number(x) => {
            let y = match iter.next().unwrap_or(Value::Nil) {
                Value::Integer(i) => i as u16,
                Value::Number(n) => n as u16,
                _ => return Err(mlua::Error::runtime("Tile(x,y,z): missing y")),
            };
            let z = match iter.next().unwrap_or(Value::Nil) {
                Value::Integer(i) => i as u8,
                Value::Number(n) => n as u8,
                _ => return Err(mlua::Error::runtime("Tile(x,y,z): missing z")),
            };
            Ok((x as u16, y, z))
        }
        Value::Nil => Err(mlua::Error::runtime("Tile(): nil position")),
        _ => Err(mlua::Error::runtime(
            "Tile(): expected Position, table, or x,y,z",
        )),
    }
}

/// Register `Tile(pos)` / `Tile(x,y,z)` constructor — C++ `luaTileCreate`.
pub fn register_tile_constructor(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<TileRef>(|_registry| {})?;
    let tile_new = lua.create_function(|lua, args: mlua::MultiValue| {
        let (x, y, z) = parse_tile_args(args)?;
        let exists = CURRENT_CTX.with(|c| {
            let Some(ptr) = *c.borrow() else {
                return true;
            };
            if ptr.is_null() {
                return true;
            }
            let ctx = unsafe { &*ptr };
            ctx.tile_exists(x, y, z)
        });
        if !exists {
            return Ok(Value::Nil);
        }
        let ud = lua.create_userdata(TileRef { x, y, z })?;
        Ok(Value::UserData(ud))
    })?;
    lua.globals().set("Tile", tile_new)?;
    Ok(())
}
