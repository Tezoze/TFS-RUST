//! Player / creature userdata bindings for Lua (`Player` / `Creature`).
//!
//! C++ reference: `src/luascript.cpp` — `Creature` / `Player` userdata methods.

use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods, Value};
use std::cell::RefCell;

use crate::context::{CURRENT_CTX, CreatureData, CreatureRef, ItemRef, LuaContext};
use crate::lua_mutation::{
    ConjureRequest, call_lua_add_condition, call_lua_add_health, call_lua_add_item,
    call_lua_add_item_full, call_lua_add_mana, call_lua_add_mana_spent, call_lua_add_skill_tries,
    call_lua_conjure_item, call_lua_creature_remove, call_lua_feed, call_lua_get_depot_chest,
    call_lua_get_depot_locker, call_lua_get_inbox, call_lua_player_say, call_lua_remove_condition,
    call_lua_remove_item, call_lua_send_cancel_message, call_lua_send_outfit_window,
    call_lua_set_direction, call_lua_set_ghost_mode, call_lua_set_in_fight, call_lua_set_outfit,
    call_lua_set_vocation, call_lua_show_text_dialog, call_player_register_creature_event,
};
use crate::userdata::container::ContainerRef;
use crate::userdata::group::GroupRef;
use crate::userdata::item::push_item_userdata;
use crate::userdata::position::PositionRef;
use crate::userdata::spell::SpellBuilder;
use crate::userdata::town::TownRef;
use crate::userdata::vocation::VocationRef;

/// Register the Creature metatable in the Lua runtime.
pub fn register_creature_metatable(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<CreatureRef>(|_registry| {})
}

