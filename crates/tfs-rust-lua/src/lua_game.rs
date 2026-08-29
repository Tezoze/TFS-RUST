//! Talkaction Phase 5 helpers — Game admin Lua surface (not `runtime.rs` dump).
//!
//! C++ reference: `luascript.cpp` `luaGameCreateNpc` / `luaGameSetGameState` /
//! `luaGameReload` / `luaGameUnlockAccount` / `luaGameUnlockIP` /
//! `luaGetIPNumberFromString` / `luaRefreshMap` / `luaGameGetItemAttributeByName` /
//! `luaGameGetExperienceStage`.

use mlua::{Lua, Value};

use crate::context::current_ctx;
use crate::lua_mutation::{call_create_npc, call_game_reload, call_set_game_state};
use crate::userdata::npc::NpcRef;
use crate::userdata::position::PositionRef;

/// TVP `ReloadTypes_t` (no mounts): `RELOAD_TYPE_SCRIPTS` / `TALKACTIONS` / `ALL`.
pub const RELOAD_TYPE_ALL: i32 = 0;
pub const RELOAD_TYPE_SCRIPTS: i32 = 13;
pub const RELOAD_TYPE_TALKACTIONS: i32 = 15;

/// TVP `GameState_t` — `GAME_STATE_NORMAL` / `CLOSED` / `SHUTDOWN`.
pub const GAME_STATE_STARTUP: i32 = 0;
pub const GAME_STATE_INIT: i32 = 1;
pub const GAME_STATE_NORMAL: i32 = 2;
pub const GAME_STATE_CLOSED: i32 = 3;
pub const GAME_STATE_SHUTDOWN: i32 = 4;
pub const GAME_STATE_CLOSING: i32 = 5;
pub const GAME_STATE_MAINTAIN: i32 = 6;

/// LSB = first dotted octet (matches `Game.convertIpToString`).
pub fn format_ip_string(ip: u32) -> String {
    format!(
        "{}.{}.{}.{}",
        ip & 0xFF,
        (ip >> 8) & 0xFF,
        (ip >> 16) & 0xFF,
        ip >> 24
    )
}

/// `boost::asio::ip::address_v4::to_uint` / `luaGetIPNumberFromString`.
/// LSB = first dotted octet (matches `Game.convertIpToString`).
pub fn ip_number_from_string(s: &str) -> u32 {
    let mut parts = s.split('.');
    let mut oct = [0u8; 4];
    for slot in &mut oct {
        let Some(p) = parts.next() else {
            return 0;
        };
        let Ok(v) = p.parse::<u8>() else {
            return 0;
        };
        *slot = v;
    }
    if parts.next().is_some() {
        return 0;
    }
    u32::from(oct[0]) | u32::from(oct[1]) << 8 | u32::from(oct[2]) << 16 | u32::from(oct[3]) << 24
}

/// `tools.cpp` `stringToItemAttribute` — unknown → `ITEM_ATTRIBUTE_NONE` (0).
/// Remere door/key names return the string alias so `getAttribute` matches pack Lua.
pub fn item_attribute_by_name(lua: &Lua, name: &str) -> Result<Value, mlua::Error> {
    let key = name.trim().to_ascii_lowercase();
    let bits: Option<i32> = match key.as_str() {
        "aid" | "actionid" => Some(1 << 0),
        "uid" | "uniqueid" => Some(1 << 1),
        "description" => Some(1 << 2),
        "text" => Some(1 << 3),
        "duration" => Some(1 << 17),
        "charges" => Some(1 << 20),
        _ => None,
    };
    if let Some(n) = bits {
        return Ok(Value::Integer(i64::from(n)));
    }
    match key.as_str() {
        "keynumber" | "keyholenumber" | "doorquestnumber" | "doorquestvalue" | "doorlevel" => {
            Ok(Value::String(lua.create_string(&key)?))
        }
        _ => Ok(Value::Integer(0)),
    }
}

