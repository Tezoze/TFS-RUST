//! Tile userdata for Lua (`Tile` in TFS scripts) — minimal PC-3a Phase 8 surface.
//!
//! C++ reference: `luascript.cpp` `luaTileCreate` / `luaTileHasProperty`;
//! `Tile::hasProperty` — `tile.cpp:27`.

use mlua::{UserData, UserDataMethods, Value};

use crate::context::CURRENT_CTX;
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
        // `tile:hasProperty(CONST_PROP_*)` — `luascript.cpp` `luaTileHasProperty`.
        // Field runes gate on `CONST_PROP_BLOCKSOLID` (0) before CREATEITEM.
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

        methods.add_method("getPosition", |lua, this, ()| {
            let ud = lua.create_userdata(PositionRef {
                x: this.x,
                y: this.y,
                z: this.z,
            })?;
            Ok(Value::UserData(ud))
        });
    }
}

fn parse_tile_position(arg: Value) -> Result<(u16, u16, u8), mlua::Error> {
    match arg {
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
        Value::Nil => Err(mlua::Error::runtime("Tile(): nil position")),
        _ => Err(mlua::Error::runtime(
            "Tile(): expected Position table or userdata",
        )),
    }
}

/// Register `Tile(pos)` constructor — C++ `luaTileCreate`.
pub fn register_tile_constructor(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<TileRef>(|_registry| {})?;
    let tile_new = lua.create_function(|lua, arg: Value| {
        let (x, y, z) = parse_tile_position(arg)?;
        // Missing / void tiles return nil — TFS `luaTileCreate` when tile is null.
        let exists = CURRENT_CTX.with(|c| {
            let Some(ptr) = *c.borrow() else {
                return true; // no context (load time) — still construct
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
