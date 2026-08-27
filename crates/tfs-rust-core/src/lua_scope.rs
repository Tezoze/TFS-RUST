//! Scoped Lua script execution with read context + mutation applier.
//!
//! C++ reference: `LuaScriptInterface::executeTimer` / creature event dispatch — single game thread.

use crate::cylinder::Cylinder;
use crate::event_dispatcher::{EventCylinder, TalkActionResult};
use crate::game_world::{GameWorld, TileMoveEventItem};
use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use tfs_rust_common::Position;
use tfs_rust_lua::{
    self, LuaMutation, set_mutation_bool_result, set_mutation_i32_result, set_mutation_item_result,
    with_lua_context, with_lua_mutation_scope,
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
            let result = unsafe { &mut *world }.lua_script_player_add_item_full(
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
        } => {
            let ok = unsafe { &mut *world }.lua_script_remove_item(
                creature_id,
                item_type,
                count,
                sub_type,
                ignore_equipped,
            )?;
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::PlayerGetDepotChest {
            creature_id,
            depot_id,
            auto_create,
        } => {
            let result = unsafe { &mut *world }.lua_script_get_depot_chest(
                creature_id,
                depot_id,
                auto_create,
            )?;
            if let Some(id) = result {
                set_mutation_item_result(id);
            }
            Ok(())
        }
        LuaMutation::PlayerGetDepotLocker {
            creature_id,
            depot_id,
        } => {
            let result =
                unsafe { &mut *world }.lua_script_get_depot_locker(creature_id, depot_id)?;
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
            let result = unsafe { &mut *world }.lua_script_container_add_item(
                container_id,
                item_type,
                count,
                index,
                flags,
            )?;
            if let Some(id) = result {
                set_mutation_item_result(id);
            }
            Ok(())
        }
        LuaMutation::ItemSetActionId { item_id, action_id } => {
            unsafe { &mut *world }.lua_script_set_action_id(item_id, action_id)
        }
        LuaMutation::ItemSetUniqueId { item_id, unique_id } => {
            unsafe { &mut *world }.lua_script_set_unique_id(item_id, unique_id)
        }
        LuaMutation::ItemSetStoreItem { item_id, store } => {
            unsafe { &mut *world }.lua_script_set_store_item(item_id, store)
        }
        LuaMutation::ItemSetCustomAttribute {
            item_id,
            key,
            value,
        } => unsafe { &mut *world }.lua_script_set_custom_attribute(item_id, key, value),
        LuaMutation::PlayerFeed {
            creature_id,
            amount,
        } => unsafe { &mut *world }.lua_script_player_feed(creature_id, amount),
        LuaMutation::PlayerSendCancelMessage { creature_id, text } => {
            unsafe { &mut *world }.lua_script_player_send_cancel_message(creature_id, text)
        }
        LuaMutation::PlayerAddCondition { creature_id, spec } => {
            unsafe { &mut *world }.lua_script_player_add_condition(creature_id, spec)
        }
        LuaMutation::PlayerSetInFight {
            creature_id,
            in_fight,
        } => unsafe { &mut *world }.lua_script_player_set_in_fight(creature_id, in_fight),
        LuaMutation::PlayerRemoveCondition {
            creature_id,
            ctype,
            cond_id,
            sub_id,
        } => unsafe { &mut *world }.lua_script_player_remove_condition(
            creature_id,
            ctype,
            cond_id,
            sub_id,
        ),
        LuaMutation::SendChannelMessage {
            channel_id,
            speak_type,
            text,
        } => unsafe { &mut *world }.lua_script_send_channel_message(channel_id, speak_type, text),
        LuaMutation::PositionSendMagicEffect { x, y, z, effect } => {
            let pos = tfs_rust_common::Position { x, y, z };
            unsafe { &mut *world }.broadcast_magic_effect(pos, effect);
            Ok(())
        }
        LuaMutation::ItemDecay { item_id } => {
            // TFS `luaItemDecay` → `Item::startDecaying` → `Game::startDecay`.
            let world = unsafe { &mut *world };
            if let Some(id) = world.resolve_item_u64(item_id) {
                world.start_decay(id);
            }
            Ok(())
        }
        LuaMutation::PlayerAddMana {
            creature_id,
            mana_change,
        } => unsafe { &mut *world }.lua_script_player_add_mana(creature_id, mana_change),
        LuaMutation::PlayerAddHealth {
            creature_id,
            health_change,
        } => unsafe { &mut *world }.lua_script_player_add_health(creature_id, health_change),
        LuaMutation::PlayerAddManaSpent {
            creature_id,
            amount,
        } => unsafe { &mut *world }.lua_script_player_add_mana_spent(creature_id, amount),
        LuaMutation::ItemTransform {
            item_id,
            new_type,
            sub_type,
        } => {
            let ok =
                unsafe { &mut *world }.lua_script_item_transform(item_id, new_type, sub_type)?;
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::CombatExecute { request } => {
            // PC-3a: `combat:execute(creature, variant)` — synchronous AoE combat.
            // C++ reference: `luascript.cpp:13198` `luaCombatExecute` →
            // `Combat::doCombat` (`combat.cpp:683,737`). The Lua side resolved
            // area offsets + formula min/max; the core iterates tiles, checks
            // `throw_possible`, and applies damage per creature.
            unsafe { &mut *world }.combat_execute_from_lua(&request)
        }
        LuaMutation::ChallengeCreature {
            challenger_id,
            target_id,
        } => {
            let ok = unsafe { &mut *world }.lua_do_challenge_creature(challenger_id, target_id);
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::CreateMonster {
            name,
            x,
            y,
            z,
            extended,
            force,
        } => {
            let result = unsafe { &mut *world }
                .lua_script_create_monster(&name, x, y, z, extended, force)?;
            if let Some(id) = result {
                set_mutation_item_result(id);
            }
            Ok(())
        }
        LuaMutation::StartRaid { name } => {
            let rv = unsafe { &mut *world }.schedule_raid_now(&name);
            set_mutation_i32_result(crate::raid_waves::raid_return_to_lua_i32(rv));
            Ok(())
        }
        LuaMutation::AddSummon {
            master_id,
            summon_id,
        } => {
            let ok = unsafe { &mut *world }.lua_script_add_summon(master_id, summon_id)?;
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::CreatureMoveToTile {
            creature_id,
            x,
            y,
            z,
            flags,
        } => {
            let ret = unsafe { &mut *world }.lua_script_creature_move_to_tile(
                creature_id,
                x,
                y,
                z,
                flags,
            )?;
            set_mutation_bool_result(ret);
            Ok(())
        }
        LuaMutation::CreatureTeleport {
            creature_id,
            x,
            y,
            z,
            push_movement,
        } => {
            let ok = unsafe { &mut *world }.lua_script_creature_teleport(
                creature_id,
                x,
                y,
                z,
                push_movement,
            )?;
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::PlayerSendTextMessage {
            creature_id,
            msg_class,
            text,
        } => {
            unsafe { &mut *world }.lua_script_player_send_text_message(creature_id, msg_class, text)
        }
        LuaMutation::NpcSay { npc_id, text } => {
            unsafe { &mut *world }.npc_lua_say_u64(npc_id, text.as_str())
        }
        LuaMutation::NpcSetFocus { npc_id, player_id } => {
            unsafe { &mut *world }.npc_lua_set_focus_u64(npc_id, player_id)
        }
        LuaMutation::PlayerBankDeposit {
            creature_id,
            amount,
        } => unsafe { &mut *world }.player_bank_deposit_u64(creature_id, amount),
        LuaMutation::PlayerBankWithdraw {
            creature_id,
            amount,
        } => unsafe { &mut *world }.player_bank_withdraw_u64(creature_id, amount),
        LuaMutation::PlayerRemoveMoney {
            creature_id,
            amount,
        } => {
            let ok = unsafe { &mut *world }.player_remove_money_u64(creature_id, amount);
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::PlayerSetBankBalance {
            creature_id,
            balance,
        } => {
            let ok = unsafe { &mut *world }.player_set_bank_balance_u64(creature_id, balance);
            if ok {
                Ok(())
            } else {
                Err("player not found".into())
            }
        }
        LuaMutation::PlayerSetPremiumEndsAt {
            creature_id,
            ends_at,
        } => {
            let ok = unsafe { &mut *world }.player_set_premium_ends_at_u64(creature_id, ends_at);
            if ok {
                Ok(())
            } else {
                Err("player not found".into())
            }
        }
        LuaMutation::PlayerSetStorageValue {
            creature_id,
            key,
            value,
        } => unsafe { &mut *world }.lua_script_player_set_storage(creature_id, key, value),
        LuaMutation::PlayerSetTown {
            creature_id,
            town_id,
        } => unsafe { &mut *world }.lua_script_player_set_town(creature_id, town_id),
        LuaMutation::SetWorldLight { level, color } => {
            let ok = unsafe { &mut *world }.set_world_light(level, color);
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::ClearField {
            exclude_item_id,
            exclude_creature_id,
        } => {
            let w = unsafe { &mut *world };
            let Some(item) = w.resolve_item_u64(exclude_item_id) else {
                return Ok(());
            };
            let exclude_cid = exclude_creature_id.and_then(|id| w.resolve_creature_u64(id));
            w.clear_field(item, exclude_cid);
            Ok(())
        }
        LuaMutation::PlayerAddSkillTries {
            creature_id,
            skill,
            tries,
        } => {
            let ok =
                unsafe { &mut *world }.lua_script_add_skill_tries(creature_id, skill, tries)?;
            if ok {
                set_mutation_bool_result(true);
            }
            Ok(())
        }
        LuaMutation::TileAddItem {
            x,
            y,
            z,
            item_type,
            count,
            flags,
        } => {
            let result = unsafe { &mut *world }
                .lua_script_tile_add_item(x, y, z, item_type, count, flags)?;
            if let Some(id) = result {
                set_mutation_item_result(id);
            }
            Ok(())
        }
        LuaMutation::GameCreateItem {
            item_type,
            count,
            position,
        } => {
            let result =
                unsafe { &mut *world }.lua_script_game_create_item(item_type, count, position)?;
            if let Some(id) = result {
                set_mutation_item_result(id);
            }
            Ok(())
        }
        LuaMutation::TargetCombatHealth {
            attacker_id,
            target_id,
            combat_type,
            damage_min,
            damage_max,
            effect,
        } => {
            let ok = unsafe { &mut *world }.lua_script_target_combat_health(
                attacker_id,
                target_id,
                combat_type,
                damage_min,
                damage_max,
                effect,
            )?;
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::PlayerSay {
            creature_id,
            text,
            speak_type,
        } => unsafe { &mut *world }.lua_script_player_say(creature_id, text, speak_type),
        LuaMutation::PlayerShowTextDialog {
            creature_id,
            item_type,
            text,
        } => unsafe { &mut *world }.lua_script_show_text_dialog(creature_id, item_type, text),
        LuaMutation::AddItemEx {
            item_id,
            dest,
            can_drop_on_map,
            index,
            flags,
        } => {
            let rv = unsafe { &mut *world }.lua_script_add_item_ex(
                item_id,
                dest,
                can_drop_on_map,
                index,
                flags,
            )?;
            set_mutation_i32_result(rv);
            Ok(())
        }
        LuaMutation::GameCreateTile {
            x,
            y,
            z,
            is_dynamic,
        } => unsafe { &mut *world }.lua_script_game_create_tile(x, y, z, is_dynamic),
        LuaMutation::PlayerRegisterCreatureEvent {
            creature_id,
            name,
            register,
        } => {
            let ok = unsafe { &mut *world }.lua_script_player_register_creature_event(
                creature_id,
                name,
                register,
            )?;
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::PlayerSendOutfitWindow { creature_id } => {
            unsafe { &mut *world }.lua_script_send_outfit_window(creature_id)
        }
        LuaMutation::PlayerSetOutfit {
            creature_id,
            look_type,
            look_head,
            look_body,
            look_legs,
            look_feet,
            look_addons,
        } => unsafe { &mut *world }.lua_script_set_outfit(
            creature_id,
            look_type,
            look_head,
            look_body,
            look_legs,
            look_feet,
            look_addons,
        ),
        LuaMutation::PlayerSetVocation {
            creature_id,
            vocation_id,
        } => {
            let ok = unsafe { &mut *world }.lua_script_set_vocation(creature_id, vocation_id)?;
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::PlayerSetDirection {
            creature_id,
            direction,
        } => {
            let ok = unsafe { &mut *world }.lua_script_set_direction(creature_id, direction)?;
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::SaveServer => {
            unsafe { &mut *world }.lua_script_save_server();
            Ok(())
        }
        LuaMutation::PlayerSetGhostMode {
            creature_id,
            enabled,
        } => unsafe { &mut *world }.lua_script_set_ghost_mode(creature_id, enabled),
        LuaMutation::CreatureRemove { creature_id } => {
            unsafe { &mut *world }.lua_script_creature_remove(creature_id)
        }
        LuaMutation::HouseSetOwner { house_id, guid } => {
            unsafe { &mut *world }.lua_script_house_set_owner(house_id, guid);
            Ok(())
        }
        LuaMutation::HouseSetAccessList {
            house_id,
            list_id,
            text,
        } => {
            unsafe { &mut *world }.lua_script_house_set_access_list(house_id, list_id, text);
            Ok(())
        }
        LuaMutation::HouseKickPlayer {
            house_id,
            kicker_id,
            target_id,
        } => {
            let ok =
                unsafe { &mut *world }.lua_script_house_kick_player(house_id, kicker_id, target_id);
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::HouseSave { house_id } => {
            let _ = house_id;
            Ok(())
        }
        LuaMutation::PlayerSetEditHouse {
            creature_id,
            house_id,
            list_id,
        } => {
            let ok = unsafe { &mut *world }.lua_script_player_set_edit_house(
                creature_id,
                house_id,
                list_id,
            );
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::PlayerSendHouseWindow {
            creature_id,
            house_id,
            list_id,
        } => {
            let ok = unsafe { &mut *world }.lua_script_player_send_house_window(
                creature_id,
                house_id,
                list_id,
            );
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::ToolUse { request } => {
            let ok = crate::tool_use::apply(unsafe { &mut *world }, &request);
            set_mutation_bool_result(ok);
            Ok(())
        }
        LuaMutation::ConjureItem { request } => {
            let ok = crate::conjure::apply(unsafe { &mut *world }, &request);
            set_mutation_bool_result(ok);
            Ok(())
        }
    }
}

/// Register the mutation applier once at server startup.
pub fn register_lua_mutation_hooks() {
    tfs_rust_lua::register_lua_mutation_applier(apply_lua_mutation);
    tfs_rust_lua::register_lua_db_bridge(crate::lua_database::apply_lua_db);
}

pub(crate) fn with_lua_script_scope<F, R>(world: &mut GameWorld, f: F) -> R
where
    F: FnOnce(&mut GameWorld) -> R,
{
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || f(unsafe { &mut *world_ptr }))
    })
}

/// Run `Monster:onSpawn` / EventCallback onSpawn after native spawn loot.
pub fn fire_on_monster_spawned(
    world: &mut GameWorld,
    cid: CreatureId,
    startup: bool,
    artificial: bool,
) {
    let pos = world
        .creatures
        .get(cid)
        .map(|k| k.position())
        .unwrap_or(Position::default());
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            world
                .events
                .on_monster_spawned(cid, pos, startup, artificial, world);
        });
    });
}

fn event_cylinder(cyl: Cylinder) -> EventCylinder {
    match cyl {
        Cylinder::Tile { pos } => EventCylinder::Tile(pos),
        Cylinder::Container { item_id, .. } => EventCylinder::Container(item_id),
        Cylinder::Inventory { player_id, .. } => EventCylinder::Inventory(player_id),
    }
}

fn lua_move_position(cyl: Cylinder) -> Position {
    match cyl {
        Cylinder::Tile { pos } => pos,
        _ => Position::new(0xFFFF, 0, 0),
    }
}

/// TFS `Events::eventPlayerOnMoveItem` after native queryAdd, before the transfer.
pub fn fire_on_player_move_item(
    world: &mut GameWorld,
    player: CreatureId,
    item: ItemId,
    count: u16,
    from: Cylinder,
    to: Cylinder,
) -> ReturnValue {
    let from_pos = lua_move_position(from);
    let to_pos = lua_move_position(to);
    let from_cyl = event_cylinder(from);
    let to_cyl = event_cylinder(to);
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            world.events.on_player_move_item(
                player, item, count, from_pos, to_pos, from_cyl, to_cyl, world,
            )
        })
    })
}

/// TFS `Events::eventPlayerOnItemMoved` after a successful transfer.
pub fn fire_on_player_item_moved(
    world: &mut GameWorld,
    player: CreatureId,
    item: ItemId,
    count: u16,
    from: Cylinder,
    to: Cylinder,
) {
    let from_pos = lua_move_position(from);
    let to_pos = lua_move_position(to);
    let from_cyl = event_cylinder(from);
    let to_cyl = event_cylinder(to);
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            world.events.on_player_item_moved(
                player, item, count, from_pos, to_pos, from_cyl, to_cyl, world,
            );
        });
    });
}

