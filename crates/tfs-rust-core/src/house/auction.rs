//! Auction settlement — 772 `FinishAuctions` (`houses.cc` ~923) fed by MyAAC bid columns.
//! The server never starts auctions or accepts in-game bids.

use super::rent::HOUSE_MONTH_SECS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuctionOutcome {
    Skip,
    /// Winner pays `rent + bid` from depot; owner assigned, `paid_until = now + 30d`.
    Award { cost: u64 },
    /// Winner cannot pay; house stays free and bid columns clear.
    InsufficientFunds,
}

/// `FinishAuctions` payment check: `DepotMoney < (House->Rent + Bid)`.
pub fn decide_auction(
    now: u32,
    owner: u32,
    bid_end: u32,
    highest_bidder: u32,
    bid: u32,
    rent: u32,
    depot_cash: u64,
) -> AuctionOutcome {
    if owner != 0 || highest_bidder == 0 || bid_end == 0 || bid_end >= now {
        return AuctionOutcome::Skip;
    }
    let cost = u64::from(rent).saturating_add(u64::from(bid));
    if depot_cash < cost {
        AuctionOutcome::InsufficientFunds
    } else {
        AuctionOutcome::Award { cost }
    }
}

/// Paid-until after a successful auction award (`houses.cc` `now + 30d`).
pub fn auction_paid_until(now: u32) -> u32 {
    now.saturating_add(HOUSE_MONTH_SECS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_owned_or_not_due() {
        assert_eq!(decide_auction(100, 1, 50, 9, 10, 5, 1000), AuctionOutcome::Skip);
        assert_eq!(decide_auction(100, 0, 150, 9, 10, 5, 1000), AuctionOutcome::Skip);
        assert_eq!(decide_auction(100, 0, 50, 0, 10, 5, 1000), AuctionOutcome::Skip);
    }

    #[test]
    fn awards_when_depot_covers_rent_plus_bid() {
        assert_eq!(
            decide_auction(100, 0, 50, 9, 20, 10, 30),
            AuctionOutcome::Award { cost: 30 }
        );
        assert_eq!(auction_paid_until(100), 100 + HOUSE_MONTH_SECS);
    }

    #[test]
    fn insufficient_funds_leaves_house_free() {
        assert_eq!(
            decide_auction(100, 0, 50, 9, 20, 10, 29),
            AuctionOutcome::InsufficientFunds
        );
    }
}
