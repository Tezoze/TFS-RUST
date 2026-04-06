//! Map tile stacks (ground, items, creatures) and flags.
// C++ reference: `Tile` (`tile.h`), `Tile::queryAdd`, `queryRemove`, `addThing`, `removeThing`.

use crate::ids::{CreatureId, ItemId};
use tfs_rust_common::{enums::ZoneType, Position};

/// OTBM / TFS tile bitfield (subset; extend as engine grows).
pub mod flags {
    pub const NONE: u32 = 0;
    /// Blocks line of sight / projectiles when combined with item blocking (simplified).
    pub const BLOCK_SOLID: u32 = 1 << 0;
    pub const BLOCK_PROJECTILE: u32 = 1 << 1;
}

#[derive(Debug, Clone)]
pub struct TileBody {
    pub position: Position,
    pub ground: Option<u16>,
    /// Non-ground items below creatures on the wire (`Tile::getBeginDownItem`, `src/tile.cpp`).
    pub down_items: Vec<ItemId>,
    /// Always-on-top items, sent before creatures (`getBeginTopItem` … `getEndTopItem`).
    pub top_items: Vec<ItemId>,
    pub creatures: Vec<CreatureId>,
    pub flags: u32,
    pub zone: ZoneType,
}

#[derive(Debug, Clone)]
pub struct HouseTile {
    pub inner: TileBody,
    /// House identifier from OTBM / `houses` table.
    pub house_id: u32,
}

#[derive(Debug, Clone)]
pub enum Tile {
    Normal(TileBody),
    House(HouseTile),
}

impl Tile {
    pub fn position(&self) -> Position {
        match self {
            Tile::Normal(t) => t.position,
            Tile::House(h) => h.inner.position,
        }
    }

    pub fn body_mut(&mut self) -> &mut TileBody {
        match self {
            Tile::Normal(t) => t,
            Tile::House(h) => &mut h.inner,
        }
    }

    pub fn body(&self) -> &TileBody {
        match self {
            Tile::Normal(t) => t,
            Tile::House(h) => &h.inner,
        }
    }

    /// TFS `Tile::queryAdd` — minimal placeholder until item database + walkability rules land.
    pub fn query_add(&self, _thing_size: u8) -> bool {
        // C++ reference: `tile.cpp` Tile::queryAdd — checks blocking, height, etc.
        true
    }

    pub fn query_remove(&self, _thing_size: u8) -> bool {
        true
    }

    pub fn add_creature(&mut self, id: CreatureId) {
        self.body_mut().creatures.push(id);
    }

    pub fn remove_creature(&mut self, id: CreatureId) -> bool {
        let body = self.body_mut();
        if let Some(i) = body.creatures.iter().position(|&c| c == id) {
            body.creatures.swap_remove(i);
            return true;
        }
        false
    }

    /// Add an item to this tile (adds to front of down_items, matching C++ `Tile::addThing`).
    // C++ ref: src/tile.cpp Tile::addThing — non-always-on-top items insert at begin of downItems.
    pub fn add_item(&mut self, item_id: ItemId) {
        self.body_mut().down_items.insert(0, item_id);
    }

    /// Add an always-on-top item to this tile.
    pub fn add_top_item(&mut self, item_id: ItemId) {
        self.body_mut().top_items.push(item_id);
    }

    /// Remove an item from this tile by its ItemId. Returns the index it was removed from, or None.
    // C++ ref: src/tile.cpp Tile::removeThing — preserves ordering.
    pub fn remove_item_by_id(&mut self, item_id: ItemId) -> Option<usize> {
        let body = self.body_mut();
        // Try down_items first
        if let Some(i) = body.down_items.iter().position(|&id| id == item_id) {
            body.down_items.remove(i);
            return Some(i);
        }
        // Try top_items
        if let Some(i) = body.top_items.iter().position(|&id| id == item_id) {
            body.top_items.remove(i);
            return Some(i);
        }
        None
    }

    /// Check if this tile has a specific item
    pub fn has_item(&self, item_id: ItemId) -> bool {
        let body = self.body();
        body.down_items.contains(&item_id) || body.top_items.contains(&item_id)
    }

    /// Total number of items on this tile (top + down, excluding ground).
    pub fn total_item_count(&self) -> usize {
        let body = self.body();
        body.top_items.len() + body.down_items.len()
    }

    /// Get the first down item (top of down stack, i.e. index 0).
    // C++ ref: src/tile.cpp Tile::getTopDownItem
    pub fn get_top_down_item(&self) -> Option<ItemId> {
        self.body().down_items.first().copied()
    }

    /// Compute the client stack position for an item on this tile.
    // C++ ref: src/tile.cpp Tile::getStackposOfItem — ground(0) + top_items + creatures + down_items.
    pub fn get_item_stack_pos(&self, item_id: ItemId) -> Option<u8> {
        let body = self.body();
        let mut n: u8 = if body.ground.is_some() { 1 } else { 0 };
        for &tid in &body.top_items {
            if tid == item_id {
                return Some(n);
            }
            n = n.saturating_add(1);
        }
        n = n.saturating_add(body.creatures.len() as u8);
        for &did in &body.down_items {
            if did == item_id {
                return Some(n);
            }
            n = n.saturating_add(1);
        }
        None
    }

    /// Count of things before down_items start (ground + top_items + creatures).
    // Used to compute stack_pos for a newly-added down item at index 0.
    pub fn down_item_start_stack_pos(&self) -> u8 {
        let body = self.body();
        let mut n: u8 = if body.ground.is_some() { 1 } else { 0 };
        n = n.saturating_add(body.top_items.len() as u8);
        n = n.saturating_add(body.creatures.len() as u8);
        n
    }
}

/// TFS `Tile::getClientIndexOfCreature` (simplified: all creatures visible).
// C++ reference: `src/tile.cpp` `Tile::getClientIndexOfCreature`.
pub fn client_creature_stack_pos(body: &TileBody, creature: CreatureId) -> i32 {
    // C++ `Tile::getClientIndexOfCreature` — only ground + top items count before creatures (`src/tile.cpp`).
    let mut n: i32 = if body.ground.is_some() { 1 } else { 0 };
    n += body.top_items.len() as i32;
    for &c in body.creatures.iter().rev() {
        if c == creature {
            return n;
        }
        n += 1;
    }
    -1
}
