//! Tile userdata for Lua (`Tile` in TFS scripts) — PC-3a Gaps 5–6 surface.
//!
//! C++ reference: `luascript.cpp` `luaTileCreate` / `luaTileHasFlag` /
//! `luaTileGetGround` / `luaTileGetTopDownItem` / `luaTileGetItems` /
//! `luaTileGetItemByType` / `luaTileGetCreatures`.

use mlua::{MetaMethod, UserData, UserDataMethods, Value};

use crate::context::{CURRENT_CTX, CreatureRef, ItemRef};
use crate::lua_mutation::call_lua_tile_add_item;
use crate::userdata::item::parse_lua_item_type_id;
use crate::userdata::position::PositionRef;

/// Position-backed tile handle for Lua.
#[derive(Clone, Copy, Debug)]
pub struct TileRef {
    pub x: u16,
    pub y: u16,
    pub z: u8,
}

/// House handle for `tile:getHouse()` — TFS `luaTileGetHouse`.
/// Truthy userdata (never `0`). E7; 772 `IsHouse` (`map.cc:2474`).
#[derive(Clone, Copy, Debug)]
pub struct HouseRef(pub u32);

impl UserData for HouseRef {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        crate::class_registry::register_with_recording(registry, "House");
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // `house:getId()` — TFS `luaHouseGetId`.
        methods.add_method("getId", |_, this, ()| Ok(this.0));
    }
}

