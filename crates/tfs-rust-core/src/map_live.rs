//! Live non-house tiles that differ from the OTBM.
//!
//! The OTBM stays the original map (`ORIGMAPPATH`). `{map}-live.bin` is the
//! saved difference (`MAPPATH` `SaveSector` content): one compact blob of dirty
//! tiles. Load replaces those tiles; it does not append. Unchanged ground is
//! omitted. A leftover `{map}-live.ron` is read once when the bin is absent.
//!
//! Corpus: `map.cc` `SaveSector` / `LoadSector`; `operate.cc` `RefreshMap`.
//! Houses stay in `tile_store`. Gated by `persistMapItems`.

use std::fs;
use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use tfs_rust_common::{Position, PropStream, PropWriteStream};

use crate::container::Container;
use crate::cylinder::{Cylinder, CylinderFlags, INDEX_WHEREEVER};
use crate::game_world::GameWorld;
use crate::ids::ItemId;
use crate::item::Item;
use crate::item_blob::{parse_item_blob, write_item_blob};
use crate::map::{LiveItemBaseline, LiveTileBaseline};
use crate::tile::Tile;

const LIVE_MAGIC: &[u8; 4] = b"LM01";
const GROUND_KEEP: u8 = 0;
const GROUND_REPLACE: u8 = 1;
const GROUND_CLEAR: u8 = 2;

