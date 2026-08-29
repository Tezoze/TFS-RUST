//! Dispatch `EventCallback(EVENT_CALLBACK_ONMOVEITEM / ONITEMMOVED)`.
//!
//! Pack: TFS `Events::eventPlayerOnMoveItem` / `eventPlayerOnItemMoved` (`events.cpp`)
//! via allowlisted `data/scripts/eventcallbacks/player/moveitem.lua`.
//! Native `queryAdd` already succeeded; Lua may cancel with a ReturnValue or
//! mutate the item (candelabrum). `onItemMoved` is void (open trap).

use mlua::{MultiValue, Value};

use crate::context::{CreatureRef, ItemRef};
use crate::event_callback::{
    EVENT_CALLBACK_ONITEMMOVED, EVENT_CALLBACK_ONMOVEITEM, multivalue_to_return_int,
};
use crate::runtime::{LuaError, LuaRuntime};
use crate::userdata::container::ContainerRef;
use crate::userdata::position::PositionRef;
use crate::userdata::tile::TileRef;

/// Cylinder passed into the move-item EventCallback.
#[derive(Clone, Copy, Debug)]
pub enum MoveItemCylinder {
    Tile { x: u16, y: u16, z: u8 },
    Container { item: u64 },
    Inventory { player: u64 },
}

impl LuaRuntime {
    /// `EventCallback(ONMOVEITEM, player, item, count, fromPos, toPos, fromCyl, toCyl)`.
    ///
    /// Returns the Lua ReturnValue integer (`0` = no error). No-op `0` when
    /// nothing is registered.
    pub fn call_player_on_move_item(
        &self,
        player: u64,
        item: u64,
        count: u16,
        from_x: u16,
        from_y: u16,
        from_z: u8,
        to_x: u16,
        to_y: u16,
        to_z: u8,
        from_cyl: MoveItemCylinder,
        to_cyl: MoveItemCylinder,
    ) -> Result<i32, LuaError> {
        let ret = self.dispatch_event_callbacks(EVENT_CALLBACK_ONMOVEITEM, |lua| {
            let player_ud = lua.create_userdata(CreatureRef(player))?;
            let item_ud = lua.create_userdata(ItemRef(item))?;
            let from_pos = lua.create_userdata(PositionRef {
                x: from_x,
                y: from_y,
                z: from_z,
            })?;
            let to_pos = lua.create_userdata(PositionRef {
                x: to_x,
                y: to_y,
                z: to_z,
            })?;
            let from_c = push_move_cylinder(lua, from_cyl)?;
            let to_c = push_move_cylinder(lua, to_cyl)?;
            Ok(MultiValue::from_iter([
                Value::UserData(player_ud),
                Value::UserData(item_ud),
                Value::Integer(i64::from(count)),
                Value::UserData(from_pos),
                Value::UserData(to_pos),
                from_c,
                to_c,
            ]))
        })?;
        Ok(ret.map(multivalue_to_return_int).unwrap_or(0))
    }

    /// `EventCallback(ONITEMMOVED, …)` — void; errors are logged by the caller.
    pub fn call_player_on_item_moved(
        &self,
        player: u64,
        item: u64,
        count: u16,
        from_x: u16,
        from_y: u16,
        from_z: u8,
        to_x: u16,
        to_y: u16,
        to_z: u8,
        from_cyl: MoveItemCylinder,
        to_cyl: MoveItemCylinder,
    ) -> Result<(), LuaError> {
        let _ = self.dispatch_event_callbacks(EVENT_CALLBACK_ONITEMMOVED, |lua| {
            let player_ud = lua.create_userdata(CreatureRef(player))?;
            let item_ud = lua.create_userdata(ItemRef(item))?;
            let from_pos = lua.create_userdata(PositionRef {
                x: from_x,
                y: from_y,
                z: from_z,
            })?;
            let to_pos = lua.create_userdata(PositionRef {
                x: to_x,
                y: to_y,
                z: to_z,
            })?;
            let from_c = push_move_cylinder(lua, from_cyl)?;
            let to_c = push_move_cylinder(lua, to_cyl)?;
            Ok(MultiValue::from_iter([
                Value::UserData(player_ud),
                Value::UserData(item_ud),
                Value::Integer(i64::from(count)),
                Value::UserData(from_pos),
                Value::UserData(to_pos),
                from_c,
                to_c,
            ]))
        })?;
        Ok(())
    }
}

