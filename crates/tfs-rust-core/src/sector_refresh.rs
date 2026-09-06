//! Live tile refresh — OTBM snapshot restore (TFS `Tile::refresh` / `Map::refreshMap`).
//!
//! Corpus `RefreshSector` reloads ORIGMAP `.sec` patches (`map.cc:1307-1350`).
//! This shard ships OTBM, so the pack surface wins: clone items at load, restore
//! on `refreshMap()` / minute cron. Houses are not snapshotted (`House::addTile`
//! clears REFRESH). Creatures stay. Decay stays in `ProcessCronSystem`.
//!
//! Minute job is `RefreshCylinders` (`operate.cc:2964-2988`, `main.cc:383`) — one
//! ORIGMAP 32×32 XY column, all Z, skip floors a player `CanSeeFloor`. Full
//! `RefreshMap` (`operate.cc:2895`) is reboot/`Game.refreshMap()` only.

use std::collections::BTreeSet;

use slotmap::SlotMap;
use tfs_rust_common::Position;

use crate::creature::CreatureKind;
use crate::cylinder::CylinderFlags;
use crate::game_world::GameWorld;
use crate::ids::ItemId;
use crate::item::Item;
use crate::item_attributes::ItemAttributes;
use crate::tile::TileBody;

/// ORIGMAP sector edge — `RefreshCylinders` / `SectorRefreshable` (`operate.cc:2796`, `:2964`).
const ORIGMAP_SECTOR: u16 = 32;
/// Inclusive `TFindCreatures` radius (`operate.cc:2799` `32-1`).
const REFRESH_SEARCH_RADIUS: i32 = 31;

/// Raster state for `RefreshCylinders` (`operate.cc:2964`).
#[derive(Debug, Default)]
pub(crate) struct RefreshCylinderState {
    xys: Vec<(u16, u16)>,
    next: usize,
    indexed: bool,
}

impl RefreshCylinderState {
    fn ensure_index(&mut self, snapshots: &std::collections::HashMap<Position, TileRefreshSnap>) {
        if self.indexed {
            return;
        }
        let mut set = BTreeSet::new();
        for pos in snapshots.keys() {
            set.insert((pos.x / ORIGMAP_SECTOR, pos.y / ORIGMAP_SECTOR));
        }
        self.xys = set.into_iter().collect();
        self.indexed = true;
        self.next = 0;
    }

    fn next_xy(&mut self) -> Option<(u16, u16)> {
        if self.xys.is_empty() {
            return None;
        }
        let xy = self.xys[self.next];
        self.next += 1;
        if self.next >= self.xys.len() {
            self.next = 0;
        }
        Some(xy)
    }
}

/// One cloned map item (no live SlotMap id) — TFS `Item::clone` without unique re-register.
#[derive(Debug, Clone)]
pub struct RefreshItemSnap {
    pub item_type: u16,
    pub count: u16,
    pub attributes: Option<Box<ItemAttributes>>,
}

impl RefreshItemSnap {
    fn from_item(item: &Item) -> Self {
        Self {
            item_type: item.item_type,
            count: item.count,
            attributes: item.attributes.clone(),
        }
    }

    fn to_item(&self) -> Item {
        Item {
            item_type: self.item_type,
            count: self.count,
            attributes: self.attributes.clone(),
            parent: None,
        }
    }
}

/// Ground + stack order from load — TFS `makeRefreshItemList`.
#[derive(Debug, Clone, Default)]
pub struct TileRefreshSnap {
    pub ground: Option<RefreshItemSnap>,
    pub down: Vec<RefreshItemSnap>,
    pub top: Vec<RefreshItemSnap>,
}