/// `Outfit(lookType)` — TFS `luaOutfit` / `g_game.outfits.getOutfitByLookType`.
/// Returns a table `{ lookType, lookHead=0, …, name, premium }` or `nil`.
pub fn register_outfit_constructor(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    let outfit_new = lua.create_function(|lua, look_type: i32| {
        let info = crate::context::current_ctx(|ctx| ctx.get_outfit_info(look_type)).flatten();
        match info {
            Some((name, premium)) => {
                let t = lua.create_table()?;
                t.set("lookType", look_type)?;
                t.set("lookHead", 0i32)?;
                t.set("lookBody", 0i32)?;
                t.set("lookLegs", 0i32)?;
                t.set("lookFeet", 0i32)?;
                t.set("lookAddons", 0i32)?;
                t.set("name", name)?;
                t.set("premium", if premium { 1i32 } else { 0i32 })?;
                Ok(Value::Table(t))
            }
            None => Ok(Value::Nil),
        }
    })?;
    lua.globals().set("Outfit", outfit_new)?;
    Ok(())
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
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        crate::class_registry::register_with_recording(registry, "Creature");
    }

    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        // TFS Thing `uid` for creatures is the creature id (`luascript.cpp`).
        fields.add_field_method_get("uid", |_, this| Ok(this.0));
    }

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

        // `player:getHouse()` — TFS `luaPlayerGetHouse` / `Map::getHouseByPlayerId`.
        methods.add_method("getHouse", |lua, this, ()| {
            let house_id = crate::context::current_ctx(|ctx| {
                ctx.get_creature(this.0)
                    .and_then(|c: CreatureData| ctx.house_id_for_owner_guid(c.guid))
            })
            .flatten();
            match house_id {
                Some(id) => {
                    let ud = lua.create_userdata(crate::userdata::HouseRef(id))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        // `player:getLevel()` — `Player::getLevel` (`player.h`). LUA-2 read;
        // channel `onSpeak` gating (advertising.lua level-1 cancel).
        methods.add_method("getLevel", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_player_level(this.0)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        // `player:getEffectiveSkillLevel(skillType)` — `luaPlayerGetEffectiveSkillLevel`.
        // `nil` when missing player or skill > `SKILL_FISHING`.
        methods.add_method("getEffectiveSkillLevel", |_, this, skill: i32| {
            with_ctx(|ctx| Ok(ctx.get_player_effective_skill(this.0, skill)))
        });

        // `player:isPzLocked()` — `luaPlayerIsPzLocked`. 772: protection-zone lock round.
        methods.add_method("isPzLocked", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.player_is_pz_locked(this.0)))
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
        methods.add_method(
            "setStorageValue",
            |_, this, (key, value): (Value, Value)| {
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
            },
        );

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

        // `player:removeMoney(amount)` — TFS `luaPlayerRemoveMoney` / inventory coins.
        methods.add_method("removeMoney", |_, this, amount: u64| {
            crate::lua_mutation::call_lua_remove_money(this.0, amount).map_err(mlua::Error::runtime)
        });

        // `player:setBankBalance(balance)` — TFS `luaPlayerSetBankBalance`.
        methods.add_method("setBankBalance", |_, this, balance: u64| {
            crate::lua_mutation::call_lua_set_bank_balance(this.0, balance)
                .map_err(mlua::Error::runtime)?;
            Ok(())
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
        // `computeDamage` / `computeHealing` (native userdata; `MechanicsProfile`).
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
        methods.add_method("computeDamage", |_, this, args: mlua::Variadic<Value>| {
            let (damage, variation, limit_min, limit_max) = parse_compute_damage_args(&args)?;
            with_ctx(|ctx| {
                let (lo, hi) =
                    ctx.compute_magic_damage_range(this.0, damage, variation, limit_min, limit_max);
                Ok((-lo, -hi))
            })
        });

        // `player:computeHealing(...)` — same formula, positive magnitudes.
        methods.add_method("computeHealing", |_, this, args: mlua::Variadic<Value>| {
            let (damage, variation, limit_min, limit_max) = parse_compute_damage_args(&args)?;
            with_ctx(|ctx| {
                Ok(ctx.compute_magic_damage_range(this.0, damage, variation, limit_min, limit_max))
            })
        });

        // `player:computeSkillDamage(damage, variation, skill[, limitMinimum[, limitMaximum]])`
        // — magic formula then `× level / 25` (`magic.cc` berserk / pack skill formula).
        methods.add_method(
            "computeSkillDamage",
            |_, this, args: mlua::Variadic<Value>| {
                let (damage, variation, _skill, limit_min, limit_max) =
                    parse_compute_skill_damage_args(&args)?;
                with_ctx(|ctx| {
                    let level = ctx.get_player_level(this.0).unwrap_or(0);
                    let (lo, hi) = ctx.compute_magic_damage_range(
                        this.0, damage, variation, limit_min, limit_max,
                    );
                    let lo = (lo * level) / 25;
                    let hi = (hi * level) / 25;
                    Ok((-lo, -hi))
                })
            },
        );

        // Pack `Player:conjureItem` — native; `formulas.conjureFromHandsOnly`.
        methods.add_method("conjureItem", |_, this, args: mlua::Variadic<Value>| {
            let request = parse_conjure_item_args(this.0, &args)?;
            call_lua_conjure_item(request).map_err(mlua::Error::runtime)
        });

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

        // `creature:addHealth(healthChange)` — TFS `luaCreatureAddHealth`.
        // E4: HP clamp like `addMana`. 772 `Heal` in `DrinkPotion` (`magic.cc:2086`).
        methods.add_method("addHealth", |_, this, health_change: i32| {
            call_lua_add_health(this.0, health_change).map_err(mlua::Error::runtime)?;
            Ok(true)
        });

        // `creature:say(text[, type])` — E5. Viewport broadcast; does **not**
        // parse spells. Default `TALKTYPE_SAY` (1). 772 `Talk`; TFS `luaCreatureSay`.
        methods.add_method(
            "say",
            |_, this, (text, speak_type): (String, Option<u8>)| {
                let speak_type = speak_type.unwrap_or(1);
                call_lua_player_say(this.0, text, speak_type).map_err(mlua::Error::runtime)?;
                Ok(true)
            },
        );

        // `player:showTextDialog(itemId, text)` — E6. `0x96` window.
        // TFS `luaPlayerShowTextDialog`; 772 `SendEditText`.
        methods.add_method(
            "showTextDialog",
            |_, this, (item_id, text): (u16, String)| {
                call_lua_show_text_dialog(this.0, item_id, text).map_err(mlua::Error::runtime)?;
                Ok(true)
            },
        );

        // `player:hasLearnedSpell(name)` — E6. TFS `luaPlayerHasLearnedSpell`;
        // 772 `SpellKnown` (`crplayer.cc:1130`) via `player_spells`.
        methods.add_method("hasLearnedSpell", |_, this, name: String| {
            with_ctx(|ctx| Ok(ctx.player_has_learned_spell(this.0, &name)))
        });

        // `player:getInstantSpells()` — TFS `luaPlayerGetInstantSpells`.
        // Learn/vocation arm only (no IGNORE_SPELL_CHECK). 772 GetSpellbook.
        methods.add_method("getInstantSpells", |lua, this, ()| {
            let spells = with_ctx(|ctx| Ok(ctx.list_player_instant_spells(this.0)))?;
            let t = lua.create_table_with_capacity(spells.len(), 0)?;
            for (i, spell) in spells.into_iter().enumerate() {
                let row = lua.create_table_with_capacity(0, 6)?;
                row.set("name", spell.name)?;
                row.set("words", spell.words)?;
                row.set("level", spell.level)?;
                row.set("mlevel", spell.magic_level)?;
                row.set("mana", spell.mana)?;
                row.set("manapercent", spell.mana_percent)?;
                t.set(i + 1, row)?;
            }
            Ok(t)
        });

        // `player:addManaSpent(amount)` — `luascript.cpp` `luaPlayerAddManaSpent`.
        // PC-3a Phase 5: advances magic level for dual-hand second conjure.
        methods.add_method("addManaSpent", |_, this, amount: u64| {
            call_lua_add_mana_spent(this.0, amount).map_err(mlua::Error::runtime)?;
            Ok(true)
        });

        // `player:addSkillTries(skillType, tries)` — `luaPlayerAddSkillTries`.
        // Returns `true` / `nil` like C++. Does not apply `rateSkill` (wrapper in player.lua).
        methods.add_method("addSkillTries", |_, this, (skill, tries): (i32, u64)| {
            match call_lua_add_skill_tries(this.0, skill, tries) {
                Ok(Some(true)) => Ok(Value::Boolean(true)),
                Ok(_) => Ok(Value::Nil),
                Err(e) => Err(mlua::Error::runtime(e)),
            }
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

        // `player:getTown()` — `luaPlayerGetTown`. Returns `Town` userdata
        // wrapping `players.town_id`, or `nil` if not a player.
        methods.add_method("getTown", |lua, this, ()| {
            let town_id = with_ctx(|ctx| Ok(ctx.get_player_town_id(this.0)))?;
            match town_id {
                Some(id) if id >= 0 => {
                    let ud = lua.create_userdata(TownRef(id as u32))?;
                    Ok(Value::UserData(ud))
                }
                _ => Ok(Value::Nil),
            }
        });

        // `player:setTown(town)` — `luaPlayerSetTown`. Town userdata only.
        // `false` if arg is not Town; `nil` if self is not a live player;
        // `true` after assigning `town_id`. No teleport (`Player::setTown`).
        methods.add_method("setTown", |_, this, town: Value| {
            let town_id = match town {
                Value::UserData(ud) => match ud.borrow::<TownRef>() {
                    Ok(t) => t.0,
                    Err(_) => return Ok(Value::Boolean(false)),
                },
                _ => return Ok(Value::Boolean(false)),
            };
            match crate::lua_mutation::call_lua_set_town(this.0, town_id) {
                Ok(()) => Ok(Value::Boolean(true)),
                Err(_) => Ok(Value::Nil),
            }
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

        // `player:setVocation(vocation)` — TFS `luaPlayerSetVocation`.
        methods.add_method("setVocation", |_, this, voc: Value| {
            let vocation_id = match voc {
                Value::UserData(ud) => match ud.borrow::<VocationRef>() {
                    Ok(v) => v.0,
                    Err(_) => return Ok(Value::Boolean(false)),
                },
                Value::Integer(n) => n as i32,
                Value::Number(n) => n as i32,
                _ => return Ok(Value::Boolean(false)),
            };
            match call_lua_set_vocation(this.0, vocation_id) {
                Ok(true) => Ok(Value::Boolean(true)),
                Ok(false) => Ok(Value::Boolean(false)),
                Err(_) => Ok(Value::Nil),
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
                        Some(iid) => push_item_userdata(lua, iid),
                        None => Ok(Value::Nil),
                    }
                } else {
                    call_lua_add_item(this.0, item_type, count.min(u16::MAX as u32) as u16)
                        .map_err(mlua::Error::runtime)?;
                    Ok(Value::Nil)
                }
            },
        );

        // `player:addItemEx(item[, canDropOnMap[, slot/index[, flags]]])` — `luaPlayerAddItemEx`.
        // Default `canDropOnMap = false` (`luascript.cpp:9311`). Returns `RETURNVALUE_*`.
        methods.add_method(
            "addItemEx",
            |_,
             this,
             (item, can_drop, slot, flags): (Value, Option<bool>, Option<i32>, Option<u32>)| {
                let Some(item_id) = crate::userdata::item::item_script_id_from_value(&item) else {
                    return Ok(Value::Boolean(false));
                };
                let parent = with_ctx(|ctx| Ok(ctx.get_item_parent(item_id)))?;
                if parent.is_some() {
                    return Ok(Value::Boolean(false));
                }
                let rv = crate::lua_mutation::call_lua_add_item_ex(
                    item_id,
                    crate::lua_mutation::LuaMoveDestination::Player {
                        creature_id: this.0,
                    },
                    can_drop.unwrap_or(false),
                    slot.unwrap_or(-1),
                    flags.unwrap_or(0),
                )
                .map_err(mlua::Error::runtime)?;
                Ok(Value::Integer(i64::from(rv)))
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
                // TFS `luaPlayerRemoveItem` — `pushBoolean(removeItemOfType(...))`.
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

        // `player:getDepotLocker(depotId[, autoCreate])` — `Player::getDepotLocker`
        // (`player.cpp:826`). Used by `data/lib/core/player.lua` `getDepotItems` and
        // `data/scripts/movements/other/tiles.lua`. Always auto-creates (matches
        // data-pack usage `getDepotLocker(depotId, true)`).
        methods.add_method(
            "getDepotLocker",
            |lua, this, (depot_id, _auto_create): (u32, Option<bool>)| {
                let id_opt =
                    call_lua_get_depot_locker(this.0, depot_id).map_err(mlua::Error::runtime)?;
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

        // TFS `Thing::isTile` — inventory cylinders are the player, not a tile.
        methods.add_method("isTile", |_, _this, ()| Ok(false));

        // `creature:isInGhostMode()` — `Creature::isInGhostMode` (`creature.h`):
        // `false` for non-players; `Player::isInGhostMode` returns `ghostMode`
        // (`player.h:363`). Used by `tiles.lua` step events (`luascript.cpp:7515`).
        methods.add_method("isInGhostMode", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.is_creature_in_ghost_mode(this.0)))
        });

        // `player:getIp()` — TFS `luaPlayerGetIp` returns packed `uint32`.
        methods.add_method("getIp", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.get_player_ip(this.0)))
        });

        // `player:setGhostMode(bool)` — TFS `luaPlayerSetGhostMode`.
        methods.add_method("setGhostMode", |_, this, enabled: bool| {
            call_lua_set_ghost_mode(this.0, enabled).map_err(mlua::Error::runtime)?;
            Ok(())
        });

        // `player:popupFYI(text)` — 772 has no FYI opcode; `0x96` letter dialog (item 1950).
        methods.add_method("popupFYI", |_, this, text: String| {
            call_lua_show_text_dialog(this.0, 1950, text).map_err(mlua::Error::runtime)?;
            Ok(true)
        });

        // `creature:remove()` — TFS `luaCreatureRemove` (kick / despawn; not `item:remove`).
        methods.add_method("remove", |_, this, ()| {
            call_lua_creature_remove(this.0).map_err(mlua::Error::runtime)?;
            Ok(true)
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

        // `creature:setDirection(dir)` — TFS `luaCreatureSetDirection`.
        methods.add_method("setDirection", |_, this, dir: u8| {
            call_lua_set_direction(this.0, dir).map_err(mlua::Error::runtime)
        });

        // `creature:getMaxHealth()` — TFS `luaCreatureGetMaxHealth`.
        methods.add_method("getMaxHealth", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_creature_max_health(this.0)
                    .ok_or_else(|| mlua::Error::runtime("creature not found"))
            })
        });

        // `player:getLastLoginSaved()` — previous `players.lastlogin` unix seconds.
        methods.add_method("getLastLoginSaved", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_player_last_login_saved(this.0)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        // `player:getLastLogout()` — `players.lastlogout`.
        methods.add_method("getLastLogout", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_player_last_logout(this.0)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        // `player:getSex()` — `PLAYERSEX_FEMALE` / `PLAYERSEX_MALE`.
        methods.add_method("getSex", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_player_sex(this.0)
                    .ok_or_else(|| mlua::Error::runtime("player not found"))
            })
        });

        // `player:getOutfit()` — table with lookType / colours (`luaPlayerGetOutfit`).
        methods.add_method("getOutfit", |lua, this, ()| {
            let outfit = with_ctx(|ctx| Ok(ctx.get_player_outfit(this.0)))?;
            match outfit {
                Some(o) => {
                    let t = lua.create_table()?;
                    t.set("lookType", o.look_type)?;
                    t.set("lookHead", o.look_head)?;
                    t.set("lookBody", o.look_body)?;
                    t.set("lookLegs", o.look_legs)?;
                    t.set("lookFeet", o.look_feet)?;
                    t.set("lookAddons", o.look_addons)?;
                    Ok(Value::Table(t))
                }
                None => Ok(Value::Nil),
            }
        });

        // `player:setOutfit(outfit)` — TFS `luaPlayerSetOutfit`.
        methods.add_method("setOutfit", |_, this, outfit: mlua::Table| {
            let look_type: i32 = table_i32(&outfit, "lookType")?.unwrap_or(0);
            let look_head: i32 = table_i32(&outfit, "lookHead")?.unwrap_or(0);
            let look_body: i32 = table_i32(&outfit, "lookBody")?.unwrap_or(0);
            let look_legs: i32 = table_i32(&outfit, "lookLegs")?.unwrap_or(0);
            let look_feet: i32 = table_i32(&outfit, "lookFeet")?.unwrap_or(0);
            let look_addons: i32 = table_i32(&outfit, "lookAddons")?.unwrap_or(0);
            call_lua_set_outfit(
                this.0,
                look_type,
                look_head,
                look_body,
                look_legs,
                look_feet,
                look_addons,
            )
            .map_err(mlua::Error::runtime)?;
            Ok(true)
        });

        // `player:sendOutfitWindow()` — TFS `luaPlayerSendOutfitWindow` (`0xC8`).
        methods.add_method("sendOutfitWindow", |_, this, ()| {
            call_lua_send_outfit_window(this.0).map_err(mlua::Error::runtime)?;
            Ok(true)
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

        // `creature:getMaster()` — `luaCreatureGetMaster`. Nil if wild / no
        // live master. Unified `CreatureRef` (Player methods via index chain).
        methods.add_method("getMaster", |lua, this, ()| {
            let master = with_ctx(|ctx| Ok(ctx.get_creature_master(this.0)))?;
            match master {
                Some(id) => {
                    let ud = lua.create_userdata(CreatureRef(id))?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
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
            crate::lua_mutation::call_add_summon(this.0, summon_id)
                .map_err(mlua::Error::runtime)?;
            Ok(true)
        });

        // `monster:getType()` → MonsterType(name).
        methods.add_method("getType", |lua, this, ()| {
            let name = with_ctx(|ctx| Ok(ctx.get_creature_monster_type_name(this.0)))?;
            match name {
                Some(n) => {
                    let ud =
                        lua.create_userdata(crate::userdata::monster_type::MonsterTypeRef {
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
                crate::lua_mutation::call_creature_teleport(this.0, x, y, z, push.unwrap_or(false))
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

        // `player:registerEvent(name)` — TFS `luaPlayerRegisterEvent` (`luascript.cpp`).
        methods.add_method("registerEvent", |lua, this, name: String| {
            if crate::creature_events::is_blocked_creature_event_name(&name) {
                tracing::warn!(
                    name = %name,
                    "registerEvent ignored (DropLoot/RegenerateStamina are not registrable)"
                );
                return Ok(false);
            }
            let known = lua
                .globals()
                .get::<Option<mlua::Table>>("_creature_event_registry")
                .ok()
                .flatten()
                .and_then(|t| t.get::<Option<bool>>(name.as_str()).ok().flatten())
                .unwrap_or(false);
            if !known {
                tracing::warn!(name = %name, "registerEvent: unknown CreatureEvent");
                return Ok(false);
            }
            call_player_register_creature_event(this.0, name, true).map_err(mlua::Error::runtime)
        });

        methods.add_method("unregisterEvent", |_, this, name: String| {
            call_player_register_creature_event(this.0, name, false).map_err(mlua::Error::runtime)
        });

        // `creature:getTile()` — TFS `luaCreatureGetTile`.
        methods.add_method("getTile", |lua, this, ()| {
            let pos = crate::context::current_ctx(|ctx| ctx.get_player_position(this.0)).flatten();
            match pos {
                Some(p) => {
                    let ud = lua.create_userdata(crate::userdata::tile::TileRef {
                        x: p.x,
                        y: p.y,
                        z: p.z,
                    })?;
                    Ok(Value::UserData(ud))
                }
                None => Ok(Value::Nil),
            }
        });

        // `player:setEditHouse(house, listId)` — TFS `luaPlayerSetEditHouse`.
        methods.add_method("setEditHouse", |_, this, (house, list_id): (Value, u32)| {
            let house_id = match house {
                Value::UserData(ud) => ud.borrow::<crate::userdata::HouseRef>()?.0,
                _ => return Ok(false),
            };
            crate::lua_mutation::call_player_set_edit_house(this.0, house_id, list_id)
                .map_err(mlua::Error::runtime)
        });

        // `player:sendHouseWindow(house, listId)` — TFS `luaPlayerSendHouseWindow`.
        methods.add_method(
            "sendHouseWindow",
            |_, this, (house, list_id): (Value, u32)| {
                let house_id = match house {
                    Value::UserData(ud) => ud.borrow::<crate::userdata::HouseRef>()?.0,
                    _ => return Ok(false),
                };
                crate::lua_mutation::call_player_send_house_window(this.0, house_id, list_id)
                    .map_err(mlua::Error::runtime)
            },
        );

        // `player:setInFight(bool)` — PC-3a Phase 3 (`poison_storm.lua`).
        methods.add_method("setInFight", |_, this, in_fight: bool| {
            call_lua_set_in_fight(this.0, in_fight).map_err(mlua::Error::runtime)?;
            Ok(())
        });

        // `__index` fallback — bridges `function Player:method(...)` /
        // `function Creature:method(...)` definitions from
        // `data/lib/core/{player,creature}.lua` and `data/scripts/functions.lua`
        // onto `CreatureRef` userdata. mlua only invokes `__index` after the
        // registered-method lookup misses, so native Rust methods take priority.
        //
        // Gap 7b: the chain is `Player` → `Creature` (first hit wins). The
        // previous hardcoded `"Player"` fallback silently missed all 15 methods
        // in `data/lib/core/creature.lua` (`getPlayer`, `isPlayer`,
        // `setMonsterOutfit`, `addSummon`, `addDamageCondition`, `canAccessPz`,
        // …) plus `functions.lua` `Creature:addAttributeCondition` — a
        // latent bug independent of the tools work.
        //
        // C++ reference: `luascript.cpp` `LuaScriptInterface::registerClass`
        // chains `Player` → `Creature`; shared helper in `class_registry`.
        methods.add_meta_method(MetaMethod::Index, |lua, _this, key: mlua::LuaString| {
            crate::class_registry::class_index_lookup(
                lua,
                crate::class_registry::CREATURE_INDEX_CHAIN,
                key,
            )
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

/// `creature:conjureItem(mana|spell, reagentId, conjureId[, count[, effect]])`.
fn parse_conjure_item_args(
    player: u64,
    args: &mlua::Variadic<Value>,
) -> Result<ConjureRequest, mlua::Error> {
    let mana_cost = match args.first() {
        Some(Value::UserData(ud)) => match ud.borrow::<SpellBuilder>() {
            Ok(spell) => spell.spell.borrow().mana as i32,
            Err(_) => 0,
        },
        Some(v) => value_as_i32(v).unwrap_or(0),
        None => 0,
    };
    let reagent_id = args.get(1).and_then(value_as_u16).unwrap_or(0);
    let conjure_id = args.get(2).and_then(value_as_u16).unwrap_or(0);
    let conjure_count = match args.get(3) {
        None | Some(Value::Nil) => None,
        Some(v) => value_as_i32(v).map(|n| n.max(0) as u32),
    };
    let effect = args
        .get(4)
        .and_then(value_as_i32)
        .map(|n| n.clamp(0, 255) as u8)
        .unwrap_or(14); // CONST_ME_MAGIC_RED
    Ok(ConjureRequest {
        player,
        mana_cost,
        reagent_id,
        conjure_id,
        conjure_count,
        effect,
    })
}

fn value_as_u16(v: &Value) -> Option<u16> {
    value_as_i32(v).and_then(|n| u16::try_from(n).ok())
}

fn table_i32(table: &mlua::Table, key: &str) -> Result<Option<i32>, mlua::Error> {
    match table.get::<Value>(key)? {
        Value::Nil => Ok(None),
        Value::Integer(n) => Ok(Some(n as i32)),
        Value::Number(n) => Ok(Some(n as i32)),
        _ => Err(mlua::Error::runtime(format!(
            "outfit.{key}: expected integer"
        ))),
    }
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
/// - `integer` — a `RETURNVALUE_*` enum code; mapped via [`return_value_message`]
///   (`tools.cpp` `getReturnMessage`).
///
/// C++ reference: `enums.h` `ReturnValue_t` (772 numbering).
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

/// Map a 772 `RETURNVALUE_*` integer to `getReturnMessage` (`tools.cpp:982-1186`).
/// Unlisted codes (including `NOERROR` / `NOTPOSSIBLE`) use the C++ `default` arm.
pub(crate) fn return_value_message(code: i32) -> String {
    match code {
        2 | 13 => "There is not enough room.".to_string(),
        3 => "You can not enter a protection zone after attacking another player.".to_string(),
        4 => "You are not invited.".to_string(),
        5 => "You cannot throw there.".to_string(),
        6 => "There is no way.".to_string(),
        7 => "Destination is out of range.".to_string(),
        9 => "You cannot move this object.".to_string(),
        10 => "Drop the double-handed object first.".to_string(),
        11 => "Both hands have to be free.".to_string(),
        12 => "You may only use one weapon.".to_string(),
        14 => "You cannot dress this object there.".to_string(),
        15 => "Put this object in your hand.".to_string(),
        16 => "Put this object in both hands.".to_string(),
        17 => "You are too far away.".to_string(),
        18 => "First go downstairs.".to_string(),
        19 => "First go upstairs.".to_string(),
        20 => "You cannot put more objects in this container.".to_string(),
        21 => "This object is too heavy for you to carry.".to_string(),
        22 => "You cannot take this object.".to_string(),
        23 => "This is impossible.".to_string(),
        24 => "You cannot put more items in this depot.".to_string(),
        25 => "Creature does not exist.".to_string(),
        26 => "You cannot use this object.".to_string(),
        27 => "A player with this name is not online.".to_string(),
        28 => "You are already trading. Finish this trade first.".to_string(),
        29 => "This player is already trading.".to_string(),
        30 => "You may not logout during or immediately after a fight!".to_string(),
        31 => "You are not allowed to shoot directly on players.".to_string(),
        32 => "Your level is too low.".to_string(),
        33 => "You do not have enough magic level.".to_string(),
        34 => "You do not have enough mana.".to_string(),
        35 => "You do not have enough soulpoints.".to_string(),
        36 => "You are exhausted.".to_string(),
        37 => "You cannot use objects that fast.".to_string(),
        38 => "Player is not reachable.".to_string(),
        39 => "You can only use it on creatures.".to_string(),
        40 => "This action is not permitted in a protection zone.".to_string(),
        41 => "You may not attack this person.".to_string(),
        42 => "You may not attack a person in a protection zone.".to_string(),
        43 => "You may not attack a person while you are in a protection zone.".to_string(),
        44 => "You may not attack this creature.".to_string(),
        45 => "You can only use it on creatures.".to_string(),
        46 => "Creature is not reachable.".to_string(),
        47 => "Turn secure mode off if you really want to attack unmarked players.".to_string(),
        48 => "You need a premium account.".to_string(),
        49 => "You need to learn this spell first.".to_string(),
        50 => "Your vocation cannot use this spell.".to_string(),
        51 => "You need to equip a weapon to use this spell.".to_string(),
        52 => "You can not leave a pvp zone after attacking another player.".to_string(),
        53 => "You can not enter a pvp zone after attacking another player.".to_string(),
        54 => "This action is not permitted in a non pvp zone.".to_string(),
        55 => "You can not logout here.".to_string(),
        56 => "You need a magic item to cast this spell.".to_string(),
        57 => "Player name is ambiguous.".to_string(),
        58 => "You may use only one shield.".to_string(),
        59 => "No party members in range.".to_string(),
        60 => "You are not the owner.".to_string(),
        61 => "No such raid exists.".to_string(),
        62 => "Another raid is already executing.".to_string(),
        63 => "Trade player is too far away.".to_string(),
        64 => "You don't own this house.".to_string(),
        65 => "Trade player already owns a house.".to_string(),
        66 => "Trade player is currently the highest bidder of an auctioned house.".to_string(),
        67 => "You can not trade this house.".to_string(),
        68 => "You don't have the required profession.".to_string(),
        69 => "This item cannot be moved there.".to_string(),
        // `RETURNVALUE_NOERROR` (0), `NOTPOSSIBLE` (1), `CREATUREBLOCK` (8), …
        _ => "Sorry, not possible.".to_string(),
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

        fn get_player_effective_skill(&self, id: ScriptCreatureId, skill: i32) -> Option<i32> {
            (id == GM_CID && (0..=6).contains(&skill)).then_some(10)
        }

        fn player_is_pz_locked(&self, id: ScriptCreatureId) -> Option<bool> {
            (id == GM_CID).then_some(false)
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

        fn player_has_learned_spell(&self, id: ScriptCreatureId, name: &str) -> bool {
            id == GM_CID && name.eq_ignore_ascii_case("Light Healing")
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

            let fishing: Option<i32> = lua
                .load("return player:getEffectiveSkillLevel(6)")
                .eval()
                .expect("getEffectiveSkillLevel");
            assert_eq!(fishing, Some(10));

            let pz: Option<bool> = lua
                .load("return player:isPzLocked()")
                .eval()
                .expect("isPzLocked");
            assert_eq!(pz, Some(false));

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

            let learned: bool = lua
                .load("return player:hasLearnedSpell('Light Healing')")
                .eval()
                .expect("hasLearnedSpell known");
            assert!(learned);
            let unlearned: bool = lua
                .load("return player:hasLearnedSpell('Berserk')")
                .eval()
                .expect("hasLearnedSpell unknown");
            assert!(!unlearned);
        });
    }

    thread_local! {
        static CAPTURED_SAY: std::cell::RefCell<Option<(u64, String, u8)>> =
            const { std::cell::RefCell::new(None) };
    }

    fn capture_say_applier(
        _: *mut (),
        mutation: crate::lua_mutation::LuaMutation,
    ) -> Result<(), String> {
        if let crate::lua_mutation::LuaMutation::PlayerSay {
            creature_id,
            text,
            speak_type,
        } = mutation
        {
            CAPTURED_SAY.with(|c| *c.borrow_mut() = Some((creature_id, text, speak_type)));
        }
        Ok(())
    }

    /// E5: `player:say` broadcasts via mutation; does not go through `player_say`.
    #[test]
    fn e5_player_say_emits_viewport_say_mutation() {
        CAPTURED_SAY.with(|c| *c.borrow_mut() = None);
        crate::lua_mutation::register_lua_mutation_applier(capture_say_applier);

        let lua = Lua::new();
        crate::userdata::register_creature_metatable(&lua).expect("creature");
        crate::lua_mutation::with_lua_mutation_scope(std::ptr::without_provenance_mut(1), || {
            with_lua_context(&GmPlayerCtx, || {
                let ud = lua.create_userdata(CreatureRef(GM_CID)).expect("ud");
                lua.globals().set("player", ud).unwrap();
                let ok: bool = lua
                    .load("return player:say('Mmmh.')")
                    .eval()
                    .expect("say default");
                assert!(ok);
            });
        });
        let (cid, text, speak) = CAPTURED_SAY.with(|c| c.borrow().clone()).expect("mutation");
        assert_eq!(cid, GM_CID);
        assert_eq!(text, "Mmmh.");
        assert_eq!(speak, 1, "default TALKTYPE_SAY");

        CAPTURED_SAY.with(|c| *c.borrow_mut() = None);
        crate::lua_mutation::with_lua_mutation_scope(std::ptr::without_provenance_mut(1), || {
            with_lua_context(&GmPlayerCtx, || {
                let _: bool = lua
                    .load("return player:say('Urgh!', 17)")
                    .eval()
                    .expect("say type");
            });
        });
        assert_eq!(
            CAPTURED_SAY.with(|c| c.borrow().as_ref().map(|t| t.2)),
            Some(17)
        );
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

    /// `return_value_message` matches TVP `getReturnMessage` (`tools.cpp:982`).
    #[test]
    fn return_value_message_maps_known_codes() {
        use super::return_value_message;
        assert_eq!(
            return_value_message(27),
            "A player with this name is not online."
        );
        assert_eq!(return_value_message(1), "Sorry, not possible.");
        assert_eq!(return_value_message(2), "There is not enough room.");
        assert_eq!(return_value_message(36), "You are exhausted.");
        // C++ `default` — `NOERROR` (0) and unknown codes.
        assert_eq!(return_value_message(0), "Sorry, not possible.");
        assert_eq!(return_value_message(999), "Sorry, not possible.");
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

    /// R4: shipped `Player()` ctor accepts userdata and numeric id (`luaPlayerCreate`).
    #[test]
    fn player_constructor_accepts_userdata_and_id() {
        use crate::context::with_lua_context;
        use tfs_rust_common::{
            ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptItemId, ScriptItemRef,
        };

        struct CtorCtx;
        impl ScriptContext for CtorCtx {
            fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
                (id == 42).then_some(ScriptCreatureData {
                    name: "OnlinePlayer".into(),
                    guid: 42,
                })
            }
            fn get_item(&self, _: ScriptItemId) -> Option<ScriptItemRef> {
                None
            }
            fn get_config_string(&self, _: &str) -> Option<String> {
                None
            }
            fn get_player_by_name(&self, name: &str) -> Option<ScriptCreatureId> {
                (name == "OnlinePlayer").then_some(42)
            }
            fn get_player_level(&self, id: ScriptCreatureId) -> Option<i32> {
                (id == 42).then_some(8)
            }
        }

        let runtime = crate::runtime::LuaRuntime::new().expect("runtime");
        let lua = &runtime.lua;
        let p = lua.create_userdata(CreatureRef(42)).expect("p");
        lua.globals().set("p", p).unwrap();
        with_lua_context(&CtorCtx, || {
            let via_ud: u64 = lua
                .load("return Player(p):getId()")
                .eval()
                .expect("Player(userdata)");
            assert_eq!(via_ud, 42);
            let via_id: u64 = lua
                .load("return Player(42):getId()")
                .eval()
                .expect("Player(id)");
            assert_eq!(via_id, 42);
            let nobody: mlua::Value = lua.load("return Player(99)").eval().expect("Player(99)");
            assert!(matches!(nobody, mlua::Value::Nil));
        });
    }

    #[test]
    fn creature_constructor_resolves_by_name() {
        use crate::context::with_lua_context;
        use tfs_rust_common::{
            ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptItemId, ScriptItemRef,
        };

        struct CtorCtx;
        impl ScriptContext for CtorCtx {
            fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
                (id == 7).then_some(ScriptCreatureData {
                    name: "Demon".into(),
                    guid: 0,
                })
            }
            fn get_item(&self, _: ScriptItemId) -> Option<ScriptItemRef> {
                None
            }
            fn get_config_string(&self, _: &str) -> Option<String> {
                None
            }
            fn get_creature_by_name(&self, name: &str) -> Option<ScriptCreatureId> {
                name.eq_ignore_ascii_case("demon").then_some(7)
            }
        }

        let runtime = crate::runtime::LuaRuntime::new().expect("runtime");
        let lua = &runtime.lua;
        with_lua_context(&CtorCtx, || {
            let id: u64 = lua
                .load("return Creature('demon'):getId()")
                .eval()
                .expect("Creature(name)");
            assert_eq!(id, 7);
            let missing: mlua::Value = lua.load("return Creature('nope')").eval().expect("missing");
            assert!(matches!(missing, mlua::Value::Nil));
        });
    }

    thread_local! {
        static CAPTURED_TOWN: std::cell::RefCell<Option<(u64, u32)>> =
            const { std::cell::RefCell::new(None) };
    }

    fn capture_set_town_applier(
        _: *mut (),
        mutation: crate::lua_mutation::LuaMutation,
    ) -> Result<(), String> {
        if let crate::lua_mutation::LuaMutation::PlayerSetTown {
            creature_id,
            town_id,
        } = mutation
        {
            CAPTURED_TOWN.with(|c| *c.borrow_mut() = Some((creature_id, town_id)));
        }
        Ok(())
    }

    /// M3: `player:setTown(Town("Thais"))` mutates town id; bad arg is `false`.
    #[test]
    fn m3_set_town_emits_mutation() {
        CAPTURED_TOWN.with(|c| *c.borrow_mut() = None);
        crate::lua_mutation::register_lua_mutation_applier(capture_set_town_applier);

        let runtime = crate::runtime::LuaRuntime::new().expect("runtime");
        let lua = &runtime.lua;
        let p = lua.create_userdata(CreatureRef(7)).expect("player");
        lua.globals().set("player", p).unwrap();

        struct TownSetCtx;
        impl ScriptContext for TownSetCtx {
            fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
                (id == 7).then_some(ScriptCreatureData {
                    name: "Gm".into(),
                    guid: 7,
                })
            }
            fn get_item(&self, _: ScriptItemId) -> Option<ScriptItemRef> {
                None
            }
            fn get_config_string(&self, _: &str) -> Option<String> {
                None
            }
            fn get_town_by_id(&self, town_id: u32) -> Option<tfs_rust_common::ScriptTownData> {
                (town_id == 1).then_some(tfs_rust_common::ScriptTownData {
                    id: 1,
                    name: "Thais".into(),
                    temple: tfs_rust_common::Position::new(32369, 32241, 7),
                })
            }
            fn get_town_by_name(&self, name: &str) -> Option<tfs_rust_common::ScriptTownData> {
                name.eq_ignore_ascii_case("Thais")
                    .then_some(tfs_rust_common::ScriptTownData {
                        id: 1,
                        name: "Thais".into(),
                        temple: tfs_rust_common::Position::new(32369, 32241, 7),
                    })
            }
        }

        crate::lua_mutation::with_lua_mutation_scope(std::ptr::without_provenance_mut(1), || {
            with_lua_context(&TownSetCtx, || {
                let ok: bool = lua
                    .load("return player:setTown(Town('Thais'))")
                    .eval()
                    .expect("setTown Thais");
                assert!(ok);
                let bad: bool = lua
                    .load("return player:setTown(1)")
                    .eval()
                    .expect("setTown number");
                assert!(!bad);
            });
        });
        assert_eq!(CAPTURED_TOWN.with(|c| *c.borrow()), Some((7, 1)));
    }

    /// M3: `getMaster()` nil for wild; player userdata for summon.
    #[test]
    fn m3_get_master_nil_wild_player_for_summon() {
        struct MasterCtx;
        impl ScriptContext for MasterCtx {
            fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
                match id {
                    1 => Some(ScriptCreatureData {
                        name: "Hero".into(),
                        guid: 1,
                    }),
                    2 => Some(ScriptCreatureData {
                        name: "Bear".into(),
                        guid: 2,
                    }),
                    3 => Some(ScriptCreatureData {
                        name: "Rat".into(),
                        guid: 3,
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
            fn is_creature_player(&self, id: ScriptCreatureId) -> bool {
                id == 1
            }
            fn get_creature_master(&self, id: ScriptCreatureId) -> Option<ScriptCreatureId> {
                (id == 2).then_some(1)
            }
        }

        let lua = Lua::new();
        crate::userdata::register_creature_metatable(&lua).expect("creature");
        with_lua_context(&MasterCtx, || {
            let summon = lua.create_userdata(CreatureRef(2)).expect("summon");
            let wild = lua.create_userdata(CreatureRef(3)).expect("wild");
            lua.globals().set("summon", summon).unwrap();
            lua.globals().set("wild", wild).unwrap();
            let wild_master: mlua::Value = lua
                .load("return wild:getMaster()")
                .eval()
                .expect("wild master");
            assert!(matches!(wild_master, mlua::Value::Nil));
            let master_id: u64 = lua
                .load("return summon:getMaster():getId()")
                .eval()
                .expect("summon master");
            assert_eq!(master_id, 1);
            let is_player: bool = lua
                .load("return summon:getMaster():isPlayer()")
                .eval()
                .expect("master isPlayer");
            assert!(is_player);
        });
    }

    /// Phase 3: login/firstlogin primitives through the userdata bindings.
    #[test]
    fn phase3_login_primitives_through_lua() {
        use tfs_rust_common::ScriptOutfit;

        struct LoginCtx;
        impl ScriptContext for LoginCtx {
            fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
                (id == GM_CID).then_some(ScriptCreatureData {
                    name: "Hero".into(),
                    guid: 1,
                })
            }
            fn get_item(&self, _: ScriptItemId) -> Option<ScriptItemRef> {
                None
            }
            fn get_config_string(&self, _: &str) -> Option<String> {
                None
            }
            fn get_player_last_login_saved(&self, id: ScriptCreatureId) -> Option<i64> {
                (id == GM_CID).then_some(1_700_000_000)
            }
            fn get_player_last_logout(&self, id: ScriptCreatureId) -> Option<i64> {
                (id == GM_CID).then_some(1_699_000_000)
            }
            fn get_player_ip(&self, id: ScriptCreatureId) -> u32 {
                if id == GM_CID {
                    u32::from_le_bytes([192, 168, 1, 10])
                } else {
                    0
                }
            }
            fn get_player_sex(&self, id: ScriptCreatureId) -> Option<u8> {
                (id == GM_CID).then_some(1)
            }
            fn get_player_outfit(&self, id: ScriptCreatureId) -> Option<ScriptOutfit> {
                (id == GM_CID).then_some(ScriptOutfit {
                    look_type: 128,
                    look_head: 78,
                    look_body: 106,
                    look_legs: 58,
                    look_feet: 95,
                    look_addons: 0,
                })
            }
            fn get_creature_max_health(&self, id: ScriptCreatureId) -> Option<i32> {
                (id == GM_CID).then_some(150)
            }
            fn get_outfit_info(&self, look_type: i32) -> Option<(String, bool)> {
                (look_type == 128).then_some(("Citizen".into(), false))
            }
        }

        let runtime = crate::runtime::LuaRuntime::new().expect("runtime");
        let lua = &runtime.lua;
        with_lua_context(&LoginCtx, || {
            let ud = lua.create_userdata(CreatureRef(GM_CID)).expect("ud");
            lua.globals().set("player", ud).unwrap();
            let last: i64 = lua
                .load("return player:getLastLoginSaved()")
                .eval()
                .expect("last login");
            assert_eq!(last, 1_700_000_000);
            let logout: i64 = lua
                .load("return player:getLastLogout()")
                .eval()
                .expect("last logout");
            assert_eq!(logout, 1_699_000_000);
            let sex: u8 = lua.load("return player:getSex()").eval().expect("sex");
            assert_eq!(sex, 1);
            let look: i32 = lua
                .load("return player:getOutfit().lookType")
                .eval()
                .expect("outfit");
            assert_eq!(look, 128);
            let uid: u64 = lua.load("return player.uid").eval().expect("uid");
            assert_eq!(uid, GM_CID);
            let hp: i32 = lua
                .load("return player:getMaxHealth()")
                .eval()
                .expect("maxhp");
            assert_eq!(hp, 150);
            let prem: i32 = lua
                .load("return Outfit(128).premium")
                .eval()
                .expect("Outfit");
            assert_eq!(prem, 0);
            let missing: mlua::Value = lua.load("return Outfit(1)").eval().expect("missing outfit");
            assert!(matches!(missing, mlua::Value::Nil));
            let ip: u32 = lua.load("return player:getIp()").eval().expect("getIp");
            assert_eq!(ip, u32::from_le_bytes([192, 168, 1, 10]));
        });
    }
}