/// In-memory tile list. On disk this is `{map}-live.bin`, not pretty text.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LiveMapDocument {
    #[serde(default)]
    pub tiles: Vec<LiveTileDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LiveTileDocument {
    pub x: u16,
    pub y: u16,
    pub z: u8,
    /// `None` keeps the OTBM ground unless [`Self::clear_ground`] is set.
    #[serde(default)]
    pub ground: Option<LiveItemDocument>,
    /// Ground was removed. Distinct from omitting an unchanged floor.
    #[serde(default)]
    pub clear_ground: bool,
    #[serde(default)]
    pub down: Vec<LiveItemDocument>,
    #[serde(default)]
    pub top: Vec<LiveItemDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LiveItemDocument {
    pub server_id: u16,
    #[serde(default = "default_count")]
    pub count: u16,
    #[serde(default)]
    pub attrs: Vec<u8>,
    #[serde(default)]
    pub children: Vec<LiveItemDocument>,
}

fn default_count() -> u16 {
    1
}

impl LiveTileDocument {
    fn position(&self) -> Position {
        Position::new(self.x, self.y, self.z)
    }
}

impl GameWorld {
    /// Record the pre-mutation tree and mark the tile dirty. House tiles are skipped.
    pub(crate) fn note_live_tile(&mut self, pos: Position) {
        let Some(tile) = self.map.get_tile(pos) else {
            return;
        };
        if matches!(tile, Tile::House(_)) {
            return;
        }
        if !self.map.live_baselines.contains_key(&pos)
            && let Some(base) = self.tile_baseline(pos)
        {
            self.map.live_baselines.insert(pos, base);
        }
        self.map.live_dirty.insert(pos);
    }

    /// Replace saved tiles onto the OTBM. Refresh tiles are snapshotted first.
    pub(crate) fn apply_live_document(&mut self, doc: &LiveMapDocument) {
        let tiles = doc.tiles.clone();
        for tile in tiles {
            self.apply_one_live_tile(tile);
        }
    }

    /// Read `{map}-live.bin` when the knob is on. A missing file is not an error.
    /// If the bin is absent and `{map}-live.ron` exists, that legacy file is applied once.
    pub(crate) fn load_live_map(&mut self) {
        if !self.persist_map_items_enabled() {
            return;
        }
        let Some(path) = self.live_map_path.clone() else {
            return;
        };
        if path.exists() {
            let bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "live map read failed");
                    return;
                }
            };
            match decode_live_bytes(&bytes) {
                Ok(doc) => {
                    let n = doc.tiles.len();
                    self.apply_live_document(&doc);
                    tracing::info!(path = %path.display(), tiles = n, "loaded live map");
                }
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "live map parse failed");
                }
            }
            return;
        }
        let ron_path = path.with_extension("ron");
        if !ron_path.exists() {
            return;
        }
        let text = match fs::read_to_string(&ron_path) {
            Ok(text) => text,
            Err(e) => {
                tracing::warn!(path = %ron_path.display(), error = %e, "legacy live map read failed");
                return;
            }
        };
        match ron::from_str::<LiveMapDocument>(&text) {
            Ok(doc) => {
                let n = doc.tiles.len();
                self.apply_live_document(&doc);
                tracing::info!(
                    path = %ron_path.display(),
                    tiles = n,
                    "loaded legacy live map; next save writes the bin"
                );
            }
            Err(e) => {
                tracing::warn!(path = %ron_path.display(), error = %e, "legacy live map parse failed");
            }
        }
    }

    /// Write dirty tiles that still differ from their OTBM baseline.
    ///
    /// No-op when `persistMapItems` is off. An empty document replaces a stale file.
    pub(crate) fn save_live_map(&mut self) {
        if !self.persist_map_items_enabled() {
            return;
        }
        let Some(path) = self.live_map_path.clone() else {
            return;
        };
        let doc = self.encode_live_document();
        if let Err(e) = write_live_bin(&path, &doc) {
            tracing::warn!(path = %path.display(), error = %e, "live map save failed");
        } else {
            tracing::info!(path = %path.display(), tiles = doc.tiles.len(), "saved live map");
        }
    }

    fn persist_map_items_enabled(&self) -> bool {
        self.config.persist_map_items().unwrap_or(false)
    }

    fn encode_live_document(&mut self) -> LiveMapDocument {
        let dirty: Vec<Position> = self.map.live_dirty.iter().copied().collect();
        let mut still = FxHashSet::default();
        let mut tiles = Vec::new();
        for pos in dirty {
            if self
                .map
                .get_tile(pos)
                .is_none_or(|t| matches!(t, Tile::House(_)))
            {
                continue;
            }
            let Some(current) = self.tile_baseline(pos) else {
                continue;
            };
            if self.map.live_baselines.get(&pos) == Some(&current) {
                continue;
            }
            let stored = self.map.live_baselines.get(&pos).cloned();
            still.insert(pos);
            tiles.push(self.tile_document(pos, &current, stored.as_ref()));
        }
        tiles.sort_by_key(|t| (t.z, t.y, t.x));
        self.map.live_dirty = still;
        LiveMapDocument { tiles }
    }

    fn apply_one_live_tile(&mut self, saved: LiveTileDocument) {
        let pos = saved.position();
        let Some(tile) = self.map.get_tile(pos) else {
            tracing::warn!(x = pos.x, y = pos.y, z = pos.z, "live map tile missing");
            return;
        };
        if matches!(tile, Tile::House(_)) {
            return;
        }
        self.map.snapshot_refresh_if_needed(pos, &self.items);
        if !self.map.live_baselines.contains_key(&pos)
            && let Some(base) = self.tile_baseline(pos)
        {
            self.map.live_baselines.insert(pos, base);
        }
        self.strip_live_tile(pos, saved.ground.is_some() || saved.clear_ground);
        if let Some(ground) = saved.ground {
            self.place_live_item(pos, ground, true);
        }
        for item in saved.down.into_iter().rev() {
            self.place_live_item(pos, item, true);
        }
        for item in saved.top {
            self.place_live_item(pos, item, true);
        }
        self.map.live_dirty.insert(pos);
    }

    fn tile_baseline(&self, pos: Position) -> Option<LiveTileBaseline> {
        let tile = self.map.get_tile(pos)?;
        if matches!(tile, Tile::House(_)) {
            return None;
        }
        let body = tile.body();
        Some(LiveTileBaseline {
            ground: body.ground_item.and_then(|id| self.item_baseline(id)),
            down: body
                .down_items()
                .iter()
                .filter_map(|id| self.item_baseline(*id))
                .collect(),
            top: body
                .top_items()
                .iter()
                .filter_map(|id| self.item_baseline(*id))
                .collect(),
        })
    }

    fn item_baseline(&self, id: ItemId) -> Option<LiveItemBaseline> {
        let item = self.items.get(id)?;
        let children = self
            .container_registry
            .get(id)
            .map(|c| {
                c.items
                    .iter()
                    .filter_map(|cid| self.item_baseline(*cid))
                    .collect()
            })
            .unwrap_or_default();
        Some(LiveItemBaseline {
            item_type: item.item_type,
            count: item.count,
            attributes: item.attributes.clone(),
            children,
        })
    }

    fn tile_document(
        &self,
        pos: Position,
        current: &LiveTileBaseline,
        original: Option<&LiveTileBaseline>,
    ) -> LiveTileDocument {
        let (ground, clear_ground) = match original {
            Some(base) if base.ground == current.ground => (None, false),
            _ if current.ground.is_none() => (None, true),
            _ => (
                current.ground.as_ref().map(|g| self.item_document(g)),
                false,
            ),
        };
        LiveTileDocument {
            x: pos.x,
            y: pos.y,
            z: pos.z,
            ground,
            clear_ground,
            down: current
                .down
                .iter()
                .map(|it| self.item_document(it))
                .collect(),
            top: current
                .top
                .iter()
                .map(|it| self.item_document(it))
                .collect(),
        }
    }

    fn item_document(&self, item: &LiveItemBaseline) -> LiveItemDocument {
        let runtime = Item {
            item_type: item.item_type,
            count: item.count,
            attributes: item.attributes.clone(),
            parent: None,
        };
        LiveItemDocument {
            server_id: item.item_type,
            count: item.count,
            attrs: write_item_blob(&runtime, &self.items_db),
            children: item
                .children
                .iter()
                .rev()
                .map(|c| self.item_document(c))
                .collect(),
        }
    }

    fn strip_live_tile(&mut self, pos: Position, replace_ground: bool) {
        let ids: Vec<ItemId> = self
            .map
            .get_tile(pos)
            .map(|t| {
                let b = t.body();
                let ground = if replace_ground {
                    b.ground_item.into_iter()
                } else {
                    None.into_iter()
                };
                ground
                    .chain(b.down_items().iter().copied())
                    .chain(b.top_items().iter().copied())
                    .collect()
            })
            .unwrap_or_default();
        for id in ids {
            let tree = self.live_container_tree(id);
            let _ = self.internal_remove_item_from_tile(pos, id, u16::MAX);
            for tid in tree {
                self.container_registry.remove(tid);
                self.items.remove(tid);
            }
        }
    }

    fn live_container_tree(&self, root: ItemId) -> Vec<ItemId> {
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            out.push(id);
            if let Some(c) = self.container_registry.get(id) {
                stack.extend(c.items.iter().copied());
            }
        }
        out
    }

    fn place_live_item(
        &mut self,
        pos: Position,
        saved: LiveItemDocument,
        on_tile: bool,
    ) -> Option<ItemId> {
        if !self.items_db.items.contains_key(&saved.server_id) {
            tracing::warn!(
                itemtype = saved.server_id,
                x = pos.x,
                y = pos.y,
                z = pos.z,
                "live map unknown item — skipped"
            );
            return None;
        }
        // Fluid `0` is empty. `.max(1)` would turn it into water (subtype 1).
        let fluid = self
            .items_db
            .items
            .get(&saved.server_id)
            .is_some_and(|t| t.is_fluid_container() || t.is_splash());
        let count = if fluid {
            saved.count
        } else {
            saved.count.max(1)
        };
        let mut item = Item::new(saved.server_id, count);
        if !saved.attrs.is_empty() {
            let is_container = self.items_db.is_container(saved.server_id);
            match parse_item_blob(&saved.attrs, is_container) {
                Ok(parsed) => {
                    item.attributes = Some(Box::new(parsed.attrs));
                    if let Some(st) = parsed.subtype_override {
                        item.count = if fluid {
                            u16::from(st)
                        } else {
                            u16::from(st).max(1)
                        };
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        itemtype = saved.server_id,
                        error = %e,
                        "live map item attrs skipped"
                    );
                }
            }
        }
        let iid = self.items.insert(item);
        let is_container = self.items_db.is_container(saved.server_id);
        if is_container || !saved.children.is_empty() {
            let cap = self.container_capacity(saved.server_id);
            let mut reg = std::mem::take(&mut self.container_registry);
            reg.register(Container::new(iid, cap));
            self.container_registry = reg;
        }
        for child in saved.children {
            let Some(cid) = self.place_live_item(pos, child, false) else {
                continue;
            };
            self.attach_live_child(iid, cid);
        }
        if on_tile {
            let flags = CylinderFlags::NO_LIMIT
                .union(CylinderFlags::NO_MERGE)
                .union(CylinderFlags::IGNORE_AUTO_STACK);
            if self.internal_add_item_to_tile(pos, iid, flags).is_err() {
                self.container_registry.remove(iid);
                self.items.remove(iid);
                return None;
            }
        }
        Some(iid)
    }

    fn attach_live_child(&mut self, parent: ItemId, child: ItemId) {
        let mut reg = std::mem::take(&mut self.container_registry);
        if let Some(c) = reg.get_mut(parent) {
            c.internal_add_item_front(child);
        }
        if let Some(ch) = reg.get_mut(child) {
            ch.parent_container = Some(parent);
        }
        self.container_registry = reg;
        if let Some(ch) = self.items.get_mut(child) {
            ch.parent = Some(Cylinder::Container {
                item_id: parent,
                index: INDEX_WHEREEVER,
            });
        }
    }
}

