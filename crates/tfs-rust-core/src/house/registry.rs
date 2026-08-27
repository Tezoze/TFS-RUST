//! House metadata registry (XML + map geometry).
//! Pack surface: TFS `House` fields (`house.h`); XML via `Houses::loadHousesXML`.

use tfs_rust_common::Position;

use crate::ids::ItemId;

/// Runtime house record — XML metadata + map-scanned geometry + DB owner/rent/bid.
#[derive(Debug, Clone)]
pub struct House {
    pub id: u32,
    pub name: String,
    pub rent: u32,
    pub town_id: u32,
    pub size: u32,
    pub entry_pos: Position,
    /// Unix `paid` column (`houses.paid`).
    pub paid_until: u32,
    pub warnings: u32,
    pub tiles: Vec<Position>,
    /// `(door_id, item)` registered from `ATTR_HOUSEDOORID`.
    pub doors: Vec<(u8, ItemId)>,
    pub beds: Vec<ItemId>,
    pub bid: u32,
    pub bid_end: u32,
    pub last_bid: u32,
    pub highest_bidder: u32,
}

impl House {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: String::new(),
            rent: 0,
            town_id: 0,
            size: 0,
            entry_pos: Position::default(),
            paid_until: 0,
            warnings: 0,
            tiles: Vec::new(),
            doors: Vec::new(),
            beds: Vec::new(),
            bid: 0,
            bid_end: 0,
            last_bid: 0,
            highest_bidder: 0,
        }
    }

    pub fn clear_bid(&mut self) {
        self.bid = 0;
        self.bid_end = 0;
        self.last_bid = 0;
        self.highest_bidder = 0;
    }

    pub fn door_id_at(
        &self,
        pos: Position,
        find_item: impl Fn(ItemId) -> Option<Position>,
    ) -> Option<u8> {
        for &(door_id, item_id) in &self.doors {
            if find_item(item_id) == Some(pos) {
                return Some(door_id);
            }
        }
        None
    }
}

/// `houseRentPeriod` config — TFS key name; implemented cycle is monthly corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseRentPeriod {
    Never,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl HouseRentPeriod {
    pub fn from_config(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "daily" => Self::Daily,
            "weekly" => Self::Weekly,
            "yearly" => Self::Yearly,
            "never" => Self::Never,
            _ => Self::Monthly,
        }
    }
}
