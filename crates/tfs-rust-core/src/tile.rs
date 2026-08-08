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

    // ── 772 sight / throw bits (24–26) ──
    /// CipSoft 772 `UNTHROW` — projectile-blocking (`ItemType::block_projectile`). Distinct from
    /// `BLOCKPATH`/`BLOCKSOLID`; used only by 772 `Map::throw_possible` (`info.cc:1189`).
    pub const UNTHROW: u32 = 1 << 24;
    /// CipSoft 772 `HOOKEAST` — wall hook facing east (`is_hangable && is_horizontal`).
    pub const HOOKEAST: u32 = 1 << 25;
    /// CipSoft 772 `HOOKSOUTH` — wall hook facing south (`is_hangable && is_vertical`).
    pub const HOOKSOUTH: u32 = 1 << 26;

    // ── Composite masks ──
    pub const FLOORCHANGE: u32 = FLOORCHANGE_DOWN
        | FLOORCHANGE_NORTH
        | FLOORCHANGE_SOUTH
        | FLOORCHANGE_EAST
        | FLOORCHANGE_WEST
        | FLOORCHANGE_SOUTH_ALT
        | FLOORCHANGE_EAST_ALT;

    // Legacy aliases used by map/LOS code.
    pub const BLOCK_SOLID: u32 = BLOCKSOLID;
    pub const BLOCK_PROJECTILE: u32 = BLOCKPATH;
}

#[derive(Debug, Clone)]
pub struct TileBody {
    pub ground: Option<u16>,
    /// Item instance for the ground — TFS `Tile::getGround()` returns a full `Item*`.
    /// Needed so StepIn/transform/decay can mutate the ground (e.g. pitfall 293↔294).
    pub ground_item: Option<ItemId>,
    /// Non-ground items below creatures on the wire (`Tile::getBeginDownItem`, `src/tile.cpp`).
    pub down_items: Vec<ItemId>,
    /// Always-on-top items, sent before creatures (`getBeginTopItem` … `getEndTopItem`).
    pub top_items: Vec<ItemId>,
    pub creatures: Vec<CreatureId>,
    pub flags: u32,
    pub zone: ZoneType,
}

impl Default for TileBody {
    fn default() -> Self {
        Self::new()
    }
}

impl TileBody {
    pub fn new() -> Self {
        Self {
            ground: None,

            ground_item: None,
            down_items: Vec::new(),
            top_items: Vec::new(),
            creatures: Vec::new(),
            flags: 0,
            zone: ZoneType::Normal,
        }
    }

    /// 772 map-container object chain — `GetFirstObject` / `getNextObject` (`map.cc:2356`, `cract.cc:89-103`, `crnonpl.cc:2185+`).
    ///
    /// `CONTENT` head is the ground BANK (when present), then items bottom→top (`down_items` stored
    /// with index 0 = top of down stack), then always-on-top items, then creatures. 772 uses
    /// creature-container objects in the chain; Rust approximates creatures at the tail.
    pub fn map_object_chain(&self) -> Vec<MapStackEntry> {
        let mut out = Vec::new();
        if let Some(g) = self.ground {
            out.push(MapStackEntry::Ground(g));
        }
        if !self.down_items.is_empty() {
            for &id in self.down_items.iter().rev() {
                out.push(MapStackEntry::Item(id));
            }
        }
        for &id in &self.top_items {
            out.push(MapStackEntry::Item(id));
        }
        for &cid in &self.creatures {
            out.push(MapStackEntry::Creature(cid));
        }
        out
    }
}

