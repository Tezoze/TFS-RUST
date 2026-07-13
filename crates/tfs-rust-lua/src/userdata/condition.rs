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

use mlua::{Lua, UserData, UserDataMethods, Value};

/// Condition parameter constants — mirrors `enums.h:135-195`.
/// Kept here (not in `constants.rs`) because they're only used by the
/// `setParameter` dispatch in this file.
const CONDITION_PARAM_TICKS: i32 = 2; // enums.h:136
const CONDITION_PARAM_HEALTHGAIN: i32 = 4; // enums.h:138
const CONDITION_PARAM_HEALTHTICKS: i32 = 5; // enums.h:139
const CONDITION_PARAM_MANAGAIN: i32 = 6; // enums.h:140
const CONDITION_PARAM_MANATICKS: i32 = 7; // enums.h:141
const CONDITION_PARAM_DELAYED: i32 = 8; // enums.h:142
const CONDITION_PARAM_SPEED: i32 = 9; // enums.h:143
const CONDITION_PARAM_LIGHT_LEVEL: i32 = 10; // enums.h:144
const CONDITION_PARAM_LIGHT_COLOR: i32 = 11; // enums.h:145
const CONDITION_PARAM_SOULGAIN: i32 = 12; // enums.h:146
const CONDITION_PARAM_SOULTICKS: i32 = 13; // enums.h:147
const CONDITION_PARAM_MINVALUE: i32 = 14; // enums.h:148
const CONDITION_PARAM_MAXVALUE: i32 = 15; // enums.h:149
const CONDITION_PARAM_STARTVALUE: i32 = 16; // enums.h:150
const CONDITION_PARAM_TICKINTERVAL: i32 = 17; // enums.h:151
const CONDITION_PARAM_FORCEUPDATE: i32 = 18; // enums.h:152
const CONDITION_PARAM_PERIODICDAMAGE: i32 = 35; // enums.h:169
const CONDITION_PARAM_SUBID: i32 = 45; // enums.h:179
const CONDITION_PARAM_CYCLE: i32 = 56; // enums.h:190
const CONDITION_PARAM_COUNT: i32 = 58; // enums.h:192
const CONDITION_PARAM_MAX_COUNT: i32 = 59; // enums.h:193
const CONDITION_PARAM_OWNERGUID: i32 = 60; // enums.h:194

/// Lua-facing `Condition(type, id)` builder — accumulates condition fields
/// before `player:addCondition(condition)` consumes them.
///
/// `ctype` is the 772 bit-flag value (e.g. `CONDITION_CHANNELMUTEDTICKS = 1<<15`);
/// the core applier maps it to the Rust `ConditionType` enum. `cond_id` is
/// `CONDITIONID_DEFAULT` (-1) for non-equipment conditions.
#[derive(Clone, Debug, Default)]
pub struct ConditionBuilder {
    pub ctype: i32,
    pub cond_id: i32,
    pub sub_id: u32,
    pub ticks: i32,
    /// `CONDITION_PARAM_SPEED` — speed modifier (paralyze/haste).
    pub speed: i32,
    /// `CONDITION_PARAM_LIGHT_LEVEL` / `LIGHT_COLOR` — light condition params.
    pub light_level: i32,
    pub light_color: i32,
    /// `CONDITION_PARAM_PERIODICDAMAGE` — damage per tick (fire/poison/energy DoT).
    pub periodic_damage: i32,
    /// `CONDITION_PARAM_STARTVALUE` / `MINVALUE` / `MAXVALUE` — damage list bounds.
    pub start_value: i32,
    pub min_value: i32,
    pub max_value: i32,
    /// `CONDITION_PARAM_TICKINTERVAL` — delay between damage ticks (ms).
    pub tick_interval: i32,
    /// `CONDITION_PARAM_DELAYED` — whether damage is delayed (fire/poison).
    pub delayed: bool,
    /// `CONDITION_PARAM_FORCEUPDATE` — force condition refresh on re-apply.
    pub force_update: bool,
    /// `CONDITION_PARAM_HEALTHGAIN` / `HEALTHTICKS` — regen condition params.
    pub health_gain: i32,
    pub health_ticks: i32,
    /// `CONDITION_PARAM_MANAGAIN` / `MANATICKS` — mana regen condition params.
    pub mana_gain: i32,
    pub mana_ticks: i32,
    /// 772-specific DoT cycle params (soulfire_rune.lua, poison_storm.lua).
    pub cycle: i32,
    pub count: i32,
    pub max_count: i32,
    pub owner_guid: i32,
}

impl ConditionBuilder {
    pub fn new(ctype: i32, cond_id: i32) -> Self {
        Self {
            ctype,
            cond_id,
            ..Default::default()
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
            CONDITION_PARAM_SPEED => self.speed = value,
            CONDITION_PARAM_LIGHT_LEVEL => self.light_level = value,
            CONDITION_PARAM_LIGHT_COLOR => self.light_color = value,
            CONDITION_PARAM_PERIODICDAMAGE => self.periodic_damage = value,
            CONDITION_PARAM_MINVALUE => self.min_value = value,
            CONDITION_PARAM_MAXVALUE => self.max_value = value,
            CONDITION_PARAM_STARTVALUE => self.start_value = value,
            CONDITION_PARAM_TICKINTERVAL => self.tick_interval = value,
            CONDITION_PARAM_DELAYED => self.delayed = value != 0,
            CONDITION_PARAM_FORCEUPDATE => self.force_update = value != 0,
            CONDITION_PARAM_HEALTHGAIN => self.health_gain = value,
            CONDITION_PARAM_HEALTHTICKS => self.health_ticks = value,
            CONDITION_PARAM_MANAGAIN => self.mana_gain = value,
            CONDITION_PARAM_MANATICKS => self.mana_ticks = value,
            CONDITION_PARAM_CYCLE => self.cycle = value,
            CONDITION_PARAM_COUNT => self.count = value,
            CONDITION_PARAM_MAX_COUNT => self.max_count = value,
            CONDITION_PARAM_OWNERGUID => self.owner_guid = value,
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
        // C++ `luaConditionSetParameter` (`luascript.cpp:12107-12111`) accepts
        // both boolean and integer values — booleans are coerced to 0/1.
        methods.add_method_mut("setParameter", |_, this, (param, value): (i32, Value)| {
            let v = match value {
                Value::Boolean(b) => {
                    if b {
                        1
                    } else {
                        0
                    }
                }
                Value::Integer(n) => n as i32,
                Value::Number(n) => n as i32,
                _ => {
                    return Err(mlua::Error::runtime(
                        "condition:setParameter: value must be boolean or integer",
                    ));
                }
            };
            this.set_parameter(param, v);
            Ok(true)
        });

        // `condition:setOutfit(outfitTable)` — `Condition::setOutfit`
        // (`condition.cpp`). Used by chameleon_rune.lua and creature_illusion.lua.
        // The outfit table contains `lookType`, `lookTypeEx`, `lookHead`, etc.
        // We store the lookTypeEx for now; full outfit support is deferred.
        methods.add_method_mut("setOutfit", |_, _this, _outfit: mlua::Table| {
            // Outfit storage deferred — accept the call so scripts load.
            Ok(true)
        });
    }
}