impl TileRefreshSnap {
    /// Snapshot after OTBM items exist in `items`.
    pub fn from_tile(body: &TileBody, items: &SlotMap<ItemId, Item>) -> Self {
        let ground = body
            .ground_item
            .and_then(|id| items.get(id))
            .map(RefreshItemSnap::from_item);
        let down = body
            .down_items
            .iter()
            .filter_map(|&id| items.get(id).map(RefreshItemSnap::from_item))
            .collect();
        let top = body
            .top_items
            .iter()
            .filter_map(|&id| items.get(id).map(RefreshItemSnap::from_item))
            .collect();
        Self { ground, down, top }
    }
}

/// C++ `TCreature::CanSeeFloor` — `cr.hh:576-582`.
fn can_see_floor(viewer_z: u8, floor_z: u8) -> bool {
    if viewer_z <= 7 {
        floor_z <= 7
    } else {
        (viewer_z as i32 - floor_z as i32).abs() <= 2
    }
}

/// `SectorRefreshable` (`operate.cc:2796-2821`) — skip if a player in the 31-radius
/// box can `CanSeeFloor` this Z.
fn sector_refreshable(sx: u16, sy: u16, z: u8, players: &[Position]) -> bool {
    let center_x = i32::from(sx) * i32::from(ORIGMAP_SECTOR) + i32::from(ORIGMAP_SECTOR) / 2;
    let center_y = i32::from(sy) * i32::from(ORIGMAP_SECTOR) + i32::from(ORIGMAP_SECTOR) / 2;
    for p in players {
        let dx = i32::from(p.x) - center_x;
        let dy = i32::from(p.y) - center_y;
        if dx.abs() > REFRESH_SEARCH_RADIUS || dy.abs() > REFRESH_SEARCH_RADIUS {
            continue;
        }
        if can_see_floor(p.z, z) {
            return false;
        }
    }
    true
}

impl GameWorld {
    /// TFS `Map::refreshMap` / corpus `RefreshMap` — restore every snapshotted tile.
    /// Lua `Game.refreshMap()` / reboot path only. Minute cron must use [`Self::refresh_cylinders`].
    pub fn refresh_map(&mut self) -> u32 {
        let positions: Vec<Position> = self.map.refresh_snapshots.keys().copied().collect();
        let n = positions.len() as u32;
        for pos in positions {
            self.refresh_one_tile(pos);
        }
        n
    }

    /// Corpus `RefreshCylinders` — one ORIGMAP XY sector, all Z (`operate.cc:2964`).
    /// `RefreshedCylinders` default is 1 (`map.cc:351`).
    pub fn refresh_cylinders(&mut self) -> u32 {
        self.refresh_cylinder_state
            .ensure_index(&self.map.refresh_snapshots);
        let Some((sx, sy)) = self.refresh_cylinder_state.next_xy() else {
            return 0;
        };
        let players: Vec<Position> = self
            .conn_to_creature
            .values()
            .filter_map(|&cid| {
                let c = self.creatures.get(cid)?;
                matches!(c, CreatureKind::Player(_)).then(|| c.position())
            })
            .collect();
        let mut jobs = Vec::new();
        for z in 0u8..=15 {
            if !sector_refreshable(sx, sy, z, &players) {
                continue;
            }
            for ox in 0..ORIGMAP_SECTOR {
                for oy in 0..ORIGMAP_SECTOR {
                    let Some(x) = sx.checked_mul(ORIGMAP_SECTOR).and_then(|b| b.checked_add(ox))
                    else {
                        continue;
                    };
                    let Some(y) = sy.checked_mul(ORIGMAP_SECTOR).and_then(|b| b.checked_add(oy))
                    else {
                        continue;
                    };
                    let pos = Position::new(x, y, z);
                    if self.map.refresh_snapshots.contains_key(&pos) {
                        jobs.push(pos);
                    }
                }
            }
        }
        let n = jobs.len() as u32;
        for pos in jobs {
            self.refresh_one_tile(pos);
        }
        n
    }

