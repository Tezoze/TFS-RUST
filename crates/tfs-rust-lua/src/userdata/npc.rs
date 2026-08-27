//! `Npc` userdata — typed creature id for NPC Lua callbacks (NPC-7).
//!
//! Domain: TFS-style `Npc` methods (`luascript.cpp` / `NpcScriptInterface`).
//! Callbacks receive [`NpcRef`]; no implicit global current-NPC state.

use mlua::{UserData, UserDataMethods};
use std::cell::RefCell;

use crate::context::{CURRENT_CTX, CreatureData, CreatureRef, LuaContext};
use crate::lua_mutation::{call_lua_npc_say, call_lua_npc_set_focus, call_npc_set_master_pos};
use crate::userdata::position::PositionRef;

/// Typed NPC handle passed into custom / lifecycle callbacks.
#[derive(Clone, Copy, Debug)]
pub struct NpcRef(pub crate::context::CreatureId);

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

/// Register `NpcRef` userdata type (methods attached via `UserData`).
pub fn register_npc_metatable(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<NpcRef>(|_registry| {})
}

impl UserData for NpcRef {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        crate::class_registry::register_with_recording(registry, "Npc");
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getId", |_, this, ()| Ok(this.0));

        methods.add_method("getName", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_creature(this.0)
                    .map(|c: CreatureData| c.name)
                    .ok_or_else(|| mlua::Error::runtime("npc not found"))
            })
        });

        methods.add_method("getParameter", |_, this, key: String| {
            with_ctx(|ctx| Ok(ctx.get_npc_parameter(this.0, &key)))
        });

        methods.add_method("getPosition", |lua, this, ()| {
            with_ctx(|ctx| {
                let pos = ctx
                    .get_player_position(this.0)
                    .ok_or_else(|| mlua::Error::runtime("npc not found"))?;
                lua.create_userdata(PositionRef {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                })
            })
        });

        methods.add_method("isInTalkRange", |_, this, player: mlua::AnyUserData| {
            let player_id = if let Ok(p) = player.borrow::<CreatureRef>() {
                p.0
            } else {
                return Err(mlua::Error::runtime(
                    "isInTalkRange: expected Player/Creature userdata",
                ));
            };
            with_ctx(|ctx| Ok(ctx.npc_is_in_talk_range(this.0, player_id)))
        });

        methods.add_method_mut(
            "setMasterPos",
            |_, this, (pos, radius): (mlua::Value, Option<u16>)| {
                let (x, y, z) = match pos {
                    mlua::Value::UserData(ud) => {
                        let p = ud.borrow::<PositionRef>()?;
                        (p.x, p.y, p.z)
                    }
                    mlua::Value::Table(t) => {
                        let x: i64 = t.get("x").or_else(|_| t.get(1))?;
                        let y: i64 = t.get("y").or_else(|_| t.get(2))?;
                        let z: i64 = t.get("z").or_else(|_| t.get(3))?;
                        (x as u16, y as u16, z as u8)
                    }
                    _ => {
                        return Err(mlua::Error::runtime("setMasterPos: expected Position"));
                    }
                };
                call_npc_set_master_pos(this.0, x, y, z, radius).map_err(mlua::Error::runtime)
            },
        );

        methods.add_method_mut("say", |_, this, text: String| {
            call_lua_npc_say(this.0, &text).map_err(mlua::Error::runtime)?;
            Ok(())
        });

        methods.add_method_mut("setFocus", |_, this, player: Option<mlua::AnyUserData>| {
            let player_id = match player {
                None => None,
                Some(ud) => {
                    let cref = ud.borrow::<CreatureRef>().map_err(|_| {
                        mlua::Error::runtime("setFocus: expected Player userdata or nil")
                    })?;
                    Some(cref.0)
                }
            };
            call_lua_npc_set_focus(this.0, player_id).map_err(mlua::Error::runtime)?;
            Ok(())
        });

        methods.add_method("getFocus", |lua, this, ()| {
            with_ctx(|ctx| match ctx.get_npc_focus(this.0) {
                Some(id) => {
                    let ud = lua.create_userdata(CreatureRef(id))?;
                    Ok(mlua::Value::UserData(ud))
                }
                None => Ok(mlua::Value::Nil),
            })
        });
    }
}
