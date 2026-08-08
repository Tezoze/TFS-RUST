//! Player / creature userdata bindings for Lua (`Player` / `Creature`).
//!
//! C++ reference: `src/luascript.cpp` — `Creature` / `Player` userdata methods.

use mlua::{MetaMethod, UserData, UserDataMethods, Value};
use std::cell::RefCell;

use crate::context::{CURRENT_CTX, CreatureData, CreatureRef, ItemRef, LuaContext};
use crate::lua_mutation::{
    call_lua_add_condition, call_lua_add_item, call_lua_add_item_full, call_lua_add_mana,
    call_lua_add_mana_spent, call_lua_feed, call_lua_get_depot_chest, call_lua_get_inbox,
    call_lua_remove_condition, call_lua_remove_item, call_lua_send_cancel_message,
    call_lua_set_in_fight,
};
use crate::userdata::container::ContainerRef;
use crate::userdata::group::GroupRef;
use crate::userdata::position::PositionRef;
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

        // `player:getMurderTimestamps()` — TVP / kills.lua unjust history.
        methods.add_method("getMurderTimestamps", |lua, this, ()| {
            with_ctx(|ctx| {
                let stamps = ctx.get_player_murder_timestamps(this.0);
                let table = lua.create_table_with_capacity(stamps.len(), 0)?;
                for (i, ts) in stamps.into_iter().enumerate() {
                    table.set(i + 1, ts)?;
                }
                Ok(table)
            })
        });

        // `player:getPlayerKillerEnd()` — red-skull end unix time.
        methods.add_method("getPlayerKillerEnd", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.get_player_killer_end(this.0).unwrap_or(0)))
        });

        // `player:getStorageValue(key)` — `Player::getStorageValue` (`player.cpp`).
        // Missing key → `-1`. Doors Phase 4: quest door gate in `doors.lua`.
        methods.add_method("getStorageValue", |_, this, key: Value| {
            let key = match key {
                Value::Integer(i) => i as u32,
                Value::Number(n) => n as u32,
                Value::Nil => 0,
                _ => {
                    return Err(mlua::Error::runtime(
                        "getStorageValue: expected integer key",
                    ));
                }
            };
            with_ctx(|ctx| Ok(ctx.get_player_storage_value(this.0, key)))
        });

        // `player:setStorageValue(key, value)` — `Player::addStorageValue`.
        // Reserved range (`const.h` PSTRG_RESERVED_RANGE) rejected like TFS Lua.
        // `value == -1` erases the key.
        methods.add_method("setStorageValue", |_, this, (key, value): (Value, Value)| {
            let key = match key {
                Value::Integer(i) => i as u32,
                Value::Number(n) => n as u32,
                Value::Nil => 0,
                _ => {
                    return Err(mlua::Error::runtime(
                        "setStorageValue: expected integer key",
                    ));
                }
            };
            let value = match value {
                Value::Integer(i) => i as i32,
                Value::Number(n) => n as i32,
                _ => {
                    return Err(mlua::Error::runtime(
                        "setStorageValue: expected integer value",
                    ));
                }
            };
            // `const.h` `PSTRG_RESERVED_RANGE_START` / `_SIZE` — `IS_IN_KEYRANGE`.
            const RESERVED_START: u32 = 10_000_000;
            const RESERVED_SIZE: u32 = 10_000_000;
            if key >= RESERVED_START && key - RESERVED_START <= RESERVED_SIZE {
                return Ok(false);
            }
            crate::lua_mutation::call_lua_set_storage_value(this.0, key, value)
                .map_err(mlua::Error::runtime)?;
            Ok(true)
        });

        methods.add_method("getBankBalance", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_player_bank_balance(this.0)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        methods.add_method("getMoney", |_, this, ()| {
            with_ctx(|ctx| {
                let g = ctx
                    .get_player_item_type_count(this.0, 2148, -1)
                    .unwrap_or(0) as u64;
                let p = ctx
                    .get_player_item_type_count(this.0, 2152, -1)
                    .unwrap_or(0) as u64;
                let c = ctx
                    .get_player_item_type_count(this.0, 2160, -1)
                    .unwrap_or(0) as u64;
                Ok(g + p * 100 + c * 10_000)
            })
        });

        methods.add_method_mut("depositMoney", |_, this, amount: u64| {
            crate::lua_mutation::call_lua_bank_deposit(this.0, amount)
                .map_err(mlua::Error::runtime)?;
            Ok(true)
        });

        methods.add_method_mut("withdrawMoney", |_, this, amount: u64| {
            crate::lua_mutation::call_lua_bank_withdraw(this.0, amount)
                .map_err(mlua::Error::runtime)?;
            Ok(true)
        });

        // `player:getMagicLevel()` — `Player::getMagicLevel` (`player.h`).
        // PC-3a Phase 1: value-callback spells call `self:getMagicLevel()` inside
        // `computeDamage` / `computeHealing` (native methods below; also `functions.lua`).
        methods.add_method("getMagicLevel", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_player_magic_level(this.0)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        // `player:getSpellCoeff()` → `(levelMult, magicMult)` from `MechanicsProfile`
        // / `data/formulas/<era>.lua` `formulas.spell`.
        methods.add_method("getSpellCoeff", |_, _this, ()| {
            with_ctx(|ctx| Ok(ctx.get_spell_coeff()))
        });

        // `player:computeDamage(damage, variation[, limitMinimum[, limitMaximum]])`
        // — 772 `ComputeDamage` (`magic.cc:776`); coeffs from profile, not hardcoded 2/3.
        methods.add_method(
            "computeDamage",
            |_, this, args: mlua::Variadic<Value>| {
                let (damage, variation, limit_min, limit_max) =
                    parse_compute_damage_args(&args)?;
                with_ctx(|ctx| {
                    let (lo, hi) = ctx.compute_magic_damage_range(
                        this.0,
                        damage,
                        variation,
                        limit_min,
                        limit_max,
                    );
                    Ok((-lo, -hi))
                })
            },
        );

        // `player:computeHealing(...)` — same formula, positive magnitudes.
        methods.add_method(
            "computeHealing",
            |_, this, args: mlua::Variadic<Value>| {
                let (damage, variation, limit_min, limit_max) =
                    parse_compute_damage_args(&args)?;
                with_ctx(|ctx| {
                    Ok(ctx.compute_magic_damage_range(
                        this.0,
                        damage,
                        variation,
                        limit_min,
                        limit_max,
                    ))
                })
            },
        );

        // `player:computeSkillDamage(damage, variation, skill[, limitMinimum[, limitMaximum]])`
        // — magic formula then `× level / 25` (`functions.lua`).
        methods.add_method(
            "computeSkillDamage",
            |_, this, args: mlua::Variadic<Value>| {
                let (damage, variation, _skill, limit_min, limit_max) =
                    parse_compute_skill_damage_args(&args)?;
                with_ctx(|ctx| {
                    let level = ctx.get_player_level(this.0).unwrap_or(0);
                    let (lo, hi) = ctx.compute_magic_damage_range(
                        this.0,
                        damage,
                        variation,
                        limit_min,
                        limit_max,
                    );
                    let lo = (lo * level) / 25;
                    let hi = (hi * level) / 25;
                    Ok((-lo, -hi))
                })
            },
        );

        // `player:getMana()` — `Player::getMana` (`player.h`).
        // PC-3a Phase 5: `conjureItem` dual-hand second-conjure mana check.
        methods.add_method("getMana", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_player_mana(this.0)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        // `player:addMana(manaChange)` — `luascript.cpp` `luaPlayerAddMana`.
        // PC-3a Phase 5: `conjureItem` deducts mana for dual-hand second conjure.
        methods.add_method("addMana", |_, this, mana_change: i32| {
            call_lua_add_mana(this.0, mana_change).map_err(mlua::Error::runtime)?;
            Ok(true)
        });

        // `player:addManaSpent(amount)` — `luascript.cpp` `luaPlayerAddManaSpent`.
        // PC-3a Phase 5: advances magic level for dual-hand second conjure.
        methods.add_method("addManaSpent", |_, this, amount: u64| {
            call_lua_add_mana_spent(this.0, amount).map_err(mlua::Error::runtime)?;
            Ok(true)
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

        // `player:getGroup()` — `Player::getGroup` (`player.h`). CH-6 talkaction
        // access gating; returns a `GroupRef` userdata wrapping the player's
        // `group_id`. `GroupRef:getAccess()` reads the `access` flag from the
        // group database via `ScriptContext::get_group_access`.
        methods.add_method("getGroup", |lua, this, ()| {
            let group_id_opt = with_ctx(|ctx| Ok(ctx.get_player_group_id(this.0)))?;
            match group_id_opt {
                Some(gid) => {
                    let ud = lua.create_userdata(GroupRef(gid))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        // `player:getPosition()` — `Creature::getPosition` (`creature.h`).
        // CH-6 talkaction `sendMagicEffect` at player position; returns a
        // `PositionRef` userdata wrapping `(x, y, z)`.
        methods.add_method("getPosition", |lua, this, ()| {
            let pos_opt = with_ctx(|ctx| Ok(ctx.get_player_position(this.0)))?;
            match pos_opt {
                Some(pos) => {
                    let ud = lua.create_userdata(PositionRef {
                        x: pos.x,
                        y: pos.y,
                        z: pos.z,
                    })?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
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

        // `player:getPremiumEndsAt()` — unix seconds (`accounts.premium_ends_at`).
        methods.add_method("getPremiumEndsAt", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_player_premium_ends_at(this.0)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        // `player:setPremiumEndsAt(timestamp)` — in-memory + DB persist.
        methods.add_method_mut("setPremiumEndsAt", |_, this, ends_at: u32| {
            crate::lua_mutation::call_lua_set_premium_ends_at(this.0, ends_at)
                .map_err(mlua::Error::runtime)?;
            Ok(true)
        });

        // `player:isPremium()` — C++ `Player::isPremium` (`player.cpp`).
        // Native so scripts work even when `configManager` context is absent.
        methods.add_method("isPremium", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.player_is_premium(this.0)))
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
                    mlua::Value::Integer(n) => {
                        (n as u16, count.unwrap_or(1), sub_type.unwrap_or(-1))
                    }
                    mlua::Value::Number(n) => {
                        (n as u16, count.unwrap_or(1), sub_type.unwrap_or(-1))
                    }
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
                    let id_opt =
                        call_lua_add_item_full(this.0, item_type, count, sub_type, can_drop, slot)
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

        methods.add_method(
            "getItemCount",
            |_, this, (item_type, sub_type): (u16, Option<i32>)| {
                let sub_type = sub_type.unwrap_or(-1);
                with_ctx(|ctx| {
                    ctx.get_player_item_type_count(this.0, item_type, sub_type)
                        .ok_or_else(|| mlua::Error::runtime("player not found"))
                })
            },
        );

        methods.add_method(
            "removeItem",
            |_,
             this,
             (item_type, count, sub_type, ignore_equipped): (
                u16,
                u32,
                Option<i32>,
                Option<bool>,
            )| {
                let sub_type = sub_type.unwrap_or(-1);
                let ignore_equipped = ignore_equipped.unwrap_or(false);
                call_lua_remove_item(this.0, item_type, count, sub_type, ignore_equipped)
                    .map_err(mlua::Error::runtime)
            },
        );

        methods.add_method(
            "getItemById",
            |lua, this, (item_type, deep_search, sub_type): (mlua::Value, bool, Option<i32>)| {
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
            let id_opt = call_lua_get_inbox(this.0).map_err(mlua::Error::runtime)?;
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

        // `player:sendCancelMessage(text)` — LUA-3. Sends a
        // `MESSAGE_STATUS_SMALL` cancel text to the player's client. Accepts
        // both a `string` (active scripts) and an integer `RETURNVALUE_*`
        // code (CH-5 commented blocks); integer codes are mapped to their
        // `getReturnMessage` description (`tools.cpp`).
        // C++ reference: `protocolgame.cpp` `sendTextMessage`.
        methods.add_method("sendCancelMessage", |_, this, value: mlua::Value| {
            let text = resolve_cancel_message_text(value)?;
            tracing::info!(creature = this.0, %text, "Lua player:sendCancelMessage()");
            call_lua_send_cancel_message(this.0, text).map_err(mlua::Error::runtime)?;
            Ok(())
        });

        // `player:getCondition(type, id, subId)` — LUA-4. Returns `true` if an
        // active condition matching `(ctype, cond_id, sub_id)` exists, `nil`
        // otherwise. Scripts use this for truthiness only
        // (`if player:getCondition(...) then`).
        // C++ reference: `luascript.cpp:2116` `Creature::getCondition`.
        methods.add_method(
            "getCondition",
            |_, this, (ctype, cond_id, sub_id): (i32, i32, u32)| {
                with_ctx(|ctx| {
                    Ok(
                        if ctx
                            .get_creature_condition(this.0, ctype, cond_id, sub_id)
                            .is_some()
                        {
                            mlua::Value::Boolean(true)
                        } else {
                            mlua::Value::Nil
                        },
                    )
                })
            },
        );

        // `player:addCondition(condition)` — LUA-4 / PC-3a Phase 3. Immediate-apply
        // condition add with full `ConditionApplySpec` fields.
        // C++ reference: `luascript.cpp:2117` `Creature::addCondition`.
        methods.add_method("addCondition", |_, this, condition: mlua::AnyUserData| {
            let builder = condition
                .borrow::<crate::userdata::condition::ConditionBuilder>()
                .map_err(mlua::Error::runtime)?;
            call_lua_add_condition(this.0, builder.to_apply_spec())
                .map_err(mlua::Error::runtime)?;
            Ok(())
        });

        // `player:removeCondition(type[, id[, subId]])` — LUA-4. Immediate-apply
        // condition removal. Optional id/subId default to -1 / 0 (match all of type
        // when core ignores id for retain).
        // C++ reference: `luascript.cpp:2118` `Creature::removeCondition`.
        methods.add_method(
            "removeCondition",
            |_, this, (ctype, cond_id, sub_id): (i32, Option<i32>, Option<u32>)| {
                call_lua_remove_condition(
                    this.0,
                    ctype,
                    cond_id.unwrap_or(-1),
                    sub_id.unwrap_or(0),
                )
                .map_err(mlua::Error::runtime)?;
                Ok(())
            },
        );

        // `creature:isPlayer()` — PC-3a Phase 3 (`poison_storm.lua`).
        methods.add_method("isPlayer", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.is_creature_player(this.0)))
        });

        // `creature:isItem()` — Thing discriminator (`data/lib/core/creature.lua`).
        methods.add_method("isItem", |_, _this, ()| Ok(false));

        // `creature:isCreature()` — always true for Creature userdata.
        methods.add_method("isCreature", |_, _this, ()| Ok(true));

        // `creature:isMonster()` — PC-3a Gap 5/6.
        methods.add_method("isMonster", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.is_creature_monster(this.0)))
        });

        // `creature:getDirection()` — facing direction 0..=7.
        methods.add_method("getDirection", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.get_player_direction(this.0).unwrap_or(0)))
        });

        // `creature:getSummons()` — array of Creature userdata.
        methods.add_method("getSummons", |lua, this, ()| {
            let ids = with_ctx(|ctx| Ok(ctx.get_creature_summons(this.0)))?;
            let t = lua.create_table_with_capacity(ids.len(), 0)?;
            for (i, id) in ids.into_iter().enumerate() {
                let ud = lua.create_userdata(CreatureRef(id))?;
                t.set(i + 1, ud)?;
            }
            Ok(t)
        });

        // `creature:addSummon(monster)` — PC-3a Gap 5.
        methods.add_method("addSummon", |_, this, summon: Value| {
            let summon_id = match summon {
                Value::UserData(ud) => ud.borrow::<CreatureRef>()?.0,
                _ => {
                    return Err(mlua::Error::runtime(
                        "addSummon: expected Creature userdata",
                    ));
                }
            };
            crate::lua_mutation::call_add_summon(this.0, summon_id).map_err(mlua::Error::runtime)?;
            Ok(true)
        });

        // `monster:getType()` → MonsterType(name).
        methods.add_method("getType", |lua, this, ()| {
            let name = with_ctx(|ctx| Ok(ctx.get_creature_monster_type_name(this.0)))?;
            match name {
                Some(n) => {
                    let ud = lua.create_userdata(crate::userdata::monster_type::MonsterTypeRef {
                        name: n,
                    })?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        // `creature:move(tile, flags)` — returns RETURNVALUE_NOERROR (0) or NOTPOSSIBLE.
        methods.add_method("move", |_, this, (tile, flags): (Value, Option<u32>)| {
            let (x, y, z) = match tile {
                Value::UserData(ud) => {
                    if let Ok(t) = ud.borrow::<crate::userdata::tile::TileRef>() {
                        (t.x, t.y, t.z)
                    } else if let Ok(p) = ud.borrow::<PositionRef>() {
                        (p.x, p.y, p.z)
                    } else {
                        return Err(mlua::Error::runtime("move: expected Tile"));
                    }
                }
                _ => return Err(mlua::Error::runtime("move: expected Tile")),
            };
            let ok = crate::lua_mutation::call_creature_move_to_tile(
                this.0,
                x,
                y,
                z,
                flags.unwrap_or(0),
            )
            .map_err(mlua::Error::runtime)?;
            // Levitate compares to RETURNVALUE_NOERROR (0).
            Ok(if ok { 0i32 } else { 1i32 })
        });

        // `creature:teleportTo(pos[, pushMovement])`.
        methods.add_method(
            "teleportTo",
            |_, this, (pos, push): (Value, Option<bool>)| {
                let (x, y, z) = match pos {
                    Value::UserData(ud) => {
                        if let Ok(p) = ud.borrow::<PositionRef>() {
                            (p.x, p.y, p.z)
                        } else {
                            return Err(mlua::Error::runtime("teleportTo: expected Position"));
                        }
                    }
                    Value::Table(t) => {
                        let x: i64 = t.get("x").or_else(|_| t.get(1))?;
                        let y: i64 = t.get("y").or_else(|_| t.get(2))?;
                        let z: i64 = t.get("z").or_else(|_| t.get(3))?;
                        (x as u16, y as u16, z as u8)
                    }
                    _ => {
                        return Err(mlua::Error::runtime("teleportTo: expected Position"));
                    }
                };
                crate::lua_mutation::call_creature_teleport(
                    this.0,
                    x,
                    y,
                    z,
                    push.unwrap_or(false),
                )
                .map_err(mlua::Error::runtime)?;
                Ok(true)
            },
        );

        // `creature:sendTextMessage(type, text)`.
        methods.add_method(
            "sendTextMessage",
            |_, this, (msg_class, text): (u8, String)| {
                crate::lua_mutation::call_send_text_message(this.0, msg_class, text)
                    .map_err(mlua::Error::runtime)?;
                Ok(())
            },
        );

        // `player:setInFight(bool)` — PC-3a Phase 3 (`poison_storm.lua`).
        methods.add_method("setInFight", |_, this, in_fight: bool| {
            call_lua_set_in_fight(this.0, in_fight).map_err(mlua::Error::runtime)?;
            Ok(())
        });

        // `__index` fallback — bridges `function Player:method(...)` definitions
        // from `data/scripts/functions.lua` (and event scripts) onto `CreatureRef`
        // userdata. mlua 0.10 calls this metamethod only when the regular method
        // lookup (registered above) fails, so native Rust methods take priority.
        //
        // Without this, `creature:conjureItem(...)` / `creature:computeDamage(...)`
        // would error with "attempt to call nil value" — `functions.lua` defines
        // them as `function Player:conjureItem(...)` (table fields on the `Player`
        // global), not as `CreatureRef` userdata methods.
        //
        // C++ reference: TFS `LuaScriptInterface::registerClass` sets
        // `Creature`/`Player` method tables so `self:method()` resolves via the
        // class hierarchy. We mirror that by chaining `__index` → `Player` table.
        methods.add_meta_method(MetaMethod::Index, |lua, _this, key: mlua::LuaString| {
            let player_table: mlua::Table = lua.globals().get("Player")?;
            player_table.get::<mlua::Value>(key)
        });
    }
}

/// Parse `computeDamage(damage, variation[, limitMinimum[, limitMaximum]])` args.
fn parse_compute_damage_args(
    args: &mlua::Variadic<Value>,
) -> Result<(i32, i32, bool, bool), mlua::Error> {
    let damage = args
        .first()
        .and_then(value_as_i32)
        .ok_or_else(|| mlua::Error::runtime("computeDamage: damage required"))?;
    let variation = args.get(1).and_then(value_as_i32).unwrap_or(0);
    let limit_min = args.get(2).and_then(value_as_bool).unwrap_or(false);
    let limit_max = args.get(3).and_then(value_as_bool).unwrap_or(false);
    Ok((damage, variation, limit_min, limit_max))
}

/// Parse `computeSkillDamage(damage, variation, skill[, limitMinimum[, limitMaximum]])`.
fn parse_compute_skill_damage_args(
    args: &mlua::Variadic<Value>,
) -> Result<(i32, i32, i32, bool, bool), mlua::Error> {
    let damage = args
        .first()
        .and_then(value_as_i32)
        .ok_or_else(|| mlua::Error::runtime("computeSkillDamage: damage required"))?;
    let variation = args.get(1).and_then(value_as_i32).unwrap_or(0);
    let skill = args.get(2).and_then(value_as_i32).unwrap_or(0);
    let limit_min = args.get(3).and_then(value_as_bool).unwrap_or(false);
    let limit_max = args.get(4).and_then(value_as_bool).unwrap_or(false);
    Ok((damage, variation, skill, limit_min, limit_max))
}

fn value_as_i32(v: &Value) -> Option<i32> {
    match v {
        Value::Integer(n) => Some(*n as i32),
        Value::Number(n) => Some(*n as i32),
        _ => None,
    }
}

fn value_as_bool(v: &Value) -> Option<bool> {
    match v {
        Value::Boolean(b) => Some(*b),
        Value::Nil => Some(false),
        _ => None,
    }
}

/// Resolve a `sendCancelMessage` argument to the player-visible text string.
///
/// Accepts:
/// - `string` — used directly (active scripts pass literal messages).
/// - `integer` — a `RETURNVALUE_*` enum code; mapped to its `getReturnMessage`
///   description (`tools.cpp`). Only the codes the channel scripts reference
///   are mapped; unknown codes fall back to the numeric string.
///
/// C++ reference: `enums.h:301-370` `ReturnValue_t` (772 numbering, 0-56 match
/// the Rust `ReturnValue` enum; codes > 56 diverge and are TODO for era-aware
/// mapping).
fn resolve_cancel_message_text(value: mlua::Value) -> Result<String, mlua::Error> {
    match value {
        mlua::Value::String(s) => Ok(s.to_str()?.to_string()),
        mlua::Value::Integer(n) => Ok(return_value_message(n as i32)),
        mlua::Value::Number(n) => Ok(return_value_message(n as i32)),
        _ => Err(mlua::Error::runtime(
            "sendCancelMessage: expected string or integer",
        )),
    }
}

/// Map a 772 `RETURNVALUE_*` integer code to its `getReturnMessage` text
/// (`tools.cpp`). Only codes referenced by the channel scripts are mapped
/// here; unknown codes fall back to the numeric string. This is a lightweight
/// inline table — the full `ReturnValue` enum lives in `tfs-rust-core` and
/// can't be referenced from this crate (no dependency).
fn return_value_message(code: i32) -> String {
    match code {
        0 => "No error.".to_string(),
        27 => "A player with this name is not online.".to_string(),
        36 => "You are exhausted.".to_string(),
        35 => "You do not have enough soulpoints.".to_string(),
        34 => "You do not have enough mana.".to_string(),
        33 => "You do not have enough magic level.".to_string(),
        32 => "Your level is too low.".to_string(),
        21 => "This object is too heavy for you to carry.".to_string(),
        20 => "You cannot put more objects in this container.".to_string(),
        17 => "You are too far away.".to_string(),
        n => format!("Return code: {n}"),
    }
}

#[cfg(test)]
mod tests {
    //! LUA-2 unit tests: a fake `ScriptContext` backs the new player read
    //! methods (`getLevel` / `getAccountType` / `getVocation():getId()` /
    //! `hasFlag`), exercised through the real mlua userdata bindings via
    //! `with_lua_context`. This validates the read path end-to-end without
    //! spinning up a full `GameWorld`.

    use crate::context::{CreatureRef, with_lua_context};
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
        fn get_player_slot_item_id(&self, _: ScriptCreatureId, _: u8) -> Option<ScriptItemId> {
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

    /// Doors Phase 4: `getStorageValue` reads via ScriptContext; reserved
    /// `setStorageValue` rejects without mutation (TFS `IS_IN_KEYRANGE`).
    #[test]
    fn player_get_storage_value_and_reserved_reject_through_lua() {
        use std::collections::HashMap;

        struct StorageCtx {
            map: HashMap<u32, i32>,
        }
        impl ScriptContext for StorageCtx {
            fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
                (id == GM_CID).then_some(ScriptCreatureData {
                    name: "Quest".into(),
                    guid: 1,
                })
            }
            fn get_item(&self, _: ScriptItemId) -> Option<ScriptItemRef> {
                None
            }
            fn get_config_string(&self, _: &str) -> Option<String> {
                None
            }
            fn get_player_storage_value(&self, id: ScriptCreatureId, key: u32) -> i32 {
                if id != GM_CID {
                    return -1;
                }
                self.map.get(&key).copied().unwrap_or(-1)
            }
            fn get_player_slot_item_id(&self, _: ScriptCreatureId, _: u8) -> Option<ScriptItemId> {
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

        let lua = Lua::new();
        crate::userdata::register_creature_metatable(&lua).expect("creature metatable");

        let mut map = HashMap::new();
        map.insert(320, 5);
        let ctx = StorageCtx { map };
        with_lua_context(&ctx, || {
            let ud = lua.create_userdata(CreatureRef(GM_CID)).expect("userdata");
            lua.globals().set("player", ud).expect("set player");

            let missing: i32 = lua
                .load("return player:getStorageValue(999)")
                .eval()
                .expect("get missing");
            assert_eq!(missing, -1);

            let got: i32 = lua
                .load("return player:getStorageValue(320)")
                .eval()
                .expect("get set key");
            assert_eq!(got, 5);

            // Reserved range rejected before mutation (`const.h` PSTRG_RESERVED_RANGE).
            let reserved: bool = lua
                .load("return player:setStorageValue(10000000, 1)")
                .eval()
                .expect("reserved");
            assert!(!reserved);
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

    // ---- LUA-3 / LUA-4 tests ----

    /// `getCondition` returns nil when no condition exists (default
    /// `ScriptContext::get_creature_condition` returns `None`).
    #[test]
    fn get_condition_returns_nil_when_absent() {
        let lua = Lua::new();
        crate::userdata::register_creature_metatable(&lua).expect("creature metatable");
        crate::userdata::register_vocation_metatable(&lua).expect("vocation metatable");

        let ctx = NullCtx;
        with_lua_context(&ctx, || {
            let globals = lua.globals();
            let ud = lua.create_userdata(CreatureRef(1)).expect("userdata");
            globals.set("player", ud).expect("set player");

            // CONDITION_CHANNELMUTEDTICKS = 1<<15 = 32768, CONDITIONID_DEFAULT = -1
            let v: mlua::Value = lua
                .load("return player:getCondition(32768, -1, 7)")
                .eval()
                .expect("getCondition");
            assert!(
                matches!(v, mlua::Value::Nil),
                "expected nil for absent condition"
            );
        });
    }

    /// `ConditionBuilder` userdata — `setTicks` / `setParameter` accumulate
    /// fields correctly. This is the regression guard for `player.lua`'s
    /// `soulCondition` build (§4.1).
    #[test]
    fn condition_builder_set_ticks_and_params() {
        use crate::userdata::condition::ConditionBuilder;

        let mut builder = ConditionBuilder::new(32768, -1); // CHANNELMUTEDTICKS, DEFAULT
        assert_eq!(builder.ctype, 32768);
        assert_eq!(builder.cond_id, -1);
        assert_eq!(builder.sub_id, 0);
        assert_eq!(builder.ticks, 0);

        // Simulate `:setParameter(CONDITION_PARAM_SUBID, 7)` + `:setParameter(CONDITION_PARAM_TICKS, 3600000)`.
        builder.set_parameter(45, 7); // CONDITION_PARAM_SUBID
        builder.set_parameter(2, 3600000); // CONDITION_PARAM_TICKS
        assert_eq!(builder.sub_id, 7);
        assert_eq!(builder.ticks, 3600000);

        // `:setTicks` overrides ticks.
        builder.set_ticks(120000);
        assert_eq!(builder.ticks, 120000);

        // Unknown params are silently ignored (matching C++ `default: break`).
        builder.set_parameter(999, 42);
        assert_eq!(builder.sub_id, 7); // unchanged
    }

    /// `return_value_message` maps the codes the channel scripts reference.
    #[test]
    fn return_value_message_maps_known_codes() {
        use super::return_value_message;
        assert_eq!(
            return_value_message(27),
            "A player with this name is not online."
        );
        assert_eq!(return_value_message(0), "No error.");
        assert_eq!(return_value_message(36), "You are exhausted.");
        // Unknown codes fall back to the numeric string.
        assert_eq!(return_value_message(999), "Return code: 999");
    }

    /// `Player(name)` constructor with a fake context that resolves one name.
    /// Verifies the `__call` metamethod on the `Player` table produces a
    /// `CreatureRef` userdata, and unknown names produce `nil`.
    #[test]
    fn player_constructor_resolves_by_name() {
        let lua = Lua::new();
        crate::userdata::register_creature_metatable(&lua).expect("creature metatable");
        crate::userdata::register_vocation_metatable(&lua).expect("vocation metatable");
        crate::userdata::register_condition_metatable(&lua).expect("condition metatable");

        // Register the Player table + __call metamethod (mirrors runtime.rs).
        let player_table = lua.create_table().expect("player table");
        let player_meta = lua.create_table().expect("player meta");
        let meta_fn = lua
            .create_function(|lua, (_self, name): (mlua::Value, String)| {
                let id_opt =
                    crate::context::current_ctx(|ctx| ctx.get_player_by_name(&name)).flatten();
                match id_opt {
                    Some(id) => {
                        let ud = lua.create_userdata(CreatureRef(id))?;
                        Ok(mlua::Value::UserData(ud))
                    }
                    None => Ok(mlua::Value::Nil),
                }
            })
            .expect("meta fn");
        player_meta.set("__call", meta_fn).expect("set __call");
        player_table.set_metatable(Some(player_meta));
        lua.globals()
            .set("Player", player_table)
            .expect("set Player");

        struct NameCtx;
        impl ScriptContext for NameCtx {
            fn get_creature(&self, _: ScriptCreatureId) -> Option<ScriptCreatureData> {
                None
            }
            fn get_item(&self, _: ScriptItemId) -> Option<ScriptItemRef> {
                None
            }
            fn get_config_string(&self, _: &str) -> Option<String> {
                None
            }
            fn get_player_by_name(&self, name: &str) -> Option<ScriptCreatureId> {
                if name == "OnlinePlayer" {
                    Some(42)
                } else {
                    None
                }
            }
        }

        let ctx = NameCtx;
        with_lua_context(&ctx, || {
            // Known player → CreatureRef userdata with the right id.
            let id: u64 = lua
                .load("return Player('OnlinePlayer'):getId()")
                .eval()
                .expect("Player('OnlinePlayer')");
            assert_eq!(id, 42);

            // Unknown player → nil.
            let v: mlua::Value = lua
                .load("return Player('Nobody')")
                .eval()
                .expect("Player('Nobody')");
            assert!(
                matches!(v, mlua::Value::Nil),
                "expected nil for unknown player"
            );
        });
    }
}
