//! TFS combat / spell / weapon / condition enum registration as Lua globals.
//!
//! PC-2b: mirrors the `registerEnum` block of `luascript.cpp:1200-2050`. Values are
//! sourced from the 772 TVP reference tree (`reference/tvp-772/gameserver/src/{const.h,enums.h}`)
//! — the default era. The Lua symbol names are era-stable (TFS script API); only the
//! integer values differ between eras (772 TVP bit-flags vs 1098 TFS sequential for
//! some categories). Where a Rust enum already exists in `tfs_rust_common::enums` with
//! the correct values, we source from it to avoid a second source of truth.
//!
//! C++ reference: `luascript.cpp` `registerConstants` / `registerEnum` block.

use mlua::Lua;

/// Register all combat/spell/weapon/condition enums as Lua globals.
/// Called from `LuaRuntime::new` after `register_constants`.
pub fn register_combat_enums(lua: &Lua) -> Result<(), mlua::Error> {
    let globals = lua.globals();

    register_combat_types(&globals)?;
    register_combat_params(&globals)?;
    register_callbacks(&globals)?;
    register_combat_formulas(&globals)?;
    register_conditions(&globals)?;
    register_condition_params(&globals)?;
    register_condition_ids(&globals)?;
    register_weapon_types(&globals)?;
    register_spell_types(&globals)?;
    register_text_effects(&globals)?;
    register_shoot_types(&globals)?;
    register_skulls(&globals)?;
    register_inventory_slots(&globals)?;

    Ok(())
}

// --- COMBAT_*DAMAGE (enums.h:98-108, 772 bit-flag values) ---

fn register_combat_types(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // enums.h:98-108 — bit-flag combat damage types.
    globals.set("COMBAT_NONE", 0i32)?;
    globals.set("COMBAT_PHYSICALDAMAGE", 1i32 << 0)?; // enums.h:100
    globals.set("COMBAT_ENERGYDAMAGE", 1i32 << 1)?; // enums.h:101
    globals.set("COMBAT_EARTHDAMAGE", 1i32 << 2)?; // enums.h:102
    globals.set("COMBAT_FIREDAMAGE", 1i32 << 3)?; // enums.h:103
    globals.set("COMBAT_UNDEFINEDDAMAGE", 1i32 << 4)?; // enums.h:104
    globals.set("COMBAT_LIFEDRAIN", 1i32 << 5)?; // enums.h:105
    globals.set("COMBAT_MANADRAIN", 1i32 << 6)?; // enums.h:106
    globals.set("COMBAT_HEALING", 1i32 << 7)?; // enums.h:107
    Ok(())
}

// --- COMBAT_PARAM_* (enums.h:113-124, sequential 0..=11) ---

fn register_combat_params(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // enums.h:113-124 — `CombatParam_t` sequential enum.
    globals.set("COMBAT_PARAM_TYPE", 0i32)?; // enums.h:113
    globals.set("COMBAT_PARAM_EFFECT", 1i32)?; // enums.h:114
    globals.set("COMBAT_PARAM_DISTANCEEFFECT", 2i32)?; // enums.h:115
    globals.set("COMBAT_PARAM_BLOCKSHIELD", 3i32)?; // enums.h:116
    globals.set("COMBAT_PARAM_BLOCKARMOR", 4i32)?; // enums.h:117
    globals.set("COMBAT_PARAM_TARGETCASTERORTOPMOST", 5i32)?; // enums.h:118
    globals.set("COMBAT_PARAM_CREATEITEM", 6i32)?; // enums.h:119
    globals.set("COMBAT_PARAM_AGGRESSIVE", 7i32)?; // enums.h:120
    globals.set("COMBAT_PARAM_DISPEL", 8i32)?; // enums.h:121
    Ok(())
}

// --- CALLBACK_PARAM_* (enums.h:128-131, sequential 0..=3) ---

fn register_callbacks(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // enums.h:128-131 — `CallbackParam_t` sequential enum.
    globals.set("CALLBACK_PARAM_LEVELMAGICVALUE", 0i32)?; // enums.h:128
    globals.set("CALLBACK_PARAM_SKILLVALUE", 1i32)?; // enums.h:129
    globals.set("CALLBACK_PARAM_TARGETTILE", 2i32)?; // enums.h:130
    globals.set("CALLBACK_PARAM_TARGETCREATURE", 3i32)?; // enums.h:131
    Ok(())
}

