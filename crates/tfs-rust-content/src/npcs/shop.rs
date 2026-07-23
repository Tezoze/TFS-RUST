//! Optional NPC shop-window definition (TFS flexibility; unused until NPC-8).
//!
//! Imported dialogue trading stays dialogue-action based. Shop windows are opt-in
//! for new/TFS content only.
//!
//! Domain: TFS `NpcType` shop modules / `luascript.cpp` shop open/list APIs.
//! 772: no client shop window — dialogue `create`/`delete`/`createmoney` actions.

use std::collections::HashMap;

/// One sellable/buyable shop line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcShopItem {
    /// Server item id.
    pub item_id: u16,
    /// Subtype / fluid type when relevant; `0` = default.
    pub subtype: u8,
    /// Buy price from player (NPC sells to player); `0` = not sold.
    pub buy_price: u32,
    /// Sell price to NPC (player sells); `0` = not bought.
    pub sell_price: u32,
    /// Display name override; empty uses item database name.
    pub name: String,
}

/// Validated shop catalog attached to an [`super::NpcDefinition`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NpcShopDefinition {
    pub items: Vec<NpcShopItem>,
    /// Optional script parameters (TFS `npc:getParameter` style).
    pub parameters: HashMap<String, String>,
}
