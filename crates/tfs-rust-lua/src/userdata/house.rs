//! House userdata for Lua (`House` in TFS scripts).
//!
//! Pack surface: TFS `luascript.cpp` `luaHouseCreate` / house methods.
//! `startTrade` omitted until player trade; `!buyhouse` uses `setOwnerGuid`.

use mlua::{MetaMethod, UserData, UserDataMethods, Value};

use crate::context::{CreatureRef, ItemRef, current_ctx};
use crate::lua_mutation::{
    call_house_kick_player, call_house_save, call_house_set_access_list, call_house_set_owner,
};
use crate::userdata::position::PositionRef;
use crate::userdata::tile::TileRef;
use crate::userdata::town::TownRef;

/// House handle — wraps the house id (`House::id`).
#[derive(Clone, Copy, Debug)]
pub struct HouseRef(pub u32);

impl UserData for HouseRef {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        crate::class_registry::register_with_recording(registry, "House");
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getId", |_, this, ()| Ok(this.0));

        methods.add_method("getName", |_, this, ()| {
            Ok(current_ctx(|ctx| ctx.get_house(this.0).map(|h| h.name)).flatten())
        });

        methods.add_method("getTownId", |_, this, ()| {
            Ok(current_ctx(|ctx| ctx.get_house(this.0).map(|h| h.town_id)).flatten())
        });

