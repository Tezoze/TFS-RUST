//! Switch-floor step in/out + depot pad announce.
//!
//! Pack surface: former `data/scripts/movements/other/tiles.lua` MoveEvent.
//! ID pairs load from that file at startup (`SteppingTiles.stepIn` / `stepOut`).
//! Aid MoveEvents (demon helmet, annihilator) still run — this never skips Lua.
//! Mapped floors still press/unpress when the item has a quest aid; TFS item-id
//! MoveEvents did not fire in that case (`getEvent` prefers aid).
//!
//! C++ reference: TFS `MoveEvents::onCreatureMove` / `executeStep`; depot count
//! is pack policy (`getDepotLocker` + `getItemHoldingCount`), not decompile.

use std::collections::HashMap;
use std::path::Path;

use crate::creature::CreatureKind;
use crate::event_dispatcher::TileMoveEventItem;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::tile::flags as tilestate;
use mlua::{Lua, Value};
use slotmap::Key;
use tfs_rust_common::Position;

/// Lua `MESSAGE_STATUS_DEFAULT` (`const.h`).
const MESSAGE_STATUS_DEFAULT: u8 = 0x15;
/// Lua `MESSAGE_INFO_DESCR` (`const.h`).
const MESSAGE_INFO_DESCR: u8 = 0x16;

/// Boot-time `from → to` maps for pressed / unpressed floor sprites.
#[derive(Clone, Debug, Default)]
pub struct SteppingTileMaps {
    pub step_in: HashMap<u16, u16>,
    pub step_out: HashMap<u16, u16>,
}

/// Load `data/defs/tiles.lua`. Missing/invalid → empty maps.
pub fn load_from_data_dir(data_dir: &Path) -> SteppingTileMaps {
    let path = data_dir.join("defs/tiles.lua");
    match load_from_file(&path) {
        Ok(maps) => {
            tracing::info!(
                file = %path.display(),
                step_in = maps.step_in.len(),
                step_out = maps.step_out.len(),
                "loaded stepping tile id maps"
            );
            maps
        }
        Err(e) => {
            tracing::warn!(
                file = %path.display(),
                error = %e,
                "stepping tile id maps not loaded"
            );
            SteppingTileMaps::default()
        }
    }
}

fn load_from_file(path: &Path) -> Result<SteppingTileMaps, String> {
    let chunk = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let lua = Lua::new();
    let value: Value = lua
        .load(&chunk)
        .set_name(path.display().to_string())
        .eval()
        .map_err(|e| e.to_string())?;
    let Value::Table(root) = value else {
        return Err("tiles.lua must return a table".into());
    };
    Ok(SteppingTileMaps {
        step_in: parse_id_map(&root, "stepIn")?,
        step_out: parse_id_map(&root, "stepOut")?,
    })
}

fn parse_id_map(root: &mlua::Table, key: &str) -> Result<HashMap<u16, u16>, String> {
    let value: Value = root.get(key).map_err(|e| e.to_string())?;
    let Value::Table(table) = value else {
        return Ok(HashMap::new());
    };
    let mut out = HashMap::new();
    for pair in table.pairs::<Value, Value>() {
        let (k, v) = pair.map_err(|e| e.to_string())?;
        let Some(from) = value_as_u16(&k) else {
            continue;
        };
        let Some(to) = value_as_u16(&v) else {
            continue;
        };
        out.insert(from, to);
    }
    Ok(out)
}

fn value_as_u16(v: &Value) -> Option<u16> {
    match v {
        Value::Integer(i) if *i >= 0 && *i <= i64::from(u16::MAX) => Some(*i as u16),
        Value::Number(n) if *n >= 0.0 && *n <= f64::from(u16::MAX) => Some(*n as u16),
        _ => None,
    }
}

/// Native StepIn/StepOut for switch floors. Does not cancel Lua MoveEvents.
pub fn on_creature_step(
    world: &mut GameWorld,
    cid: CreatureId,
    from: Position,
    to: Position,
    step_out_items: &[TileMoveEventItem],
    step_in_items: &[TileMoveEventItem],
) {
    let _ = from;
    for item in step_out_items {
        apply_step_out(world, cid, item);
    }
    for item in step_in_items {
        apply_step_in(world, cid, to, item);
    }
}

