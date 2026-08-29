//! TFS `data/lib/core/game.lua` map helpers — native pack surface.
//!
//! Pack: `Game.isItemInPosition`, `Game.transformItemInPosition`, `Tile.relocateTo`, …
//! C++ reference: composes `Game::findItemOfType` / `Game::transformItem` (`game.cpp`).

use slotmap::Key;
use tfs_rust_common::Position;
use tfs_rust_common::ScriptContext;
use tfs_rust_common::ScriptThing;
use tfs_rust_lua::LuaMoveDestination;

use crate::game_world::GameWorld;
use crate::ids::ItemId;

impl GameWorld {
    fn tile_item_by_type(&self, pos: Position, item_type: u16) -> Option<ItemId> {
        let tile = self.map.get_tile(pos)?;
        for iid in tile.body().script_stack_item_ids() {
            let item = self.items.get(iid)?;
            if item.item_type == item_type {
                return Some(iid);
            }
        }
        None
    }

    /// `Game.isItemInPosition` — `game.lua`.
    pub fn game_is_item_in_position(&self, pos: Position, item_type: u16) -> Result<bool, String> {
        if self.map.get_tile(pos).is_none() {
            return Err("Game.isItemInPosition - Tile not found".to_string());
        }
        Ok(self.tile_item_by_type(pos, item_type).is_some())
    }

    /// `Game.removeItemInPosition` — `game.lua`.
    pub fn game_remove_item_in_position(
        &mut self,
        pos: Position,
        item_type: u16,
    ) -> Result<bool, String> {
        if self.map.get_tile(pos).is_none() {
            return Err("Game.removeItemInPosition - Tile not found".to_string());
        }
        let Some(iid) = self.tile_item_by_type(pos, item_type) else {
            return Ok(false);
        };
        self.lua_script_item_remove(iid.data().as_ffi(), -1)
    }

    /// `Game.transformItemInPosition` — `game.lua`.
    pub fn game_transform_item_in_position(
        &mut self,
        pos: Position,
        from_type: u16,
        to_type: u16,
    ) -> Result<bool, String> {
        if self.map.get_tile(pos).is_none() {
            return Err("Game.transformItemInPosition - Tile not found".to_string());
        }
        let Some(iid) = self.tile_item_by_type(pos, from_type) else {
            return Ok(false);
        };
        let item_u64 = iid.data().as_ffi();
        self.lua_script_item_transform(item_u64, to_type, -1)?;
        if let Some(id) = self.resolve_item_u64(item_u64) {
            self.start_decay(id);
        }
        Ok(true)
    }

    /// `Game.removeItemsInPosition` — `game.lua`.
    pub fn game_remove_items_in_position(&mut self, pos: Position) -> Result<(), String> {
        if self.map.get_tile(pos).is_none() {
            return Err("Game.removeItemsInPosition - Tile not found".to_string());
        }
        let tile = self.map.get_tile(pos).expect("checked");
        let body = tile.body();
        let mut to_remove: Vec<ItemId> = Vec::new();
        for iid in body.script_stack_item_ids() {
            let Some(item) = self.items.get(iid) else {
                continue;
            };
            let movable = self
                .items_db
                .items
                .get(&item.item_type)
                .is_some_and(|t| !t.is_immovable());
            if movable {
                to_remove.push(iid);
            }
        }
        for iid in to_remove {
            self.lua_script_item_remove(iid.data().as_ffi(), -1)?;
        }
        Ok(())
    }

    /// `Game.setMapItemActionId` — `game.lua`.
    pub fn game_set_map_item_action_id(
        &mut self,
        pos: Position,
        item_type: u16,
        action_id: u16,
    ) -> Result<bool, String> {
        if self.map.get_tile(pos).is_none() {
            return Err("Game.setMapItemActionId - Tile not found".to_string());
        }
        let Some(iid) = self.tile_item_by_type(pos, item_type) else {
            return Err("Game.setMapItemActionId - Item not found".to_string());
        };
        self.lua_script_set_action_id(iid.data().as_ffi(), action_id)?;
        Ok(true)
    }

    /// `Game.getStorageValue` — ephemeral quest globals (`game.lua` `globalStorageTable`).
    pub fn get_global_storage(&self, key: u32) -> Option<i32> {
        self.global_storage.get(&key).copied()
    }

    /// `Game.setStorageValue` — ephemeral quest globals.
    pub fn set_global_storage(&mut self, key: u32, value: i32) {
        self.global_storage.insert(key, value);
    }

    /// `Tile.relocateTo` — `data/lib/core/tile.lua`.
    pub fn tile_relocate_to(&mut self, from: Position, to: Position) -> bool {
        if from == to || self.map.get_tile(to).is_none() {
            return false;
        }
        let count = self.tile_get_thing_count(from.x, from.y, from.z);
        let mut things = Vec::new();
        for i in (0..count).rev() {
            if let Some(th) = self.tile_get_thing(from.x, from.y, from.z, i) {
                things.push(th);
            }
        }
        for th in things {
            match th {
                ScriptThing::Item(id) => {
                    let Some(iid) = self.resolve_item_u64(id) else {
                        continue;
                    };
                    let Some(item) = self.items.get(iid) else {
                        continue;
                    };
                    let item_type = item.item_type;
                    let fluid = self
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
                        let _ = self.lua_script_item_remove(id, -1);
                    } else if self
                        .items_db
                        .items
                        .get(&item_type)
                        .is_some_and(|t| t.moveable())
                    {
                        let _ = self.lua_script_item_move_to(
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
                ScriptThing::Creature(id) => {
                    let _ = self.lua_script_creature_teleport(id, to.x, to.y, to.z, false);
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim_harness::minimal_world;

    #[test]
    fn global_storage_roundtrip() {
        let mut world = minimal_world();
        assert_eq!(world.get_global_storage(10000), None);
        world.set_global_storage(10000, 1);
        assert_eq!(world.get_global_storage(10000), Some(1));
    }
}
