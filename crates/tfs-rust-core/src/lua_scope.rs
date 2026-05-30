//! Scoped Lua script execution with read context + mutation applier.
//!
//! C++ reference: `LuaScriptInterface::executeTimer` / creature event dispatch — single game thread.

use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use tfs_rust_lua::{
    self, LuaMutation, with_lua_context, with_lua_mutation_scope,
    set_mutation_bool_result, set_mutation_item_result,
};

fn apply_lua_mutation(world_ptr: *mut (), mutation: LuaMutation) -> Result<(), String> {
    if world_ptr.is_null() {
        return Err("Lua mutation scope not active".into());
    }
    let world = world_ptr as *mut GameWorld;
    // SAFETY: `world_ptr` is set by `with_lua_mutation_scope` immediately before Lua runs
    // and cleared before returning. Game thread only.
    match mutation {
        LuaMutation::PlayerAddItem {
            creature_id,
            item_type,
            count,
        } => unsafe { &mut *world }.lua_script_add_item(creature_id, item_type, count),
        LuaMutation::PlayerAddItemFull {
            creature_id,
            item_type,
            count,
            sub_type,
            can_drop_on_map,
            slot,
        } => {
            let result = unsafe {
                &mut *world
            }
            .lua_script_player_add_item_full(
                creature_id,
                item_type,
                count,
                sub_type,
                can_drop_on_map,
                slot,
            )?;
            if let Some(id) = result {
                set_mutation_item_result(id);
            }
            Ok(())
        }
        LuaMutation::PlayerRemoveItem {
            creature_id,
            item_type,
            count,
            sub_type,
            ignore_equipped,
        } => unsafe {
            &mut *world
        }
        .lua_script_remove_item(creature_id, item_type, count, sub_type, ignore_equipped),
        LuaMutation::PlayerGetDepotChest {
            creature_id,
            depot_id,
            auto_create,
        } => {
            let result = unsafe {
                &mut *world
            }
            .lua_script_get_depot_chest(creature_id, depot_id, auto_create)?;
            if let Some(id) = result {
                set_mutation_item_result(id);
            }
            Ok(())
        }
        LuaMutation::PlayerGetInbox { creature_id } => {
            let result = unsafe { &mut *world }.lua_script_get_inbox(creature_id)?;
            if let Some(id) = result {
                set_mutation_item_result(id);
            }
            Ok(())
        }
        LuaMutation::ItemMoveTo {
            item_id,
            dest,
            flags,
        } => {
            let ok = unsafe { &mut *world }.lua_script_item_move_to(item_id, dest, flags)?;
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::ItemRemove { item_id, count } => {
            let ok = unsafe { &mut *world }.lua_script_item_remove(item_id, count)?;
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::ContainerAddItem {
            container_id,
            item_type,
            count,
            index,
            flags,
        } => {
            let result = unsafe {
                &mut *world
            }
            .lua_script_container_add_item(container_id, item_type, count, index, flags)?;
            if let Some(id) = result {
                set_mutation_item_result(id);
            }
            Ok(())
        }
        LuaMutation::ItemSetActionId {
            item_id,
            action_id,
        } => unsafe { &mut *world }.lua_script_set_action_id(item_id, action_id),
        LuaMutation::ItemSetUniqueId {
            item_id,
            unique_id,
        } => unsafe { &mut *world }.lua_script_set_unique_id(item_id, unique_id),
        LuaMutation::ItemSetStoreItem { item_id, store } => unsafe {
            &mut *world
        }
        .lua_script_set_store_item(item_id, store),
    }
}

/// Register the mutation applier once at server startup.
pub fn register_lua_mutation_hooks() {
    tfs_rust_lua::register_lua_mutation_applier(apply_lua_mutation);
}

fn with_equip_mutation_scope<F, R>(world: &mut GameWorld, f: F) -> R
where
    F: FnOnce(&mut GameWorld) -> R,
{
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || f(unsafe { &mut *world_ptr }))
    })
}

/// Run a creature login script with read context and mutation scope active.
pub fn fire_on_login(world: &mut GameWorld, cid: CreatureId) {
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            world.events.on_login(cid, world);
        });
    });
}

/// TFS `Events::eventPlayerOnInventoryUpdate` with read/mutation scope for userdata.
pub fn fire_on_player_inventory_update(
    world: &mut GameWorld,
    player: CreatureId,
    item: ItemId,
    slot: u8,
    equip: bool,
) {
    with_equip_mutation_scope(world, |world| {
        world
            .events
            .on_player_inventory_update(player, item, slot, equip);
    });
}

/// TFS `MoveEvents::onPlayerEquip` with `isCheck == true` — `player.cpp` `queryAdd`.
pub fn fire_on_player_equip_check(
    world: &mut GameWorld,
    player: CreatureId,
    item: ItemId,
    slot: u8,
) -> ReturnValue {
    let item_type = world.items.get(item).map(|i| i.item_type).unwrap_or(0);
    let player_level = player_level_u32(world, player);
    with_equip_mutation_scope(world, |world| {
        world
            .events
            .on_player_equip_check(player, item, item_type, slot, player_level)
    })
}

/// TFS `MoveEvents::onPlayerEquip` — `postAddNotification`.
pub fn fire_on_player_equip(world: &mut GameWorld, player: CreatureId, item: ItemId, slot: u8) {
    let item_type = world.items.get(item).map(|i| i.item_type).unwrap_or(0);
    let player_level = player_level_u32(world, player);
    with_equip_mutation_scope(world, |world| {
        world
            .events
            .on_player_equip(player, item, item_type, slot, player_level);
    });
}

/// TFS `MoveEvents::onPlayerDeEquip` — `postRemoveNotification`.
pub fn fire_on_player_deequip(world: &mut GameWorld, player: CreatureId, item: ItemId, slot: u8) {
    let item_type = world.items.get(item).map(|i| i.item_type).unwrap_or(0);
    let player_level = player_level_u32(world, player);
    with_equip_mutation_scope(world, |world| {
        world
            .events
            .on_player_deequip(player, item, item_type, slot, player_level);
    });
}

fn player_level_u32(world: &GameWorld, player: CreatureId) -> u32 {
    world
        .creatures
        .get(player)
        .and_then(|c| match c {
            crate::creature::CreatureKind::Player(p) => Some(p.level.max(0) as u32),
            _ => None,
        })
        .unwrap_or(0)
}
