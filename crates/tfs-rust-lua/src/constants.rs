//! Lua global constants for the TFS scripting surface.
//!
//! Mirrors the `registerConstants` / `registerEnum` block of `luascript.cpp`.
//! Values are the **772 wire/mechanics era** (default `clientVersion = 772`); the
//! Lua symbol names are era-stable (taken from `luascript.cpp` `registerMethod` /
//! `registerEnum`), only the integer values differ between eras.
//!
//! ## Source discipline
//! Per `TFS-protocol-versioning.md`: 772 values come from the TVP reference tree
//! `reference/tvp-772/gameserver/src/{const.h,enums.h}`. Each group cites the
//! exact file:line. Where a Rust enum already exists in `tfs_rust_common::enums`
//! with the correct 772 values, we source from it to avoid a second source of
//! truth; otherwise we hardcode with a `// <file>:<line>` cite.
//!
//! This replaces the scattered `globals.set(...)` constant lines that previously
//! lived in `runtime.rs::register_event_script_bootstrap`. Class-table stubs
//! (`Player`, `Creature`, …), the `Channel` constructor, the `Condition` stub,
//! and the `hasEventCallback`/`EventCallback` no-ops stay in `runtime.rs` —
//! only bare enum/flag constants move here.

use mlua::Lua;

/// Register all TFS Lua global constants onto `lua`.
///
/// Called once from `LuaRuntime::new` after the class-table stubs. Idempotent in
/// shape (overwrites any prior value); safe to call after
/// `register_event_script_bootstrap`.
///
/// # Errors
///
/// Returns `mlua::Error` if any `globals().set(...)` fails.
pub fn register_constants(lua: &Lua) -> Result<(), mlua::Error> {
    let globals = lua.globals();

    register_account_types(&globals)?;
    register_talk_types(&globals)?;
    register_fluids(&globals)?;
    register_player_flags(&globals)?;
    register_vocations(&globals)?;
    register_conditions(&globals)?;
    register_return_values(&globals)?;
    register_item_attributes(&globals)?;
    register_item_groups(&globals)?;
    register_misc(&globals)?;
    register_game_states(&globals)?;
    register_reload_types(&globals)?;
    register_config_keys(lua)?;

    Ok(())
}

// --- ACCOUNT_TYPE_* (enums.h:80-85) ---

fn register_account_types(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // enums.h:80-85 — account-level access tier used by channel hook gating.
    globals.set("ACCOUNT_TYPE_NORMAL", 1i32)?;
    globals.set("ACCOUNT_TYPE_TUTOR", 2i32)?;
    globals.set("ACCOUNT_TYPE_SENIORTUTOR", 3i32)?;
    globals.set("ACCOUNT_TYPE_GAMEMASTER", 4i32)?;
    globals.set("ACCOUNT_TYPE_COMMUNITYMANAGER", 5i32)?;
    globals.set("ACCOUNT_TYPE_GOD", 6i32)?;
    Ok(())
}

// --- TALKTYPE_* (const.h:62-76, 772 values) ---
//
// NOTE: these are the **772** wire values. They differ from the TFS 1.4.2
// (1098) `SpeakType` enum in `tfs_rust_common::enums`, which carries 1098
// numbering (ChannelYellow=7, ChannelWhite=8, …). Until a 772-correct
// `SpeakClasses` enum is added to `tfs_rust_common::enums`, we hardcode here
// with `const.h:NN` cites — the single source of truth for 772 wire talktype
// values is `game_world_chat.rs`'s local `const`s, which match these exactly.

fn register_talk_types(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // const.h:62-76 — 772 `SpeakClasses`. TFS 1.4.2 `TALKTYPE_MONSTER_SAY` is 36.
    globals.set("TALKTYPE_SAY", 1i32)?; // const.h:62
    globals.set("TALKTYPE_CHANNEL_Y", 5i32)?; // const.h:66 — Yellow
    globals.set("TALKTYPE_CHANNEL_R1", 10i32)?; // const.h:71 — Red (#c text)
    globals.set("TALKTYPE_CHANNEL_O", 12i32)?; // const.h:73 — orange
    globals.set("TALKTYPE_CHANNEL_R2", 14i32)?; // const.h:74 — red anonymous (#d text)
    globals.set("TALKTYPE_MONSTER_SAY", 0x11i32)?; // const.h:76
    Ok(())
}