/// TFS `Events::eventPlayerOnReportBug`.
pub fn fire_on_player_report_bug(
    world: &mut GameWorld,
    player: CreatureId,
    message: &str,
    pos: Position,
    category: u8,
) {
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            world
                .events
                .on_player_report_bug(player, message, pos, category, world);
        });
    });
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

/// TFS `GlobalEvents::startup` (`globalevent.cpp`).
pub fn fire_on_startup(world: &mut GameWorld) {
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            world.events.on_startup();
        });
    });
}

/// TFS `GlobalEvents::execute(GLOBALEVENT_SHUTDOWN)` (`globalevent.cpp`).
pub fn fire_on_shutdown(world: &mut GameWorld) {
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            world.events.on_shutdown();
        });
    });
}

/// TFS `Game::checkPlayersRecord` → `GLOBALEVENT_RECORD` (`game.cpp`).
pub fn fire_on_record(world: &mut GameWorld, current: u32, old: u32) {
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            world.events.on_record(current, old, world);
        });
    });
}

/// After a successful spawn: `players_online` row + `Game::checkPlayersRecord`.
pub fn after_player_online(world: &mut GameWorld, guid: u32) {
    let db = world.db.clone();
    tokio::spawn(async move {
        if let Err(e) = tfs_rust_db::insert_player_online(&db, guid).await {
            tracing::warn!(error = %e, guid, "players_online insert failed");
        }
    });
    let current = world.player_by_guid.len() as u32;
    if current <= world.players_record {
        return;
    }
    let old = world.players_record;
    world.players_record = current;
    let db = world.db.clone();
    tokio::spawn(async move {
        if let Err(e) = tfs_rust_db::save_players_record(&db, current).await {
            tracing::warn!(error = %e, current, "players_record save failed");
        }
    });
    fire_on_record(world, current, old);
}

