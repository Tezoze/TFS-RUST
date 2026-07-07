//! Lua script mutations queued/applied during script execution.
//!
//! C++ reference: `LuaScriptInterface` methods that mutate game state (`luascript.cpp`).

use std::cell::Cell;
use std::sync::OnceLock;

/// Game-state mutation requested from Lua userdata methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LuaMutation {
    PlayerAddItem {
        creature_id: u64,
        item_type: u16,
        count: u16,
    },
    PlayerAddItemFull {
        creature_id: u64,
        item_type: u16,
        count: u32,
        sub_type: i32,
        can_drop_on_map: bool,
        slot: u8,
    },
    PlayerRemoveItem {
        creature_id: u64,
        item_type: u16,
        count: u32,
        sub_type: i32,
        ignore_equipped: bool,
    },
    PlayerGetDepotChest {
        creature_id: u64,
        depot_id: u32,
        auto_create: bool,
    },
    PlayerGetInbox {
        creature_id: u64,
    },
    ItemMoveTo {
        item_id: u64,
        dest: LuaMoveDestination,
        flags: u32,
    },
    ItemRemove {
        item_id: u64,
        count: i32,
    },
    ContainerAddItem {
        container_id: u64,
        item_type: u16,
        count: u32,
        index: i32,
        flags: u32,
    },
    ItemSetActionId {
        item_id: u64,
        action_id: u16,
    },
    ItemSetUniqueId {
        item_id: u64,
        unique_id: u16,
    },
    ItemSetStoreItem {
        item_id: u64,
        store: bool,
    },
    /// 772 `SKILL_FED` refill — `player:feed(amount)` adds to `food_remaining`
    /// (`moveuse.cc:1846` `SetTimer(SKILL_FED, CurFoodTime + ObjFoodTime, ...)`).
    PlayerFeed {
        creature_id: u64,
        amount: u32,
    },
    /// `player:sendCancelMessage(text)` — `protocolgame.cpp` `sendTextMessage(
    /// MESSAGE_STATUS_SMALL, text)`. LUA-3; outbound-only, immediate apply.
    /// `text` is the already-resolved player-visible message string (the binding
    /// maps integer RETURNVALUE codes to descriptions before constructing this).
    PlayerSendCancelMessage {
        creature_id: u64,
        text: String,
    },
    /// `player:addCondition(condition)` — `luascript.cpp:2117`
    /// `Creature::addCondition`. LUA-4; immediate apply. `ctype` is the
    /// Lua-facing 772 bit-flag value (e.g. `CONDITION_CHANNELMUTEDTICKS = 1<<15`);
    /// the core applier maps it to the Rust `ConditionType` enum.
    PlayerAddCondition {
        creature_id: u64,
        ctype: i32,
        cond_id: i32,
        sub_id: u32,
        ticks: i32,
    },
    /// `player:removeCondition(type, id, subId)` — `luascript.cpp:2118`
    /// `Creature::removeCondition`. LUA-4; immediate apply.
    PlayerRemoveCondition {
        creature_id: u64,
        ctype: i32,
        cond_id: i32,
        sub_id: u32,
    },
    /// `sendChannelMessage(channelId, type, message)` — `chat.cpp` channel
    /// broadcast (server-originated, anonymous speaker). LUA-4 §1.7.
    SendChannelMessage {
        channel_id: u16,
        speak_type: u8,
        text: String,
    },
    /// `Position:sendMagicEffect(effect)` — `game.cpp:4816` `addMagicEffect`.
    /// CH-6 talkaction `/i` green-sparkle at player position. Broadcasts the
    /// effect to all spectators at `(x, y, z)`.
    PositionSendMagicEffect {
        x: u16,
        y: u16,
        z: u8,
        effect: u8,
    },
    /// `item:decay()` — `items.cpp` `startDecay` / `Game::checkDecay`.
    /// CH-6 talkaction `/i` item decay scheduling. Schedules the item for
    /// decay transformation after its `decayTime` elapses.
    ItemDecay {
        item_id: u64,
    },
}