// --- FLUID_* (const.h:94-106, 772 sequential) ---
//
// TVP `FluidTypes_t` is 0..=12, not TFS 1.4.2's colour-mapped `FluidTypes_t`
// (`FLUID_OIL = BROWN+8`, `FLUID_MANA` vs `FLUID_MANAFLUID`). Scripts in
// `data/scripts/actions/other/` use the 772 names (`FLUID_MANAFLUID`,
// `FLUID_LIFEFLUID`, `FLUID_OIL` = 7).

fn register_fluids(globals: &mlua::Table) -> Result<(), mlua::Error> {
    globals.set("FLUID_NONE", 0i32)?; // const.h:94
    globals.set("FLUID_WATER", 1i32)?; // const.h:95
    globals.set("FLUID_WINE", 2i32)?; // const.h:96
    globals.set("FLUID_BEER", 3i32)?; // const.h:97
    globals.set("FLUID_MUD", 4i32)?; // const.h:98
    globals.set("FLUID_BLOOD", 5i32)?; // const.h:99
    globals.set("FLUID_SLIME", 6i32)?; // const.h:100
    globals.set("FLUID_OIL", 7i32)?; // const.h:101
    globals.set("FLUID_URINE", 8i32)?; // const.h:102
    globals.set("FLUID_MILK", 9i32)?; // const.h:103
    globals.set("FLUID_MANAFLUID", 10i32)?; // const.h:104
    globals.set("FLUID_LIFEFLUID", 11i32)?; // const.h:105
    globals.set("FLUID_LEMONADE", 12i32)?; // const.h:106
    Ok(())
}

// --- PlayerFlag_* (const.h:264-266, 772 values) ---

fn register_player_flags(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // const.h:264-266 — per-player permission flags tested via `player:hasFlag`.
    globals.set("PlayerFlag_CannotUseCombat", 1i32 << 0)?;
    globals.set("PlayerFlag_CannotAttackPlayer", 1i32 << 1)?;
    globals.set("PlayerFlag_CannotAttackMonster", 1i32 << 2)?;
    globals.set("PlayerFlag_CannotBeAttacked", 1i32 << 3)?;
    globals.set("PlayerFlag_CanTalkRedPrivate", 1i32 << 21)?; // const.h:264
    globals.set("PlayerFlag_CanTalkRedChannel", 1i32 << 22)?; // const.h:265
    globals.set("PlayerFlag_TalkOrangeHelpChannel", 1i32 << 23)?; // const.h:266
    // const.h:506 — PC-3a Phase 5: `conjureItem` dual-hand mana gate.
    globals.set("PlayerFlag_HasInfiniteMana", 1i64 << 10)?;
    // const.h:516 — used by quest / capacity scripts.
    globals.set("PlayerFlag_HasInfiniteCapacity", 1i64 << 20)?;
    // const.h:500–501 — summon/convince admin overrides.
    globals.set("PlayerFlag_CanConvinceAll", 1i64 << 4)?;
    globals.set("PlayerFlag_CanSummonAll", 1i64 << 5)?;
    globals.set("PlayerFlag_CanIllusionAll", 1i64 << 6)?;
    // const.h:278 — always-premium group flag (`Player::isPremium`).
    globals.set("PlayerFlag_IsAlwaysPremium", 1i64 << 35)?;
    Ok(())
}

// --- VOCATION_* (enums.h:297) ---

fn register_vocations(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // enums.h:297 — no-vocation sentinel used by advertising channel gating.
    globals.set("VOCATION_NONE", 0i32)?;
    Ok(())
}

// --- CONDITION_* / CONDITIONID_* / CONDITION_PARAM_* ---
//
// Only the subset currently referenced by `data/events/scripts/player.lua`
// (soul condition build) is registered here. The CH-5 mute constants
// (`CONDITION_CHANNELMUTEDTICKS`, `CONDITION_PARAM_SUBID`, `CONDITION_PARAM_TICKS`)
// land in LUA-4 alongside the real `Condition` userdata.

