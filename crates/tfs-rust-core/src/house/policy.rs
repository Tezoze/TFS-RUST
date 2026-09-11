//! House policy evictions — 772 `ProcessRent` Evict* (`houses.cc:1139-1236`).
//!
//! Pack schema is TFS `houses` / `players` / `accounts` / `guilds` (no `HouseTransfers`).
//! `TransferHouses` stays AAC/`!sellhouse` trade. `StartAuctions` stays MyAAC (lesson 802).
//! Cadence is boot + save/reboot fire (`InitHouses` / `ProcessHouses`), not the minute arm.

use tfs_rust_db::house::HouseStore;

use crate::game_world::GameWorld;

impl GameWorld {
    /// Apply eviction list — skip if owner already changed.
    pub(crate) fn apply_house_policy_scan(&mut self, evict: Vec<(u32, u32)>) {
        let now = super::persist::unix_now();
        for (house_id, owner_guid) in evict {
            let current = self
                .houses
                .houses
                .get(&house_id)
                .and_then(|a| a.owner_guid)
                .unwrap_or(0);
            if current != owner_guid || owner_guid == 0 {
                continue;
            }
            self.house_set_owner(house_id, 0, now);
        }
    }

    /// `EvictFreeAccounts` / `EvictDeletedCharacters` / `EvictExGuildLeaders`.
    /// Called from `process_and_persist_houses_inner` (boot + save), not every minute.
    pub(crate) async fn run_house_policy_scan(&mut self) {
        let guild_halls: Vec<(u32, u32)> = self
            .houses
            .records
            .iter()
            .filter(|(_, rec)| rec.is_guild_hall)
            .filter_map(|(&id, _)| {
                let owner = self
                    .houses
                    .houses
                    .get(&id)
                    .and_then(|a| a.owner_guid)
                    .unwrap_or(0);
                (owner != 0).then_some((id, owner))
            })
            .collect();
        let store = HouseStore::new(&self.db);
        let mut evict = store.policy_eviction_rows().await.unwrap_or_default();
        if !guild_halls.is_empty() {
            let leaders = store.guild_leader_guids().await.unwrap_or_default();
            for (house_id, owner) in guild_halls {
                if !leaders.contains(&owner) {
                    evict.push((house_id, owner));
                }
            }
        }
        if !evict.is_empty() {
            self.apply_house_policy_scan(evict);
        }
    }
}
