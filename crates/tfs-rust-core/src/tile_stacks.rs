//! Lazy down / top / creature vectors on a tile.
//!
//! Layout only — stack order, ground/top/down semantics, flags, and zone stay on
//! [`crate::tile::TileBody`]. Ground-only tiles keep `stacks = None`.
//!
//! Pack surface: TFS split `downItems` / `topItems` / `creatures` (`tile.h`).
//! Corpus: TVP `StaticTile` lazy `unique_ptr` vectors (`tile.h` ~364–424).

use crate::ids::{CreatureId, ItemId};

/// Heap payload allocated on first non-ground insert.
///
/// `down_items` is newest-first (`Tile::addThing` inserts at begin). Creature
/// remove is order-preserving (`CutObject` splice — not `swap_remove`).
#[derive(Debug, Clone, Default)]
pub struct TileStacks {
    pub down_items: Vec<ItemId>,
    pub top_items: Vec<ItemId>,
    pub creatures: Vec<CreatureId>,
}

impl TileStacks {
    pub fn is_empty(&self) -> bool {
        self.down_items.is_empty() && self.top_items.is_empty() && self.creatures.is_empty()
    }

    /// `None` when every stack is empty — ground-only tiles stay unboxed.
    pub fn boxed_if_nonempty(
        down_items: Vec<ItemId>,
        top_items: Vec<ItemId>,
        creatures: Vec<CreatureId>,
    ) -> Option<Box<Self>> {
        if down_items.is_empty() && top_items.is_empty() && creatures.is_empty() {
            None
        } else {
            Some(Box::new(Self {
                down_items,
                top_items,
                creatures,
            }))
        }
    }
}
