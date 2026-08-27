//! Houses: access lists, beds, rent, auctions, persistence, edit windows.
//!
//! Domain: TFS `house.cpp` `House` / `Door` / `AccessList`.
//! Outcomes: 772 `houses.cc` rent/eviction/auction settlement; TFS pack lists/XML/Lua.

mod access;
mod auction;
mod depot_cash;
mod look;
mod ownership;
mod persist;
mod registry;
mod rent;
mod serialize;
mod tick;
mod window;

pub use access::{
    AccessHouseLevel, AccessList, GUEST_LIST, HouseAccess, SUBOWNER_LIST, can_edit_access_list,
};
pub use auction::{AuctionOutcome, auction_paid_until, decide_auction};
pub use registry::{House, HouseRentPeriod};
pub use rent::{HOUSE_GRACE_SECS, HOUSE_MONTH_SECS, RentAction, decide_rent};
pub use serialize::{LoadedHouseItem, decode_tile_store_blob, encode_house_tile_store};
pub use window::{HouseEditSession, house_window_door_id_ok, house_window_id_ok};

use std::collections::{HashMap, HashSet};

use tfs_rust_common::Position;
use tfs_rust_content::houses_xml::HouseXmlEntry;

use crate::ids::ItemId;

/// Runtime house registry + access lists (`Houses` / `House` in TFS).
#[derive(Debug, Default)]
pub struct HouseManager {
    pub houses: HashMap<u32, HouseAccess>,
    pub records: HashMap<u32, House>,
    /// Per-door access lists keyed by `(house_id, door_id)` — C++ `Door::accessList`.
    pub door_lists: HashMap<(u32, u8), AccessList>,
    /// TFS `Game::bedSleepersMap` — sleeper GUID → occupied bed item (`game.cpp`).
    pub bed_sleepers: HashMap<u32, ItemId>,
    /// `Player::setEditHouse` session keyed by player GUID.
    pub edit_sessions: HashMap<u32, HouseEditSession>,
    /// Name → GUID cache for access-list parse (boot + logins).
    pub name_to_guid: HashMap<String, u32>,
    /// Offline eviction: item ids waiting to be written into `player_depotitems`.
    pub pending_depot_dumps: HashMap<u32, Vec<ItemId>>,
    pub pending_depot_town: HashMap<u32, u32>,
    pub last_process_unix: i64,
}

impl HouseManager {
    /// Ensure empty house records exist (map OTBM house tiles / houses.xml seed).
    pub fn ensure_houses(&mut self, ids: impl IntoIterator<Item = u32>) {
        for id in ids {
            self.houses.entry(id).or_default();
            self.records.entry(id).or_insert_with(|| House::new(id));
        }
    }

    pub fn apply_xml_entries(&mut self, entries: &[HouseXmlEntry]) {
        for e in entries {
            self.houses.entry(e.id).or_default();
            let rec = self.records.entry(e.id).or_insert_with(|| House::new(e.id));
            rec.name = e.name.clone();
            rec.rent = e.rent;
            rec.town_id = e.town_id;
            rec.size = e.size;
            rec.entry_pos = e.entry;
        }
    }

    pub fn attach_tile(&mut self, house_id: u32, pos: Position) {
        self.houses.entry(house_id).or_default();
        let rec = self
            .records
            .entry(house_id)
            .or_insert_with(|| House::new(house_id));
        if !rec.tiles.contains(&pos) {
            rec.tiles.push(pos);
        }
    }

    pub fn attach_door(&mut self, house_id: u32, door_id: u8, item_id: ItemId) {
        let rec = self
            .records
            .entry(house_id)
            .or_insert_with(|| House::new(house_id));
        if !rec
            .doors
            .iter()
            .any(|(d, i)| *d == door_id && *i == item_id)
        {
            rec.doors.push((door_id, item_id));
        }
    }

    pub fn attach_bed(&mut self, house_id: u32, item_id: ItemId) {
        let rec = self
            .records
            .entry(house_id)
            .or_insert_with(|| House::new(house_id));
        if !rec.beds.contains(&item_id) {
            rec.beds.push(item_id);
        }
    }