/// Execute a fired `addEvent` timer callback with read context and mutation scope.
///
/// C++ reference: `LuaEnvironment::executeTimerEvent` (`luascript.cpp:18238`).
/// Called from the game loop when `GameCommand::LuaCallback { event_id }` arrives.
pub fn fire_on_timer_event(world: &mut GameWorld, event_id: u64) {
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            world.events.execute_timer_event(event_id);
        });
    });
}

/// CH-6: Dispatch a talkaction with read context and mutation scope active.
///
/// C++ reference: `talkaction.cpp:84-134` `TalkActions::playerSaySpell` →
/// `TalkAction::executeSay`. The `onSay` Lua callback may trigger mutations
/// (`addItem`, `sendMagicEffect`, …) so the mutation scope must be active.
pub fn fire_talkaction(world: &mut GameWorld, cid: CreatureId, text: &str) -> TalkActionResult {
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            let result = world.events.dispatch_talkaction(text, cid);
            tracing::debug!(?cid, text, ?result, "fire_talkaction result");
            result
        })
    })
}

/// PC-3a: Dispatch a spell's `onCastSpell` Lua callback with read context and
/// mutation scope active.
///
/// C++ reference: `InstantSpell::castSpell` → `LuaEnvironment::callLuaFunction`
/// (`spells.cpp` / `luascript.cpp:363`). The callback may trigger mutations
/// (`combat:execute`, `sendMagicEffect`, …) so the mutation scope must be active.
/// Returns `true` if the callback was found and returned `true`.
pub fn fire_on_cast_spell(
    world: &mut GameWorld,
    spell_words: &str,
    cid: CreatureId,
    need_direction: bool,
    has_param: bool,
    param: &str,
) -> bool {
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            world
                .events
                .dispatch_on_cast_spell(spell_words, cid, need_direction, has_param, param)
        })
    })
}

