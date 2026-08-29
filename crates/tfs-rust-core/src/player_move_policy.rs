//! Native player item-move policy — TVP `moveitem.lua` rules without VM entry.
//!
//! Pack: TFS `Events::eventPlayerOnMoveItem` / `eventPlayerOnItemMoved` (`events.cpp`);
//!       `data/scripts/eventcallbacks/player/moveitem.lua`.
//! IDs / aid bands: `data/defs/tools.lua` `moveItemPolicy` + `actionIds.blockingTile`.
//! Corpus: 772 `operate.cc` `CheckMoveObject` (moveability, pickup, range) runs before this.

use slotmap::Key;
use tfs_rust_common::Position;

use crate::cylinder::Cylinder;
use crate::game_world::GameWorld;
use crate::ids::ItemId;
use crate::return_value::ReturnValue;

/// TFS `eventPlayerOnMoveItem` — after native `queryAdd`, before transfer.
pub fn on_player_move_item(world: &mut GameWorld, item: ItemId, to: &Cylinder) -> ReturnValue {
    let policy = &world.tool_ids.move_item_policy;
    let Some(item_ref) = world.items.get(item) else {
        return ReturnValue::NotPossible;
    };
    if policy.is_quest_object_aid(item_ref.action_id()) {
        return ReturnValue::NotMoveable;
    }
    let item_type = item_ref.item_type;
    if let Some(&new_type) = policy.pre_move_transforms.get(&item_type) {
        let _ = world.lua_script_item_transform(item.data().as_ffi(), new_type, -1);
    }
    if let Cylinder::Tile { pos } = to
        && let Some(blocking_aid) = world.tool_ids.action_ids.get("blockingTile").copied()
        && let Some(ground_id) = world
            .map
            .get_tile(*pos)
            .and_then(|t| t.body().ground_item)
        && let Some(ground) = world.items.get(ground_id)
        && ground.action_id() == blocking_aid
    {
        return ReturnValue::NotEnoughRoom;
    }
    ReturnValue::NoError
}

/// TFS `eventPlayerOnItemMoved` — after successful transfer.
pub fn on_player_item_moved(world: &mut GameWorld, item: ItemId, to: &Cylinder) {
    let policy = &world.tool_ids.move_item_policy;
    let Some(item_ref) = world.items.get(item) else {
        return;
    };
    let Some(&new_type) = policy.post_move_transforms.get(&item_ref.item_type) else {
        return;
    };
    let effect_id = policy.post_move_effect_id;
    let _ = world
        .lua_script_item_transform(item.data().as_ffi(), new_type, -1);
    if let (Some(effect_id), Some(pos)) = (effect_id, cylinder_effect_position(world, to)) {
        world.broadcast_magic_effect(pos, effect_id);
    }
}