/// One step in the 772 tile object linked list (`map.cc` `GetFirstObject` walk).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapStackEntry {
    Ground(u16),
    Item(crate::ids::ItemId),
    Creature(CreatureId),
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
            // Order-preserving remove — Cip `CutObject` splices the map-container list.
            // `swap_remove` reorders remaining creatures and desyncs MoveCreature stackpos.
            body.creatures.remove(i);
            return true;
        }
        false
    }

    /// Add an item to this tile (adds to front of down_items, matching C++ `Tile::addThing`).
    // C++ ref: src/tile.cpp Tile::addThing — non-always-on-top items insert at begin of downItems.
    pub fn add_item(&mut self, item_id: ItemId) {
        self.body_mut().down_items.insert(0, item_id);
    }

    /// Add an always-on-top item to this tile, inserted at `index` (for sorted insertion
    /// by `alwaysOnTopOrder`, matching C++ `Tile::addThing` `tile.cpp:898-906`). The caller
    /// computes the insertion index by comparing `alwaysOnTopOrder` against existing top
    /// items — items with equal order are inserted BEFORE existing ones (`<=`), so a splash
    /// (order 2) lands before a ladder (order 2). This keeps the server's `top_items` vector
    /// in the same order the 772 client renders them (the client sorts by `.dat`
    /// `alwaysOnTopOrder` on `0x6A` add, which omits stackpos). Without this,
    /// `get_item_stack_pos` returns a different index than the client's, causing `0x6C`
    /// remove to delete the wrong item (e.g. a ladder instead of a splash).
    pub fn add_top_item_at(&mut self, item_id: ItemId, index: usize) {
        let items = &mut self.body_mut().top_items;
        if index >= items.len() {
            items.push(item_id);
        } else {
            items.insert(index, item_id);
        }
    }

    /// Add an always-on-top item to this tile (append to end, unsorted).
    /// Only use when the caller cannot compute the `alwaysOnTopOrder` insertion index
    /// (e.g. OTBM load, where items are already in map-editor order). For runtime adds
    /// via `internal_add_item_to_tile`, use `add_top_item_at` with a sorted index.
    pub fn add_top_item(&mut self, item_id: ItemId) {
        self.body_mut().top_items.push(item_id);
    }

    /// Remove an item from this tile by its ItemId. Returns the index it was removed from, or None.
    // C++ ref: src/tile.cpp Tile::removeThing — preserves ordering.
    pub fn remove_item_by_id(&mut self, item_id: ItemId) -> Option<usize> {
        let body = self.body_mut();
        // Try ground first
        if body.ground_item == Some(item_id) {
            body.ground = None;
            body.ground_item = None;
            return Some(0);
        }
        // Try down_items
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
        body.ground_item == Some(item_id)
            || body.down_items.contains(&item_id)
            || body.top_items.contains(&item_id)
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
    ///
    /// `cip_order`: Cip map-container (Bank→Bottom→Top→Creature) vs TVP
    /// (ground→top→creatures→bottom). Must match the order used in
    /// [`tfs_rust_net::map_description::TileContent::cip_map_order`].
    // C++ ref: `GetObjectRNum` (`info.cc:205`) / TVP `Tile::getStackposOfItem`.
    pub fn get_item_stack_pos(&self, item_id: ItemId) -> Option<u8> {
        self.get_item_stack_pos_ordered(item_id, false)
    }

    /// Stackpos with explicit tile-order mode (see [`Self::get_item_stack_pos`]).
    ///
    /// When `cip_order`, `is_priority_bottom` distinguishes Cip BOTTOM (before creatures)
    /// from LOW (after creatures) among `down_items`.
    pub fn get_item_stack_pos_ordered(
        &self,
        item_id: ItemId,
        cip_order: bool,
    ) -> Option<u8> {
        // Legacy cip path treated every down item as BOTTOM — wrong for PRIORITY_LOW.
        // Prefer [`Self::get_item_stack_pos_cip`] at GameWorld call sites.
        self.get_item_stack_pos_cip(item_id, cip_order, |_| true)
    }

    /// Item stackpos with Cip BOTTOM vs LOW classification.
    ///
    /// `is_priority_bottom`: true for Cip `PRIORITY_BOTTOM` (pools/splashes — not magic
    /// fields; those are `PRIORITY_LOW`). When `cip_order` is false, ignored (TVP order).
    pub fn get_item_stack_pos_cip(
        &self,
        item_id: ItemId,
        cip_order: bool,
        is_priority_bottom: impl Fn(ItemId) -> bool,
    ) -> Option<u8> {
        let body = self.body();
        // Ground item occupies stackpos 0 — TFS `Tile::getItemStackPos` (`tile.cpp`).
        if body.ground_item == Some(item_id) {
            return Some(0);
        }
        let mut n: u8 = if body.ground.is_some() { 1 } else { 0 };
        if cip_order {
            // Oldest down first (rev of newest-first storage) — matches Cip PlaceObject append.
            for &did in body.down_items.iter().rev() {
                if !is_priority_bottom(did) {
                    continue;
                }
                if did == item_id {
                    return Some(n);
                }
                n = n.saturating_add(1);
            }
            for &tid in &body.top_items {
                if tid == item_id {
                    return Some(n);
                }
                n = n.saturating_add(1);
            }
            // Creatures occupy the next slots; LOW items follow.
            //
            // `PlaceObject` (`map.cc:2036-2075`) forces `Append` for every priority except
            // CREATURE and LOW, so a LOW item enters the chain *before* the existing LOW
            // group (`!Append && CurPriority >= ObjPriority` breaks on the first LOW). The
            // 7.72 client applies the same rule when it inserts an `0x6A` add, which carries
            // no stackpos. `down_items` is stored newest-first, so LOW is walked forward.
            n = n.saturating_add(body.creatures.len() as u8);
            for &did in &body.down_items {
                if is_priority_bottom(did) {
                    continue;
                }
                if did == item_id {
                    return Some(n);
                }
                n = n.saturating_add(1);
            }
            None
        } else {
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
    }

    /// Count of things before down_items start (ground + top_items + creatures) — TVP order.
    // Used to compute stack_pos for a newly-added down item at index 0.
    pub fn down_item_start_stack_pos(&self) -> u8 {
        let body = self.body();
        let mut n: u8 = if body.ground.is_some() { 1 } else { 0 };
        n = n.saturating_add(body.top_items.len() as u8);
        n = n.saturating_add(body.creatures.len() as u8);
        n
    }

    /// C++ `Tile::getUseItem` — `tile.cpp` ~1603 (container priority + `getThing` stack walk).
    ///
    /// `cip_order` / `is_priority_bottom` select the tile order the client indexes against —
    /// see [`Self::item_id_at_stack_pos_ordered`]. They must match the order the same viewer
    /// received the tile in, or a Use lands on the neighbouring object.
    pub fn item_id_for_use<F, G>(
        &self,
        stack_pos: u8,
        cip_order: bool,
        is_priority_bottom: G,
        is_container: F,
    ) -> Option<ItemId>
    where
        F: Fn(ItemId) -> bool,
        G: Fn(ItemId) -> bool,
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

        let thing_at = self.item_id_at_stack_pos_ordered(stack_pos, cip_order, is_priority_bottom);

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
        self.item_id_at_stack_pos_ordered(stack_pos, false, |_| true)
    }

    /// Inverse of [`Self::get_item_stack_pos_cip`] — the two must stay in lockstep, since one
    /// numbers the objects the client is told about and the other reads that numbering back.
    pub fn item_id_at_stack_pos_ordered(
        &self,
        stack_pos: u8,
        cip_order: bool,
        is_priority_bottom: impl Fn(ItemId) -> bool,
    ) -> Option<ItemId> {
        let body = self.body();
        // Ground item occupies stackpos 0 — must match `get_item_stack_pos_cip` (`:302`).
        if stack_pos == 0 {
            return body.ground_item;
        }
        let mut n: u8 = if body.ground.is_some() { 1 } else { 0 };

        if cip_order {
            // BOTTOM is an appended group (oldest heads it); LOW is not (newest heads it).
            for &did in body.down_items.iter().rev() {
                if !is_priority_bottom(did) {
                    continue;
                }
                if n == stack_pos {
                    return Some(did);
                }
                n = n.saturating_add(1);
            }
        }

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
            if cip_order && is_priority_bottom(did) {
                continue;
            }
            if n == stack_pos {
                return Some(did);
            }
            n = n.saturating_add(1);
        }
        None
    }

    /// C++ `Tile::getTopVisibleThing` — `tile.cpp` ~322–347.
    pub fn top_visible_look_target<F, G>(
        &self,
        can_see_creature: F,
        item_is_opaque: G,
    ) -> Option<LookTarget>
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

