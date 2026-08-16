//! Houses: access lists, beds, persistence hooks.
//!
//! Domain: TFS `house.cpp` `House` / `Door` / `AccessList`.
//! Outcomes: door use gate matches TFS/TVP `Door::canUse` (owner/subowner or per-door list).

use std::collections::{HashMap, HashSet};

use crate::ids::ItemId;

/// C++ `GUEST_LIST` — `house.h`.
pub const GUEST_LIST: u32 = 0x100;
/// C++ `SUBOWNER_LIST` — `house.h`.
pub const SUBOWNER_LIST: u32 = 0x101;

/// C++ `AccessHouseLevel_t` — `house.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccessHouseLevel {
    NotInvited = 0,
    Guest = 1,
    SubOwner = 2,
    Owner = 3,
}

/// C++ `AccessList` — player GUID set + `*` allow-everyone (`house.cpp` `parseList` / `isInList`).
/// Guild ranks are ignored until guild house lists are wired (lines with `@` are skipped).
#[derive(Debug, Clone, Default)]
pub struct AccessList {
    pub player_guids: HashSet<u32>,
    pub allow_everyone: bool,
    /// Original text for `getAccessList` / save round-trip.
    pub raw: String,
}

impl AccessList {
    /// Collect lowercase player name candidates from a list blob (no DB).
    pub fn candidate_names(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        for (i, line) in text.lines().enumerate() {
            if i >= 100 {
                break;
            }
            let line = line.trim().trim_matches('\t');
            if line.is_empty() || line.starts_with('#') || line.len() > 100 {
                continue;
            }
            let lower = line.to_ascii_lowercase();
            if lower.contains('@') || lower.contains('!') || lower.contains('?') {
                continue;
            }
            if lower == "*" || lower.contains('*') {
                continue;
            }
            out.push(lower);
        }
        out
    }

    /// C++ `AccessList::parseList` — resolve player names via `resolve` (name → guid).
    pub fn parse_list(text: &str, mut resolve: impl FnMut(&str) -> Option<u32>) -> Self {
        let mut list = Self {
            raw: text.to_string(),
            ..Self::default()
        };
        if text.is_empty() {
            return list;
        }
        for (i, line) in text.lines().enumerate() {
            if i >= 100 {
                break;
            }
            let line = line.trim().trim_matches('\t');
            if line.is_empty() || line.starts_with('#') || line.len() > 100 {
                continue;
            }
            let lower = line.to_ascii_lowercase();
            if lower.contains('@') {
                // Guild / guild-rank — deferred (no-op until guild house lists).
                continue;
            }
            if lower == "*" {
                list.allow_everyone = true;
                continue;
            }
            if lower.contains('!') || lower.contains('*') || lower.contains('?') {
                continue;
            }
            if let Some(guid) = resolve(&lower) {
                list.player_guids.insert(guid);
            }
        }
        list
    }

    /// C++ `AccessList::isInList` (player GUID path; guild ranks omitted).
    pub fn is_in_list(&self, player_guid: u32) -> bool {
        self.allow_everyone || self.player_guids.contains(&player_guid)
    }
}

#[derive(Debug, Clone, Default)]
pub struct HouseAccess {
    pub owner_guid: Option<u32>,
    pub subowners: HashSet<u32>,
    pub guests: HashSet<u32>,
    /// Guest-list `*` — everyone is invited (`AccessList::allowEveryone` on guest list).
    pub guests_allow_everyone: bool,
}

#[derive(Debug, Default)]
pub struct HouseManager {
    pub houses: HashMap<u32, HouseAccess>,
    /// Per-door access lists keyed by `(house_id, door_id)` — C++ `Door::accessList`.
    pub door_lists: HashMap<(u32, u8), AccessList>,
    /// TFS `Game::bedSleepersMap` — sleeper GUID → occupied bed item (`game.cpp`).
    pub bed_sleepers: HashMap<u32, ItemId>,
}

impl HouseManager {
    /// Ensure empty house records exist (map OTBM house tiles / houses.xml seed).
    pub fn ensure_houses(&mut self, ids: impl IntoIterator<Item = u32>) {
        for id in ids {
            self.houses.entry(id).or_default();
        }
    }

    pub fn set_owner(&mut self, house_id: u32, guid: u32) {
        let access = self.houses.entry(house_id).or_default();
        access.owner_guid = if guid == 0 { None } else { Some(guid) };
    }

    /// C++ `House::getHouseAccessLevel` — `house.cpp` (~112).
    pub fn access_level(
        &self,
        house_id: u32,
        player_guid: u32,
        can_edit_houses: bool,
    ) -> AccessHouseLevel {
        if can_edit_houses {
            return AccessHouseLevel::Owner;
        }
        let Some(access) = self.houses.get(&house_id) else {
            return AccessHouseLevel::NotInvited;
        };
        if access.owner_guid == Some(player_guid) {
            return AccessHouseLevel::Owner;
        }
        if access.subowners.contains(&player_guid) {
            return AccessHouseLevel::SubOwner;
        }
        if access.guests_allow_everyone || access.guests.contains(&player_guid) {
            return AccessHouseLevel::Guest;
        }
        AccessHouseLevel::NotInvited
    }