        methods.add_method("getTown", |lua, this, ()| {
            match current_ctx(|ctx| ctx.get_house(this.0)).flatten() {
                Some(h) => {
                    let ud = lua.create_userdata(TownRef(h.town_id))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("getRent", |_, this, ()| {
            Ok(current_ctx(|ctx| ctx.get_house(this.0).map(|h| h.rent)).flatten())
        });

        // `house:isGuildHall()` — TFS `luaHouseIsGuildHall`. No guildhall XML yet → false.
        methods.add_method("isGuildHall", |_, _this, ()| Ok(false));

        methods.add_method("getOwnerGuid", |_, this, ()| {
            Ok(current_ctx(|ctx| ctx.get_house(this.0).map(|h| h.owner_guid)).flatten())
        });

        methods.add_method("setOwnerGuid", |_, this, guid: u32| {
            call_house_set_owner(this.0, guid).map_err(mlua::Error::runtime)?;
            Ok(())
        });

        methods.add_method("getExitPosition", |lua, this, ()| {
            match current_ctx(|ctx| ctx.get_house(this.0)).flatten() {
                Some(h) => {
                    let ud = lua.create_userdata(PositionRef {
                        x: h.exit.x,
                        y: h.exit.y,
                        z: h.exit.z,
                    })?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("getTileCount", |_, this, ()| {
            Ok(current_ctx(|ctx| ctx.get_house(this.0).map(|h| h.tiles.len() as u32)).flatten())
        });

        methods.add_method("getTiles", |lua, this, ()| {
            let tiles = current_ctx(|ctx| ctx.get_house(this.0).map(|h| h.tiles)).flatten();
            let table = lua.create_table()?;
            if let Some(tiles) = tiles {
                for (i, pos) in tiles.into_iter().enumerate() {
                    let ud = lua.create_userdata(TileRef {
                        x: pos.x,
                        y: pos.y,
                        z: pos.z,
                    })?;
                    table.set(i + 1, ud)?;
                }
            }
            Ok(table)
        });

        methods.add_method("getDoors", |lua, this, ()| {
            let ids = current_ctx(|ctx| ctx.get_house(this.0).map(|h| h.door_item_ids)).flatten();
            let table = lua.create_table()?;
            if let Some(ids) = ids {
                for (i, id) in ids.into_iter().enumerate() {
                    let ud = lua.create_userdata(ItemRef(id))?;
                    table.set(i + 1, ud)?;
                }
            }
            Ok(table)
        });

        methods.add_method("getBeds", |lua, this, ()| {
            let ids = current_ctx(|ctx| ctx.get_house(this.0).map(|h| h.bed_item_ids)).flatten();
            let table = lua.create_table()?;
            if let Some(ids) = ids {
                for (i, id) in ids.into_iter().enumerate() {
                    let ud = lua.create_userdata(ItemRef(id))?;
                    table.set(i + 1, ud)?;
                }
            }
            Ok(table)
        });

        methods.add_method("getPlayers", |lua, this, ()| {
            let ids = current_ctx(|ctx| ctx.get_house(this.0).map(|h| h.player_ids)).flatten();
            let table = lua.create_table()?;
            if let Some(ids) = ids {
                for (i, id) in ids.into_iter().enumerate() {
                    let ud = lua.create_userdata(CreatureRef(id))?;
                    table.set(i + 1, ud)?;
                }
            }
            Ok(table)
        });

        methods.add_method("getAccessList", |_, this, list_id: u32| {
            Ok(current_ctx(|ctx| ctx.house_access_list(this.0, list_id)).flatten())
        });

        methods.add_method(
            "setAccessList",
            |_, this, (list_id, text): (u32, String)| {
                call_house_set_access_list(this.0, list_id, text).map_err(mlua::Error::runtime)?;
                Ok(())
            },
        );

        methods.add_method("getDoorIdByPosition", |_, this, pos: Value| {
            let (x, y, z) = match pos {
                Value::UserData(ud) => {
                    if let Ok(p) = ud.borrow::<PositionRef>() {
                        (p.x, p.y, p.z)
                    } else {
                        return Ok(None);
                    }
                }
                _ => return Ok(None),
            };
            Ok(current_ctx(|ctx| ctx.house_door_id_at(this.0, x, y, z)).flatten())
        });

        methods.add_method(
            "canEditAccessList",
            |_, this, (list_id, player): (u32, Value)| {
                let creature_id = match player {
                    Value::UserData(ud) => ud.borrow::<CreatureRef>()?.0,
                    _ => return Ok(false),
                };
                Ok(
                    current_ctx(|ctx| ctx.house_can_edit_access_list(this.0, list_id, creature_id))
                        .unwrap_or(false),
                )
            },
        );

        methods.add_method("kickPlayer", |_, this, (kicker, target): (Value, Value)| {
            let kicker_id = match kicker {
                Value::UserData(ud) => ud.borrow::<CreatureRef>()?.0,
                _ => return Ok(false),
            };
            let target_id = match target {
                Value::UserData(ud) => ud.borrow::<CreatureRef>()?.0,
                _ => return Ok(false),
            };
            call_house_kick_player(this.0, kicker_id, target_id).map_err(mlua::Error::runtime)
        });

        methods.add_method("save", |_, this, ()| {
            call_house_save(this.0).map_err(mlua::Error::runtime)?;
            Ok(true)
        });

        methods.add_meta_method(MetaMethod::Index, |lua, _this, key: mlua::LuaString| {
            crate::class_registry::class_index_lookup(
                lua,
                crate::class_registry::HOUSE_INDEX_CHAIN,
                key,
            )
        });
    }
}

/// `House(id)` — C++ `luaHouseCreate`. Unknown id → `nil`.
pub fn register_house_constructor(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<HouseRef>(|_registry| {})?;
    let house_new = lua.create_function(|lua, arg: Value| {
        let id = match arg {
            Value::Integer(n) if n > 0 => n as u32,
            Value::Number(n) if n > 0.0 && n.fract() == 0.0 => n as u32,
            _ => return Ok(Value::Nil),
        };
        let exists = current_ctx(|ctx| ctx.get_house(id)).flatten().is_some();
        if !exists {
            return Ok(Value::Nil);
        }
        let ud = lua.create_userdata(HouseRef(id))?;
        Ok(Value::UserData(ud))
    })?;
    crate::class_registry::register_class(lua, "House", Some(house_new))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::with_lua_context;
    use mlua::Lua;
    use tfs_rust_common::{
        Position, ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptHouseData,
        ScriptItemId, ScriptItemRef,
    };

    struct HouseCtx;

    impl ScriptContext for HouseCtx {
        fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
            (id == 7).then_some(ScriptCreatureData {
                name: "Owner".into(),
                guid: 10,
            })
        }
        fn get_item(&self, _: ScriptItemId) -> Option<ScriptItemRef> {
            None
        }
        fn get_config_string(&self, _: &str) -> Option<String> {
            None
        }
        fn get_house(&self, house_id: u32) -> Option<ScriptHouseData> {
            (house_id == 1).then_some(ScriptHouseData {
                id: 1,
                name: "Spiritkeep".into(),
                town_id: 1,
                rent: 19210,
                owner_guid: 10,
                exit: Position::new(50, 50, 7),
                tiles: vec![Position::new(50, 50, 7)],
                door_item_ids: vec![],
                bed_item_ids: vec![],
                player_ids: vec![7],
            })
        }
        fn house_can_edit_access_list(
            &self,
            house_id: u32,
            list_id: u32,
            creature_id: ScriptCreatureId,
        ) -> bool {
            house_id == 1 && list_id == 0x100 && creature_id == 7
        }
    }

    #[test]
    fn house_ctor_and_reads() {
        let lua = Lua::new();
        register_house_constructor(&lua).expect("house ctor");
        let ctx = HouseCtx;
        with_lua_context(&ctx, || {
            let name: String = lua
                .load("local h = House(1); return h:getName()")
                .eval()
                .unwrap();
            assert_eq!(name, "Spiritkeep");
            let rent: u32 = lua.load("return House(1):getRent()").eval().unwrap();
            assert_eq!(rent, 19210);
            let missing: bool = lua.load("return House(99) == nil").eval().unwrap();
            assert!(missing);
        });
    }
}
