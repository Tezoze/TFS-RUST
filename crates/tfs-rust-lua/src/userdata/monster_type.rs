//! `MonsterType` Lua userdata — PC-3a Phase 3 residual / illusion spells.
//!
//! C++ reference: `luascript.cpp` `luaMonsterTypeCreate` / `luaMonsterTypeGetOutfit` /
//! `luaMonsterTypeIsIllusionable` — `monsters.h`.

use mlua::{UserData, UserDataMethods, Value};

use crate::context::CURRENT_CTX;

/// Name-backed monster type handle for Lua.
#[derive(Clone, Debug)]
pub struct MonsterTypeRef {
    pub name: String,
}

impl UserData for MonsterTypeRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // `monsterType:getOutfit()` — returns a table with `lookType` (and extras).
        // C++ `luaMonsterTypeGetOutfit` — used by `condition:setOutfit(monsterType:getOutfit())`.
        methods.add_method("getOutfit", |lua, this, ()| {
            let look_type = CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_monster_type_look_type(&this.name).unwrap_or(0))
            })?;
            let t = lua.create_table()?;
            t.set("lookType", look_type)?;
            // `ConditionBuilder::setOutfit` accepts a table with lookType.
            Ok(Value::Table(t))
        });

        methods.add_method("isIllusionable", |_, this, ()| {
            CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_monster_type_is_illusionable(&this.name))
            })
        });

        methods.add_method("name", |_, this, ()| Ok(this.name.clone()));
    }
}

/// Register `MonsterType(name)` — returns nil when unknown.
pub fn register_monster_type_constructor(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<MonsterTypeRef>(|_registry| {})?;
    let ctor = lua.create_function(|lua, name: String| {
        let exists = CURRENT_CTX.with(|c| {
            let Some(ptr) = *c.borrow() else {
                return true; // load-time — allow construct
            };
            if ptr.is_null() {
                return true;
            }
            let ctx = unsafe { &*ptr };
            ctx.monster_type_exists(&name)
        });
        if !exists {
            return Ok(Value::Nil);
        }
        let ud = lua.create_userdata(MonsterTypeRef { name })?;
        Ok(Value::UserData(ud))
    })?;
    lua.globals().set("MonsterType", ctor)?;
    Ok(())
}
