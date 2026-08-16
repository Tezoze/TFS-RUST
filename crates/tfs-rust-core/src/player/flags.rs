//! `PlayerFlag` bits from `groups.xml` — `src/const.h`, `src/groups.cpp`.
// C++ reference: `PlayerFlags` enum, `Group::parseFlag`.

use tfs_rust_content::groups::GroupDatabase;

/// C++ `PlayerFlag_CannotUseCombat` — `src/const.h`. 772 equivalent: `NO_ATTACK` right
/// (`enums.hh:524`, `crcombat.cc:391,589`). Blocks all attack actions.
pub const PLAYER_FLAG_CANNOT_USE_COMBAT: u64 = 1 << 0;
/// C++ `PlayerFlag_CannotAttackPlayer` — `src/const.h` bit 1. Access groups (GM/God)
/// must not initiate PvP (melee, runes, spells, AoE). No 772 `RIGHT` equivalent —
/// TFS-domain flag; 772 GMs used `NO_ATTACK` for *all* combat.
pub const PLAYER_FLAG_CANNOT_ATTACK_PLAYER: u64 = 1 << 1;
/// C++ `PlayerFlag_CannotAttackMonster` — `src/const.h` bit 2.
pub const PLAYER_FLAG_CANNOT_ATTACK_MONSTER: u64 = 1 << 2;
/// C++ `PlayerFlag_CannotBeAttacked` — `src/const.h`. 772 equivalent: `INVULNERABLE` right
/// (`enums.hh:517`, `crmain.cc:536-538`). Zeroes incoming damage to the target.
pub const PLAYER_FLAG_CANNOT_BE_ATTACKED: u64 = 1 << 3;
/// C++ `PlayerFlag_CannotPickupItem` — `src/const.h`
pub const PLAYER_FLAG_CANNOT_PICKUP_ITEM: u64 = 1 << 14;
/// C++ `PlayerFlag_HasInfiniteCapacity` — `src/const.h`
pub const PLAYER_FLAG_HAS_INFINITE_CAPACITY: u64 = 1 << 20;
/// C++ `PlayerFlag_HasInfiniteMana` — `src/const.h`. 772 equivalent: `UNLIMITED_MANA`
/// (`enums.hh:518`, `magic.cc:753` `CheckMana`).
pub const PLAYER_FLAG_HAS_INFINITE_MANA: u64 = 1 << 10;
/// C++ `PlayerFlag_HasInfiniteSoul` — `src/const.h`.
pub const PLAYER_FLAG_HAS_INFINITE_SOUL: u64 = 1 << 11;
/// C++ `PlayerFlag_HasNoExhaustion` — `src/const.h`.
pub const PLAYER_FLAG_HAS_NO_EXHAUSTION: u64 = 1 << 12;
/// C++ `PlayerFlag_CannotUseSpells` — `src/const.h`.
pub const PLAYER_FLAG_CANNOT_USE_SPELLS: u64 = 1 << 13;
/// C++ `PlayerFlag_IgnoredByMonsters` — `src/const.h`
pub const PLAYER_FLAG_IGNORED_BY_MONSTERS: u64 = 1 << 8;
/// C++ `PlayerFlag_NotGainInFight` — `src/const.h`. 772 equivalent: `NO_LOGOUT_BLOCK`
/// (`enums.hh:526`, `crmain.cc:438` `BlockLogout`). Skips Infight / PZ lock / logout block.
pub const PLAYER_FLAG_NOT_GAIN_IN_FIGHT: u64 = 1 << 9;
/// C++ `PlayerFlag_CanBroadcast` — `src/const.h`
pub const PLAYER_FLAG_CAN_BROADCAST: u64 = 1 << 16;
/// C++ `PlayerFlag_CanEditHouses` — `src/const.h` bit 17.
pub const PLAYER_FLAG_CAN_EDIT_HOUSES: u64 = 1 << 17;
/// C++ `PlayerFlag_CanPushAllCreatures` — `src/const.h:517` bit 21.
/// **772 deviation:** 772 has no push bypass for GMs (no push-related `RIGHT` in
/// `enums.hh:455-534`; `CheckMoveObject`/`CheckMapDestination`/`MovePossible` never call
/// `CheckRight` for the actor). This flag is a TFS-domain-shape feature that lets access
/// groups bypass Gate A (race unpushable) + Gate B (per-creature `MovePossible`) when pushing
/// creatures. Gate C (range cap, elevation, AVOID, PZ→non-PZ, `ThrowPossible`) still applies.
pub const PLAYER_FLAG_CAN_PUSH_ALL_CREATURES: u64 = 1 << 21;
/// C++ `PlayerFlag_CanTalkRedPrivate` — `src/const.h`
pub const PLAYER_FLAG_CAN_TALK_RED_PRIVATE: u64 = 1 << 22;
/// C++ `PlayerFlag_CannotBeMuted` — `src/const.h`
pub const PLAYER_FLAG_CANNOT_BE_MUTED: u64 = 1 << 36;
/// C++ `PlayerFlag_SetMaxSpeed` — `src/const.h`. When set, `Player::updateBaseSpeed`
/// caps the character's base speed at `PLAYER_MAX_SPEED` (1500).
pub const PLAYER_FLAG_SET_MAX_SPEED: u64 = 1 << 29;
/// C++ `PlayerFlag_IgnoreProtectionZone` — `src/const.h` bit 33. 772 equivalent:
/// `ATTACK_EVERYWHERE` (`enums.hh:523`, `crcombat.cc:383–410` PZ / vocation / NoPvp bypass).
pub const PLAYER_FLAG_IGNORE_PROTECTION_ZONE: u64 = 1 << 33;
/// C++ `PlayerFlag_IgnoreSpellCheck` — `src/const.h` bit 34. 772 equivalent:
/// `ALL_SPELLS` (`enums.hh:520`, `magic.cc:619` `CheckSpellbook`).
pub const PLAYER_FLAG_IGNORE_SPELL_CHECK: u64 = 1 << 34;
/// C++ `PlayerFlag_IsAlwaysPremium` — `src/const.h` bit 35. Group flag
/// `isalwayspremium` → treat as premium regardless of `accounts.premium_ends_at`.
pub const PLAYER_FLAG_IS_ALWAYS_PREMIUM: u64 = 1 << 35;
/// 772 `KEEP_INVENTORY` right (`enums.hh:519`, `crplayer.cc:299-300`).
/// GMs with this right keep their inventory on death (`LOSE_INVENTORY_NONE`).
pub const PLAYER_FLAG_KEEP_INVENTORY: u64 = 1 << 37;

