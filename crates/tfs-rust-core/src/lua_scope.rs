//! Scoped Lua script execution with read context + mutation applier.
//!
//! C++ reference: `LuaScriptInterface::executeTimer` / creature event dispatch — single game thread.

use crate::event_dispatcher::TalkActionResult;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use tfs_rust_lua::{
    self, set_mutation_bool_result, set_mutation_item_result, with_lua_context,
    with_lua_mutation_scope, LuaMutation,
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
        } => unsafe { &mut *world }.lua_script_remove_item(
            creature_id,
            item_type,
            count,
            sub_type,
            ignore_equipped,
        ),
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
        LuaMutation::PlayerAddCondition { creature_id, spec } => unsafe {
            &mut *world
        }
        .lua_script_player_add_condition(creature_id, spec),
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
        LuaMutation::PlayerAddManaSpent {
            creature_id,
            amount,
        } => unsafe { &mut *world }.lua_script_player_add_mana_spent(creature_id, amount),
        LuaMutation::ItemTransform {
            item_id,
            new_type,
            sub_type,
        } => {
            let ok = unsafe { &mut *world }.lua_script_item_transform(item_id, new_type, sub_type)?;
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
            let ok = unsafe { &mut *world }
                .lua_do_challenge_creature(challenger_id, target_id);
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
            let result = unsafe { &mut *world }.lua_script_create_monster(
                &name, x, y, z, extended, force,
            )?;
            if let Some(id) = result {
                set_mutation_item_result(id);
            }
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
            let ret = unsafe { &mut *world }
                .lua_script_creature_move_to_tile(creature_id, x, y, z, flags)?;
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
        } => unsafe { &mut *world }
            .lua_script_player_send_text_message(creature_id, msg_class, text),
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
        LuaMutation::SetWorldLight { level, color } => {
            let ok = unsafe { &mut *world }.set_world_light(level, color);
            set_mutation_bool_result(ok);
            Ok(())
        }
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
            world.events.dispatch_on_cast_spell(
                spell_words,
                cid,
                need_direction,
                has_param,
                param,
            )
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

/// TFS `Action::executeUse` — fire Lua `onUse` if registered for this item.
///
/// Returns `true` when Lua handled the use (skip native container/teleport).
pub fn fire_on_use_action(
    world: &mut GameWorld,
    player: CreatureId,
    item: ItemId,
    from: tfs_rust_common::Position,
    target_item: Option<ItemId>,
    target_creature: Option<CreatureId>,
    to: tfs_rust_common::Position,
) -> bool {
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
            )
        })
    })
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
            world.events.dispatch_on_use_weapon(
                item_id,
                cid,
                target_creature,
                target_pos,
                hit,
            )
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
        world
            .events
            .on_npc_custom_predicate(npc, player, callback)
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