fn push_move_cylinder(
    lua: &mlua::Lua,
    cyl: MoveItemCylinder,
) -> Result<mlua::Value, LuaError> {
    match cyl {
        MoveItemCylinder::Tile { x, y, z } => {
            let ud = lua.create_userdata(TileRef { x, y, z })?;
            Ok(mlua::Value::UserData(ud))
        }
        MoveItemCylinder::Container { item } => {
            let ud = lua.create_userdata(ContainerRef(item))?;
            Ok(mlua::Value::UserData(ud))
        }
        MoveItemCylinder::Inventory { player } => {
            let ud = lua.create_userdata(CreatureRef(player))?;
            Ok(mlua::Value::UserData(ud))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::load_data_lib;
    use crate::context::with_lua_context;
    use crate::lua_mutation::{
        LuaMutation, register_lua_mutation_applier, set_mutation_bool_result,
        with_lua_mutation_scope,
    };
    use std::cell::Cell;
    use std::path::{Path, PathBuf};
    use tfs_rust_common::{
        Position, ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptItemData,
        ScriptItemId, ScriptItemRef,
    };

    const ITEM: u64 = 1;
    const GROUND: u64 = 2;
    const RETURNVALUE_NOTMOVEABLE: i32 = 9;
    const RETURNVALUE_NOTENOUGHROOM: i32 = 2;

    fn workspace_data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    fn load_moveitem(runtime: &mut LuaRuntime, data_root: &Path) {
        load_data_lib(runtime, data_root).expect("data lib");
        let path = data_root.join("scripts/eventcallbacks/player/moveitem.lua");
        let _guard = runtime.enter_scripts_interface();
        runtime
            .load_script(path.to_str().expect("utf8"))
            .expect("moveitem.lua");
        runtime
            .sync_event_callbacks_from_lua()
            .expect("sync moveitem callbacks");
        // `actionIds` lives in `data/global.lua`, which load_data_lib does not exec.
        runtime
            .exec_chunk(
                "test_action_ids",
                "actionIds = actionIds or {}; actionIds.blockingTile = actionIds.blockingTile or 4005",
            )
            .expect("actionIds.blockingTile");
    }

    fn tile_pair() -> (MoveItemCylinder, MoveItemCylinder) {
        (
            MoveItemCylinder::Tile {
                x: 100,
                y: 100,
                z: 7,
            },
            MoveItemCylinder::Tile {
                x: 101,
                y: 100,
                z: 7,
            },
        )
    }

    struct MoveItemCtx {
        item_type: u16,
        action_id: u16,
        ground: Option<u64>,
        ground_action_id: u16,
    }

    impl ScriptContext for MoveItemCtx {
        fn get_creature(&self, _: ScriptCreatureId) -> Option<ScriptCreatureData> {
            None
        }
        fn get_item(&self, id: ScriptItemId) -> Option<ScriptItemRef> {
            Some(ScriptItemRef(id))
        }
        fn get_config_string(&self, _: &str) -> Option<String> {
            None
        }
        fn get_item_data(&self, id: ScriptItemId) -> Option<ScriptItemData> {
            if id == ITEM {
                Some(ScriptItemData {
                    item_type: self.item_type,
                    count: 1,
                    weight: 0,
                    name: "test".into(),
                    action_id: self.action_id,
                    unique_id: 0,
                    is_store_item: false,
                    fluid_type: 0,
                    sub_type: 0,
                })
            } else if id == GROUND {
                Some(ScriptItemData {
                    item_type: 100,
                    count: 1,
                    weight: 0,
                    name: "ground".into(),
                    action_id: self.ground_action_id,
                    unique_id: 0,
                    is_store_item: false,
                    fluid_type: 0,
                    sub_type: 0,
                })
            } else {
                None
            }
        }
        fn tile_get_ground_item(&self, _: u16, _: u16, _: u8) -> Option<ScriptItemId> {
            self.ground
        }
        fn get_item_position(&self, _: ScriptItemId) -> Option<Position> {
            Some(Position {
                x: 101,
                y: 100,
                z: 7,
            })
        }
    }

    fn call_move(runtime: &LuaRuntime, ctx: &MoveItemCtx) -> i32 {
        let (from, to) = tile_pair();
        with_lua_context(ctx, || {
            runtime
                .call_player_on_move_item(1, ITEM, 1, 100, 100, 7, 101, 100, 7, from, to)
                .expect("onMoveItem")
        })
    }

    #[test]
    fn move_item_noop_when_unregistered() {
        let data_root = workspace_data_root();
        if !data_root.join("scripts/lib/event_callbacks.lua").exists() {
            eprintln!("data pack not present — skipping");
            return;
        }
        let runtime = LuaRuntime::new().expect("runtime");
        load_data_lib(&runtime, &data_root).expect("data lib");
        let (from, to) = tile_pair();
        let rv = runtime
            .call_player_on_move_item(1, ITEM, 1, 100, 100, 7, 101, 100, 7, from, to)
            .expect("no callback");
        assert_eq!(rv, 0);
        runtime
            .call_player_on_item_moved(1, ITEM, 1, 100, 100, 7, 101, 100, 7, from, to)
            .expect("no callback");
    }

    #[test]
    fn move_item_quest_aid_not_moveable() {
        let data_root = workspace_data_root();
        if !data_root
            .join("scripts/eventcallbacks/player/moveitem.lua")
            .exists()
        {
            eprintln!("data pack not present — skipping");
            return;
        }
        let mut runtime = LuaRuntime::new().expect("runtime");
        load_moveitem(&mut runtime, &data_root);
        let ctx = MoveItemCtx {
            item_type: 2148,
            action_id: 1500,
            ground: None,
            ground_action_id: 0,
        };
        assert_eq!(call_move(&runtime, &ctx), RETURNVALUE_NOTMOVEABLE);
    }

    #[test]
    fn move_item_candelabrum_transforms_before_move() {
        let data_root = workspace_data_root();
        if !data_root
            .join("scripts/eventcallbacks/player/moveitem.lua")
            .exists()
        {
            eprintln!("data pack not present — skipping");
            return;
        }
        let mut runtime = LuaRuntime::new().expect("runtime");
        load_moveitem(&mut runtime, &data_root);
        thread_local! {
            static TRANSFORMED: Cell<Option<(u64, u16)>> = const { Cell::new(None) };
        }
        TRANSFORMED.with(|c| c.set(None));
        register_lua_mutation_applier(|_, mutation| {
            if let LuaMutation::ItemTransform {
                item_id, new_type, ..
            } = mutation
            {
                TRANSFORMED.with(|c| c.set(Some((item_id, new_type))));
                set_mutation_bool_result(true);
            }
            Ok(())
        });
        let ctx = MoveItemCtx {
            item_type: 2057,
            action_id: 0,
            ground: None,
            ground_action_id: 0,
        };
        let rv = with_lua_mutation_scope(1 as *mut (), || call_move(&runtime, &ctx));
        assert_eq!(rv, 0);
        assert_eq!(TRANSFORMED.with(|c| c.get()), Some((ITEM, 2042)));
    }

    #[test]
    fn move_item_trap_transforms_after_move() {
        let data_root = workspace_data_root();
        if !data_root
            .join("scripts/eventcallbacks/player/moveitem.lua")
            .exists()
        {
            eprintln!("data pack not present — skipping");
            return;
        }
        let mut runtime = LuaRuntime::new().expect("runtime");
        load_moveitem(&mut runtime, &data_root);
        thread_local! {
            static TRANSFORMED: Cell<Option<(u64, u16)>> = const { Cell::new(None) };
        }
        TRANSFORMED.with(|c| c.set(None));
        register_lua_mutation_applier(|_, mutation| {
            match mutation {
                LuaMutation::ItemTransform {
                    item_id, new_type, ..
                } => {
                    TRANSFORMED.with(|c| c.set(Some((item_id, new_type))));
                    set_mutation_bool_result(true);
                }
                LuaMutation::PositionSendMagicEffect { .. } => {}
                _ => {}
            }
            Ok(())
        });
        let ctx = MoveItemCtx {
            item_type: 2579,
            action_id: 0,
            ground: None,
            ground_action_id: 0,
        };
        let (from, to) = tile_pair();
        with_lua_mutation_scope(1 as *mut (), || {
            with_lua_context(&ctx, || {
                runtime
                    .call_player_on_item_moved(1, ITEM, 1, 100, 100, 7, 101, 100, 7, from, to)
                    .expect("onItemMoved")
            })
        });
        assert_eq!(TRANSFORMED.with(|c| c.get()), Some((ITEM, 2578)));
    }

    #[test]
    fn move_item_blocking_tile_not_enough_room() {
        let data_root = workspace_data_root();
        if !data_root
            .join("scripts/eventcallbacks/player/moveitem.lua")
            .exists()
        {
            eprintln!("data pack not present — skipping");
            return;
        }
        let mut runtime = LuaRuntime::new().expect("runtime");
        load_moveitem(&mut runtime, &data_root);
        let ctx = MoveItemCtx {
            item_type: 2148,
            action_id: 0,
            ground: Some(GROUND),
            ground_action_id: 4005,
        };
        assert_eq!(call_move(&runtime, &ctx), RETURNVALUE_NOTENOUGHROOM);
    }
}