/// Map `groups.xml` / `groups.lua` flag keys to `PlayerFlags` bits (subset used by core).
fn flag_name_to_bit(name: &str) -> Option<u64> {
    match name.to_ascii_lowercase().as_str() {
        "cannotusecombat" => Some(PLAYER_FLAG_CANNOT_USE_COMBAT),
        "cannotattackplayer" => Some(PLAYER_FLAG_CANNOT_ATTACK_PLAYER),
        "cannotattackmonster" => Some(PLAYER_FLAG_CANNOT_ATTACK_MONSTER),
        "cannotbeattacked" => Some(PLAYER_FLAG_CANNOT_BE_ATTACKED),
        "cannotpickupitem" => Some(PLAYER_FLAG_CANNOT_PICKUP_ITEM),
        "hasinfinitecapacity" => Some(PLAYER_FLAG_HAS_INFINITE_CAPACITY),
        "hasinfinitemana" => Some(PLAYER_FLAG_HAS_INFINITE_MANA),
        "hasinfinitesoul" => Some(PLAYER_FLAG_HAS_INFINITE_SOUL),
        "hasnoexhaustion" => Some(PLAYER_FLAG_HAS_NO_EXHAUSTION),
        "cannotusespells" => Some(PLAYER_FLAG_CANNOT_USE_SPELLS),
        "ignoredbymonsters" => Some(PLAYER_FLAG_IGNORED_BY_MONSTERS),
        "notgaininfight" => Some(PLAYER_FLAG_NOT_GAIN_IN_FIGHT),
        "canbroadcast" => Some(PLAYER_FLAG_CAN_BROADCAST),
        "canedithouses" => Some(PLAYER_FLAG_CAN_EDIT_HOUSES),
        "canpushallcreatures" => Some(PLAYER_FLAG_CAN_PUSH_ALL_CREATURES),
        "cantalkredprivate" => Some(PLAYER_FLAG_CAN_TALK_RED_PRIVATE),
        "cannotbemuted" => Some(PLAYER_FLAG_CANNOT_BE_MUTED),
        "setmaxspeed" => Some(PLAYER_FLAG_SET_MAX_SPEED),
        "ignoreprotectionzone" => Some(PLAYER_FLAG_IGNORE_PROTECTION_ZONE),
        "ignorespellcheck" => Some(PLAYER_FLAG_IGNORE_SPELL_CHECK),
        "isalwayspremium" => Some(PLAYER_FLAG_IS_ALWAYS_PREMIUM),
        "keepinventory" => Some(PLAYER_FLAG_KEEP_INVENTORY),
        _ => None,
    }
}

