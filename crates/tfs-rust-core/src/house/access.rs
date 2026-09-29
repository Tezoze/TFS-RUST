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

/// `CheckAccessRight` (`houses.cc:99`).
///
/// `1` allow, `-1` deny, `0` the line does not apply. `guild` is `(rank, guild name)`.
pub fn check_access_right(rule: &str, player_name: &str, guild: Option<(&str, &str)>) -> i8 {
    let (negate, rule) = match rule.strip_prefix('!') {
        Some(rest) => (true, rest),
        None => (false, rule),
    };
    if rule.is_empty() {
        return 0;
    }
    let matched = if rule.contains('@') {
        let Some((rank, guild_name)) = guild.filter(|(_, name)| !name.is_empty()) else {
            return 0;
        };
        let help = if rank.is_empty() {
            format!("@{guild_name}")
        } else {
            format!("{rank}@{guild_name}")
        };
        match_house_string(rule, &help)
    } else {
        match_house_string(rule, player_name)
    };
    if !matched {
        0
    } else if negate {
        -1
    } else {
        1
    }
}

/// `MatchString` (`strings.cc:409`) — case-insensitive, `*` any run, `?` one character.
fn match_house_string(pattern: &str, text: &str) -> bool {
    fn rec(pattern: &[u8], text: &[u8]) -> bool {
        match (pattern.first().copied(), text.first().copied()) {
            (None, None) => true,
            (Some(b'*'), _) => {
                rec(&pattern[1..], text) || (!text.is_empty() && rec(pattern, &text[1..]))
            }
            (Some(b'?'), Some(_)) => rec(&pattern[1..], &text[1..]),
            (Some(p), Some(t)) if p.to_ascii_lowercase() == t.to_ascii_lowercase() => {
                rec(&pattern[1..], &text[1..])
            }
            _ => false,
        }
    }
    rec(pattern.as_bytes(), text.as_bytes())
}

/// `MayOpenDoor` line scan (`houses.cc:586`). First deny wins, else the first allow.
pub fn door_text_allows(text: &str, player_name: &str, guild: Option<(&str, &str)>) -> bool {
    if text.is_empty() {
        return false;
    }
    for line in text.split('\n') {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let rule = truncate_guest_line(line);
        match check_access_right(rule, player_name, guild) {
            -1 => return false,
            1 => return true,
            _ => {}
        }
    }
    false
}

fn truncate_guest_line(line: &str) -> &str {
    const MAX_HOUSE_GUEST_NAME: usize = 100;
    if line.len() >= MAX_HOUSE_GUEST_NAME {
        let mut end = MAX_HOUSE_GUEST_NAME - 1;
        while !line.is_char_boundary(end) {
            end -= 1;
        }
        &line[..end]
    } else {
        line
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
        assert!(!can_edit_access_list(
            AccessHouseLevel::SubOwner,
            SUBOWNER_LIST
        ));
        assert!(!can_edit_access_list(AccessHouseLevel::Guest, GUEST_LIST));
    }

    #[test]
    fn door_text_matches_name_and_deny() {
        assert!(door_text_allows("alice\n", "Alice", None));
        assert!(!door_text_allows("!alice\nalice\n", "Alice", None));
        assert!(door_text_allows("# note\n!bob\n*a*\n", "Alice", None));
        assert!(!door_text_allows("", "Alice", None));
    }
}
