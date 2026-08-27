//! Native pack tool helpers — `onUseQuest` / `destroyItem` / `onUse*` / `checkScarabTile`.
//!
//! Pack: `data/scripts/functions.lua`. ID tables: `data/defs/tools.lua` /
//! `data/defs/scarab_tiles.lua`.
//!
//! C++ reference: TFS pack `functions.lua`; corpus `moveuse.cc` `UseWeapon` /
//! `UseChest` / `moveuse.dat` Digging/Cutting/Roping/Fun.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use mlua::{Lua, Value};
use rand::RngExt;
use slotmap::Key;
use tfs_rust_common::Position;
use tfs_rust_common::ScriptContext;
use tfs_rust_common::remere_attr;
use tfs_rust_lua::{
    LuaMoveDestination, QuestRewardSpec, ToolUseKind, ToolUseRequest,
};

use crate::game_world::GameWorld;
use crate::ids::ItemId;
use crate::item_attributes::CustomAttrValue;
use crate::player_flags::PLAYER_FLAG_HAS_INFINITE_CAPACITY;
use crate::return_value::ReturnValue;

const CONTAINER_POSITION: u16 = 0xFFFF; // TFS CONTAINER_POSITION
const CONST_ME_POFF: u8 = 3; // const.h CONST_ME_POFF
const MESSAGE_INFO_DESCR: u8 = 0x16;
const TILESTATE_PROTECTIONZONE: i32 = 1 << 7;

/// Boot-time tool / field / rope id tables from `data/defs/tools.lua`.
#[derive(Clone, Debug, Default)]
pub struct ToolIdTables {
    pub action_ids: HashMap<String, u16>,
    /// Named single ids (`ids.pumpkin`, `ids.wheatMature`, …).
    pub named_ids: HashMap<String, u16>,
    pub jungle_grass: HashMap<u16, u16>,
    pub pick_grounds: HashSet<u16>,
    pub sand_ids: HashSet<u16>,
    pub holes: HashSet<u16>,
    pub hole_id: HashSet<u16>,
    pub rope_spots: HashSet<u16>,
    pub fields: HashSet<u16>,
    pub corpse_ids: HashSet<u16>,
    pub scarab_tiles: HashSet<(u16, u16, u8)>,
    pub scarab_monster: String,
    pub scarab_timer_secs: i64,
    pub scarab_spawn_chance: u32,
    pub sand_hole_chance: u32,
}

impl ToolIdTables {
    fn named(&self, key: &str) -> Option<u16> {
        self.named_ids.get(key).copied()
    }

    fn pick_hole_aid(&self) -> Option<u16> {
        self.action_ids.get("pickHole").copied()
    }

    fn sand_hole_aid(&self) -> Option<u16> {
        self.action_ids.get("sandHole").copied()
    }
}

/// Load `data/defs/tools.lua` + `scarab_tiles.lua`. Missing/invalid → empty.
pub fn load_from_data_dir(data_dir: &Path) -> ToolIdTables {
    let path = data_dir.join("defs/tools.lua");
    let mut tables = match load_tools_file(&path) {
        Ok(t) => {
            tracing::info!(file = %path.display(), rope = t.rope_spots.len(), "loaded tool id tables");
            t
        }
        Err(e) => {
            tracing::warn!(file = %path.display(), error = %e, "tool id tables not loaded");
            ToolIdTables::default()
        }
    };
    let scarab_path = data_dir.join("defs/scarab_tiles.lua");
    match load_scarab_file(&scarab_path) {
        Ok(set) => {
            tracing::info!(file = %scarab_path.display(), n = set.len(), "loaded scarab tiles");
            tables.scarab_tiles = set;
        }
        Err(e) => {
            tracing::warn!(file = %scarab_path.display(), error = %e, "scarab tiles not loaded");
        }
    }
    tables
}

fn load_tools_file(path: &Path) -> Result<ToolIdTables, String> {
    let chunk = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let lua = Lua::new();
    let value: Value = lua
        .load(&chunk)
        .set_name(path.display().to_string())
        .eval()
        .map_err(|e| e.to_string())?;
    let Value::Table(root) = value else {
        return Err("tools.lua must return a table".into());
    };
    let scarab: Value = root.get("scarab").unwrap_or(Value::Nil);
    let chances: Value = root.get("chances").unwrap_or(Value::Nil);
    Ok(ToolIdTables {
        action_ids: parse_named_u16(&root, "actionIds")?,
        named_ids: parse_named_u16(&root, "ids")?,
        jungle_grass: parse_id_map(&root, "jungleGrass")?,
        pick_grounds: parse_id_set(&root, "pickGrounds")?,
        sand_ids: parse_id_set(&root, "sandIds")?,
        holes: parse_id_set(&root, "holes")?,
        hole_id: parse_id_set(&root, "holeId")?,
        rope_spots: parse_id_set(&root, "ropeSpots")?,
        fields: parse_id_set(&root, "Fields")?,
        corpse_ids: parse_id_set(&root, "corpseIds")?,
        scarab_tiles: HashSet::new(),
        scarab_monster: table_string(&scarab, "monster").unwrap_or_default(),
        scarab_timer_secs: table_i64(&scarab, "timerSecs").unwrap_or(0),
        scarab_spawn_chance: table_u32(&scarab, "spawnChance").unwrap_or(0),
        sand_hole_chance: table_u32(&chances, "sandHole").unwrap_or(0),
    })
}

