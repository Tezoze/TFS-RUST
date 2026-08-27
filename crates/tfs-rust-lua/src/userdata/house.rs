//! House userdata for Lua (`House` in TFS scripts).
//!
//! Pack surface: TFS `luascript.cpp` `luaHouseCreate` / house methods.
//! `startTrade` (`luaHouseStartTrade`) returns `RETURNVALUE_*` without native P2P trade.

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

        // TFS `luaHouseIsGuildHall` / `House::guildHall`.
        methods.add_method("isGuildHall", |_, this, ()| {
            Ok(current_ctx(|ctx| ctx.get_house(this.0).map(|h| h.is_guild_hall))
                .flatten()
                .unwrap_or(false))
        });

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

        // TFS `luaHouseGetTileCount` is `houseTiles.size()`; XML `size` until OTBM attach.
        methods.add_method("getTileCount", |_, this, ()| {
            Ok(current_ctx(|ctx| {
                ctx.get_house(this.0).map(|h| {
                    if !h.tiles.is_empty() {
                        h.tiles.len() as u32
                    } else {
                        h.size
                    }
                })
            })
            .flatten())
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

        // TFS `luaHouseStartTrade`. Player-to-player trade is not native — never
        // creates a transfer item. Always returns `RETURNVALUE_*` (never nil) so
        // `!sellhouse` can `sendCancelMessage(returnValue)`.
        methods.add_method("startTrade", |_, this, (player, partner): (Value, Value)| {
            Ok(house_start_trade(this.0, &player, &partner))
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

fn creature_id_from_value(v: &Value) -> Option<tfs_rust_common::ScriptCreatureId> {
    match v {
        Value::UserData(ud) => ud.borrow::<CreatureRef>().ok().map(|c| c.0),
        _ => None,
    }
}

/// TFS `Position::areInRange<2, 2, 0>` (`luaHouseStartTrade`).
fn positions_in_trade_range(a: tfs_rust_common::Position, b: tfs_rust_common::Position) -> bool {
    a.z == b.z && a.x.abs_diff(b.x) <= 2 && a.y.abs_diff(b.y) <= 2
}

/// TFS `luaHouseStartTrade` check order. Trade is not native → never starts a trade.
fn house_start_trade(house_id: u32, player: &Value, partner: &Value) -> i32 {
    const NOT_POSSIBLE: i32 = 1; // RETURNVALUE_NOTPOSSIBLE
    const FAR_AWAY: i32 = 63; // RETURNVALUE_TRADEPLAYERFARAWAY
    const YOU_DONT_OWN: i32 = 64; // RETURNVALUE_YOUDONTOWNTHISHOUSE
    const ALREADY_OWNS: i32 = 65; // RETURNVALUE_TRADEPLAYERALREADYOWNSAHOUSE
    const HIGHEST_BIDDER: i32 = 66; // RETURNVALUE_TRADEPLAYERHIGHESTBIDDER
    const CANNOT_TRADE: i32 = 67; // RETURNVALUE_YOUCANNOTTRADETHISHOUSE

    let Some(player_id) = creature_id_from_value(player) else {
        return NOT_POSSIBLE;
    };
    let Some(partner_id) = creature_id_from_value(partner) else {
        return NOT_POSSIBLE;
    };

    current_ctx(|ctx| {
        let Some(house) = ctx.get_house(house_id) else {
            return NOT_POSSIBLE;
        };
        let Some(player_c) = ctx.get_creature(player_id) else {
            return NOT_POSSIBLE;
        };
        let Some(partner_c) = ctx.get_creature(partner_id) else {
            return NOT_POSSIBLE;
        };
        let (Some(ppos), Some(tpos)) = (
            ctx.get_player_position(player_id),
            ctx.get_player_position(partner_id),
        ) else {
            return FAR_AWAY;
        };
        if !positions_in_trade_range(ppos, tpos) {
            return FAR_AWAY;
        }
        if house.owner_guid != player_c.guid {
            return YOU_DONT_OWN;
        }
        if ctx.house_id_for_owner_guid(partner_c.guid).is_some() {
            return ALREADY_OWNS;
        }
        let partner_guid = partner_c.guid;
        if ctx.list_house_ids().into_iter().any(|id| {
            ctx.get_house(id)
                .is_some_and(|h| h.highest_bidder == partner_guid)
        }) {
            return HIGHEST_BIDDER;
        }
        CANNOT_TRADE
    })
    .unwrap_or(NOT_POSSIBLE)
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

    struct HouseCtx {
        partner_pos: Position,
        partner_owns: bool,
        house2_highest_bidder: u32,
    }

    impl HouseCtx {
        fn standard() -> Self {
            Self {
                partner_pos: Position::new(51, 50, 7),
                partner_owns: false,
                house2_highest_bidder: 0,
            }
        }

        fn house1() -> ScriptHouseData {
            ScriptHouseData {
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
                is_guild_hall: false,
                size: 687,
                highest_bidder: 0,
            }
        }
    }

    impl ScriptContext for HouseCtx {
        fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
            match id {
                7 => Some(ScriptCreatureData {
                    name: "Owner".into(),
                    guid: 10,
                }),
                8 => Some(ScriptCreatureData {
                    name: "Partner".into(),
                    guid: 20,
                }),
                _ => None,
            }
        }
        fn get_item(&self, _: ScriptItemId) -> Option<ScriptItemRef> {
            None
        }
        fn get_config_string(&self, _: &str) -> Option<String> {
            None
        }
        fn get_player_position(&self, creature_id: ScriptCreatureId) -> Option<Position> {
            match creature_id {
                7 => Some(Position::new(50, 50, 7)),
                8 => Some(self.partner_pos),
                _ => None,
            }
        }
        fn get_house(&self, house_id: u32) -> Option<ScriptHouseData> {
            match house_id {
                1 => Some(Self::house1()),
                2 => Some(ScriptHouseData {
                    id: 2,
                    name: "Other".into(),
                    town_id: 1,
                    rent: 0,
                    owner_guid: 0,
                    exit: Position::new(60, 60, 7),
                    tiles: vec![],
                    door_item_ids: vec![],
                    bed_item_ids: vec![],
                    player_ids: vec![],
                    is_guild_hall: false,
                    size: 10,
                    highest_bidder: self.house2_highest_bidder,
                }),
                3 => Some(ScriptHouseData {
                    id: 3,
                    name: "Warriors Guildhall".into(),
                    town_id: 1,
                    rent: 0,
                    owner_guid: 0,
                    exit: Position::new(0, 0, 7),
                    tiles: vec![],
                    door_item_ids: vec![],
                    bed_item_ids: vec![],
                    player_ids: vec![],
                    is_guild_hall: true,
                    size: 583,
                    highest_bidder: 0,
                }),
                _ => None,
            }
        }
        fn list_house_ids(&self) -> Vec<u32> {
            vec![1, 2]
        }
        fn house_id_for_owner_guid(&self, owner_guid: u32) -> Option<u32> {
            match owner_guid {
                10 => Some(1),
                20 if self.partner_owns => Some(2),
                _ => None,
            }
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

    fn eval_start_trade(ctx: &HouseCtx, swap: bool) -> i32 {
        let lua = Lua::new();
        register_house_constructor(&lua).expect("house ctor");
        crate::userdata::register_creature_metatable(&lua).expect("creature");
        let player = lua.create_userdata(CreatureRef(7)).expect("player");
        let partner = lua.create_userdata(CreatureRef(8)).expect("partner");
        lua.globals().set("player", player).unwrap();
        lua.globals().set("partner", partner).unwrap();
        with_lua_context(ctx, || {
            let expr = if swap {
                "return House(1):startTrade(partner, player)"
            } else {
                "return House(1):startTrade(player, partner)"
            };
            lua.load(expr).eval().unwrap()
        })
    }

    #[test]
    fn house_ctor_and_reads() {
        let lua = Lua::new();
        register_house_constructor(&lua).expect("house ctor");
        let ctx = HouseCtx::standard();
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
            let guild: bool = lua.load("return House(1):isGuildHall()").eval().unwrap();
            assert!(!guild);
            let guild_hall: bool = lua.load("return House(3):isGuildHall()").eval().unwrap();
            assert!(guild_hall);
            let tiles: u32 = lua.load("return House(1):getTileCount()").eval().unwrap();
            assert_eq!(tiles, 1);
            let xml_size: u32 = lua.load("return House(2):getTileCount()").eval().unwrap();
            assert_eq!(xml_size, 10);
        });
    }

    #[test]
    fn start_trade_owner_in_range_cannot_trade() {
        assert_eq!(eval_start_trade(&HouseCtx::standard(), false), 67);
    }

    #[test]
    fn start_trade_not_owner() {
        assert_eq!(eval_start_trade(&HouseCtx::standard(), true), 64);
    }

    #[test]
    fn start_trade_partner_far_away() {
        let mut ctx = HouseCtx::standard();
        ctx.partner_pos = Position::new(51, 50, 8);
        assert_eq!(eval_start_trade(&ctx, false), 63);
        ctx.partner_pos = Position::new(60, 50, 7);
        assert_eq!(eval_start_trade(&ctx, false), 63);
    }

    #[test]
    fn start_trade_partner_already_owns() {
        let mut ctx = HouseCtx::standard();
        ctx.partner_owns = true;
        assert_eq!(eval_start_trade(&ctx, false), 65);
    }

    #[test]
    fn start_trade_partner_highest_bidder() {
        let mut ctx = HouseCtx::standard();
        ctx.house2_highest_bidder = 20;
        assert_eq!(eval_start_trade(&ctx, false), 66);
    }
}
