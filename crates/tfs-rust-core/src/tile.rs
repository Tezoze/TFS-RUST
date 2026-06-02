//! Map tile stacks (ground, items, creatures) and flags.
// C++ reference: `Tile` (`tile.h`), `Tile::queryAdd`, `queryRemove`, `addThing`, `removeThing`.

use crate::ids::{CreatureId, ItemId};
use crate::thing::LookTarget;
use tfs_rust_common::enums::ZoneType;

/// TFS `tileflags_t` (`src/tile.h`) — runtime tile state bitfield.
/// C++ ref: src/tile.h:23-52
pub mod flags {
    pub const NONE: u32 = 0;

    // ── Floor-change bits (0–6) ──
    pub const FLOORCHANGE_DOWN: u32 = 1 << 0;
    pub const FLOORCHANGE_NORTH: u32 = 1 << 1;
    pub const FLOORCHANGE_SOUTH: u32 = 1 << 2;
    pub const FLOORCHANGE_EAST: u32 = 1 << 3;
    pub const FLOORCHANGE_WEST: u32 = 1 << 4;
    pub const FLOORCHANGE_SOUTH_ALT: u32 = 1 << 5;
    pub const FLOORCHANGE_EAST_ALT: u32 = 1 << 6;

    // ── Zone / special bits (7–16) ──
    pub const PROTECTIONZONE: u32 = 1 << 7;
    pub const NOPVPZONE: u32 = 1 << 8;
    pub const NOLOGOUT: u32 = 1 << 9;
    pub const PVPZONE: u32 = 1 << 10;
    pub const TELEPORT: u32 = 1 << 11;
    pub const MAGICFIELD: u32 = 1 << 12;
    pub const MAILBOX: u32 = 1 << 13;
    pub const TRASHHOLDER: u32 = 1 << 14;
    pub const BED: u32 = 1 << 15;
    pub const DEPOT: u32 = 1 << 16;

    // ── Blocking bits (17–23) ──
    pub const BLOCKSOLID: u32 = 1 << 17;
    pub const BLOCKPATH: u32 = 1 << 18;
    pub const IMMOVABLEBLOCKSOLID: u32 = 1 << 19;
    pub const IMMOVABLEBLOCKPATH: u32 = 1 << 20;
    pub const IMMOVABLENOFIELDBLOCKPATH: u32 = 1 << 21;
    pub const NOFIELDBLOCKPATH: u32 = 1 << 22;
    pub const SUPPORTS_HANGABLE: u32 = 1 << 23;

    // ── Composite masks ──
    pub const FLOORCHANGE: u32 = FLOORCHANGE_DOWN | FLOORCHANGE_NORTH | FLOORCHANGE_SOUTH
        | FLOORCHANGE_EAST | FLOORCHANGE_WEST | FLOORCHANGE_SOUTH_ALT | FLOORCHANGE_EAST_ALT;

    // Legacy aliases used by map/LOS code.
    pub const BLOCK_SOLID: u32 = BLOCKSOLID;
    pub const BLOCK_PROJECTILE: u32 = BLOCKPATH;
}

#[derive(Debug, Clone)]
pub struct TileBody {
    pub ground: Option<u16>,
    /// Non-ground items below creatures on the wire (`Tile::getBeginDownItem`, `src/tile.cpp`).
    pub down_items: Vec<ItemId>,
    /// Always-on-top items, sent before creatures (`getBeginTopItem` … `getEndTopItem`).
    pub top_items: Vec<ItemId>,
    pub creatures: Vec<CreatureId>,
    pub flags: u32,
    pub zone: ZoneType,
}