fn register_conditions(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // enums.h:266 — `Condition(CONDITION_SOUL, CONDITIONID_DEFAULT)` in player.lua.
    globals.set("CONDITION_SOUL", 1i32 << 13)?; // enums.h:266
    // enums.h:267 — global flood mute (CH-5 `player_remove_message_buffer`).
    globals.set("CONDITION_MUTED", 1i32 << 14)?; // enums.h:267
    // enums.h:268 — per-channel offer-throttle mute (CH-5 commented blocks).
    globals.set("CONDITION_CHANNELMUTEDTICKS", 1i32 << 15)?; // enums.h:268
    // enums.h:275 — was previously registered as `0` (wrong); fixed to `-1`.
    // See lua-api-plan.md §4.3.
    globals.set("CONDITIONID_DEFAULT", -1i32)?; // enums.h:275
    // enums.h:136 — condition duration in ms; set via `:setParameter` or `setTicks`.
    globals.set("CONDITION_PARAM_TICKS", 2i32)?; // enums.h:136
    // enums.h:146-147 — soul regen params set on the soul condition.
    globals.set("CONDITION_PARAM_SOULGAIN", 12i32)?; // enums.h:146
    globals.set("CONDITION_PARAM_SOULTICKS", 13i32)?; // enums.h:147
    // enums.h:179 — per-channel sub-id (channel id); used by CH-5 mute blocks.
    globals.set("CONDITION_PARAM_SUBID", 45i32)?; // enums.h:179
    Ok(())
}

// --- RETURNVALUE_* (enums.h:300-370 ReturnValue_t, 772 numbering) ---
//
// The full 772 `ReturnValue` enum block. The 772 numbering diverges from TFS
// 1.4.2 (1098) after position 56 (`YouNeedAMagicItemToCastSpell`): 772 omits
// `CannotConjureItemHere` and `YouNeedToSplitYourSpears`, so codes 57+ shift
// down by 2. Only `RETURNVALUE_PLAYERWITHTHISNAMEISNOTONLINE` (27) is
// referenced by the channel scripts today, but the full block is registered
// for API completeness and future script compatibility.

