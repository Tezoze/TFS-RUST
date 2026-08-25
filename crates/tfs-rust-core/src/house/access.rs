//! House access lists and invite/door gates.
//!
//! Domain: TFS `house.cpp` `House` / `Door` / `AccessList`.
//! Outcomes: door use gate matches TFS/TVP `Door::canUse` (owner/subowner or per-door list).

use std::collections::HashSet;

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
    /// Cached `House::ownerName` / corpus `THouse::OwnerName` (look text, boot `getNameByGuid`).
    pub owner_name: String,
    pub subowners: HashSet<u32>,
    pub guests: HashSet<u32>,
    /// Guest-list `*` — everyone is invited (`AccessList::allowEveryone` on guest list).
    pub guests_allow_everyone: bool,
    /// Guest list raw text (`House::getAccessList` / `house_lists` round-trip).
    pub guest_list_raw: String,
    /// Subowner list raw text.
    pub subowner_list_raw: String,
}

/// C++ `House::canEditAccessList` — `house.cpp` (~312).
pub fn can_edit_access_list(level: AccessHouseLevel, list_id: u32) -> bool {
    match level {
        AccessHouseLevel::Owner => true,
        AccessHouseLevel::SubOwner => list_id == GUEST_LIST,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_list_skips_comments_and_guild() {
        let list = AccessList::parse_list("# comment\n@guild\nalice", |n| {
            if n == "alice" { Some(1) } else { None }
        });
        assert!(list.is_in_list(1));
        assert_eq!(list.player_guids.len(), 1);
    }

    #[test]
    fn subowner_edits_guests_only() {
        assert!(can_edit_access_list(AccessHouseLevel::Owner, GUEST_LIST));
        assert!(can_edit_access_list(AccessHouseLevel::Owner, SUBOWNER_LIST));
        assert!(can_edit_access_list(AccessHouseLevel::Owner, 3));
        assert!(can_edit_access_list(AccessHouseLevel::SubOwner, GUEST_LIST));
        assert!(!can_edit_access_list(AccessHouseLevel::SubOwner, SUBOWNER_LIST));
        assert!(!can_edit_access_list(AccessHouseLevel::Guest, GUEST_LIST));
    }
}