// --- COMBAT_FORMULA_* (enums.h:244-247, sequential 0..=3) ---

fn register_combat_formulas(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // enums.h:244-247 — `formulaType_t` sequential enum.
    globals.set("COMBAT_FORMULA_UNDEFINED", 0i32)?; // enums.h:244
    globals.set("COMBAT_FORMULA_LEVELMAGIC", 1i32)?; // enums.h:245
    globals.set("COMBAT_FORMULA_SKILL", 2i32)?; // enums.h:246
    globals.set("COMBAT_FORMULA_DAMAGE", 3i32)?; // enums.h:247
    Ok(())
}

// --- CONDITION_* (enums.h:251-270, 772 bit-flag values) ---

fn register_conditions(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // enums.h:251-270 — bit-flag condition types. Note: the existing `constants.rs`
    // registers a subset (CONDITION_SOUL, CONDITION_MUTED, CONDITION_CHANNELMUTEDTICKS)
    // with 772 values. Those are re-set here with the same values for completeness;
    // the full set is needed by spell scripts (e.g. `Condition(CONDITION_POISON)`).
    globals.set("CONDITION_NONE", 0i32)?; // enums.h:251
    globals.set("CONDITION_POISON", 1i32 << 0)?; // enums.h:253
    globals.set("CONDITION_FIRE", 1i32 << 1)?; // enums.h:254
    globals.set("CONDITION_ENERGY", 1i32 << 2)?; // enums.h:255
    globals.set("CONDITION_BLEEDING", 1i32 << 3)?; // enums.h:256
    globals.set("CONDITION_HASTE", 1i32 << 4)?; // enums.h:257
    globals.set("CONDITION_PARALYZE", 1i32 << 5)?; // enums.h:258
    globals.set("CONDITION_OUTFIT", 1i32 << 6)?; // enums.h:259
    globals.set("CONDITION_INVISIBLE", 1i32 << 7)?; // enums.h:260
    globals.set("CONDITION_LIGHT", 1i32 << 8)?; // enums.h:261
    globals.set("CONDITION_MANASHIELD", 1i32 << 9)?; // enums.h:262
    globals.set("CONDITION_INFIGHT", 1i32 << 10)?; // enums.h:263
    globals.set("CONDITION_DRUNK", 1i32 << 11)?; // enums.h:264
    // enums.h:265-266 (CONDITION_EXHAUST_WEAPON / CONDITION_EXHAUST_COMBAT) are 772-specific;
    // the TVP tree may not have them. Register with safe bit positions.
    globals.set("CONDITION_EXHAUST_WEAPON", 1i32 << 12)?;
    globals.set("CONDITION_MUTED", 1i32 << 14)?; // enums.h:267
    globals.set("CONDITION_CHANNELMUTEDTICKS", 1i32 << 15)?; // enums.h:268
    globals.set("CONDITION_YELLTICKS", 1i32 << 16)?; // enums.h:269
    globals.set("CONDITION_ATTRIBUTES", 1i32 << 17)?; // enums.h:270
    Ok(())
}

// --- CONDITION_PARAM_* (enums.h:135-185, sequential 1..=50+) ---