/// Destination for `item:moveTo` — `luascript.cpp` `luaItemMoveTo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LuaMoveDestination {
    Container { item_id: u64 },
    Player { creature_id: u64 },
    Tile { x: u16, y: u16, z: u8 },
}

type LuaMutationApplier = fn(*mut (), LuaMutation) -> Result<(), String>;

static MUTATION_APPLIER: OnceLock<LuaMutationApplier> = OnceLock::new();

thread_local! {
    /// Opaque `&mut GameWorld` as `*mut ()` — set only for the duration of
    /// [`with_lua_mutation_scope`] on the game thread.
    static MUTATION_WORLD: Cell<*mut ()> = const { Cell::new(std::ptr::null_mut()) };
    static MUTATION_BOOL_RESULT: Cell<Option<bool>> = const { Cell::new(None) };
    static MUTATION_ITEM_RESULT: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Register the core handler that applies mutations (called once at startup).
pub fn register_lua_mutation_applier(applier: LuaMutationApplier) {
    let _ = MUTATION_APPLIER.set(applier);
}

/// Execute `f` with an active Lua mutation scope bound to `world`.
pub fn with_lua_mutation_scope<F, R>(world: *mut (), f: F) -> R
where
    F: FnOnce() -> R,
{
    MUTATION_WORLD.set(world);
    MUTATION_BOOL_RESULT.set(None);
    MUTATION_ITEM_RESULT.set(None);
    let result = f();
    MUTATION_WORLD.set(std::ptr::null_mut());
    result
}

pub fn take_mutation_bool_result() -> Option<bool> {
    MUTATION_BOOL_RESULT.take()
}

pub fn take_mutation_item_result() -> Option<u64> {
    MUTATION_ITEM_RESULT.take()
}

pub fn set_mutation_bool_result(v: bool) {
    MUTATION_BOOL_RESULT.set(Some(v));
}

pub fn set_mutation_item_result(v: u64) {
    MUTATION_ITEM_RESULT.set(Some(v));
}

fn apply_mutation(mutation: LuaMutation) -> Result<(), String> {
    let world = MUTATION_WORLD.get();
    if world.is_null() {
        return Err("Lua mutation scope not active".into());
    }
    let applier = MUTATION_APPLIER
        .get()
        .ok_or_else(|| "Lua mutation applier not registered".to_string())?;
    applier(world, mutation)
}

pub fn call_lua_add_item(creature_id: u64, item_type: u16, count: u16) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerAddItem {
        creature_id,
        item_type,
        count,
    })
}

pub fn call_lua_add_item_full(
    creature_id: u64,
    item_type: u16,
    count: u32,
    sub_type: i32,
    can_drop_on_map: bool,
    slot: u8,
) -> Result<Option<u64>, String> {
    apply_mutation(LuaMutation::PlayerAddItemFull {
        creature_id,
        item_type,
        count,
        sub_type,
        can_drop_on_map,
        slot,
    })?;
    Ok(take_mutation_item_result())
}

pub fn call_lua_remove_item(
    creature_id: u64,
    item_type: u16,
    count: u32,
    sub_type: i32,
    ignore_equipped: bool,
) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerRemoveItem {
        creature_id,
        item_type,
        count,
        sub_type,
        ignore_equipped,
    })
}

pub fn call_lua_get_depot_chest(
    creature_id: u64,
    depot_id: u32,
    auto_create: bool,
) -> Result<Option<u64>, String> {
    apply_mutation(LuaMutation::PlayerGetDepotChest {
        creature_id,
        depot_id,
        auto_create,
    })?;
    Ok(take_mutation_item_result())
}

pub fn call_lua_get_inbox(creature_id: u64) -> Result<Option<u64>, String> {
    apply_mutation(LuaMutation::PlayerGetInbox { creature_id })?;
    Ok(take_mutation_item_result())
}

