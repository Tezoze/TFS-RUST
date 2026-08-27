//! House DB persist + offline rent/auction (boot and daily save).
//! C++ reference: `iomapserialize.cpp` `saveHouseInfo` / `saveHouseItems`;
//! corpus `ProcessHouses` (`houses.cc` ~1943).

use std::time::{SystemTime, UNIX_EPOCH};

use tfs_rust_common::error::Result;
use tfs_rust_db::{
    HouseInfoUpsert, HouseListRow, HouseOwnerRow, HouseStore, ItemRecord, ItemStore, ItemTable,
};

use crate::game_world::GameWorld;
use crate::game_world_save::append_save_item_tree;
use crate::ids::ItemId;

use super::HouseManager;
use super::access::{GUEST_LIST, SUBOWNER_LIST};
use super::auction::{AuctionOutcome, decide_auction};
use super::depot_cash::{deduct_depot_records, depot_records_money};
use super::rent::{RentAction, decide_rent};

pub(crate) fn unix_now() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().min(u64::from(u32::MAX)) as u32)
        .unwrap_or(0)
}

impl HouseManager {
    pub fn apply_owner_row(&mut self, row: &HouseOwnerRow) {
        let id = row.id_u32();
        self.set_owner(id, row.owner_u32());
        let rec = self
            .records
            .entry(id)
            .or_insert_with(|| super::House::new(id));
        rec.paid_until = row.paid;
        rec.warnings = row.warnings.max(0) as u32;
        rec.bid = row.bid.max(0) as u32;
        rec.bid_end = row.bid_end.max(0) as u32;
        rec.last_bid = row.last_bid.max(0) as u32;
        rec.highest_bidder = row.highest_bidder.max(0) as u32;
        if rec.name.is_empty() && !row.name.is_empty() {
            rec.name = row.name.clone();
        }
        if rec.rent == 0 && row.rent > 0 {
            rec.rent = row.rent as u32;
        }
        if rec.town_id == 0 && row.town_id > 0 {
            rec.town_id = row.town_id as u32;
        }
        if rec.size == 0 && row.size > 0 {
            rec.size = row.size as u32;
        }
    }

    pub fn house_list_rows(&self) -> Vec<HouseListRow> {
        let mut rows = Vec::new();
        for (&id, access) in &self.houses {
            if !access.guest_list_raw.is_empty() {
                rows.push(HouseListRow::new(
                    id,
                    GUEST_LIST,
                    access.guest_list_raw.clone(),
                ));
            }
            if !access.subowner_list_raw.is_empty() {
                rows.push(HouseListRow::new(
                    id,
                    SUBOWNER_LIST,
                    access.subowner_list_raw.clone(),
                ));
            }
        }
        for (&(house_id, door_id), list) in &self.door_lists {
            if list.raw.is_empty() {
                continue;
            }
            rows.push(HouseListRow::new(
                house_id,
                u32::from(door_id),
                list.raw.clone(),
            ));
        }
        rows
    }

    pub fn house_info_upserts(&self) -> Vec<HouseInfoUpsert> {
        let mut rows = Vec::new();
        for (&id, rec) in &self.records {
            let owner = self.houses.get(&id).and_then(|a| a.owner_guid).unwrap_or(0);
            rows.push(HouseInfoUpsert {
                id,
                owner,
                paid: rec.paid_until,
                warnings: rec.warnings as i32,
                name: rec.name.clone(),
                rent: rec.rent as i32,
                town_id: rec.town_id as i32,
                bid: rec.bid as i32,
                bid_end: rec.bid_end as i32,
                last_bid: rec.last_bid as i32,
                highest_bidder: rec.highest_bidder as i32,
                size: rec.size as i32,
                beds: rec.beds.len() as i32,
            });
        }
        rows
    }
}

impl GameWorld {
    /// `IOMapSerialize::saveHouseItems` + `saveHouseInfo`.
    pub async fn save_houses(&self) -> Result<()> {
        let store = HouseStore::new(&self.db);
        let tiles = self.encode_house_tile_store();
        store.replace_all_tile_store(&tiles).await?;
        store
            .upsert_house_info(&self.houses.house_info_upserts())
            .await?;
        store
            .replace_all_house_lists(&self.houses.house_list_rows())
            .await?;
        Ok(())
    }

    /// Corpus `ProcessHouses`: online first, then offline depot rows, then persist.
    pub async fn process_and_persist_houses(&mut self) -> Result<()> {
        let now = unix_now();
        let period = self.house_rent_period_from_config();
        let grace = self.house_grace_secs_from_config();
        self.process_houses_online(now, period, grace);
        self.process_houses_offline(now, grace).await?;
        self.flush_pending_depot_dumps().await?;
        self.save_houses().await
    }

    async fn process_houses_offline(&mut self, now: u32, grace_secs: u32) -> Result<()> {
        let ids: Vec<u32> = self.houses.list_ids();
        for id in ids {
            self.settle_one_offline_auction(id, now).await?;
            self.collect_one_offline_rent(id, now, grace_secs).await?;
        }
        Ok(())
    }

