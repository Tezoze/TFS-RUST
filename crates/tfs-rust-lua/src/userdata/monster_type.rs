//! `MonsterType` Lua userdata — PC-3a Phase 3 residual / illusion spells.
//!
//! C++ reference: `luascript.cpp` `luaMonsterTypeCreate` / `luaMonsterTypeGetOutfit` /
//! `luaMonsterTypeIsIllusionable` — `monsters.h`.
//! Gap 7c: constructor is a class table (`register_class`); no `__index` chain
//! (the only `mType:register` call site is a `#`-prefixed example).

use mlua::{UserData, UserDataMethods, Value};

use crate::context::CURRENT_CTX;

/// Name-backed monster type handle for Lua.
#[derive(Clone, Debug)]
pub struct MonsterTypeRef {
    pub name: String,
}

impl UserData for MonsterTypeRef {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        crate::class_registry::register_with_recording(registry, "MonsterType");
    }

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

        methods.add_method("isSummonable", |_, this, ()| {
            CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_monster_type_is_summonable(&this.name))
            })
        });

        methods.add_method("isConvinceable", |_, this, ()| {
            CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_monster_type_is_convinceable(&this.name))
            })
        });

        methods.add_method("getManaCost", |_, this, ()| {
            CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_monster_type_mana_cost(&this.name))
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
    // Class table + `__call` so `data/scripts/lib/register_monster_type.lua`
    // can assign `MonsterType.register = function(self, mask)`. No userdata
    // `__index` chain (Gap 7b): `mType:register(...)` has no in-pack caller
    // (`data/monster/lua/#example.lua` was a TFS sketch and was deleted).
    // C++ `registerClass("MonsterType")` — `luascript.cpp`. Gap 7c.
    crate::class_registry::register_class(lua, "MonsterType", Some(ctor))?;
    Ok(())
}
