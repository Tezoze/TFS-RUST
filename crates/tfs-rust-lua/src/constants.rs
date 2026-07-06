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
    register_player_flags(&globals)?;
    register_vocations(&globals)?;
    register_conditions(&globals)?;
    register_return_values(&globals)?;
    register_misc(&globals)?;

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
    // const.h:62-76 — channel scripts reference CHANNEL_Y/O/R1/R2.
    globals.set("TALKTYPE_CHANNEL_Y", 5i32)?; // const.h:66 — Yellow
    globals.set("TALKTYPE_CHANNEL_R1", 10i32)?; // const.h:71 — Red (#c text)
    globals.set("TALKTYPE_CHANNEL_O", 12i32)?; // const.h:73 — orange
    globals.set("TALKTYPE_CHANNEL_R2", 14i32)?; // const.h:74 — red anonymous (#d text)
    Ok(())
}

// --- PlayerFlag_* (const.h:264-266, 772 values) ---

fn register_player_flags(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // const.h:264-266 — per-player permission flags tested via `player:hasFlag`.
    globals.set("PlayerFlag_CanTalkRedPrivate", 1i32 << 21)?; // const.h:264
    globals.set("PlayerFlag_CanTalkRedChannel", 1i32 << 22)?; // const.h:265
    globals.set("PlayerFlag_TalkOrangeHelpChannel", 1i32 << 23)?; // const.h:266
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
    // enums.h:275 — was previously registered as `0` (wrong); fixed to `-1`.
    // See lua-api-plan.md §4.3.
    globals.set("CONDITIONID_DEFAULT", -1i32)?; // enums.h:275
    // enums.h:146-147 — soul regen params set on the (currently no-op) stub.
    globals.set("CONDITION_PARAM_SOULGAIN", 12i32)?; // enums.h:146
    globals.set("CONDITION_PARAM_SOULTICKS", 13i32)?; // enums.h:147
    Ok(())
}

// --- RETURNVALUE_* (enums.h ReturnValue_t) ---
//
// Only `RETURNVALUE_NOERROR` is referenced by active scripts today; the full
// enum block (including `RETURNVALUE_PLAYERWITHTHISNAMEISNOTONLINE` for the
// CH-5 `!mute`/`!unmute` cancel path) lands in LUA-4.

fn register_return_values(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // enums.h ReturnValue_t — `RETURNVALUE_NOERROR` is the success sentinel
    // returned by `data/events/scripts/player.lua` `onLogout`.
    globals.set("RETURNVALUE_NOERROR", 0i32)?;
    Ok(())
}

// --- Misc globals previously set as constants in bootstrap ---

fn register_misc(globals: &mlua::Table) -> Result<(), mlua::Error> {
    // `APPLY_SKILL_MULTIPLIER` — read by `data/events/scripts/player.lua`
    // `onGainSkillTries` to gate the skill-rate multiplier path. TFS default
    // is `true`; kept as a boolean global (not an enum).
    globals.set("APPLY_SKILL_MULTIPLIER", true)?;
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

        // TALKTYPE_* (const.h:66,71,73,74 — 772 values)
        assert_eq!(get("TALKTYPE_CHANNEL_Y"), 5);
        assert_eq!(get("TALKTYPE_CHANNEL_R1"), 10);
        assert_eq!(get("TALKTYPE_CHANNEL_O"), 12);
        assert_eq!(get("TALKTYPE_CHANNEL_R2"), 14);

        // PlayerFlag_* (const.h:264-266)
        assert_eq!(get("PlayerFlag_CanTalkRedPrivate"), 1 << 21);
        assert_eq!(get("PlayerFlag_CanTalkRedChannel"), 1 << 22);
        assert_eq!(get("PlayerFlag_TalkOrangeHelpChannel"), 1 << 23);

        // VOCATION_NONE (enums.h:297)
        assert_eq!(get("VOCATION_NONE"), 0);

        // CONDITION_* (enums.h:266,275,146,147) — incl. the CONDITIONID_DEFAULT fix.
        assert_eq!(get("CONDITION_SOUL"), 1 << 13);
        assert_eq!(get("CONDITIONID_DEFAULT"), -1);
        assert_eq!(get("CONDITION_PARAM_SOULGAIN"), 12);
        assert_eq!(get("CONDITION_PARAM_SOULTICKS"), 13);

        // RETURNVALUE_NOERROR
        assert_eq!(get("RETURNVALUE_NOERROR"), 0);

        // APPLY_SKILL_MULTIPLIER
        assert!(globals
            .get::<bool>("APPLY_SKILL_MULTIPLIER")
            .expect("APPLY_SKILL_MULTIPLIER"));
    }
}
