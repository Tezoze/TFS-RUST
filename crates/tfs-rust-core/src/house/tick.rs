//! Daily house processing — 772 `ProcessHouses` order: auctions then rent.
//! C++ reference: `houses.cc` `ProcessHouses` (~1943), `FinishAuctions`, `CollectRent`.

use crate::game_world::GameWorld;

use super::HOUSE_GRACE_SECS;
use super::auction::{AuctionOutcome, auction_paid_until, decide_auction};
use super::depot_cash::{container_tree_delete_money, player_town_depot_money};
use super::registry::HouseRentPeriod;
use super::rent::{RentAction, decide_rent};

impl GameWorld {
    /// Settle due MyAAC bids then collect rent for online owners.
    /// Offline depot cash is handled by the async boot/save path.
    pub fn process_houses_online(
        &mut self,
        now_unix: u32,
        period: HouseRentPeriod,
        grace_secs: u32,
    ) {
        if period == HouseRentPeriod::Never {
            return;
        }
        self.settle_auctions_online(now_unix);
        self.collect_rent_online(now_unix, grace_secs);
        self.houses.last_process_unix = i64::from(now_unix);
    }

    fn settle_auctions_online(&mut self, now: u32) {
        let ids: Vec<u32> = self.houses.list_ids();
        for id in ids {
            let (owner, bid_end, bidder, bid, rent, town_id, name) = {
                let rec = match self.houses.records.get(&id) {
                    Some(r) => r,
                    None => continue,
                };
                let owner = self
                    .houses
                    .houses
                    .get(&id)
                    .and_then(|a| a.owner_guid)
                    .unwrap_or(0);
                (
                    owner,
                    rec.bid_end,
                    rec.highest_bidder,
                    rec.bid,
                    rec.rent,
                    rec.town_id,
                    rec.name.clone(),
                )
            };
            let Some(&cid) = self.player_by_guid.get(&bidder) else {
                continue;
            };
            let cash = player_town_depot_money(self, cid, town_id);
            match decide_auction(now, owner, bid_end, bidder, bid, rent, cash) {
                AuctionOutcome::Skip => {}
                AuctionOutcome::Award { cost } => {
                    if let Some(chest) = self.player_get_depot_chest(cid, town_id, true)
                        && container_tree_delete_money(self, chest, cost)
                    {
                        self.house_set_owner(id, bidder, now);
                        if let Some(rec) = self.houses.records.get_mut(&id) {
                            rec.paid_until = auction_paid_until(now);
                            rec.clear_bid();
                        }
                        self.house_deliver_letter(
                            bidder,
                            town_id,
                            format!("You won the auction for {name}."),
                        );
                    }
                }
                AuctionOutcome::InsufficientFunds => {
                    if let Some(rec) = self.houses.records.get_mut(&id) {
                        rec.clear_bid();
                    }
                }
            }
        }
    }

    fn collect_rent_online(&mut self, now: u32, grace_secs: u32) {
        let ids: Vec<u32> = self.houses.list_ids();
        for id in ids {
            let (owner, paid, warnings, rent, town_id, name) = {
                let rec = match self.houses.records.get(&id) {
                    Some(r) => r,
                    None => continue,
                };
                let owner = self
                    .houses
                    .houses
                    .get(&id)
                    .and_then(|a| a.owner_guid)
                    .unwrap_or(0);
                if owner == 0 {
                    continue;
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
            let Some(&cid) = self.player_by_guid.get(&owner) else {
                continue;
            };
            let cash = player_town_depot_money(self, cid, town_id);
            match decide_rent(now, paid, warnings, rent, cash, grace_secs) {
                RentAction::Skip => {}
                RentAction::Paid { new_paid_until } => {
                    if let Some(chest) = self.player_get_depot_chest(cid, town_id, true) {
                        let _ = container_tree_delete_money(self, chest, u64::from(rent));
                    }
                    if let Some(rec) = self.houses.records.get_mut(&id) {
                        rec.paid_until = new_paid_until;
                        rec.warnings = 0;
                    }
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
                }
                RentAction::Evict => {
                    self.house_set_owner(id, 0, now);
                }
            }
        }
    }

    pub fn house_rent_period_from_config(&self) -> HouseRentPeriod {
        let raw = crate::config::get_string_or(&self.config, "houseRentPeriod", "monthly")
            .unwrap_or_else(|_| "monthly".into());
        super::registry::HouseRentPeriod::from_config(&raw)
    }

    pub fn house_grace_secs_from_config(&self) -> u32 {
        let days = crate::config::get_i64_or(&self.config, "houseRentGraceDays", 7).unwrap_or(7);
        if days <= 0 {
            HOUSE_GRACE_SECS
        } else {
            (days as u32).saturating_mul(86_400)
        }
    }
}