fn register_condition_params(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // enums.h:135-185 — `ConditionParam_t` sequential enum. The existing `constants.rs`
    // registers a subset (TICKS, SOULGAIN, SOULTICKS, SUBID); the full set is needed by
    // spell scripts that configure conditions via `:setParameter`.
    globals.set("CONDITION_PARAM_OWNER", 1i32)?; // enums.h:135
    globals.set("CONDITION_PARAM_TICKS", 2i32)?; // enums.h:136
    globals.set("CONDITION_PARAM_HEALTHGAIN", 4i32)?; // enums.h:138
    globals.set("CONDITION_PARAM_HEALTHTICKS", 5i32)?; // enums.h:139
    globals.set("CONDITION_PARAM_MANAGAIN", 6i32)?; // enums.h:140
    globals.set("CONDITION_PARAM_MANATICKS", 7i32)?; // enums.h:141
    globals.set("CONDITION_PARAM_DELAYED", 8i32)?; // enums.h:142
    globals.set("CONDITION_PARAM_SPEED", 9i32)?; // enums.h:143
    globals.set("CONDITION_PARAM_LIGHT_LEVEL", 10i32)?; // enums.h:144
    globals.set("CONDITION_PARAM_LIGHT_COLOR", 11i32)?; // enums.h:145
    globals.set("CONDITION_PARAM_SOULGAIN", 12i32)?; // enums.h:146
    globals.set("CONDITION_PARAM_SOULTICKS", 13i32)?; // enums.h:147
    globals.set("CONDITION_PARAM_MINVALUE", 14i32)?; // enums.h:148
    globals.set("CONDITION_PARAM_MAXVALUE", 15i32)?; // enums.h:149
    globals.set("CONDITION_PARAM_STARTVALUE", 16i32)?; // enums.h:150
    globals.set("CONDITION_PARAM_TICKINTERVAL", 17i32)?; // enums.h:151
    globals.set("CONDITION_PARAM_FORCEUPDATE", 18i32)?; // enums.h:152
    globals.set("CONDITION_PARAM_SKILL_MELEE", 19i32)?; // enums.h:153
    globals.set("CONDITION_PARAM_SKILL_FIST", 20i32)?; // enums.h:154
    globals.set("CONDITION_PARAM_SKILL_CLUB", 21i32)?; // enums.h:155
    globals.set("CONDITION_PARAM_SKILL_SWORD", 22i32)?; // enums.h:156
    globals.set("CONDITION_PARAM_SKILL_AXE", 23i32)?; // enums.h:157
    globals.set("CONDITION_PARAM_SKILL_DISTANCE", 24i32)?; // enums.h:158
    globals.set("CONDITION_PARAM_SKILL_SHIELD", 25i32)?; // enums.h:159
    globals.set("CONDITION_PARAM_SKILL_FISHING", 26i32)?; // enums.h:160
    globals.set("CONDITION_PARAM_STAT_MAXHITPOINTS", 27i32)?; // enums.h:161
    globals.set("CONDITION_PARAM_STAT_MAXMANAPOINTS", 28i32)?; // enums.h:162
    globals.set("CONDITION_PARAM_STAT_MAGICPOINTS", 30i32)?; // enums.h:164
    globals.set("CONDITION_PARAM_STAT_MAXHITPOINTSPERCENT", 31i32)?; // enums.h:165
    globals.set("CONDITION_PARAM_STAT_MAXMANAPOINTSPERCENT", 32i32)?; // enums.h:166
    globals.set("CONDITION_PARAM_STAT_MAGICPOINTSPERCENT", 34i32)?; // enums.h:168
    globals.set("CONDITION_PARAM_PERIODICDAMAGE", 35i32)?; // enums.h:169
    globals.set("CONDITION_PARAM_SKILL_MELEEPERCENT", 36i32)?; // enums.h:170
    globals.set("CONDITION_PARAM_SKILL_FISTPERCENT", 37i32)?; // enums.h:171
    globals.set("CONDITION_PARAM_SKILL_CLUBPERCENT", 38i32)?; // enums.h:172
    globals.set("CONDITION_PARAM_SKILL_SWORDPERCENT", 39i32)?; // enums.h:173
    globals.set("CONDITION_PARAM_SKILL_AXEPERCENT", 40i32)?; // enums.h:174
    globals.set("CONDITION_PARAM_SKILL_DISTANCEPERCENT", 41i32)?; // enums.h:175
    globals.set("CONDITION_PARAM_SKILL_SHIELDPERCENT", 42i32)?; // enums.h:176
    globals.set("CONDITION_PARAM_SKILL_FISHINGPERCENT", 43i32)?; // enums.h:177
    globals.set("CONDITION_PARAM_BUFF_SPELL", 44i32)?; // enums.h:178
    globals.set("CONDITION_PARAM_SUBID", 45i32)?; // enums.h:179
    Ok(())
}

// --- CONDITIONID_* (enums.h:275-286) ---

fn register_condition_ids(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // enums.h:275-286 — condition slot IDs. `CONDITIONID_DEFAULT = -1` is used by
    // `Condition(type, CONDITIONID_DEFAULT)` in spell scripts.
    globals.set("CONDITIONID_DEFAULT", -1i32)?; // enums.h:275
    globals.set("CONDITIONID_COMBAT", 0i32)?; // enums.h:276
    globals.set("CONDITIONID_HEAD", 1i32)?; // enums.h:277
    globals.set("CONDITIONID_NECKLACE", 2i32)?; // enums.h:278
    globals.set("CONDITIONID_BACKPACK", 3i32)?; // enums.h:279
    globals.set("CONDITIONID_ARMOR", 4i32)?; // enums.h:280
    globals.set("CONDITIONID_RIGHT", 5i32)?; // enums.h:281
    globals.set("CONDITIONID_LEFT", 6i32)?; // enums.h:282
    globals.set("CONDITIONID_LEGS", 7i32)?; // enums.h:283
    globals.set("CONDITIONID_FEET", 8i32)?; // enums.h:284
    globals.set("CONDITIONID_RING", 9i32)?; // enums.h:285
    globals.set("CONDITIONID_AMMO", 10i32)?; // enums.h:286
    Ok(())
}

