//! Server item type ids shared by depot/inbox runtime.
// C++ reference: `src/const.h`

/// Map depot locker — `ITEM_LOCKER1`.
pub const ITEM_LOCKER1: u16 = 2589;
/// Depot chest item inside universal depot container — `ITEM_DEPOT`.
pub const ITEM_DEPOT: u16 = 2594;
/// Player inbox — `ITEM_INBOX`.
pub const ITEM_INBOX: u16 = 14404;
/// Unstamped parcel — TFS `ITEM_PARCEL` (`const.h`); 772 `PARCEL_NEW`.
pub const ITEM_PARCEL: u16 = 2595;
/// Stamped parcel — TFS `ITEM_PARCEL_STAMPED` (`const.h`); 772 `PARCEL_STAMPED`.
pub const ITEM_PARCEL_STAMPED: u16 = 2596;
/// Unstamped letter — TFS `ITEM_LETTER` (`const.h`); 772 `LETTER_NEW`.
pub const ITEM_LETTER: u16 = 2597;
/// Stamped letter — `ITEM_LETTER_STAMPED` (`const.h`); 772 `LETTER_STAMPED`.
pub const ITEM_LETTER_STAMPED: u16 = 2598;
/// Parcel label — TFS `ITEM_LABEL` (`const.h`); 772 `PARCEL_LABEL`.
pub const ITEM_LABEL: u16 = 2599;
/// Read-only document — TFS `ITEM_DOCUMENT_RO` (`const.h`); house transfer item.
pub const ITEM_DOCUMENT_RO: u16 = 1968;
/// Market slot inside virtual depot locker — `ITEM_MARKET`.
pub const ITEM_MARKET: u16 = 14405;