    pub fn set_owner(&mut self, house_id: u32, guid: u32) {
        let access = self.houses.entry(house_id).or_default();
        access.owner_guid = if guid == 0 { None } else { Some(guid) };
        access.owner_name.clear();
        self.records
            .entry(house_id)
            .or_insert_with(|| House::new(house_id));
    }

    /// TFS `Map::getHouseByPlayerId` — first house owned by `guid`, if any.
    pub fn house_id_for_owner(&self, guid: u32) -> Option<u32> {
        self.houses
            .iter()
            .find_map(|(id, access)| (access.owner_guid == Some(guid)).then_some(*id))
    }

    /// TFS `House::ownerName` after `IOLoginData::getNameByGuid`.
    pub fn set_owner_name(&mut self, house_id: u32, name: String) {
        if let Some(access) = self.houses.get_mut(&house_id) {
            access.owner_name = name;
        }
    }

    /// Login / takeover: fill `ownerName` for every house this GUID owns.
    pub fn set_owner_name_for_guid(&mut self, guid: u32, name: &str) {
        for access in self.houses.values_mut() {
            if access.owner_guid == Some(guid) {
                access.owner_name = name.to_string();
            }
        }
    }

    pub fn apply_owner_names(&mut self, guid_to_name: &HashMap<u32, String>) {
        for access in self.houses.values_mut() {
            if let Some(g) = access.owner_guid
                && let Some(n) = guid_to_name.get(&g)
            {
                access.owner_name = n.clone();
            }
        }
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

    pub fn can_edit_list(
        &self,
        house_id: u32,
        list_id: u32,
        player_guid: u32,
        can_edit_houses: bool,
    ) -> bool {
        can_edit_access_list(
            self.access_level(house_id, player_guid, can_edit_houses),
            list_id,
        )
    }

    pub fn get_access_list_text(&self, house_id: u32, list_id: u32) -> Option<String> {
        match list_id {
            GUEST_LIST => self.houses.get(&house_id).map(|a| a.guest_list_raw.clone()),
            SUBOWNER_LIST => self
                .houses
                .get(&house_id)
                .map(|a| a.subowner_list_raw.clone()),
            door => {
                let door_id = u8::try_from(door).ok()?;
                self.door_lists
                    .get(&(house_id, door_id))
                    .map(|l| l.raw.clone())
                    .or(Some(String::new()))
            }
        }
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
                access.guest_list_raw = parsed.raw.clone();
                access.guests = if parsed.allow_everyone {
                    HashSet::new()
                } else {
                    parsed.player_guids
                };
            }
            SUBOWNER_LIST => {
                let access = self.houses.entry(house_id).or_default();
                access.subowner_list_raw = parsed.raw.clone();
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

    pub fn clear_lists(&mut self, house_id: u32) {
        if let Some(access) = self.houses.get_mut(&house_id) {
            access.guests.clear();
            access.subowners.clear();
            access.guests_allow_everyone = false;
            access.guest_list_raw.clear();
            access.subowner_list_raw.clear();
        }
        self.door_lists.retain(|(hid, _), _| *hid != house_id);
    }

    pub fn list_ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self.records.keys().copied().collect();
        ids.sort_unstable();
        ids
    }

    /// Stub kept for call-site compatibility; teleport lives on [`GameWorld::house_kick_player`].
    pub fn kick_player(&mut self, _house_id: u32, _player_guid: u32) {}
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
    fn house_id_for_owner_finds_owned_house() {
        let mut h = HouseManager::default();
        h.set_owner(7, 42);
        h.set_owner(8, 99);
        assert_eq!(h.house_id_for_owner(42), Some(7));
        assert_eq!(h.house_id_for_owner(99), Some(8));
        assert_eq!(h.house_id_for_owner(1), None);
        h.set_owner(7, 0);
        assert_eq!(h.house_id_for_owner(42), None);
    }
}
