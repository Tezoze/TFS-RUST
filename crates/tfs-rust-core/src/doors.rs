//! Door and key use + quest/level door step — pack `doors.lua` / `closing_doors.lua` / `level_doors.lua`.
//!
//! ID tables: `data/defs/doors.lua`. Mechanics stay native.
//!
//! C++ reference: TFS `doors.lua` onUse; 772 `UseKeyDoor` / `UseChangeObject` /
//! `ClearField` (`moveuse.cc`); `SeparationEvent` on open quest/level doors.

use std::collections::HashSet;
use std::path::Path;

use mlua::{Lua, Value};
use slotmap::Key;
use tfs_rust_common::Position;
use tfs_rust_common::remere_attr;
use tfs_rust_common::{ScriptContext, ScriptThing};

use crate::creature::CreatureKind;
use crate::event_dispatcher::TileMoveEventItem;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::item_attributes::CustomAttrValue;

const MESSAGE_INFO_DESCR: u8 = 0x16;
const MESSAGE_STATUS_SMALL: u8 = 0x17;
const MESSAGE_EVENT_ADVANCE: u8 = 0x13;

/// Boot-time door/key sprite sets from `data/defs/doors.lua`.
#[derive(Clone, Debug, Default)]
pub struct DoorIdTables {
    pub keys: HashSet<u16>,
    pub open: HashSet<u16>,
    pub closed: HashSet<u16>,
    pub locked: HashSet<u16>,
    pub open_extra: HashSet<u16>,
    pub closed_extra: HashSet<u16>,
    pub open_house: HashSet<u16>,
    pub closed_house: HashSet<u16>,
    pub open_quest: HashSet<u16>,
    pub closed_quest: HashSet<u16>,
    pub open_level: HashSet<u16>,
    pub closed_level: HashSet<u16>,
}

impl DoorIdTables {
    fn is_open_toggle(&self, id: u16) -> bool {
        self.open.contains(&id) || self.open_extra.contains(&id) || self.open_house.contains(&id)
    }

    fn is_closed_toggle(&self, id: u16) -> bool {
        self.closed.contains(&id)
            || self.closed_extra.contains(&id)
            || self.closed_house.contains(&id)
    }

    fn is_key_door(&self, id: u16) -> bool {
        self.open.contains(&id) || self.closed.contains(&id) || self.locked.contains(&id)
    }
}

/// Load `data/defs/doors.lua`. Missing/invalid → empty sets.
pub fn load_from_data_dir(data_dir: &Path) -> DoorIdTables {
    let path = data_dir.join("defs/doors.lua");
    match load_from_file(&path) {
        Ok(tables) => {
            tracing::info!(
                file = %path.display(),
                keys = tables.keys.len(),
                "loaded door id tables"
            );
            tables
        }
        Err(e) => {
            tracing::warn!(file = %path.display(), error = %e, "door id tables not loaded");
            DoorIdTables::default()
        }
    }
}

fn load_from_file(path: &Path) -> Result<DoorIdTables, String> {
    let chunk = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let lua = Lua::new();
    let value: Value = lua
        .load(&chunk)
        .set_name(path.display().to_string())
        .eval()
        .map_err(|e| e.to_string())?;
    let Value::Table(root) = value else {
        return Err("doors.lua must return a table".into());
    };
    Ok(DoorIdTables {
        keys: parse_id_set(&root, "keys")?,
        open: parse_id_set(&root, "open")?,
        closed: parse_id_set(&root, "closed")?,
        locked: parse_id_set(&root, "locked")?,
        open_extra: parse_id_set(&root, "openExtra")?,
        closed_extra: parse_id_set(&root, "closedExtra")?,
        open_house: parse_id_set(&root, "openHouse")?,
        closed_house: parse_id_set(&root, "closedHouse")?,
        open_quest: parse_id_set(&root, "openQuest")?,
        closed_quest: parse_id_set(&root, "closedQuest")?,
        open_level: parse_id_set(&root, "openLevel")?,
        closed_level: parse_id_set(&root, "closedLevel")?,
    })
}