fn register_return_values(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // enums.h:300-370 — `ReturnValue` enum (772 era, 0-indexed sequential).
    globals.set("RETURNVALUE_NOERROR", 0i32)?;
    globals.set("RETURNVALUE_NOTPOSSIBLE", 1i32)?;
    globals.set("RETURNVALUE_NOTENOUGHROOM", 2i32)?;
    globals.set("RETURNVALUE_PLAYERISPZLOCKED", 3i32)?;
    globals.set("RETURNVALUE_PLAYERISNOTINVITED", 4i32)?;
    globals.set("RETURNVALUE_CANNOTTHROW", 5i32)?;
    globals.set("RETURNVALUE_THEREISNOWAY", 6i32)?;
    globals.set("RETURNVALUE_DESTINATIONOUTOFREACH", 7i32)?;
    globals.set("RETURNVALUE_CREATUREBLOCK", 8i32)?;
    globals.set("RETURNVALUE_NOTMOVEABLE", 9i32)?;
    globals.set("RETURNVALUE_DROPTWOHANDEDITEM", 10i32)?;
    globals.set("RETURNVALUE_BOTHHANDSNEEDTOBEFREE", 11i32)?;
    globals.set("RETURNVALUE_CANONLYUSEONEWEAPON", 12i32)?;
    globals.set("RETURNVALUE_NEEDEXCHANGE", 13i32)?;
    globals.set("RETURNVALUE_CANNOTBEDRESSED", 14i32)?;
    globals.set("RETURNVALUE_PUTTHISOBJECTINYOURHAND", 15i32)?;
    globals.set("RETURNVALUE_PUTTHISOBJECTINBOTHHANDS", 16i32)?;
    globals.set("RETURNVALUE_TOOFARAWAY", 17i32)?;
    globals.set("RETURNVALUE_FIRSTGODOWNSTAIRS", 18i32)?;
    globals.set("RETURNVALUE_FIRSTGOUPSTAIRS", 19i32)?;
    globals.set("RETURNVALUE_CONTAINERNOTENOUGHROOM", 20i32)?;
    globals.set("RETURNVALUE_NOTENOUGHCAPACITY", 21i32)?;
    globals.set("RETURNVALUE_CANNOTPICKUP", 22i32)?;
    globals.set("RETURNVALUE_THISISIMPOSSIBLE", 23i32)?;
    globals.set("RETURNVALUE_DEPOTISFULL", 24i32)?;
    globals.set("RETURNVALUE_CREATUREDOESNOTEXIST", 25i32)?;
    globals.set("RETURNVALUE_CANNOTUSETHISOBJECT", 26i32)?;
    // 27 — referenced by help.lua `!mute`/`!unmute` cancel path.
    globals.set("RETURNVALUE_PLAYERWITHTHISNAMEISNOTONLINE", 27i32)?;
    globals.set("RETURNVALUE_YOUAREALREADYTRADING", 28i32)?;
    globals.set("RETURNVALUE_THISPLAYERISALREADYTRADING", 29i32)?;
    globals.set("RETURNVALUE_YOUMAYNOTLOGOUTDURINGAFIGHT", 30i32)?;
    globals.set("RETURNVALUE_DIRECTPLAYERSHOOT", 31i32)?;
    globals.set("RETURNVALUE_NOTENOUGHLEVEL", 32i32)?;
    globals.set("RETURNVALUE_NOTENOUGHMAGICLEVEL", 33i32)?;
    globals.set("RETURNVALUE_NOTENOUGHMANA", 34i32)?;
    globals.set("RETURNVALUE_NOTENOUGHSOUL", 35i32)?;
    globals.set("RETURNVALUE_YOUAREEXHAUSTED", 36i32)?;
    globals.set("RETURNVALUE_YOUCANNOTUSEOBJECTSTHATFAST", 37i32)?;
    globals.set("RETURNVALUE_PLAYERISNOTREACHABLE", 38i32)?;
    globals.set("RETURNVALUE_CANONLYUSETHISRUNEONCREATURES", 39i32)?;
    globals.set("RETURNVALUE_ACTIONNOTPERMITTEDINPROTECTIONZONE", 40i32)?;
    globals.set("RETURNVALUE_YOUMAYNOTATTACKTHISPLAYER", 41i32)?;
    globals.set("RETURNVALUE_YOUMAYNOTATTACKAPERSONINPROTECTIONZONE", 42i32)?;
    globals.set(
        "RETURNVALUE_YOUMAYNOTATTACKAPERSONWHILEINPROTECTIONZONE",
        43i32,
    )?;
    globals.set("RETURNVALUE_YOUMAYNOTATTACKTHISCREATURE", 44i32)?;
    globals.set("RETURNVALUE_YOUCANONLYUSEITONCREATURES", 45i32)?;
    globals.set("RETURNVALUE_CREATUREISNOTREACHABLE", 46i32)?;
    globals.set("RETURNVALUE_TURNSECUREMODETOATTACKUNMARKEDPLAYERS", 47i32)?;
    globals.set("RETURNVALUE_YOUNEEDPREMIUMACCOUNT", 48i32)?;
    globals.set("RETURNVALUE_YOUNEEDTOLEARNTHISSPELL", 49i32)?;
    globals.set("RETURNVALUE_YOURVOCATIONCANNOTUSETHISSPELL", 50i32)?;
    globals.set("RETURNVALUE_YOUNEEDAWEAPONTOUSETHISSPELL", 51i32)?;
    globals.set("RETURNVALUE_PLAYERISPZLOCKEDLEAVEPVPZONE", 52i32)?;
    globals.set("RETURNVALUE_PLAYERISPZLOCKEDENTERPVPZONE", 53i32)?;
    globals.set("RETURNVALUE_ACTIONNOTPERMITTEDINANOPVPZONE", 54i32)?;
    globals.set("RETURNVALUE_YOUCANNOTLOGOUTHERE", 55i32)?;
    globals.set("RETURNVALUE_YOUNEEDAMAGICITEMTOCASTSPELL", 56i32)?;
    globals.set("RETURNVALUE_NAMEISTOOAMBIGUOUS", 57i32)?;
    globals.set("RETURNVALUE_CANONLYUSEONESHIELD", 58i32)?;
    globals.set("RETURNVALUE_NOPARTYMEMBERSINRANGE", 59i32)?;
    globals.set("RETURNVALUE_YOUARENOTTHEOWNER", 60i32)?;
    globals.set("RETURNVALUE_NOSUCHRAIDEXISTS", 61i32)?;
    globals.set("RETURNVALUE_ANOTHERRAIDISALREADYEXECUTING", 62i32)?;
    globals.set("RETURNVALUE_TRADEPLAYERFARAWAY", 63i32)?;
    globals.set("RETURNVALUE_YOUDONTOWNTHISHOUSE", 64i32)?;
    globals.set("RETURNVALUE_TRADEPLAYERALREADYOWNSAHOUSE", 65i32)?;
    globals.set("RETURNVALUE_TRADEPLAYERHIGHESTBIDDER", 66i32)?;
    globals.set("RETURNVALUE_YOUCANNOTTRADETHISHOUSE", 67i32)?;
    globals.set("RETURNVALUE_YOUDONTHAVEREQUIREDPROFESSION", 68i32)?;
    globals.set("RETURNVALUE_ITEMCANNOTBEMOVEDTHERE", 69i32)?;
    Ok(())
}

