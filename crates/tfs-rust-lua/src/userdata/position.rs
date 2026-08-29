//! Position userdata for Lua (`Position` in TFS scripts).
//!
//! C++ reference: `src/luascript.cpp` — position userdata; helpers from
//! `data/lib/core/position.lua` (`getNextPosition`, `moveUpstairs`).
//! Pack `getDistanceBetween`: former `data/global.lua` (not corpus map distance).

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
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        crate::class_registry::register_with_recording(registry, "Position");
    }

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

        methods.add_method_mut("moveUpstairs", |lua, this, ()| {
            if this.z == 0 {
                let ud = lua.create_userdata(*this)?;
                return Ok(Value::UserData(ud));
            }
            this.z = this.z.saturating_sub(1);

            // Prefer south of upstairs tile, else scan directions.
            let candidates: [(i16, i16); 8] = [
                (0, 1),   // SOUTH default
                (0, -1),  // NORTH
                (1, 0),   // EAST
                (-1, 0),  // WEST
                (-1, 1),  // SW
                (1, 1),   // SE
                (-1, -1), // NW
                (1, -1),  // NE
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

        // `Position:isInRange(from, to)` — `data/lib/core/position.lua`.
        methods.add_method("isInRange", |_, this, (from, to): (Value, Value)| {
            let (fx, fy, fz) = parse_position_arg(&from)?;
            let (tx, ty, tz) = parse_position_arg(&to)?;
            let nw_x = fx.min(tx);
            let nw_y = fy.min(ty);
            let nw_z = fz.min(tz);
            let se_x = fx.max(tx);
            let se_y = fy.max(ty);
            let se_z = fz.max(tz);
            Ok(this.x >= nw_x
                && this.x <= se_x
                && this.y >= nw_y
                && this.y <= se_y
                && this.z >= nw_z
                && this.z <= se_z)
        });

        // `Position + offset` — doors.lua shoves with `Position(-1,0,0)` style offsets.
        // Right-hand Position stores wrapped u16 for negatives (`(-1i64) as u16`);
        // interpret as i16 so absolute + offset works (TFS Position is int32).
        methods.add_meta_method(MetaMethod::Add, |lua, this, other: Value| {
            let (ox, oy, oz) = match other {
                Value::UserData(ud) => {
                    let o = ud.borrow::<PositionRef>()?;
                    (o.x as i16 as i32, o.y as i16 as i32, o.z as i8 as i32)
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

        methods.add_meta_method(MetaMethod::Index, |lua, _this, key: mlua::LuaString| {
            crate::class_registry::class_index_lookup(
                lua,
                crate::class_registry::POSITION_INDEX_CHAIN,
                key,
            )
        });
    }
}

fn parse_position_arg(value: &Value) -> Result<(u16, u16, u8), mlua::Error> {
    match value {
        Value::UserData(ud) => {
            let p = ud.borrow::<PositionRef>()?;
            Ok((p.x, p.y, p.z))
        }
        Value::Table(t) => {
            let x: i64 = t.get("x").or_else(|_| t.get(1))?;
            let y: i64 = t.get("y").or_else(|_| t.get(2))?;
            let z: i64 = t.get("z").or_else(|_| t.get(3))?;
            Ok((x as u16, y as u16, z as u8))
        }
        _ => Err(mlua::Error::runtime("expected Position")),
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
                let ud = lua.create_userdata(PositionRef { x: x as u16, y, z })?;
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
                let ud = lua.create_userdata(PositionRef { x: x as u16, y, z })?;
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
    // `Position` is a class table (extensible by `function Position:method(...)`
    // in `data/lib/core/position.lua`) with a `__call` ctor. Gap 7a.
    crate::class_registry::register_class(lua, "Position", Some(ctor))?;
    register_get_distance_between(lua)?;
    Ok(())
}

/// Pack `getDistanceBetween` (`data/global.lua`): Chebyshev XY, +15 if Z differs.
/// Script range helper only — not walk/combat distance (corpus is XY Chebyshev).
const PACK_FLOOR_DISTANCE_EXTRA: i32 = 15;

fn register_get_distance_between(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.globals().set(
        "getDistanceBetween",
        lua.create_function(|_, (a, b): (Value, Value)| {
            let (ax, ay, az) = lua_position_xyz(&a)?;
            let (bx, by, bz) = lua_position_xyz(&b)?;
            let mut pos_dif = (ax - bx).abs().max((ay - by).abs());
            if az != bz {
                pos_dif += PACK_FLOOR_DISTANCE_EXTRA;
            }
            Ok(pos_dif)
        })?,
    )
}

fn lua_position_xyz(v: &Value) -> Result<(i32, i32, i32), mlua::Error> {
    match v {
        Value::UserData(ud) => {
            let p = ud
                .borrow::<PositionRef>()
                .map_err(|_| mlua::Error::runtime("getDistanceBetween: expected Position"))?;
            Ok((i32::from(p.x), i32::from(p.y), i32::from(p.z)))
        }
        Value::Table(t) => {
            let x: i64 = t.get("x").or_else(|_| t.get(1))?;
            let y: i64 = t.get("y").or_else(|_| t.get(2))?;
            let z: i64 = t.get("z").or_else(|_| t.get(3))?;
            Ok((x as i32, y as i32, z as i32))
        }
        _ => Err(mlua::Error::runtime(
            "getDistanceBetween: expected Position or {x,y,z}",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_add_negative_offset_yields_neighbor() {
        let lua = mlua::Lua::new();
        register_position_metatable(&lua).expect("Position register");
        let (ax, ay, az, bx, by, bz): (u16, u16, u8, u16, u16, u8) = lua
            .load(
                r#"
                local base = Position(100, 200, 7)
                local a = base + Position(-1, 0, 0)
                local b = base + Position(1, 0, 0)
                return a.x, a.y, a.z, b.x, b.y, b.z
                "#,
            )
            .eval()
            .expect("eval");
        assert_eq!((ax, ay, az), (99, 200, 7));
        assert_eq!((bx, by, bz), (101, 200, 7));
    }

    #[test]
    fn get_distance_between_chebyshev_plus_floor_extra() {
        let lua = mlua::Lua::new();
        register_position_metatable(&lua).expect("Position register");
        let same: i32 = lua
            .load("return getDistanceBetween(Position(10, 20, 7), Position(13, 21, 7))")
            .eval()
            .expect("same floor");
        assert_eq!(same, 3);
        let other: i32 = lua
            .load("return getDistanceBetween({x=10,y=20,z=7}, {x=13,y=21,z=6})")
            .eval()
            .expect("other floor");
        assert_eq!(other, 18);
    }
}