/// C++ `Tile::getClientIndexOfCreature` — viewer-aware stack position.
///
/// Only counts creatures the viewer can see (`player->canSeeCreature(c)`,
/// `tile.cpp:1207-1214`). The `creatures` slice is the tile's creature list
/// **before** the move (including the moving creature). Invisible creatures
/// below the target are skipped in the count — matching the gameserver which
/// gates each increment on `player->canSeeCreature(c)`.
///
/// `items_before_creatures` is ground-relative item count before the creature
/// section: TVP = `top_items.len()`; Cip `GetObjectRNum` = BOTTOM downs + `top_items`
/// (not PRIORITY_LOW downs).
// C++ reference: `gameserver/src/tile.cpp` `Tile::getClientIndexOfCreature`.
pub fn creature_stack_pos_for_viewer(
    ground_present: bool,
    items_before_creatures: usize,
    creatures: &[CreatureId],
    creature: CreatureId,
    can_see: impl Fn(CreatureId) -> bool,
) -> i32 {
    let mut n: i32 = if ground_present { 1 } else { 0 };
    n += items_before_creatures as i32;
    for &c in creatures.iter().rev() {
        if c == creature {
            return n;
        }
        if can_see(c) {
            n += 1;
        }
    }
    -1
}

/// TFS / TVP `Tile::getClientIndexOfCreature` (simplified: all creatures visible).
///
/// Does **not** count `down_items` (emitted after creatures on the wire).
// C++ reference: `src/tile.cpp` `Tile::getClientIndexOfCreature`.
pub fn client_creature_stack_pos(body: &TileBody, creature: CreatureId) -> i32 {
    creature_stack_pos_for_viewer(
        body.ground.is_some(),
        body.top_items.len(),
        &body.creatures,
        creature,
        |_| true,
    )
}

