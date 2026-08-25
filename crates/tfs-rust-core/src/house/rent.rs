//! Rent decision — 772 `CollectRent` (`houses.cc` ~1239).
//!
//! Monthly cycle (`PaidUntil += 30d`), 7-day grace with one warning letter,
//! payment from depot cash. TFS `Houses::payHouses` uses bank + 7 warnings;
//! corpus wins.

/// Seconds in the 772 house month (`houses.cc` `30 * 24 * 60 * 60`).
pub const HOUSE_MONTH_SECS: u32 = 30 * 24 * 60 * 60;
/// Default grace after `paid_until` (`houses.cc` `7 * 24 * 60 * 60`).
pub const HOUSE_GRACE_SECS: u32 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RentAction {
    Skip,
    Paid { new_paid_until: u32 },
    Warn { days_left: u32 },
    Evict,
}

/// Decide the rent action for one owned house.
///
/// `warnings == 0` means no warning letter has been sent this grace window.
/// Corpus sends a single letter while `now < Deadline`.
pub fn decide_rent(
    now: u32,
    paid_until: u32,
    warnings: u32,
    rent: u32,
    depot_cash: u64,
    grace_secs: u32,
) -> RentAction {
    if rent == 0 || paid_until > now {
        return RentAction::Skip;
    }
    if depot_cash >= u64::from(rent) {
        return RentAction::Paid {
            new_paid_until: paid_until.saturating_add(HOUSE_MONTH_SECS),
        };
    }
    let deadline = paid_until.saturating_add(grace_secs);
    if now < deadline {
        if warnings == 0 {
            let days_left = 1 + deadline.saturating_sub(now).saturating_sub(3600) / 86_400;
            RentAction::Warn { days_left }
        } else {
            RentAction::Skip
        }
    } else {
        RentAction::Evict
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_when_not_due() {
        assert_eq!(decide_rent(100, 200, 0, 50, 0, HOUSE_GRACE_SECS), RentAction::Skip);
    }

    #[test]
    fn skip_unowned_rent_zero() {
        assert_eq!(decide_rent(100, 0, 0, 0, 0, HOUSE_GRACE_SECS), RentAction::Skip);
    }

    #[test]
    fn pays_when_due_and_funded() {
        assert_eq!(
            decide_rent(100, 50, 0, 10, 10, HOUSE_GRACE_SECS),
            RentAction::Paid {
                new_paid_until: 50 + HOUSE_MONTH_SECS
            }
        );
    }

    #[test]
    fn warns_inside_grace_once() {
        let now = 100;
        let paid = 90;
        match decide_rent(now, paid, 0, 50, 0, HOUSE_GRACE_SECS) {
            RentAction::Warn { days_left } => assert!(days_left >= 1),
            other => panic!("expected warn, got {other:?}"),
        }
        assert_eq!(
            decide_rent(now, paid, 1, 50, 0, HOUSE_GRACE_SECS),
            RentAction::Skip
        );
    }

    #[test]
    fn evicts_after_grace() {
        let paid = 10;
        let now = paid + HOUSE_GRACE_SECS;
        assert_eq!(decide_rent(now, paid, 1, 50, 0, HOUSE_GRACE_SECS), RentAction::Evict);
    }
}