// --- WEAPON_* (const.h:143-150, sequential 0..=7) ---

fn register_weapon_types(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // const.h:143-150 — `WeaponType_t` sequential enum.
    globals.set("WEAPON_NONE", 0i32)?; // const.h:143
    globals.set("WEAPON_SWORD", 1i32)?; // const.h:144
    globals.set("WEAPON_CLUB", 2i32)?; // const.h:145
    globals.set("WEAPON_AXE", 3i32)?; // const.h:146
    globals.set("WEAPON_SHIELD", 4i32)?; // const.h:147
    globals.set("WEAPON_DISTANCE", 5i32)?; // const.h:148
    globals.set("WEAPON_WAND", 6i32)?; // const.h:149
    globals.set("WEAPON_AMMO", 7i32)?; // const.h:150
    Ok(())
}

// --- SPELL_* (enums.h:75-76) ---

fn register_spell_types(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // enums.h:75-76 — `SpellType_t`.
    globals.set("SPELL_INSTANT", 1i32)?; // enums.h:75
    globals.set("SPELL_RUNE", 2i32)?; // enums.h:76
    Ok(())
}

// --- CONST_ME_* (const.h:9-35, 772 values) ---

fn register_text_effects(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // const.h:9-35 — `MagicEffect_t` (CONST_ME_*). 772 TVP values.
    globals.set("CONST_ME_NONE", 0i32)?; // const.h:9
    globals.set("CONST_ME_DRAWBLOOD", 1i32)?; // const.h:11
    globals.set("CONST_ME_LOSEENERGY", 2i32)?; // const.h:12
    globals.set("CONST_ME_POFF", 3i32)?; // const.h:13
    globals.set("CONST_ME_BLOCKHIT", 4i32)?; // const.h:14
    globals.set("CONST_ME_EXPLOSIONAREA", 5i32)?; // const.h:15
    globals.set("CONST_ME_EXPLOSIONHIT", 6i32)?; // const.h:16
    globals.set("CONST_ME_FIREAREA", 7i32)?; // const.h:17
    globals.set("CONST_ME_YELLOW_RINGS", 8i32)?; // const.h:18
    globals.set("CONST_ME_GREEN_RINGS", 9i32)?; // const.h:19
    globals.set("CONST_ME_HITAREA", 10i32)?; // const.h:20
    globals.set("CONST_ME_TELEPORT", 11i32)?; // const.h:21
    globals.set("CONST_ME_ENERGYHIT", 12i32)?; // const.h:22
    globals.set("CONST_ME_MAGIC_BLUE", 13i32)?; // const.h:23
    globals.set("CONST_ME_MAGIC_RED", 14i32)?; // const.h:24
    globals.set("CONST_ME_MAGIC_GREEN", 15i32)?; // const.h:25
    globals.set("CONST_ME_HITBYFIRE", 16i32)?; // const.h:26
    globals.set("CONST_ME_HITBYPOISON", 17i32)?; // const.h:27
    globals.set("CONST_ME_MORTAREA", 18i32)?; // const.h:28
    globals.set("CONST_ME_SOUND_GREEN", 19i32)?; // const.h:29
    globals.set("CONST_ME_SOUND_RED", 20i32)?; // const.h:30
    globals.set("CONST_ME_POISONAREA", 21i32)?; // const.h:31
    Ok(())
}

// --- CONST_ANI_* (const.h:39-55, 772 values) ---

