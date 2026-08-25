//! House `tile_store` blobs — TFS `IOMapSerialize::{saveTile,saveItem,loadHouseItems}`.
//! C++ reference: `iomapserialize.cpp` `saveTile` / `saveItem` / `loadItem` / `loadContainer`.
//!
//! Per-tile: `u16 x, u16 y, u8 z, u32 item_count`, then nested `saveItem` payloads.
//! Ground is never stored. Nested containers use `ATTR_CONTAINER_ITEMS` (23).

use tfs_rust_common::{Position, PropStream, PropWriteStream};
use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_db::TileStoreRow;

use crate::container::ContainerType;
use crate::game_world::GameWorld;
use crate::ids::ItemId;
use crate::item::Item;
use crate::item_attributes::AttrType;
use crate::item_blob::{parse_item_blob_from_stream, write_item_blob};
use crate::tile::Tile;

/// Decoded house-tile item (tests + loader).
#[derive(Debug, Clone)]
pub struct LoadedHouseItem {
    pub server_id: u16,
    pub item: Item,
    pub children: Vec<LoadedHouseItem>,
}

fn item_door_id(item: &Item) -> u8 {
    item.attributes
        .as_deref()
        .map(|a| a.get_door_id())
        .unwrap_or(0)
}

/// Whether a map item should be written to `tile_store`.
/// TFS `saveTile` keeps: `moveable || door || (container && !empty) || canWriteText || bed`.
pub fn should_save_house_item(world: &GameWorld, item_id: ItemId) -> bool {
    let Some(item) = world.items.get(item_id) else {
        return false;
    };
    let Some(it) = world.items_db.items.get(&item.item_type) else {
        return item.attributes.is_some();
    };
    if it.moveable() {
        return true;
    }
    if item_door_id(item) != 0 {
        return true;
    }
    if it.can_write_text {
        return true;
    }
    if it.is_bed() {
        return true;
    }
    if world.items_db.is_container(item.item_type)
        && world
            .container_registry
            .get(item_id)
            .is_some_and(|c| !c.items.is_empty())
    {
        return true;
    }
    false
}

fn tile_save_items(world: &GameWorld, tile: &Tile) -> Vec<ItemId> {
    let body = tile.body();
    let mut ids: Vec<ItemId> = body
        .down_items
        .iter()
        .copied()
        .chain(body.top_items.iter().copied())
        .filter(|&id| should_save_house_item(world, id))
        .collect();
    // C++ `saveTile` push_front → reverse of tile list order.
    ids.reverse();
    ids
}

fn write_saved_item(world: &GameWorld, item_id: ItemId, w: &mut PropWriteStream) {
    let Some(item) = world.items.get(item_id) else {
        return;
    };
    w.write_u16(item.item_type);
    let attr = write_item_blob(item, &world.items_db);
    for b in attr {
        w.write_u8(b);
    }
    let children = world
        .container_registry
        .get(item_id)
        .map(|c| {
            let mut v = c.items.clone();
            v.reverse();
            v
        })
        .unwrap_or_default();
    if !children.is_empty() {
        w.write_u8(AttrType::ContainerItems as u8);
        w.write_u32(children.len() as u32);
        for child in children {
            write_saved_item(world, child, w);
        }
    }
    w.write_u8(0);
}

fn encode_one_tile(world: &GameWorld, pos: Position, tile: &Tile) -> Option<Vec<u8>> {
    let ids = tile_save_items(world, tile);
    if ids.is_empty() {
        return None;
    }
    let mut w = PropWriteStream::new();
    w.write_u16(pos.x);
    w.write_u16(pos.y);
    w.write_u8(pos.z);
    w.write_u32(ids.len() as u32);
    for id in ids {
        write_saved_item(world, id, &mut w);
    }
    Some(w.finish())
}

/// Encode all house tiles that have persistable items (`saveHouseItems`).
pub fn encode_house_tile_store(world: &GameWorld) -> Vec<TileStoreRow> {
    let mut rows = Vec::new();
    for (house_id, rec) in &world.houses.records {
        for &pos in &rec.tiles {
            let Some(tile) = world.map.get_tile(pos) else {
                continue;
            };
            if let Some(data) = encode_one_tile(world, pos, tile) {
                rows.push(TileStoreRow::new(*house_id, data));
            }
        }
    }
    rows
}

