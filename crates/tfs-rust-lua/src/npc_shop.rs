//! TFS `openShopWindow` / `closeShopWindow` globals and per-player shop callbacks.
//!
//! Pack: `NpcScriptInterface::luaOpenShopWindow` / `luaCloseShopWindow` — `npc.cpp`;
//! `NpcEventsHandler::onPlayerTrade` callback signature — `npc.cpp`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use mlua::{Function, Lua, RegistryKey, Table, Value};

use crate::context::{CreatureRef, current_ctx};
use crate::lua_mutation::{ShopItemSpec, call_lua_close_shop_window, call_lua_open_shop_window};
use crate::runtime::LuaRuntime;

#[derive(Debug, Default)]
pub(crate) struct ShopWindowCallbacks {
    pub buy: Option<RegistryKey>,
    pub sell: Option<RegistryKey>,
}

impl LuaRuntime {
    pub fn has_shop_callbacks(&self, player_u64: u64) -> (bool, bool) {
        match self.shop_callbacks.borrow().get(&player_u64) {
            Some(c) => (c.buy.is_some(), c.sell.is_some()),
            None => (false, false),
        }
    }

    pub fn clear_shop_callbacks(&self, player_u64: u64) {
        self.shop_callbacks.borrow_mut().remove(&player_u64);
    }

    /// TFS buy callback — `(player, itemId, subType, amount, ignoreCap, inBackpacks)`.
    pub fn call_shop_buy(
        &self,
        player_u64: u64,
        item_id: u16,
        sub_type: u8,
        amount: u8,
        ignore_cap: bool,
        in_backpacks: bool,
    ) -> Result<(), crate::runtime::LuaError> {
        let callbacks = self.shop_callbacks.borrow();
        let Some(key) = callbacks.get(&player_u64).and_then(|c| c.buy.as_ref()) else {
            return Ok(());
        };
        let function: Function = self
            .lua
            .registry_value(key)
            .map_err(crate::runtime::LuaError::Init)?;
        drop(callbacks);
        let player_ud = self
            .lua
            .create_userdata(CreatureRef(player_u64))
            .map_err(crate::runtime::LuaError::Init)?;
        let _: mlua::Value = self.call_lua(
            &function,
            (
                player_ud,
                item_id,
                u16::from(sub_type),
                u16::from(amount),
                ignore_cap,
                in_backpacks,
            ),
        )?;
        Ok(())
    }

    /// TFS sell callback — `(player, itemId, subType, amount, ignoreEquipped, _)`.
    pub fn call_shop_sell(
        &self,
        player_u64: u64,
        item_id: u16,
        sub_type: u8,
        amount: u8,
        ignore_equipped: bool,
    ) -> Result<(), crate::runtime::LuaError> {
        let callbacks = self.shop_callbacks.borrow();
        let Some(key) = callbacks.get(&player_u64).and_then(|c| c.sell.as_ref()) else {
            return Ok(());
        };
        let function: Function = self
            .lua
            .registry_value(key)
            .map_err(crate::runtime::LuaError::Init)?;
        drop(callbacks);
        let player_ud = self
            .lua
            .create_userdata(CreatureRef(player_u64))
            .map_err(crate::runtime::LuaError::Init)?;
        let _: mlua::Value = self.call_lua(
            &function,
            (
                player_ud,
                item_id,
                u16::from(sub_type),
                u16::from(amount),
                ignore_equipped,
                false,
            ),
        )?;
        Ok(())
    }
}

fn store_callback(lua: &Lua, value: Value) -> Result<Option<RegistryKey>, mlua::Error> {
    match value {
        Value::Function(f) => lua.create_registry_value(f).map(Some),
        Value::Nil => Ok(None),
        _ => Ok(None),
    }
}

fn parse_shop_table(table: &Table) -> Result<Vec<ShopItemSpec>, mlua::Error> {
    let mut items = Vec::new();
    for pair in table.clone().pairs::<Value, Table>() {
        let (_, row) = pair?;
        let item_id: u16 = row.get("id")?;
        let sub_type: u8 = row
            .get::<Option<i32>>("subType")?
            .or_else(|| row.get::<Option<i32>>("subtype").ok().flatten())
            .unwrap_or(0)
            .clamp(0, 255) as u8;
        let buy: i32 = row.get("buy").unwrap_or(0);
        let sell: i32 = row.get("sell").unwrap_or(0);
        let buy_price = if buy <= 0 { 0 } else { buy as u32 };
        let sell_price = if sell <= 0 { 0 } else { sell as u32 };
        let name: String = row.get("name").unwrap_or_default();
        items.push(ShopItemSpec {
            item_id,
            sub_type,
            buy_price,
            sell_price,
            name,
        });
    }
    Ok(items)
}

fn resolve_player_creature(raw: Value) -> Result<u64, mlua::Error> {
    match raw {
        Value::UserData(ud) => {
            if let Ok(p) = ud.borrow::<CreatureRef>() {
                Ok(p.0)
            } else {
                Err(mlua::Error::runtime("expected Player creature id"))
            }
        }
        Value::Integer(n) => Ok(n as u64),
        Value::Number(n) => Ok(n as u64),
        _ => Err(mlua::Error::runtime("expected player cid")),
    }
}

/// Register `openShopWindow` / `closeShopWindow` on the Lua globals table.
pub fn register_npc_shop_globals(
    lua: &Lua,
    shop_callbacks: Rc<RefCell<HashMap<u64, ShopWindowCallbacks>>>,
) -> Result<(), mlua::Error> {
    let rt_open = Rc::clone(&shop_callbacks);
    lua.globals().set(
        "openShopWindow",
        lua.create_function(move |lua, args: mlua::MultiValue| {
            let mut iter = args.into_iter();
            let player_raw = iter
                .next()
                .ok_or_else(|| mlua::Error::runtime("openShopWindow: missing player"))?;
            let items_val = iter
                .next()
                .ok_or_else(|| mlua::Error::runtime("openShopWindow: missing items"))?;
            let buy_fn = iter.next().unwrap_or(Value::Nil);
            let sell_fn = iter.next().unwrap_or(Value::Nil);

            let player_u64 = resolve_player_creature(player_raw)?;
            let items_table = items_val
                .as_table()
                .ok_or_else(|| mlua::Error::runtime("openShopWindow: items must be a table"))?;
            let items = parse_shop_table(items_table)?;

            let buy_key = store_callback(lua, buy_fn)?;
            let sell_key = store_callback(lua, sell_fn)?;
            rt_open.borrow_mut().insert(
                player_u64,
                ShopWindowCallbacks {
                    buy: buy_key,
                    sell: sell_key,
                },
            );

            let npc_u64 = current_ctx(|ctx| ctx.find_shop_npc_for_player(player_u64))
                .flatten()
                .ok_or_else(|| mlua::Error::runtime("openShopWindow: no focused NPC"))?;

            call_lua_open_shop_window(player_u64, npc_u64, items).map_err(mlua::Error::external)?;
            Ok(true)
        })?,
    )?;

    lua.globals().set(
        "closeShopWindow",
        lua.create_function(move |_, player_raw: Value| {
            let player_u64 = resolve_player_creature(player_raw)?;
            let npc_u64 = current_ctx(|ctx| ctx.find_shop_npc_for_player(player_u64)).flatten();
            call_lua_close_shop_window(player_u64, npc_u64, true).map_err(mlua::Error::external)?;
            Ok(true)
        })?,
    )?;

    Ok(())
}
