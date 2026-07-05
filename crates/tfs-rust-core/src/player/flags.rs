//! `PlayerFlag` bits from `groups.xml` — `src/const.h`, `src/groups.cpp`.
// C++ reference: `PlayerFlags` enum, `Group::parseFlag`.

use tfs_rust_content::groups::GroupDatabase;

/// C++ `PlayerFlag_CannotPickupItem` — `src/const.h`
pub const PLAYER_FLAG_CANNOT_PICKUP_ITEM: u64 = 1 << 14;
/// C++ `PlayerFlag_HasInfiniteCapacity` — `src/const.h`
pub const PLAYER_FLAG_HAS_INFINITE_CAPACITY: u64 = 1 << 20;
/// C++ `PlayerFlag_IgnoredByMonsters` — `src/const.h`
pub const PLAYER_FLAG_IGNORED_BY_MONSTERS: u64 = 1 << 8;

/// Map `groups.xml` `<flag name="..." value="1"/>` keys to `PlayerFlags` bits (subset used by inventory).
fn flag_name_to_bit(name: &str) -> Option<u64> {
    match name.to_ascii_lowercase().as_str() {
        "cannotpickupitem" => Some(PLAYER_FLAG_CANNOT_PICKUP_ITEM),
        "hasinfinitecapacity" => Some(PLAYER_FLAG_HAS_INFINITE_CAPACITY),
        "ignoredbymonsters" => Some(PLAYER_FLAG_IGNORED_BY_MONSTERS),
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