// --- ITEM_GROUP_* (`itemloader.h` itemgroup_t) ---
//
// Doors Phase 6: `closing_doors.lua` / `doRelocate` use SPLASH + MAGICFIELD.

fn register_item_groups(globals: &mlua::Table) -> Result<(), mlua::Error> {
    globals.set("ITEM_GROUP_NONE", 0i32)?;
    globals.set("ITEM_GROUP_GROUND", 1i32)?;
    globals.set("ITEM_GROUP_CONTAINER", 2i32)?;
    globals.set("ITEM_GROUP_WEAPON", 3i32)?;
    globals.set("ITEM_GROUP_AMMUNITION", 4i32)?;
    globals.set("ITEM_GROUP_ARMOR", 5i32)?;
    globals.set("ITEM_GROUP_CHARGES", 6i32)?;
    globals.set("ITEM_GROUP_TELEPORT", 7i32)?;
    globals.set("ITEM_GROUP_MAGICFIELD", 8i32)?;
    globals.set("ITEM_GROUP_WRITEABLE", 9i32)?;
    globals.set("ITEM_GROUP_KEY", 10i32)?;
    globals.set("ITEM_GROUP_SPLASH", 11i32)?;
    globals.set("ITEM_GROUP_FLUID", 12i32)?;
    globals.set("ITEM_GROUP_DOOR", 13i32)?;
    globals.set("ITEM_GROUP_DEPRECATED", 14i32)?;
    Ok(())
}

// --- ITEM_ATTRIBUTE_* (enums.h:51-84 itemAttrTypes bitflags) ---
//
// PC-3a Phase 5: `conjureItem` / `destroyItem` check duration / unique / action id.
// Doors Phase 3: Remere key/door fields are **string** aliases → custom attrs
// (`remere_attr::*` / OTBM map load). TVP bitflags collide with TFS 1.4.2 STORE_ITEM+.

fn register_item_attributes(globals: &mlua::Table) -> Result<(), mlua::Error> {
    globals.set("ITEM_ATTRIBUTE_NONE", 0i32)?;
    globals.set("ITEM_ATTRIBUTE_ACTIONID", 1i32 << 0)?;
    globals.set("ITEM_ATTRIBUTE_UNIQUEID", 1i32 << 1)?;
    globals.set("ITEM_ATTRIBUTE_DESCRIPTION", 1i32 << 2)?;
    globals.set("ITEM_ATTRIBUTE_TEXT", 1i32 << 3)?;
    globals.set("ITEM_ATTRIBUTE_DURATION", 1i32 << 17)?;
    globals.set("ITEM_ATTRIBUTE_CHARGES", 1i32 << 20)?;
    // Remere OTBM / doors.lua — string custom-attr aliases (not TFS bitflags).
    globals.set(
        "ITEM_ATTRIBUTE_KEYNUMBER",
        tfs_rust_common::remere_attr::KEYNUMBER,
    )?;
    globals.set(
        "ITEM_ATTRIBUTE_KEYHOLENUMBER",
        tfs_rust_common::remere_attr::KEYHOLENUMBER,
    )?;
    globals.set(
        "ITEM_ATTRIBUTE_DOORQUESTNUMBER",
        tfs_rust_common::remere_attr::DOORQUESTNUMBER,
    )?;
    globals.set(
        "ITEM_ATTRIBUTE_DOORQUESTVALUE",
        tfs_rust_common::remere_attr::DOORQUESTVALUE,
    )?;
    globals.set(
        "ITEM_ATTRIBUTE_DOORLEVEL",
        tfs_rust_common::remere_attr::DOORLEVEL,
    )?;
    Ok(())
}