/// PC-3a Gap 6: Fire a rune script callback (`rune:{id}`).
pub fn fire_on_cast_rune(
    world: &mut GameWorld,
    rune_id: u16,
    cid: CreatureId,
    target_creature: Option<CreatureId>,
    target_pos: Option<(u16, u16, u8)>,
) -> bool {
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            world
                .events
                .dispatch_on_cast_rune(rune_id, cid, target_creature, target_pos)
        })
    })
}

/// TFS `Game::playerUseItem` / `playerUseItemEx` — hotkey uses send `(0xFFFF, 0, 0)`.
#[inline]
pub fn is_hotkey_use_position(pos: tfs_rust_common::Position) -> bool {
    pos.x == 0xFFFF && pos.y == 0 && pos.z == 0
}

/// TFS `Action::executeUse` — fire Lua `onUse` if registered for this item.
///
/// Returns `true` when Lua handled the use (skip native container/teleport).
/// `is_hotkey` is the 6th Lua arg (`callFunction(6)`).
#[allow(clippy::too_many_arguments)]
pub fn fire_on_use_action(
    world: &mut GameWorld,
    player: CreatureId,
    item: ItemId,
    from: tfs_rust_common::Position,
    target_item: Option<ItemId>,
    target_creature: Option<CreatureId>,
    to: tfs_rust_common::Position,
    is_hotkey: bool,
) -> bool {
    if crate::doors::try_use(world, player, item, target_item, to) {
        return true;
    }
    let item_type = world.items.get(item).map(|i| i.item_type).unwrap_or(0);
    let action_id = world.items.get(item).map(|i| i.action_id()).unwrap_or(0);
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            world.events.dispatch_on_use_action(
                player,
                item,
                item_type,
                action_id,
                from,
                target_item,
                target_creature,
                to,
                is_hotkey,
            )
        })
    })
}