    async fn settle_one_offline_auction(&mut self, id: u32, now: u32) -> Result<()> {
        let (owner, bid_end, bidder, bid, rent) = {
            let rec = match self.houses.records.get(&id) {
                Some(r) => r,
                None => return Ok(()),
            };
            let owner = self
                .houses
                .houses
                .get(&id)
                .and_then(|a| a.owner_guid)
                .unwrap_or(0);
            (owner, rec.bid_end, rec.highest_bidder, rec.bid, rec.rent)
        };
        if self.player_by_guid.contains_key(&bidder) {
            return Ok(());
        }
        match decide_auction(now, owner, bid_end, bidder, bid, rent, 0) {
            AuctionOutcome::Skip => return Ok(()),
            AuctionOutcome::InsufficientFunds | AuctionOutcome::Award { .. } => {}
        }
        let db = self.db.clone();
        let store = ItemStore::new(&db);
        let mut rows = store
            .load_items(bidder as i32, ItemTable::Depot)
            .await
            .unwrap_or_default();
        let cash = depot_records_money(&rows);
        match decide_auction(now, owner, bid_end, bidder, bid, rent, cash) {
            AuctionOutcome::Skip => Ok(()),
            AuctionOutcome::Award { cost } => {
                if deduct_depot_records(&mut rows, cost) {
                    store
                        .save_items(bidder as i32, ItemTable::Depot, &rows)
                        .await?;
                    self.house_set_owner(id, bidder, now);
                } else if let Some(rec) = self.houses.records.get_mut(&id) {
                    rec.clear_bid();
                }
                Ok(())
            }
            AuctionOutcome::InsufficientFunds => {
                if let Some(rec) = self.houses.records.get_mut(&id) {
                    rec.clear_bid();
                }
                Ok(())
            }
        }
    }

    async fn collect_one_offline_rent(&mut self, id: u32, now: u32, grace_secs: u32) -> Result<()> {
        let (owner, paid, warnings, rent, town_id, name) = {
            let rec = match self.houses.records.get(&id) {
                Some(r) => r,
                None => return Ok(()),
            };
            let owner = self
                .houses
                .houses
                .get(&id)
                .and_then(|a| a.owner_guid)
                .unwrap_or(0);
            if owner == 0 {
                return Ok(());
            }
            (
                owner,
                rec.paid_until,
                rec.warnings,
                rec.rent,
                rec.town_id,
                rec.name.clone(),
            )
        };
        if self.player_by_guid.contains_key(&owner) {
            return Ok(());
        }
        match decide_rent(now, paid, warnings, rent, 0, grace_secs) {
            RentAction::Skip => return Ok(()),
            RentAction::Paid { .. } | RentAction::Warn { .. } | RentAction::Evict => {}
        }
        let db = self.db.clone();
        let store = ItemStore::new(&db);
        let mut rows = store
            .load_items(owner as i32, ItemTable::Depot)
            .await
            .unwrap_or_default();
        let cash = depot_records_money(&rows);
        match decide_rent(now, paid, warnings, rent, cash, grace_secs) {
            RentAction::Skip => Ok(()),
            RentAction::Paid { new_paid_until } => {
                if deduct_depot_records(&mut rows, u64::from(rent)) {
                    store
                        .save_items(owner as i32, ItemTable::Depot, &rows)
                        .await?;
                    if let Some(rec) = self.houses.records.get_mut(&id) {
                        rec.paid_until = new_paid_until;
                        rec.warnings = 0;
                    }
                }
                Ok(())
            }
            RentAction::Warn { days_left } => {
                self.house_deliver_letter(
                    owner,
                    town_id,
                    format!(
                        "Warning: rent for {name} is overdue. You have {days_left} day(s) left."
                    ),
                );
                if let Some(rec) = self.houses.records.get_mut(&id) {
                    rec.warnings = 1;
                }
                Ok(())
            }
            RentAction::Evict => {
                self.house_set_owner(id, 0, now);
                Ok(())
            }
        }
    }

    async fn flush_pending_depot_dumps(&mut self) -> Result<()> {
        let dumps = std::mem::take(&mut self.houses.pending_depot_dumps);
        let towns = std::mem::take(&mut self.houses.pending_depot_town);
        let db = self.db.clone();
        let store = ItemStore::new(&db);
        for (guid, items) in dumps {
            if items.is_empty() {
                continue;
            }
            let town_id = towns.get(&guid).copied().unwrap_or(1);
            let mut rows = store
                .load_items(guid as i32, ItemTable::Depot)
                .await
                .unwrap_or_default();
            let max_sid = rows.iter().map(|r| r.sid).max().unwrap_or(100);
            let roots: Vec<(i32, ItemId)> = items
                .iter()
                .copied()
                .map(|id| (town_id as i32, id))
                .collect();
            let mut extra: Vec<ItemRecord> = Vec::new();
            if let Err(e) = append_save_item_tree(self, &roots, &mut extra) {
                tracing::warn!(guid, error = %e, "house depot dump serialize failed");
                continue;
            }
            let offset = max_sid.saturating_sub(100);
            for rec in &mut extra {
                if rec.pid > 99 {
                    rec.pid += offset;
                }
                rec.sid += offset;
            }
            rows.extend(extra);
            if let Err(e) = store.save_items(guid as i32, ItemTable::Depot, &rows).await {
                tracing::warn!(guid, error = %e, "house depot dump save failed");
            }
            self.items_pending_release.extend(items);
        }
        Ok(())
    }
}