// --- Misc globals previously set as constants in bootstrap ---

fn register_misc(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // `APPLY_SKILL_MULTIPLIER` — read by `data/events/scripts/player.lua`
    // `onGainSkillTries` to gate the skill-rate multiplier path. TFS default
    // is `true`; kept as a boolean global (not an enum).
    globals.set("APPLY_SKILL_MULTIPLIER", true)?;
    // TFS `house.h` `GUEST_LIST` / `SUBOWNER_LIST` — access-list window ids.
    globals.set("GUEST_LIST", 0x100i32)?;
    globals.set("SUBOWNER_LIST", 0x101i32)?;
    Ok(())
}

fn register_game_states(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // TVP `GameState_t` — `luaGameSetGameState`.
    globals.set("GAME_STATE_STARTUP", 0i32)?;
    globals.set("GAME_STATE_INIT", 1i32)?;
    globals.set("GAME_STATE_NORMAL", 2i32)?;
    globals.set("GAME_STATE_CLOSED", 3i32)?;
    globals.set("GAME_STATE_SHUTDOWN", 4i32)?;
    globals.set("GAME_STATE_CLOSING", 5i32)?;
    globals.set("GAME_STATE_MAINTAIN", 6i32)?;
    Ok(())
}

fn register_reload_types(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // TVP `ReloadTypes_t` without mounts (talkactions `/reload`).
    globals.set("RELOAD_TYPE_ALL", 0i32)?;
    globals.set("RELOAD_TYPE_ACTIONS", 1i32)?;
    globals.set("RELOAD_TYPE_CHAT", 2i32)?;
    globals.set("RELOAD_TYPE_CONFIG", 3i32)?;
    globals.set("RELOAD_TYPE_CREATURESCRIPTS", 4i32)?;
    globals.set("RELOAD_TYPE_EVENTS", 5i32)?;
    globals.set("RELOAD_TYPE_GLOBAL", 6i32)?;
    globals.set("RELOAD_TYPE_GLOBALEVENTS", 7i32)?;
    globals.set("RELOAD_TYPE_ITEMS", 8i32)?;
    globals.set("RELOAD_TYPE_MONSTERS", 9i32)?;
    globals.set("RELOAD_TYPE_MOVEMENTS", 10i32)?;
    globals.set("RELOAD_TYPE_NPCS", 11i32)?;
    globals.set("RELOAD_TYPE_RAIDS", 12i32)?;
    globals.set("RELOAD_TYPE_SCRIPTS", 13i32)?;
    globals.set("RELOAD_TYPE_SPELLS", 14i32)?;
    globals.set("RELOAD_TYPE_TALKACTIONS", 15i32)?;
    globals.set("RELOAD_TYPE_WEAPONS", 16i32)?;
    Ok(())
}