/// TFS `MoveEvents::onCreatureMove` StepOut/StepIn — fire with mutation + ScriptContext.
///
/// C++ reference: `tile.cpp` `postRemoveNotification` / `postAddNotification` →
/// `MoveEvents::onCreatureMove` → `MoveEvent::executeStep`. Revscripts like
/// `closing_doors.lua` call `Tile()`, `item:transform`, `doRelocate` — all need the
/// same scope as `fire_on_use_action`. Called from `move_creature_on_map` after the
/// creature has left `from` and landed on `to` so `getCreatureCount()` sees leavers gone.
pub(crate) fn fire_creature_step_events(
    world: &mut GameWorld,
    cid: CreatureId,
    from: tfs_rust_common::Position,
    to: tfs_rust_common::Position,
    step_out_items: &[TileMoveEventItem],
    step_in_items: &[TileMoveEventItem],
) {
    crate::stepping_tiles::on_creature_step(world, cid, from, to, step_out_items, step_in_items);
    crate::doors::on_creature_step(world, cid, from, step_out_items, step_in_items);
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            // C++ `getLastPosition()` for executeStep — tile we left (`from`).
            let last_pos = from;
            for item in step_out_items {
                let _ = world.events.on_step_out(
                    Some(cid),
                    item.item_id,
                    item.item_type,
                    item.action_id,
                    from,
                    last_pos,
                );
            }
            for item in step_in_items {
                let _ = world.events.on_step_in(
                    Some(cid),
                    item.item_id,
                    item.item_type,
                    item.action_id,
                    to,
                    last_pos,
                );
            }
        });
    });
}

