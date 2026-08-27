//! Lua globals for native tool helpers (`onUseQuest`, `destroyItem`, `onUse*`,
//! `checkScarabTile`). Implementations live in `tfs-rust-core::tool_use`.

use mlua::{Lua, Table, Value};

use crate::context::{CreatureRef, ItemRef};
use crate::lua_mutation::{
    QuestChestSpec, QuestRewardSpec, ToolUseKind, ToolUseRequest, call_lua_tool_use,
};
use crate::userdata::container::ContainerRef;
use crate::userdata::position::PositionRef;

pub fn register_tool_use_globals(lua: &Lua) -> Result<(), mlua::Error> {
    let g = lua.globals();
    g.set(
        "destroyItem",
        lua.create_function(|_, (player, target, to): (Value, Value, Value)| {
            call_bool(parse_tool(
                ToolUseKind::DestroyItem,
                player,
                Value::Nil,
                Value::Nil,
                target,
                to,
                None,
            )?)
        })?,
    )?;
    g.set(
        "onUseMachete",
        lua.create_function(
            |_, (player, item, from, target, to): (Value, Value, Value, Value, Value)| {
                call_bool(parse_tool(
                    ToolUseKind::Machete,
                    player,
                    item,
                    from,
                    target,
                    to,
                    None,
                )?)
            },
        )?,
    )?;
    g.set(
        "onUsePick",
        lua.create_function(
            |_, (player, item, from, target, to): (Value, Value, Value, Value, Value)| {
                call_bool(parse_tool(
                    ToolUseKind::Pick,
                    player,
                    item,
                    from,
                    target,
                    to,
                    None,
                )?)
            },
        )?,
    )?;
    g.set(
        "onUseKnife",
        lua.create_function(
            |_, (player, item, from, target, to): (Value, Value, Value, Value, Value)| {
                call_bool(parse_tool(
                    ToolUseKind::Knife,
                    player,
                    item,
                    from,
                    target,
                    to,
                    None,
                )?)
            },
        )?,
    )?;
    g.set(
        "onUseRope",
        lua.create_function(
            |_, (player, item, from, target, to): (Value, Value, Value, Value, Value)| {
                call_bool(parse_tool(
                    ToolUseKind::Rope,
                    player,
                    item,
                    from,
                    target,
                    to,
                    None,
                )?)
            },
        )?,
    )?;
    g.set(
        "onUseShovel",
        lua.create_function(
            |_, (player, item, from, target, to): (Value, Value, Value, Value, Value)| {
                call_bool(parse_tool(
                    ToolUseKind::Shovel,
                    player,
                    item,
                    from,
                    target,
                    to,
                    None,
                )?)
            },
        )?,
    )?;
    g.set(
        "onUseScythe",
        lua.create_function(
            |_, (player, item, from, target, to): (Value, Value, Value, Value, Value)| {
                call_bool(parse_tool(
                    ToolUseKind::Scythe,
                    player,
                    item,
                    from,
                    target,
                    to,
                    None,
                )?)
            },
        )?,
    )?;
    g.set(
        "onUseQuest",
        lua.create_function(|_, (player, item, chest): (Value, Value, Value)| {
            let quest = match &chest {
                Value::Table(t) => Some(parse_chest(t)?),
                _ => None,
            };
            call_bool(parse_tool(
                ToolUseKind::Quest,
                player,
                item,
                Value::Nil,
                Value::Nil,
                Value::Nil,
                quest,
            )?)
        })?,
    )?;
    g.set(
        "checkScarabTile",
        lua.create_function(|_, pos: Value| {
            let (x, y, z) = parse_pos(&pos).unwrap_or((0, 0, 0));
            call_bool(ToolUseRequest {
                kind: ToolUseKind::CheckScarab,
                player: 0,
                item: None,
                target_item: None,
                target_creature: None,
                target_is_item_userdata: false,
                target_itemid: None,
                target_actionid: 0,
                from: (0, 0, 0),
                to: (x, y, z),
                quest: None,
            })
        })?,
    )?;
    Ok(())
}

fn call_bool(req: ToolUseRequest) -> Result<bool, mlua::Error> {
    call_lua_tool_use(req).map_err(mlua::Error::runtime)
}

