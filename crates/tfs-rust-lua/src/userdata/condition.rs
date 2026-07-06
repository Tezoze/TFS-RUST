//! `Condition` userdata for Lua — replaces the no-op soul stub.
//!
//! C++ reference: `luascript.cpp` `luaCreateCondition` / `Condition::setParameter`
//! / `Condition::setTicks` — `condition.cpp` / `condition.h`.
//!
//! LUA-4 §1.6: a `ConditionBuilder` accumulates `{ ctype, cond_id, sub_id,
//! ticks }` and is consumed by `player:addCondition(condition)` which reads the
//! builder's fields into `LuaMutation::PlayerAddCondition`. The
//! `setTicks` / `setParameter` methods keep `data/events/scripts/player.lua`'s
//! `soulCondition` loading unchanged (regression guard — §4.1).

use mlua::{Lua, UserData, UserDataMethods};

/// Condition parameter constants — mirrors `enums.h:135-179`.
/// Kept here (not in `constants.rs`) because they're only used by the
/// `setParameter` dispatch in this file.
const CONDITION_PARAM_TICKS: i32 = 2; // enums.h:136
const CONDITION_PARAM_SOULGAIN: i32 = 12; // enums.h:146
const CONDITION_PARAM_SOULTICKS: i32 = 13; // enums.h:147
const CONDITION_PARAM_SUBID: i32 = 45; // enums.h:179

/// Lua-facing `Condition(type, id)` builder — accumulates condition fields
/// before `player:addCondition(condition)` consumes them.
///
/// `ctype` is the 772 bit-flag value (e.g. `CONDITION_CHANNELMUTEDTICKS = 1<<15`);
/// the core applier maps it to the Rust `ConditionType` enum. `cond_id` is
/// `CONDITIONID_DEFAULT` (-1) for non-equipment conditions.
#[derive(Clone, Debug)]
pub struct ConditionBuilder {
    pub ctype: i32,
    pub cond_id: i32,
    pub sub_id: u32,
    pub ticks: i32,
}

impl ConditionBuilder {
    pub fn new(ctype: i32, cond_id: i32) -> Self {
        Self {
            ctype,
            cond_id,
            sub_id: 0,
            ticks: 0,
        }
    }

    /// `Condition::setTicks` — `condition.h`. Sets the condition duration in ms.
    pub fn set_ticks(&mut self, ticks: i32) {
        self.ticks = ticks;
    }

    /// `Condition::setParameter` — `condition.cpp`. Dispatches on the
    /// `CONDITION_PARAM_*` constant. Unknown params are silently ignored
    /// (matching C++ `default: break`).
    pub fn set_parameter(&mut self, param: i32, value: i32) {
        match param {
            CONDITION_PARAM_TICKS => self.ticks = value,
            CONDITION_PARAM_SUBID => self.sub_id = value as u32,
            // Soul params are accepted but not consumed by the channel-mute
            // path — they exist so `player.lua`'s `soulCondition` loads.
            CONDITION_PARAM_SOULGAIN | CONDITION_PARAM_SOULTICKS => {}
            _ => {}
        }
    }
}

pub fn register_condition_metatable(lua: &Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<ConditionBuilder>(|_registry| {})
}

impl UserData for ConditionBuilder {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // `condition:setTicks(ticks)` — `Condition::setTicks` (`condition.h`).
        // Used by `player.lua`'s `soulCondition` build.
        methods.add_method_mut("setTicks", |_, this, ticks: i32| {
            this.set_ticks(ticks);
            Ok(())
        });

        // `condition:setParameter(param, value)` — `Condition::setParameter`
        // (`condition.cpp`). Delegates to `ConditionBuilder::set_parameter`
        // which dispatches on the `CONDITION_PARAM_*` constant.
        methods.add_method_mut("setParameter", |_, this, (param, value): (i32, i32)| {
            this.set_parameter(param, value);
            Ok(())
        });
    }
}
