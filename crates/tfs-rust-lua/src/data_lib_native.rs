//! Native replacements for `data/lib/core/*.lua` — registered after lib load.
//!
//! Pack surface: `Game.*` map helpers, global storage — `data/lib/core/game.lua`.
//! C++ reference: TFS composes tile/item APIs in Lua; Rust owns outcomes natively.

use mlua::{Lua, Value};

use crate::class_registry;
use crate::context::current_ctx;
use crate::lua_game::format_ip_string;
use crate::lua_mutation::{
    call_game_map_remove_item, call_game_map_remove_movable_items,
    call_game_map_set_item_action_id, call_game_map_transform_item, call_game_set_global_storage,
    call_lua_send_magic_effect, call_send_text_message,
};

fn parse_position(pos: Value) -> Result<(u16, u16, u8), mlua::Error> {
    match pos {
        Value::UserData(ud) => {
            if let Ok(p) = ud.borrow::<crate::userdata::position::PositionRef>() {
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

/// Override Lua `data/lib/core` helpers with native bindings. Call after
/// [`crate::actions::load_data_lib`] so these replace the pack Lua bodies.
pub fn register_data_lib_native(lua: &Lua) -> Result<(), mlua::Error> {
    let game = class_registry::register_class(lua, "Game", None)?;

    game.set(
        "isItemInPosition",
        lua.create_function(|_, (pos, item_type): (Value, u16)| {
            let (x, y, z) = parse_position(pos)?;
            current_ctx(|ctx| ctx.game_is_item_in_position(x, y, z, item_type))
                .ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?
                .map_err(mlua::Error::runtime)
        })?,
    )?;

    game.set(
        "removeItemInPosition",
        lua.create_function(|_, (pos, item_type): (Value, u16)| {
            let (x, y, z) = parse_position(pos)?;
            call_game_map_remove_item(x, y, z, item_type).map_err(mlua::Error::runtime)
        })?,
    )?;

    game.set(
        "transformItemInPosition",
        lua.create_function(|_, (pos, from_type, to_type): (Value, u16, u16)| {
            let (x, y, z) = parse_position(pos)?;
            call_game_map_transform_item(x, y, z, from_type, to_type).map_err(mlua::Error::runtime)
        })?,
    )?;

    game.set(
        "removeItemsInPosition",
        lua.create_function(|_, pos: Value| {
            let (x, y, z) = parse_position(pos)?;
            call_game_map_remove_movable_items(x, y, z).map_err(mlua::Error::runtime)?;
            Ok(())
        })?,
    )?;

    game.set(
        "setMapItemActionId",
        lua.create_function(|_, (pos, item_type, action_id): (Value, u16, u16)| {
            let (x, y, z) = parse_position(pos)?;
            call_game_map_set_item_action_id(x, y, z, item_type, action_id)
                .map_err(mlua::Error::runtime)
        })?,
    )?;

    game.set(
        "getStorageValue",
        lua.create_function(|_, key: u32| Ok(current_ctx(|ctx| ctx.get_global_storage(key)).flatten()))?,
    )?;

    game.set(
        "setStorageValue",
        lua.create_function(|_, (key, value): (u32, i32)| {
            call_game_set_global_storage(key, value).map_err(mlua::Error::runtime)?;
            Ok(())
        })?,
    )?;

    game.set(
        "sendMagicEffect",
        lua.create_function(|_, (pos, effect): (Value, u8)| {
            let (x, y, z) = parse_position(pos)?;
            call_lua_send_magic_effect(x, y, z, effect).map_err(mlua::Error::runtime)
        })?,
    )?;

    game.set(
        "broadcastMessage",
        lua.create_function(|_, (message, message_type): (String, Option<u8>)| {
            let msg_type = message_type.unwrap_or(21); // MESSAGE_STATUS_WARNING
            let ids = current_ctx(|ctx| ctx.online_player_ids()).unwrap_or_default();
            for id in ids {
                call_send_text_message(id, msg_type, message.clone())
                    .map_err(mlua::Error::runtime)?;
            }
            Ok(())
        })?,
    )?;

    game.set(
        "convertIpToString",
        lua.create_function(|_, ip: u32| Ok(format_ip_string(ip)))?,
    )?;

    game.set(
        "getReverseDirection",
        lua.create_function(|_, direction: u8| Ok(reverse_direction(direction)))?,
    )?;

    game.set(
        "getSkillType",
        lua.create_function(|_, weapon_type: u8| Ok(skill_type_for_weapon(weapon_type)))?,
    )?;

    let player = lua.globals().get::<mlua::Table>("Player")?;
    player.set(
        "getClosestFreePosition",
        lua.create_function(|lua, (self_val, position, extended): (Value, Value, Value)| {
            let creature_id = match self_val {
                Value::UserData(ud) => ud.borrow::<crate::context::CreatureRef>()?.0,
                _ => {
                    return Err(mlua::Error::runtime(
                        "Player.getClosestFreePosition: expected creature",
                    ));
                }
            };
            let (x, y, z) = parse_position(position)?;
            let max_radius = match extended {
                Value::Boolean(true) => 2,
                Value::Nil => 1,
                v => v
                    .as_integer()
                    .map(|n| n as i32)
                    .or_else(|| v.as_number().map(|n| n as i32))
                    .unwrap_or(1)
                    .max(0),
            };
            let (ox, oy, oz) = current_ctx(|ctx| {
                ctx.get_creature_closest_free_position(creature_id, x, y, z, max_radius, false)
            })
            .ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
            let ud = lua.create_userdata(crate::userdata::position::PositionRef {
                x: ox,
                y: oy,
                z: oz,
            })?;
            Ok(Value::UserData(ud))
        })?,
    )?;

    Ok(())
}

/// TFS `Game.getReverseDirection` — `data/lib/core/game.lua` (`position.h` directions).
fn reverse_direction(direction: u8) -> u8 {
    match direction {
        3 => 1, // WEST -> EAST
        1 => 3, // EAST -> WEST
        0 => 2, // NORTH -> SOUTH
        2 => 0, // SOUTH -> NORTH
        6 => 5, // NW -> SE
        7 => 4, // NE -> SW
        4 => 7, // SW -> NE
        5 => 6, // SE -> NW
        _ => 0, // default NORTH
    }
}

/// TFS `Game.getSkillType` — `data/lib/core/game.lua`.
fn skill_type_for_weapon(weapon_type: u8) -> u8 {
    match weapon_type {
        2 => 1,  // WEAPON_CLUB -> SKILL_CLUB
        1 => 2,  // WEAPON_SWORD -> SKILL_SWORD
        3 => 3,  // WEAPON_AXE -> SKILL_AXE
        5 => 4,  // WEAPON_DISTANCE -> SKILL_DISTANCE
        4 => 5,  // WEAPON_SHIELD -> SKILL_SHIELD
        _ => 0,  // SKILL_FIST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_direction_pairs() {
        assert_eq!(reverse_direction(3), 1);
        assert_eq!(reverse_direction(1), 3);
    }
}