fn parse_lua_position(pos: Value) -> Result<(u16, u16, u8), mlua::Error> {
    match pos {
        Value::UserData(ud) => {
            if let Ok(p) = ud.borrow::<PositionRef>() {
                Ok((p.x, p.y, p.z))
            } else {
                Err(mlua::Error::runtime("expected Position"))
            }
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

/// Attach Phase 5/6 `Game.*` methods onto the existing class table.
pub fn register_game_admin_api(lua: &Lua, game: &mlua::Table) -> Result<(), mlua::Error> {
    game.set(
        "createNpc",
        lua.create_function(|lua, (name, pos, force): (String, Value, Option<bool>)| {
            let (x, y, z) = parse_lua_position(pos)
                .map_err(|_| mlua::Error::runtime("Game.createNpc: expected Position"))?;
            match call_create_npc(name, x, y, z, force.unwrap_or(false)) {
                Ok(Some(id)) => {
                    let ud = lua.create_userdata(NpcRef(id))?;
                    Ok(Value::UserData(ud))
                }
                Ok(None) => Ok(Value::Nil),
                Err(e) => Err(mlua::Error::runtime(e)),
            }
        })?,
    )?;
    game.set(
        "setGameState",
        lua.create_function(|_, state: i32| {
            call_set_game_state(state).map_err(mlua::Error::runtime)?;
            Ok(true)
        })?,
    )?;
    game.set(
        "reload",
        lua.create_function(|_, ty: i32| {
            call_game_reload(ty).map_err(mlua::Error::runtime)?;
            Ok(true)
        })?,
    )?;
    game.set(
        "unlockAccount",
        lua.create_function(|_, _account: u32| {
            // TFS `luaGameUnlockAccount` — login-attempt map (always true). No lock map yet.
            Ok(true)
        })?,
    )?;
    game.set("unlockIp", lua.create_function(|_, _ip: u32| Ok(true))?)?;
    game.set(
        "getItemAttributeByName",
        lua.create_function(|lua, name: String| item_attribute_by_name(lua, &name))?,
    )?;
    game.set(
        "getExperienceStage",
        lua.create_function(|_, level: i32| {
            Ok(current_ctx(|ctx| ctx.get_experience_stage(level)).unwrap_or(1.0))
        })?,
    )?;
    Ok(())
}

pub fn register_admin_globals(lua: &Lua) -> Result<(), mlua::Error> {
    let globals = lua.globals();
    globals.set(
        "getIPNumberFromString",
        lua.create_function(|_, s: String| Ok(ip_number_from_string(&s)))?,
    )?;
    globals.set(
        "refreshMap",
        lua.create_function(|_, ()| {
            tracing::info!("refreshMap: full remap is out of scope; returning 0");
            Ok(0u32)
        })?,
    )?;
    globals.set(
        "getWorldUpTime",
        lua.create_function(|_, ()| Ok(current_ctx(|ctx| ctx.get_world_up_time()).unwrap_or(0)))?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_number_matches_convert_ip_to_string_endianness() {
        let ip = ip_number_from_string("127.0.0.1");
        assert_eq!(ip & 0xFF, 127);
        assert_eq!((ip >> 8) & 0xFF, 0);
        assert_eq!((ip >> 16) & 0xFF, 0);
        assert_eq!(ip >> 24, 1);
        assert_eq!(ip_number_from_string("not-an-ip"), 0);
        assert_eq!(ip_number_from_string("1.2.3"), 0);
    }

    #[test]
    fn item_attribute_by_name_aid_is_actionid_bit() {
        let lua = Lua::new();
        match item_attribute_by_name(&lua, "aid").expect("aid") {
            Value::Integer(n) => assert_eq!(n, 1),
            other => panic!("expected int, got {other:?}"),
        }
        match item_attribute_by_name(&lua, "nope").expect("none") {
            Value::Integer(n) => assert_eq!(n, 0),
            other => panic!("expected 0, got {other:?}"),
        }
        match item_attribute_by_name(&lua, "keynumber").expect("key") {
            Value::String(s) => assert_eq!(s.to_str().unwrap(), "keynumber"),
            other => panic!("expected string, got {other:?}"),
        }
    }
}
