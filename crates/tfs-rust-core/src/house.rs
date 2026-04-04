//! Houses: access lists, beds, persistence hooks.
// C++ reference: `house.cpp` `House`.

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct HouseAccess {
    pub owner_guid: Option<u32>,
    pub subowners: HashSet<u32>,
    pub guests: HashSet<u32>,
}

#[derive(Debug, Default)]
pub struct HouseManager {
    pub houses: HashMap<u32, HouseAccess>,
}

impl HouseManager {
    pub fn set_owner(&mut self, house_id: u32, guid: u32) {
        self.houses.entry(house_id).or_default().owner_guid = Some(guid);
    }

    pub fn kick_player(&mut self, _house_id: u32, _player_guid: u32) {
        // Phase 5: teleport / close door.
    }
}
