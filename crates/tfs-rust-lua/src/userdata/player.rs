//! Player / creature userdata bindings for Lua (`Player` / `Creature`).
//!
//! C++ reference: `src/luascript.cpp` — `Creature` / `Player` userdata methods.

use mlua::{UserData, UserDataMethods, Value};
use std::cell::RefCell;

use crate::context::{CreatureData, CreatureRef, ItemRef, CURRENT_CTX, LuaContext};
use crate::lua_mutation::{
    call_lua_add_item, call_lua_add_item_full, call_lua_feed, call_lua_get_depot_chest,
    call_lua_get_inbox, call_lua_remove_item,
};
use crate::userdata::container::ContainerRef;
use crate::userdata::vocation::VocationRef;

/// Register the Creature metatable in the Lua runtime.
pub fn register_creature_metatable(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<CreatureRef>(|_registry| {})
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

impl UserData for CreatureRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getId", |_, this, ()| Ok(this.0));

        methods.add_method("getName", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_creature(this.0)
                    .map(|c: CreatureData| c.name)
                    .ok_or_else(|| mlua::Error::runtime("creature not found"))
            })
        });

        methods.add_method("getGuid", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_creature(this.0)
                    .map(|c: CreatureData| c.guid)
                    .ok_or_else(|| mlua::Error::runtime("creature not found"))
            })
        });

        // `player:getLevel()` — `Player::getLevel` (`player.h`). LUA-2 read;
        // channel `onSpeak` gating (advertising.lua level-1 cancel).
        methods.add_method("getLevel", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_player_level(this.0)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        // `player:getAccountType()` — `accounts.type` tier (`enums.h:80-85`).
        // LUA-2 read; the backing field is plumbed from `accounts.type` at login.
        methods.add_method("getAccountType", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_player_account_type(this.0)
                    .map(i32::from)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        // `player:getVocation()` — returns a `Vocation` userdata whose `:getId()`
        // resolves to `players.vocation` (`player.h` `Vocation`). LUA-2 §1.4
        // option a: wraps the raw id; `getName`/`getPromotion` land later.
        methods.add_method("getVocation", |lua, this, ()| {
            let id_opt = with_ctx(|ctx| Ok(ctx.get_player_vocation_id(this.0)))?;
            match id_opt {
                Some(vid) => {
                    let ud = lua.create_userdata(VocationRef(vid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        // `player:hasFlag(flag)` — `Player::hasFlag` (`player.h`) over the
        // resolved `groups.xml` flag bits for `players.group_id`. LUA-2 read;
        // channel scripts test `PlayerFlag_CanTalkRedChannel` etc.
        methods.add_method("hasFlag", |_, this, flag: u64| {
            with_ctx(|ctx| Ok(ctx.player_has_flag(this.0, flag)))
        });

        methods.add_method("getSlotItem", |lua, this, slot: u8| {
            let id_opt = with_ctx(|ctx| Ok(ctx.get_player_slot_item_id(this.0, slot)))?;
            match id_opt {
                Some(iid) => {
                    let ud = lua.create_userdata(ItemRef(iid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("getCapacity", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_player_capacity(this.0)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        methods.add_method("getFreeCapacity", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_player_free_capacity(this.0)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        methods.add_method(
            "addItem",
            |lua,
             this,
             (item_type, count, can_drop, sub_type, slot): (
                mlua::Value,
                Option<u32>,
                Option<bool>,
                Option<i32>,
                Option<u8>,
            )| {
                let (item_type, count, sub_type) = match item_type {
                    mlua::Value::Integer(n) => (n as u16, count.unwrap_or(1), sub_type.unwrap_or(-1)),
                    mlua::Value::Number(n) => (n as u16, count.unwrap_or(1), sub_type.unwrap_or(-1)),
                    mlua::Value::String(s) => {
                        let name = s.to_str()?.to_string();
                        let ty = with_ctx(|ctx| {
                            ctx.get_item_type_id_by_name(&name)
                                .ok_or_else(|| mlua::Error::runtime("unknown item name"))
                        })?;
                        (ty, count.unwrap_or(1), sub_type.unwrap_or(-1))
                    }
                    _ => return Err(mlua::Error::runtime("invalid item type")),
                };
                let can_drop = can_drop.unwrap_or(true);
                let slot = slot.unwrap_or(0);

                if can_drop || sub_type != -1 || slot != 0 {
                    let id_opt = call_lua_add_item_full(
                        this.0,
                        item_type,
                        count,
                        sub_type,
                        can_drop,
                        slot,
                    )
                    .map_err(mlua::Error::runtime)?;
                    match id_opt {
                        Some(iid) => {
                            let ud = lua.create_userdata(ItemRef(iid))?;
                            Ok(Value::UserData(ud))
                        }
                        None => Ok(Value::Nil),
                    }
                } else {
                    call_lua_add_item(this.0, item_type, count.min(u16::MAX as u32) as u16)
                        .map_err(mlua::Error::runtime)?;
                    Ok(Value::Nil)
                }
            },
        );

        methods.add_method("getItemCount", |_, this, (item_type, sub_type): (u16, Option<i32>)| {
            let sub_type = sub_type.unwrap_or(-1);
            with_ctx(|ctx| {
                ctx.get_player_item_type_count(this.0, item_type, sub_type)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        methods.add_method(
            "removeItem",
            |_,
             this,
             (item_type, count, sub_type, ignore_equipped): (u16, u32, Option<i32>, Option<bool>)| {
                let sub_type = sub_type.unwrap_or(-1);
                let ignore_equipped = ignore_equipped.unwrap_or(false);
                call_lua_remove_item(this.0, item_type, count, sub_type, ignore_equipped)
                    .map_err(mlua::Error::runtime)
            },
        );

        methods.add_method(
            "getItemById",
            |lua,
             this,
             (item_type, deep_search, sub_type): (mlua::Value, bool, Option<i32>)| {
                let sub_type = sub_type.unwrap_or(-1);
                let item_id = match item_type {
                    mlua::Value::Integer(n) => n as u16,
                    mlua::Value::Number(n) => n as u16,
                    mlua::Value::String(s) => {
                        let name = s.to_str()?.to_string();
                        with_ctx(|ctx| {
                            ctx.get_item_type_id_by_name(&name)
                                .ok_or_else(|| mlua::Error::runtime("unknown item name"))
                        })?
                    }
                    _ => return Err(mlua::Error::runtime("invalid item id")),
                };
                let id_opt = with_ctx(|ctx| {
                    Ok(ctx.find_player_item_by_type(this.0, item_id, deep_search, sub_type))
                })?;
                match id_opt {
                    Some(iid) => {
                        let ud = lua.create_userdata(ItemRef(iid))?;
                        Ok(Value::UserData(ud))
                    }
                    None => Ok(Value::Nil),
                }
            },
        );

        methods.add_method(
            "getDepotChest",
            |lua, this, (depot_id, auto_create): (u32, Option<bool>)| {
                let auto_create = auto_create.unwrap_or(false);
                let id_opt = call_lua_get_depot_chest(this.0, depot_id, auto_create)
                    .map_err(mlua::Error::runtime)?;
                match id_opt {
                    Some(iid) => {
                        let ud = lua.create_userdata(ContainerRef(iid))?;
                        Ok(Value::UserData(ud))
                    }
                    None => Ok(Value::Boolean(false)),
                }
            },
        );

        methods.add_method("getInbox", |lua, this, ()| {
            let id_opt =
                call_lua_get_inbox(this.0).map_err(mlua::Error::runtime)?;
            match id_opt {
                Some(iid) => {
                    let ud = lua.create_userdata(ContainerRef(iid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Boolean(false)),
            }
        });

        methods.add_method("getContainerId", |_, this, container: mlua::AnyUserData| {
            let container_id = container
                .borrow::<ContainerRef>()
                .map(|c| c.0)
                .or_else(|_| container.borrow::<ItemRef>().map(|i| i.0))?;
            with_ctx(|ctx| {
                Ok(ctx
                    .get_player_container_id(this.0, container_id)
                    .map(i32::from)
                    .unwrap_or(-1))
            })
        });

        methods.add_method("getContainerById", |lua, this, client_cid: u8| {
            let id_opt = with_ctx(|ctx| Ok(ctx.get_player_container_by_cid(this.0, client_cid)))?;
            match id_opt {
                Some(iid) => {
                    let ud = lua.create_userdata(ContainerRef(iid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("getContainerIndex", |_, this, client_cid: u8| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_player_container_index(this.0, client_cid)
                    .map(i32::from)
                    .unwrap_or(-1))
            })
        });

        // 772 `player:feed(amount)` — refill `SKILL_FED` `Cycle` (`moveuse.cc:1846`).
        // C++ TFS uses `CONDITION_REGENERATION`; the 772 decompile uses a timer-skill.
        // We model the decompile: `food_remaining += amount`, capped at `MAX_FOOD`.
        methods.add_method("feed", |_, this, amount: u32| {
            call_lua_feed(this.0, amount).map_err(mlua::Error::runtime)?;
            Ok(())
        });

        // 772 `player:getFood()` — read `SKILL_FED` `Cycle` (`crskill.cc:220`).
        methods.add_method("getFood", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.get_player_food(this.0).unwrap_or(0)))
        });
    }
}

#[cfg(test)]
mod tests {
    //! LUA-2 unit tests: a fake `ScriptContext` backs the new player read
    //! methods (`getLevel` / `getAccountType` / `getVocation():getId()` /
    //! `hasFlag`), exercised through the real mlua userdata bindings via
    //! `with_lua_context`. This validates the read path end-to-end without
    //! spinning up a full `GameWorld`.

    use crate::context::{with_lua_context, CreatureRef};
    use mlua::Lua;
    use tfs_rust_common::{
        ScriptContainerData, ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptCylinder,
        ScriptItemData, ScriptItemId, ScriptItemRef,
    };

    /// Fake context returning a GM player (account type 6, level 42, vocation 1,
    /// `PlayerFlag_CanTalkRedChannel` set) for creature id `1`.
    struct GmPlayerCtx;

    const GM_CID: ScriptCreatureId = 1;
    const GM_LEVEL: i32 = 42;
    const GM_ACCOUNT_TYPE: u8 = 6; // ACCOUNT_TYPE_GOD
    const GM_VOCATION: i32 = 1; // Knight
    const GM_FLAG: u64 = 1 << 22; // PlayerFlag_CanTalkRedChannel

    impl ScriptContext for GmPlayerCtx {
        fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
            if id == GM_CID {
                Some(ScriptCreatureData {
                    name: "GM".into(),
                    guid: 100,
                })
            } else {
                None
            }
        }

        fn get_item(&self, _id: ScriptItemId) -> Option<ScriptItemRef> {
            None
        }

        fn get_config_string(&self, _: &str) -> Option<String> {
            None
        }

        fn get_player_level(&self, id: ScriptCreatureId) -> Option<i32> {
            (id == GM_CID).then_some(GM_LEVEL)
        }

        fn get_player_account_type(&self, id: ScriptCreatureId) -> Option<u8> {
            (id == GM_CID).then_some(GM_ACCOUNT_TYPE)
        }

        fn get_player_vocation_id(&self, id: ScriptCreatureId) -> Option<i32> {
            (id == GM_CID).then_some(GM_VOCATION)
        }

        fn player_has_flag(&self, id: ScriptCreatureId, flag: u64) -> bool {
            id == GM_CID && flag == GM_FLAG
        }

        // The remaining trait methods keep their default-`None`/`false`/empty
        // implementations; the test only exercises the four overrides above.
        // Suppress unused-import warnings for the types the defaults reference.
        fn get_player_slot_item_id(
            &self,
            _: ScriptCreatureId,
            _: u8,
        ) -> Option<ScriptItemId> {
            None
        }
        fn get_item_data(&self, _: ScriptItemId) -> Option<ScriptItemData> {
            None
        }
        fn get_container_data(&self, _: ScriptItemId) -> Option<ScriptContainerData> {
            None
        }
        fn get_item_parent(&self, _: ScriptItemId) -> Option<ScriptCylinder> {
            None
        }
        fn get_item_top_parent(&self, _: ScriptItemId) -> Option<ScriptCylinder> {
            None
        }
    }

    #[test]
    fn player_read_methods_return_gm_values_through_lua() {
        let lua = Lua::new();
        crate::userdata::register_creature_metatable(&lua).expect("creature metatable");
        crate::userdata::register_vocation_metatable(&lua).expect("vocation metatable");

        let ctx = GmPlayerCtx;
        with_lua_context(&ctx, || {
            let globals = lua.globals();
            // Bind a CreatureRef userdata to global `player`.
            let ud = lua.create_userdata(CreatureRef(GM_CID)).expect("userdata");
            globals.set("player", ud).expect("set player");

            // getLevel
            let level: i32 = lua
                .load("return player:getLevel()")
                .eval()
                .expect("getLevel");
            assert_eq!(level, GM_LEVEL);

            // getAccountType
            let atype: i32 = lua
                .load("return player:getAccountType()")
                .eval()
                .expect("getAccountType");
            assert_eq!(atype, i32::from(GM_ACCOUNT_TYPE));

            // getVocation():getId()
            let vid: i32 = lua
                .load("return player:getVocation():getId()")
                .eval()
                .expect("getVocation():getId()");
            assert_eq!(vid, GM_VOCATION);

            // hasFlag — set flag returns true, unset flag returns false.
            // LuaJIT (5.1) has no `<<` operator; pass the literal bit value
            // (1 << 22 == 4194304) directly.
            let has_red: bool = lua
                .load("return player:hasFlag(4194304)")
                .eval()
                .expect("hasFlag(1<<22)");
            assert!(has_red, "GM should have PlayerFlag_CanTalkRedChannel");
            let has_other: bool = lua
                .load("return player:hasFlag(1)")
                .eval()
                .expect("hasFlag(1<<0)");
            assert!(!has_other, "GM should not have flag bit 0");
        });
    }

    /// A no-op context (all defaults) confirms the new methods degrade
    /// gracefully — `getLevel`/`getAccountType` error (player not found),
    /// `getVocation` returns nil, `hasFlag` returns false — rather than panic.
    struct NullCtx;

    impl ScriptContext for NullCtx {
        fn get_creature(&self, _: ScriptCreatureId) -> Option<ScriptCreatureData> {
            None
        }
        fn get_item(&self, _: ScriptItemId) -> Option<ScriptItemRef> {
            None
        }
        fn get_config_string(&self, _: &str) -> Option<String> {
            None
        }
    }

    #[test]
    fn player_read_methods_default_none_does_not_panic() {
        let lua = Lua::new();
        crate::userdata::register_creature_metatable(&lua).expect("creature metatable");
        crate::userdata::register_vocation_metatable(&lua).expect("vocation metatable");

        let ctx = NullCtx;
        with_lua_context(&ctx, || {
            let globals = lua.globals();
            let ud = lua.create_userdata(CreatureRef(99)).expect("userdata");
            globals.set("player", ud).expect("set player");

            // hasFlag → false (default), no panic. LuaJIT has no `<<`; use the
            // literal bit value (1 << 22 == 4194304).
            let flag: bool = lua
                .load("return player:hasFlag(4194304)")
                .eval()
                .expect("hasFlag");
            assert!(!flag);

            // getVocation → nil (default), no panic.
            let v: mlua::Value = lua
                .load("return player:getVocation()")
                .eval()
                .expect("getVocation");
            assert!(matches!(v, mlua::Value::Nil), "expected nil vocation");
        });
    }
}
