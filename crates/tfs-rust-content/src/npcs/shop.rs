//! Optional NPC shop-window definition.
//!
//! Imported 772 dialogue trading stays dialogue-action based. The shop window
//! is the later-codec trade list (`0x7A`).
//!
//! Domain: TFS `NpcType` shop modules / `luascript.cpp` shop open/list APIs.

use std::collections::HashMap;

/// One sellable/buyable shop line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcShopItem {
    /// Server item id of the product. For a packed offer this is the content.
    pub item_id: u16,
    /// Subtype / fluid type when relevant; `0` = default.
    pub subtype: u8,
    /// Buy price from the player (NPC sells to player); `0` = not sold.
    pub buy_price: u32,
    /// Sell price to the NPC (player sells); `0` = not bought.
    pub sell_price: u32,
    /// Display name override; empty uses the item database name.
    pub name: String,
    /// When set, each purchased unit is one of these containers filled with `item_id`.
    pub container_id: Option<u16>,
}

/// Player-facing shop replies. `%N` name, `%C` count, `%T` total gold, `%I` item name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcShopMessages {
    pub no_shop: String,
    pub need_money: String,
    pub need_space: String,
    pub bought: String,
    pub sold: String,
    pub need_item: String,
}

impl Default for NpcShopMessages {
    fn default() -> Self {
        Self {
            no_shop: "Sorry, I do not sell anything.".into(),
            need_money: "You do not have enough money.".into(),
            need_space: "You do not have enough capacity.".into(),
            bought: "Bought %C %I for %T gold.".into(),
            sold: "Sold %C %I for %T gold.".into(),
            need_item: "You do not have that item.".into(),
        }
    }
}

impl NpcShopMessages {
    pub fn format(&self, template: &str, name: &str, count: u32, total: u64, item: &str) -> String {
        template
            .replace("%N", name)
            .replace("%C", &count.to_string())
            .replace("%T", &total.to_string())
            .replace("%I", item)
    }
}

/// Validated shop catalog attached to an [`super::NpcDefinition`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcShopDefinition {
    pub items: Vec<NpcShopItem>,
    pub messages: NpcShopMessages,
    /// Container wrapped around a purchase when the client asks for backpacks.
    /// `0` uses the default shopping bag.
    pub bag_item_id: u16,
    /// Gold charged per bag when the bag is not itself a catalog line.
    pub bag_price: u32,
    /// Optional script parameters (TFS `npc:getParameter` style).
    pub parameters: HashMap<String, String>,
}

impl Default for NpcShopDefinition {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            messages: NpcShopMessages::default(),
            bag_item_id: 0,
            bag_price: 0,
            parameters: HashMap::new(),
        }
    }
}

/// Named ship destination for `{ listDestinations = true }` and travel-by-name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcDestination {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub price: u32,
    pub premium: bool,
    pub level: u32,
}