/// Resolve enabled flags for a group id from loaded `groups.xml`.
pub fn flags_for_group(groups: &GroupDatabase, group_id: u16) -> u64 {
    let Some(group) = groups.groups.get(&group_id) else {
        return 0;
    };
    let mut bits = 0u64;
    for (name, &enabled) in &group.flags {
        if enabled && let Some(bit) = flag_name_to_bit(name) {
            bits |= bit;
        }
    }
    bits
}

#[inline]
pub fn has_player_flag(flags: u64, flag: u64) -> bool {
    flags & flag != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tfs_rust_content::groups::Group;

    fn make_group(id: u16, flags: &[(&str, bool)]) -> GroupDatabase {
        let mut map = HashMap::new();
        let mut flag_map = HashMap::new();
        for (name, enabled) in flags {
            flag_map.insert(name.to_string(), *enabled);
        }
        map.insert(
            id,
            Group {
                id,
                name: "test".to_string(),
                access: true,
                max_depot_items: 0,
                max_vip_entries: 0,
                flags: flag_map,
            },
        );
        GroupDatabase { groups: map }
    }

    #[test]
    fn cannot_attack_player_flag_resolves_from_group() {
        let groups = make_group(4, &[("cannotattackplayer", true)]);
        let flags = flags_for_group(&groups, 4);
        assert!(has_player_flag(flags, PLAYER_FLAG_CANNOT_ATTACK_PLAYER));
        assert!(!has_player_flag(flags, PLAYER_FLAG_CANNOT_ATTACK_MONSTER));
    }

    #[test]
    fn cannot_attack_monster_flag_resolves_from_group() {
        let groups = make_group(4, &[("cannotattackmonster", true)]);
        let flags = flags_for_group(&groups, 4);
        assert!(has_player_flag(flags, PLAYER_FLAG_CANNOT_ATTACK_MONSTER));
    }

    #[test]
    fn set_max_speed_flag_resolves_from_group() {
        let groups = make_group(6, &[("setmaxspeed", true)]);
        let flags = flags_for_group(&groups, 6);
        assert!(has_player_flag(flags, PLAYER_FLAG_SET_MAX_SPEED));
    }

    #[test]
    fn set_max_speed_flag_absent_when_disabled() {
        let groups = make_group(6, &[("setmaxspeed", false)]);
        let flags = flags_for_group(&groups, 6);
        assert!(!has_player_flag(flags, PLAYER_FLAG_SET_MAX_SPEED));
    }

    #[test]
    fn has_infinite_mana_flag_resolves_from_group() {
        let groups = make_group(2, &[("hasinfinitemana", true)]);
        let flags = flags_for_group(&groups, 2);
        assert!(has_player_flag(flags, PLAYER_FLAG_HAS_INFINITE_MANA));
    }

    #[test]
    fn ignore_spell_check_flag_resolves_from_group() {
        let groups = make_group(6, &[("ignorespellcheck", true)]);
        let flags = flags_for_group(&groups, 6);
        assert!(has_player_flag(flags, PLAYER_FLAG_IGNORE_SPELL_CHECK));
    }

    #[test]
    fn cannot_use_spells_flag_resolves_from_group() {
        let groups = make_group(4, &[("cannotusespells", true)]);
        let flags = flags_for_group(&groups, 4);
        assert!(has_player_flag(flags, PLAYER_FLAG_CANNOT_USE_SPELLS));
    }

    #[test]
    fn not_gain_in_fight_flag_resolves_from_group() {
        let groups = make_group(4, &[("notgaininfight", true)]);
        let flags = flags_for_group(&groups, 4);
        assert!(has_player_flag(flags, PLAYER_FLAG_NOT_GAIN_IN_FIGHT));
    }

    #[test]
    fn isalwayspremium_flag_resolves_from_group() {
        let groups = make_group(2, &[("isalwayspremium", true)]);
        let flags = flags_for_group(&groups, 2);
        assert!(has_player_flag(flags, PLAYER_FLAG_IS_ALWAYS_PREMIUM));
    }

    #[test]
    fn can_edit_houses_flag_resolves_from_group() {
        let groups = make_group(6, &[("canedithouses", true)]);
        let flags = flags_for_group(&groups, 6);
        assert!(has_player_flag(flags, PLAYER_FLAG_CAN_EDIT_HOUSES));
    }

    #[test]
    fn keep_inventory_flag_resolves_from_group() {
        let groups = make_group(2, &[("keepinventory", true)]);
        let flags = flags_for_group(&groups, 2);
        assert!(has_player_flag(flags, PLAYER_FLAG_KEEP_INVENTORY));
    }

    #[test]
    fn can_push_all_creatures_flag_resolves_from_group() {
        let groups = make_group(2, &[("canpushallcreatures", true)]);
        let flags = flags_for_group(&groups, 2);
        assert!(has_player_flag(flags, PLAYER_FLAG_CAN_PUSH_ALL_CREATURES));
    }
}
