//! `PlayerFlag` bits from `groups.xml` — `src/const.h`, `src/groups.cpp`.
// C++ reference: `PlayerFlags` enum, `Group::parseFlag`.

use tfs_rust_content::groups::GroupDatabase;

/// C++ `PlayerFlag_CannotUseCombat` — `src/const.h`. 772 equivalent: `NO_ATTACK` right
/// (`enums.hh:524`, `crcombat.cc:391,589`). Blocks all attack actions.
pub const PLAYER_FLAG_CANNOT_USE_COMBAT: u64 = 1 << 0;
/// C++ `PlayerFlag_CannotBeAttacked` — `src/const.h`. 772 equivalent: `INVULNERABLE` right
/// (`enums.hh:517`, `crmain.cc:536-538`). Zeroes incoming damage to the target.
pub const PLAYER_FLAG_CANNOT_BE_ATTACKED: u64 = 1 << 3;
/// C++ `PlayerFlag_CannotPickupItem` — `src/const.h`
pub const PLAYER_FLAG_CANNOT_PICKUP_ITEM: u64 = 1 << 14;
/// C++ `PlayerFlag_HasInfiniteCapacity` — `src/const.h`
pub const PLAYER_FLAG_HAS_INFINITE_CAPACITY: u64 = 1 << 20;
/// C++ `PlayerFlag_HasInfiniteMana` — `src/const.h`
pub const PLAYER_FLAG_HAS_INFINITE_MANA: u64 = 1 << 10;
/// C++ `PlayerFlag_IgnoredByMonsters` — `src/const.h`
pub const PLAYER_FLAG_IGNORED_BY_MONSTERS: u64 = 1 << 8;
/// C++ `PlayerFlag_CanBroadcast` — `src/const.h`
pub const PLAYER_FLAG_CAN_BROADCAST: u64 = 1 << 16;
/// C++ `PlayerFlag_CanTalkRedPrivate` — `src/const.h`
pub const PLAYER_FLAG_CAN_TALK_RED_PRIVATE: u64 = 1 << 22;
/// C++ `PlayerFlag_CannotBeMuted` — `src/const.h`
pub const PLAYER_FLAG_CANNOT_BE_MUTED: u64 = 1 << 36;
/// C++ `PlayerFlag_SetMaxSpeed` — `src/const.h`. When set, `Player::updateBaseSpeed`
/// caps the character's base speed at `PLAYER_MAX_SPEED` (1500).
pub const PLAYER_FLAG_SET_MAX_SPEED: u64 = 1 << 29;
/// C++ `PlayerFlag_IsAlwaysPremium` — `src/const.h` bit 35. Group flag
/// `isalwayspremium` → treat as premium regardless of `accounts.premium_ends_at`.
pub const PLAYER_FLAG_IS_ALWAYS_PREMIUM: u64 = 1 << 35;

/// Map `groups.xml` / `groups.lua` flag keys to `PlayerFlags` bits (subset used by core).
fn flag_name_to_bit(name: &str) -> Option<u64> {
    match name.to_ascii_lowercase().as_str() {
        "cannotusecombat" => Some(PLAYER_FLAG_CANNOT_USE_COMBAT),
        "cannotbeattacked" => Some(PLAYER_FLAG_CANNOT_BE_ATTACKED),
        "cannotpickupitem" => Some(PLAYER_FLAG_CANNOT_PICKUP_ITEM),
        "hasinfinitecapacity" => Some(PLAYER_FLAG_HAS_INFINITE_CAPACITY),
        "hasinfinitemana" => Some(PLAYER_FLAG_HAS_INFINITE_MANA),
        "ignoredbymonsters" => Some(PLAYER_FLAG_IGNORED_BY_MONSTERS),
        "canbroadcast" => Some(PLAYER_FLAG_CAN_BROADCAST),
        "cantalkredprivate" => Some(PLAYER_FLAG_CAN_TALK_RED_PRIVATE),
        "cannotbemuted" => Some(PLAYER_FLAG_CANNOT_BE_MUTED),
        "setmaxspeed" => Some(PLAYER_FLAG_SET_MAX_SPEED),
        "isalwayspremium" => Some(PLAYER_FLAG_IS_ALWAYS_PREMIUM),
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
        if enabled {
            if let Some(bit) = flag_name_to_bit(name) {
                bits |= bit;
            }
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
    fn isalwayspremium_flag_resolves_from_group() {
        let groups = make_group(2, &[("isalwayspremium", true)]);
        let flags = flags_for_group(&groups, 2);
        assert!(has_player_flag(flags, PLAYER_FLAG_IS_ALWAYS_PREMIUM));
    }
}