/// Cip `GetObjectRNum` creature index — only BOTTOM downs (+ tops) before creatures.
///
/// `bottom_down_count` = number of `down_items` with Cip `PRIORITY_BOTTOM`
/// (pools/splashes via `ItemType::is_cip_priority_bottom`). Magic fields are LOW and
/// must not inflate this index (`objects.srv` / `GetObjectPriority`).
pub fn client_creature_stack_pos_cip(
    body: &TileBody,
    creature: CreatureId,
    bottom_down_count: usize,
) -> i32 {
    creature_stack_pos_for_viewer(
        body.ground.is_some(),
        bottom_down_count + body.top_items.len(),
        &body.creatures,
        creature,
        |_| true,
    )
}

#[cfg(test)]
mod look_tests {
    use super::*;
    use slotmap::SlotMap;

    fn tile_body(
        ground: Option<u16>,
        down: Vec<ItemId>,
        top: Vec<ItemId>,
        creatures: Vec<CreatureId>,
    ) -> TileBody {
        TileBody {
            ground,
            ground_item: None,
            down_items: down,
            top_items: top,
            creatures,
            flags: 0,
            zone: ZoneType::Normal,
        }
    }

    #[test]
    fn map_object_chain_ground_then_down_bottom_to_top() {
        let mut items: slotmap::SlotMap<crate::ids::ItemId, ()> = slotmap::SlotMap::with_key();
        let bottom = items.insert(());
        let top = items.insert(());
        let body = TileBody {
            ground: Some(102),

            ground_item: None,
            down_items: vec![top, bottom],
            top_items: vec![],
            creatures: vec![],
            flags: 0,
            zone: ZoneType::Normal,
        };
        let chain = body.map_object_chain();
        assert_eq!(
            chain,
            vec![
                MapStackEntry::Ground(102),
                MapStackEntry::Item(bottom),
                MapStackEntry::Item(top),
            ]
        );
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
        assert_eq!(
            tile.item_id_for_use(stack, false, |_| false, |_| false),
            Some(ladder)
        );
    }

    #[test]
    fn cip_stackpos_counts_down_items_before_creatures() {
        let mut items: SlotMap<ItemId, _> = SlotMap::with_key();
        let field = items.insert(());
        let mut creatures: SlotMap<CreatureId, _> = SlotMap::with_key();
        let mob = creatures.insert(());
        let body = tile_body(Some(106), vec![field], vec![], vec![mob]);
        // TVP: ground(1) + creature(1) + field → field at 2
        assert_eq!(
            Tile::Normal(body.clone()).get_item_stack_pos_ordered(field, false),
            Some(2)
        );
        // Cip BOTTOM field: ground(1) + field → field at 1; creature at 2
        assert_eq!(
            Tile::Normal(body.clone()).get_item_stack_pos_cip(field, true, |_| true),
            Some(1)
        );
        assert_eq!(client_creature_stack_pos(&body, mob), 1);
        assert_eq!(client_creature_stack_pos_cip(&body, mob, 1), 2);

        // Cip PRIORITY_LOW down item does NOT bump creature stackpos.
        let low = items.insert(());
        let body_low = tile_body(Some(106), vec![low], vec![], vec![mob]);
        assert_eq!(client_creature_stack_pos_cip(&body_low, mob, 0), 1);
        assert_eq!(
            Tile::Normal(body_low.clone()).get_item_stack_pos_cip(low, true, |_| false),
            Some(2) // ground + creature + low
        );
    }