fn load_scarab_file(path: &Path) -> Result<HashSet<(u16, u16, u8)>, String> {
    let chunk = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let lua = Lua::new();
    let value: Value = lua
        .load(&chunk)
        .set_name(path.display().to_string())
        .eval()
        .map_err(|e| e.to_string())?;
    let Value::Table(root) = value else {
        return Err("scarab_tiles.lua must return a table".into());
    };
    let list: Value = root.get("positions").map_err(|e| e.to_string())?;
    let Value::Table(table) = list else {
        return Ok(HashSet::new());
    };
    let mut out = HashSet::new();
    for pair in table.pairs::<Value, Value>() {
        let (_, v) = pair.map_err(|e| e.to_string())?;
        let Value::Table(row) = v else {
            continue;
        };
        let Some(x) = value_as_u16(&row.get("x").unwrap_or(Value::Nil)) else {
            continue;
        };
        let Some(y) = value_as_u16(&row.get("y").unwrap_or(Value::Nil)) else {
            continue;
        };
        let Some(z) = value_as_u16(&row.get("z").unwrap_or(Value::Nil)) else {
            continue;
        };
        out.insert((x, y, z as u8));
    }
    Ok(out)
}

fn parse_named_u16(root: &mlua::Table, key: &str) -> Result<HashMap<String, u16>, String> {
    let value: Value = root.get(key).map_err(|e| e.to_string())?;
    let Value::Table(table) = value else {
        return Ok(HashMap::new());
    };
    let mut out = HashMap::new();
    for pair in table.pairs::<Value, Value>() {
        let (k, v) = pair.map_err(|e| e.to_string())?;
        let name = match k {
            Value::String(s) => s.to_str().map_err(|e| e.to_string())?.to_string(),
            _ => continue,
        };
        if let Some(id) = value_as_u16(&v) {
            out.insert(name, id);
        }
    }
    Ok(out)
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

fn table_string(v: &Value, key: &str) -> Option<String> {
    let Value::Table(t) = v else {
        return None;
    };
    match t.get(key).ok()? {
        Value::String(s) => s.to_str().ok().map(|s| s.to_string()),
        _ => None,
    }
}

fn table_i64(v: &Value, key: &str) -> Option<i64> {
    let Value::Table(t) = v else {
        return None;
    };
    match t.get(key).ok()? {
        Value::Integer(i) => Some(i),
        Value::Number(n) => Some(n as i64),
        _ => None,
    }
}

fn table_u32(v: &Value, key: &str) -> Option<u32> {
    table_i64(v, key).and_then(|i| u32::try_from(i).ok())
}

#[cfg(test)]
thread_local! {
    static FORCE_DICE_1_3: std::cell::Cell<Option<u32>> = const { std::cell::Cell::new(None) };
}

fn dice_1_to_3() -> u32 {
    #[cfg(test)]
    if let Some(v) = FORCE_DICE_1_3.with(|c| c.get()) {
        return v;
    }
    rand::rng().random_range(1..=3)
}

fn dice_1_to_100() -> u32 {
    rand::rng().random_range(1..=100)
}

fn unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Apply a native tool-use helper. Returns the Lua boolean.
pub fn apply(world: &mut GameWorld, req: &ToolUseRequest) -> bool {
    match req.kind {
        ToolUseKind::DestroyItem => destroy_item(world, req),
        ToolUseKind::Machete => on_use_machete(world, req),
        ToolUseKind::Pick => on_use_pick(world, req),
        ToolUseKind::Knife => on_use_knife(world, req),
        ToolUseKind::Rope => on_use_rope(world, req),
        ToolUseKind::Shovel => on_use_shovel(world, req),
        ToolUseKind::Scythe => on_use_scythe(world, req),
        ToolUseKind::Quest => on_use_quest(world, req),
        ToolUseKind::CheckScarab => {
            check_scarab_tile(world, Position::new(req.to.0, req.to.1, req.to.2));
            true
        }
    }
}

fn to_pos(req: &ToolUseRequest) -> Position {
    Position::new(req.to.0, req.to.1, req.to.2)
}

fn destroy_item(world: &mut GameWorld, req: &ToolUseRequest) -> bool {
    if !req.target_is_item_userdata {
        return false;
    }
    let Some(tid) = req.target_item else {
        return false;
    };
    let Some(iid) = world.resolve_item_u64(tid) else {
        return false;
    };
    let Some(item) = world.items.get(iid) else {
        return false;
    };
    if item.unique_id() != 0 || item.action_id() != 0 {
        return false;
    }
    if req.to.0 == CONTAINER_POSITION {
        let _ = world.lua_script_player_send_cancel_message(
            req.player,
            ReturnValue::NotPossible.description().to_string(),
        );
        return true;
    }
    let item_type = item.item_type;
    let destroy_id = world
        .items_db
        .items
        .get(&item_type)
        .map(|t| t.destroy_to)
        .unwrap_or(0);
    if destroy_id == 0 {
        return false;
    }
    world.broadcast_magic_effect(to_pos(req), CONST_ME_POFF);
    if dice_1_to_3() == 1 {
        dump_container_to_tile(world, iid, to_pos(req));
        let _ = world.lua_script_item_transform(tid, destroy_id, -1);
    }
    true
}

fn dump_container_to_tile(world: &mut GameWorld, iid: ItemId, dest: Position) {
    if !world
        .items
        .get(iid)
        .is_some_and(|i| world.items_db.is_container(i.item_type))
    {
        return;
    }
    world.hydrate_container_if_needed(iid);
    let kids: Vec<u64> = world
        .container_registry
        .get(iid)
        .map(|c| {
            (0..c.size())
                .rev()
                .filter_map(|i| c.get_item(i).map(|id| id.data().as_ffi()))
                .collect()
        })
        .unwrap_or_default();
    for child in kids {
        let _ = world.lua_script_item_move_to(
            child,
            LuaMoveDestination::Tile {
                x: dest.x,
                y: dest.y,
                z: dest.z,
            },
            0,
        );
    }
}

fn on_use_machete(world: &mut GameWorld, req: &ToolUseRequest) -> bool {
    let Some(target_id) = req.target_itemid else {
        return true;
    };
    if world.tool_ids.named("rushWood") == Some(target_id) {
        world.broadcast_magic_effect(to_pos(req), CONST_ME_POFF);
        if let Some(tid) = req.target_item {
            let _ = world.lua_script_item_remove(tid, -1);
        }
        return true;
    }
    if let Some(&grass) = world.tool_ids.jungle_grass.get(&target_id)
        && let Some(tid) = req.target_item
    {
        let _ = world.lua_script_item_transform(tid, grass, -1);
        if let Some(iid) = world.resolve_item_u64(tid) {
            world.start_decay(iid);
        }
        return true;
    }
    destroy_item(world, req)
}

fn on_use_pick(world: &mut GameWorld, req: &ToolUseRequest) -> bool {
    let pos = to_pos(req);
    let Some(tile) = world.map.get_tile(pos) else {
        return false;
    };
    let Some(ground_id) = tile.body().ground_item else {
        return false;
    };
    let Some(ground) = world.items.get(ground_id) else {
        return false;
    };
    let gtype = ground.item_type;
    let aid = ground.action_id();
    if world.tool_ids.pick_grounds.contains(&gtype)
        && world.tool_ids.pick_hole_aid() == Some(aid)
        && let Some(open_id) = world.tool_ids.named("pickHoleOpen")
    {
        let _ = world.lua_script_item_transform(ground_id.data().as_ffi(), open_id, -1);
        world.start_decay(ground_id);
        let dest = Position::new(pos.x, pos.y, pos.z.saturating_add(1));
        let _ = tile_relocate_to(world, pos, dest);
        return true;
    }
    false
}

fn on_use_knife(world: &mut GameWorld, req: &ToolUseRequest) -> bool {
    if !req.target_is_item_userdata {
        return false;
    }
    let Some(tid) = req.target_item else {
        return false;
    };
    let pumpkin = world.tool_ids.named("pumpkin");
    let head = world.tool_ids.named("pumpkinhead");
    if pumpkin.is_none() || req.target_itemid != pumpkin {
        return false;
    }
    let Some(head) = head else {
        return false;
    };
    let _ = world.lua_script_item_transform(tid, head, 1);
    if let Some(iid) = world.resolve_item_u64(tid) {
        world.start_decay(iid);
    }
    true
}

fn on_use_rope(world: &mut GameWorld, req: &ToolUseRequest) -> bool {
    let pos = to_pos(req);
    let Some(tile) = world.map.get_tile(pos) else {
        return false;
    };
    let ground_type = tile.body().ground;
    if let Some(gt) = ground_type
        && world.tool_ids.rope_spots.contains(&gt)
    {
        let dest = move_upstairs(world, pos);
        if world.map.get_tile(dest).is_none() {
            return false;
        }
        if world.tile_has_flag(dest.x, dest.y, dest.z, TILESTATE_PROTECTIONZONE)
            && world.player_is_pz_locked(req.player) == Some(true)
        {
            let _ = world.lua_script_player_send_cancel_message(
                req.player,
                ReturnValue::PlayerIsPzLocked.description().to_string(),
            );
            return true;
        }
        let _ = world.lua_script_creature_teleport(req.player, dest.x, dest.y, dest.z, false);
        return true;
    }
    let Some(target_id) = req.target_itemid else {
        return false;
    };
    if !world.tool_ids.hole_id.contains(&target_id) {
        return false;
    }
    let below = Position::new(pos.x, pos.y, pos.z.saturating_add(1));
    let Some(_) = world.map.get_tile(below) else {
        return false;
    };
    let thing_creature = world.tile_get_bottom_creature(below.x, below.y, below.z);
    let thing_item = world.tile_get_top_down_item(below.x, below.y, below.z);
    if thing_creature.is_none() && thing_item.is_none() {
        return true;
    }
    let up = move_upstairs(world, below);
    if let Some(cid) = thing_creature {
        let _ = world.lua_script_creature_teleport(cid, up.x, up.y, up.z, false);
        return true;
    }
    if let Some(iid) = thing_item {
        let movable = world
            .resolve_item_u64(iid)
            .and_then(|id| world.items.get(id))
            .and_then(|i| world.items_db.items.get(&i.item_type))
            .is_some_and(|t| t.moveable());
        if movable {
            let _ = world.lua_script_item_move_to(
                iid,
                LuaMoveDestination::Tile {
                    x: up.x,
                    y: up.y,
                    z: up.z,
                },
                0,
            );
        }
    }
    true
}

fn on_use_shovel(world: &mut GameWorld, req: &ToolUseRequest) -> bool {
    let pos = to_pos(req);
    let Some(tile) = world.map.get_tile(pos) else {
        return false;
    };
    let Some(ground_id) = tile.body().ground_item else {
        return false;
    };
    let Some(ground) = world.items.get(ground_id) else {
        return false;
    };
    let ground_type = ground.item_type;
    if world.tool_ids.holes.contains(&ground_type) {
        let _ = world.lua_script_item_transform(
            ground_id.data().as_ffi(),
            ground_type.saturating_add(1),
            -1,
        );
        world.start_decay(ground_id);
        let dest = Position::new(pos.x, pos.y, pos.z.saturating_add(1));
        let _ = tile_relocate_to(world, pos, dest);
        return true;
    }
    if world.tool_ids.sand_ids.contains(&ground_type) {
        let roll = dice_1_to_100();
        if world.tool_ids.sand_hole_aid() == Some(req.target_actionid)
            && roll <= world.tool_ids.sand_hole_chance
            && let Some(open_id) = world.tool_ids.named("sandHoleOpen")
        {
            let _ = world.lua_script_item_transform(ground_id.data().as_ffi(), open_id, -1);
            world.start_decay(ground_id);
        } else {
            check_scarab_tile(world, pos);
        }
        world.broadcast_magic_effect(pos, CONST_ME_POFF);
        return true;
    }
    false
}

fn on_use_scythe(world: &mut GameWorld, req: &ToolUseRequest) -> bool {
    let mature = world.tool_ids.named("wheatMature");
    let cut = world.tool_ids.named("wheatCut");
    let bunch = world.tool_ids.named("wheatBunch");
    let growing = world.tool_ids.named("wheatGrowing");
    if mature.is_some() && req.target_itemid == mature {
        if let Some(tid) = req.target_item
            && let Some(cut) = cut
        {
            let _ = world.lua_script_item_transform(tid, cut, -1);
            if let Some(iid) = world.resolve_item_u64(tid) {
                world.start_decay(iid);
            }
        }
        if let Some(bunch) = bunch {
            let pos = to_pos(req);
            let _ = world.lua_script_game_create_item(bunch, 1, Some((pos.x, pos.y, pos.z)));
        }
        return true;
    }
    if growing.is_some() && req.target_itemid == growing {
        let _ = world.lua_script_player_send_cancel_message(
            req.player,
            "It's not mature yet.".to_string(),
        );
        return true;
    }
    destroy_item(world, req)
}

fn check_scarab_tile(world: &mut GameWorld, pos: Position) {
    if !world.tool_ids.scarab_tiles.contains(&(pos.x, pos.y, pos.z)) {
        return;
    }
    let Some(tile) = world.map.get_tile(pos) else {
        return;
    };
    let Some(ground_id) = tile.body().ground_item else {
        return;
    };
    let timer = world
        .items
        .get(ground_id)
        .and_then(|i| i.attributes.as_ref())
        .and_then(|a| a.get_custom_attribute("scarabtiletimer"))
        .and_then(|v| match v {
            CustomAttrValue::Integer(n) => Some(*n),
            _ => None,
        })
        .unwrap_or(0);
    let now = unix_secs();
    if now <= timer {
        return;
    }
    if dice_1_to_100() <= world.tool_ids.scarab_spawn_chance {
        let name = world.tool_ids.scarab_monster.clone();
        if !name.is_empty() {
            let _ = world.lua_script_create_monster(&name, pos.x, pos.y, pos.z, false, false);
        }
    } else if let Some(coin) = world.tool_ids.named("scarabCoin") {
        let _ = world.lua_script_game_create_item(coin, 1, Some((pos.x, pos.y, pos.z)));
    }
    let timer_secs = world.tool_ids.scarab_timer_secs;
    if let Some(item) = world.items.get_mut(ground_id) {
        item.attributes
            .get_or_insert_with(|| Box::new(crate::item_attributes::ItemAttributes::new()))
            .set_custom_attribute(
                "scarabtiletimer",
                CustomAttrValue::Integer(now + timer_secs),
            );
    }
}

fn on_use_quest(world: &mut GameWorld, req: &ToolUseRequest) -> bool {
    let Some(chest) = req.quest.as_ref() else {
        return false;
    };
    let Some(cid) = world.resolve_creature_u64(req.player) else {
        return false;
    };
    if world.player_get_storage(cid, chest.storage_value) != -1 {
        let name = chest_item_name(world, req.item);
        let _ = world.lua_script_player_send_text_message(
            req.player,
            MESSAGE_INFO_DESCR,
            format!("The {name} is empty."),
        );
        return true;
    }
    let main_id = chest.item.id;
    let Some(main_ty) = world.items_db.items.get(&main_id).cloned() else {
        return false;
    };
    let count_for_name = chest.item.count.unwrap_or(0);
    let mut reward_name = if main_ty.stackable() && count_for_name > 1 {
        format!("{count_for_name} {}", main_ty.get_plural_name())
    } else {
        main_ty.name.clone()
    };
    if !main_ty.article.is_empty() && count_for_name.max(1) <= 1 {
        reward_name = format!("{} {reward_name}", main_ty.article);
    }
    let mut reward_weight = type_weight(&main_ty, chest.item.count);
    for extra in &chest.content {
        if let Some(ty) = world.items_db.items.get(&extra.id) {
            reward_weight = reward_weight.saturating_add(type_weight(ty, extra.count));
        }
    }
    let term = if main_ty.stackable() && count_for_name > 1 {
        "they are"
    } else {
        "it is"
    };
    let no_cap = format!(
        "You have found {reward_name}. Weighing {}.{:02} oz {term} too heavy.",
        reward_weight / 100,
        reward_weight % 100
    );
    let infinite = world.player_has_flag(cid, PLAYER_FLAG_HAS_INFINITE_CAPACITY);
    let free = world.player_free_capacity_u32(cid).unwrap_or(0);
    if reward_weight > free && !infinite {
        let _ = world.lua_script_player_send_text_message(req.player, MESSAGE_INFO_DESCR, no_cap);
        return true;
    }
    let create_count = chest
        .item
        .count
        .or(chest.item.subtype)
        .or(chest.item.charges)
        .unwrap_or(1);
    let Some(reward_u64) = world
        .lua_script_game_create_item(main_id, create_count, None)
        .ok()
        .flatten()
    else {
        return false;
    };
    apply_reward_attrs(world, reward_u64, &chest.item);
    if !chest.content.is_empty() && main_ty.is_container() {
        for extra in &chest.content {
            let n = extra.count.or(extra.subtype).or(extra.charges).unwrap_or(1);
            if let Ok(Some(child)) =
                world.lua_script_container_add_item(reward_u64, extra.id, u32::from(n), -1, 0)
            {
                apply_reward_attrs(world, child, extra);
            }
        }
    }
    let actual_weight = world
        .resolve_item_u64(reward_u64)
        .and_then(|id| {
            world.items.get(id).map(|item| {
                let tw = world
                    .items_db
                    .items
                    .get(&item.item_type)
                    .map(|t| t.weight)
                    .unwrap_or(0);
                let stack = world
                    .items_db
                    .items
                    .get(&item.item_type)
                    .is_some_and(|t| t.stackable());
                item.total_weight_oz(tw, stack)
            })
        })
        .unwrap_or(reward_weight);
    let free2 = world.player_free_capacity_u32(cid).unwrap_or(0);
    if free2 >= actual_weight {
        let rv = world
            .lua_script_add_item_ex(
                reward_u64,
                LuaMoveDestination::Player {
                    creature_id: req.player,
                },
                false,
                -1,
                0,
            )
            .unwrap_or(ReturnValue::NotPossible as i32);
        if rv == ReturnValue::NoError as i32 {
            let _ = world.lua_script_player_send_text_message(
                req.player,
                MESSAGE_INFO_DESCR,
                format!("You have found {reward_name}."),
            );
            let _ = world.player_set_storage(cid, chest.storage_value, 1);
        } else {
            let _ = world.lua_script_player_send_text_message(
                req.player,
                MESSAGE_INFO_DESCR,
                format!("You have found {reward_name}, but you have no room to take it."),
            );
            remove_item_any(world, reward_u64);
        }
    } else {
        let _ = world.lua_script_player_send_text_message(req.player, MESSAGE_INFO_DESCR, no_cap);
        remove_item_any(world, reward_u64);
    }
    true
}

fn type_weight(ty: &tfs_rust_content::otb::ItemType, count: Option<u16>) -> u32 {
    let n = u32::from(count.unwrap_or(1).max(1));
    ty.weight.saturating_mul(n)
}

fn remove_item_any(world: &mut GameWorld, item_u64: u64) {
    if let Some(iid) = world.resolve_item_u64(item_u64)
        && world.items.get(iid).and_then(|i| i.parent).is_none()
    {
        world.items.remove(iid);
        return;
    }
    let _ = world.lua_script_item_remove(item_u64, -1);
}

fn chest_item_name(world: &GameWorld, item: Option<u64>) -> String {
    item.and_then(|u| world.resolve_item_u64(u))
        .and_then(|id| world.items.get(id))
        .and_then(|i| {
            world
                .items_db
                .items
                .get(&i.item_type)
                .map(|t| t.name.clone())
        })
        .unwrap_or_default()
}

fn apply_reward_attrs(world: &mut GameWorld, item_u64: u64, spec: &QuestRewardSpec) {
    if let Some(text) = &spec.text
        && let Some(iid) = world.resolve_item_u64(item_u64)
        && let Some(item) = world.items.get_mut(iid)
    {
        item.set_text(text.clone());
    }
    if let Some(key) = spec.keynumber {
        let _ = world.lua_script_set_custom_attribute(
            item_u64,
            remere_attr::KEYNUMBER.to_string(),
            key,
        );
    }
}

/// `Tile.relocateTo` — `data/lib/core/tile.lua`.
fn tile_relocate_to(world: &mut GameWorld, from: Position, to: Position) -> bool {
    if from == to || world.map.get_tile(to).is_none() {
        return false;
    }
    let count = world.tile_get_thing_count(from.x, from.y, from.z);
    let mut things = Vec::new();
    for i in (0..count).rev() {
        if let Some(th) = world.tile_get_thing(from.x, from.y, from.z, i) {
            things.push(th);
        }
    }
    for th in things {
        match th {
            tfs_rust_common::ScriptThing::Item(id) => {
                let Some(iid) = world.resolve_item_u64(id) else {
                    continue;
                };
                let Some(item) = world.items.get(iid) else {
                    continue;
                };
                let item_type = item.item_type;
                let fluid = world
                    .items_db
                    .items
                    .get(&item_type)
                    .map(|t| {
                        if t.is_fluid_container() || t.is_splash() {
                            item.get_sub_type(t)
                        } else {
                            item.fluid_type()
                        }
                    })
                    .unwrap_or(item.fluid_type());
                if fluid != 0 {
                    let _ = world.lua_script_item_remove(id, -1);
                } else if world
                    .items_db
                    .items
                    .get(&item_type)
                    .is_some_and(|t| t.moveable())
                {
                    let _ = world.lua_script_item_move_to(
                        id,
                        LuaMoveDestination::Tile {
                            x: to.x,
                            y: to.y,
                            z: to.z,
                        },
                        0,
                    );
                }
            }
            tfs_rust_common::ScriptThing::Creature(id) => {
                let _ = world.lua_script_creature_teleport(id, to.x, to.y, to.z, false);
            }
        }
    }
    true
}

/// `Position:moveUpstairs` — `data/lib/core/position.lua`.
fn move_upstairs(world: &GameWorld, pos: Position) -> Position {
    if pos.z == 0 {
        return pos;
    }
    let z = pos.z - 1;
    let south = offset(pos.x, pos.y, 0, 1);
    if tile_walkable_lua(world, south.0, south.1, z) {
        return Position::new(south.0, south.1, z);
    }
    // NORTH..NORTHEAST; SOUTH iteration tries WEST (pack loop).
    const DIRS: [(i32, i32); 8] = [
        (0, -1),
        (1, 0),
        (-1, 0), // SOUTH → WEST
        (-1, 0),
        (-1, 1),
        (1, 1),
        (-1, -1),
        (1, -1),
    ];
    for (dx, dy) in DIRS {
        let (x, y) = offset(pos.x, pos.y, dx, dy);
        if tile_walkable_lua(world, x, y, z) {
            return Position::new(x, y, z);
        }
    }
    Position::new(south.0, south.1, z)
}

fn offset(x: u16, y: u16, dx: i32, dy: i32) -> (u16, u16) {
    ((x as i32 + dx) as u16, (y as i32 + dy) as u16)
}

/// `Tile.isWalkable` — `data/lib/core/tile.lua`.
fn tile_walkable_lua(world: &GameWorld, x: u16, y: u16, z: u8) -> bool {
    let pos = Position { x, y, z };
    let Some(tile) = world.map.get_tile(pos) else {
        return false;
    };
    let body = tile.body();
    let Some(gt) = body.ground else {
        return false;
    };
    if world
        .items_db
        .items
        .get(&gt)
        .is_some_and(|t| t.block_solid())
    {
        return false;
    }
    for &iid in body.top_items.iter().chain(body.down_items.iter()) {
        let Some(item) = world.items.get(iid) else {
            continue;
        };
        let Some(ty) = world.items_db.items.get(&item.item_type) else {
            continue;
        };
        if ty.is_magic_field() {
            continue;
        }
        if ty.moveable() {
            continue;
        }
        if ty.block_solid() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cylinder::Cylinder;
    use crate::ids::CreatureId;
    use crate::item::Item;
    use crate::sim_harness::{insert_player, minimal_world, pickup_item_type, test_player};
    use crate::tile::{Tile, TileBody};
    use std::sync::Arc;
    use tfs_rust_common::enums::ZoneType;

    fn workspace_data() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    fn load_pack_ids(world: &mut GameWorld) {
        world.tool_ids = load_from_data_dir(&workspace_data());
    }

    fn named(world: &GameWorld, key: &str) -> u16 {
        world
            .tool_ids
            .named(key)
            .unwrap_or_else(|| panic!("defs missing ids.{key}"))
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

    fn req_destroy(
        player: CreatureId,
        target: ItemId,
        to: Position,
        is_ud: bool,
    ) -> ToolUseRequest {
        ToolUseRequest {
            kind: ToolUseKind::DestroyItem,
            player: player.data().as_ffi(),
            item: None,
            target_item: Some(target.data().as_ffi()),
            target_creature: None,
            target_is_item_userdata: is_ud,
            target_itemid: worldless_type(),
            target_actionid: 0,
            from: (0, 0, 0),
            to: (to.x, to.y, to.z),
            quest: None,
        }
    }

    fn worldless_type() -> Option<u16> {
        None
    }

    #[test]
    fn load_tools_lua_ids() {
        let tables = load_from_data_dir(&workspace_data());
        assert_eq!(tables.action_ids.get("pickHole").copied(), Some(4003));
        assert_eq!(tables.action_ids.get("sandHole").copied(), Some(4002));
        assert!(tables.pick_grounds.contains(&354));
        assert!(tables.pick_grounds.contains(&355));
        assert!(tables.holes.contains(&468));
        assert_eq!(tables.jungle_grass.get(&2782), Some(&2781));
        assert!(tables.rope_spots.contains(&384));
        assert!(tables.rope_spots.contains(&418));
        assert!(tables.named_ids.contains_key("pumpkin"));
        assert!(tables.named_ids.contains_key("wheatMature"));
        assert!(!tables.scarab_monster.is_empty());
        assert!(tables.sand_hole_chance > 0);
    }

    #[test]
    fn defs_rope_spots_384_418() {
        let tables = load_from_data_dir(&workspace_data());
        assert_eq!(tables.rope_spots.len(), 2);
        assert!(tables.rope_spots.contains(&384));
        assert!(tables.rope_spots.contains(&418));
    }

    #[test]
    fn destroy_item_unique_skip() {
        let mut world = minimal_world();
        register_types(&mut world, &[1442, 2256]);
        if let Some(db) = Arc::get_mut(&mut world.items_db)
            && let Some(t) = db.items.get_mut(&1442)
        {
            t.destroy_to = 2256;
        }
        let mut item = Item::new_single(1442);
        item.set_unique_id(1);
        let iid = world.items.insert(item);
        let cid = insert_player(&mut world, test_player("D", Position::new(50, 50, 7)));
        FORCE_DICE_1_3.with(|c| c.set(Some(1)));
        let mut req = req_destroy(cid, iid, Position::new(50, 50, 7), true);
        req.target_itemid = Some(1442);
        assert!(!apply(&mut world, &req));
        assert_eq!(world.items.get(iid).unwrap().item_type, 1442);
        FORCE_DICE_1_3.with(|c| c.set(None));
    }

    #[test]
    fn destroy_item_container_position() {
        let mut world = minimal_world();
        register_types(&mut world, &[1442, 2256]);
        if let Some(db) = Arc::get_mut(&mut world.items_db)
            && let Some(t) = db.items.get_mut(&1442)
        {
            t.destroy_to = 2256;
        }
        let iid = world.items.insert(Item::new_single(1442));
        let cid = insert_player(&mut world, test_player("C", Position::new(50, 50, 7)));
        FORCE_DICE_1_3.with(|c| c.set(Some(1)));
        let mut req = req_destroy(cid, iid, Position::new(CONTAINER_POSITION, 0, 0), true);
        req.target_itemid = Some(1442);
        assert!(apply(&mut world, &req));
        assert_eq!(world.items.get(iid).unwrap().item_type, 1442);
        FORCE_DICE_1_3.with(|c| c.set(None));
    }

    #[test]
    fn destroy_item_one_in_three_transforms() {
        let mut world = minimal_world();
        register_types(&mut world, &[1442, 2256]);
        if let Some(db) = Arc::get_mut(&mut world.items_db)
            && let Some(t) = db.items.get_mut(&1442)
        {
            t.destroy_to = 2256;
        }
        let iid = world.items.insert(Item::new_single(1442));
        let cid = insert_player(&mut world, test_player("T", Position::new(50, 50, 7)));
        FORCE_DICE_1_3.with(|c| c.set(Some(1)));
        let mut req = req_destroy(cid, iid, Position::new(50, 50, 7), true);
        req.target_itemid = Some(1442);
        assert!(apply(&mut world, &req));
        assert_eq!(world.items.get(iid).unwrap().item_type, 2256);
        FORCE_DICE_1_3.with(|c| c.set(None));
    }

    #[test]
    fn destroy_item_not_userdata_false() {
        let mut world = minimal_world();
        let cid = insert_player(&mut world, test_player("Z", Position::new(50, 50, 7)));
        let req = ToolUseRequest {
            kind: ToolUseKind::DestroyItem,
            player: cid.data().as_ffi(),
            item: None,
            target_item: None,
            target_creature: None,
            target_is_item_userdata: false,
            target_itemid: Some(0),
            target_actionid: 0,
            from: (0, 0, 0),
            to: (50, 50, 7),
            quest: None,
        };
        assert!(!apply(&mut world, &req));
    }

    #[test]
    fn knife_pumpkin_to_head() {
        let mut world = minimal_world();
        load_pack_ids(&mut world);
        let pumpkin = named(&world, "pumpkin");
        let head = named(&world, "pumpkinhead");
        register_types(&mut world, &[pumpkin, head]);
        let iid = world.items.insert(Item::new_single(pumpkin));
        let cid = insert_player(&mut world, test_player("K", Position::new(50, 50, 7)));
        let req = ToolUseRequest {
            kind: ToolUseKind::Knife,
            player: cid.data().as_ffi(),
            item: None,
            target_item: Some(iid.data().as_ffi()),
            target_creature: None,
            target_is_item_userdata: true,
            target_itemid: Some(pumpkin),
            target_actionid: 0,
            from: (0, 0, 0),
            to: (50, 50, 7),
            quest: None,
        };
        assert!(apply(&mut world, &req));
        assert_eq!(world.items.get(iid).unwrap().item_type, head);
    }

    #[test]
    fn pick_hole_pick_grounds_aid() {
        let mut world = minimal_world();
        load_pack_ids(&mut world);
        let dest = named(&world, "pickHoleOpen");
        let dirt = *world
            .tool_ids
            .pick_grounds
            .iter()
            .next()
            .expect("pickGrounds");
        let aid = world.tool_ids.pick_hole_aid().expect("pickHole aid");
        register_types(&mut world, &[dirt, dest]);
        let pos = Position::new(50, 50, 7);
        let below = Position::new(50, 50, 8);
        let iid = place_ground(&mut world, pos, dirt, aid);
        world.map.insert_tile(
            below,
            Tile::Normal(TileBody {
                ground: Some(100),
                ground_item: None,
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: 0,
                zone: ZoneType::Normal,
            }),
        );
        let cid = insert_player(&mut world, test_player("P", pos));
        let req = ToolUseRequest {
            kind: ToolUseKind::Pick,
            player: cid.data().as_ffi(),
            item: None,
            target_item: Some(iid.data().as_ffi()),
            target_creature: None,
            target_is_item_userdata: true,
            target_itemid: Some(dirt),
            target_actionid: aid,
            from: (pos.x, pos.y, pos.z),
            to: (pos.x, pos.y, pos.z),
            quest: None,
        };
        assert!(apply(&mut world, &req));
        assert_eq!(world.items.get(iid).unwrap().item_type, dest);
    }

    #[test]
    fn scythe_immature_cancel_and_mature_wheat() {
        let mut world = minimal_world();
        load_pack_ids(&mut world);
        let wheat_growing = named(&world, "wheatGrowing");
        let wheat_mature = named(&world, "wheatMature");
        let wheat_cut = named(&world, "wheatCut");
        let wheat_bunch = named(&world, "wheatBunch");
        register_types(
            &mut world,
            &[wheat_growing, wheat_mature, wheat_cut, wheat_bunch],
        );
        let cid = insert_player(&mut world, test_player("S", Position::new(50, 50, 7)));
        let growing = world.items.insert(Item::new_single(wheat_growing));
        let req_g = ToolUseRequest {
            kind: ToolUseKind::Scythe,
            player: cid.data().as_ffi(),
            item: None,
            target_item: Some(growing.data().as_ffi()),
            target_creature: None,
            target_is_item_userdata: true,
            target_itemid: Some(wheat_growing),
            target_actionid: 0,
            from: (0, 0, 0),
            to: (50, 50, 7),
            quest: None,
        };
        assert!(apply(&mut world, &req_g));
        assert_eq!(world.items.get(growing).unwrap().item_type, wheat_growing);

        let pos = Position::new(51, 50, 7);
        world.map.insert_tile(
            pos,
            Tile::Normal(TileBody {
                ground: Some(100),
                ground_item: None,
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: 0,
                zone: ZoneType::Normal,
            }),
        );
        let mature = world.items.insert(Item::new_single(wheat_mature));
        let req_m = ToolUseRequest {
            kind: ToolUseKind::Scythe,
            player: cid.data().as_ffi(),
            item: None,
            target_item: Some(mature.data().as_ffi()),
            target_creature: None,
            target_is_item_userdata: true,
            target_itemid: Some(wheat_mature),
            target_actionid: 0,
            from: (0, 0, 0),
            to: (pos.x, pos.y, pos.z),
            quest: None,
        };
        assert!(apply(&mut world, &req_m));
        assert_eq!(world.items.get(mature).unwrap().item_type, wheat_cut);
    }

    #[test]
    fn shovel_hole_transform_plus_one() {
        let mut world = minimal_world();
        load_pack_ids(&mut world);
        let hole = *world.tool_ids.holes.iter().next().expect("holes");
        let open = hole.saturating_add(1);
        register_types(&mut world, &[hole, open]);
        let pos = Position::new(50, 50, 7);
        let below = Position::new(50, 50, 8);
        let iid = place_ground(&mut world, pos, hole, 0);
        world.map.insert_tile(
            below,
            Tile::Normal(TileBody {
                ground: Some(100),
                ground_item: None,
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: 0,
                zone: ZoneType::Normal,
            }),
        );
        let cid = insert_player(&mut world, test_player("H", pos));
        let req = ToolUseRequest {
            kind: ToolUseKind::Shovel,
            player: cid.data().as_ffi(),
            item: None,
            target_item: Some(iid.data().as_ffi()),
            target_creature: None,
            target_is_item_userdata: true,
            target_itemid: Some(hole),
            target_actionid: 0,
            from: (pos.x, pos.y, pos.z),
            to: (pos.x, pos.y, pos.z),
            quest: None,
        };
        assert!(apply(&mut world, &req));
        assert_eq!(world.items.get(iid).unwrap().item_type, open);
    }
}