/// `configKeys` table — TVP `configmanager.h` enum indices used by
/// `configManager.getBoolean` / `getNumber(configKeys.…)`.
fn register_config_keys(lua: &Lua) -> Result<(), mlua::Error> {
    let keys = lua.create_table()?;
    // `boolean_config_t::FREE_PREMIUM` — index 7 in TVP `configmanager.h`.
    keys.set("FREE_PREMIUM", 7i32)?;
    // TVP `boolean_config_t::GUILHALLS_ONLYFOR_LEADERS` / `HOUSES_ONLY_PREMIUM`.
    keys.set("GUILHALLS_ONLYFOR_LEADERS", 49i32)?;
    keys.set("HOUSES_ONLY_PREMIUM", 50i32)?;
    keys.set("USE_HOUSE_AREA_PRICES", 51i32)?;
    // TVP `integer_config_t::HOUSE_PRICE` → `housePriceEachSQM`.
    keys.set("HOUSE_PRICE", 8i32)?;
    // TVP `integer_config_t`.
    keys.set("RATE_EXPERIENCE", 3i32)?;
    keys.set("RATE_SKILL", 4i32)?;
    keys.set("RATE_LOOT", 5i32)?;
    keys.set("RATE_MAGIC", 6i32)?;
    keys.set("KILLS_DAY_RED_SKULL", 30i32)?;
    keys.set("KILLS_WEEK_RED_SKULL", 31i32)?;
    keys.set("KILLS_MONTH_RED_SKULL", 32i32)?;
    keys.set("KILLS_DAY_BANISHMENT", 33i32)?;
    keys.set("KILLS_WEEK_BANISHMENT", 34i32)?;
    keys.set("KILLS_MONTH_BANISHMENT", 35i32)?;
    lua.globals().set("configKeys", keys)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_constants_sets_expected_values() {
        let lua = Lua::new();
        register_constants(&lua).expect("register_constants");

        let globals = lua.globals();
        let get = |name: &str| globals.get::<i32>(name).expect(name);

        // ACCOUNT_TYPE_* (enums.h:80-85)
        assert_eq!(get("ACCOUNT_TYPE_NORMAL"), 1);
        assert_eq!(get("ACCOUNT_TYPE_TUTOR"), 2);
        assert_eq!(get("ACCOUNT_TYPE_SENIORTUTOR"), 3);
        assert_eq!(get("ACCOUNT_TYPE_GAMEMASTER"), 4);
        assert_eq!(get("ACCOUNT_TYPE_COMMUNITYMANAGER"), 5);
        assert_eq!(get("ACCOUNT_TYPE_GOD"), 6);

        assert_eq!(get("GAME_STATE_NORMAL"), 2);
        assert_eq!(get("GAME_STATE_CLOSED"), 3);
        assert_eq!(get("GAME_STATE_SHUTDOWN"), 4);
        assert_eq!(get("RELOAD_TYPE_TALKACTIONS"), 15);

        // TALKTYPE_* (const.h:62-76 — 772 values)
        assert_eq!(get("TALKTYPE_SAY"), 1);
        assert_eq!(get("TALKTYPE_CHANNEL_Y"), 5);
        assert_eq!(get("TALKTYPE_CHANNEL_R1"), 10);
        assert_eq!(get("TALKTYPE_CHANNEL_O"), 12);
        assert_eq!(get("TALKTYPE_CHANNEL_R2"), 14);
        assert_eq!(get("TALKTYPE_MONSTER_SAY"), 0x11);

        // FLUID_* (const.h:94-106 — 772 sequential)
        assert_eq!(get("FLUID_NONE"), 0);
        assert_eq!(get("FLUID_WATER"), 1);
        assert_eq!(get("FLUID_WINE"), 2);
        assert_eq!(get("FLUID_BEER"), 3);
        assert_eq!(get("FLUID_MUD"), 4);
        assert_eq!(get("FLUID_BLOOD"), 5);
        assert_eq!(get("FLUID_SLIME"), 6);
        assert_eq!(get("FLUID_OIL"), 7);
        assert_eq!(get("FLUID_URINE"), 8);
        assert_eq!(get("FLUID_MILK"), 9);
        assert_eq!(get("FLUID_MANAFLUID"), 10);
        assert_eq!(get("FLUID_LIFEFLUID"), 11);
        assert_eq!(get("FLUID_LEMONADE"), 12);

        // PlayerFlag_* (const.h:264-266, 506, 516)
        assert_eq!(get("PlayerFlag_CannotUseCombat"), 1 << 0);
        assert_eq!(get("PlayerFlag_CannotAttackPlayer"), 1 << 1);
        assert_eq!(get("PlayerFlag_CannotAttackMonster"), 1 << 2);
        assert_eq!(get("PlayerFlag_CannotBeAttacked"), 1 << 3);
        assert_eq!(get("PlayerFlag_CanTalkRedPrivate"), 1 << 21);
        assert_eq!(get("PlayerFlag_CanTalkRedChannel"), 1 << 22);
        assert_eq!(get("PlayerFlag_TalkOrangeHelpChannel"), 1 << 23);
        assert_eq!(
            globals
                .get::<i64>("PlayerFlag_HasInfiniteMana")
                .expect("PlayerFlag_HasInfiniteMana"),
            1 << 10
        );
        assert_eq!(
            globals
                .get::<i64>("PlayerFlag_HasInfiniteCapacity")
                .expect("PlayerFlag_HasInfiniteCapacity"),
            1 << 20
        );
        assert_eq!(
            globals
                .get::<i64>("PlayerFlag_IsAlwaysPremium")
                .expect("PlayerFlag_IsAlwaysPremium"),
            1 << 35
        );

        let config_keys: mlua::Table = globals.get("configKeys").expect("configKeys");
        assert_eq!(
            config_keys
                .get::<i32>("FREE_PREMIUM")
                .expect("FREE_PREMIUM"),
            7
        );
        assert_eq!(
            config_keys
                .get::<i32>("GUILHALLS_ONLYFOR_LEADERS")
                .expect("GUILHALLS_ONLYFOR_LEADERS"),
            49
        );
        assert_eq!(
            config_keys
                .get::<i32>("HOUSES_ONLY_PREMIUM")
                .expect("HOUSES_ONLY_PREMIUM"),
            50
        );
        assert_eq!(
            config_keys
                .get::<i32>("USE_HOUSE_AREA_PRICES")
                .expect("USE_HOUSE_AREA_PRICES"),
            51
        );
        assert_eq!(
            config_keys.get::<i32>("HOUSE_PRICE").expect("HOUSE_PRICE"),
            8
        );
        assert_eq!(get("ITEM_ATTRIBUTE_ACTIONID"), 1 << 0);
        assert_eq!(get("ITEM_ATTRIBUTE_UNIQUEID"), 1 << 1);
        assert_eq!(get("ITEM_ATTRIBUTE_DURATION"), 1 << 17);
        assert_eq!(
            globals
                .get::<String>("ITEM_ATTRIBUTE_KEYNUMBER")
                .expect("ITEM_ATTRIBUTE_KEYNUMBER"),
            "keynumber"
        );
        assert_eq!(
            globals
                .get::<String>("ITEM_ATTRIBUTE_KEYHOLENUMBER")
                .expect("ITEM_ATTRIBUTE_KEYHOLENUMBER"),
            "keyholenumber"
        );
        assert_eq!(
            globals
                .get::<String>("ITEM_ATTRIBUTE_DOORQUESTNUMBER")
                .expect("ITEM_ATTRIBUTE_DOORQUESTNUMBER"),
            "doorquestnumber"
        );
        assert_eq!(
            globals
                .get::<String>("ITEM_ATTRIBUTE_DOORQUESTVALUE")
                .expect("ITEM_ATTRIBUTE_DOORQUESTVALUE"),
            "doorquestvalue"
        );
        assert_eq!(
            globals
                .get::<String>("ITEM_ATTRIBUTE_DOORLEVEL")
                .expect("ITEM_ATTRIBUTE_DOORLEVEL"),
            "doorlevel"
        );

        // VOCATION_NONE (enums.h:297)
        assert_eq!(get("VOCATION_NONE"), 0);

        assert_eq!(get("GUEST_LIST"), 0x100);
        assert_eq!(get("SUBOWNER_LIST"), 0x101);

        // CONDITION_* (enums.h:266-268,275,136,146,147,179)
        assert_eq!(get("CONDITION_SOUL"), 1 << 13);
        assert_eq!(get("CONDITION_MUTED"), 1 << 14);
        assert_eq!(get("CONDITION_CHANNELMUTEDTICKS"), 1 << 15);
        assert_eq!(get("CONDITIONID_DEFAULT"), -1);
        assert_eq!(get("CONDITION_PARAM_TICKS"), 2);
        assert_eq!(get("CONDITION_PARAM_SOULGAIN"), 12);
        assert_eq!(get("CONDITION_PARAM_SOULTICKS"), 13);
        assert_eq!(get("CONDITION_PARAM_SUBID"), 45);

        // RETURNVALUE_* (enums.h:300-370, 772 numbering)
        assert_eq!(get("RETURNVALUE_NOERROR"), 0);
        assert_eq!(get("RETURNVALUE_PLAYERWITHTHISNAMEISNOTONLINE"), 27);
        assert_eq!(get("RETURNVALUE_YOUAREEXHAUSTED"), 36);
        assert_eq!(get("RETURNVALUE_ITEMCANNOTBEMOVEDTHERE"), 69);

        // ITEM_GROUP_* (itemloader.h)
        assert_eq!(get("ITEM_GROUP_MAGICFIELD"), 8);
        assert_eq!(get("ITEM_GROUP_SPLASH"), 11);

        // APPLY_SKILL_MULTIPLIER
        assert!(
            globals
                .get::<bool>("APPLY_SKILL_MULTIPLIER")
                .expect("APPLY_SKILL_MULTIPLIER")
        );
    }
}