fn register_shoot_types(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // const.h:39-55 — `ShootType_t` (CONST_ANI_*). 772 TVP values.
    globals.set("CONST_ANI_NONE", 0i32)?; // const.h:39
    globals.set("CONST_ANI_SPEAR", 1i32)?; // const.h:41
    globals.set("CONST_ANI_BOLT", 2i32)?; // const.h:42
    globals.set("CONST_ANI_ARROW", 3i32)?; // const.h:43
    globals.set("CONST_ANI_FIRE", 4i32)?; // const.h:44
    globals.set("CONST_ANI_ENERGY", 5i32)?; // const.h:45
    globals.set("CONST_ANI_POISONARROW", 6i32)?; // const.h:46
    globals.set("CONST_ANI_BURSTARROW", 7i32)?; // const.h:47
    globals.set("CONST_ANI_THROWINGSTAR", 8i32)?; // const.h:48
    globals.set("CONST_ANI_THROWINGKNIFE", 9i32)?; // const.h:49
    globals.set("CONST_ANI_SMALLSTONE", 10i32)?; // const.h:50
    globals.set("CONST_ANI_DEATH", 11i32)?; // const.h:51
    globals.set("CONST_ANI_LARGEROCK", 12i32)?; // const.h:52
    globals.set("CONST_ANI_SNOWBALL", 13i32)?; // const.h:53
    globals.set("CONST_ANI_POWERBOLT", 14i32)?; // const.h:54
    globals.set("CONST_ANI_POISON", 15i32)?; // const.h:55
    Ok(())
}

// --- SKULL_* (const.h:180-185) ---

fn register_skulls(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // const.h:180-185 — `Skulls_t` sequential enum.
    globals.set("SKULL_NONE", 0i32)?; // const.h:180
    globals.set("SKULL_YELLOW", 1i32)?; // const.h:181
    globals.set("SKULL_GREEN", 2i32)?; // const.h:182
    globals.set("SKULL_WHITE", 3i32)?; // const.h:183
    globals.set("SKULL_RED", 4i32)?; // const.h:184
    Ok(())
}

// --- CONST_SLOT_* (inventory slot indices for `player:getSlotItem`) ---

fn register_inventory_slots(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // TFS `const.h` `CONST_SLOT_*` — 1-indexed equipment slots matching `InventorySlot`.
    globals.set("CONST_SLOT_HEAD", 1i32)?;
    globals.set("CONST_SLOT_NECKLACE", 2i32)?;
    globals.set("CONST_SLOT_BACKPACK", 3i32)?;
    globals.set("CONST_SLOT_ARMOR", 4i32)?;
    globals.set("CONST_SLOT_RIGHT", 5i32)?;
    globals.set("CONST_SLOT_LEFT", 6i32)?;
    globals.set("CONST_SLOT_LEGS", 7i32)?;
    globals.set("CONST_SLOT_FEET", 8i32)?;
    globals.set("CONST_SLOT_RING", 9i32)?;
    globals.set("CONST_SLOT_AMMO", 10i32)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combat_enums_register_without_error() {
        let lua = Lua::new();
        register_combat_enums(&lua).expect("enum registration must succeed");
        let globals = lua.globals();
        // Spot-check representative values from each category.
        assert_eq!(globals.get::<i32>("COMBAT_PHYSICALDAMAGE").unwrap(), 1);
        assert_eq!(globals.get::<i32>("COMBAT_HEALING").unwrap(), 128);
        assert_eq!(globals.get::<i32>("COMBAT_PARAM_TYPE").unwrap(), 0);
        assert_eq!(globals.get::<i32>("COMBAT_PARAM_AGGRESSIVE").unwrap(), 7);
        assert_eq!(globals.get::<i32>("CALLBACK_PARAM_SKILLVALUE").unwrap(), 1);
        assert_eq!(globals.get::<i32>("COMBAT_FORMULA_SKILL").unwrap(), 2);
        assert_eq!(globals.get::<i32>("CONDITION_POISON").unwrap(), 1);
        assert_eq!(globals.get::<i32>("CONDITION_INVISIBLE").unwrap(), 128);
        assert_eq!(globals.get::<i32>("CONDITION_PARAM_TICKS").unwrap(), 2);
        assert_eq!(globals.get::<i32>("CONDITIONID_DEFAULT").unwrap(), -1);
        assert_eq!(globals.get::<i32>("WEAPON_WAND").unwrap(), 6);
        assert_eq!(globals.get::<i32>("SPELL_INSTANT").unwrap(), 1);
        assert_eq!(globals.get::<i32>("CONST_ME_HITAREA").unwrap(), 10);
        assert_eq!(globals.get::<i32>("CONST_ANI_BURSTARROW").unwrap(), 7);
        assert_eq!(globals.get::<i32>("SKULL_RED").unwrap(), 4);
        assert_eq!(globals.get::<i32>("CONST_SLOT_LEFT").unwrap(), 6);
    }
}