/// TFS `MoveEvents::onItemMove` — fire AddItem/RemoveItem after the tile change.
///
/// C++ reference: `tile.cpp` `postAddNotification` (`LINK_OWNER`) /
/// `postRemoveNotification` → `MoveEvents::onItemMove` → `executeAddRemItem`.
/// Pack scripts (`premium_bridge.lua`) call `doRelocate` / `Tile()` — same
/// mutation + ScriptContext nest as `fire_creature_step_events`.
pub(crate) fn fire_item_move_events(
    world: &mut GameWorld,
    item: ItemId,
    pos: tfs_rust_common::Position,
    is_add: bool,
) {
    let (item_type, action_id) = world
        .items
        .get(item)
        .map(|i| (i.item_type, i.action_id()))
        .unwrap_or((0, 0));
    let tile_items = world.tile_move_event_items(pos);
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            world
                .events
                .on_item_move(item, item_type, action_id, pos, is_add, &tile_items);
        });
    });
}

/// TFS `Weapon::executeUseWeapon` — fire `onUseWeapon(player, variant[, hit])`.
pub fn fire_on_use_weapon(
    world: &mut GameWorld,
    item_id: u16,
    cid: CreatureId,
    target_creature: Option<CreatureId>,
    target_pos: Option<(u16, u16, u8)>,
    hit: bool,
) -> bool {
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || {
            let world = unsafe { &mut *world_ptr };
            world
                .events
                .dispatch_on_use_weapon(item_id, cid, target_creature, target_pos, hit)
        })
    })
}