pub fn call_lua_item_move_to(
    item_id: u64,
    dest: LuaMoveDestination,
    flags: u32,
) -> Result<bool, String> {
    apply_mutation(LuaMutation::ItemMoveTo {
        item_id,
        dest,
        flags,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

pub fn call_lua_item_remove(item_id: u64, count: i32) -> Result<bool, String> {
    apply_mutation(LuaMutation::ItemRemove { item_id, count })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

pub fn call_lua_container_add_item(
    container_id: u64,
    item_type: u16,
    count: u32,
    index: i32,
    flags: u32,
) -> Result<Option<u64>, String> {
    apply_mutation(LuaMutation::ContainerAddItem {
        container_id,
        item_type,
        count,
        index,
        flags,
    })?;
    Ok(take_mutation_item_result())
}

pub fn call_lua_set_action_id(item_id: u64, action_id: u16) -> Result<(), String> {
    apply_mutation(LuaMutation::ItemSetActionId {
        item_id,
        action_id,
    })
}

pub fn call_lua_set_unique_id(item_id: u64, unique_id: u16) -> Result<(), String> {
    apply_mutation(LuaMutation::ItemSetUniqueId {
        item_id,
        unique_id,
    })
}

pub fn call_lua_set_store_item(item_id: u64, store: bool) -> Result<(), String> {
    apply_mutation(LuaMutation::ItemSetStoreItem { item_id, store })
}

/// 772 `player:feed(amount)` — refill `food_remaining` (`moveuse.cc:1846`).
pub fn call_lua_feed(creature_id: u64, amount: u32) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerFeed {
        creature_id,
        amount,
    })
}

/// `player:sendCancelMessage(text)` — LUA-3. Enqueues a
/// `MESSAGE_STATUS_SMALL` text message to the player's connection.
/// `text` is the already-resolved player-visible message.
pub fn call_lua_send_cancel_message(creature_id: u64, text: String) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerSendCancelMessage {
        creature_id,
        text,
    })
}

/// `player:addCondition(condition)` — LUA-4. Immediate-apply condition add.
/// `ctype` is the Lua-facing 772 bit-flag value; the core applier maps it.
pub fn call_lua_add_condition(
    creature_id: u64,
    ctype: i32,
    cond_id: i32,
    sub_id: u32,
    ticks: i32,
) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerAddCondition {
        creature_id,
        ctype,
        cond_id,
        sub_id,
        ticks,
    })
}

/// `player:removeCondition(type, id, subId)` — LUA-4. Immediate-apply
/// condition removal.
pub fn call_lua_remove_condition(
    creature_id: u64,
    ctype: i32,
    cond_id: i32,
    sub_id: u32,
) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerRemoveCondition {
        creature_id,
        ctype,
        cond_id,
        sub_id,
    })
}

/// `sendChannelMessage(channelId, type, message)` — LUA-4 §1.7.
/// Server-originated channel broadcast (anonymous speaker).
pub fn call_lua_send_channel_message(
    channel_id: u16,
    speak_type: u8,
    text: String,
) -> Result<(), String> {
    apply_mutation(LuaMutation::SendChannelMessage {
        channel_id,
        speak_type,
        text,
    })
}

/// `Position:sendMagicEffect(effect)` — CH-6. Broadcasts a magic effect to
/// all spectators at `(x, y, z)`. C++ `Game::addMagicEffect`.
pub fn call_lua_send_magic_effect(x: u16, y: u16, z: u8, effect: u8) -> Result<(), String> {
    apply_mutation(LuaMutation::PositionSendMagicEffect { x, y, z, effect })
}

/// `item:decay()` — CH-6. Schedules the item for decay transformation.
/// C++ `items.cpp` `startDecay` / `Game::checkDecay`.
pub fn call_lua_item_decay(item_id: u64) -> Result<(), String> {
    apply_mutation(LuaMutation::ItemDecay { item_id })
}
