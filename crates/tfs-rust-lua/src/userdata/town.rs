//! Town userdata for Lua (`Town` in TFS scripts).
//!
//! Domain: TFS `luascript.cpp` `luaTownCreate` / `luaTownGetId` /
//! `luaTownGetName` / `luaTownGetTemplePosition`. Town list is OTBM
//! `TownData` (`map.towns`).

use mlua::{UserData, UserDataMethods, Value};

use crate::context::{CURRENT_CTX, current_ctx};
use crate::userdata::position::PositionRef;

/// Town handle — wraps the OTBM town id (`Town::id`).
#[derive(Clone, Copy, Debug)]
pub struct TownRef(pub u32);

impl UserData for TownRef {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        crate::class_registry::register_with_recording(registry, "Town");
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // `town:getId()` — `luaTownGetId`.
        methods.add_method("getId", |_, this, ()| Ok(this.0));

        // `town:getName()` — `luaTownGetName`.
        methods.add_method("getName", |_, this, ()| {
            CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_town_by_id(this.0).map(|t| t.name))
            })
        });

        // `town:getTemplePosition()` — `luaTownGetTemplePosition`.
        methods.add_method("getTemplePosition", |lua, this, ()| {
            CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                match ctx.get_town_by_id(this.0) {
                    Some(town) => {
                        let ud = lua.create_userdata(PositionRef {
                            x: town.temple.x,
                            y: town.temple.y,
                            z: town.temple.z,
                        })?;
                        Ok(Value::UserData(ud))
                    }
                    None => Ok(Value::Nil),
                }
            })
        });
    }
}

/// `Town(id or name)` — C++ `luaTownCreate`. Number → id; string → name;
/// unknown / missing context → `nil`.
pub fn register_town_constructor(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<TownRef>(|_registry| {})?;
    let town_new = lua.create_function(|lua, arg: Value| {
        let town = match arg {
            Value::Integer(n) if n >= 0 => {
                current_ctx(|ctx| ctx.get_town_by_id(n as u32)).flatten()
            }
            Value::Number(n) if n >= 0.0 && n.fract() == 0.0 => {
                current_ctx(|ctx| ctx.get_town_by_id(n as u32)).flatten()
            }
            Value::String(s) => {
                let name = s.to_str()?.to_string();
                current_ctx(|ctx| ctx.get_town_by_name(&name)).flatten()
            }
            _ => None,
        };
        match town {
            Some(t) => {
                let ud = lua.create_userdata(TownRef(t.id))?;
                Ok(Value::UserData(ud))
            }
            None => Ok(Value::Nil),
        }
    })?;
    crate::class_registry::register_class(lua, "Town", Some(town_new))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{CreatureRef, with_lua_context};
    use tfs_rust_common::{
        Position, ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptItemId, ScriptItemRef,
        ScriptTownData,
    };

    struct TownCtx;

    impl ScriptContext for TownCtx {
        fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
            (id == 7).then_some(ScriptCreatureData {
                name: "Gm".into(),
                guid: 7,
            })
        }
        fn get_item(&self, _: ScriptItemId) -> Option<ScriptItemRef> {
            None
        }
        fn get_config_string(&self, _: &str) -> Option<String> {
            None
        }
        fn get_player_town_id(&self, creature_id: ScriptCreatureId) -> Option<i32> {
            (creature_id == 7).then_some(1)
        }
        fn get_town_by_id(&self, town_id: u32) -> Option<ScriptTownData> {
            (town_id == 1).then_some(ScriptTownData {
                id: 1,
                name: "Thais".into(),
                temple: Position::new(32369, 32241, 7),
            })
        }
        fn get_town_by_name(&self, name: &str) -> Option<ScriptTownData> {
            name.eq_ignore_ascii_case("Thais")
                .then_some(ScriptTownData {
                    id: 1,
                    name: "Thais".into(),
                    temple: Position::new(32369, 32241, 7),
                })
        }
    }

    #[test]
    fn town_constructor_resolves_id_and_name() {
        let runtime = crate::runtime::LuaRuntime::new().expect("runtime");
        let lua = &runtime.lua;
        with_lua_context(&TownCtx, || {
            let id: u32 = lua.load("return Town(1):getId()").eval().expect("Town(1)");
            assert_eq!(id, 1);
            let name: String = lua
                .load("return Town('thais'):getName()")
                .eval()
                .expect("Town('thais')");
            assert_eq!(name, "Thais");
            let missing: Value = lua.load("return Town('Nope')").eval().expect("missing");
            assert!(matches!(missing, Value::Nil));
            let numeric_string: Value = lua
                .load("return Town('1') or Town(tonumber('1'))")
                .eval()
                .expect("numeric string fallback");
            let ud = match numeric_string {
                Value::UserData(ud) => ud,
                other => panic!("expected userdata, got {other:?}"),
            };
            assert_eq!(ud.borrow::<TownRef>().expect("TownRef").0, 1);
        });
    }

    #[test]
    fn get_town_temple_and_player_get_town() {
        let runtime = crate::runtime::LuaRuntime::new().expect("runtime");
        let lua = &runtime.lua;
        let p = lua.create_userdata(CreatureRef(7)).expect("player");
        lua.globals().set("player", p).unwrap();
        with_lua_context(&TownCtx, || {
            let x: u16 = lua
                .load("return player:getTown():getTemplePosition().x")
                .eval()
                .expect("getTown temple");
            assert_eq!(x, 32369);
        });
    }
}