fn cylinder_effect_position(world: &GameWorld, cyl: &Cylinder) -> Option<Position> {
    match cyl {
        Cylinder::Tile { pos } => Some(*pos),
        Cylinder::Container { item_id, .. } => world.script_item_position(*item_id),
        Cylinder::Inventory { player_id, .. } => {
            world.creatures.get(*player_id).map(|c| c.position())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cylinder::Cylinder;
    use crate::item::Item;
    use crate::sim_harness::{ensure_walkable_tile, minimal_world};
    use crate::tile::{Tile, TileBody};
    use crate::tool_use::{MoveItemPolicyTables, load_from_data_dir};
    use std::path::PathBuf;
    use std::sync::Arc;
    use tfs_rust_common::Position;
    use tfs_rust_common::enums::ZoneType;
    use tfs_rust_content::otb::ItemType;

    fn workspace_data() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    fn load_policy_world() -> GameWorld {
        let mut world = minimal_world();
        let data = workspace_data();
        if data.join("defs/tools.lua").exists() {
            world.tool_ids = load_from_data_dir(&data);
        }
        world
    }

    fn register_item_types(world: &mut GameWorld, type_ids: &[u16]) {
        let db = Arc::make_mut(&mut world.items_db);
        for &id in type_ids {
            db.items.entry(id).or_insert_with(ItemType::default);
        }
    }

    fn place_ground(world: &mut GameWorld, pos: Position, type_id: u16, aid: u16) {
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
    }

    fn insert_item_on_tile(world: &mut GameWorld, pos: Position, type_id: u16, aid: u16) -> ItemId {
        let mut item = Item::new_single(type_id);
        if aid != 0 {
            item.set_action_id(aid);
        }
        let iid = world.items.insert(item);
        world
            .internal_add_item_to_tile(pos, iid, crate::cylinder::CylinderFlags::NONE)
            .expect("add to tile");
        iid
    }

    #[test]
    fn quest_action_id_not_moveable() {
        let data = workspace_data();
        if !data.join("defs/tools.lua").exists() {
            eprintln!("data pack not present — skipping");
            return;
        }
        let mut world = load_policy_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let iid = insert_item_on_tile(&mut world, pos, 2148, 1500);
        let to = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, to, 100);
        assert_eq!(
            on_player_move_item(&mut world, iid, &Cylinder::Tile { pos: to }),
            ReturnValue::NotMoveable
        );
    }

    #[test]
    fn candelabrum_transforms_before_move() {
        let data = workspace_data();
        if !data.join("defs/tools.lua").exists() {
            eprintln!("data pack not present — skipping");
            return;
        }
        let mut world = load_policy_world();
        let from_id = *world
            .tool_ids
            .move_item_policy
            .pre_move_transforms
            .keys()
            .next()
            .expect("preMoveTransforms");
        let to_id = world.tool_ids.move_item_policy.pre_move_transforms[from_id];
        register_item_types(&mut world, &[*from_id, to_id]);
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let iid = insert_item_on_tile(&mut world, pos, *from_id, 0);
        let dest = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, dest, 100);
        assert_eq!(
            on_player_move_item(&mut world, iid, &Cylinder::Tile { pos: dest }),
            ReturnValue::NoError
        );
        assert_eq!(world.items.get(iid).map(|i| i.item_type), Some(to_id));
    }

    #[test]
    fn blocking_tile_ground_not_enough_room() {
        let data = workspace_data();
        if !data.join("defs/tools.lua").exists() {
            eprintln!("data pack not present — skipping");
            return;
        }
        let mut world = load_policy_world();
        let blocking_aid = world
            .tool_ids
            .action_ids
            .get("blockingTile")
            .copied()
            .expect("blockingTile in tools.lua");
        let from = Position::new(100, 100, 7);
        let to = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, from, 100);
        place_ground(&mut world, to, 100, blocking_aid);
        let iid = insert_item_on_tile(&mut world, from, 2148, 0);
        assert_eq!(
            on_player_move_item(&mut world, iid, &Cylinder::Tile { pos: to }),
            ReturnValue::NotEnoughRoom
        );
    }

    #[test]
    fn trap_closes_after_move_with_effect() {
        let data = workspace_data();
        if !data.join("defs/tools.lua").exists() {
            eprintln!("data pack not present — skipping");
            return;
        }
        let mut world = load_policy_world();
        let open_id = *world
            .tool_ids
            .move_item_policy
            .post_move_transforms
            .keys()
            .next()
            .expect("postMoveTransforms");
        let closed_id = world.tool_ids.move_item_policy.post_move_transforms[open_id];
        register_item_types(&mut world, &[*open_id, closed_id]);
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let iid = insert_item_on_tile(&mut world, pos, *open_id, 0);
        on_player_item_moved(&mut world, iid, &Cylinder::Tile { pos });
        assert_eq!(world.items.get(iid).map(|i| i.item_type), Some(closed_id));
    }

    #[test]
    fn empty_policy_is_noop() {
        let mut world = minimal_world();
        world.tool_ids.move_item_policy = MoveItemPolicyTables::default();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let iid = insert_item_on_tile(&mut world, pos, 2148, 1500);
        assert_eq!(
            on_player_move_item(&mut world, iid, &Cylinder::Tile { pos }),
            ReturnValue::NoError
        );
    }
}
