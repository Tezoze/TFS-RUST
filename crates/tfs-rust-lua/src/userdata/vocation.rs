//! Vocation userdata for Lua (`player:getVocation()` → `:getId()` / `:getPromotion()`).
//!
//! C++ reference: `src/luascript.cpp` `luaPlayerGetVocation` / `luaVocationGetPromotion`
//! / `luaVocationGetDemotion`.

use mlua::{Lua, MetaMethod, UserData, UserDataMethods, Value};

use crate::context::{CURRENT_CTX, current_ctx};

/// Vocation handle — wraps the raw `players.vocation` id (`enums.h:297`
/// `VOCATION_NONE = 0`). Stored by value inside the userdata; no SlotMap
/// lookup needed (the id is the entire payload).
#[derive(Clone, Copy, Debug)]
pub struct VocationRef(pub i32);

pub fn register_vocation_metatable(lua: &Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<VocationRef>(|_registry| {})
}

/// `Vocation(id)` — TFS `luaVocationCreate`. Unknown id / no context → `nil`.
pub fn register_vocation_constructor(lua: &Lua) -> Result<(), mlua::Error> {
    let vocation_new = lua.create_function(|lua, id: i32| {
        let exists = current_ctx(|ctx| ctx.vocation_exists(id)).unwrap_or(false);
        if exists {
            let ud = lua.create_userdata(VocationRef(id))?;
            Ok(Value::UserData(ud))
        } else {
            Ok(Value::Nil)
        }
    })?;
    crate::class_registry::register_class(lua, "Vocation", Some(vocation_new))?;
    Ok(())
}

fn with_ctx<F, R>(f: F) -> Result<R, mlua::Error>
where
    F: FnOnce(&dyn crate::context::LuaContext) -> Result<R, mlua::Error>,
{
    CURRENT_CTX.with(|c| {
        let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
        if ptr.is_null() {
            return Err(mlua::Error::runtime("LuaContext not set"));
        }
        let ctx = unsafe { &*ptr };
        f(ctx)
    })
}

impl UserData for VocationRef {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        crate::class_registry::register_with_recording(registry, "Vocation");
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // `vocation:getId()` — `Vocation::id` (`player.h` / `vocation.h`).
        methods.add_method("getId", |_, this, ()| Ok(this.0));

        // `vocation:getPromotion()` — TFS `luaVocationGetPromotion`.
        methods.add_method("getPromotion", |lua, this, ()| {
            let promo = with_ctx(|ctx| Ok(ctx.get_vocation_promotion(this.0)))?;
            match promo {
                Some(id) => {
                    let ud = lua.create_userdata(VocationRef(id))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        // `vocation:getDemotion()` — TFS `luaVocationGetDemotion`.
        methods.add_method("getDemotion", |lua, this, ()| {
            let demo = with_ctx(|ctx| Ok(ctx.get_vocation_demotion(this.0)))?;
            match demo {
                Some(id) => {
                    let ud = lua.create_userdata(VocationRef(id))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        // Gap 7b — `__index` fallback so `vocation:getBase()` resolves
        // `function Vocation.getBase(self)` from `data/lib/core/vocation.lua`.
        methods.add_meta_method(MetaMethod::Index, |lua, _this, key: mlua::LuaString| {
            crate::class_registry::class_index_lookup(
                lua,
                crate::class_registry::VOCATION_INDEX_CHAIN,
                key,
            )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::with_lua_context;
    use tfs_rust_common::{
        ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptItemId, ScriptItemRef,
    };

    struct VocCtx;

    impl ScriptContext for VocCtx {
        fn get_creature(&self, _: ScriptCreatureId) -> Option<ScriptCreatureData> {
            None
        }
        fn get_item(&self, _: ScriptItemId) -> Option<ScriptItemRef> {
            None
        }
        fn get_config_string(&self, _: &str) -> Option<String> {
            None
        }
        fn get_vocation_promotion(&self, vocation_id: i32) -> Option<i32> {
            (vocation_id == 4).then_some(8)
        }
        fn get_vocation_demotion(&self, vocation_id: i32) -> Option<i32> {
            (vocation_id == 8).then_some(4)
        }
        fn vocation_exists(&self, vocation_id: i32) -> bool {
            matches!(vocation_id, 4 | 8)
        }
    }

    #[test]
    fn promotion_and_demotion_through_lua() {
        let runtime = crate::runtime::LuaRuntime::new().expect("runtime");
        let lua = &runtime.lua;
        with_lua_context(&VocCtx, || {
            let knight = lua.create_userdata(VocationRef(4)).expect("knight");
            lua.globals().set("voc", knight).unwrap();
            let pid: i32 = lua
                .load("return voc:getPromotion():getId()")
                .eval()
                .expect("promotion");
            assert_eq!(pid, 8);
            let none: mlua::Value = lua
                .load("return voc:getDemotion()")
                .eval()
                .expect("knight demotion");
            assert!(matches!(none, mlua::Value::Nil));

            let elite = lua.create_userdata(VocationRef(8)).expect("elite");
            lua.globals().set("voc", elite).unwrap();
            let did: i32 = lua
                .load("return voc:getDemotion():getId()")
                .eval()
                .expect("demotion");
            assert_eq!(did, 4);

            let via_ctor: i32 = lua
                .load("return Vocation(8):getId()")
                .eval()
                .expect("Vocation(8)");
            assert_eq!(via_ctor, 8);
            let missing: mlua::Value = lua.load("return Vocation(99)").eval().expect("missing");
            assert!(matches!(missing, mlua::Value::Nil));
        });
    }
}