    fn refresh_one_tile(&mut self, pos: Position) {
        let Some(snap) = self.map.refresh_snapshots.get(&pos).cloned() else {
            return;
        };
        let to_remove: Vec<ItemId> = self
            .map
            .get_tile(pos)
            .map(|t| {
                let b = t.body();
                b.ground_item
                    .into_iter()
                    .chain(b.down_items.iter().copied())
                    .chain(b.top_items.iter().copied())
                    .collect()
            })
            .unwrap_or_default();
        for iid in to_remove {
            let _ = self.internal_remove_item_from_tile(pos, iid, u16::MAX);
        }
        if let Some(g) = snap.ground {
            let iid = self.items.insert(g.to_item());
            let _ = self.internal_add_item_to_tile(pos, iid, CylinderFlags::NO_LIMIT);
        }
        for it in snap.down.into_iter().chain(snap.top) {
            let iid = self.items.insert(it.to_item());
            let _ = self.internal_add_item_to_tile(pos, iid, CylinderFlags::NO_LIMIT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Item;
    use crate::sim_harness::{beat_driven_test_world, ensure_walkable_tile};
    use crate::tile::flags as tile_flags;
    use tfs_rust_common::Position;

    fn snap_tile(world: &mut GameWorld, pos: Position) {
        ensure_walkable_tile(&mut world.map, pos, 100);
        if let Some(t) = world.map.get_tile_mut(pos) {
            t.body_mut().flags |= tile_flags::REFRESH;
        }
        let ground_id = world.items.insert(Item::new_single(100));
        if let Some(t) = world.map.get_tile_mut(pos) {
            t.body_mut().ground_item = Some(ground_id);
            t.body_mut().ground = Some(100);
        }
        world.map.refresh_snapshots.insert(
            pos,
            TileRefreshSnap::from_tile(world.map.get_tile(pos).unwrap().body(), &world.items),
        );
    }

    fn drop_junk(world: &mut GameWorld, pos: Position) -> ItemId {
        let junk = world.items.insert(Item::new_single(3031));
        world
            .internal_add_item_to_tile(pos, junk, CylinderFlags::NO_LIMIT)
            .expect("drop");
        junk
    }

    fn has_junk(world: &GameWorld, pos: Position, junk: ItemId) -> bool {
        let body = world.map.get_tile(pos).unwrap().body();
        body.down_items.contains(&junk) || body.top_items.contains(&junk)
    }

    #[test]
    fn refresh_restores_dropped_item_away() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(80, 80, 7);
        snap_tile(&mut world, pos);
        let junk = drop_junk(&mut world, pos);
        assert_eq!(world.refresh_map(), 1);
        assert!(
            !has_junk(&world, pos, junk),
            "dropped item must be gone after refresh"
        );
        assert!(world.items.get(junk).is_none());
    }

    #[test]
    fn refresh_cylinders_one_xy_sector_per_call() {
        let mut world = beat_driven_test_world();
        let a = Position::new(80, 80, 7);
        let b = Position::new(112, 80, 7);
        snap_tile(&mut world, a);
        snap_tile(&mut world, b);
        let junk_a = drop_junk(&mut world, a);
        let junk_b = drop_junk(&mut world, b);

        let n0 = world.refresh_cylinders();
        assert_eq!(n0, 1, "first cylinder is one ORIGMAP XY");
        assert!(!has_junk(&world, a, junk_a), "sector (2,2) restores first");
        assert!(has_junk(&world, b, junk_b), "other XY waits for next minute");

        let n1 = world.refresh_cylinders();
        assert_eq!(n1, 1);
        assert!(!has_junk(&world, b, junk_b));
    }

    #[test]
    fn sector_refreshable_skips_visible_floor() {
        let here = Position::new(80, 80, 7);
        assert!(!sector_refreshable(2, 2, 7, &[here]));
        assert!(
            sector_refreshable(2, 2, 7, &[Position::new(80, 80, 11)]),
            "z=11 cannot CanSeeFloor 7"
        );
        assert!(
            sector_refreshable(2, 2, 7, &[Position::new(112, 80, 7)]),
            "dx=32 is outside radius 31"
        );
    }
}
