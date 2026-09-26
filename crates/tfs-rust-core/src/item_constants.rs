//! Depot, mail, and document type ids. Catalog keys are the client ids.
//! C++ reference: `const.h` `item_t`. Document is this catalog's 2834, not parchment 2819.

/// Map depot locker — `ITEM_LOCKER1`.
pub const ITEM_LOCKER1: u16 = 3497;
/// Depot chest item inside universal depot container — `ITEM_DEPOT`.
pub const ITEM_DEPOT: u16 = 3502;
/// Player inbox — `ITEM_INBOX`. Not a row in this catalog.
pub const ITEM_INBOX: u16 = 14404;
/// Unstamped parcel — TFS `ITEM_PARCEL` (`const.h`); 772 `PARCEL_NEW`.
pub const ITEM_PARCEL: u16 = 3503;
/// Stamped parcel — TFS `ITEM_PARCEL_STAMPED` (`const.h`); 772 `PARCEL_STAMPED`.
pub const ITEM_PARCEL_STAMPED: u16 = 3504;
/// Unstamped letter — TFS `ITEM_LETTER` (`const.h`); 772 `LETTER_NEW`.
pub const ITEM_LETTER: u16 = 3505;
/// Stamped letter — `ITEM_LETTER_STAMPED` (`const.h`); 772 `LETTER_STAMPED`.
pub const ITEM_LETTER_STAMPED: u16 = 3506;
/// Parcel label — TFS `ITEM_LABEL` (`const.h`); 772 `PARCEL_LABEL`.
pub const ITEM_LABEL: u16 = 3507;
/// House transfer document. Catalog id 2834 (`const.h` lists parchment 2819).
pub const ITEM_DOCUMENT_RO: u16 = 2834;
/// Market slot inside virtual depot locker — `ITEM_MARKET`. Not a row in this catalog.
pub const ITEM_MARKET: u16 = 14405;