fn read_saved_item(stream: &mut PropStream<'_>, items_db: &tfs_rust_content::items::ItemDatabase) -> Result<LoadedHouseItem> {
    let server_id = stream.read_u16()?;
    let is_container = items_db.is_container(server_id);
    let parsed = parse_item_blob_from_stream(stream, is_container)?;
    let mut item = Item::new_single(server_id);
    item.attributes = Some(Box::new(parsed.attrs));
    if let Some(st) = parsed.subtype_override {
        item.count = u16::from(st).max(1);
    }
    let mut children = Vec::new();
    if let Some(n) = parsed.container_item_count {
        for _ in 0..n {
            children.push(read_saved_item(stream, items_db)?);
        }
        let end = stream.read_u8()?;
        if end != 0 {
            return Err(TfsRustError::PropStream(
                "house item missing container attr end".into(),
            ));
        }
    }
    Ok(LoadedHouseItem {
        server_id,
        item,
        children,
    })
}

/// Decode one `tile_store.data` blob (one or more tiles concatenated).
pub fn decode_tile_store_blob(
    data: &[u8],
    items_db: &tfs_rust_content::items::ItemDatabase,
) -> Result<Vec<(Position, Vec<LoadedHouseItem>)>> {
    let mut stream = PropStream::new(data);
    let mut out = Vec::new();
    loop {
        let x = match stream.read_u16() {
            Ok(v) => v,
            Err(_) => break,
        };
        let y = stream.read_u16()?;
        let z = stream.read_u8()?;
        let count = stream.read_u32()?;
        let mut items = Vec::new();
        for _ in 0..count {
            items.push(read_saved_item(&mut stream, items_db)?);
        }
        out.push((Position::new(x, y, z), items));
    }
    Ok(out)
}

fn place_loaded_item(world: &mut GameWorld, pos: Position, loaded: LoadedHouseItem) -> Option<ItemId> {
    if world.items_db.items.get(&loaded.server_id).is_none() {
        tracing::warn!(itemtype = loaded.server_id, "house tile_store unknown item — skipped");
        return None;
    }
    // Stationary door/bed: match existing map item and overlay attrs (TFS `loadItem`).
    if let Some(existing) = find_matching_stationary(world, pos, &loaded) {
        if let Some(dst) = world.items.get_mut(existing) {
            dst.attributes = loaded.item.attributes;
            if loaded.item.count > 0 {
                dst.count = loaded.item.count;
            }
        }
        for child in loaded.children {
            let Some(cid) = place_loaded_item(world, pos, child) else {
                continue;
            };
            add_child_to_container(world, existing, cid);
        }
        return Some(existing);
    }
    let mut item = loaded.item;
    item.item_type = loaded.server_id;
    let iid = world.items.insert(item);
    if !loaded.children.is_empty() || world.items_db.is_container(loaded.server_id) {
        let cap = world.container_capacity(loaded.server_id);
        let mut reg = std::mem::take(&mut world.container_registry);
        reg.register(crate::container::Container::new(iid, cap));
        world.container_registry = reg;
    }
    for child in loaded.children {
        let Some(cid) = place_loaded_item(world, pos, child) else {
            continue;
        };
        add_child_to_container(world, iid, cid);
    }
    let _ = world.internal_add_item_to_tile(pos, iid, crate::cylinder::CylinderFlags::NO_LIMIT);
    Some(iid)
}

fn find_matching_stationary(world: &GameWorld, pos: Position, loaded: &LoadedHouseItem) -> Option<ItemId> {
    let it = world.items_db.items.get(&loaded.server_id)?;
    if it.moveable() {
        return None;
    }
    let tile = world.map.get_tile(pos)?;
    let want_door = item_door_id(&loaded.item);
    for &id in tile
        .body()
        .down_items
        .iter()
        .chain(tile.body().top_items.iter())
    {
        let Some(item) = world.items.get(id) else {
            continue;
        };
        if item.item_type != loaded.server_id {
            continue;
        }
        if want_door != 0 && item_door_id(item) != want_door {
            continue;
        }
        return Some(id);
    }
    None
}