fn write_live_bin(path: &Path, doc: &LiveMapDocument) -> anyhow::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    let body = encode_live_bytes(doc)?;
    let tmp = temp_path(path);
    fs::write(&tmp, body)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn encode_live_bytes(doc: &LiveMapDocument) -> anyhow::Result<Vec<u8>> {
    let mut w = PropWriteStream::new();
    write_raw(&mut w, LIVE_MAGIC);
    w.write_u32(doc.tiles.len() as u32);
    for tile in &doc.tiles {
        w.write_u16(tile.x);
        w.write_u16(tile.y);
        w.write_u8(tile.z);
        if let Some(ground) = &tile.ground {
            w.write_u8(GROUND_REPLACE);
            write_live_item(&mut w, ground);
        } else if tile.clear_ground {
            w.write_u8(GROUND_CLEAR);
        } else {
            w.write_u8(GROUND_KEEP);
        }
        write_live_list(&mut w, &tile.down);
        write_live_list(&mut w, &tile.top);
    }
    Ok(w.finish())
}

fn write_live_list(w: &mut PropWriteStream, items: &[LiveItemDocument]) {
    w.write_u32(items.len() as u32);
    for item in items {
        write_live_item(w, item);
    }
}

fn write_live_item(w: &mut PropWriteStream, item: &LiveItemDocument) {
    w.write_u16(item.server_id);
    w.write_u16(item.count);
    w.write_u32(item.attrs.len() as u32);
    write_raw(w, &item.attrs);
    w.write_u32(item.children.len() as u32);
    for child in &item.children {
        write_live_item(w, child);
    }
}