fn apply_step_in(world: &mut GameWorld, cid: CreatureId, pos: Position, item: &TileMoveEventItem) {
    let Some(&dest) = world.stepping_tiles.step_in.get(&item.item_type) else {
        return;
    };
    if !matches!(world.creatures.get(cid), Some(CreatureKind::Player(_))) {
        return;
    }
    if player_is_ghost(world, cid) {
        return;
    }
    if tile_has_protection_zone(world, pos) && facing_depot_item(world, cid).is_some() {
        announce_depot(world, cid);
        transform_floor(world, item.item_id, dest);
        return;
    }
    transform_floor(world, item.item_id, dest);
}

fn apply_step_out(world: &mut GameWorld, cid: CreatureId, item: &TileMoveEventItem) {
    let Some(&dest) = world.stepping_tiles.step_out.get(&item.item_type) else {
        return;
    };
    if matches!(world.creatures.get(cid), Some(CreatureKind::Player(_)))
        && player_is_ghost(world, cid)
    {
        return;
    }
    transform_floor(world, item.item_id, dest);
}

fn player_is_ghost(world: &GameWorld, cid: CreatureId) -> bool {
    matches!(
        world.creatures.get(cid),
        Some(CreatureKind::Player(p)) if p.ghost_mode
    )
}

fn tile_has_protection_zone(world: &GameWorld, pos: Position) -> bool {
    world
        .map
        .get_tile(pos)
        .is_some_and(|t| t.body().flags & tilestate::PROTECTIONZONE != 0)
}

fn facing_depot_item(world: &GameWorld, cid: CreatureId) -> Option<ItemId> {
    let (look, dir) = match world.creatures.get(cid) {
        Some(k) => (k.position(), k.base().direction),
        None => return None,
    };
    let look = look.offset(dir);
    let tile = world.map.get_tile(look)?;
    let body = tile.body();
    let ids = body
        .ground_item
        .into_iter()
        .chain(body.top_items.iter().copied())
        .chain(body.down_items.iter().copied());
    for iid in ids {
        let Some(item) = world.items.get(iid) else {
            continue;
        };
        if world.items_db.is_depot(item.item_type) {
            return Some(iid);
        }
    }
    None
}

fn announce_depot(world: &mut GameWorld, cid: CreatureId) {
    let Some(depot_item) = facing_depot_item(world, cid) else {
        return;
    };
    let uid = u32::from(
        world
            .items
            .get(depot_item)
            .map(|i| i.unique_id())
            .unwrap_or(0),
    );
    let depot_id = world.depot_id_from_locker_item(depot_item, uid as i32);
    if depot_id == 0 {
        return;
    }
    let Some(locker) = world.player_get_depot_locker(cid, depot_id) else {
        return;
    };
    let holding = world
        .script_container_data(locker)
        .map(|d| d.item_holding_count)
        .unwrap_or(0);
    let shown = holding.max(1);
    let noun = if shown == 1 { "item." } else { "items." };
    let text = format!("Your depot contains {shown} {noun}");
    let _ = world.lua_script_player_send_text_message(
        cid.data().as_ffi(),
        MESSAGE_STATUS_DEFAULT,
        text,
    );
    let max_items = world.player_get_max_depot_items(cid);
    // Pack Lua had `max >= count` (almost always true). Full means count reached the cap.
    if holding >= max_items {
        let _ = world.lua_script_player_send_text_message(
            cid.data().as_ffi(),
            MESSAGE_INFO_DESCR,
            "Your depot is full. Remove surplus items before storing new ones.".into(),
        );
    }
}

