//! Position userdata for Lua (`Position` in TFS scripts).
//!
//! C++ reference: `src/luascript.cpp` — position userdata; helpers from
//! `data/lib/core/position.lua` (`getNextPosition`, `moveUpstairs`).

use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods, Value};

use crate::context::CURRENT_CTX;
use crate::lua_mutation::call_lua_send_magic_effect;

/// Position handle wrapping `(x, y, z)` coordinates for Lua.
#[derive(Clone, Copy, Debug)]
pub struct PositionRef {
    pub x: u16,
    pub y: u16,
    pub z: u8,
}

impl UserData for PositionRef {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        // Scripts read/write `pos.x` / `pos.y` / `pos.z` (find_person, levitate).
        fields.add_field_method_get("x", |_, this| Ok(this.x));
        fields.add_field_method_get("y", |_, this| Ok(this.y));
        fields.add_field_method_get("z", |_, this| Ok(this.z));
        fields.add_field_method_set("x", |_, this, v: u16| {
            this.x = v;
            Ok(())
        });
        fields.add_field_method_set("y", |_, this, v: u16| {
            this.y = v;
            Ok(())
        });
        fields.add_field_method_set("z", |_, this, v: u8| {
            this.z = v;
            Ok(())
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("sendMagicEffect", |_, this, effect: u8| {
            call_lua_send_magic_effect(this.x, this.y, this.z, effect).map_err(mlua::Error::runtime)
        });

        methods.add_method("getX", |_, this, ()| Ok(this.x));
        methods.add_method("getY", |_, this, ()| Ok(this.y));
        methods.add_method("getZ", |_, this, ()| Ok(this.z));

        // `Position:getNextPosition(direction[, steps])` — `data/lib/core/position.lua`.
        methods.add_method_mut(
            "getNextPosition",
            |_, this, (direction, steps): (u8, Option<u16>)| {
                let steps = steps.unwrap_or(1) as i32;
                let (dx, dy) = direction_offset(direction);
                this.x = (this.x as i32 + dx * steps) as u16;
                this.y = (this.y as i32 + dy * steps) as u16;
                Ok(())
            },
        );

        // `Position:moveUpstairs()` — `data/lib/core/position.lua`.
        // Mutates self to a walkable tile one floor up; returns self.
        methods.add_method_mut("moveUpstairs", |lua, this, ()| {
            if this.z == 0 {
                let ud = lua.create_userdata(*this)?;
                return Ok(Value::UserData(ud));
            }
            this.z = this.z.saturating_sub(1);

            // Prefer south of upstairs tile, else scan directions.
            let candidates: [(i16, i16); 8] = [
                (0, 1),  // SOUTH default
                (0, -1), // NORTH
                (1, 0),  // EAST
                (-1, 0), // WEST
                (-1, 1), // SW
                (1, 1),  // SE
                (-1, -1), // NW
                (1, -1), // NE
            ];

            let base_x = this.x;
            let base_y = this.y;
            let z = this.z;

            let pick = CURRENT_CTX.with(|c| {
                let Some(ptr) = *c.borrow() else {
                    return Some((base_x, (base_y as i32 + 1) as u16));
                };
                if ptr.is_null() {
                    return Some((base_x, (base_y as i32 + 1) as u16));
                }
                let ctx = unsafe { &*ptr };
                for (dx, dy) in candidates {
                    let x = (base_x as i32 + dx as i32) as u16;
                    let y = (base_y as i32 + dy as i32) as u16;
                    if ctx.tile_is_walkable(x, y, z) {
                        return Some((x, y));
                    }
                }
                // Fallback: south even if not walkable (matches lib swap).
                Some((base_x, (base_y as i32 + 1) as u16))
            });

            if let Some((x, y)) = pick {
                this.x = x;
                this.y = y;
            }
            let ud = lua.create_userdata(*this)?;
            Ok(Value::UserData(ud))
        });

        // `Position + offset` used by moveUpstairs lib — optional.
        methods.add_meta_method(MetaMethod::Add, |lua, this, other: Value| {
            let (ox, oy, oz) = match other {
                Value::UserData(ud) => {
                    let o = ud.borrow::<PositionRef>()?;
                    (o.x as i32, o.y as i32, o.z as i32)
                }
                Value::Table(t) => {
                    let x: i64 = t.get("x").or_else(|_| t.get(1)).unwrap_or(0);
                    let y: i64 = t.get("y").or_else(|_| t.get(2)).unwrap_or(0);
                    let z: i64 = t.get("z").or_else(|_| t.get(3)).unwrap_or(0);
                    (x as i32, y as i32, z as i32)
                }
                _ => {
                    return Err(mlua::Error::runtime(
                        "Position+: expected Position or table",
                    ));
                }
            };
            let ud = lua.create_userdata(PositionRef {
                x: (this.x as i32 + ox) as u16,
                y: (this.y as i32 + oy) as u16,
                z: (this.z as i32 + oz) as u8,
            })?;
            Ok(Value::UserData(ud))
        });
    }
}

fn direction_offset(direction: u8) -> (i32, i32) {
    match direction {
        0 => (0, -1),  // NORTH
        1 => (1, 0),   // EAST
        2 => (0, 1),   // SOUTH
        3 => (-1, 0),  // WEST
        4 => (-1, 1),  // SOUTHWEST
        5 => (1, 1),   // SOUTHEAST
        6 => (-1, -1), // NORTHWEST
        7 => (1, -1),  // NORTHEAST
        _ => (0, 0),
    }
}

/// Register `Position` userdata + `Position(x, y, z)` / `Position(pos)` constructor.
pub fn register_position_metatable(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<PositionRef>(|_registry| {})?;
    let ctor = lua.create_function(|lua, args: mlua::MultiValue| {
        let mut iter = args.into_iter();
        let first = iter.next().unwrap_or(Value::Nil);
        match first {
            Value::UserData(ud) => {
                if let Ok(pos) = ud.borrow::<PositionRef>() {
                    let ud = lua.create_userdata(*pos)?;
                    return Ok(Value::UserData(ud));
                }
                Err(mlua::Error::runtime(
                    "Position(): expected Position userdata or coordinates",
                ))
            }
            Value::Table(t) => {
                let x: i64 = t.get("x").or_else(|_| t.get(1))?;
                let y: i64 = t.get("y").or_else(|_| t.get(2))?;
                let z: i64 = t.get("z").or_else(|_| t.get(3))?;
                let ud = lua.create_userdata(PositionRef {
                    x: x as u16,
                    y: y as u16,
                    z: z as u8,
                })?;
                Ok(Value::UserData(ud))
            }
            Value::Integer(x) => {
                let y = match iter.next().unwrap_or(Value::Nil) {
                    Value::Integer(i) => i as u16,
                    Value::Number(n) => n as u16,
                    _ => {
                        return Err(mlua::Error::runtime("Position(x,y,z): missing y"));
                    }
                };
                let z = match iter.next().unwrap_or(Value::Nil) {
                    Value::Integer(i) => i as u8,
                    Value::Number(n) => n as u8,
                    Value::Nil => 7,
                    _ => {
                        return Err(mlua::Error::runtime("Position(x,y,z): missing z"));
                    }
                };
                let ud = lua.create_userdata(PositionRef {
                    x: x as u16,
                    y,
                    z,
                })?;
                Ok(Value::UserData(ud))
            }
            Value::Number(x) => {
                let y = match iter.next().unwrap_or(Value::Nil) {
                    Value::Integer(i) => i as u16,
                    Value::Number(n) => n as u16,
                    _ => {
                        return Err(mlua::Error::runtime("Position(x,y,z): missing y"));
                    }
                };
                let z = match iter.next().unwrap_or(Value::Nil) {
                    Value::Integer(i) => i as u8,
                    Value::Number(n) => n as u8,
                    Value::Nil => 7,
                    _ => {
                        return Err(mlua::Error::runtime("Position(x,y,z): missing z"));
                    }
                };
                let ud = lua.create_userdata(PositionRef {
                    x: x as u16,
                    y,
                    z,
                })?;
                Ok(Value::UserData(ud))
            }
            Value::Nil => {
                let ud = lua.create_userdata(PositionRef { x: 0, y: 0, z: 0 })?;
                Ok(Value::UserData(ud))
            }
            _ => Err(mlua::Error::runtime(
                "Position(): expected x,y,z or Position/table",
            )),
        }
    })?;
    lua.globals().set("Position", ctor)?;
    Ok(())
}