fn decode_live_bytes(bytes: &[u8]) -> anyhow::Result<LiveMapDocument> {
    let mut stream = PropStream::new(bytes);
    let magic = read_exact(&mut stream, LIVE_MAGIC.len())?;
    if magic.as_slice() != LIVE_MAGIC {
        anyhow::bail!("live map magic mismatch");
    }
    let n = stream.read_u32()?;
    let mut tiles = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let x = stream.read_u16()?;
        let y = stream.read_u16()?;
        let z = stream.read_u8()?;
        let tag = stream.read_u8()?;
        let (ground, clear_ground) = match tag {
            GROUND_KEEP => (None, false),
            GROUND_REPLACE => (Some(read_live_item(&mut stream)?), false),
            GROUND_CLEAR => (None, true),
            other => anyhow::bail!("live map ground tag {other}"),
        };
        tiles.push(LiveTileDocument {
            x,
            y,
            z,
            ground,
            clear_ground,
            down: read_live_list(&mut stream)?,
            top: read_live_list(&mut stream)?,
        });
    }
    Ok(LiveMapDocument { tiles })
}

fn read_live_list(stream: &mut PropStream<'_>) -> anyhow::Result<Vec<LiveItemDocument>> {
    let n = stream.read_u32()?;
    let mut items = Vec::with_capacity(n as usize);
    for _ in 0..n {
        items.push(read_live_item(stream)?);
    }
    Ok(items)
}

fn read_live_item(stream: &mut PropStream<'_>) -> anyhow::Result<LiveItemDocument> {
    let server_id = stream.read_u16()?;
    let count = stream.read_u16()?;
    let attr_len = stream.read_u32()? as usize;
    let attrs = read_exact(stream, attr_len)?;
    let n = stream.read_u32()?;
    let mut children = Vec::with_capacity(n as usize);
    for _ in 0..n {
        children.push(read_live_item(stream)?);
    }
    Ok(LiveItemDocument {
        server_id,
        count,
        attrs,
        children,
    })
}

fn write_raw(w: &mut PropWriteStream, bytes: &[u8]) {
    for byte in bytes {
        w.write_u8(*byte);
    }
}

fn read_exact(stream: &mut PropStream<'_>, len: usize) -> anyhow::Result<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        out.push(stream.read_u8()?);
    }
    Ok(out)
}