/// TFS `Events::eventPlayerOnInventoryUpdate` with read/mutation scope for userdata.
pub fn fire_on_player_inventory_update(
    world: &mut GameWorld,
    player: CreatureId,
    item: ItemId,
    slot: u8,
    equip: bool,
) {
    with_lua_script_scope(world, |world| {
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
    with_lua_script_scope(world, |world| {
        world
            .events
            .on_player_equip_check(player, item, item_type, slot, player_level)
    })
}

/// TFS `MoveEvents::onPlayerEquip` — `postAddNotification`.
pub fn fire_on_player_equip(world: &mut GameWorld, player: CreatureId, item: ItemId, slot: u8) {
    let item_type = world.items.get(item).map(|i| i.item_type).unwrap_or(0);
    let player_level = player_level_u32(world, player);
    with_lua_script_scope(world, |world| {
        world
            .events
            .on_player_equip(player, item, item_type, slot, player_level);
    });
    // Native `MoveEvent::EquipItem` abilities — `movement.cpp` (speed/skills/stats).
    // XML `function="onEquipItem"` is a C++ builtin, not the Lua stub we register.
    world.apply_equip_item_abilities(player, item, slot);
}

/// TFS `MoveEvents::onPlayerDeEquip` — `postRemoveNotification`.
pub fn fire_on_player_deequip(world: &mut GameWorld, player: CreatureId, item: ItemId, slot: u8) {
    let item_type = world.items.get(item).map(|i| i.item_type).unwrap_or(0);
    let player_level = player_level_u32(world, player);
    // Native deequip abilities first — needs `inventoryAbilities` still true
    // (`DeEquipItem` clears the flag itself; `clear_inventory_ability_on_deequip` is redundant).
    world.remove_equip_item_abilities(player, item, slot);
    with_lua_script_scope(world, |world| {
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

fn with_npc_mutation_scope<F, R>(world: &mut GameWorld, f: F) -> R
where
    F: FnOnce(&mut GameWorld) -> R,
{
    let world_ptr = std::ptr::from_mut(world);
    with_lua_mutation_scope(world_ptr as *mut (), || {
        let ctx: &dyn tfs_rust_common::ScriptContext = unsafe { &*world_ptr };
        with_lua_context(ctx, || f(unsafe { &mut *world_ptr }))
    })
}

/// NPC-7: fire `onAppear` for a registered lifecycle callback.
pub fn fire_npc_appear(
    world: &mut GameWorld,
    npc: CreatureId,
    callback: tfs_rust_content::npcs::NpcCallbackId,
) {
    with_npc_mutation_scope(world, |world| {
        world.events.on_npc_appear(npc, callback);
    });
}

/// NPC-7: fire `onDisappear`.
pub fn fire_npc_disappear(
    world: &mut GameWorld,
    npc: CreatureId,
    callback: tfs_rust_content::npcs::NpcCallbackId,
) {
    with_npc_mutation_scope(world, |world| {
        world.events.on_npc_disappear(npc, callback);
    });
}

/// NPC-7: fire `onMove`.
pub fn fire_npc_move(
    world: &mut GameWorld,
    npc: CreatureId,
    callback: tfs_rust_content::npcs::NpcCallbackId,
    from: tfs_rust_common::Position,
    to: tfs_rust_common::Position,
) {
    with_npc_mutation_scope(world, |world| {
        world.events.on_npc_move(npc, callback, from, to);
    });
}

/// NPC-7: fire custom `onSay` lifecycle callback.
pub fn fire_npc_say(
    world: &mut GameWorld,
    npc: CreatureId,
    callback: tfs_rust_content::npcs::NpcCallbackId,
    speaker: CreatureId,
    text: &str,
) {
    with_npc_mutation_scope(world, |world| {
        world.events.on_npc_say(npc, callback, speaker, text);
    });
}

/// NPC-7: fire `onThink` only when a callback id is registered on the definition.
pub fn fire_npc_think(
    world: &mut GameWorld,
    npc: CreatureId,
    callback: tfs_rust_content::npcs::NpcCallbackId,
    interval_ms: u32,
) {
    with_npc_mutation_scope(world, |world| {
        world.events.on_npc_think(npc, callback, interval_ms);
    });
}

/// NPC-7: evaluate a custom dialogue predicate (read-only by contract).
pub fn fire_npc_custom_predicate(
    world: &mut GameWorld,
    npc: CreatureId,
    player: CreatureId,
    callback: tfs_rust_content::npcs::NpcCallbackId,
) -> bool {
    with_npc_mutation_scope(world, |world| {
        world.events.on_npc_custom_predicate(npc, player, callback)
    })
}

/// NPC-7: run a custom dialogue action with immediate mutation scope.
pub fn fire_npc_custom_action(
    world: &mut GameWorld,
    npc: CreatureId,
    player: CreatureId,
    callback: tfs_rust_content::npcs::NpcCallbackId,
) -> bool {
    with_npc_mutation_scope(world, |world| {
        world.events.on_npc_custom_action(npc, player, callback)
    })
}

#[cfg(test)]
mod tests {
    use super::is_hotkey_use_position;
    use tfs_rust_common::Position;

    #[test]
    fn hotkey_use_position_is_ffff_0_0() {
        assert!(is_hotkey_use_position(Position::new(0xFFFF, 0, 0)));
        assert!(!is_hotkey_use_position(Position::new(0xFFFF, 1, 0)));
        assert!(!is_hotkey_use_position(Position::new(100, 100, 7)));
    }
}