fn add_child_to_container(world: &mut GameWorld, parent: ItemId, child: ItemId) {
    let mut reg = std::mem::take(&mut world.container_registry);
    if let Some(c) = reg.get_mut(parent) {
        c.internal_add_item_front(child);
    }
    if let Some(ch) = reg.get_mut(child) {
        ch.parent_container = Some(parent);
        ch.container_type = ContainerType::Normal;
    }
    world.container_registry = reg;
}

/// Hydrate `tile_store` rows onto the map (`IOMapSerialize::loadHouseItems`).
pub fn load_tile_store_into_world(world: &mut GameWorld, rows: &[TileStoreRow]) {
    for row in rows {
        match decode_tile_store_blob(&row.data, &world.items_db) {
            Ok(tiles) => {
                for (pos, items) in tiles {
                    // Dynamic (moveable) contents are extra; stationary doors/beds overlay attrs.
                    for item in items {
                        let _ = place_loaded_item(world, pos, item);
                    }
                }
            }
            Err(e) => {
                tracing::warn!(house_id = row.house_id_u32(), error = %e, "tile_store blob parse failed");
            }
        }
    }
}

impl GameWorld {
    /// Collect `tile_store` rows for save (`IOMapSerialize::saveHouseItems`).
    pub fn encode_house_tile_store(&self) -> Vec<TileStoreRow> {
        encode_house_tile_store(self)
    }

    pub fn load_house_tile_store(&mut self, rows: &[TileStoreRow]) {
        load_tile_store_into_world(self, rows);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Item;
    use std::collections::HashMap;
    use tfs_rust_common::PropWriteStream;
    use tfs_rust_content::items::ItemDatabase;
    use tfs_rust_content::otb::ItemType;

    #[test]
    fn tile_blob_roundtrip_nested_container() {
        // Manually encode x,y,z,count + bag (container) containing a gold coin.
        let bag = 1987u16;
        let gold = 2148u16;
        let db = {
            let mut items = HashMap::new();
            let mut bag_ty = ItemType {
                server_id: bag,
                group: ItemType::GROUP_CONTAINER,
                ..Default::default()
            };
            bag_ty.flags |= 1 << 6;
            let mut gold_ty = ItemType {
                server_id: gold,
                ..Default::default()
            };
            gold_ty.flags |= (1 << 6) | (1 << 7); // moveable + stackable
            items.insert(bag, bag_ty);
            items.insert(gold, gold_ty);
            ItemDatabase {
                items,
                client_to_server: HashMap::new(),
            }
        };

        let mut inner = PropWriteStream::new();
        inner.write_u16(gold);
        let gold_item = Item::new(gold, 5);
        for b in write_item_blob(&gold_item, &db) {
            inner.write_u8(b);
        }
        inner.write_u8(0);
        let inner_bytes = inner.finish();

        let mut w = PropWriteStream::new();
        w.write_u16(100);
        w.write_u16(200);
        w.write_u8(7);
        w.write_u32(1);
        w.write_u16(bag);
        for b in write_item_blob(&Item::new_single(bag), &db) {
            w.write_u8(b);
        }
        w.write_u8(AttrType::ContainerItems as u8);
        w.write_u32(1);
        for b in inner_bytes {
            w.write_u8(b);
        }
        w.write_u8(0);
        let blob = w.finish();

        let tiles = decode_tile_store_blob(&blob, &db).expect("decode");
        assert_eq!(tiles.len(), 1);
        assert_eq!(tiles[0].0, Position::new(100, 200, 7));
        assert_eq!(tiles[0].1.len(), 1);
        assert_eq!(tiles[0].1[0].server_id, bag);
        assert_eq!(tiles[0].1[0].children.len(), 1);
        assert_eq!(tiles[0].1[0].children[0].server_id, gold);
        assert_eq!(tiles[0].1[0].children[0].item.count, 5);
    }
}