fn temp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("live.bin");
    path.with_file_name(format!(".{name}.tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cylinder::CylinderFlags;
    use crate::item::Item;
    use crate::test_support::{ensure_walkable_tile, minimal_world};
    use crate::tile::flags as tile_flags;
    use tfs_rust_common::Position;

    const SWORD: u16 = 2148;
    const DROP: u16 = 1987;

    fn pos() -> Position {
        Position::new(100, 100, 7)
    }

    fn tile_with_sword(world: &mut GameWorld) {
        ensure_walkable_tile(&mut world.map, pos(), 100);
        let sword = world.items.insert(Item::new_single(SWORD));
        world
            .internal_add_item_to_tile(pos(), sword, CylinderFlags::NO_LIMIT)
            .expect("sword");
        world.map.live_baselines.clear();
        world.map.live_dirty.clear();
        world.map.refresh_snapshots.clear();
    }

    fn ids_of_type(world: &GameWorld, item_type: u16) -> Vec<ItemId> {
        let body = world.map.get_tile(pos()).unwrap().body();
        body.down_items()
            .iter()
            .chain(body.top_items().iter())
            .copied()
            .filter(|id| {
                world
                    .items
                    .get(*id)
                    .is_some_and(|it| it.item_type == item_type)
            })
            .collect()
    }

    fn doc_sword_and_drop() -> LiveMapDocument {
        LiveMapDocument {
            tiles: vec![LiveTileDocument {
                x: 100,
                y: 100,
                z: 7,
                ground: None,
                clear_ground: false,
                down: vec![
                    LiveItemDocument {
                        server_id: SWORD,
                        count: 1,
                        attrs: Vec::new(),
                        children: Vec::new(),
                    },
                    LiveItemDocument {
                        server_id: DROP,
                        count: 1,
                        attrs: Vec::new(),
                        children: Vec::new(),
                    },
                ],
                top: Vec::new(),
            }],
        }
    }

    #[test]
    fn picked_up_item_stays_gone_after_replace() {
        let mut world = minimal_world();
        tile_with_sword(&mut world);
        let doc = LiveMapDocument {
            tiles: vec![LiveTileDocument {
                x: 100,
                y: 100,
                z: 7,
                ground: None,
                clear_ground: false,
                down: Vec::new(),
                top: Vec::new(),
            }],
        };
        world.apply_live_document(&doc);
        assert!(ids_of_type(&world, SWORD).is_empty());
        assert!(world.map.get_tile(pos()).is_some());
    }

    #[test]
    fn drop_is_not_duplicated_beside_original() {
        let mut world = minimal_world();
        tile_with_sword(&mut world);
        world.apply_live_document(&doc_sword_and_drop());
        assert_eq!(ids_of_type(&world, SWORD).len(), 1);
        assert_eq!(ids_of_type(&world, DROP).len(), 1);
    }

    #[test]
    fn refresh_snapshot_is_otbm_before_replace() {
        let mut world = minimal_world();
        tile_with_sword(&mut world);
        if let Some(t) = world.map.get_tile_mut(pos()) {
            t.body_mut().flags |= tile_flags::REFRESH;
        }
        world.map.refresh_positions.insert(pos());
        world.apply_live_document(&doc_sword_and_drop());
        let snap = world.map.refresh_snapshots.get(&pos()).expect("snap");
        let types: Vec<u16> = snap
            .down
            .iter()
            .chain(snap.top.iter())
            .map(|it| it.item_type)
            .collect();
        assert_eq!(types, vec![SWORD]);
        assert_eq!(world.refresh_map(), 1);
        assert!(ids_of_type(&world, DROP).is_empty());
        assert_eq!(ids_of_type(&world, SWORD).len(), 1);
        assert!(!world.map.live_dirty.contains(&pos()));
    }

    #[test]
    fn empty_saved_stack_removes_otbm_item() {
        let mut world = minimal_world();
        tile_with_sword(&mut world);
        world.apply_live_document(&LiveMapDocument {
            tiles: vec![LiveTileDocument {
                x: 100,
                y: 100,
                z: 7,
                ground: None,
                clear_ground: false,
                down: Vec::new(),
                top: Vec::new(),
            }],
        });
        assert!(ids_of_type(&world, SWORD).is_empty());
        assert!(world.map.get_tile(pos()).unwrap().body().ground.is_some());
    }

    #[test]
    fn knob_off_does_not_apply_file() {
        let mut world = minimal_world();
        tile_with_sword(&mut world);
        let dir =
            std::env::temp_dir().join(format!("tfs-live-map-{}-{}", std::process::id(), pos().x));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("forgotten-live.bin");
        write_live_bin(&path, &doc_sword_and_drop()).unwrap();
        world.live_map_path = Some(path);
        assert!(!world.config.persist_map_items().unwrap());
        world.load_live_map();
        assert!(ids_of_type(&world, DROP).is_empty());
        assert_eq!(ids_of_type(&world, SWORD).len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn restored_baseline_is_omitted_from_save() {
        let mut world = minimal_world();
        tile_with_sword(&mut world);
        world.note_live_tile(pos());
        let sword = ids_of_type(&world, SWORD)[0];
        world
            .internal_remove_item_from_tile(pos(), sword, u16::MAX)
            .expect("pickup");
        let again = world.items.insert(Item::new_single(SWORD));
        world
            .internal_add_item_to_tile(pos(), again, CylinderFlags::NO_LIMIT)
            .expect("put back");
        let doc = world.encode_live_document();
        assert!(doc.tiles.is_empty());
    }

    #[test]
    fn empty_vial_count_zero_stays_empty() {
        let mut world = minimal_world();
        ensure_walkable_tile(&mut world.map, pos(), 100);
        let mut items = std::collections::HashMap::clone(&world.items_db.items);
        let mut vial = tfs_rust_content::otb::ItemType::default();
        vial.id = 2874;
        vial.server_id = 2874;
        vial.group = tfs_rust_content::otb::ItemType::GROUP_FLUID;
        items.insert(2874, vial);
        world.items_db = std::sync::Arc::new(tfs_rust_content::items::ItemDatabase {
            items,
            client_to_server: std::collections::HashMap::new(),
        });
        world.apply_live_document(&LiveMapDocument {
            tiles: vec![LiveTileDocument {
                x: 100,
                y: 100,
                z: 7,
                ground: None,
                clear_ground: false,
                down: vec![LiveItemDocument {
                    server_id: 2874,
                    count: 0,
                    attrs: vec![15, 0],
                    children: Vec::new(),
                }],
                top: Vec::new(),
            }],
        });
        let placed = ids_of_type(&world, 2874);
        assert_eq!(placed.len(), 1);
        assert_eq!(world.items.get(placed[0]).unwrap().count, 0);
    }

    #[test]
    fn unchanged_ground_is_omitted_from_save() {
        let mut world = minimal_world();
        tile_with_sword(&mut world);
        world.note_live_tile(pos());
        let sword = ids_of_type(&world, SWORD)[0];
        world
            .internal_remove_item_from_tile(pos(), sword, u16::MAX)
            .expect("pickup");
        let doc = world.encode_live_document();
        assert_eq!(doc.tiles.len(), 1);
        assert!(doc.tiles[0].ground.is_none());
        assert!(!doc.tiles[0].clear_ground);
        assert!(doc.tiles[0].down.is_empty());
    }

    #[test]
    fn binary_roundtrip_keeps_empty_vial_count() {
        let doc = LiveMapDocument {
            tiles: vec![LiveTileDocument {
                x: 32355,
                y: 32212,
                z: 6,
                ground: None,
                clear_ground: false,
                down: vec![LiveItemDocument {
                    server_id: 2874,
                    count: 0,
                    attrs: vec![15, 0],
                    children: Vec::new(),
                }],
                top: Vec::new(),
            }],
        };
        let bytes = encode_live_bytes(&doc).expect("encode");
        let back = decode_live_bytes(&bytes).expect("decode");
        assert_eq!(back, doc);
    }

    #[test]
    fn legacy_ron_loads_when_bin_is_absent() {
        let mut world = minimal_world();
        tile_with_sword(&mut world);
        let dir =
            std::env::temp_dir().join(format!("tfs-live-ron-{}-{}", std::process::id(), pos().x));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let cfg = dir.join("config.lua");
        fs::write(&cfg, "persistMapItems = true\n").unwrap();
        world.config = std::rc::Rc::new(crate::config::ConfigManager::load(&cfg).unwrap());
        let text = ron::ser::to_string(&doc_sword_and_drop()).unwrap();
        fs::write(dir.join("forgotten-live.ron"), text).unwrap();
        world.live_map_path = Some(dir.join("forgotten-live.bin"));
        world.load_live_map();
        assert_eq!(ids_of_type(&world, DROP).len(), 1);
        assert_eq!(ids_of_type(&world, SWORD).len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}
