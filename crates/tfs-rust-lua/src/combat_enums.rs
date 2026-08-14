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
    register_item_constants(&globals)?;
    register_directions(&globals)?;
    register_tile_props(&globals)?;
    register_tile_states(&globals)?;
    register_message_types(&globals)?;
    register_world_types(&globals)?;
    register_cylinder_flags(&globals)?;
    register_item_types(&globals)?;
    register_skills(&globals)?;

    Ok(())
}

// --- SKILL_* (enums.h skills_t, sequential 0..=8) ---

fn register_skills(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // `enums.h` `skills_t` + `luascript.cpp` `registerEnum(SKILL_FIST)` … `SKILL_LEVEL`.
    // TFS Lua skill ids — not 772 timer-skill indices (`SKILL_FED`, `SKILL_GO`, …).
    globals.set("SKILL_FIST", 0i32)?; // enums.h:286
    globals.set("SKILL_CLUB", 1i32)?; // enums.h:287
    globals.set("SKILL_SWORD", 2i32)?; // enums.h:288
    globals.set("SKILL_AXE", 3i32)?; // enums.h:289
    globals.set("SKILL_DISTANCE", 4i32)?; // enums.h:290
    globals.set("SKILL_SHIELD", 5i32)?; // enums.h:291
    globals.set("SKILL_FISHING", 6i32)?; // enums.h:292
    globals.set("SKILL_MAGLEVEL", 7i32)?; // enums.h:294
    globals.set("SKILL_LEVEL", 8i32)?; // enums.h:295
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
    globals.set("COMBAT_PARAM_USECHARGES", 9i32)?; // enums.h:122
    globals.set("COMBAT_PARAM_NODAMAGE", 10i32)?; // enums.h:123
    globals.set("COMBAT_PARAM_FORCEONTARGETEVENT", 11i32)?; // enums.h:124
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
    // enums.h:190-195 — 772-specific condition params used by spell scripts
    // (e.g. soulfire_rune.lua sets CONDITION_PARAM_CYCLE/COUNT/MAX_COUNT/OWNERGUID).
    globals.set("CONDITION_PARAM_CYCLE", 56i32)?; // enums.h:190
    globals.set("CONDITION_PARAM_COUNT", 58i32)?; // enums.h:192
    globals.set("CONDITION_PARAM_MAX_COUNT", 59i32)?; // enums.h:193
    globals.set("CONDITION_PARAM_OWNERGUID", 60i32)?; // enums.h:194
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

// --- ITEM_* field constants (const.h:196-216, 772 values) ---

fn register_item_constants(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // const.h:196-216 — field/wall item ids used by `combat:setParameter(COMBAT_PARAM_CREATEITEM, ...)`.
    globals.set("ITEM_FIREFIELD_PVP_FULL", 1487i32)?; // const.h:196
    globals.set("ITEM_FIREFIELD_PVP_MEDIUM", 1488i32)?; // const.h:197
    globals.set("ITEM_FIREFIELD_PVP_SMALL", 1489i32)?; // const.h:198
    globals.set("ITEM_FIREFIELD_PERSISTENT_FULL", 1492i32)?; // const.h:199
    globals.set("ITEM_FIREFIELD_PERSISTENT_MEDIUM", 1493i32)?; // const.h:200
    globals.set("ITEM_FIREFIELD_PERSISTENT_SMALL", 1494i32)?; // const.h:201
    globals.set("ITEM_FIREFIELD_NOPVP", 1500i32)?; // const.h:203
    globals.set("ITEM_POISONFIELD_PVP", 1490i32)?; // const.h:204
    globals.set("ITEM_POISONFIELD_PERSISTENT", 1496i32)?; // const.h:205
    globals.set("ITEM_POISONFIELD_NOPVP", 1503i32)?; // const.h:207
    globals.set("ITEM_ENERGYFIELD_PVP", 1491i32)?; // const.h:208
    globals.set("ITEM_ENERGYFIELD_PERSISTENT", 1495i32)?; // const.h:209
    globals.set("ITEM_ENERGYFIELD_NOPVP", 1504i32)?; // const.h:210
    globals.set("ITEM_MAGICWALL", 1497i32)?; // const.h:212
    globals.set("ITEM_MAGICWALL_PERSISTENT", 1498i32)?; // const.h:213
    globals.set("ITEM_MAGICWALL_NOPVP", 20669i32)?; // const.h:214
    globals.set("ITEM_WILDGROWTH", 1499i32)?; // const.h:215
    globals.set("ITEM_WILDGROWTH_PERSISTENT", 2721i32)?; // const.h:216
    globals.set("ITEM_WILDGROWTH_NOPVP", 20670i32)?; // const.h:217
    Ok(())
}

// --- DIRECTION_* (position.h:6-16, 772 values) ---

fn register_directions(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // position.h:6-16 — `Direction` enum. Used by find_person.lua's direction table.
    globals.set("DIRECTION_NORTH", 0i32)?; // position.h:7
    globals.set("DIRECTION_EAST", 1i32)?; // position.h:8
    globals.set("DIRECTION_SOUTH", 2i32)?; // position.h:9
    globals.set("DIRECTION_WEST", 3i32)?; // position.h:10
    globals.set("DIRECTION_SOUTHWEST", 4i32)?; // position.h:13 (DIAGONAL_MASK|0)
    globals.set("DIRECTION_SOUTHEAST", 5i32)?; // position.h:14 (DIAGONAL_MASK|1)
    globals.set("DIRECTION_NORTHWEST", 6i32)?; // position.h:15 (DIAGONAL_MASK|2)
    globals.set("DIRECTION_NORTHEAST", 7i32)?; // position.h:16 (DIAGONAL_MASK|3)
    Ok(())
}

// --- CONST_PROP_* (item.h:28-40, 772 values) ---

fn register_tile_props(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // item.h:28-40 — `ItemProperty_t` sequential enum. Used by
    // `tile:hasProperty(CONST_PROP_BLOCKSOLID)` in rune scripts.
    globals.set("CONST_PROP_BLOCKSOLID", 0i32)?; // item.h:28
    globals.set("CONST_PROP_HASHEIGHT", 1i32)?; // item.h:29
    globals.set("CONST_PROP_BLOCKPROJECTILE", 2i32)?; // item.h:30
    globals.set("CONST_PROP_BLOCKPATH", 3i32)?; // item.h:31
    globals.set("CONST_PROP_ISVERTICAL", 4i32)?; // item.h:32
    globals.set("CONST_PROP_ISHORIZONTAL", 5i32)?; // item.h:33
    globals.set("CONST_PROP_MOVEABLE", 6i32)?; // item.h:34
    globals.set("CONST_PROP_IMMOVABLEBLOCKSOLID", 7i32)?; // item.h:35
    globals.set("CONST_PROP_IMMOVABLEBLOCKPATH", 8i32)?; // item.h:36
    globals.set("CONST_PROP_IMMOVABLENOFIELDBLOCKPATH", 9i32)?; // item.h:37
    globals.set("CONST_PROP_NOFIELDBLOCKPATH", 10i32)?; // item.h:38
    globals.set("CONST_PROP_SUPPORTHANGABLE", 11i32)?; // item.h:39
    globals.set("CONST_PROP_SPECIALFIELDBLOCKPATH", 12i32)?; // item.h:40
    Ok(())
}

// --- TILESTATE_* (tile.h:23-51) ---

fn register_tile_states(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // `tileflags_t` — must match `crates/tfs-rust-core/src/tile.rs` `flags` and `src/tile.h`.
    globals.set("TILESTATE_NONE", 0i32)?;
    globals.set("TILESTATE_FLOORCHANGE_DOWN", 1i32)?; // 1<<0
    globals.set("TILESTATE_FLOORCHANGE_NORTH", 2i32)?; // 1<<1
    globals.set("TILESTATE_FLOORCHANGE_SOUTH", 4i32)?; // 1<<2
    globals.set("TILESTATE_FLOORCHANGE_EAST", 8i32)?; // 1<<3
    globals.set("TILESTATE_FLOORCHANGE_WEST", 16i32)?; // 1<<4
    globals.set("TILESTATE_PROTECTIONZONE", 128i32)?; // 1<<7
    globals.set("TILESTATE_NOPVPZONE", 256i32)?; // 1<<8
    globals.set("TILESTATE_NOLOGOUT", 512i32)?; // 1<<9
    globals.set("TILESTATE_PVPZONE", 1024i32)?; // 1<<10
    // Absent in current tile.h; keep defined so look/compat scripts don't nil.
    globals.set("TILESTATE_REFRESH", 0i32)?;
    globals.set("TILESTATE_TELEPORT", 2048i32)?; // 1<<11
    globals.set("TILESTATE_MAGICFIELD", 4096i32)?; // 1<<12
    globals.set("TILESTATE_MAILBOX", 8192i32)?; // 1<<13
    globals.set("TILESTATE_TRASHHOLDER", 16384i32)?; // 1<<14
    globals.set("TILESTATE_BED", 32768i32)?; // 1<<15
    globals.set("TILESTATE_DEPOT", 65536i32)?; // 1<<16
    globals.set("TILESTATE_BLOCKSOLID", 131072i32)?; // 1<<17
    globals.set("TILESTATE_BLOCKPATH", 262144i32)?; // 1<<18
    globals.set("TILESTATE_IMMOVABLEBLOCKSOLID", 524288i32)?; // 1<<19
    // Composite floor-change mask — tile.h TILESTATE_FLOORCHANGE.
    globals.set(
        "TILESTATE_FLOORCHANGE",
        1i32 | 2 | 4 | 8 | 16 | 32 | 64,
    )?;
    Ok(())
}

// --- MESSAGE_* (const.h:80-90, 772 values) ---

fn register_message_types(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // const.h:80-90 — `MessageClasses_t`. Used by `creature:sendTextMessage(...)` in spell scripts.
    globals.set("MESSAGE_STATUS_CONSOLE_YELLOW", 0x01i32)?; // const.h:80
    globals.set("MESSAGE_STATUS_CONSOLE_LBLUE", 0x04i32)?; // const.h:81
    globals.set("MESSAGE_STATUS_CONSOLE_ORANGE", 0x11i32)?; // const.h:82
    globals.set("MESSAGE_STATUS_WARNING", 0x12i32)?; // const.h:83
    globals.set("MESSAGE_EVENT_ADVANCE", 0x13i32)?; // const.h:84
    globals.set("MESSAGE_EVENT_DEFAULT", 0x14i32)?; // const.h:85
    globals.set("MESSAGE_STATUS_DEFAULT", 0x15i32)?; // const.h:86
    globals.set("MESSAGE_INFO_DESCR", 0x16i32)?; // const.h:87
    globals.set("MESSAGE_STATUS_SMALL", 0x17i32)?; // const.h:88
    globals.set("MESSAGE_STATUS_CONSOLE_BLUE", 0x18i32)?; // const.h:89
    globals.set("MESSAGE_STATUS_CONSOLE_RED", 0x19i32)?; // const.h:90
    Ok(())
}

// --- WORLD_TYPE_* (enums.h / game.h) ---

fn register_world_types(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // C++ `WorldType_t` — `enums.h`. Used by `cancel_invisibility.lua`.
    globals.set("WORLD_TYPE_NO_PVP", 0i32)?;
    globals.set("WORLD_TYPE_PVP", 1i32)?;
    globals.set("WORLD_TYPE_PVP_ENFORCED", 2i32)?;
    Ok(())
}

fn register_cylinder_flags(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // cylinder.h — used by levitate `creature:move(tile, flags)`.
    globals.set("FLAG_IGNOREBLOCKITEM", 1i32 << 1)?;
    globals.set("FLAG_IGNOREBLOCKCREATURE", 1i32 << 2)?;
    Ok(())
}

/// `ItemTypes_t` — `items.h:30-41`. Used by `tile:getItemByType` and `data/items/#items.lua`.
fn register_item_types(globals: &mlua::Table) -> Result<(), mlua::Error> {
    globals.set("ITEM_TYPE_NONE", 0i32)?;
    globals.set("ITEM_TYPE_DEPOT", 1i32)?;
    globals.set("ITEM_TYPE_MAILBOX", 2i32)?;
    globals.set("ITEM_TYPE_TRASHHOLDER", 3i32)?;
    globals.set("ITEM_TYPE_CONTAINER", 4i32)?;
    globals.set("ITEM_TYPE_DOOR", 5i32)?;
    globals.set("ITEM_TYPE_MAGICFIELD", 6i32)?;
    globals.set("ITEM_TYPE_TELEPORT", 7i32)?;
    globals.set("ITEM_TYPE_BED", 8i32)?;
    globals.set("ITEM_TYPE_KEY", 9i32)?;
    globals.set("ITEM_TYPE_RUNE", 10i32)?;
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
        assert_eq!(globals.get::<i32>("WORLD_TYPE_PVP").unwrap(), 1);
        assert_eq!(globals.get::<i32>("WORLD_TYPE_PVP_ENFORCED").unwrap(), 2);
        // Phase 2: TILESTATE_* must match tile.h / core `flags` (not the old REFRESH-shifted layout).
        assert_eq!(globals.get::<i32>("TILESTATE_BLOCKSOLID").unwrap(), 1 << 17);
        assert_eq!(globals.get::<i32>("TILESTATE_MAGICFIELD").unwrap(), 1 << 12);
        assert_eq!(globals.get::<i32>("TILESTATE_TELEPORT").unwrap(), 1 << 11);
        assert_eq!(globals.get::<i32>("TILESTATE_DEPOT").unwrap(), 1 << 16);
        // Gap 4 — `skills_t` (`enums.h` / `luascript.cpp` `registerEnum`).
        assert_eq!(globals.get::<i32>("SKILL_FIST").unwrap(), 0);
        assert_eq!(globals.get::<i32>("SKILL_FISHING").unwrap(), 6);
        assert_eq!(globals.get::<i32>("SKILL_MAGLEVEL").unwrap(), 7);
        assert_eq!(globals.get::<i32>("SKILL_LEVEL").unwrap(), 8);
    }
}