impl UserData for TileRef {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        crate::class_registry::register_with_recording(registry, "Tile");
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // TFS `Thing::isTile` — `moveitem.lua` `toCylinder:isTile()`.
        methods.add_method("isTile", |_, _this, ()| Ok(true));

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
            let id = CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_get_ground_item(this.x, this.y, this.z))
            })?;
            match id {
                Some(iid) => {
                    let ud = lua.create_userdata(ItemRef(iid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
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

        // `tile:getBottomCreature()` — `luascript.cpp` `luaTileGetBottomCreature`.
        // TFS `creatures->rbegin()` (oldest). Rust `push`s newest last → first entry.
        methods.add_method("getBottomCreature", |lua, this, ()| {
            let id = CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_get_bottom_creature(this.x, this.y, this.z))
            })?;
            match id {
                Some(cid) => {
                    let ud = lua.create_userdata(CreatureRef(cid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
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

        // `tile:addItem(itemId[, count/subType = 1[, flags = 0]])` — `luaTileAddItem`.
        methods.add_method(
            "addItem",
            |lua, this, (item_id, count, flags): (Value, Option<u32>, Option<u32>)| {
                let Some(item_type) = parse_lua_item_type_id(item_id)? else {
                    return Ok(Value::Nil);
                };
                let count = count.unwrap_or(1).min(u32::from(u16::MAX)) as u16;
                let flags = flags.unwrap_or(0);
                match call_lua_tile_add_item(this.x, this.y, this.z, item_type, count, flags) {
                    Ok(Some(id)) => {
                        let ud = lua.create_userdata(ItemRef(id))?;
                        Ok(Value::UserData(ud))
                    }
                    Ok(None) => Ok(Value::Nil),
                    Err(e) => Err(mlua::Error::runtime(e)),
                }
            },
        );

        // `tile:addItemEx(item[, flags = 0])` — `luaTileAddItemEx`.
        methods.add_method(
            "addItemEx",
            |_, this, (item, flags): (Value, Option<u32>)| {
                let Some(item_id) = crate::userdata::item::item_script_id_from_value(&item) else {
                    return Ok(Value::Nil);
                };
                let parent = CURRENT_CTX.with(|c| {
                    let ptr =
                        (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                    if ptr.is_null() {
                        return Err(mlua::Error::runtime("LuaContext not set"));
                    }
                    let ctx = unsafe { &*ptr };
                    Ok(ctx.get_item_parent(item_id))
                })?;
                if parent.is_some() {
                    return Ok(Value::Nil);
                }
                let rv = crate::lua_mutation::call_lua_add_item_ex(
                    item_id,
                    crate::lua_mutation::LuaMoveDestination::Tile {
                        x: this.x,
                        y: this.y,
                        z: this.z,
                    },
                    false,
                    -1,
                    flags.unwrap_or(0),
                )
                .map_err(mlua::Error::runtime)?;
                Ok(Value::Integer(i64::from(rv)))
            },
        );

        // `tile:getHouse()` — TFS `luaTileGetHouse`. `nil` or House userdata.
        // Never `0` (Lua 0 is truthy). 772 `IsHouse(Obj1)` (`moveuse.cc:313-318`).
        methods.add_method("getHouse", |lua, this, ()| {
            let house_id = CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.tile_get_house_id(this.x, this.y, this.z))
            })?;
            match house_id {
                Some(id) if id != 0 => {
                    let ud = lua.create_userdata(HouseRef(id))?;
                    Ok(Value::UserData(ud))
                }
                _ => Ok(Value::Nil),
            }
        });

        // Gap 7b — `__index` fallback so `tile:relocateTo(pos)` /
        // `tile:isCreature()` resolve methods defined as
        // `function Tile.relocateTo(self, ...)` in `data/lib/core/tile.lua`.
        // Native methods above keep priority (mlua calls `__index` only on
        // miss). C++ `LuaScriptInterface::registerClass`; shared helper in
        // `class_registry`.
        methods.add_meta_method(MetaMethod::Index, |lua, _this, key: mlua::LuaString| {
            crate::class_registry::class_index_lookup(
                lua,
                crate::class_registry::TILE_INDEX_CHAIN,
                key,
            )
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

/// `Game.createTile(position[, isDynamic])` / `Game.createTile(x, y, z[, isDynamic])`.
pub(crate) fn parse_create_tile_args(
    args: mlua::MultiValue,
) -> Result<(u16, u16, u8, bool), mlua::Error> {
    let values: Vec<Value> = args.into_iter().collect();
    let is_dynamic = match values.first() {
        Some(Value::Table(_) | Value::UserData(_)) => match values.get(1) {
            Some(Value::Boolean(b)) => *b,
            _ => false,
        },
        _ => match values.get(3) {
            Some(Value::Boolean(b)) => *b,
            _ => false,
        },
    };
    let (x, y, z) = parse_tile_args(values.into_iter().collect())?;
    Ok((x, y, z, is_dynamic))
}

/// Register `Tile(pos)` / `Tile(x,y,z)` constructor — C++ `luaTileCreate`.
///
/// `Tile` is registered via `register_class` so it is a class table (extensible
/// by `function Tile.relocateTo(self, ...)` in `data/lib/core/tile.lua`) with a
/// `__call` ctor. Gap 7a — C++ `LuaScriptInterface::registerClass`.
pub fn register_tile_constructor(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<TileRef>(|_registry| {})?;
    lua.register_userdata_type::<HouseRef>(|_registry| {})?;
    crate::class_registry::register_class(lua, "House", None)?;
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
    crate::class_registry::register_class(lua, "Tile", Some(tile_new))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::with_lua_context;
    use mlua::Lua;
    use tfs_rust_common::{
        ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptItemData, ScriptItemId,
        ScriptItemRef,
    };

    struct TileCtx;

    impl ScriptContext for TileCtx {
        fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
            (id == 7).then_some(ScriptCreatureData {
                name: "Bottom".into(),
                guid: 1,
            })
        }
        fn get_item(&self, id: ScriptItemId) -> Option<ScriptItemRef> {
            Some(ScriptItemRef(id))
        }
        fn get_config_string(&self, _: &str) -> Option<String> {
            None
        }
        fn get_item_data(&self, _id: ScriptItemId) -> Option<ScriptItemData> {
            Some(ScriptItemData {
                item_type: 100,
                count: 1,
                weight: 0,
                name: "grass".into(),
                action_id: 4000,
                unique_id: 0,
                is_store_item: false,
                fluid_type: 0,
                sub_type: 1,
            })
        }
        fn tile_exists(&self, _: u16, _: u16, _: u8) -> bool {
            true
        }
        fn tile_get_ground_item(&self, _: u16, _: u16, _: u8) -> Option<ScriptItemId> {
            Some(11)
        }
        fn tile_get_creatures(&self, _: u16, _: u16, _: u8) -> Vec<ScriptCreatureId> {
            vec![7, 8]
        }
        fn tile_get_house_id(&self, x: u16, y: u16, z: u8) -> Option<u32> {
            if x == 1 && y == 1 && z == 7 {
                Some(42)
            } else {
                None
            }
        }
    }

    #[test]
    fn get_ground_is_item_userdata_and_get_bottom_creature() {
        let lua = Lua::new();
        register_tile_constructor(&lua).expect("tile");
        crate::userdata::register_item_metatable(&lua).expect("item");
        crate::userdata::register_creature_metatable(&lua).expect("creature");

        with_lua_context(&TileCtx, || {
            let tile = lua
                .create_userdata(TileRef { x: 1, y: 1, z: 7 })
                .expect("tile ud");
            lua.globals().set("t", tile).unwrap();

            let aid: u16 = lua
                .load("return t:getGround():getActionId()")
                .eval()
                .unwrap();
            assert_eq!(aid, 4000);

            let name: String = lua
                .load("return t:getBottomCreature():getName()")
                .eval()
                .unwrap();
            assert_eq!(name, "Bottom");
        });
    }

    /// E7: house tile returns House userdata (truthy); non-house returns nil, never 0.
    #[test]
    fn e7_get_house_is_nil_or_userdata_never_zero() {
        let lua = Lua::new();
        register_tile_constructor(&lua).expect("tile");

        with_lua_context(&TileCtx, || {
            let house_tile = lua
                .create_userdata(TileRef { x: 1, y: 1, z: 7 })
                .expect("house tile");
            lua.globals().set("h", house_tile).unwrap();
            let kind: String = lua.load("return type(h:getHouse())").eval().unwrap();
            assert_eq!(kind, "userdata");
            let id: u32 = lua.load("return h:getHouse():getId()").eval().unwrap();
            assert_eq!(id, 42);
            let truthy: bool = lua
                .load("return h:getHouse() and true or false")
                .eval()
                .unwrap();
            assert!(truthy);

            let other = lua
                .create_userdata(TileRef { x: 2, y: 2, z: 7 })
                .expect("other");
            lua.globals().set("n", other).unwrap();
            let is_nil: bool = lua.load("return n:getHouse() == nil").eval().unwrap();
            assert!(is_nil, "non-house must be nil, not 0");
            let not_zero: bool = lua.load("return n:getHouse() ~= 0").eval().unwrap();
            assert!(not_zero);
        });
    }

    /// R5: `Game.createTile({x,y,z}, true)` returns Tile userdata.
    #[test]
    fn r5_game_create_tile_table_form_returns_tile() {
        crate::lua_mutation::register_lua_mutation_applier(|_, mutation| match mutation {
            crate::lua_mutation::LuaMutation::GameCreateTile {
                x: 32426,
                y: 32201,
                z: 14,
                is_dynamic: true,
            } => Ok(()),
            other => panic!("unexpected mutation {other:?}"),
        });
        let runtime = crate::runtime::LuaRuntime::new().expect("runtime");
        let lua = &runtime.lua;
        crate::lua_mutation::with_lua_mutation_scope(std::ptr::without_provenance_mut(1), || {
            let z: u8 = lua
                .load(
                    "local t = Game.createTile({x = 32426, y = 32201, z = 14}, true)
                     return t:getPosition().z",
                )
                .eval()
                .expect("createTile");
            assert_eq!(z, 14);
        });
    }
}
