//! `Monster` userdata — spawn-scoped inventory for the rarity `onSpawn` hook.
//!
//! Pack: TFS `luascript.cpp` `Monster` methods (`getType`, slot/container access).
//! Corpus: spawn-rolled inventory on the living monster (`TMonster::TMonster`,
//! `crnonpl.cc:2050`). Option (a): inventory accessors are live only while
//! `onSpawn` is running so later scripts cannot desync combat stats.

use mlua::{Lua, MetaMethod, UserData, UserDataMethods, Value};
use std::cell::{Cell, RefCell};

use crate::context::{CURRENT_CTX, CreatureData, LuaContext};
use crate::userdata::item::push_item_userdata;
use crate::userdata::monster_type::MonsterTypeRef;
use crate::userdata::position::PositionRef;

/// Typed monster handle. `token` must match the active spawn-inventory scope.
#[derive(Clone, Copy, Debug)]
pub struct MonsterRef {
    pub creature: crate::context::CreatureId,
    pub token: u64,
}

thread_local! {
    static SPAWN_INVENTORY_SCOPE: Cell<Option<(u64, u64)>> = const { Cell::new(None) };
    static NEXT_SPAWN_TOKEN: Cell<u64> = const { Cell::new(1) };
}

/// Run `f` with monster inventory accessors enabled for `creature`.
///
/// After return (including Lua errors) the scope is cleared so a stashed
/// [`MonsterRef`] cannot mutate a live monster.
pub fn with_monster_spawn_inventory_scope<R>(creature: u64, f: impl FnOnce(u64) -> R) -> R {
    let token = NEXT_SPAWN_TOKEN.with(|c| {
        let t = c.get();
        c.set(t.wrapping_add(1).max(1));
        t
    });
    SPAWN_INVENTORY_SCOPE.with(|c| c.set(Some((creature, token))));
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            SPAWN_INVENTORY_SCOPE.with(|c| c.set(None));
        }
    }
    let _guard = Guard;
    f(token)
}

pub fn monster_spawn_inventory_is_live(creature: u64, token: u64) -> bool {
    SPAWN_INVENTORY_SCOPE.with(|c| c.get() == Some((creature, token)))
}

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

fn require_spawn_inventory(this: &MonsterRef) -> Result<(), mlua::Error> {
    if monster_spawn_inventory_is_live(this.creature, this.token) {
        Ok(())
    } else {
        Err(mlua::Error::runtime(
            "monster inventory is only available during onSpawn",
        ))
    }
}

pub fn register_monster_metatable(lua: &Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<MonsterRef>(|_registry| {})
}

impl UserData for MonsterRef {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        crate::class_registry::register_with_recording(registry, "Monster");
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getId", |_, this, ()| Ok(this.creature));

        methods.add_method("getName", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_creature(this.creature)
                    .map(|c: CreatureData| c.name)
                    .ok_or_else(|| mlua::Error::runtime("monster not found"))
            })
        });

        methods.add_method("getPosition", |lua, this, ()| {
            with_ctx(|ctx| {
                let pos = ctx
                    .get_player_position(this.creature)
                    .ok_or_else(|| mlua::Error::runtime("monster not found"))?;
                lua.create_userdata(PositionRef {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                })
            })
        });

        methods.add_method("getType", |lua, this, ()| {
            let name = with_ctx(|ctx| Ok(ctx.get_creature_monster_type_name(this.creature)))?;
            match name {
                Some(name) => {
                    let ud = lua.create_userdata(MonsterTypeRef { name })?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("getSlotItem", |lua, this, slot: u8| {
            require_spawn_inventory(this)?;
            let id = with_ctx(|ctx| Ok(ctx.get_monster_slot_item_id(this.creature, slot)))?;
            match id {
                Some(iid) => push_item_userdata(lua, iid),
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("getBag", |lua, this, ()| {
            require_spawn_inventory(this)?;
            let id = with_ctx(|ctx| Ok(ctx.get_monster_bag_item_id(this.creature)))?;
            match id {
                Some(iid) => push_item_userdata(lua, iid),
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("getBodyItems", |lua, this, ()| {
            require_spawn_inventory(this)?;
            let ids = with_ctx(|ctx| Ok(ctx.get_monster_body_item_ids(this.creature)))?;
            let table = lua.create_table()?;
            for (i, id) in ids.into_iter().enumerate() {
                table.set(i + 1, push_item_userdata(lua, id)?)?;
            }
            Ok(Value::Table(table))
        });

        methods.add_meta_method(MetaMethod::Index, |lua, _this, key: mlua::LuaString| {
            crate::class_registry::class_index_lookup(
                lua,
                crate::class_registry::MONSTER_INDEX_CHAIN,
                key,
            )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::with_lua_context;
    use mlua::Lua;
    use tfs_rust_common::{
        ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptItemId, ScriptItemRef,
    };

    struct InvCtx {
        bag: u64,
    }

    impl ScriptContext for InvCtx {
        fn get_creature(&self, _: ScriptCreatureId) -> Option<ScriptCreatureData> {
            Some(ScriptCreatureData {
                name: "Rat".into(),
                guid: 0,
            })
        }
        fn get_item(&self, id: ScriptItemId) -> Option<ScriptItemRef> {
            Some(ScriptItemRef(id))
        }
        fn get_config_string(&self, _: &str) -> Option<String> {
            None
        }
        fn get_monster_bag_item_id(&self, _: ScriptCreatureId) -> Option<ScriptItemId> {
            Some(self.bag)
        }
        fn is_registered_container(&self, id: ScriptItemId) -> bool {
            id == self.bag
        }
        fn get_creature_monster_type_name(&self, _: ScriptCreatureId) -> Option<String> {
            Some("rat".into())
        }
    }

    fn setup(lua: &Lua) {
        crate::class_registry::register_engine_class_tables(lua).expect("classes");
        register_monster_metatable(lua).expect("monster mt");
        crate::userdata::register_item_metatable(lua).expect("item mt");
        crate::userdata::register_container_metatable(lua).expect("container mt");
        crate::userdata::register_monster_type_constructor(lua).expect("mtype");
        crate::userdata::register_position_metatable(lua).expect("pos");
    }

    #[test]
    fn get_bag_works_inside_spawn_scope() {
        let lua = Lua::new();
        setup(&lua);
        let ctx = InvCtx { bag: 99 };
        with_lua_context(&ctx, || {
            with_monster_spawn_inventory_scope(7, |token| {
                let ud = lua
                    .create_userdata(MonsterRef { creature: 7, token })
                    .expect("ud");
                lua.globals().set("m", ud).expect("set");
                let ok: bool = lua.load("return m:getBag() ~= nil").eval().expect("getBag");
                assert!(ok);
            });
        });
    }

    #[test]
    fn stashed_handle_cannot_read_inventory_after_on_spawn() {
        let lua = Lua::new();
        setup(&lua);
        let ctx = InvCtx { bag: 99 };
        with_lua_context(&ctx, || {
            with_monster_spawn_inventory_scope(7, |token| {
                let ud = lua
                    .create_userdata(MonsterRef { creature: 7, token })
                    .expect("ud");
                lua.globals().set("m", ud).expect("set");
                token
            });
            let result = lua.load("return m:getBag()").exec();
            assert!(result.is_err(), "stashed handle must error");
        });
    }
}