fn parse_tool(
    kind: ToolUseKind,
    player: Value,
    item: Value,
    from: Value,
    target: Value,
    to: Value,
    quest: Option<QuestChestSpec>,
) -> Result<ToolUseRequest, mlua::Error> {
    let player_id = creature_id(&player).unwrap_or(0);
    let item_id = item_id(&item);
    let from_pos = parse_pos(&from).unwrap_or((0, 0, 0));
    let to_pos = parse_pos(&to).unwrap_or((0, 0, 0));
    let (target_item, target_creature, target_is_item_userdata, target_itemid, target_actionid) =
        parse_target(&target);
    Ok(ToolUseRequest {
        kind,
        player: player_id,
        item: item_id,
        target_item,
        target_creature,
        target_is_item_userdata,
        target_itemid,
        target_actionid,
        from: from_pos,
        to: to_pos,
        quest,
    })
}

fn creature_id(v: &Value) -> Option<u64> {
    let Value::UserData(ud) = v else {
        return None;
    };
    ud.borrow::<CreatureRef>().ok().map(|c| c.0)
}

fn item_id(v: &Value) -> Option<u64> {
    crate::userdata::item::item_script_id_from_value(v)
}

fn parse_pos(v: &Value) -> Option<(u16, u16, u8)> {
    match v {
        Value::UserData(ud) => ud.borrow::<PositionRef>().ok().map(|p| (p.x, p.y, p.z)),
        Value::Table(t) => {
            let x: i64 = t.get("x").or_else(|_| t.get(1)).ok()?;
            let y: i64 = t.get("y").or_else(|_| t.get(2)).ok()?;
            let z: i64 = t.get("z").or_else(|_| t.get(3)).ok()?;
            Some((x as u16, y as u16, z as u8))
        }
        _ => None,
    }
}

fn parse_target(v: &Value) -> (Option<u64>, Option<u64>, bool, Option<u16>, u16) {
    match v {
        Value::UserData(ud) => {
            if let Ok(item) = ud.borrow::<ItemRef>() {
                let id = item.0;
                let (itemid, aid) = crate::context::current_ctx(|ctx| {
                    ctx.get_item_data(id)
                        .map(|d| (Some(d.item_type), d.action_id))
                        .unwrap_or((None, 0))
                })
                .unwrap_or((None, 0));
                return (Some(id), None, true, itemid, aid);
            }
            if let Ok(cont) = ud.borrow::<ContainerRef>() {
                let id = cont.0;
                let (itemid, aid) = crate::context::current_ctx(|ctx| {
                    ctx.get_item_data(id)
                        .map(|d| (Some(d.item_type), d.action_id))
                        .unwrap_or((None, 0))
                })
                .unwrap_or((None, 0));
                return (Some(id), None, true, itemid, aid);
            }
            if let Ok(c) = ud.borrow::<CreatureRef>() {
                return (None, Some(c.0), false, None, 0);
            }
            (None, None, false, None, 0)
        }
        Value::Table(t) => {
            let itemid = lua_u16(t.get("itemid").ok());
            let aid = lua_u16(t.get("actionid").ok()).unwrap_or(0);
            (None, None, false, itemid, aid)
        }
        _ => (None, None, false, None, 0),
    }
}

fn lua_u16(v: Option<Value>) -> Option<u16> {
    match v? {
        Value::Integer(i) if i >= 0 && i <= i64::from(u16::MAX) => Some(i as u16),
        Value::Number(n) if n >= 0.0 && n <= f64::from(u16::MAX) => Some(n as u16),
        _ => None,
    }
}

fn parse_chest(t: &Table) -> Result<QuestChestSpec, mlua::Error> {
    let storage_value: u32 = t.get("storageValue")?;
    let item_tbl: Table = t.get("item")?;
    let item = parse_reward(&item_tbl)?;
    let mut content = Vec::new();
    if let Ok(Value::Table(list)) = t.get::<Value>("content") {
        for pair in list.pairs::<Value, Value>() {
            let (_, v) = pair?;
            if let Value::Table(row) = v {
                content.push(parse_reward(&row)?);
            }
        }
    }
    Ok(QuestChestSpec {
        storage_value,
        item,
        content,
    })
}

fn parse_reward(t: &Table) -> Result<QuestRewardSpec, mlua::Error> {
    Ok(QuestRewardSpec {
        id: opt_u16(t, "id")?.unwrap_or(0),
        count: opt_u16(t, "count")?,
        subtype: opt_u16(t, "subtype")?,
        charges: opt_u16(t, "charges")?,
        text: t.get::<Option<String>>("text")?,
        keynumber: match t.get::<Value>("keynumber")? {
            Value::Nil => None,
            Value::Integer(i) => Some(i),
            Value::Number(n) => Some(n as i64),
            _ => None,
        },
    })
}

fn opt_u16(t: &Table, key: &str) -> Result<Option<u16>, mlua::Error> {
    Ok(lua_u16(t.get(key).ok()))
}