    /// `PlaceObject` (`map.cc:2040`) does not append LOW objects, so the most recently
    /// placed one heads the group. The 7.72 client inserts an `0x6A` add the same way;
    /// walking LOW oldest-first made `0x6C` remove the item underneath and ghost the
    /// moved one.
    #[test]
    fn cip_stackpos_puts_newest_low_item_first() {
        let mut items: SlotMap<ItemId, _> = SlotMap::with_key();
        let older = items.insert(());
        let newer = items.insert(());
        // `Tile::add_item` inserts at the front, so index 0 is the most recent.
        let body = tile_body(Some(106), vec![newer, older], vec![], vec![]);
        let tile = Tile::Normal(body);
        assert_eq!(tile.get_item_stack_pos_cip(newer, true, |_| false), Some(1));
        assert_eq!(tile.get_item_stack_pos_cip(older, true, |_| false), Some(2));
    }

    /// The stackpos the client is given and the stackpos it sends back must resolve to the
    /// same object. Without a Cip-ordered inverse, a Use on a tile holding a field plus two
    /// ordinary items landed one object off.
    #[test]
    fn cip_stackpos_round_trips_through_the_inverse_lookup() {
        let mut items: SlotMap<ItemId, _> = SlotMap::with_key();
        let field = items.insert(());
        let ladder = items.insert(());
        let newer = items.insert(());
        let older = items.insert(());
        let mut creatures: SlotMap<CreatureId, _> = SlotMap::with_key();
        let mob = creatures.insert(());
        let is_bottom = |id: ItemId| id == field;

        // down_items newest-first: LOW newer, LOW older, BOTTOM field.
        let body = tile_body(
            Some(106),
            vec![newer, older, field],
            vec![ladder],
            vec![mob],
        );
        let tile = Tile::Normal(body);

        // ground 0, field 1, ladder 2, creature 3, newer 4, older 5.
        for item in [field, ladder, newer, older] {
            let stack = tile
                .get_item_stack_pos_cip(item, true, is_bottom)
                .expect("stack pos");
            assert_eq!(
                tile.item_id_at_stack_pos_ordered(stack, true, is_bottom),
                Some(item),
                "stackpos {stack} must resolve back to the item it was computed for"
            );
        }
        assert_eq!(tile.item_id_at_stack_pos_ordered(3, true, is_bottom), None);
    }

    /// Ground items occupy stackpos 0 in `get_item_stack_pos_cip`; the inverse must resolve
    /// stackpos 0 back to the ground item. Without this, a Use/Move targeting the ground tile
    /// (stackpos 0) returned `None` instead of the ground `ItemId`.
    #[test]
    fn ground_item_stackpos_0_round_trips_through_the_inverse_lookup() {
        let mut items: SlotMap<ItemId, _> = SlotMap::with_key();
        let ground_item = items.insert(());
        let ladder = items.insert(());
        let mut body = tile_body(Some(106), vec![ladder], vec![], vec![]);
        body.ground_item = Some(ground_item);
        let tile = Tile::Normal(body);

        let stack = tile
            .get_item_stack_pos_cip(ground_item, true, |_| false)
            .expect("ground item stack pos");
        assert_eq!(stack, 0);
        assert_eq!(
            tile.item_id_at_stack_pos_ordered(0, true, |_| false),
            Some(ground_item),
            "stackpos 0 must resolve back to the ground item"
        );
        // Non-ordered path too.
        assert_eq!(
            tile.item_id_at_stack_pos(0),
            Some(ground_item),
            "stackpos 0 must resolve back to the ground item (non-ordered)"
        );
        // Stackpos 0 on a tile with ground but no ground_item → None.
        let body2 = tile_body(Some(106), vec![ladder], vec![], vec![]);
        let tile2 = Tile::Normal(body2);
        assert_eq!(
            tile2.item_id_at_stack_pos_ordered(0, true, |_| false),
            None,
            "stackpos 0 with no ground_item must return None"
        );
    }

    #[test]
    fn get_use_item_prefers_container_when_stack_misses() {
        let mut items: SlotMap<ItemId, _> = SlotMap::with_key();
        let bag = items.insert(());
        let body = tile_body(Some(106), vec![bag], vec![], vec![]);
        let tile = Tile::Normal(body);
        assert_eq!(
            tile.item_id_for_use(99, false, |_| false, |id| id == bag),
            Some(bag)
        );
    }
}