fn transform_floor(world: &mut GameWorld, item_id: ItemId, new_type: u16) {
    let _ = world.lua_script_item_transform(item_id.data().as_ffi(), new_type, -1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::CreatureKind;
    use crate::cylinder::Cylinder;
    use crate::item::Item;
    use crate::sim_harness::{insert_player, minimal_world, pickup_item_type, test_player};
    use crate::tile::{Tile, TileBody};
    use std::sync::Arc;
    use tfs_rust_common::enums::ZoneType;
    use tfs_rust_content::items::ITEM_TYPE_DEPOT;

    fn workspace_data() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    fn register_types(world: &mut GameWorld, ids: &[u16]) {
        let db = Arc::get_mut(&mut world.items_db).expect("unique items_db");
        for &id in ids {
            db.items.entry(id).or_insert_with(|| pickup_item_type(id));
        }
    }

    fn place_ground(world: &mut GameWorld, pos: Position, type_id: u16, aid: u16) -> ItemId {
        let mut item = Item::new_single(type_id);
        if aid != 0 {
            item.set_action_id(aid);
        }
        item.parent = Some(Cylinder::Tile { pos });
        let iid = world.items.insert(item);
        world.map.insert_tile(
            pos,
            Tile::Normal(TileBody {
                ground: Some(type_id),
                ground_item: Some(iid),
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: 0,
                zone: ZoneType::Normal,
            }),
        );
        iid
    }

    fn snap(world: &GameWorld, iid: ItemId) -> TileMoveEventItem {
        let item = world.items.get(iid).expect("item");
        TileMoveEventItem {
            item_id: iid,
            item_type: item.item_type,
            action_id: item.action_id(),
            unique_id: item.unique_id(),
        }
    }

    #[test]
    fn load_stepping_tiles_from_pack_lua() {
        let maps = load_from_data_dir(&workspace_data());
        assert_eq!(maps.step_in.get(&416), Some(&417));
        assert_eq!(maps.step_in.get(&426), Some(&425));
        assert_eq!(maps.step_out.get(&417), Some(&416));
        assert_eq!(maps.step_out.get(&425), Some(&426));
    }

    #[test]
    fn stepping_tile_step_in_transforms_without_aid() {
        let mut world = minimal_world();
        register_types(&mut world, &[416, 417]);
        world.stepping_tiles.step_in.insert(416, 417);
        let pos = Position::new(50, 50, 7);
        let iid = place_ground(&mut world, pos, 416, 0);
        let cid = insert_player(&mut world, test_player("Pad", pos));
        let ev = snap(&world, iid);
        on_creature_step(&mut world, cid, pos, pos, &[], &[ev]);
        assert_eq!(world.items.get(iid).unwrap().item_type, 417);
    }

    #[test]
    fn stepping_tile_aid_still_transforms() {
        let mut world = minimal_world();
        register_types(&mut world, &[426, 425]);
        world.stepping_tiles.step_in.insert(426, 425);
        let pos = Position::new(50, 50, 7);
        let iid = place_ground(&mut world, pos, 426, 3022);
        let cid = insert_player(&mut world, test_player("Quest", pos));
        let ev = snap(&world, iid);
        on_creature_step(&mut world, cid, pos, pos, &[], &[ev]);
        assert_eq!(world.items.get(iid).unwrap().item_type, 425);
    }

    #[test]
    fn stepping_tile_ghost_does_not_transform() {
        let mut world = minimal_world();
        register_types(&mut world, &[416, 417]);
        world.stepping_tiles.step_in.insert(416, 417);
        let pos = Position::new(50, 50, 7);
        let iid = place_ground(&mut world, pos, 416, 0);
        let cid = insert_player(&mut world, test_player("Ghost", pos));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            p.ghost_mode = true;
        }
        let ev = snap(&world, iid);
        on_creature_step(&mut world, cid, pos, pos, &[], &[ev]);
        assert_eq!(world.items.get(iid).unwrap().item_type, 416);
    }

    #[test]
    fn stepping_tile_depot_pad_transforms_in_pz() {
        let mut world = minimal_world();
        register_types(&mut world, &[416, 417, 2589]);
        if let Some(db) = Arc::get_mut(&mut world.items_db) {
            if let Some(t) = db.items.get_mut(&2589) {
                t.type_tag = ITEM_TYPE_DEPOT;
            }
        }
        world.stepping_tiles.step_in.insert(416, 417);
        let pos = Position::new(50, 50, 7);
        let north = Position::new(50, 49, 7);
        let iid = place_ground(&mut world, pos, 416, 0);
        if let Some(tile) = world.map.get_tile_mut(pos) {
            tile.body_mut().flags |= tilestate::PROTECTIONZONE;
        }
        let mut locker = Item::new_single(2589);
        locker.set_unique_id(1);
        locker.set_depot_id(1);
        locker.parent = Some(Cylinder::Tile { pos: north });
        let depot_iid = world.items.insert(locker);
        world.map.insert_tile(
            north,
            Tile::Normal(TileBody {
                ground: Some(100),
                ground_item: None,
                down_items: vec![depot_iid],
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: 0,
                zone: ZoneType::Normal,
            }),
        );
        let cid = insert_player(&mut world, test_player("Depot", pos));
        let ev = snap(&world, iid);
        on_creature_step(&mut world, cid, pos, pos, &[], &[ev]);
        assert_eq!(world.items.get(iid).unwrap().item_type, 417);
    }
}