impl TileBody {
    pub fn new() -> Self {
        Self {
            ground: None,
            down_items: Vec::new(),
            top_items: Vec::new(),
            creatures: Vec::new(),
            flags: 0,
            zone: ZoneType::Normal,
        }
    }
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
    pub fn empty_normal() -> Self {
        Tile::Normal(TileBody::new())
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

    /// C++ `Tile::getUseItem` — `tile.cpp` ~1603 (container priority + `getThing` stack walk).
    pub fn item_id_for_use<F>(&self, stack_pos: u8, is_container: F) -> Option<ItemId>
    where
        F: Fn(ItemId) -> bool,
    {
        let body = self.body();
        if body.down_items.is_empty() && body.top_items.is_empty() {
            // C++ returns `ground` when the item list is empty; ground has no `ItemId` in Rust.
            return None;
        }

        let container_item = body
            .down_items
            .iter()
            .chain(body.top_items.iter())
            .copied()
            .find(|&id| is_container(id));

        let thing_at = self.item_id_at_stack_pos(stack_pos);

        if let Some(container_id) = container_item {
            return match thing_at {
                Some(item_id) => Some(item_id),
                None => Some(container_id),
            };
        }

        thing_at
    }

    /// Inverse of [`Tile::get_item_stack_pos`] — resolve client `stack_pos` to an item on this tile.
    // C++ ref: `Tile::getThing` / `Game::playerUseItem` stack walk (`tile.cpp`, `game.cpp`).
    pub fn item_id_at_stack_pos(&self, stack_pos: u8) -> Option<ItemId> {
        let body = self.body();
        let mut n: u8 = if body.ground.is_some() { 1 } else { 0 };
        for &tid in &body.top_items {
            if n == stack_pos {
                return Some(tid);
            }
            n = n.saturating_add(1);
        }
        let after_top = n;
        let creature_end = after_top.saturating_add(body.creatures.len() as u8);
        if stack_pos >= after_top && stack_pos < creature_end {
            return None;
        }
        n = creature_end;
        for &did in &body.down_items {
            if n == stack_pos {
                return Some(did);
            }
            n = n.saturating_add(1);
        }
        None
    }

    /// C++ `Tile::getTopVisibleThing` — `tile.cpp` ~322–347.
    pub fn top_visible_look_target<F, G>(&self, can_see_creature: F, item_is_opaque: G) -> Option<LookTarget>
    where
        F: Fn(CreatureId) -> bool,
        G: Fn(ItemId) -> bool,
    {
        top_visible_look_target_from_body(self.body(), can_see_creature, item_is_opaque)
    }
}

/// Shared look stack walk for [`Tile::top_visible_look_target`] and tests.
pub fn top_visible_look_target_from_body<F, G>(
    body: &TileBody,
    can_see_creature: F,
    item_is_opaque: G,
) -> Option<LookTarget>
where
    F: Fn(CreatureId) -> bool,
    G: Fn(ItemId) -> bool,
{
    for &creature_id in &body.creatures {
        if can_see_creature(creature_id) {
            return Some(LookTarget::Creature(creature_id));
        }
    }
    for &item_id in &body.down_items {
        if item_is_opaque(item_id) {
            return Some(LookTarget::Item(item_id));
        }
    }
    for &item_id in body.top_items.iter().rev() {
        if item_is_opaque(item_id) {
            return Some(LookTarget::Item(item_id));
        }
    }
    body.ground.map(LookTarget::Ground)
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

#[cfg(test)]
mod look_tests {
    use super::*;
    use slotmap::SlotMap;

    fn tile_body(ground: Option<u16>, down: Vec<ItemId>, top: Vec<ItemId>, creatures: Vec<CreatureId>) -> TileBody {
        TileBody {
            ground,
            down_items: down,
            top_items: top,
            creatures,
            flags: 0,
            zone: ZoneType::Normal,
        }
    }

    #[test]
    fn get_top_visible_ground_only() {
        let body = tile_body(Some(106), vec![], vec![], vec![]);
        let got = top_visible_look_target_from_body(&body, |_| true, |_| true);
        assert_eq!(got, Some(LookTarget::Ground(106)));
    }

    #[test]
    fn get_top_visible_immovable_down_item_over_ground() {
        let mut items: SlotMap<ItemId, _> = SlotMap::with_key();
        let tree = items.insert(());
        let body = tile_body(Some(106), vec![tree], vec![], vec![]);
        let got = top_visible_look_target_from_body(&body, |_| true, |_| true);
        assert_eq!(got, Some(LookTarget::Item(tree)));
    }

    #[test]
    fn get_top_visible_skips_look_through_to_ground() {
        let mut items: SlotMap<ItemId, _> = SlotMap::with_key();
        let transparent = items.insert(());
        let opaque = |id: ItemId| id != transparent;
        let body = tile_body(Some(1), vec![transparent], vec![], vec![]);
        let got = top_visible_look_target_from_body(&body, |_| true, opaque);
        assert_eq!(got, Some(LookTarget::Ground(1)));
    }

    #[test]
    fn get_top_visible_creature_wins_over_items() {
        let mut items: SlotMap<ItemId, _> = SlotMap::with_key();
        let tree = items.insert(());
        let mut creatures: SlotMap<CreatureId, _> = SlotMap::with_key();
        let monster = creatures.insert(());
        let body = tile_body(Some(106), vec![tree], vec![], vec![monster]);
        let got = top_visible_look_target_from_body(&body, |_| true, |_| true);
        assert_eq!(got, Some(LookTarget::Creature(monster)));
    }

    #[test]
    fn get_item_stack_pos_roundtrip_for_down_item() {
        let mut items: SlotMap<ItemId, _> = SlotMap::with_key();
        let ladder = items.insert(());
        let body = tile_body(Some(106), vec![ladder], vec![], vec![]);
        let tile = Tile::Normal(body);
        let stack = tile.get_item_stack_pos(ladder).expect("ladder stack pos");
        assert_eq!(stack, 1);
        assert_eq!(tile.item_id_at_stack_pos(stack), Some(ladder));
        assert_eq!(tile.item_id_for_use(stack, |_| false), Some(ladder));
    }

    #[test]
    fn get_use_item_prefers_container_when_stack_misses() {
        let mut items: SlotMap<ItemId, _> = SlotMap::with_key();
        let bag = items.insert(());
        let body = tile_body(Some(106), vec![bag], vec![], vec![]);
        let tile = Tile::Normal(body);
        assert_eq!(tile.item_id_for_use(99, |id| id == bag), Some(bag));
    }
}