    /// TFS `House::isInvited` — `house.cpp` (owner, subowner, guest list).
    /// Unknown house id stays unrestricted for tile/item moves (existing contract).
    pub fn is_invited(&self, house_id: u32, player_guid: u32) -> bool {
        let Some(access) = self.houses.get(&house_id) else {
            return true;
        };
        if access.owner_guid == Some(player_guid) {
            return true;
        }
        if access.subowners.contains(&player_guid) {
            return true;
        }
        access.guests_allow_everyone || access.guests.contains(&player_guid)
    }

    /// C++ `Door::canUse` — `house.cpp` (~535) / TVP `house.cpp` (~600).
    /// Owner / subowner / `CanEditHouses`, else per-door access list.
    pub fn door_can_use(
        &self,
        house_id: u32,
        door_id: u8,
        player_guid: u32,
        can_edit_houses: bool,
    ) -> bool {
        if self.access_level(house_id, player_guid, can_edit_houses) >= AccessHouseLevel::SubOwner {
            return true;
        }
        self.door_lists
            .get(&(house_id, door_id))
            .is_some_and(|list| list.is_in_list(player_guid))
    }

    /// Apply one `house_lists` row (`IOMapSerialize::loadHouseInfo` / `House::setAccessList`).
    pub fn apply_list_row(
        &mut self,
        house_id: u32,
        list_id: u32,
        text: &str,
        mut resolve: impl FnMut(&str) -> Option<u32>,
    ) {
        let parsed = AccessList::parse_list(text, &mut resolve);
        match list_id {
            GUEST_LIST => {
                let access = self.houses.entry(house_id).or_default();
                access.guests_allow_everyone = parsed.allow_everyone;
                access.guests = if parsed.allow_everyone {
                    HashSet::new()
                } else {
                    parsed.player_guids
                };
            }
            SUBOWNER_LIST => {
                let access = self.houses.entry(house_id).or_default();
                access.subowners = parsed.player_guids;
            }
            door => {
                let door_id = u8::try_from(door).unwrap_or(0);
                if parsed.raw.is_empty() && !parsed.allow_everyone && parsed.player_guids.is_empty()
                {
                    self.door_lists.remove(&(house_id, door_id));
                } else {
                    self.door_lists.insert((house_id, door_id), parsed);
                }
            }
        }
    }

    pub fn kick_player(&mut self, _house_id: u32, _player_guid: u32) {
        // Optional follow-up: teleport / close door.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn door_can_use_owner_and_subowner() {
        let mut h = HouseManager::default();
        h.set_owner(1, 10);
        h.houses.entry(1).or_default().subowners.insert(20);
        h.houses.entry(1).or_default().guests.insert(30);
        assert!(h.door_can_use(1, 1, 10, false));
        assert!(h.door_can_use(1, 1, 20, false));
        // Guest alone cannot open house door (needs door list).
        assert!(!h.door_can_use(1, 1, 30, false));
        assert!(!h.door_can_use(1, 1, 99, false));
    }

    #[test]
    fn door_can_use_per_door_list() {
        let mut h = HouseManager::default();
        h.set_owner(1, 10);
        h.apply_list_row(1, 3, "alice\nbob", |name| match name {
            "alice" => Some(100),
            "bob" => Some(200),
            _ => None,
        });
        assert!(h.door_can_use(1, 3, 100, false));
        assert!(h.door_can_use(1, 3, 200, false));
        assert!(!h.door_can_use(1, 3, 300, false));
        assert!(!h.door_can_use(1, 4, 100, false));
    }

    #[test]
    fn door_can_use_star_allows_everyone() {
        let mut h = HouseManager::default();
        h.set_owner(1, 10);
        h.apply_list_row(1, 2, "*", |_| None);
        assert!(h.door_can_use(1, 2, 999, false));
    }

    #[test]
    fn door_can_use_can_edit_houses() {
        let mut h = HouseManager::default();
        h.set_owner(1, 10);
        assert!(h.door_can_use(1, 1, 999, true));
    }

    #[test]
    fn guest_and_subowner_lists_from_db_ids() {
        let mut h = HouseManager::default();
        h.set_owner(1, 10);
        h.apply_list_row(1, GUEST_LIST, "guest", |n| {
            if n == "guest" { Some(30) } else { None }
        });
        h.apply_list_row(1, SUBOWNER_LIST, "sub", |n| {
            if n == "sub" { Some(20) } else { None }
        });
        assert!(h.is_invited(1, 30));
        assert!(h.door_can_use(1, 1, 20, false));
        assert!(!h.door_can_use(1, 1, 30, false));
    }

    #[test]
    fn guest_star_invites_everyone() {
        let mut h = HouseManager::default();
        h.set_owner(1, 10);
        h.apply_list_row(1, GUEST_LIST, "*", |_| None);
        assert!(h.is_invited(1, 999));
        assert!(!h.door_can_use(1, 1, 999, false));
    }

    #[test]
    fn access_list_skips_comments_and_guild() {
        let list = AccessList::parse_list("# comment\n@guild\nalice", |n| {
            if n == "alice" { Some(1) } else { None }
        });
        assert!(list.is_in_list(1));
        assert_eq!(list.player_guids.len(), 1);
    }
}
