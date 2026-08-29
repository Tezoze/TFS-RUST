//! `Party` userdata — talkaction `!share` / pack `data/lib/core/party.lua`.
//!
//! C++ reference: `luascript.cpp` `luaPartyGetLeader` / `luaPartyIsSharedExperienceActive` /
//! `luaPartySetSharedExperience`.

use mlua::{UserData, UserDataMethods, Value};

use crate::context::{CURRENT_CTX, CreatureRef, LuaContext};
use crate::lua_mutation::{call_party_set_shared_experience, call_send_text_message};
use std::cell::RefCell;

/// Party handle — `Party::id`.
#[derive(Clone, Copy, Debug)]
pub struct PartyRef(pub u32);

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

pub fn register_party_metatable(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<PartyRef>(|_registry| {})
}

impl UserData for PartyRef {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        crate::class_registry::register_with_recording(registry, "Party");
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getLeader", |lua, this, ()| {
            with_ctx(|ctx| match ctx.get_party_leader(this.0) {
                Some(id) => {
                    let ud = lua.create_userdata(CreatureRef(id))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            })
        });

        methods.add_method("isSharedExperienceActive", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.party_shared_experience_active(this.0)))
        });

        methods.add_method("setSharedExperience", |_, this, enabled: bool| {
            call_party_set_shared_experience(this.0, enabled).map_err(mlua::Error::runtime)
        });

        methods.add_method("getMembers", |lua, this, ()| {
            let ids = with_ctx(|ctx| Ok(ctx.get_party_members(this.0)))?;
            let t = lua.create_table_with_capacity(ids.len(), 0)?;
            for (i, id) in ids.into_iter().enumerate() {
                let ud = lua.create_userdata(CreatureRef(id))?;
                t.set(i + 1, ud)?;
            }
            Ok(Value::Table(t))
        });

        // `Party.broadcastPartyLoot(text)` — `data/lib/core/party.lua`.
        methods.add_method("broadcastPartyLoot", |_, this, text: String| {
            const MESSAGE_INFO_DESCR: u8 = 0x16;
            with_ctx(|ctx| {
                if let Some(leader) = ctx.get_party_leader(this.0) {
                    call_send_text_message(leader, MESSAGE_INFO_DESCR, text.clone())
                        .map_err(mlua::Error::runtime)?;
                }
                for member in ctx.get_party_members(this.0) {
                    call_send_text_message(member, MESSAGE_INFO_DESCR, text.clone())
                        .map_err(mlua::Error::runtime)?;
                }
                Ok(())
            })
        });
    }
}