fn parse_id_set(root: &mlua::Table, key: &str) -> Result<HashSet<u16>, String> {
    let value: Value = root.get(key).map_err(|e| e.to_string())?;
    let Value::Table(table) = value else {
        return Ok(HashSet::new());
    };
    let mut out = HashSet::new();
    for pair in table.pairs::<Value, Value>() {
        let (_, v) = pair.map_err(|e| e.to_string())?;
        if let Some(id) = value_as_u16(&v) {
            out.insert(id);
        }
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

/// Pack `doors.lua` onUse. Returns `true` when the use is consumed.
pub fn try_use(
    world: &mut GameWorld,
    cid: CreatureId,
    item_id: ItemId,
    target_item: Option<ItemId>,
    to: Position,
) -> bool {
    let Some(item_type) = world.items.get(item_id).map(|i| i.item_type) else {
        return false;
    };
    if world.door_ids.keys.contains(&item_type) {
        return use_key(world, cid, item_id, target_item, to);
    }
    use_door(world, cid, item_id, item_type, to)
}

fn use_door(
    world: &mut GameWorld,
    cid: CreatureId,
    item_id: ItemId,
    item_type: u16,
    to: Position,
) -> bool {
    if world.door_ids.closed_quest.contains(&item_type) {
        return use_quest_door(world, cid, item_id, item_type, to);
    }
    if world.door_ids.closed_level.contains(&item_type) {
        return use_level_door(world, cid, item_id, item_type, to);
    }
    if world.door_ids.locked.contains(&item_type) {
        send_text(world, cid, MESSAGE_INFO_DESCR, "It is locked.");
        return true;
    }
    if world.door_ids.is_open_toggle(item_type) {
        world.clear_field(item_id, None);
        transform(world, item_id, item_type.saturating_sub(1));
        return true;
    }
    if world.door_ids.is_closed_toggle(item_type) {
        transform(world, item_id, item_type.saturating_add(1));
        return true;
    }
    false
}

fn use_quest_door(
    world: &mut GameWorld,
    cid: CreatureId,
    item_id: ItemId,
    item_type: u16,
    to: Position,
) -> bool {
    let mut quest_value = custom_i32(world, item_id, remere_attr::DOORQUESTVALUE).unwrap_or(0);
    if quest_value == 0 {
        quest_value = -1;
    }
    let quest_number = custom_i32(world, item_id, remere_attr::DOORQUESTNUMBER).unwrap_or(0);
    let stored = world.player_get_storage(cid, quest_number as u32);
    if stored == quest_value || player_has_access(world, cid) {
        transform(world, item_id, item_type.saturating_add(1));
        teleport(world, cid, to, true);
    } else {
        send_text(
            world,
            cid,
            MESSAGE_INFO_DESCR,
            "The door seems to be sealed against unwanted intruders.",
        );
    }
    true
}

fn use_level_door(
    world: &mut GameWorld,
    cid: CreatureId,
    item_id: ItemId,
    item_type: u16,
    to: Position,
) -> bool {
    // Pack: `(has DOORLEVEL and level >= doorlevel) or access`.
    let has_level = custom_i32(world, item_id, remere_attr::DOORLEVEL).is_some();
    let door_level = custom_i32(world, item_id, remere_attr::DOORLEVEL).unwrap_or(0);
    let player_level = player_level(world, cid) as i32;
    let ok = (has_level && player_level >= door_level) || player_has_access(world, cid);
    if ok {
        transform(world, item_id, item_type.saturating_add(1));
        teleport(world, cid, to, true);
    } else {
        send_text(world, cid, MESSAGE_INFO_DESCR, "Only the worthy may pass.");
    }
    true
}

fn use_key(
    world: &mut GameWorld,
    cid: CreatureId,
    key_id: ItemId,
    target_item: Option<ItemId>,
    to: Position,
) -> bool {
    let door_id = match target_item {
        Some(id) => id,
        None => match top_item_at(world, to) {
            Some(id) => id,
            None => return false,
        },
    };
    let Some(door_type) = world.items.get(door_id).map(|i| i.item_type) else {
        return false;
    };
    if world.door_ids.keys.contains(&door_type) {
        return false;
    }
    if !world.door_ids.is_key_door(door_type) {
        return false;
    }
    let Some(hole) = custom_i32(world, door_id, remere_attr::KEYHOLENUMBER) else {
        send_text(world, cid, MESSAGE_STATUS_SMALL, "The key does not match.");
        return true;
    };
    let key_n = custom_i32(world, key_id, remere_attr::KEYNUMBER).unwrap_or(0);
    if key_n != hole {
        send_text(world, cid, MESSAGE_STATUS_SMALL, "The key does not match.");
        return true;
    }
    let mut transform_to = door_type.saturating_add(2);
    if world.door_ids.open.contains(&door_type) {
        transform_to = door_type.saturating_sub(2);
        world.clear_field(door_id, None);
    } else if world.door_ids.closed.contains(&door_type) {
        transform_to = door_type.saturating_sub(1);
    }
    transform(world, door_id, transform_to);
    true
}

/// Close open quest/level doors on StepOut; bounce under-level players on StepIn.
pub fn on_creature_step(
    world: &mut GameWorld,
    cid: CreatureId,
    from: Position,
    step_out_items: &[TileMoveEventItem],
    step_in_items: &[TileMoveEventItem],
) {
    for item in step_out_items {
        if world.door_ids.open_level.contains(&item.item_type)
            || world.door_ids.open_quest.contains(&item.item_type)
        {
            world.clear_field(item.item_id, Some(cid));
            transform(world, item.item_id, item.item_type.saturating_sub(1));
        }
    }
    for item in step_in_items {
        if !world.door_ids.open_level.contains(&item.item_type) {
            continue;
        }
        if !matches!(world.creatures.get(cid), Some(CreatureKind::Player(_))) {
            continue;
        }
        if player_has_access(world, cid) {
            continue;
        }
        let door_level = custom_i32(world, item.item_id, remere_attr::DOORLEVEL).unwrap_or(0);
        if (player_level(world, cid) as i32) < door_level {
            send_text(
                world,
                cid,
                MESSAGE_EVENT_ADVANCE,
                "Only the worthy may pass.",
            );
            teleport(world, cid, from, true);
        }
    }
}

fn top_item_at(world: &GameWorld, pos: Position) -> Option<ItemId> {
    match world.tile_get_top_visible_thing(pos.x, pos.y, pos.z, None) {
        Some(ScriptThing::Item(sid)) => world.resolve_item_u64(sid),
        _ => None,
    }
}

fn custom_i32(world: &GameWorld, item_id: ItemId, key: &str) -> Option<i32> {
    let item = world.items.get(item_id)?;
    let val = item.attributes.as_deref()?.get_custom_attribute(key)?;
    match val {
        CustomAttrValue::Integer(i) => i32::try_from(*i).ok(),
        CustomAttrValue::Float(f) => Some(*f as i32),
        _ => None,
    }
}

fn player_has_access(world: &GameWorld, cid: CreatureId) -> bool {
    let Some(CreatureKind::Player(p)) = world.creatures.get(cid) else {
        return false;
    };
    world
        .groups
        .groups
        .get(&p.group_id)
        .is_some_and(|g| g.access)
}

fn player_level(world: &GameWorld, cid: CreatureId) -> u32 {
    match world.creatures.get(cid) {
        Some(CreatureKind::Player(p)) => p.level.max(0) as u32,
        _ => 0,
    }
}

fn transform(world: &mut GameWorld, item_id: ItemId, new_type: u16) {
    let _ = world.lua_script_item_transform(item_id.data().as_ffi(), new_type, -1);
}

fn teleport(world: &mut GameWorld, cid: CreatureId, dest: Position, push: bool) {
    let _ = world.lua_script_creature_teleport(cid.data().as_ffi(), dest.x, dest.y, dest.z, push);
}

fn send_text(world: &mut GameWorld, cid: CreatureId, class: u8, text: &str) {
    let _ = world.lua_script_player_send_text_message(cid.data().as_ffi(), class, text.into());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Item;
    use crate::sim_harness::{insert_player, minimal_world, pickup_item_type, test_player};
    use std::sync::Arc;

    fn workspace_data() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    fn register_types(world: &mut GameWorld, ids: &[u16]) {
        let db = Arc::get_mut(&mut world.items_db).expect("unique items_db");
        for &id in ids {
            db.items.entry(id).or_insert_with(|| pickup_item_type(id));
        }
    }

    #[test]
    fn load_door_ids_from_defs() {
        let tables = load_from_data_dir(&workspace_data());
        assert!(tables.keys.contains(&2091));
        assert!(tables.locked.contains(&1212));
        assert!(tables.closed.contains(&1210));
        assert!(tables.open.contains(&1211));
        assert!(tables.closed_quest.contains(&1223));
        assert!(tables.open_level.contains(&1228));
    }

    #[test]
    fn locked_door_use_is_consumed() {
        let mut world = minimal_world();
        register_types(&mut world, &[1212]);
        world.door_ids.locked.insert(1212);
        let iid = world.items.insert(Item::new_single(1212));
        let cid = insert_player(&mut world, test_player("Lock", Position::new(50, 50, 7)));
        assert!(try_use(
            &mut world,
            cid,
            iid,
            None,
            Position::new(50, 50, 7)
        ));
        assert_eq!(world.items.get(iid).unwrap().item_type, 1212);
    }

    #[test]
    fn closed_door_transforms_open() {
        let mut world = minimal_world();
        register_types(&mut world, &[1210, 1211]);
        world.door_ids.closed.insert(1210);
        world.door_ids.open.insert(1211);
        let iid = world.items.insert(Item::new_single(1210));
        let cid = insert_player(&mut world, test_player("Open", Position::new(50, 50, 7)));
        assert!(try_use(
            &mut world,
            cid,
            iid,
            None,
            Position::new(50, 50, 7)
        ));
        assert_eq!(world.items.get(iid).unwrap().item_type, 1211);
    }

    #[test]
    fn matching_key_unlocks_locked_door() {
        let mut world = minimal_world();
        register_types(&mut world, &[2091, 1212, 1214]);
        world.door_ids.keys.insert(2091);
        world.door_ids.locked.insert(1212);
        world.door_ids.open.insert(1214);
        let mut key = Item::new_single(2091);
        key.attributes
            .get_or_insert_with(|| Box::new(crate::item_attributes::ItemAttributes::new()))
            .set_custom_attribute(remere_attr::KEYNUMBER, CustomAttrValue::Integer(42));
        let key_id = world.items.insert(key);
        let mut door = Item::new_single(1212);
        door.attributes
            .get_or_insert_with(|| Box::new(crate::item_attributes::ItemAttributes::new()))
            .set_custom_attribute(remere_attr::KEYHOLENUMBER, CustomAttrValue::Integer(42));
        let door_id = world.items.insert(door);
        let cid = insert_player(&mut world, test_player("Key", Position::new(50, 50, 7)));
        assert!(try_use(
            &mut world,
            cid,
            key_id,
            Some(door_id),
            Position::new(50, 51, 7)
        ));
        assert_eq!(world.items.get(door_id).unwrap().item_type, 1214);
    }
}
